# Development Guide

This document covers contributor and maintainer workflows for voip-ms.

## Regenerating the API surface

`src/generated.rs` is produced by the `xtask` workspace member from
three committed inputs:

* `tools/server.wsdl` — method list, parameter names, parameter types.
* `tools/api-responses.json` — inferred response shape per method,
  extracted from the official HTML docs by `cargo xtask
  extract-responses`.
* `tools/api-response-overrides.json` — hand-edited corrections to the
  above (per-path scalar retypes or full shape replacement).

### Picking up new methods (WSDL change)

```bash
curl -o tools/server.wsdl https://voip.ms/api/v1/server.wsdl
cargo xtask gen
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### Refreshing response shapes (HTML docs change)

voip.ms's `apidocs.php` page is Cloudflare-gated, so the HTML has to be
saved manually:

1. Log into the voip.ms customer portal in a browser.
2. Open `https://voip.ms/m/apidocs.php` and save the rendered HTML to
   `target/apidocs.html` (or any path under `target/`, which is
   gitignored).
3. Re-run the extractor and inspect the diff:

   ```bash
   cargo xtask extract-responses target/apidocs.html
   git diff tools/api-responses.json
   ```

4. If a scalar landed with the wrong type (a phone-number `did` parsed
   as `integer`, a `0/1` flag parsed as `integer`, …) or a method's
   Output block failed to parse, edit
   `tools/api-response-overrides.json` — never edit
   `tools/api-responses.json` by hand.
5. Regenerate and run the quality gate:

   ```bash
   cargo xtask gen
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```

The overrides schema lives in
[xtask/src/overrides.rs](xtask/src/overrides.rs) — see the module docs
for the path grammar.

## Releasing

Publishing is automated via [.github/workflows/release.yaml](.github/workflows/release.yaml).

1. Ensure Cargo.toml has the target version.
2. Move release notes from Unreleased into a versioned section in CHANGELOG.md.
3. Push a tag in the form vX.Y.Z.

```bash
git tag v0.1.1
git push origin v0.1.1
```

On tag push, the workflow verifies the tag version matches Cargo.toml, runs
fmt/clippy/tests, performs cargo publish --dry-run, publishes to crates.io
using CRATES_IO_TOKEN, and creates a GitHub release.

## Testing strategy

1. Integration tests in `tests/client.rs` use `wiremock` to assert:
	 * a success response surfaces the full envelope verbatim
	 * a non-success status maps to `Error::Api`
	 * a 5xx status maps to `Error::Http`
	 * a missing `status` field maps to `Error::InvalidResponse`
	 * optional `None` fields are not sent in query parameters
	 * `Client::call` supports follow-on typed deserialization
2. Coverage target is around 80% for hand-written code paths (`client.rs`
	 and `error.rs`). `generated.rs` is mechanical and validated transitively
	 through representative integration tests.
3. Live calls to voip.ms are intentionally excluded from CI because they
	 require real credentials and account state.

## CI/CD workflows

* `rust-ci.yaml` runs on pull requests and pushes to `main`:
	* `cargo fmt --all -- --check`
	* `cargo clippy --all -- -D warnings`
	* `cargo test` with coverage instrumentation
	* coverage summary posted to pull requests via
		`ecliptical/covdir-report-action`
* `dependabot-automerge.yaml` auto-approves and auto-merges safe Cargo
	updates from Dependabot.
* `release.yaml` runs on `v*` tags:
	* validates tag version against Cargo.toml
	* runs fmt, clippy, tests, and publish dry-run checks
	* publishes to crates.io with `CRATES_IO_TOKEN`
	* creates a GitHub release from the tag
