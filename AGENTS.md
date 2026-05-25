# Agent Guidelines for voip-ms

This document captures the design decisions, patterns, and trade-offs behind
this crate. It is the context an AI agent (or a new contributor) needs in
order to make consistent changes.

## Project Overview

**Purpose**: Async Rust client for the [voip.ms](https://voip.ms) REST API.

**Scope**: Every method the voip.ms REST endpoint exposes (222 as of the
committed WSDL) gets a typed request struct and a `Client` method. Nothing
more — no retry layer, no credential discovery, no configuration loading.

## Design Decisions

### 1. WSDL is the source of truth for inputs

**Decision**: The 222 `*Params` structs and `Client` methods in
`src/generated.rs` are generated from `tools/server.wsdl` by the
`xtask` workspace member (`xtask/src/main.rs`). Both the generator
and the WSDL snapshot are committed.

**Rationale**:

* The WSDL is the only machine-readable description of every method the
  voip.ms backend exposes. The public HTML docs at
  `https://voip.ms/m/apidocs.php` are gated by Cloudflare and not
  parseable programmatically.
* Code-generating is the only practical way to keep ~5 kLOC of mechanical
  Rust honest as voip.ms adds methods.
* The generator is an `xtask` (not a `build.rs`) so end-users don't pay
  codegen cost on `cargo build`. It's a pure-Rust workspace member, not
  a Python script, so contributors don't need a separate toolchain.

**How to apply**: When voip.ms adds an API method, replace
`tools/server.wsdl` and run `cargo xtask gen`. Do not hand-edit
`src/generated.rs` — the `@generated` banner reflects reality.

### 2. Responses are raw-by-default, with generated typed structs

**Decision**: Every generated `Client` method exposes both:

* a raw method that returns `Result<Value>`, and
* a typed `*_typed` method that returns `Result<T>` where `T` is a
  generated `*Response` struct (`GetBalanceResponse`,
  `GetDIDsInfoResponse`, …) shipped in `src/generated.rs`.

The `*Response` structs are produced by the same `xtask` run that
generates `*Params`, from three inputs:

1. `tools/server.wsdl` — method list and naming.
2. `tools/api-responses.json` — shape inferred by parsing
   `apidocs.php`'s `print_r`-style Output blocks (extractor is
   `xtask/src/extract.rs`, invoked via `cargo xtask extract-responses`
   over a saved HTML page).
3. `tools/api-response-overrides.json` — hand-edited corrections,
   either per-path scalar retypes or a full shape replacement for the
   handful of methods the extractor can't parse (`setSIPURI` has no
   Output block; `getLNPDetails` uses a non-standard PHP dialect).

All response fields are `Option<T>` with `#[serde(default)]` so that
voip.ms adding, removing, or omitting a field never breaks
deserialization. Numbers, booleans (`0/1`, `Y/N`), dates, and decimals
arrive as JSON strings from the API; the deserializers in
`src/responses.rs` (`deserialize_opt_*`) normalize both string and
native-typed forms and treat `"0000-00-00"` placeholders as `None`.

**Rationale**: The WSDL declares a single generic `arrayResponse` type
for all 222 operations — there is no machine-readable response schema.
The HTML docs do have sample outputs in a parseable `print_r` form,
which is enough to infer shapes for ~99 % of methods automatically; the
overrides file covers the rest without polluting the generator. Raw
`Value` methods remain available for callers who want full forward
compatibility with voip.ms drift on unknown fields.

**How to apply**: When voip.ms updates the docs, save a fresh copy of
`apidocs.php` HTML under `target/` (gitignored), run `cargo xtask
extract-responses <path>` to refresh `tools/api-responses.json`, review
the diff, and only edit `tools/api-response-overrides.json` if a scalar
is mis-typed or a method's Output block can't be parsed. Then
`cargo xtask gen` to refresh `src/generated.rs`.

### 3. All request fields are `Option<T>`

**Decision**: Generated `*Params` structs derive `Default` and every field
is `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`.

**Rationale**: The WSDL declares every input as nominally required
(`minOccurs="1"`), but the real voip.ms API treats most fields as
optional, with server-side defaults — especially the large `set*` and
`create*` methods (`createSubAccount` has 44 fields). Mirroring WSDL's
required-ness would force users to fill in fields they don't care about
and would break with every voip.ms default tweak. `Option` + `Default` +
struct-update-syntax gives the cleanest call sites:

```rust
SetSubAccountParams {
    id: Some(1234),
    description: Some("desk phone".into()),
    ..Default::default()
}
```

The trade-off: the type system does not enforce required fields. Users
must consult the official voip.ms docs to know what each method actually
needs. This is called out in the README.

### 4. Credentials live on the `Client`, not in the request structs

**Decision**: `api_username` and `api_password` are fields on `Client`,
**not** on the generated `*Params` structs (even though the WSDL lists
them on every input). The codegen explicitly filters them out via
`CLIENT_FIELDS`.

**Rationale**: Repeating credentials per-call is hostile to callers and
encourages copy-paste of secrets through code paths. One `Client::new` /
`Client::builder` and they're injected on every wire request.

### 5. Acronym-aware camelCase → snake_case conversion

**Decision**: `xtask/src/main.rs` tokenizes method names with an explicit
acronym list (`DID`, `SMS`, `IVR`, `LNP`, `CDR`, `URI`, `PDF`, `ID`, …)
sorted longest-first.

**Rationale**: The naïve `[a-z][A-Z]` split mangles voip.ms's
acronym-heavy names (`getDIDsInfo` → `get_di_ds_info`,
`getFaxMessagePDF` → `get_fax_message_p_d_f`). The acronym list yields
`get_dids_info` and `get_fax_message_pdf` instead — names a Rust
developer would have chosen by hand. New acronyms get added to the
`ACRONYMS` set in the generator.

**How to apply**: When a new voip.ms method introduces an acronym that
produces a single-letter token in `tokenize()`, add it to the `ACRONYMS`
constant in `xtask/src/main.rs` and regenerate.

### 6. No HTTP-level retry, no auth caching, no rate limiting

**Decision**: `Client::call` is one GET request, one JSON parse, one
status check. There is no built-in retry, backoff, or rate limiter.

**Rationale**: voip.ms's retry semantics depend heavily on which method
you're calling (`addCharge` is not safely retryable; `getBalance` is).
Baking in a retry policy would force the wrong default on someone. Users
who want one can wrap their `Client` in `tower::retry` or compose any
middleware via a custom `reqwest::Client` passed to `Client::builder`.

### 7. GET, not POST

**Decision**: All calls are GET with query parameters.

**Rationale**: voip.ms documents and accepts both, but every documented
example is GET. GET also keeps the request observable in logs/proxies
during development. The only risk is URL length on the few methods with
40+ parameters (`createSubAccount`, `setSubAccount`, `setQueue`); none
of those exceed typical URL limits in practice because most parameters
are `None` thanks to design decision #3.

## Code Patterns

### Calling the wire API

The `Client::call` method is the single point that hits the network. All
generated methods are thin wrappers:

```rust
pub async fn get_balance(&self, params: &GetBalanceParams) -> Result<Value> {
    self.call("getBalance", params).await
}
```

If a regeneration drift is ever needed (e.g. a method needs custom
encoding), break that one method out of the codegen with an explicit
skip-list and hand-write it in `src/client.rs`. Do not pollute
`generated.rs` with special cases.

### Error surfacing

Three variants, no more:

* `Error::Http` — wraps `reqwest::Error`. Includes both transport-level
  failures and `error_for_status`'s non-2xx surfacing.
* `Error::Api(ApiStatus)` — the response parsed as `{ "status": "..." }`
  with something other than `"success"`. The wire string is exposed
  verbatim through `ApiStatus`; we intentionally do **not** define a
  per-code enum because the set of statuses varies per method and is
  not stable.
* `Error::InvalidResponse(String)` — the response was 2xx and JSON but
  didn't contain a `status` field. Should be rare; if it happens
  systematically for a method, that's a voip.ms-side break.

## Project Structure

```
voip-ms/
├── Cargo.toml           # Workspace root + library package
├── LICENSE              # MIT
├── README.md            # User-facing docs
├── AGENTS.md            # This file
├── CHANGELOG.md
├── .cargo/config.toml   # `cargo xtask` alias
├── .rustfmt.toml        # edition = "2024"
├── .gitignore
├── .github/
│   ├── dependabot.yml   # Weekly cargo + actions updates
│   └── workflows/
│       ├── rust-ci.yaml              # fmt, clippy, test, coverage
│       ├── dependabot-automerge.yaml # auto-merge safe Cargo updates
│       └── release.yaml              # tag-validated publish + GitHub release
├── src/
│   ├── lib.rs           # Module surface; re-exports generated.rs
│   ├── client.rs        # Client, ClientBuilder, call()
│   ├── error.rs         # Error, ApiStatus, Result
│   ├── generated.rs     # 222 *Params + Client methods + *Response (generated)
│   └── responses.rs     # Custom serde deserializers for generated.rs
├── tests/
│   └── client.rs        # wiremock-based integration tests
├── tools/
│   ├── server.wsdl                   # Committed WSDL snapshot
│   ├── api-responses.json            # Extracted response shapes (generated)
│   └── api-response-overrides.json   # Hand-edited shape corrections
└── xtask/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # WSDL+responses+overrides → src/generated.rs
        ├── extract.rs           # apidocs HTML → tools/api-responses.json
        ├── overrides.rs         # Overrides schema + apply logic
        └── response_codegen.rs  # Shape → *Response struct emitter
```

## Dependencies

* **chrono 0.4**: Date/time helpers for starter typed response structs.
* **reqwest 0.13** (`json`, `query`, no default features): HTTP client + JSON
  deserialization. TLS backend is feature-gated.
* **rust_decimal 1**: Decimal parsing for money-like response fields.
* **serde 1** + **serde_json 1**: Request serialization, response
  deserialization.
* **thiserror 2**: Error derive.
* **url 2**: Base URL handling.

Dev-dependencies:

* **tokio 1** (`macros`, `rt-multi-thread`): Test runtime.
* **wiremock 0.6**: HTTP mocking in `tests/client.rs`.

## TLS Features

`default = ["rustls-tls-native-roots"]`.

| Feature | TLS stack | Root certs | Use case |
|---|---|---|---|
| `rustls-tls-native-roots` *(default)* | rustls | system | most servers, containers with CA bundle |
| `rustls-tls-webpki-roots` | rustls | embedded Mozilla | scratch/distroless images |
| `native-tls` | OS native | OS native | platforms where rustls is undesirable |

Pick one; they are not mutually exclusive at the type level, but enabling
both rustls feature sets is wasteful.

## Contributor Workflows

Contributor and maintainer workflows (testing strategy, CI/CD behavior,
regeneration, and releases) are documented in `DEVELOPMENT.md`.

## Evolution Notes

The crate started from a 4-question scoping conversation. The choices that
turned out load-bearing:

1. **Full typed coverage vs generic call-by-name**: We went with full
   typed coverage because it's discoverable from `Client::` autocomplete.
   The WSDL having 222 methods (not the ~80 estimated) made codegen
   the only viable route.
2. **Response shape**: Per-method typed responses are generated from
   the docs' sample-output blocks plus a small hand-edited overrides
   file. The raw `Value` methods stay available for callers that want
   forward compatibility with voip.ms drift on unknown fields, while
   `*_typed` calls deserialize into a known struct without callers
   writing their own.
3. **Optionality**: All-`Option` was chosen over WSDL's nominal
   required-ness because the API itself is more permissive than the WSDL
   and `Default + ..Default::default()` is the idiomatic Rust experience
   for sparse-update structs.
