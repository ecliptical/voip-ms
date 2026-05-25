# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2026-05-25

### Changed

- Switched generated method naming so typed responses are now the default:
  unsuffixed methods return generated `*Response` structs, and raw JSON
  access moved to explicit `*_raw` methods.
- Updated examples and docs to match the typed-by-default API.
- Improved typed response deserialization robustness for string-like fields
  that voip.ms sometimes emits as numbers or booleans.

### Added

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
