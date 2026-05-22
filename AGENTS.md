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

### 2. Responses are `serde_json::Value`, not typed structs

**Decision**: Every generated `Client` method returns `Result<Value>`. There
are no `*Response` types.

**Rationale**: The WSDL declares a single generic `arrayResponse` type for
all 222 operations — there is no machine-readable description of any
response shape. Inventing 222 hand-curated response types would:

* Be a large, error-prone surface to maintain.
* Become silently wrong whenever voip.ms tweaks a response.
* Force two crate revisions (this one and the user's) for every shape
  change.

Returning `Value` keeps the crate bare-bones and shifts the typed
deserialization to where the schema actually lives — in the caller's code,
against the specific shape they need. Callers who want typing use
`serde_json::from_value` (see README).

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
│       └── dependabot-automerge.yaml # auto-merge patch/minor
├── src/
│   ├── lib.rs           # Module surface; re-exports generated.rs
│   ├── client.rs        # Client, ClientBuilder, call()
│   ├── error.rs         # Error, ApiStatus, Result
│   └── generated.rs     # 222 *Params structs + Client methods (generated)
├── tests/
│   └── client.rs        # wiremock-based integration tests
├── tools/
│   └── server.wsdl      # Committed WSDL snapshot
└── xtask/
    ├── Cargo.toml
    └── src/main.rs      # WSDL → src/generated.rs (run via `cargo xtask gen`)
```

## Dependencies

* **reqwest 0.12** (`json`, no default features): HTTP client + JSON
  deserialization. TLS backend is feature-gated.
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

## Testing Strategy

1. **Integration tests** (`tests/client.rs`) using `wiremock` to assert:
   * a `success` response surfaces the full envelope verbatim
   * a non-`success` status maps to `Error::Api`
   * a 5xx surfaces as `Error::Http`
   * a missing `status` field surfaces as `Error::InvalidResponse`
   * `None` fields don't appear in the query string
   * `Client::call` is usable directly for typed deserialization

2. **Coverage target**: ~80% (Client + Error). `generated.rs` is
   mechanical and tested transitively by exercising one or two methods
   via the wiremock fixtures.

3. **Not tested**: live calls to voip.ms. Doing so would require real
   credentials and a live account; CI cannot reasonably exercise it.

## CI/CD

* **`rust-ci.yaml`**: Runs on PR + push to `main`.
  * `cargo fmt --all -- --check`
  * `cargo clippy --all -- -D warnings`
  * `cargo test` with `RUSTFLAGS=-Cinstrument-coverage`; coverage report
    posted as a PR comment via `ecliptical/covdir-report-action`.

* **`dependabot-automerge.yaml`**: Auto-approves and merges patch/minor
  Cargo updates from Dependabot.

## Evolution Notes

The crate started from a 4-question scoping conversation. The choices that
turned out load-bearing:

1. **Full typed coverage vs generic call-by-name**: We went with full
   typed coverage because it's discoverable from `Client::` autocomplete.
   The WSDL having 222 methods (not the ~80 estimated) made codegen
   the only viable route.
2. **Response shape**: Typed responses were considered and rejected
   because the WSDL doesn't carry that information and hand-curating
   ~222 envelopes is busywork that goes stale quickly.
3. **Optionality**: All-`Option` was chosen over WSDL's nominal
   required-ness because the API itself is more permissive than the WSDL
   and `Default + ..Default::default()` is the idiomatic Rust experience
   for sparse-update structs.
