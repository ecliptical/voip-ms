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

### 1. WSDL is the source of truth for inputs

**Decision**: The 222 `*Params` structs and `Client` methods in
`src/generated.rs` are generated from `tools/server.wsdl` by the
`xtask` workspace member (`xtask/src/main.rs`). Both the generator
and the WSDL snapshot are committed.

**Rationale**:

* The WSDL is the only machine-readable description of every method the
  VoIP.ms backend exposes. The public HTML docs at
  `https://voip.ms/m/apidocs.php` are gated by Cloudflare and not
  parseable programmatically.
* Code-generating is the only practical way to keep ~5 kLOC of mechanical
  Rust honest as VoIP.ms adds methods.
* The generator is an `xtask` (not a `build.rs`) so end-users don't pay
  codegen cost on `cargo build`. It's a pure-Rust workspace member, not
  a Python script, so contributors don't need a separate toolchain.

The WSDL's scalar types are advisory, not authoritative: `xsd:string` →
`String`, `xsd:integer` → `u64` (every integer param is a non-negative id
or count, and response ids are already `u64`, so `i64` would force a cast
on every round-trip), `xsd:decimal` → `rust_decimal::Decimal` (the decimal
params are money amounts, which `f64` would serialize with float
artifacts). The `xsd_to_rust` mapping lives in `xtask/src/main.rs`; the
field-name override table (see 5a) corrects individual fields the WSDL
mistypes entirely.

**How to apply**: When VoIP.ms adds an API method, replace
`tools/server.wsdl` and run `cargo xtask gen`. Do not hand-edit
`src/generated.rs` — the `@generated` banner reflects reality.

### 2. Responses are typed-by-default, with raw escape hatches

**Decision**: Every generated `Client` method exposes both:

* an unsuffixed typed method that returns a generated `*Response`
  struct (`GetBalanceResponse`, `GetDIDsInfoResponse`, …), and
* a `*_raw` method that returns `Result<Value>`.

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

The same `extract-responses` pass also mines two doc-comment sources
into `api-responses.json`: `param_docs` (per-parameter descriptions from
each method's `Parameters` cell, including `[Required]` markers,
examples, and value constraints) and `method_docs` (each method's
one-line summary). `cargo xtask gen` renders these as `///` comments on
the `*Params` fields and on the `*Params` struct + `Client` method,
respectively.

All response fields are `Option<T>` with `#[serde(default)]` so that
VoIP.ms adding, removing, or omitting a field never breaks
deserialization. Numbers, booleans (`0/1`, `Y/N`), dates, and decimals
arrive as JSON strings from the API; the deserializers in
`src/responses.rs` (`deserialize_opt_*`) normalize both string and
native-typed forms and treat `"0000-00-00"` placeholders as `None`.

**Rationale**: The WSDL declares a single generic `arrayResponse` type
for all 222 operations — there is no machine-readable response schema.
The HTML docs do have sample outputs in a parseable `print_r` form,
which is enough to infer shapes for ~99 % of methods automatically; the
overrides file covers the rest without polluting the generator.
`*_raw` methods remain available for callers who want full forward
compatibility with VoIP.ms drift on unknown fields.

**How to apply**: When VoIP.ms updates the docs, re-run the full refresh
procedure (re-extract `api-responses.json` *and* `api-statuses.json` from
a freshly saved HTML page, review the diffs, correct only
`api-response-overrides.json`, then `cargo xtask gen`). The exact
commands, review checklist, and gotchas are in
[DEVELOPMENT.md](DEVELOPMENT.md#regenerating-the-api-surface) — that is
the canonical, reproducible procedure; keep it in sync when the codegen
inputs or steps change.

### 3. All request fields are `Option<T>`

**Decision**: Generated `*Params` structs derive `Default` and every field
is `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`.

**Rationale**: The WSDL declares every input as nominally required
(`minOccurs="1"`), but the real VoIP.ms API treats most fields as
optional, with server-side defaults — especially the large `set*` and
`create*` methods (`createSubAccount` has 44 fields). Mirroring WSDL's
required-ness would force users to fill in fields they don't care about
and would break with every VoIP.ms default tweak. `Option` + `Default` +
struct-update-syntax gives the cleanest call sites:

```rust
SetSubAccountParams {
    id: Some(1234),
    description: Some("desk phone".into()),
    ..Default::default()
}
```

The trade-off: the type system does not enforce required fields. Users
must consult the official VoIP.ms docs to know what each method actually
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

**Rationale**: The naïve `[a-z][A-Z]` split mangles VoIP.ms's
acronym-heavy names (`getDIDsInfo` → `get_di_ds_info`,
`getFaxMessagePDF` → `get_fax_message_p_d_f`). The acronym list yields
`get_dids_info` and `get_fax_message_pdf` instead — names a Rust
developer would have chosen by hand. New acronyms get added to the
`ACRONYMS` set in the generator.

Field identifiers go through the same tokenizer: a camelCase wire name
(`isMobile`, `rateCenter`, `sipuri`) becomes a snake_case Rust ident
(`is_mobile`, `rate_center`, `sip_uri`) with a serde `rename` back to the
wire form, on both the `*Params` (serialize) and `*Response` (deserialize)
side. `rust_field_ident` in `xtask/src/main.rs` also keyword-escapes
(`type` → `r#type`) and `field_`-prefixes names that aren't
identifier-shaped.

**How to apply**: When a new VoIP.ms method introduces an acronym that
produces a single-letter token in `tokenize()`, add it to the `ACRONYMS`
constant in `xtask/src/main.rs` and regenerate.

### 5a. Domain types substituted by field name

**Decision**: A small set of stringly-typed fields are upgraded to
domain types during codegen, driven by the field's snake_case name (not
its method). Two override mechanisms feed the same substitution table
in `xtask/src/field_overrides.rs`:

* **Built-in substitutions** (hand-written in `field_overrides.rs`):
  the 12 routing-related fields (`routing`, `failover_busy`,
  `failover_noanswer`, `failover_unreachable`, plus the
  `fail_over_routing_*` variants used by queues) map to
  [`crate::Routing`], a tagged enum hand-written in `src/types.rs`
  that parses VoIP.ms's `kind:value` strings (`account:100001_VoIP`,
  `fwd:5551234567`, `sip:user@host:port`, `none:`, …). Routing
  changes shape rarely and benefits from a custom `FromStr` (e.g.
  SIP URIs may contain `:`, so only the first `:` is the separator).
* **Seconds-or-sentinel durations** map to [`crate::Seconds`] (six queue /
  announcement fields in `SECONDS_FIELDS`) or [`crate::WaitTime`]
  (`maximum_wait_time`), hand-written enums that hold a `u64` count *or* an
  unbounded sentinel -- VoIP.ms documents these as a number of seconds or a
  word (`none` / `unlimited`), which a bare `u64` can't represent. They carry
  their own (de)serialization (tolerant of a number, a numeric string, or
  either sentinel word).
* **Boolean flags** map to `bool`, registered in the `FLAG_01_FIELDS` /
  `FLAG_YES_NO_FIELDS` consts of `xtask/src/field_overrides.rs`. Many
  parameters VoIP.ms documents as `1 = true, 0 = false` (or `yes`/`no`) are
  under-typed by the WSDL as `xsd:integer` / `xsd:string`, so the extractor
  would emit `i64` / `String` and leak the wire encoding. The wire form lives
  in a serializer, not the type: the override carries a `param_serializer`
  (`serialize_opt_flag_01` / `serialize_opt_flag_yes_no` in `src/responses.rs`)
  emitted as `serialize_with` on the param, since a bare `bool` serializes to
  `true`/`false`, which these parameters reject. Responses use the existing
  tolerant `deserialize_opt_bool_from_string_number_or_yn`, accepting
  `1`/`0`/`yes`/`no`/`true`/`false` as string, number, or bool. A validate-only
  flag whose `false` means the same as absent (the `test` param) sets
  `param_skip_if` so it's emitted as plain `bool` (default `false`, omitted from
  the request when `false`) rather than `Option<bool>`.
* **Phone-number identifier fields** stay `String` on both the param and
  response side (`PHONE_STRING_FIELDS` in `xtask/src/field_overrides.rs`:
  `did`, `number`, `phone_number`, `contact`, `destination`, `stationid`).
  A phone number is an identifier, never a quantity -- it can carry leading
  zeros, exceed `i64` range, or hold a SIP form (`sip:2563` in
  `setPhonebook`) -- but both the WSDL (`xsd:integer` on the fax `did` and
  `setCallback`/`setPhonebook` `number` params) and the extractor (an
  all-digit doc sample infers `integer`) under-type them, so the override
  forces `String` globally instead of patching method-by-method. The
  response side keeps the tolerant string deserializer since VoIP.ms may
  ship the value as a bare JSON number. Deliberately excluded: the plural
  `dids` (sometimes a list of numeric vPRI ids) and `from` (a date filter
  in the `getSMS`-family params, an email in `getEmailToFax`'s response).
  `cargo xtask gen` warns when a `patches` entry is shadowed by a
  field-name override so retired per-method patches get removed.
* **Date-range params** (`date_from`, `date_to` in `DATE_FIELDS`) map to
  [`chrono::NaiveDate`], whose own `Serialize` emits the documented
  `YYYY-MM-DD` wire form. The bare `date` field is excluded -- it is a
  datetime in some responses and a date in others, so no single type fits.
* **Declarative enum overrides** in
  `tools/api-response-overrides.json` under the new `enums` (variant
  list with wire strings) and `field_types` (field-name → enum-name)
  sections. The generator emits the enum type (deriving `Debug`, `Clone`,
  `PartialEq`, `Eq`, `Hash` -- not `Copy`, since the `Unknown(String)`
  catch-all holds a `String`), `as_wire` / `from_wire`, `Display`,
  `Serialize`, `Deserialize`, plus a per-enum
  `deserialize_opt_*` helper, and substitutes the field's type in
  every `*Params` and `*Response` struct that has that field. Used
  for `DtmfMode`, `Nat`, `EmailAttachmentFormat`,
  `TranscriptionFormat`, `PlayInstructions`, `RingStrategy`,
  `RingGroupOrder`, `VoicemailFolder`, `QueueEmptyBehavior`,
  `EstimatedHoldTimeAnnounce`, `CallPickupBehavior`, `RecordingSort`,
  `DialingMode`, `TollFreeCarrier`, `DidBillingType`, and `LocationType`.
  Integer-coded enums (`1`/`2`, `-1`) work the same way -- the generated
  deserializer accepts the wire value as a JSON string, number, or bool.

Both kinds of substituted enum carry an `Unknown(String)` (or
`Unknown { tag, value }` for `Routing`) catch-all so VoIP.ms adding
a new variant or shipping an unexpected value never breaks
deserialization.

Field-name substitution is global but shape-aware: on the response side it
applies only to scalar-shaped fields, since a substituted scalar type can
never stand in for a list or object -- so a reference catalog returned
under an overridden field name (`getNAT`'s `nat`, `getPlayInstructions`'s
`play_instructions`) keeps its structural type automatically. Beyond that,
two JSON sections handle fields whose name means different things in
different structs:

* `field_type_skip` (`["StructName.field"]`) suppresses the name-based
  override for one struct -- on both the `*Params` and `*Response` side --
  keeping its WSDL/inferred/patched type. Two cases use it:
  `GetVoicemailsResponseVoicemail.urgent` is a *count*, not the per-message
  flag; and `getFaxMessages`'s `folder` is a free-text fax-folder name
  (`SENT` / `ALL` / user-created via `setFaxFolder`), not one of the fixed
  [`VoicemailFolder`] variants the global `folder` mapping would impose.
* `field_type_override` (`{"StructName.field": "EnumName"}`) is the
  assigning complement: it types one struct's field as a specific enum,
  overriding both the inferred type and any `field_types` entry. The `type`
  field needs this -- it's a search mode in `SearchVanityParams`, a message
  direction in `GetSMSResponseSMS`, and a reference-data code elsewhere, so
  no single global mapping fits. A per-struct entry wins over the global
  table on both the param and response side.

**Rationale**: Field names like `routing`, `dtmf_mode`, and `nat` mean
the same thing across every method they appear on. Substituting by
field name keeps the override table tiny and avoids per-method
duplication. Hand-written types stay in `src/types.rs` for cases that
need custom parsing; routine `set of fixed strings` enums are declared
in JSON to keep the generator the source of truth.

**How to apply**: For a new closed-set scalar (e.g. a `priority` field
with documented values `low`/`normal`/`high`), add an entry to `enums`
and a `field_types` mapping in `tools/api-response-overrides.json` and
regenerate. For a new boolean flag, add its field name to
`FLAG_01_FIELDS` or `FLAG_YES_NO_FIELDS` in
`xtask/src/field_overrides.rs` (no JSON or new type needed) --
`cargo xtask check-flags` audits those tables against the doc-mined
parameter descriptions and reports both uncovered flag-like params and
stale entries. For a scalar
that needs structured parsing (multi-part value, custom validation),
hand-write it in `src/types.rs`, register the field names in
`xtask/src/field_overrides.rs::ROUTING_FIELDS`-style const, and add the
deserializer to `src/responses.rs`.

### 6. No HTTP-level retry, no auth caching, no rate limiting

**Decision**: `Client::call_raw` is one GET request, one JSON parse, one
status check. There is no built-in retry, backoff, or rate limiter.

**Rationale**: VoIP.ms's retry semantics depend heavily on which method
you're calling (`addCharge` is not safely retryable; `getBalance` is).
Baking in a retry policy would force the wrong default on someone. Users
who want one can wrap their `Client` in `tower::retry` or compose any
middleware via a custom `reqwest::Client` passed to `Client::builder`.

### 7. GET, not POST

**Decision**: All calls are GET with query parameters.

**Rationale**: VoIP.ms documents and accepts both, but every documented
example is GET. GET also keeps the request observable in logs/proxies
during development. The only risk is URL length on the few methods with
40+ parameters (`createSubAccount`, `setSubAccount`, `setQueue`); none
of those exceed typical URL limits in practice because most parameters
are `None` thanks to design decision #3.

## Code Patterns

### Calling the wire API

The `Client::call_raw` method is the single point that hits the network.
`Client::call` and `Client::call_at` deserialize its result. All generated
methods are thin wrappers over `Client::call` (or `Client::call_raw` for
the `*_raw` variants):

```rust
pub async fn get_balance(&self, params: &GetBalanceParams) -> Result<GetBalanceResponse> {
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
  with something other than `"success"`. `ApiStatus` is a generated enum
  with one PascalCase variant per documented code (~475 of them) plus an
  `Unknown(String)` catch-all, so a code VoIP.ms returns but hasn't
  documented is preserved verbatim rather than lost — the variant set is
  documentation, not a closed contract. `ApiStatus::from_wire` /
  `as_str` round-trip the wire string, `description()` returns the
  documented meaning (`None` for `Unknown`), and `is_documented()`
  reports whether it's a known variant. The enum, its impls, and the
  description table are emitted by `cargo xtask gen` from
  `tools/api-statuses.json`, which is extracted from the docs' global
  "Error Codes" table via `cargo xtask extract-statuses <html>`. Because
  the docs ship a couple of codes capitalized (`Invalid_threshold`), the
  variant's `as_str` preserves the wire casing while the variant
  *identifier* normalizes through the same acronym-aware PascalCase as
  method/type names (`no_did` → `NoDID`).

  **Empty-collection statuses are not errors for typed calls.** VoIP.ms
  returns a distinct `no_*` status per list method when the list is empty
  (`no_sms`, `no_cdr`, `no_messages`, …). The typed `Client::call` /
  `call_at` (and so every unsuffixed generated method) fold any status for
  which `ApiStatus::is_empty()` is true into a successful data-less response
  -- collection fields deserialize to `None` -- instead of `Error::Api`. The
  `*_raw` methods (and `call_raw`) deliberately keep the strict verbatim
  contract: they still surface an empty status as `Error::Api`, so the raw
  escape hatch reflects exactly what VoIP.ms returned. `check_status` in
  `src/client.rs` classifies the status; the two paths diverge in
  `call_raw` vs `call`/`call_at`. The classification is hand-curated in the
  `empty_statuses` array of `tools/api-response-overrides.json` and emitted
  into `ApiStatus::is_empty()` by `cargo xtask gen`; codes that look like
  `no_*` but signal a real failure (`no_base64file`, `no_callstatus`,
  `no_change_billingtype`, `no_provision`, `no_provision_update`,
  `no_sequences`) are deliberately excluded. To reclassify, edit that array
  and regenerate -- an entry naming a status code absent from
  `tools/api-statuses.json` fails the codegen.
* `Error::InvalidResponse(String)` — the response was 2xx and JSON but
  didn't contain a `status` field. Should be rare; if it happens
  systematically for a method, that's a VoIP.ms-side break.

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
│   ├── responses.rs     # Custom serde deserializers for generated.rs
│   └── types.rs         # Hand-written domain types (Routing, …)
├── tests/
│   └── client.rs        # wiremock-based integration tests
├── tools/
│   ├── server.wsdl                   # Committed WSDL snapshot
│   ├── api-responses.json            # Extracted response shapes (generated)
│   ├── api-statuses.json             # Extracted error-code table (generated)
│   └── api-response-overrides.json   # Hand-edited shape corrections + enums
├── livetest/                         # Operator-local live-API drift harness (bin)
│   └── src/
│       ├── main.rs          # CLI, connectivity pre-check, sweep + probe run
│       ├── config.rs        # Two-dimensional AREA × DEPTH selection; secrets
│       ├── wire_methods.rs  # 222 wire names (generated: cargo xtask dump-methods)
│       ├── areas/           # One module per functional area + the registry
│       └── harness/         # Report, RAII Scope, ledger, marker, drift probe
└── xtask/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # WSDL+responses+overrides → src/generated.rs
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
  fields and date-range params; the `serde` feature supplies the params'
  `YYYY-MM-DD` `Serialize`.
* **reqwest 0.13.4** (`json`, `query`, no default features): HTTP client + JSON
  deserialization. TLS backend is feature-gated. Floored at 0.13.4 -- the
  earlier 0.13.x rustls features the TLS flags reference were renamed there.
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
