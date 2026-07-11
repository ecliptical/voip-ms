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
use voip_ms::{ApiStatus, Client, Error, QueueEmptyBehavior, RingStrategy, SetQueueParams};

/// Every `(required)` `setQueue` field but the caller-supplied `queue_name`,
/// filled with values the API accepts, so a fixture only sets its name and the
/// `queue_number` via `..required_queue_params(number)`. Shared by the `queue`
/// area and the `callflow` static-member fixture that stands up a throwaway
/// queue. A queue with a missing required field is rejected one field at a time
/// (`missing_number`, then the next), so all of them must be present at once.
///
/// `queue_number` is caller-supplied because two queue fixtures run in one
/// `--all-areas` sweep and a duplicate number is refused; each derives a
/// distinct one from the run token.
pub fn required_queue_params(queue_number: u64) -> SetQueueParams {
    SetQueueParams {
        queue_number: Some(queue_number),
        queue_language: Some("en".into()),
        priority_weight: Some("1".into()),
        report_hold_time_agent: Some("yes".into()),
        join_when_empty: Some(QueueEmptyBehavior::Yes),
        leave_when_empty: Some(QueueEmptyBehavior::No),
        ring_strategy: Some(RingStrategy::RingAll),
        ring_inuse: Some(false),
        ..Default::default()
    }
}

/// A deterministic-per-run queue number folded from the run token and a
/// `seq` (so the two same-run queue fixtures never collide). Kept in the
/// 4-digit space; cross-run collisions are reclaimed by the marker sweep.
pub fn queue_number(token: &str, seq: u64) -> u64 {
    let mut hash: u64 = seq;
    for b in token.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(*b));
    }

    1000 + (hash % 9000)
}

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
    // The `label` (`fixture:getX`) is the report key, not a wire method: the
    // wire call is `getX`. Strip the prefix so the read-back invokes the same
    // method the top-level probe does -- passing the full label made every
    // read-back hit an unknown method and fail with `invalid_method` (issue #18
    // Class B), so the populated-response drift check never ran.
    let method = label.strip_prefix("fixture:").unwrap_or(label);
    let outcome = probe::<P, T>(client, method, params, count).await;

    // Defensive diagnostic: if a read-back still returns an error status,
    // capture the exact request and the raw response envelope so a live run can
    // pin the cause without a second targeted run.
    if let ProbeOutcome::ApiError(ref status) = outcome {
        capture_read_back_error(client, area, method, params, status).await;
    }

    let drifted = matches!(outcome, ProbeOutcome::Drift { .. });
    report.record_probe(area, label, outcome);
    !drifted
}

/// Dump the exact read-back request (wire method + serialized query params) and
/// the raw response envelope for a fixture read-back that returned an error
/// status, so the Class B `invalid_method` case can be diagnosed from a live run.
async fn capture_read_back_error<P>(
    client: &Client,
    area: &str,
    method: &str,
    params: &P,
    status: &str,
) where
    P: Serialize + Sync,
{
    eprintln!("[capture] {area}/{method}: read-back returned error status `{status}`");
    match serde_json::to_value(params) {
        Ok(json) => eprintln!("[capture]   request: method={method} params={json}"),
        Err(error) => {
            eprintln!("[capture]   request: method={method} (params unserializable: {error})");
        }
    }

    match client.call_raw_unchecked(method, params).await {
        Ok(body) => {
            let pretty = serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string());
            eprintln!("[capture]   raw response:");
            for line in pretty.lines() {
                eprintln!("[capture]     {line}");
            }
        }
        Err(error) => eprintln!("[capture]   raw response unavailable: {error}"),
    }
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

/// Fold a teardown `del_*` result so a delete of an already-absent resource
/// counts as success: an "absent" status ([`ApiStatus::is_empty`], e.g.
/// `no_conference`, or an `invalid_*` "not a valid <resource> ID" code) means
/// the object the teardown targets is gone, which is the teardown's goal.
///
/// Without this, a fixture whose resource was already reclaimed -- by an earlier
/// aborted step, or a prior run's sweep -- reports a *phantom* cleanup failure
/// naming an orphan that does not exist. A genuine delete failure (permission,
/// transport, a dependency still attached) is not absent-like and still errors.
pub fn tolerate_absent<T>(result: Result<T, Error>) -> anyhow::Result<()> {
    match result {
        Ok(_) => Ok(()),
        Err(Error::Api(status)) if is_absent(&status) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Whether a `del_*` error status means the target is already gone: an
/// empty-collection status ([`ApiStatus::is_empty`], e.g. `no_conference`) or
/// an `invalid_<resource>` code, which VoIP.ms returns for a delete addressing
/// a non-existent id (e.g. `invalid_conference` -- "This is not a valid
/// Conference ID"). Scoped to a teardown deleting an id the harness itself
/// created, where "not a valid id" can only mean it is already reclaimed.
///
/// The `invalid_credentials`/`invalid_*_auth` family is *not* absence -- it is a
/// real auth failure that must stay a teardown failure -- so it is excluded even
/// though it shares the `invalid_` prefix.
fn is_absent(status: &ApiStatus) -> bool {
    if status.is_empty() {
        return true;
    }

    let code = status.to_string();
    code.starts_with("invalid_") && !code.contains("credential") && !code.contains("auth")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerate_absent_folds_absent_and_empty_statuses_to_ok() {
        // Empty-collection status: the resource list is empty, so the target
        // is gone. `del_*` methods return a response payload, so `T` is not
        // `()`; the annotation stands in for a real delete response.
        assert!(tolerate_absent::<()>(Err(Error::Api(ApiStatus::NoConference))).is_ok());
        // "not a valid <resource> id": a delete of an already-reclaimed id.
        assert!(tolerate_absent::<()>(Err(Error::Api(ApiStatus::InvalidConference))).is_ok());
        assert!(
            tolerate_absent::<()>(Err(Error::Api(ApiStatus::Unknown("invalid_queue".into()))))
                .is_ok()
        );
    }

    #[test]
    fn tolerate_absent_passes_success_through() {
        // A successful delete discards its response payload.
        assert!(tolerate_absent(Ok("del-response")).is_ok());
    }

    #[test]
    fn tolerate_absent_preserves_a_genuine_failure() {
        // A non-absent error status is a real teardown failure and must not be
        // swallowed -- including a credentials error, which shares the
        // `invalid_` prefix but is not absence.
        assert!(tolerate_absent::<()>(Err(Error::Api(ApiStatus::InvalidCredentials))).is_err());
    }

    #[test]
    fn is_absent_matches_resource_absence_not_auth_failures() {
        assert!(is_absent(&ApiStatus::NoConference));
        assert!(is_absent(&ApiStatus::InvalidConference));
        assert!(is_absent(&ApiStatus::Unknown("invalid_widget".into())));
        // A credentials error is not an "absent target"; it must stay a failure
        // even though its wire code starts with `invalid_`.
        assert!(!is_absent(&ApiStatus::InvalidCredentials));
    }

    #[test]
    fn queue_number_is_stable_and_distinct_per_seq() {
        let a0 = queue_number("tok", 0);
        let a1 = queue_number("tok", 1);
        assert_eq!(
            queue_number("tok", 0),
            a0,
            "stable for a fixed (token, seq)"
        );
        assert_ne!(a0, a1, "distinct per seq so same-run queues don't collide");
        assert!((1000..=9999).contains(&a0));
        assert!((1000..=9999).contains(&a1));
    }
}
