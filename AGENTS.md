# Agent Guidelines for voip-ms

This document captures the design decisions, patterns, and trade-offs behind
this crate. It is the context an AI agent (or a new contributor) needs in
order to make consistent changes.

## Project Overview

**Purpose**: Async Rust client for the [VoIP.ms](https://voip.ms) REST API.

**Scope**: Every method the VoIP.ms REST endpoint exposes (222 as of the
committed WSDL) gets a typed request struct and a `Client` method. Nothing
more — no retry layer, no credential discovery, no configuration loading.

## Design Decisions

Decisions 1-3, 5, 5a, 7, 8 and 8a are recorded in full in `.claude/rules/`, one file per area, each listing the paths it covers. Code, DEVELOPMENT.md and the decisions themselves cite them by number, so the numbers here stay fixed. Claude Code loads a rule file when you read a matching path; other agents open it before editing there.

- 1 -- inputs are generated from the WSDL; never hand-edit `src/generated.rs`, run `cargo xtask gen`: [.claude/rules/codegen.md](.claude/rules/codegen.md).
- 2 -- typed responses plus `*_raw`; response inputs, overrides, `additions`, param docs and the docs-refresh procedure: [.claude/rules/codegen.md](.claude/rules/codegen.md). Values the crate cannot read (`Reported<T>`, `Unknown(String)`): [.claude/rules/domain-types.md](.claude/rules/domain-types.md). Derives: below.
- 3 -- every `*Params` field is `Option<T>`; the generated `new` constructors: [.claude/rules/codegen.md](.claude/rules/codegen.md).
- 5 -- identifier naming (`ACRONYMS`, `FIELD_IDENT_OVERRIDE`, `FIELD_IDENT_RENAME`): [.claude/rules/codegen.md](.claude/rules/codegen.md).
- 5a -- field-name type substitution (routing, durations, timezones, flags, ids, enums) and the `check-types` / `check-flags` audits: [.claude/rules/domain-types.md](.claude/rules/domain-types.md).
- 7 -- GET, or multipart POST for a file parameter; a new one goes in `BASE64_FILE_PARAM_PATHS`: [.claude/rules/client.md](.claude/rules/client.md).
- 8 -- record-listing timestamps (`OFFSET_OPS`, `attach_offset`): [.claude/rules/timestamps.md](.claude/rules/timestamps.md).
- 8a -- named-zone timestamps; a method enters `ZONE_OPS` as `Server` only after a measurement during DST: [.claude/rules/timestamps.md](.claude/rules/timestamps.md).

### Derives

Each family carries the one serde direction it uses -- `*Params` derive
`Serialize`, `*Response` derive `Deserialize` -- plus `PartialEq` and `Eq`,
which are independent of serde and let a whole value be compared, deduped, or
diffed.

**A derive needs a reason.** Every public type derives `Debug`, which the Rust
API guidelines require. A public type that holds plain data also derives
`Clone`, `PartialEq` and `Eq` -- what a caller needs to keep a copy of a value
and to assert on it -- and `Copy` when it is a small value. Nothing else goes
on without a reason that holds for the type itself:

* A serde impl is a wire contract that has to stay correct, and a `Default`
  manufactures a value that may be wrong, so neither is emitted without a
  caller.
* `Hash` and `PartialOrd`/`Ord` are not free either. They claim the type is a
  key or has an order, and a derived order on an enum is declaration order, which
  reads as a domain claim it does not make: a derived `Ord` on
  `TransactionDate` would sort every `At` before every `On` regardless of date.
  `TimezoneOffset` is the one ordered type, because it is a number of hours and
  its derived order is the numeric one.
* A private type -- including everything in `xtask` and `livetest` -- carries
  only what the code uses. The compiler is the test: drop the derive and see
  whether the workspace, tests included, still builds under clippy's
  `-D warnings`, which also catches a `.clone()` that silently falls back to
  cloning a reference.

Removing a derive that has shipped is a breaking change, and adding one is not,
so when in doubt leave it off. Three places depend on a specific trait being
present and will fail the build if one is dropped: `check-types`, the
`is_copy_ty` table, and the probe macros' `Default` bound on a params type.

`*Params` derive `Default`, which is what makes the struct-update idiom of
decision 3 work. `*Response` do not: a response is received, never built, and
a defaulted one would claim success over empty fields. That is also why
`ApiStatus` has no `Default` -- a status has no resting value, and `Success`
was only ever the answer to "what does the derive need", not to "what does an
unset status mean". The per-field `#[serde(default)]` is unaffected either way:
it defaults the field's own type, not the struct.

### 4. Credentials live on the `Client`, not in the request structs

**Decision**: `api_username` and `api_password` are fields on `Client`,
**not** on the generated `*Params` structs (even though the WSDL lists
them on every input). The codegen explicitly filters them out via
`CLIENT_FIELDS`.

**Rationale**: Repeating credentials per-call is hostile to callers and
encourages copy-paste of secrets through code paths. One `Client::new` /
`Client::builder` and they're injected on every wire request.

### 6. No HTTP-level retry, no auth caching, no rate limiting

**Decision**: `Client::call_raw` is one request, one JSON parse, one status
check. There is no built-in retry, backoff, or rate limiter.

**Rationale**: VoIP.ms's retry semantics depend heavily on which method
you're calling (`addCharge` is not safely retryable; `getBalance` is).
Baking in a retry policy would force the wrong default on someone. Users
who want one can wrap their `Client` in `tower::retry` or compose any
middleware via a custom `reqwest::Client` passed to `Client::builder`.

## Code Patterns

- Calling the wire API: one `send`, the shared `form.rs` serializer, no wire form taken from `Display`: [.claude/rules/client.md](.claude/rules/client.md).
- Error surfacing: the four `Error` variants and the empty-collection statuses: [.claude/rules/client.md](.claude/rules/client.md).
- Transport-failure classification: `TransportFailure` and `RetryOutlook`, both exhaustive: [.claude/rules/client.md](.claude/rules/client.md).

## Project Structure

```
voip-ms/
├── Cargo.toml           # Workspace root + library package
├── LICENSE              # MIT
├── README.md            # User-facing docs
├── AGENTS.md            # This file
├── .claude/rules/       # Area rules: the design decisions AGENTS.md indexes
├── CHANGELOG.md
├── .cargo/config.toml   # `cargo xtask` alias
├── .rustfmt.toml        # edition = "2024"
├── .gitignore
├── .github/
│   ├── dependabot.yml   # Weekly cargo + actions updates
│   └── workflows/
│       ├── rust-ci.yaml              # fmt, clippy, TLS check, test, coverage
│       ├── dependabot-automerge.yaml # auto-merge safe Cargo updates
│       └── release.yaml              # tag-validated publish + GitHub release
├── src/
│   ├── lib.rs           # Module surface; re-exports generated.rs
│   ├── client.rs        # Client, ClientBuilder, call()
│   ├── error.rs         # Error, ApiStatus, Result, TransportFailure
│   ├── form.rs          # Params -> the wire fields both transports carry
│   ├── generated.rs     # 222 *Params + Client methods + *Response (generated)
│   ├── responses.rs     # Custom serde deserializers for generated.rs; audio_in
│   └── types.rs         # Hand-written domain types (Routing, …)
├── tests/
│   ├── client.rs                # wiremock integration + wire-contract tests
│   ├── generated_params.rs      # drift oracle: every *Params serializes
│   ├── generated_responses.rs   # drift oracle: every *Response deserializes
│   ├── generated_enums.rs       # drift oracle: wire-enum tag<->variant maps
│   └── generated_api_status.rs  # drift oracle: every ApiStatus code round-trips
├── tools/
│   ├── server.wsdl                   # Committed WSDL snapshot
│   ├── api-responses.json            # Extracted response shapes (generated)
│   ├── api-statuses.json             # Extracted error-code table (generated)
│   ├── param-docs.json               # Rendered param descriptions (written by gen)
│   └── api-response-overrides.json   # Hand-edited shape corrections + enums
├── livetest/                         # Operator-local live-API drift harness (bin)
│   └── src/
│       ├── main.rs          # CLI, connectivity pre-check, sweep + probe run
│       ├── config.rs        # Two-dimensional AREA × DEPTH selection; secrets
│       ├── wire_methods.rs  # 222 wire names (generated: cargo xtask dump-methods)
│       ├── response_fields.rs # modeled key paths (generated: cargo xtask gen)
│       ├── areas/           # One module per functional area + the registry
│       └── harness/         # Report, RAII Scope, ledger, marker, drift probe
└── xtask/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # WSDL+responses+overrides → src/generated.rs
        ├── check_flags.rs       # audit: doc-mined boolean params vs the FLAG_* tables
        ├── check_types.rs       # audit: a field typed one way to read, another to write
        ├── dump_fields.rs       # response shapes → livetest/src/response_fields.rs (also run by gen)
        ├── dump_methods.rs      # src/generated.rs → livetest/src/wire_methods.rs
        ├── extract.rs           # apidocs HTML → tools/api-responses.json
        ├── field_overrides.rs   # Field-name → domain-type substitution table
        ├── overrides.rs         # Overrides schema + apply logic
        └── response_codegen.rs  # Shape → *Response struct emitter
```

## Dependencies

Deps whose types appear in the public API (`chrono`, `reqwest`, `rust_decimal`,
`serde_json`, `serde`) are pinned to a minor and re-exported from the crate root
so callers name the exact compatible version without a separate dependency.

* **chrono 0.4** (`serde`): `NaiveDate`/`NaiveDateTime` in typed response
  fields and date-range params, and `DateTime<FixedOffset>` inside a zoned
  `WallClock` (decisions #8 and #8a); the `serde` feature supplies the params'
  `YYYY-MM-DD` `Serialize`.
* **chrono-tz 0.10**: `Tz` on the zone params, [`crate::TimezoneName`] and
  [`crate::SERVER_ZONE`].
* **reqwest 0.13.5** (`json`, `multipart`, `query`, no default features): HTTP
  client + JSON deserialization. `multipart` carries the file-parameter methods
  (decision #7). TLS backend is feature-gated. Two things force the patch
  floor rather than a bare `0.13`: the earlier 0.13.x rustls features the TLS
  flags reference were renamed in 0.13.4, and `reqwest::Error::is_dns` --
  which `Error::transport` reads to separate a resolution failure from the
  connect error wrapping it -- only exists from 0.13.5.
* **rust_decimal 1.42**: Decimal parsing for money-like response fields.
* **serde 1.0** + **serde_json 1.0**: Request serialization, response
  deserialization (`serde_json::Value` is the `call_raw` return type).
* **thiserror 2**: Error derive (internal; no `thiserror` type is public).

Dev-dependencies:

* **tokio 1** (`macros`, `rt-multi-thread`): Test runtime.
* **wiremock 0.6**: HTTP mocking in `tests/client.rs`.

## TLS Features

`default = ["rustls-tls-native-roots"]`.

| Feature | TLS stack | Root certs | Use case |
|---|---|---|---|
| `rustls-tls-native-roots` *(default)* | rustls | OS trust store | most servers, containers with a CA bundle |
| `native-tls` | OS native | OS native | platforms where rustls is undesirable |

reqwest 0.13.4's `rustls` feature verifies via `rustls-platform-verifier` (the
OS trust store), which subsumes the former native-certs path. There is no
embedded-Mozilla-roots feature; an image with no OS trust store needs
`rustls-no-provider` plus a hand-built `ClientConfig` via
`use_preconfigured_tls`.

## Contributor Workflows

Contributor and maintainer workflows (testing strategy, CI/CD behavior,
regeneration, and releases) are documented in `DEVELOPMENT.md`.

Run `cargo clippy` and `cargo test` with `--workspace`. The root package is
also the workspace root and the workspace sets no `default-members`, so
without `--workspace` either command selects the root package alone and skips
`livetest` and `xtask`. A renamed public field or method can then break a call
site in `livetest` that the local run never compiles, and CI or the release
workflow fails on it. Before a commit, and before pushing a release tag, run
the checks the way CI does:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features unchecked-raw -- -D warnings
cargo test --workspace --features unchecked-raw
```

## Evolution Notes

The crate started from a 4-question scoping conversation. The choices that
turned out load-bearing:

1. **Full typed coverage vs generic call-by-name**: We went with full
   typed coverage because it's discoverable from `Client::` autocomplete.
   The WSDL having 222 methods (not the ~80 estimated) made codegen
   the only viable route.
2. **Response shape**: Per-method typed responses are generated from
   the docs' sample-output blocks plus a small hand-edited overrides
  file. The `*_raw` methods stay available for callers that want
  forward compatibility with VoIP.ms drift on unknown fields, while
  unsuffixed calls deserialize into a known struct without callers
  writing their own.
3. **Optionality**: All-`Option` was chosen over WSDL's nominal
   required-ness because the API itself is more permissive than the WSDL
   and `Default + ..Default::default()` is the idiomatic Rust experience
   for sparse-update structs.
