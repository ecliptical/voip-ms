# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** integer request params are `u64` (was `i64`), matching the
  response side. Every VoIP.ms integer param is a non-negative id or count
  (the documented `-1` sentinels are enum-typed), and ids read from responses
  are `u64`, so the old `i64` forced a cast on every get/set round-trip.
- **Breaking:** decimal request params are `rust_decimal::Decimal` (was
  `f64`). The affected params are money amounts (`charge`, `payment`,
  `setup`, `monthly`, `minute`) plus `timezone`; `Decimal` serializes the
  exact value, where `f64` could ship float artifacts on the two methods
  that move money.
- **Breaking:** `date_from` / `date_to` params are `chrono::NaiveDate` (was
  `String`); its `Serialize` emits the documented `YYYY-MM-DD` wire form.
  The crate's `chrono` dependency gains the `serde` feature for this.
- **Breaking:** generated field identifiers are snake_case. Wire names that
  are camelCase (`isMobile`, `rateCenter`, `emailToFax`) or run-together
  acronyms (`sipuri`) become idiomatic idents (`is_mobile`, `rate_center`,
  `email_to_fax`, `sip_uri`) with a serde `rename` preserving the wire form
  on both the params and response side; `#![allow(non_snake_case)]` is gone
  from the generated module. Params and responses now share one
  keyword-escaping ident helper, closing a latent gap where a param named
  `match` or `ref` would have emitted invalid Rust.
- **Breaking:** `addLNPPort`'s `locationType` param and `getLNPDetails`'s
  matching response field are the new `LocationType` enum
  (`Residential`/`Business`, wire `0`/`1`) instead of a bare number.
- Name-based field-type substitution no longer applies to collection-shaped
  response fields -- a scalar override can never stand in for a list/object,
  so reference catalogs (`getNAT`, `getPlayInstructions`) keep their
  structural types without needing `field_type_skip` entries (both entries
  removed).
- New `cargo xtask check-flags`: audits the hand-curated boolean-flag tables
  against the doc-mined parameter descriptions, reporting flag-like params
  not yet typed as `bool` and stale table entries. Finding `locationType`
  above was its first catch.
- **Breaking:** the DID-identifier rule below now covers every phone-number
  field: `number`, `phone_number`, `contact`, `destination`, and `stationid`
  join `did` in the generator's built-in String-override table
  (`PHONE_STRING_FIELDS`). Concretely, `SetCallbackParams.number` and
  `SetPhonebookParams.number` change from `Option<i64>` to `Option<String>` --
  the WSDL declared them `xsd:integer` even though `setPhonebook` documents
  `sip:2563` as a valid value, so a SIP phonebook entry was previously
  impossible to send and a get/set round-trip required a lossy parse (the
  matching response fields were already `String`). All previously patched
  response fields keep their exact types; the per-method patches that
  re-stated this rule are removed from `tools/api-response-overrides.json`,
  and `cargo xtask gen` now warns when a patch is shadowed by a field-name
  override.
- **Breaking:** DID identifier fields are now `String`, never numeric. The WSDL
  declared the `did` parameter of `getFaxNumbersInfo`,
  `getFaxNumbersPortability`, `setFaxNumberEmail`, `setFaxNumberInfo`, and
  `setFaxNumberURLCallback` as `xsd:integer`, so they were generated as
  `Option<i64>`; a DID is a phone number (an identifier, not a quantity) that
  can carry leading zeros and exceed `i64` range, so passing one lost
  information. They are now `Option<String>`, uniform with every other `did`.
  Likewise `getDIDvPRI`'s `dids` response field was inferred as `Vec<u64>` from
  a numeric-looking sample but holds DIDs; it is now `Vec<String>`.

### Fixed

- Register four undocumented empty-collection statuses that the live API returns
  for an empty list but that are absent from the API's error-code table:
  `no_emailtofax` (`getEmailToFax`), `no_folder` (`getFaxFolders`),
  `no_transactions` (`getTransactionHistory`), and `no_vpri` (`getVPRIs`). The
  typed methods now return an empty response for these instead of `Error::Api`.
  Found by auditing every list-returning method against the live API.

## [0.8.0] - 2026-07-09

### Fixed

- `searchFaxAreaCodeCAN` and `searchFaxAreaCodeUSA`: `ratecenters` is a list of
  `{area_code, available, ratecenter}` objects, not a scalar. The doc sample was
  a mis-parsed `print_r` dump that flattened the array into an `array(` scalar
  plus a spurious `0` field, so an area code with matches failed with "expected
  string, number, or bool, got [{...}]". An area code with no matches returns
  `{"status":"success"}` with no `ratecenters` field, which now deserializes to
  an empty list.

## [0.7.0] - 2026-07-09

### Fixed

- Five response fields whose types did not match what the live API returns,
  each of which failed deserialization of an otherwise-successful call:
  - `getTerminationRates`: `route` is a list of `{value, description}` entries,
    not a single object.
  - `e911AddressTypes`: `types` is a list of `{value, description}` catalog
    entries, not flattened scalars (the doc sample was a mis-parsed `print_r`
    dump).
  - `getFaxNumbersInfo`: a number's `did` is a dotted string
    (`647.948.4755`), typed `String` instead of `u64`.
  - `getLNPListStatus`: `list_status` is a string-keyed `code => description`
    map (including an empty-string key), now a
    `HashMap<String, String>` instead of a scalar.
  - `getReportEstimatedHoldTime`: the `types` entries carry free-text
    `value`/`description` strings (e.g. `"once"` / `"Yes, only once"`), not
    yes/no booleans.
- Four more responses whose `print_r` doc sample was flattened into sibling
  scalars instead of the real nested shape:
  - `e911Info`: `info` is a nested object (`did`, `full_name`, address parts,
    …), not top-level scalars.
  - `getLNPList`, `getLNPNotes`, `getLNPAttachList`: `list` is a list of
    objects (`{portid, numbers, foc_date, status}` / `{note, date, time}` /
    `{attachid, type, size}`), not flattened scalars.
- DID and phone-number identifier fields retyped from `u64` to `String`
  (`did`, SMS/MMS `contact`, CDR `destination`, `phone_number`, `number`,
  fax `stationid`/`from`/`destination`, `deleted_did`, `DIDAdded`, …): these
  are identifiers, not quantities, and can carry a `+`, formatting, or a short
  code that failed integer parsing. Caller-ID / forward override fields
  (`callerid_number`, `callerid_override`, `default_e911`, `sms_forward`) are
  likewise `String` but fold voip.ms's `-1` "not set" sentinel (and empty) to
  `None`.

### Added

- A `map` response shape kind in the codegen, emitting a bare
  `HashMap<String, V>` that defaults to empty (absent means empty, matching the
  list convention), for reference catalogs whose keys are data rather than
  schema.
- `deserialize_opt_string_sentinel_none`, a string deserializer that folds the
  `-1`/empty "not set" sentinel to `None`, for caller-ID override fields.

## [0.6.0] - 2026-07-09

### Changed

- **Breaking:** every list-valued response field is now a bare `Vec<T>` that
  defaults to empty, instead of `Option<Vec<T>>`. VoIP.ms signals an empty
  collection by omitting the field (or via an `is_empty` status that strips the
  subtree), so absent and empty always meant the same thing -- the `Option` only
  added a `None` no caller could act on differently from `Some(vec![])`. Callers
  drop the `.unwrap_or_default()` / `.unwrap()` / `.as_ref()` dance and use the
  `Vec` directly (`.is_empty()`, `.iter()`, indexing).

### Fixed

- **Breaking:** `getNAT`, `getPlayInstructions`, and `getJoinWhenEmptyTypes`
  return a list of `{value, description}` option objects, not a scalar. Their
  response fields are now `Vec<…>` of a generated element struct with
  `value: Option<String>` and `description: Option<String>`:
  - `GetNATResponse.nat`: was `Option<Nat>`, now `Vec<GetNATResponseNAT>`.
  - `GetPlayInstructionsResponse.play_instructions`: was
    `Option<PlayInstructions>`, now
    `Vec<GetPlayInstructionsResponsePlayInstruction>`.
  - `GetJoinWhenEmptyTypesResponseType.value` / `.description`: were
    `Option<bool>`, now `Option<String>`.
  The name-based `Nat` / `PlayInstructions` enum substitution wrongly overrode
  the list-typed reference-listing fields, and the extractor mis-inferred the
  `yes`/`Yes` sample cells as booleans; a live `value` of `Strict` or an array
  payload then failed to deserialize. The `Nat` and `PlayInstructions` enums are
  unchanged and still type the corresponding scalar setting fields elsewhere.

## [0.5.0] - 2026-07-07

### Changed

- **Breaking:** `date` on `getVoicemailMessages` is now `Option<chrono::NaiveDateTime>`
  instead of `Option<chrono::NaiveDate>`. VoIP.ms returns a full timestamp for
  this field, e.g. `2023-06-26 15:37:05`, which failed to deserialize as a
  bare date with "trailing input". The new type matches the identical `date`
  field on `getCDR` and `getResellerCDR`.

## [0.4.0] - 2026-07-07

### Changed

- **Breaking:** `callerid` on `getVoicemailMessages`, `getFAXMessages`,
  `getPhonebook`, and `getCallerIDFiltering` is now `Option<String>` instead of
  `Option<u64>`. VoIP.ms returns the caller's display form for inbound caller ID
  -- a name and number in angle brackets, e.g. `NAME <4164442828>` -- which
  failed to deserialize as an integer with "invalid digit found in string". The
  new type matches the identical `callerid` on `getVoicemailTranscriptions`,
  `getCDR`, and `getResellerCDR`. A purely numeric caller ID still round-trips as
  its string form.

## [0.3.2] - 2026-07-06

### Fixed

- Response list fields (`getVoicemailMessageFile`'s `message`,
  `getRecordingFile`'s `recordings`, `getConferenceRecordingFile`'s `recording`,
  and every other generated `Option<Vec<_>>` field) failed to deserialize with
  "invalid type: map, expected a sequence" when VoIP.ms returned a single-row
  result as a bare object instead of a one-element array -- which the live API
  does for the fetch-one file methods. A tolerant single-or-sequence
  deserializer now backs every list field: an array is taken as-is, a lone
  object (or scalar) becomes a one-element `Vec`, and null / absent /
  empty-string stays `None`.

## [0.3.1] - 2026-06-25

### Fixed

- Boolean flag parameters `answered`, `noanswer`, `busy`, and `failed`
  (`getCDR` / `getResellerCDR`), `activate` (`signupClient`), `portout`
  (`cancelDID`), and `advanced` (`getBalance`) serialized as bare
  `true`/`false`. They are documented as `1`/`0` flags but were left out of the
  `FLAG_01_FIELDS` override, so they missed the `1`/`0` param `serialize_with`.
  They now serialize as `1`/`0` like every other flag.

## [0.3.0] - 2026-06-25

### Changed

- **Breaking:** Removed the `rustls-tls-webpki-roots` feature. reqwest 0.13.4's
  `rustls` feature verifies against the OS trust store via
  `rustls-platform-verifier` and no longer exposes an embedded-Mozilla-roots
  toggle. `rustls-tls-native-roots` (default) and `native-tls` remain; an image
  with no OS trust store now needs `rustls-no-provider` plus a hand-built
  `ClientConfig`. The minimum reqwest is raised to 0.13.4 accordingly.
- Dropped the direct `url` dependency; the crate uses `reqwest::Url`, and
  reqwest re-exports it.

- **Breaking:** Boolean-flag parameters and fields are now typed `bool` instead
  of `i64` / `String` / `f64`. Many VoIP.ms parameters documented as
  `1 = true, 0 = false` (or `yes`/`no`) were under-typed by the WSDL; they now
  take a plain `bool`. The `1`/`0` or `yes`/`no` the API requires (a bare `bool`
  serializes `true`/`false`, which these reject) is produced by a param
  `serialize_with`, and tolerant deserialization accepts
  `1`/`0`/`yes`/`no`/`true`/`false` as string, number, or JSON bool. Affected
  fields are listed in `FLAG_01_FIELDS` / `FLAG_YES_NO_FIELDS` in
  `xtask/src/field_overrides.rs`. Validate-only flags whose `false` is
  equivalent to absent (the `test` param) are plain `bool` (default `false`,
  omitted from the request when `false`); the rest are `Option<bool>` so an
  explicit `Some(false)` ("turn it off") stays distinct from `None` ("leave
  unchanged"). Callers passing a string (e.g. `enable: Some("1".to_string())`)
  must migrate to `enable: Some(true)`, and `test` to `test: true`.
- **Breaking:** Queue/announcement duration fields documented as a number of
  seconds *or* a no-limit word are now typed `Seconds` / `WaitTime` (hand-written
  enums holding a `u64` count or an unbounded sentinel) instead of `String`:
  `retry_timer`, `wrapup_time`, `member_delay`, `announce_round_seconds`,
  `frequency_announcement`, `announce_position_frecuency` (`Seconds`, sentinel
  `none`) and `maximum_wait_time` (`WaitTime`, sentinel `unlimited`). Callers
  pass `Some(Seconds::Value(30))` / `Some(Seconds::Unlimited)`. Both are
  re-exported from the crate root and deserialize tolerantly (number, numeric
  string, or sentinel word).
- **Breaking:** `ApiStatus` is now an enum instead of a `String` newtype. It
  has one variant per documented VoIP.ms `status` code (~475, e.g.
  `ApiStatus::InvalidCredentials`, `ApiStatus::APINotEnabled`,
  `ApiStatus::NoDID`) for ergonomic `match` arms, plus an
  `ApiStatus::Unknown(String)` catch-all that preserves any undocumented code
  verbatim. `ApiStatus::description()` returns the documented human-readable
  meaning (`None` for `Unknown`), `as_str()` returns the verbatim wire string,
  `is_documented()` reports whether the code is a known variant, and
  `from_wire()` / `From<String>` / `From<&str>` parse the wire string. Code
  matching on the old `ApiStatus(String)` tuple must migrate to the variants
  (or match `ApiStatus::Unknown(s)` / call `as_str()`). The enum is generated
  by `cargo xtask gen` from the new committed `tools/api-statuses.json`,
  extracted from the docs' global error-code table via the new
  `cargo xtask extract-statuses` subcommand.
- **Breaking:** The typed list methods no longer error when the collection is
  empty. VoIP.ms returns a distinct `no_*` status per list method when there
  are no entries (`no_sms`, `no_cdr`, `no_messages`, …); the typed `Client`
  methods now fold any such status (`ApiStatus::is_empty()`) into a successful
  response with the collection field `None`, instead of `Err(Error::Api(...))`.
  Code that matched `Err(Error::Api(ApiStatus::NoSMS))` (or the other empty
  codes) on a typed call must instead handle an `Ok` whose collection field is
  `None`/empty. The `*_raw` methods are unchanged and still surface the empty
  status as `Error::Api`. Codes that look like `no_*` but signal a real failure
  (`no_base64file`, `no_callstatus`, `no_change_billingtype`, `no_provision`,
  `no_provision_update`, `no_sequences`) still error. The classification lives
  in the new `empty_statuses` array of `tools/api-response-overrides.json`.

### Added

- `ApiStatus::is_empty()`, reporting whether a status means "the requested
  collection is empty" rather than a failure. Generated from the
  `empty_statuses` array in `tools/api-response-overrides.json`.

- `field_type_skip` section in `tools/api-response-overrides.json` to suppress
  the global field-name override for one struct -- on both the `*Params` and
  `*Response` side -- where a flag/enum name is reused for an unrelated value
  (used for `getVoicemails`' `urgent` message count and `getFaxMessages`'
  free-text `folder` name).
- `field_type_override` section in `tools/api-response-overrides.json` to assign
  one struct's field a specific enum, overriding the global field-name table
  (used for the `type` field, which means different things per method).
- Generated `Client` methods and `*Params` structs now carry the official
  per-method description as a doc comment (mined from the docs into the new
  `method_docs` section of `tools/api-responses.json`; ~218 of 222 methods
  have one).
- Crate-level docs now cover IP allow-listing (and the `getIP` exemption) and
  the REST wire format.
- Re-exported `chrono`, `reqwest`, `rust_decimal`, `serde_json`, and `serde`
  from the crate root. Their types appear in the public API, so callers can now
  name those types (and `match` on `Error::Http`) without declaring a separate,
  independently-versioned dependency.
- The substituted enum types (`DtmfMode`, `MessageType`, …) and the
  hand-written `Routing` / `Seconds` / `WaitTime` now derive `Hash`, so they can
  be used as `HashMap` / `HashSet` keys. (`Copy` is not derived: the
  `Unknown(String)` catch-all holds a `String`.)

### Fixed

- `getFaxMessages`' `folder` parameter and response field were mistyped as the
  `VoicemailFolder` enum by the global `folder` override. A fax folder is a
  free-text name (`SENT` / `ALL` / user-created via `setFaxFolder`) outside that
  variant set; both are now `String`.

## [0.1.3] - 2026-05-25

### Changed

- Switched generated method naming so typed responses are now the default:
  unsuffixed methods return generated `*Response` structs, and raw JSON
  access moved to explicit `*_raw` methods.
- Renamed the low-level `Client` helpers to match: `Client::call_typed`
  → [`Client::call`], `Client::call_typed_at` → [`Client::call_at`],
  and the prior raw-JSON `Client::call` → [`Client::call_raw`].
  Generated method bodies now call `self.call(...)` for the typed
  wrapper and `self.call_raw(...)` for the `*_raw` wrapper.
- Generated PascalCase type names now preserve acronym casing
  (`GetDIDsInfoParams`, `SendSMSResponse`, `GetDTMFModesResponse`,
  `GetVPRIsResponse`, etc.) instead of title-casing acronyms
  (`GetDidsInfoParams`, `SendSmsResponse`, …). The acronym table that
  drives snake_case method names is now also used to render PascalCase
  type names, including nested element types whose wire field name is
  lowercase (`GetDIDsInfoResponseDID`, `GetSMSResponseSMS`,
  `GetSIPURIsResponseSIPURI`).
- Smarter English singularization for nested element type names:
  `-xes` / `-zes` / `-ches` / `-shes` words drop the full `es`
  (`faxes` → `fax`, so the prior `GetFAXMessagesResponseFaxe` is now
  `GetFAXMessagesResponseFAX`); `-sses` words drop just the trailing
  `es` (`addresses` → `address`); plain `-ses` words like `phrases`
  fall through to the simple `-s` strip (the prior `RecognizedPhras`
  is now `RecognizedPhrase`).
- Updated examples and docs to match the typed-by-default API.
- README link references now use absolute docs.rs URLs so they render
  on crates.io and GitHub instead of relying on rustdoc intra-doc
  resolution.
- Improved typed response deserialization robustness for string-like fields
  that VoIP.ms sometimes emits as numbers or booleans.

### Added

- New public type [`Routing`] (and `RoutingParseError`) modeling the
  tagged `kind:value` strings VoIP.ms uses for call-routing fields
  (`account:`, `fwd:`, `vm:`, `sip:`, `sys:`, `grp:`, `queue:`, `ivr:`,
  `cb:`, `tc:`, `disa:`, `did:`, `phone:`, `none:`). Generated
  `*Params` and `*Response` structs now type 12 routing-related fields
  (`routing`, `failover_*`, `fail_over_routing_*`) as
  `Option<Routing>` instead of `Option<String>`. Unknown tags
  round-trip via a `Routing::Unknown { tag, value }` catch-all so
  forward compatibility is preserved.
- New public enums for documented VoIP.ms scalars: `DtmfMode`, `Nat`,
  `EmailAttachmentFormat`, `TranscriptionFormat`, `PlayInstructions`,
  `RingStrategy`, `RingGroupOrder`, `VoicemailFolder`, `QueueEmptyBehavior`
  (`join_when_empty` / `leave_when_empty`), `EstimatedHoldTimeAnnounce`,
  `CallPickupBehavior`, `RecordingSort`, `SearchType`, `VanityType`,
  `MessageType` (SMS/MMS direction), `DialingMode`, `TollFreeCarrier`, and
  `DidBillingType` (integer-coded). Each carries an `Unknown(String)`
  variant for values not in the documented set, so VoIP.ms adding new
  options never breaks deserialization. `QueueEmptyBehavior` and
  `EstimatedHoldTimeAnnounce` also correct a latent bug: the queue response
  fields were inferred as `bool` from a `yes`/`no` sample and would have
  dropped the third value (`strict` / `once`).
- The `type` field is now typed per struct: a search mode (`SearchType`) in
  the DID/toll-free search params, a vanity prefix (`VanityType`) in
  `searchVanity`, and a message direction (`MessageType`, wire `1`/`0`) in
  the SMS/MMS params and responses. Reference-data `type` lookups whose
  valid set comes from another endpoint stay `String`.
- Generated enum deserializers now accept the wire value as a JSON string,
  number, or bool (VoIP.ms returns the SMS `type` as a bare number), not
  only a string.
- Codegen overrides schema extended with top-level `enums` and
  `field_types` sections in `tools/api-response-overrides.json`. New
  enums can be added declaratively (name, variants, wire strings) and
  mapped to one or more field names without touching the generator
  source.
- Dry-run support for runnable examples:
  `VOIP_MS_DRY_RUN=true` for `get_balance`, `list_dids`, and `send_sms`;
  `LIVE_VERIFY_DRY_RUN=true` for `live_api_verify` smoke/extended flows.

## [0.1.2] - 2026-05-25

### Changed

- Replaced the small set of hand-written starter response types
  (`StatusResponse`, `GetBalanceResponse`, `GetDidsInfoResponse`) with a
  full generated `*Response` struct per method (222 in total). Shapes are
  inferred from the official VoIP.ms HTML docs by a new
  `cargo xtask extract-responses` extractor, with hand-edited corrections
  in `tools/api-response-overrides.json`. All response fields are
  `Option<T>` and tolerate string-or-number, `0/1`, `Y/N`, and
  date/datetime placeholder forms via custom deserializers in
  `src/responses.rs`.
- `src/responses.rs` no longer re-exports any types; it now contains only
  the shared `deserialize_opt_*` helpers used by `src/generated.rs`.
- Examples updated to consume the generated `*Response` structs.

### Added

- `cargo xtask extract-responses <html>` for refreshing
  `tools/api-responses.json` from a saved copy of `apidocs.php`.
- `tools/api-response-overrides.json` schema (see `xtask/src/overrides.rs`)
  supporting per-path scalar retypes and full shape replacement.
- `DEVELOPMENT.md` documents the HTML-refresh workflow.

## [0.1.1] - 2026-05-22

### Changed

- Added and documented typed response ergonomics more clearly across user and
  maintainer docs: raw `serde_json::Value` methods, generated `*_typed`
  methods, `call_typed` / `call_typed_at`, and starter partial typed response
  structs.
- Moved contributor and maintainer workflows out of `README.md` into a new
  `DEVELOPMENT.md` guide (regeneration, testing strategy, CI/CD behavior,
  and release process), keeping README focused on crate usage.

## [0.1.0] - 2026-05-22

### Changed

- Bumped `reqwest` to `0.13` and adjusted feature flag mappings to its
  reorganized TLS surface. User-facing feature names
  (`rustls-tls-native-roots`, `rustls-tls-webpki-roots`, `native-tls`)
  are unchanged.
- Dependabot auto-merge now skips `0.x → 0.y` Cargo updates, which are
  classified as `semver-minor` by Dependabot but are breaking under
  Cargo's SemVer interpretation. Those land as reviewed PRs.

### Added

- Initial release skeleton: async `Client` over `reqwest`, typed
  `*Params` request structs and `Client` methods for all 222 VoIP.ms
  REST operations, generated from `tools/server.wsdl` by the
  `xtask` workspace member (`cargo xtask gen`).
- `Client::call` for invoking methods not yet covered by the
  generator and for typed deserialization via `serde_json::from_value`.
- `Error::Http` / `Error::Api(ApiStatus)` / `Error::InvalidResponse`
  error surface.
- TLS feature flags: `rustls-tls-native-roots` (default),
  `rustls-tls-webpki-roots`, `native-tls`.
- Examples: `get_balance`, `send_sms`, `list_dids` (run with
  `VOIP_MS_USERNAME` / `VOIP_MS_PASSWORD` set).
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
  with coverage instrumentation and Dependabot auto-merge for
  patch/minor cargo updates.

[Unreleased]: https://github.com/ecliptical/voip-ms/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.3.1
[0.3.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.3.0
[0.1.3]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.3
[0.1.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.2
[0.1.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.1
[0.1.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.0
