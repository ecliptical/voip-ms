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
`src/generated.rs` — the `@generated` banner reflects reality. The
hand-maintained `tests/generated_*.rs` oracles are the tripwire for this
step: after a regen, a failure there means the surface changed in a way the
oracles don't yet know about; reconcile them by hand (see DEVELOPMENT.md's
testing strategy), never suppress or auto-generate them.

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
3. `tools/api-response-overrides.json` -- hand-edited corrections:
   per-path scalar retypes, per-path scalar `additions`, or a full shape
   replacement for the handful of methods the extractor can't parse
   (`setSIPURI` has no Output block; `getLNPDetails` uses a non-standard
   PHP dialect).

An extractor driven by the docs can only see a documented field, so a
field VoIP.ms returns but never documents reaches the typed surface
through `additions` -- a path plus a scalar type, appended to the
extracted shape (`getCDR`'s `ip` and `useragent`). A full shape
replacement would also work but freezes the method against later doc
updates, and the extracted shape stays authoritative for everything
else. An addition naming a field the extractor already found fails the
codegen: the docs have caught up and the entry is stale.

Finding such a field is the live harness's job, not the extractor's.
Because no `*Response` sets `deny_unknown_fields` (decision 2 depends on
it), an unmodeled key cannot fail a deserialization, so the raw-vs-typed
probe is blind to one by construction -- it only fires when the typed
read *fails*. `cargo xtask gen` therefore also emits the modeled key paths
per method into `livetest/src/response_fields.rs` -- from the shapes it
just rendered the structs from, so the table cannot fall behind them --
and the probe diffs every live envelope against them, reporting an
`UNMODELED` outcome that prints the `additions` entry to paste.
`cargo xtask dump-fields` rebuilds that table on its own when needed.
The two directions are complementary: drift is "the crate can't read
what arrived", unmodeled is "the crate silently dropped part of it".

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

**A value the crate cannot read costs its own field, not the response.** A
`*Response` is one value built from one envelope, so a deserializer that errors
on a single field discards every record beside it -- the caller gets an error
instead of the rows that were fine. Every response date is therefore
[`crate::Reported<T>`] (`Parsed(T)` or `Unreadable(String)`), and every
substituted enum carries `Unknown(String)`. The wrapper keeps the text rather
than answering `None`, which would discard it: the caller can still salvage what
arrived, and "unreadable" stays distinguishable from "absent", which is what the
live harness reads. Params are the other way round -- they are written, never
received, so they keep the bare `chrono` type, which is why `FieldOverride`
carries a `response_rust_type` beside `rust_type`.

The rule is that an unexpected **value** degrades and an unexpected **contract**
does not. The timestamp readers are where the two meet. In a named-zone field a
value with no offset is part of the contract, and reads as
[`crate::WallClock::Bare`], which claims no zone, rather than as UTC, which
would invent one. In a record-listing field it is a broken contract, since
there it is a wall clock shifted by the `timezone` sent that `attach_offset`
never qualified, so `deserialize_opt_record_listing_timestamp` refuses it
(decision #8). A JSON list or object where a scalar belongs is a shape, and no
scalar type can stand in for one, so that is rejected too.

**The class is not closed.** Seven readers still fail the whole envelope on a
well-shaped string they cannot read. Four are scalar helpers in
`src/responses.rs`: `deserialize_opt_u64_from_string_or_number` (`"1,234"`),
`deserialize_opt_decimal_from_string_or_number` (`"$1.00"`),
`deserialize_opt_bool_from_string_number_or_yn` (`"maybe"`) and
`deserialize_opt_routing`. The other three are the sentinel types in
`src/types.rs` -- [`crate::Seconds`], [`crate::WaitTime`] and
[`crate::MaxMembers`] -- which accept a number, a numeric string or their own
sentinel word and reject everything else, so a `getQueues` row reporting
`announce_frequency` as `"every 30s"` costs the response. Those reach
`maximum_wait_time`, `maximum_callers`, `max_members` and the `SECONDS_FIELDS`
set. That is the whole remaining exposure, and it is not hypothetical: `GetTransactionHistoryResponseTransaction::ammount` is a strict
`Decimal` on the same row whose `uniqueid` VoIP.ms already reports as the
literal `n/a`. Extending `Reported<T>` to them is the same change made here for
dates, and wants the same thing first -- an observation, or a decision recorded
as one.

The one exception is the envelope's own `status`, which is a required
[`ApiStatus`] on each top-level `*Response`. `Client::fetch` has already
required the field to exist and classified it before a typed call returns, so
handing back a raw string would make the caller parse it a second time to
learn whether it was `success` or an empty-collection code. `ApiStatus` carries
a `Success` variant for that reason -- it is synthesized by the generator
(`SUCCESS_STATUS`), since the docs' error-code table lists only errors. A
nested record's same-named field (a fax's, a port's, an e911 record's) is
unrelated and keeps its inferred type; `response_codegen.rs` distinguishes them
by whether the struct is the method's root.

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

A `new` constructor softens that without changing the shape. `cargo xtask gen`
emits one per `*Params` struct from the `(required)` markers the extractor
mines out of each parameter's description, taking exactly those fields and
leaving every other at its `Default`. Two bounds keep it honest:

* **The markers are the HTML docs' word, not the API's.** The WSDL
  over-declares required-ness (which is why decision 3 exists) and the docs are
  better but not perfect, so the constructor is a convenience and never a
  contract. A field it asks for may be optional in practice, and one it omits
  may turn out to be needed. Where the crate deliberately contradicts a marker
  it needs an entry in `REQUIRED_CTOR_SKIP` (`xtask/src/main.rs`): the offset
  ops' `timezone` is marked required and defaulted to UTC by decision 5a, so a
  constructor demanding it would contradict the method it builds for.
* **A long positional argument list reads worse than a struct literal.**
  `MAX_CTOR_FIELDS` caps it at six; `AddLNPPortParams` (12 required) and
  `CreateVoicemailParams` (11) get no `new` at all.

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
identifier-shaped. A keyword escape is a last resort, not an outcome:
`r#type` at a call site says nothing about what the field holds, so every
`type` field instead gets a descriptive name from `FIELD_IDENT_OVERRIDE`
(`search_type`, `direction`, `file_type`, `code`, …), keyed by struct and
applied on both sides. The same table carries a rename where the upstream WSDL
names one method's id differently than its siblings.

**Type names keep VoIP.ms's acronym casing** (`GetDIDsInfoParams`,
`SendSMSResponse`, `ApiStatus::NoSIPURI`) even though C-CASE and clippy's
`upper_case_acronyms` want `GetDidsInfoParams`. The name then reads the same
here as in the API docs, which is what a reader cross-referencing them needs;
the renaming alternative was weighed in 0.13 and declined. The generated module
carries `#![allow(clippy::upper_case_acronyms)]` so the lint does not fire
against these types in a consumer's own build. Method names are the idiomatic
form regardless (`get_dids_info`, `send_sms`), because the tokenizer produces
them.

**How to apply**: When a new VoIP.ms method introduces an acronym that
produces a single-letter token in `tokenize()`, add it to the `ACRONYMS`
constant in `xtask/src/main.rs` and regenerate. When a new method has a field
named `type`, give it a `FIELD_IDENT_OVERRIDE` entry naming what it holds in
that struct rather than letting it emit as `r#type`.

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
* **Timezones are `chrono_tz::Tz` everywhere**, translated per wire contract.
  The field name `timezone` means two incompatible things upstream, so the
  assignments are per-struct (injected into the `field_type_override` map in
  `main.rs`, since the JSON section is validated enum-only), not name-based:
  - *Named-zone* params (`createVoicemail` / `setVoicemail` / `getTimezones` --
    `field_overrides.rs::NAMED_ZONE_TZ_PARAM_PATHS`) are strict `Tz`, carried
    on the wire via `serialize_opt_tz`. *Named-zone responses*
    (`getVoicemails`, `getTimezones` -- `NAMED_ZONE_TZ_RESPONSE_PATHS`) are the
    tolerant [`crate::TimezoneName`] (`Known(Tz)` or `Unrecognized(String)`)
    via `deserialize_opt_from_wire_text`: voip.ms's live `getTimezones` catalog
    lists legacy names the IANA database has dropped (`Asia/Beijing`,
    `US/Pacific-New`, `Factory`, old Saudi `Riyadh87`/`88`/`89`,
    `Canada/East-Saskatchewan`), and a strict `Tz` failed the whole response on
    the first one -- confirmed by `cargo run -p livetest`, not by the wiremock
    suite (whose fixtures never included a legacy name).
  - *Record-listing* `timezone` params (`getCDR` / `getResellerCDR` / `getSMS`
    / `getMMS` / `getResellerSMS` / `getResellerMMS` -- the `OFFSET_OPS` table
    in `main.rs`) want a number of hours (`-12..=13`), which the WSDL
    under-types inconsistently (`xsd:decimal` on the CDR pair, `xsd:string` on
    the SMS/MMS four). The public field is still `Option<Tz>`; the generator
    emits a private `*ParamsWire` twin plus a `TryFrom<&*Params>` that picks the
    number with `TimezoneOffset::for_window` at the query start date
    (`date_from` / `from`), or at the end date (`date_to` / `to`) when there is
    no start, and routes both generated method bodies through it. A named zone
    with neither date (`TimezoneOffsetError::MissingQueryDate`), a date string
    that does not parse (`ParamsError::InvalidDate`, named zone or not, naming
    the param), or a number outside the range is `Error::InvalidParams` before
    any request is sent. The public
    struct still derives `Serialize` -- there `timezone` emits the IANA name
    (what a log should show); only the wire twin carries the number, so a raw
    `call_raw` caller picks the number itself with `for_window`.

    **The number is not a UTC offset during DST.** VoIP.ms records timestamps
    as [`crate::SERVER_ZONE`] wall clocks (US/Canada Eastern, observing DST)
    and reports each shifted by `timezone + 5` hours, as if that zone were
    always UTC-5; the `date_from` / `date_to` days are matched on the shifted
    value. Measured on 2026-09-23 against a call placed at 16:28:34 UTC, whose
    recording id embeds that Unix time: `timezone` `-12`, `-5`, `-4`, `0`,
    `5.5` and `13` reported `05:28:34`, `12:28:34`, `13:28:34`, `17:28:34`,
    `22:58:34` and `06:28:34` (the next day), and at `13` the call was listed
    under 2026-09-24. So during DST `n` reports UTC+`n+1`. `for_window` sends
    the caller's offset minus the server zone's, less five, so the window is the
    caller's days at the start date. A caller who names no zone gets UTC days.

    That outside DST the same fixed base gives UTC+`n` is **inferred, not
    measured**: every record on the test account was made during DST, and every
    read was made during it. If the shift were different for a winter record or
    a winter read, every winter row would be qualified wrong. The unit tests
    that assert winter values assert this model. Settling it takes reads made
    after 2026-11-01 of a record with a known instant, one made before the
    change (such as the call above) and one made after it.

    A number outside `-12..=13` is refused rather than clamped, so a zone's
    window is either right or an error. During Eastern DST a UTC-12 zone needs
    `-13`, which 0.13 sent as `-12` (and matched an hour off); outside DST
    `Pacific/Kiritimati` (+14) needs `14`.

    Omitting the parameter is not a fallback to some account zone; there is no
    such setting. The portal's account and contact pages have no zone field.
    `getCDR` and `getResellerCDR` answer `invalid_timezone` when it is omitted,
    and `getSMS` / `getMMS` treat an omitted one as `-5`. The reseller SMS/MMS
    pair was not measured: the account has no reseller client to query. The
    crate always sends one because the window is then the caller's.
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
* **Calendar dates** (`date_from`, `date_to`, `reseller_nextbilling` in
  `DATE_FIELDS`) map to [`chrono::NaiveDate`], whose own `Serialize` emits the
  documented `YYYY-MM-DD` wire form. The bare `date` field is excluded -- it is
  a datetime in some responses and a date in others, so no single type fits.
  `getTransactionHistory` is the case where one field holds a point in time and
  a range: [`crate::TransactionDate`] (`TRANSACTION_DATE_RESPONSE_PATHS`,
  assigned per struct for the same reason the timezones are) carries a timestamp
  (`At`), a bare date (`On`), or a range (`Period`, wire `2026-08-01 to
  2026-08-31`), with an `Unrecognized(String)` catch-all. The doc sample shows
  only a timestamp, so the extractor inferred `datetime`, and a live range
  failed the whole envelope -- the same shape of break as the legacy zone names,
  found the same way.

  **The range is the requested window, not a billing period.** The report ends
  with a synthesized row per usage-metered charge -- CNAM queries, communication
  charges -- totaling that charge over the range the caller asked for, and the
  row carries the range itself in place of a timestamp. The customer portal
  shows it directly: a search from 2026-04-01 to 2026-09-22 ends in a
  `CNAM Queries` row dated `2026-04-01 to 2026-09-22`, the search range
  verbatim. The row is synthesized rather than recorded, so it has no
  transaction to name and reports `uniqueid` as the literal `n/a`, which is the
  cheapest way for a consumer to tell it from a real one. Production logs agree:
  four distinct range values inside one four-minute session as the caller varied
  the window, two of them (`2026-08-07 to 2026-08-07`,
  `2026-08-01 to 2026-08-07`) aligning to no billing period at all.

  That is what keeps the type off `getCharges` and `getDeposits`, which are the
  same ledger kept for a reseller client. Neither takes a date range -- `client`
  is their only parameter -- so neither has a window to aggregate over and
  neither can produce the row; both stay `NaiveDate`.

  The reasoning generalizes: a range in a `date` field is a property of a
  *report that totals something over a window*, so the methods to suspect are
  the ones taking `date_from` / `date_to`. Walking that set leaves nothing else
  to fix. The `OFFSET_OPS` six are zoned (decision #8);
  `getCallTranscriptions` and `getVoicemailTranscriptions` report `date` as
  `String` and cannot fail on any value; `getCallRecordings` has no `date`.
  `getConferenceRecordings` and `getVoicemailMessages` are the two that share
  the shape and type `date` as a point in time (`NaiveDateTime`, and
  `WallClock` per decision #8a): each lists individual records and totals
  nothing, so there is no per-charge sum for a window row to carry -- a
  recording and a voicemail each happened at an instant. Revisit that only if
  one of them grows a summary row.

  `On` is a separate variant rather than a midnight `At` because folding a bare
  date into a timestamp would invent a time of day and render it back with one,
  so `Display` would report a precision the wire never carried.
* **Numeric ids the WSDL under-types as strings** (`U64_FIELDS`, plus
  `setConference`'s 20 prompt slots in `CONFERENCE_PROMPT_FIELDS`) map to
  `u64`. This is the class where the two inference sources disagreed
  wholesale: the doc samples gave the `get` side `u64` while the WSDL declared
  the `set`/`create` side `xsd:string`, so a caller who listed a record and
  then updated it converted each field by hand. Only the param side moves --
  every response already reported a number -- so no response gains a way to
  fail. Entries are the *wire* name, which is why `ring_group` is listed twice
  (`delRingGroup` spells it `ringgroup`).
* **Identifiers an all-digit sample made look numeric** (`zip`, `password`,
  `security_code`, `dtmf_digits`, `callerid_prefix` in
  `IDENTIFIER_STRING_FIELDS`) map to `String`, the same reasoning as
  `PHONE_STRING_FIELDS`: a ZIP of `02134`, a PIN of `0123`, a dial string with
  `*` or `#`, and a prefix voip.ms reports as `MIA [555]` all lose information
  as a number. Here the *response* side moves, and only toward tolerance.
* **`pause`** (`DECIMAL_FIELDS`) maps to [`rust_decimal::Decimal`]:
  `setForwarding` documents "0 to 10 in increments of 0.5", which `u64` cannot
  hold and the response already reported as a decimal.
* **`maximum_callers`** joins `maximum_wait_time` in `WAIT_TIME_FIELDS` as
  [`crate::WaitTime`]. The type is selected by its sentinel's wire spelling,
  not by its name: the queue's caller cap is documented "1 to 60 or
  'unlimited'", which is `WaitTime`'s exact wire form, where `Seconds` writes
  `none` and `MaxMembers` a capitalized `Unlimited`.

  Three fields resist this alignment and are recorded in `check-types`'s
  `DELIBERATE` list rather than forced. `client` is `u64` except on
  `getClients` and `getDIDsInfo`, whose parameter is documented to accept an
  e-mail address or a sub-account name as well as the id. `recording` is `u64`
  except on the call-hunting pair, whose response reports the system recording
  name `default`. Both use `field_type_skip` for the exceptions.
  `music_on_hold`'s `volume` is two different things sharing a name: the param
  is the documented `1`/`0` quiet toggle, the response reports the rendition it
  produced (`mp3` / `quietmp3`).

  Four questions the documents could not settle were answered against the
  live API rather than guessed, using `cargo run --example call_raw`:
  `getReportEstimatedHoldTime` really does offer `once` beside `yes`/`no`, so
  `report_hold_time_agent` is the enum and not the `bool` its sample implied;
  a recording slot accepts `none` and `0` interchangeably and always reports
  `0`, so `u64` loses nothing; `volume=1` stores the quiet rendition while `0`
  (or anything else) stores the normal one; and a mailbox created from
  `digits=01` comes back as `1`, so the leading zero the docs' example shows is
  normalized away and `mailbox` is a number rather than an identifier.
* **Declarative enum overrides** in
  `tools/api-response-overrides.json` under the new `enums` (variant
  list with wire strings) and `field_types` (field-name → enum-name)
  sections. The generator emits the enum type (deriving `Debug`, `Clone`,
  `PartialEq`, `Eq` -- not `Copy`, since the `Unknown(String)` catch-all holds
  a `String`), `as_wire` / `from_wire`, `Display`, and substitutes the field's
  type in every `*Params` and `*Response` struct that has that field.

  **Only the serde direction a field reaches it through is emitted**, tracked
  in `EnumSides` while the structs render: `Serialize` for an enum some
  `*Params` writes, `Deserialize` for one some `*Response` reads, both for the
  15 that are both. A response field names no per-enum helper: it reads through
  the generic `deserialize_opt_from_wire_text::<Enum, _>`, the same reader
  `TimezoneName` uses, and the seconds-or-sentinel types go through
  `deserialize_opt_via::<T, _>` the same way. The same rule retired `ApiStatus`'s `Serialize`,
  `TimezoneName`'s `Serialize`, and `TimezoneOffset`'s `Deserialize`. Used
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
stale entries. It reads three spellings (`yes/no`, `Boolean: 1/0`, and a
`1=`/`0=` value list); the second was added after `cnam` and `sip_traffic`
stayed integers through an audit that reported "ok", because the bare form
names no value for the `1=`/`0=` rule to match. For a scalar
that needs structured parsing (multi-part value, custom validation),
hand-write it in `src/types.rs`, register the field names in
`xtask/src/field_overrides.rs::ROUTING_FIELDS`-style const, and add the
deserializer to `src/responses.rs`.

`cargo xtask check-types` is the complementary audit: it parses
`src/generated.rs` and reports a field a method family types one way on the
`set`/`create` side and another on the `get` side. Both audits print and exit
zero by default, since a finding wants human judgment about which of the two
types is right; both take `--deny`, which turns a finding into a non-zero exit,
and CI runs them that way. So "it reports nothing" is a rule rather than a
note here. A pair whose two types are meant to differ goes in its `DELIBERATE`
list with the reason, which is the record of *why* rather than a way to
silence it. A
collection type is excluded from the comparison: a root response's payload
list often shares its name with the record id it holds
(`GetDISAsResponse::disa` against `SetDISAParams::disa`), and no override
could make those one type.

### 6. No HTTP-level retry, no auth caching, no rate limiting

**Decision**: `Client::call_raw` is one request, one JSON parse, one status
check. There is no built-in retry, backoff, or rate limiter.

**Rationale**: VoIP.ms's retry semantics depend heavily on which method
you're calling (`addCharge` is not safely retryable; `getBalance` is).
Baking in a retry policy would force the wrong default on someone. Users
who want one can wrap their `Client` in `tower::retry` or compose any
middleware via a custom `reqwest::Client` passed to `Client::builder`.

### 7. GET, except a method carrying a file

**Decision**: A call is a GET with query parameters, unless its parameters
include a base64-encoded file, in which case it is a `multipart/form-data`
POST carrying every parameter -- credentials and method name included -- as a
form field. Four methods qualify: `setRecording`, `sendFaxMessage`, `sendMMS`
(`media2`), and `addLNPFile`. `Client` decides per method from the generated
surface; a caller does not choose, and no parameter or builder option
overrides it. The one exception is `Client::call_multipart_raw` (and its
`_unchecked` twin), for an upload method this crate has not been regenerated
for and so cannot recognize.

**Rationale**: VoIP.ms documents and accepts both, but every documented example
is GET, and GET keeps the request observable in logs and proxies during
development. That observability is worth keeping for the 218 methods that can
have it, which is why the transport is targeted rather than switched wholesale.

A file parameter cannot have it. VoIP.ms's front end caps the request line at
8190 bytes (Apache's default `LimitRequestLine`): measured against
`rest.php`, a request line of about 8182 bytes answers 200 and one of about
8187 answers 414. After the credentials and the method name that leaves roughly
8 kB for the percent-encoded parameters, about a third of a second of 8 kHz
mono 16-bit audio -- a 2.8 second greeting is 60,428 base64 characters, more
than seven times over. `addLNPFile` is documented "Only accepted through POST
request", so for it no size works over GET. The risk this decision originally
weighed, many small parameters on `createSubAccount` / `setSubAccount` /
`setQueue`, really is mitigated by most of them being `None` (decision #3); one
large parameter is not, and that is the case the original reasoning did not
cover.

The POST has to be `multipart/form-data`. `rest.php` hands an
`application/x-www-form-urlencoded` POST to a SOAP handler, which answers with
an XML fault -- the trap that makes the API look GET-only on a first test.
Multipart is accepted at 100 kB and at 8 MB, and a read-only `getBalance` over
multipart returns `success`, so the API does not restrict the transport to
upload methods; this crate restricts it to the methods that need it.

Which methods those are is derived from the parameters, not from a second
document. `tools/server.wsdl` (decision #1) does not record `addLNPFile`'s
POST-only requirement -- that lives only in the HTML docs -- but every method
that needs POST has a base64 file parameter and every method with one needs
POST, so the parameter carries the whole rule. The four paths are listed in
`BASE64_FILE_PARAM_PATHS` (`xtask/src/field_overrides.rs`), from which the
generator emits `requires_multipart`. Every generated method calls `call` /
`call_raw`, and those (with `call_raw_unchecked` and the private `call_zoned`)
read the transport from `requires_multipart` by wire name, so the choice lives
in one place and a caller dispatching by name gets the same answer
as a generated method. `call_multipart_raw` is public for the same reason
`call_raw` is: a method this crate hasn't been regenerated for still needs a way
to be called, and for an upload method that way has to be a POST.

**How to apply**: When a new method takes a base64 file parameter, add its
`"wireMethod.field"` path to `BASE64_FILE_PARAM_PATHS` and regenerate.
`cargo xtask gen` has three outcomes for that table, all covered by unit tests
in `xtask/src/main.rs`:

* it **fails** on an entry naming a parameter the WSDL does not declare, since
  a path left behind by a docs revision would drop the method back onto a GET
  without a word;
* it **fails** on an entry naming an op that is also in `OFFSET_OPS`. An offset
  op sends its `*ParamsWire` twin, which has only ever gone out over GET; listing
  it would move that twin onto a multipart body without a word. Nothing overlaps
  today, and reconciling the wire twin with a multipart body is unexamined work,
  so the generator refuses rather than guesses;
* it **warns** when a parameter the docs describe as base64 is absent from the
  table. That is the tripwire for a fifth method appearing in a docs refresh;
  it warns rather than fails because the reading comes from mined HTML and
  needs a human to confirm the parameter really carries a file.

### 8. Record-listing timestamps are qualified by undoing the server's shift

**Decision**: The six methods that take a `timezone` number (decision #5a's
`OFFSET_OPS`) type their response timestamp as `Reported<WallClock>`. The typed
method sends the number `TimezoneOffset::for_window` picks, then, before
deserializing, moves each reported wall clock back by `timezone + 5` hours and
resolves it in [`crate::SERVER_ZONE`], via `Client::call_zoned` and the public
`attach_offset`. A value comes back `WallClock::Zoned` with the offset the
server zone was at then (`-04:00` or `-05:00`).

**Rationale**: VoIP.ms reports these timestamps as server-zone wall clocks
shifted by a fixed `timezone + 5` hours (decision #5a), so the number sent is
not the offset they are in. 0.13 attached the number sent as the offset, which
is an hour wrong for every row recorded during DST: the call at 16:28:34 UTC
read back at `timezone=0` as `17:28:34+00:00`. One downstream consumer had
already read an unqualified `2026-09-18T14:44:11` as UTC and reported a
registration time that had passed, which is what the typing exists to prevent.

The shift is fixed arithmetic, so it can be undone exactly for any number sent,
including a fractional one (`5.5` reports `22:58:34` for that call). What
remains is the server-zone wall clock, and resolving it per row gives each row
the offset in force at its own instant, so a range that crosses a DST change
comes back right on both sides. The one wall clock that cannot be resolved is
the hour the server zone repeats when clocks fall back. `attach_offset` writes
that value as its moved-back server-zone wall clock followed by the zone's name
(`2026-11-01 01:30:00 America/Toronto`), and it reads as `WallClock::Bare` --
which is why the type is `WallClock` rather than `DateTime<FixedOffset>`. It is
written on the server clock rather than left shifted so that every row's
`.local()` is on the same clock; left shifted, it would sort five hours away
from its neighbors at `timezone=0`.

The zone name is also what keeps the guard. `deserialize_opt_record_listing_timestamp`
reads a value with an offset as `Zoned` and one ending in the server zone's
name as `Bare`. It refuses a `YYYY-MM-DD HH:MM:SS` wall clock with neither:
that is a shifted wall clock that was never qualified, which a raw caller who
skipped `attach_offset` would otherwise read as if it meant something. Any other
text is an odd value that `attach_offset` left as it arrived, and it degrades to
`Reported::Unreadable`, so one malformed date costs its own row and not the
envelope. The guard cannot catch `attach_offset` given a number other than the
one sent.

Qualifying is a step on the JSON, not a `Deserialize` impl: serde has no access
to the request, so it cannot know the number sent. `attach_offset` is public
because a `call_raw` caller needs the same step (`livetest`'s `probe_zoned` is
one), and it takes the `TimezoneOffset` sent rather than an offset to attach.
Each method's paths reach a caller through `offset_timestamps(method)`, emitted
by the same codegen pass that retypes the fields -- a raw caller reading them
out of a generated method body would be copying something that moves with the
response shape. It is the one public route: the per-method `*_TIMESTAMPS`
consts the typed methods pass to `call_zoned` are private, so there is no
second spelling of the same paths to keep in step. It is a lookup and not a
step inside `call_raw`: the raw calls return exactly what VoIP.ms sent, and a
qualified envelope would no longer be that.

`attach_offset` leaves a blank value alone. A blank is one record's missing
timestamp, which the deserializers fold to `None`; rewriting it would produce a
string that parses as nothing, and since one unparseable value fails the whole
envelope, that would turn a single missing timestamp into the loss of every
record beside it.

**The live check is instant equality across numbers.** The `cdr` area's
`fixture:getCDR:<zone>` reads one window at UTC and at a named zone (a
whole-hour and a half-hour one) through `Client::get_cdr` and fails if a
record's instant moves. It passed on 2026-09-23. It cannot catch a shift that is
wrong by the same amount for every number -- 0.13's error was exactly that --
so the absolute reference is the recorded call: typed `get_cdr` reports it as
`12:28:34-04:00`.

The other `NaiveDateTime` response fields are typed through decision #8a
where they were measured. Six are recorded in the server zone and typed as
`WallClock`, and three stay `NaiveDateTime` because nothing independent of the
API could date them.

**How to apply**: `cargo xtask gen` derives the fields from the response shapes:
every `datetime` scalar under an `OFFSET_OPS` method is retyped and its path
emitted as that method's `*_TIMESTAMPS` const and `offset_timestamps` arm. Three things fail the run rather
than degrade quietly, because each would leave a field silently naive while the
build stayed green:

* an offset op whose response declares no timestamp at all (a docs refresh that
  dropped the field);
* a `datetime` the walk cannot name, which is any of: a bare one as a list
  element or map value, since a path ends at a field name; one inside a map,
  whose values `attach_offset`'s `*` does not reach, because over an object `*`
  already means the bare single record VoIP.ms sends for a one-element list; and
  one under a collection nested directly inside another, where the path
  `attach_offset` would walk and the struct the emitter wraps it in stop
  agreeing. Each needs the path form, the emitter, or both extended first, which
  is a decision rather than a default. The guards ask whether the shape holds a
  timestamp before refusing, so a nested list of strings is not an error;
* a missing response shape for an op that sends an offset.

Note the asymmetry the second case fixes: the "no timestamp at all" check only
fires when *every* timestamp is missed, so a response that grows a second
timestamp somewhere unaddressable would otherwise pass.

A response that is not a record is not in that set. `emit_struct` promotes one
into a one-field record (`value`, `items`, `entries`), `timestamp_fields`
synthesizes the same field, and that field goes through the override table like
any other.

All three have to agree on the *name*, not just on the shape, and twice now they
have not: a promoted `value` the emitter typed from the raw shape because its
arm never consulted the resolver, and an `items` element the walk singularized
to `Item` while the emitter called it `Items`. Both emitted a bare
`NaiveDateTime` under a path `call_zoned` rewrites, so every typed call on that
method failed at runtime on a generator run that reported success -- quieter, and
so worse, than the refusal each replaced. Whatever names an element struct must
be one function called from both sides (`element_type_name`), and the tests
assert the emitted type rather than the walk's return value, because the walk
agreeing with itself is exactly what both failures looked like.

Naming the element through `element_type_name` costs one shape: a response that
is a list of lists. `emit_struct` hands the whole list to `field_type`, so the
inner list becomes an element struct of its own (`{ items: Vec<T> }`) where the
wire has a bare array, and serde rejects it. That shape is unsupported by
construction now rather than by accident, which is the trade that makes the root
case agree with the field case. Nothing returns one -- `src/generated.rs` has no
`pub items:` field at all, so no response is a top-level list -- and a docs
refresh that produced one would need both the emitter and the walk taught about
it together.

### 8a. Timestamps in a named zone are qualified in that zone

**Decision**: A response timestamp that is a wall clock in a known named zone
is `Option<Reported<WallClock>>`, where [`crate::WallClock`] is
`Zoned(DateTime<FixedOffset>)` or `Bare(NaiveDateTime)`. The `ZONE_OPS` table in
`xtask/src/main.rs` names each method and where its zone comes from
(`ZoneSource`):

* **`Server`** -- [`crate::SERVER_ZONE`]. The plain typed method qualifies
  each value itself through `Client::call_in_zone`: `getRegistrationStatus`
  (`register_next`), `getCallRecordings` / `getCallRecording` (`datetime`),
  `getDIDsInfo` (`order_date`), `getFaxMessages` (`date`) and `getMediaMMS`
  (`date`).
* **`Supplied`** -- a zone the caller holds. `getVoicemailMessages`' `date` is
  in the mailbox's own `timezone`. The plain `get_voicemail_messages` leaves
  every value bare, and `get_voicemail_messages_in_zone(params, tz)` qualifies
  them.

A raw caller does the same with the public `attach_zone` over the paths
`zone_timestamps(method)` answers, which also names the source
([`crate::ZoneTimestamps`], [`crate::TimestampZone`]). Both are emitted from
`ZONE_OPS`, the same way `offset_timestamps` is emitted from `OFFSET_OPS`.

**Rationale (server zone)**: each `Server` method was measured on 2026-09-23,
during DST, against an instant taken independently of the API, and each
reported the Eastern wall clock at UTC-04:00:

| Method | Reference | Reported |
|---|---|---|
| `getRegistrationStatus` | baresip registered at 14:40:30 UTC with a 600 s interval | `register_next` `10:50:30` |
| `getCallRecordings`, `getCallRecording` | call at 16:28:34 UTC; the recording id embeds Unix time `1790180914` | `datetime` `12:28:34` |
| `getDIDsInfo` | DID ordered between 17:19:00 and 17:19:32 UTC | `order_date` `13:19:31` |
| `getFaxMessages` | fax sent between 17:24:15 and 17:24:33 UTC | `date` `13:24:33` |
| `getMediaMMS` | MMS sent between 17:20:47 and 17:21:40 UTC | `date` `13:21:39` |

A fixed UTC-5 would have read an hour earlier in every row. The probe DID and
fax number were canceled afterwards, and the DID the call went to had
`record_calls` switched on for the call and back off.

Three fields stay `NaiveDateTime`, unmeasured, because no reference could be
created without an effect the API cannot undo:

* `GetBackOrdersResponseBackOrder::order_date` -- a back order cannot be
  canceled through the API and may later buy a number.
* `GetLNPDetailsResponse::date` -- a port request is a real filing with the
  losing carrier.
* `GetConferenceRecordingsResponseRecording::date` -- no API call turns on
  conference recording, and no documented routing value dials into a
  conference.

Every measured table agreed, so these are likely `SERVER_ZONE` as well; adding
one to `ZONE_OPS` wants a measurement, not the pattern.

**Rationale (mailbox zone)**: Measured against the live API on 2026-09-22 and
2026-09-23 with the test account and `examples/call_raw.rs`:

* The response names no zone. Every message carries exactly `callerid`, `date`,
  `duration`, `folder`, `listened`, `mailbox`, `message_num` and `urgent`, and
  the request takes only `mailbox`, `folder`, `date_from` and `date_to`, so it
  cannot choose one either.
* `date` is the stored instant, rendered in the mailbox's *current* `timezone`
  setting at the time it is read. Mailbox 101's message 17 read `12:47:40`
  while the mailbox was set to `Pacific/Honolulu` and `18:47:40` after it went
  back to `America/Toronto`. Message 14 read `2026-08-24 21:25:11` under
  Toronto and `2026-08-25 03:25:11` under `Europe/Berlin`. So a caller holding
  the mailbox's current zone resolves every row correctly, including rows
  recorded under an earlier setting.
* `date_from` / `date_to` are **not** matched in the mailbox's zone. Under
  Toronto, message 14 (01:25 UTC on 2026-08-25) matched `2026-08-24` and not
  `2026-08-25`, so the window is not UTC. Under Berlin the same message read
  `2026-08-25 03:25:11` and still matched `2026-08-24` only. So the window is
  Eastern days. No message on the account fell between 04:00 and 05:00 UTC
  during DST, the one hour that separates `SERVER_ZONE` days from fixed UTC-5
  ones, so which of the two it is was not measured. The finding is written on
  both params through `ZoneOp::param_notes`, and the run fails if an entry
  names a param the WSDL does not declare.

Each row is resolved at its own instant, so rows on either side of a DST change
get different offsets. A wall clock the zone repeats when clocks fall back is
ambiguous, and one it skips when they spring forward should not occur. Both read
as `Bare`, because choosing a side would be a guess, and so does one at an
offset with a seconds part (a zone's pre-standard local mean time), which the
wire spelling cannot carry since it stops at minutes. A value that already names
an offset (`names_offset`), a blank, and text that is not a
`YYYY-MM-DD HH:MM:SS` wall clock are left as they arrived.

`attach_zone` and `attach_offset` share one walk. Each only resolves a value --
`attach_offset` after moving it back by the shift -- and the walk rewrites it in
place, trimmed: as the instant with its offset, or, for a wall clock with no
single offset, as that wall clock followed by the zone's name
(`2026-11-01 01:30:00 America/Toronto`). Both helpers write that one form, so a
server-zone field and a record-listing field agree on the unresolvable hour, and
both kinds of field read it as `Bare`. A named-zone field also reads the plain
spelling, which is what an unqualified call leaves.

`WallClock`'s equality is hand-written. `DateTime<FixedOffset>`'s own `==`
compares instants and ignores the offset, so a derived one would call
`18:47:40-04:00` and `22:47:40+00:00` equal while they report different wall
clocks, and a test could not catch `attach_zone` choosing the wrong offset.

A named-zone field accepts a bare value rather than refusing it, unlike a
record-listing field. A bare value there comes from a call that supplied no zone
or from a row left unresolved, and is the wall clock in the zone it was
rendered in, so it is part of the field's contract rather than a break in it.
`Bare` claims no zone where reading it as UTC would invent one. Unreadable text
still degrades to `Reported::Unreadable`.

The crate still makes one request per method call. Looking the mailbox zone up
with `getVoicemails` inside `get_voicemail_messages_in_zone` would double the
requests, and a failed `getVoicemails` would then fail a message read. The
caller can fetch the mailbox once for many reads.

**How to apply**: A new method whose timestamps are wall clocks in a named zone
goes in `ZONE_OPS` with its `ZoneSource`: `Server` only once an independent
reference instant has been compared against it during DST, `Supplied` with a
doc fragment naming where the caller reads the zone. `cargo xtask gen` fails
when a zone op is also an offset op, when it takes no parameters (the template
for those qualifies nothing), when its response has no timestamp, or when a
`param_notes` entry names an undeclared param. livetest's `probe` qualifies a
`Server` method on its own, and the `voicemail` area probes
`getVoicemailMessages` against the mailboxes `getVoicemails` reports, qualified
in each mailbox's zone, until one returns a message, so the key diff has rows to
compare or the method is skipped.

## Code Patterns

### Calling the wire API

The private `Client::send` is the single point that hits the network; it takes
the transport (decision #7) and returns the parsed envelope, and `Client::fetch`
adds the status classification on top. The public `call` and `call_raw` (and
`call_raw_unchecked`, behind the `unchecked-raw` feature) take the transport
from `Transport::for_method`, which reads the generated
`requires_multipart(method)`; the private `raw` helper holds the body
`call_raw` and `call_multipart_raw` share. `call_multipart_raw` and `call_multipart_raw_unchecked`
are the single explicit escape hatch, for an upload method this crate has not
been regenerated for, which `requires_multipart` answers `false` for. A caller
dispatching by wire name calls `call_raw` like a generated method does;
`requires_multipart(method)` answers the transport question without making the
call, `offset_timestamps(method)` names the timestamps the caller then
completes with `attach_offset` (decision #8), and `zone_timestamps(method)` the
ones it qualifies with `attach_zone` (decision #8a). Every generated method is a thin
wrapper over `call` or `call_raw`, whatever its transport:

```rust
pub async fn get_balance(&self, params: &GetBalanceParams) -> Result<GetBalanceResponse> {
  self.call("getBalance", params).await
}

pub async fn set_recording(&self, params: &SetRecordingParams) -> Result<SetRecordingResponse> {
  self.call("setRecording", params).await
}
```

Both transports render their fields through one serializer (`src/form.rs`), so
they differ in where a value rides and never in how it is written. That
serializer matches `serde_urlencoded`, which is what `reqwest`'s `.query()`
applies to a parameter set, and its tests assert the agreement value by value --
including the one arm where `Display` would diverge, a whole float, which the
query string renders `1.0` and `to_string` renders `1`. The multipart form was
originally built by reading back the query string the GET form produced, which
bought the same parity at the cost of encoding and decoding the whole payload:
roughly 30 MB of transient allocation for an 8 MB fax.

A parameter with no field rendering -- a nested value, or anything serde reports
as other than a scalar -- is `Error::InvalidParams(ParamsError::Unencodable)`
naming the parameter, and nothing is sent. Neither transport can carry one, so
this is not a property of the transport that happened to be chosen.

**No wire form rests on a `Display` impl that could change.** A wire form is a
contract with VoIP.ms; `Display` is free to render for a person, and in this
crate it does -- `Error` prints `API status: did_in_use (DID Number is already
in use)` where `ApiStatus` prints `did_in_use`, and `TransportFailure` has no
`Display` at all for the same reason (see "Transport-failure classification").
The risk is a wire form that is only *incidentally* a `Display` impl, so that
making the rendering friendlier for a log silently changes what is sent.

That rules out borrowing a *type's* rendering, not `to_string` as such. In
`form.rs` the integer arms use it, since a primitive integer's `Display` is its
decimal digits by specification and cannot drift; the float arms do not, since a
float's is fixed to something else (`1` for `1.0`, never an exponent) and so go
through `ryu`. `Routing` states its wire form as `Routing::to_wire`, which
`Serialize` calls and `FromStr` inverts, leaving its `Display` free to change --
the two render the same text today and only the first is a contract.
`TimezoneOffset` still serializes through `collect_str` over `rust_decimal`'s
`Display`, whose scale is load-bearing (`Asia/Kolkata` sends `5.50`).

**How to apply**: give a domain type that goes on the wire an explicit wire form
(`to_wire` / `as_wire`) and have `Serialize` call that. If a type's only route
to text is its `Display`, treat that as the signal to add one rather than to
borrow the rendering meant for a reader. `Display` stays in error messages, the
`Debug` impls, and log lines.

If a regeneration drift is ever needed (e.g. a method needs custom
encoding), break that one method out of the codegen with an explicit
skip-list and hand-write it in `src/client.rs`. Do not pollute
`generated.rs` with special cases.

### Error surfacing

Four variants, no more:

* `Error::Http` -- wraps `reqwest::Error`. Includes both transport-level
  failures and `error_for_status`'s non-2xx surfacing.
* `Error::Api(ApiStatus)` -- the response parsed as `{ "status": "..." }`
  with something other than `"success"`. `ApiStatus` is a generated enum
  with one PascalCase variant per documented code (~475 of them) plus an
  `Unknown(String)` catch-all, so a code VoIP.ms returns but hasn't
  documented is preserved verbatim rather than lost -- the variant set is
  documentation, not a closed contract. `ApiStatus::from_wire` /
  `as_wire` round-trip the wire string, `description()` returns the
  documented meaning (`None` for `Unknown`), and `is_documented()`
  reports whether it's a known variant. The enum, its impls, and the
  description table are emitted by `cargo xtask gen` from
  `tools/api-statuses.json`, which is extracted from the docs' global
  "Error Codes" table via `cargo xtask extract-statuses <html>`. Because
  the docs ship a couple of codes capitalized (`Invalid_threshold`), the
  variant's `as_wire` preserves the wire casing while the variant
  *identifier* normalizes through the same acronym-aware PascalCase as
  method/type names (`no_did` → `NoDID`). `Display` on the *error* renders the
  code with its documented meaning (`API status: did_in_use (DID Number is
  already in use)`) so a log line says what went wrong; `Display` on
  `ApiStatus` itself stays the bare wire string, matching `as_wire`.

  `ApiStatus::Success` is the one variant the table does not supply. It is
  synthesized by the generator, because a typed response's `status` field
  (decision 2) reports it and would otherwise land in `Unknown("success")`.
  `load_statuses` fails the run if a docs refresh starts listing `success`, so
  the variant cannot be emitted twice.

  **Empty-collection statuses are not errors for typed calls.** VoIP.ms
  returns a distinct `no_*` status per list method when the list is empty
  (`no_sms`, `no_cdr`, `no_messages`, …). The typed `Client::call` (and so
  every unsuffixed generated method) folds any status for which
  `ApiStatus::is_empty_collection()` is true into a successful data-less response
  -- collection fields deserialize to `None` -- instead of `Error::Api`. The
  `*_raw` methods (and `call_raw`) deliberately keep the strict verbatim
  contract: they still surface an empty status as `Error::Api`, so the raw
  escape hatch reflects exactly what VoIP.ms returned. `check_status` in
  `src/client.rs` classifies the status; the two paths diverge in
  `call_raw` vs `call`. The classification is hand-curated in the
  `empty_statuses` array of `tools/api-response-overrides.json` and emitted
  into `ApiStatus::is_empty_collection()` by `cargo xtask gen`; codes that look like
  `no_*` but signal a real failure (`no_base64file`, `no_callstatus`,
  `no_change_billingtype`, `no_provision`, `no_provision_update`,
  `no_sequences`) are deliberately excluded. To reclassify, edit that array
  and regenerate -- an entry naming a status code absent from
  `tools/api-statuses.json` fails the codegen.
* `Error::InvalidResponse(String)` -- the response was 2xx and JSON but
  didn't contain a `status` field. Should be rare; if it happens
  systematically for a method, that's a VoIP.ms-side break.
* `Error::InvalidParams(ParamsError)` -- the parameters could not be
  converted to their wire form, so no request was sent. `ParamsError` names
  which check failed: `Timezone(TimezoneOffsetError)` (see 5a's record-listing
  offsets), `InvalidDate { param, value }` (a record-listing `from` / `to`
  that is not a date, checked whether or not a zone is set), and
  `Unencodable(String)`, a parameter with no wire-field rendering (decision
  #7). The inner enum exists so the next parameter
  validation is additive: the variant name is general and a specific payload
  would have forced either a second `Error` variant or a breaking change. The
  timezone hop carries `#[from]`, so a generated `TryFrom<&*Params>` returning a
  `TimezoneOffsetError` still reaches `Error` through one `?`.

### Transport-failure classification

**Decision**: `Error::transport()` reduces a failure to a `TransportFailure`
(`Rejected(StatusCode)` / `Timeout` / `Dns` / `Connect` / `Body` / `Other`),
and `TransportFailure` answers two questions about it:
`never_reached_upstream()` (could account state have changed) and
`retry_outlook()` (is repeating the identical call worth anything, as a
`RetryOutlook`). Neither type implements `Display`.

**Rationale**: This is knowledge about HTTP and about this API, not about any
consumer's product, and every consumer of the crate was re-deriving it from
`Error::Http`'s inner `reqwest::Error` -- one by hand-copying another's
implementation, which drifted within a review cycle with nothing to catch it.
Two distinctions only this crate knows are the ones consumers got wrong: an
allow-list rejection is `Api(ApiStatus::IPNotEnabled)` on a 200 rather than an
HTTP 403, so it is not a transport failure at all; and reqwest reports a
resolution failure through the connect error that wraps it, so `is_dns` must be
read before `is_connect`. The two questions stay separate because a refusal
answers them differently -- a stale proxy credential answering 401 changed no
state *and* is futile to repeat, and collapsing them produced a retry loop. That
split is also why 408 is carved out of `never_reached_upstream`: RFC 9110 defines
it for an incomplete request, but an intermediary returning it for a slow
response is a 504 in 408's clothing, so the state claim is unprovable while
`AfterWaiting` still reads right.

Rendering is deliberately excluded: a model reading a retry decision and a
person reading a terminal diagnostic want different sentences, so a `Display`
here would pull that judgment into the wrong crate. This sits alongside
decision #6 -- the crate classifies, it still does not retry.

**How to apply**: A new `TransportFailure` variant must be added to the arms of
both `never_reached_upstream` and `retry_outlook` (neither uses a wildcard) and
to the per-variant tests in `src/error.rs`, so it decides rather than inherits.
Classification order is part of the contract; the tests pin it, including that
a DNS failure does not read as `Connect`.

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
│       ├── rust-ci.yaml              # fmt, clippy, TLS check, test, coverage
│       ├── dependabot-automerge.yaml # auto-merge safe Cargo updates
│       └── release.yaml              # tag-validated publish + GitHub release
├── src/
│   ├── lib.rs           # Module surface; re-exports generated.rs
│   ├── client.rs        # Client, ClientBuilder, call()
│   ├── error.rs         # Error, ApiStatus, Result, TransportFailure
│   ├── form.rs          # Params -> the wire fields both transports carry
│   ├── generated.rs     # 222 *Params + Client methods + *Response (generated)
│   ├── responses.rs     # Custom serde deserializers for generated.rs
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
