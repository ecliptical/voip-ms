---
name: release
description: Release a new voip-ms version to crates.io and GitHub. Use when the user asks to release, tag or publish a version of the crate.
---

# Release voip-ms

A crates.io version is permanent: yanking hides it from new resolves but never
deletes it. The release procedure itself is DEVELOPMENT.md, "Releasing"; these
steps wrap it.

1. **Pick the version.** Read `## [Unreleased]` in CHANGELOG.md. If it is
   empty, stop and say there is nothing to release. Below 1.0, an entry marked
   **Breaking** takes the next minor and anything else the next patch, unless
   the user names the version. Say which version you picked and why.
2. **Follow DEVELOPMENT.md, "Releasing", steps 1 to 4.** Commit the release as
   `Release X.Y.Z` on the default branch, staging `Cargo.toml`, `README.md`
   and `CHANGELOG.md` by path, and push it with
   `git push origin HEAD:refs/heads/<default>`.
3. **Wait for `rust-ci.yaml` on the release commit.** Find the run with
   `gh run list --commit <sha> --workflow rust-ci.yaml`, then watch it with
   `gh run watch <id> --exit-status` under Monitor. If it fails, fix the cause
   in a new commit and wait again; don't tag a commit CI has not passed.
4. **Tag and push the tag:** `git tag vX.Y.Z`, then `git push origin vX.Y.Z`.
5. **Watch `release.yaml` to the end** the same way. If a job fails, read
   `gh run view <id> --log-failed`:
   - failed before the publish step: nothing shipped. Fix it, and ask the user
     before deleting and recreating the tag, since others may have fetched it;
   - failed at or after the publish step: check whether crates.io has the
     version (step 6). A version that published cannot be published again, so
     a fix ships as the next patch.
6. **Confirm what shipped:** `cargo info voip-ms@X.Y.Z` shows the crates.io
   entry, and `gh release view vX.Y.Z` the GitHub release.
7. **Report** the version, its crates.io and GitHub release links, and any
   step that failed or that you skipped.
