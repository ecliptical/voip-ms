//! Shared building blocks for the `Lifecycle`-depth create -> read -> delete
//! fixtures and the marker-orphan pre-flight sweep.
//!
//! A fixture's read-back is the whole reason the fixtures exist: a *populated*
//! typed response exercises the element deserializers (the voicemail-date and
//! callerid folds live there) that an empty account never reaches. So the
//! read-back routes through the same raw-vs-typed [`probe`] as the read-only
//! phase -- a shape mismatch on the freshly created object surfaces as drift,
//! not as a fixture failure.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::harness::area::SweepResult;
use crate::harness::marker::is_owned_marker;
use crate::harness::probe::{ProbeOutcome, probe};
use crate::harness::{Outcome, Report};
use voip_ms::{ApiStatus, Client, Error};

/// Read a just-created resource back through the typed path with drift
/// diffing, recording the outcome under `area`/`label`. The scoped `params`
/// narrow the list to the created object so the response is populated.
///
/// Returns `true` when the read-back deserialized cleanly (drift-free),
/// regardless of whether the object was actually found -- a drift is recorded
/// and reported here and must not also count as a fixture failure elsewhere.
pub async fn read_back<P, T>(
    client: &Client,
    report: &mut Report,
    area: &str,
    label: &str,
    params: &P,
    count: impl Fn(&T) -> Option<usize>,
) -> bool
where
    P: Serialize + Sync,
    T: DeserializeOwned,
{
    let outcome = probe::<P, T>(client, label, params, count).await;
    let drifted = matches!(outcome, ProbeOutcome::Drift { .. });
    report.record_probe(area, label, outcome);
    !drifted
}

/// A marker-bearing orphan the sweep found and must delete: a human label for
/// logging and the identifier needed to delete it.
pub struct Orphan {
    pub label: String,
    pub id: u64,
}

/// True when a free-text field (a description or name) carries the harness
/// marker. Convenience over [`is_owned_marker`] for the common `Option<String>`
/// field shape.
pub fn owned(field: &Option<String>) -> bool {
    field.as_deref().is_some_and(is_owned_marker)
}

/// Run one area's marker-orphan reconciliation: enumerate marker-bearing
/// leftovers, report each, delete them in the order given, then re-enumerate
/// and return a non-clean [`SweepResult`] naming anything that survived.
///
/// `list` enumerates current marker-bearing orphans (already dependency-ordered
/// by the caller if it matters). `delete` removes one by id. Both are async
/// closures returning a boxed future so an area can call its own typed
/// `del_*`/`get_*` methods.
pub async fn sweep_orphans<L, LFut, D, DFut>(
    report: &mut Report,
    area: &str,
    kind: &str,
    list: L,
    delete: D,
) -> SweepResult
where
    L: Fn() -> LFut,
    LFut: Future<Output = anyhow::Result<Vec<Orphan>>>,
    D: Fn(u64) -> DFut,
    DFut: Future<Output = anyhow::Result<()>>,
{
    let orphans = match list().await {
        Ok(orphans) => orphans,
        // An empty-collection status is a zero-orphan enumeration, not a
        // failure: `getPhonebookGroups`-style methods return e.g.
        // `no_phonebook_group` for an empty account, which the typed `call`
        // path turns into `Error::Api` whenever the status isn't registered
        // in `ApiStatus::is_empty` (undocumented codes decode as `Unknown`
        // and can't be proven empty). Mirror the probe path's handling here,
        // and defensively extend it to any `Unknown` status that *looks*
        // like the same "no_*" empty-collection convention, so an
        // undocumented status doesn't abort the whole sweep.
        Err(error) if is_empty_like(&error) => return SweepResult::clean(),
        Err(error) => {
            report.record(
                area,
                &format!("sweep:{kind}:enumerate"),
                Outcome::Fail(format!("enumerating orphans: {error:#}")),
            );
            // Enumeration failure is a non-clean slate: we cannot prove the
            // account is orphan-free, so refuse to proceed.
            return SweepResult {
                unreconciled: vec![format!("{kind}: enumeration failed")],
            };
        }
    };

    if orphans.is_empty() {
        return SweepResult::clean();
    }

    for orphan in &orphans {
        report.record(
            area,
            &format!("sweep:{kind}:found"),
            Outcome::Skip(format!("reclaiming orphan {}", orphan.label)),
        );
        match delete(orphan.id).await {
            Ok(()) => println!("[sweep] {area}/{kind}: deleted orphan {}", orphan.label),
            Err(error) => report.record(
                area,
                &format!("sweep:{kind}:delete"),
                Outcome::Fail(format!("deleting orphan {}: {error:#}", orphan.label)),
            ),
        }
    }

    // Re-enumerate: anything still present after the delete pass is unreconciled.
    let remaining = match list().await {
        Ok(remaining) => remaining,
        Err(error) if is_empty_like(&error) => Vec::new(),
        Err(error) => {
            report.record(
                area,
                &format!("sweep:{kind}:reconfirm"),
                Outcome::Fail(format!("re-enumerating orphans: {error:#}")),
            );
            return SweepResult {
                unreconciled: vec![format!("{kind}: re-enumeration failed")],
            };
        }
    };

    SweepResult {
        unreconciled: remaining.into_iter().map(|o| o.label).collect(),
    }
}

/// Whether an enumeration failure is actually VoIP.ms's "the list is empty"
/// status wearing an error costume, not a real failure.
///
/// Areas' `list_*_orphans` helpers call a typed `get_*` method and propagate
/// its error via `?`, which erases the concrete [`voip_ms::Error`] into
/// [`anyhow::Error`]; downcast it back to check. Documented empty-collection
/// codes are caught by [`ApiStatus::is_empty`]. Undocumented ones (e.g.
/// `no_phonebook_group`, missing from the API's own status table) decode as
/// `Unknown` and can't be proven empty that way, so also treat any `Unknown`
/// status whose wire code follows the same `no_*` convention as empty --
/// until the library's status table is corrected, this is the only signal
/// available.
fn is_empty_like(error: &anyhow::Error) -> bool {
    match error.downcast_ref::<Error>() {
        Some(Error::Api(status)) => {
            status.is_empty() || matches!(status, ApiStatus::Unknown(s) if s.starts_with("no_"))
        }
        _ => false,
    }
}
