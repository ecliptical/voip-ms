# Development Guide

This document covers contributor and maintainer workflows for voip-ms.

## Regenerating the API surface

VoIP.ms periodically adds methods (WSDL) and revises its user-facing
documentation (response shapes, parameter notes, status codes). Re-run the
full refresh below whenever either changes — and proactively every few
months even absent a known change, since the docs drift silently. The whole
process is mechanical; the judgement is in **reviewing the diffs** (see the
checklist) before committing.

### Inputs

`src/generated.rs` is produced by the `xtask` workspace member from four
committed inputs:

* `tools/server.wsdl` — the source of truth for the method list, parameter
  names, and parameter types. Drives every `*Params` struct and `Client`
  method.
* `tools/api-responses.json` — *generated* by `cargo xtask extract-responses`
  from the saved HTML docs. Holds three things:
  * `methods` — the inferred response shape per method (→ `*Response`
    structs);
  * `param_docs` — per-parameter descriptions mined from each method's
    `Parameters` cell, including `[Required]` markers, examples, and value
    constraints (→ `///` comments on `*Params` fields);
  * `method_docs` — each method's one-line summary (→ the lead `///` comment
    on both the `*Params` struct and the `Client` method). ~218 of 222
    methods carry one; the rest are simply absent in the source.
* `tools/api-statuses.json` — *generated* by `cargo xtask extract-statuses`
  from the same HTML. The global error-code table (`status` string →
  description), rendered as the `ApiStatus` enum (one PascalCase variant per
  code + `Unknown(String)`, with `description()`/`is_documented()` lookups).
* `tools/api-response-overrides.json` — *hand-edited* corrections to the
  above (per-path scalar retypes, or a full shape replacement for the handful
  of methods the extractor can't parse). Never edit the generated
  `api-responses.json` / `api-statuses.json` by hand — fix the override file
  and regenerate. The overrides schema lives in
  [xtask/src/overrides.rs](xtask/src/overrides.rs); see its module docs for
  the path grammar and the `enums` / `field_types` / `field_type_skip`
  sections. Boolean-flag field names (typed `bool`, serialized to `1`/`0` or
  `yes`/`no`) are registered in
  [xtask/src/field_overrides.rs](xtask/src/field_overrides.rs), not the JSON.

### Reference documents

| File | Source | Versioned? |
|------|--------|------------|
| `tools/server.wsdl` | `https://voip.ms/api/v1/server.wsdl` (public) | Yes — committed |
| `tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html` | `https://voip.ms/m/apidocs.php` (requires login) | No — gitignored |
| `tools/scratch/API.zip` | `https://voip.ms/api/v1/API.zip` (public) | No — gitignored |

Only `server.wsdl` is committed. The scratch files are not version-controlled.

The HTML doc is the one input you can't script: `apidocs.php` is behind
Cloudflare and requires a logged-in browser session.

1. Log into the VoIP.ms customer portal in a browser.
2. Open `https://voip.ms/m/apidocs.php` and use the browser's "Save Page As →
   Web Page, Complete" to write
   `tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html` (plus its
   `_files/` directory). `tools/scratch/` is gitignored, so nothing here is
   committed.

### Full refresh procedure

Run all of it whenever either input changes; the WSDL-only and HTML-only
shortcuts are just subsets.

```bash
# 1. Refresh the public inputs.
curl -o tools/server.wsdl https://voip.ms/api/v1/server.wsdl
#    (save the HTML doc manually as described above)

# 2. Re-extract the two generated tools/ JSON files from the HTML.
HTML="tools/scratch/VoIP.ms - Customer Portal_ API Documentation.html"
cargo xtask extract-responses "$HTML"
cargo xtask extract-statuses  "$HTML"

# 3. Review the extract diffs BEFORE regenerating (see checklist below).
git diff tools/api-responses.json tools/api-statuses.json

# 4. Audit the boolean-flag tables against the refreshed docs. Advisory:
#    a reported candidate is either a new flag (add it to FLAG_01_FIELDS /
#    FLAG_YES_NO_FIELDS in xtask/src/field_overrides.rs) or a two-valued
#    enum (declare it in the overrides JSON instead); a "stale" entry means
#    the docs dropped the field.
cargo xtask check-flags

# 5. Apply any corrections to tools/api-response-overrides.json (NOT the
#    generated files), then regenerate src/generated.rs. Heed its warnings:
#    a patch shadowed by a field-name override should be deleted.
cargo xtask gen

# 6. Run the full quality gate — note the doc build, which CI does NOT run.
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links" \
  cargo doc --no-deps

# 7. Review the generated diff, update CHANGELOG.md, and commit
#    tools/*.json + src/generated.rs together.
git diff src/generated.rs
```

Both `cargo xtask gen` and `extract-*` print summary counts (methods covered,
status codes, method descriptions, methods missing output blocks). Compare
them against the previous run — a sudden drop usually means the HTML layout
shifted and a scanner needs updating (see "Extractor internals").

### Review checklist

The extractors are regex/structure scanners over hand-authored HTML, so each
refresh needs eyes on the diff, not just a green build:

* **New WSDL methods** — `cargo xtask gen` reports the method count. New
  methods get a `*Params`/`*Response`/`Client` method automatically. Confirm
  the snake_case and PascalCase names read correctly; a new acronym can
  produce a single-letter token (`getDIDsInfo` → `get_di_ds_info`). Fix by
  adding the acronym to the `ACRONYMS` const in
  [xtask/src/main.rs](xtask/src/main.rs) and regenerating. A new method also
  needs a home in the `livetest` harness: run `cargo xtask dump-methods` to
  refresh `livetest/src/wire_methods.rs`, then assign the method to exactly one
  area's `methods()` in `livetest/src/areas/`. The completeness gate
  (`cargo test -p livetest`) fails until every wire method is owned by exactly
  one area.
* **New status codes** — `api-statuses.json` grows; each new code becomes an
  `ApiStatus` variant. If two codes collapse to the same PascalCase variant,
  `cargo xtask gen` fails loudly with a "duplicate status variant" error —
  resolve by adjusting the acronym list or treating one as `Unknown`.
* **Mis-typed response scalars** — a phone-number field parsed as `integer`,
  a `0/1` flag parsed as `integer`, a date placeholder, etc. Fix via a
  per-path retype in `api-response-overrides.json`.
* **Unparseable Output blocks** — the extractor warns (`skipping output —
  parse error`). Two methods are known-unparseable and covered by full shape
  replacements in the overrides file (`setSIPURI` has no Output block;
  `getLNPDetails` uses a non-standard PHP dialect). A *newly* unparseable
  method needs the same treatment.
* **`doc` build failures** — newly mined `param_docs`/`method_docs` text can
  contain bare URLs or `[bracketed]` prose that rustdoc rejects. The generator
  sanitizes these (`sanitize_doc_word` in `main.rs`: URLs → `<…>` autolinks,
  `[` / `]` → escaped), but a novel pattern can still slip through — that's
  why step 5 runs the strict `cargo doc`. Extend `sanitize_doc_word` if it
  does.
* **Merged parameter rows** — when the source HTML lacks a clean line break
  between two parameters, the param-doc scanner can merge them
  (e.g. a description ending `…Todayclient => [Required] …`). These are
  cosmetic doc-comment artifacts; fix the source-faithful text only if it's
  egregious.

### Extractor internals

If a refresh shows a scanner has stopped finding content (counts drop sharply),
the HTML layout likely changed. The scanners in
[xtask/src/extract.rs](xtask/src/extract.rs) all key off specific CSS class
strings:

* method detail/title cells: `toptitlex normaltextbold`
* two-column rows (params, outputs, method descriptions, status codes):
  `leftmenubottomtdlinefull normaltext` / `leftmenubottomtdlinerightfull normaltext`
* the status table is anchored on the `Error Codes` title cell and stops at
  the next section title; method descriptions are filtered to known WSDL
  operation names so section headers and the error table are excluded.

Update the class-string constants if VoIP.ms reskins the docs.

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

The suite has two layers with different jobs.

1. **Behavioral tests** encode independent knowledge of the VoIP.ms wire
	 contract, so they hold regardless of how `generated.rs` is produced:
	 * `tests/client.rs` (`wiremock`) asserts a success response surfaces the
		 full envelope verbatim; a non-success status maps to `Error::Api`; a 5xx
		 to `Error::Http`; a missing `status` field to `Error::InvalidResponse`;
		 `None` fields are omitted from the query; and typed deserialization via
		 `call`/`call_at`/`call_raw` works. It also pins specific documented wire
		 forms (routing tags, `1`/`0` and `yes`/`no` flags, `-1`/`0000-00-00`
		 sentinels, enum codes).
	 * Unit tests in `src/responses.rs` cover every custom (de)serializer,
		 including the malformed-input error paths.

2. **Codegen-drift oracles** (`tests/generated_*.rs`) are a fixed,
	 hand-maintained check on the generated surface. `src/generated.rs` is
	 produced by `cargo xtask gen`; these tests are authored *independently* of
	 it, so they catch a generator that drops, renames, or mistypes a type:
	 * `generated_params.rs` -- all 222 `*Params` serialize to a JSON object.
	 * `generated_responses.rs` -- all 333 `*Response` structs deserialize from
		 a bare `{}` (every field is optional/defaulted).
	 * `generated_enums.rs` -- each wire enum's `from_wire`/`as_wire`/`Display`/
		 serde round-trip and `Unknown` fallback.
	 * `generated_api_status.rs` -- every documented `ApiStatus` code,
		 transcribed by hand, round-trips and is classified as documented.

	 These are deliberately NOT generated by `xtask`: sharing the codegen's
	 inputs would give them the codegen's blind spots. After a
	 [regeneration](#regenerating-the-api-surface), if the type/enum/status set
	 changed, a compile or assertion failure here is the gate working -- update
	 the affected oracle by hand to match the new intended surface, do not
	 suppress it.

3. Live calls to VoIP.ms are intentionally excluded from CI because they
	 require real credentials and account state.

## Live API verification (`livetest`)

The `livetest` workspace member is an operator-local harness that exercises the
API against a real account to catch response-shape drift the wiremock tests
cannot -- the recurring class of bug where the live API returns JSON the
generated `*Response` type can't deserialize. Its central check fetches each
method's raw envelope, then attempts the typed deserialization over it, so a
failure is unambiguously drift (raw succeeds, typed fails) rather than a network
or API error, and it prints the raw JSON alongside the serde error, ready to
paste into a `tools/api-response-overrides.json` fix.

It is a `[[bin]]` member (`publish = false`), so it never ships in the published
crate and never runs under `cargo test`. Invoke it explicitly:

```bash
cargo run -p livetest -- [OPTIONS]
cargo run -p livetest -- --list-areas   # enumerate areas; no credentials needed
cargo run -p livetest -- --help
```

### Selection: areas x depth

Two orthogonal dimensions choose what runs:

* **Areas** (`--areas dids,sms,...`, `--all-areas`, `--exclude ...`) -- the 222
  methods partition into functional areas (dids, sms, mms, voicemail, cdr,
  account, subaccount, fax, forwarding, e911, conference, ivr, queue,
  phonebook, porting, reseller, callflow, reference). The default set is every
  *free* area; the costly-by-nature areas (`account`, `e911`, `fax`, `porting`,
  `reseller`) are excluded until named, so they run only "once in a while."
* **Depth** (`--depth probe|lifecycle|costly`, default `probe`):
  * `probe` -- read-only calls only, with the raw-vs-typed drift diff. Free.
  * `lifecycle` -- also runs free create -> read -> delete fixtures.
  * `costly` -- also runs money/irreversible methods. Requires
    `--i-understand-this-costs-money`. Each costly method fires only when its
    own input (e.g. `--order-test-did`, `--sms-dst`, `--payment-amount`) is
    supplied; without it the method is skipped, not failed.

A pre-flight sweep reconciles leftover fixtures from prior runs (matched by an
ownership marker, plus a git-ignored run-ledger for un-markable resources like
DIDs) before creating any new ones, and aborts if it cannot reach a clean slate.

### Connecting through an API server

By default the harness targets the real VoIP.ms API server, which requires the
caller's source IP to be on the account's API allow-list. To run from a
non-allow-listed host, point it at an allow-listed API server (a reverse proxy
that forwards to VoIP.ms from an allow-listed IP) and, if that server gates
access, supply its add-on HTTP Basic auth (separate from the VoIP.ms API
credentials):

* `--api-url` / `LIVETEST_API_URL` -- the server's REST endpoint.
* `--api-basic-username` / `LIVETEST_API_BASIC_USERNAME`
* `--api-basic-password` / `LIVETEST_API_BASIC_PASSWORD`

### Secrets

All secrets are supplied at runtime, never committed:

* `--username` / `VOIP_MS_USERNAME`, `--password` / `VOIP_MS_PASSWORD` -- the
  VoIP.ms API credentials (username is the account email; the password is the
  distinct API password from the portal's SOAP & REST/JSON API page).
* the API-server URL and Basic-auth pair above, if used.
* fixture inputs (destination numbers, DIDs, amounts) passed per run.

Credentials and the API-server auth are redacted from all harness output, and
the run-ledger (`livetest-ledger.jsonl`, which may contain DIDs/usernames) is
git-ignored. Keep any local env file out of the repo.

### Example

A read-only drift sweep across every area (free and costly-by-nature), through
an allow-listed API server, with all secrets from the environment:

```bash
VOIP_MS_USERNAME=... VOIP_MS_PASSWORD=... \
LIVETEST_API_URL=https://your-api-server.example/api/v1/rest.php \
LIVETEST_API_BASIC_USERNAME=... LIVETEST_API_BASIC_PASSWORD=... \
cargo run -p livetest -- --all-areas
```

The run prints per-method `pass` / `fail` / `skip` / `drift` lines and a final
JSON summary; it exits non-zero if anything failed or drifted.

### CI

Not wired to CI yet. GitHub-hosted runners use ephemeral egress IPs that the
VoIP.ms per-account allow-list rejects (`ip_not_enabled`), so a future CI job
must run from an allow-listed host or through an allow-listed API server, with
the credentials and server auth held as secrets.

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

Live verification is not a CI workflow; it is the operator-local `livetest`
harness described under "Live API verification (`livetest`)" above.
