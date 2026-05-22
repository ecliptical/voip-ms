# Development Guide

This document covers contributor and maintainer workflows for voip-ms.

## Regenerating the API surface

The 222 typed request structs and Client methods are generated from
[tools/server.wsdl](tools/server.wsdl) by the xtask workspace member
([xtask/src/main.rs](xtask/src/main.rs)).

To pick up new methods after voip.ms updates the WSDL:

```bash
# Download the latest WSDL from voip.ms:
curl -o tools/server.wsdl https://voip.ms/api/v1/server.wsdl

# Regenerate and verify:
cargo xtask gen
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

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
