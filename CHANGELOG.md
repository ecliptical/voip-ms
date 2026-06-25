# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** Boolean-flag parameters and fields are now typed `bool` instead
  of `i64` / `String` / `f64`. Many voip.ms parameters documented as
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
- **Breaking:** `ApiStatus` is now an enum instead of a `String` newtype. It
  has one variant per documented voip.ms `status` code (~475, e.g.
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

### Added

- `field_type_skip` section in `tools/api-response-overrides.json` to suppress
  the global field-name override for one struct where a flag/enum name is reused
  for an unrelated value (used for `getVoicemails`' `urgent` message count).
- `field_type_override` section in `tools/api-response-overrides.json` to assign
  one struct's field a specific enum, overriding the global field-name table
  (used for the `type` field, which means different things per method).
- Generated `Client` methods and `*Params` structs now carry the official
  per-method description as a doc comment (mined from the docs into the new
  `method_docs` section of `tools/api-responses.json`; ~218 of 222 methods
  have one).
- Crate-level docs now cover IP allow-listing (and the `getIP` exemption) and
  the REST wire format.

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
  that voip.ms sometimes emits as numbers or booleans.

### Added

- New public type [`Routing`] (and `RoutingParseError`) modeling the
  tagged `kind:value` strings voip.ms uses for call-routing fields
  (`account:`, `fwd:`, `vm:`, `sip:`, `sys:`, `grp:`, `queue:`, `ivr:`,
  `cb:`, `tc:`, `disa:`, `did:`, `phone:`, `none:`). Generated
  `*Params` and `*Response` structs now type 12 routing-related fields
  (`routing`, `failover_*`, `fail_over_routing_*`) as
  `Option<Routing>` instead of `Option<String>`. Unknown tags
  round-trip via a `Routing::Unknown { tag, value }` catch-all so
  forward compatibility is preserved.
- New public enums for documented voip.ms scalars: `DtmfMode`, `Nat`,
  `EmailAttachmentFormat`, `TranscriptionFormat`, `PlayInstructions`,
  `RingStrategy`, `RingGroupOrder`, `VoicemailFolder`, `QueueEmptyBehavior`
  (`join_when_empty` / `leave_when_empty`), `EstimatedHoldTimeAnnounce`,
  `CallPickupBehavior`, `RecordingSort`, `SearchType`, `VanityType`,
  `MessageType` (SMS/MMS direction), `DialingMode`, `TollFreeCarrier`, and
  `DidBillingType` (integer-coded). Each carries an `Unknown(String)`
  variant for values not in the documented set, so voip.ms adding new
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
  number, or bool (voip.ms returns the SMS `type` as a bare number), not
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
  inferred from the official voip.ms HTML docs by a new
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
  `*Params` request structs and `Client` methods for all 222 voip.ms
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

[Unreleased]: https://github.com/ecliptical/voip-ms/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.3
[0.1.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.2
[0.1.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.1
[0.1.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.0
