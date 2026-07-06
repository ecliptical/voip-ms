# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
