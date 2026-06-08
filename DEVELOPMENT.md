# Development Guide

This document covers contributor and maintainer workflows for voip-ms.

## Regenerating the API surface

`src/generated.rs` is produced by the `xtask` workspace member from
three committed inputs:

* `tools/server.wsdl` — method list, parameter names, parameter types.
* `tools/api-responses.json` — inferred response shape per method
  (`methods`) plus mined parameter descriptions (`param_docs`), both
  extracted from the official HTML docs by `cargo xtask
  extract-responses`. The `param_docs` map keys each wire method to a
  `{ param_name: description }` table; `cargo xtask gen` renders those
  descriptions as `///` doc comments on the matching `*Params` fields.
* `tools/api-response-overrides.json` — hand-edited corrections to the
  above (per-path scalar retypes or full shape replacement).

### Reference documents

| File | Source | Versioned? |
|------|--------|------------|
| `tools/server.wsdl` | `https://voip.ms/api/v1/server.wsdl` (public) | Yes — committed |
| `tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html` | `https://voip.ms/m/apidocs.php` (requires login) | No — gitignored |
| `tools/scratch/API.zip` | `https://voip.ms/api/v1/API.zip` (public) | No — gitignored |

Only `server.wsdl` is committed. The scratch files are not version-controlled;
download them locally as needed:

```bash
# Public files — fetch directly
curl -o tools/server.wsdl https://voip.ms/api/v1/server.wsdl
curl -o tools/scratch/API.zip https://voip.ms/api/v1/API.zip

# HTML docs — Cloudflare-gated, must be saved manually from a browser session
# (see "Refreshing response shapes" below)
```

### Picking up new methods (WSDL change)

```bash
curl -o tools/server.wsdl https://voip.ms/api/v1/server.wsdl
cargo xtask gen
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### Refreshing response shapes (HTML docs change)

voip.ms's `apidocs.php` page is behind Cloudflare and requires a logged-in
session, so the HTML must be saved manually:

1. Log into the voip.ms customer portal in a browser.
2. Open `https://voip.ms/m/apidocs.php` and save the complete rendered page
   (HTML + supporting files) to `tools/scratch/` — e.g. save as
   `tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html`.
   That directory is gitignored, so the file won't be committed.
3. Re-run the extractor and inspect the diff:

   ```bash
   cargo xtask extract-responses "tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html"
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

## Live API verification workflow

The repository includes a dedicated workflow for optional live verification:
`.github/workflows/live-api-verify.yaml`.

Use this workflow for on-demand execution via `workflow_dispatch`. It is
intentionally separate from `rust-ci.yaml` so pull requests remain
deterministic and credential-free, and it deliberately does **not** run on
tag pushes: GitHub-hosted runners use ephemeral egress IPs that voip.ms's
per-account API IP allow-list rejects with `ip_not_enabled`. Trigger it
manually from an allow-listed host (or wire it to a self-hosted runner with
a known static egress IP) when you want a live check.

### Required account configuration

1. Create a dedicated voip.ms sandbox account (or isolated reseller test scope).
2. Enable API access and generate API credentials.
3. Allow-list the GitHub runner egress IP(s) on the voip.ms API page.
4. For SMS checks, provide at least one DID with SMS available and enabled.
5. For sub-account lifecycle checks, ensure the sandbox has permission to
   create and delete sub-accounts.

### Required GitHub Actions secrets

* `VOIP_MS_USERNAME`
* `VOIP_MS_PASSWORD`

Optional fixture secrets used by opt-in checks:

* `VOIP_MS_TEST_DID`
* `VOIP_MS_SMS_DST`
* `VOIP_MS_SMS_MESSAGE`

### Safety model

The live harness defaults to read-only smoke checks.

State-changing checks require both:

* `LIVE_VERIFY_MODE=extended`
* `LIVE_VERIFY_ALLOW_STATE_CHANGES=true`

Potentially costly checks (for example sending SMS) require both:

* `LIVE_VERIFY_ENABLE_SMS_SEND_CHECK=true`
* `LIVE_VERIFY_ALLOW_COSTLY=true`

This dual-gate model prevents accidental financial transactions and keeps
release verification safe by default.

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
* `live-api-verify.yaml` supports optional live verification:
	* `workflow_dispatch` only — operator-invokable smoke or extended checks
	* not wired to any push or tag event, because GitHub-hosted runners
		use ephemeral egress IPs that voip.ms's per-account API IP
		allow-list will reject (`ip_not_enabled`). Trigger manually from
		a host whose IP is on the voip.ms API allow-list, or from a
		self-hosted runner with a known static egress IP.
	* explicit safety gates for state-changing or costly operations
