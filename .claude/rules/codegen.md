---
paths:
  - "xtask/**"
  - "tools/**"
  - "tests/generated_*.rs"
  - "livetest/src/response_fields.rs"
---

# Code generation (WSDL, response shapes, naming)

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

The WSDL's parameter *names* are not authoritative either: it lags the portal
docs, which list dozens of parameters it lacks and name a few differently.
`CORRECTIONS` in `xtask/src/wsdl.rs` renames or adds a WSDL input, applied as
the WSDL is parsed so every xtask command sees the same surface. A
disagreement with the docs alone does not earn an entry -- the docs have their
own copy-paste errors (`getResellerMMS` documents its MMS id as `sms`). An entry
needs a second source: voip.ms's PHP client in its public `API.zip`, which posts
each parameter under an explicit key without reading the WSDL, or a live call.
An entry that stops fitting the WSDL fails the parse, with a message naming
the fix (DEVELOPMENT.md lists the cases).

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
   PHP dialect) or gets the structure of wrong (`getTerminationRates`'s
   `route` is a list the docs print as one object).

Every correction lives in the overrides file, never in
`api-responses.json`: the extract rewrites that file whole, so an edit
there is lost on the next docs refresh. The extract records a `digest`
of its output and `cargo xtask gen` fails when the file no longer
matches it, which turns a hand edit into a CI failure rather than a
regression a later refresh introduces.

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
respectively. The docs sometimes lose the line break before a row, gluing it
onto the previous description (`getResellerSMS`'s `to` ends `- Default value:
Todayclient => [Required] ...`); the extractor splits such a row off when the
word before its arrow ends with an input the method declares and has not
listed yet, so the row's `(required)` marker reaches its own field.

**A param doc describes the field a caller writes, in the field's terms.** The
mined text is voip.ms's, written for its wire form, so the generator corrects
it in three places rather than render it verbatim:

* The paragraph some methods print flush left under their parameter table
  (the routing-string format, the `members` grammar) is about several
  parameters, so the extractor keeps it apart in `param_notes` instead of
  running it into the last row, where it landed on `test` for the DID orders.
  The routing format is written by the generator (`ROUTING_FORMAT_DOC`) on the
  first routing field of every params struct -- its `routing` when it has
  one -- and the struct's other routing fields refer to that field. The docs
  print the format in three versions of differing completeness and on no
  method that takes `routing_match`, so one fixed text is the only way every
  routing field gets the whole of it.
* A `bool` flag's description loses the wire spellings of its values
  (`(Boolean: 1/0)`, `(Values: 'yes'/'no')`) and says a default as
  `true` / `false`, since the field takes neither `1` nor `yes`.
* `param_docs` in `tools/api-response-overrides.json` replaces a description
  outright: one the generic rules leave in wire terms, one the docs word
  differently for the same field on two methods (`ip_restriction`), or one a
  note has to be worked into. It names other params by their Rust ident.
  `cargo xtask gen` warns about a note no field renders and about a flag
  description that still spells a wire value.

The same text, before rustdoc wrapping and escaping, is written to
`tools/param-docs.json` (keyed by wire method, then wire field, under
`param_docs`), so a consumer that reads descriptions from JSON rather than
rustdoc gets the corrected text too. Each entry is the `doc` and whether a
caller must set the field (`required`), as data because an override can
replace the text that carried the `(required)` marker. Both are in the crate's
terms: a `bool` flag says `true` / `false`, and the offset ops' `timezone` is
not required although voip.ms needs it on the wire, since the typed method
sends UTC in its place. Only `cargo xtask gen`
writes it; the mined `param_docs` in `tools/api-responses.json` stay the
docs' own text.

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
[DEVELOPMENT.md](../../DEVELOPMENT.md#regenerating-the-api-surface) — that is
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
  it needs an entry in `REQUIRED_OVERRULED` (`xtask/src/main.rs`): the offset
  ops' `timezone` is marked required and defaulted to UTC by decision 5a, so a
  constructor demanding it would contradict the method it builds for.
  `getLNPList`'s `portid` carries a marker pasted from `getLNPDetails`, and a
  live call without it lists the ports (`no_ports` on an account with none)
  where `getLNPDetails` answers `invalid_portingid`. The same rule
  (`is_required`) sets `required` in `tools/param-docs.json`, and a
  `param_docs` override must carry `(required)` exactly when it is true.
* **A long positional argument list reads worse than a struct literal.**
  `MAX_CTOR_FIELDS` caps it at six; `AddLNPPortParams` (12 required) and
  `CreateVoicemailParams` (11) get no `new` at all.

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

**An ident follows one naming scheme; the wire name does not have to.**
voip.ms names one concept several ways (`firstname` / `firstName`, `zip` /
`zip_code`, `email_enable` / `email_enabled`, `callhunting` beside
`ring_group`) and misspells some (`announce_position_frecuency`). A caller
writing a struct literal, and a consumer that uses the idents as parameter
names, should not have to know which spelling one method picked, so
`FIELD_IDENT_RENAME` in `xtask/src/main.rs` maps a wire name to the ident in
every struct, params and response alike:

* every word is split (`callhunting` -> `call_hunting`, `ratecenter` ->
  `rate_center`), including the `id` of a record id (`portid` -> `port_id`);
* an on/off switch ends in `_enabled`;
* one spelling per concept: `first_name` / `last_name`, `zip_code`,
  `failover_*`, `toll_free_*` for `tf*`, `caller_id` for `callerid`,
  `unavailable` for `unavail`;
* a misspelling is corrected.

The wire name still travels in the serde `rename`, so nothing changes on the
wire. Renaming an ident breaks every caller that names it, so a new entry for
a field that has shipped waits for a breaking release.
`cargo xtask gen` fails on an entry whose wire name no struct carries any more
or that names the ident the tokenizer derives anyway. A `FIELD_IDENT_OVERRIDE`
entry wins over the table for its one struct.

Type names are not renamed: `GetTimeConditionsParams` keeps voip.ms's word for
the method, and a nested response struct keeps the wire name of its field
(`GetVoicemailSetupsResponseVoicemailsetup`).

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
When a new field's wire name breaks the scheme above, add it to
`FIELD_IDENT_RENAME`.
