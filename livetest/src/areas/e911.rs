//! The `e911` area: emergency-service address provisioning for DIDs.
//! Costly-by-nature because provisioning carries a fee and records a physical
//! address (irreversible, sensitive), so it is excluded from the default set
//! until named. Only the address-type enumeration probes cleanly; `e911Info`
//! needs a DID and `e911Validate` a full address, so both are skipped at probe
//! depth. The provision/update/cancel writes are owned but run only at costly
//! depth.
//!
//! At `Depth::Costly`, a run given `--e911-did` without `--e911-provision`
//! first reads that DID's current record with `e911Info`, which costs nothing.
//! The safe path runs next and unconditionally when a DID
//! plus a complete address is supplied: `e911Validate` checks the address
//! without provisioning anything. Provisioning is gated behind `--e911-provision`
//! -- only then does the fixture run `e911Provision -> e911Info -> e911Update`
//! and cancel via `e911Cancel`. voip.ms refuses the cancel with `e911_pending`
//! while it validates a new address (16 minutes, observed once), so the cancel
//! retries for up to [`CANCEL_ATTEMPTS`] minutes and holds the run that long.
//! Validation is deliberately preferred over
//! provisioning. `e911ProvisionManually` (the manual-review provisioning
//! variant) is not fired: `e911Provision` is the chosen provisioning path, and
//! provisioning the same DID twice would leak a second address.

use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::{Depth, E911Config};
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::{read_back, tolerate_absent};
use crate::harness::probe::{ProbeOutcome, probe};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct E911;

const AREA: &str = "e911";

/// How many times the teardown sends `e911Cancel`, [`CANCEL_RETRY_WAIT`] apart,
/// while voip.ms answers `e911_pending`.
const CANCEL_ATTEMPTS: u32 = 30;
const CANCEL_RETRY_WAIT: Duration = Duration::from_secs(60);

#[async_trait(?Send)]
impl Area for E911 {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "e911AddressTypes",
            "e911Cancel",
            "e911Info",
            "e911Provision",
            "e911ProvisionManually",
            "e911Update",
            "e911Validate",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "e911AddressTypes",
            E911AddressTypesParams,
            E911AddressTypesResponse,
            types
        );
        skip_needs_input!(report, AREA, "e911Info", "requires an e911-enabled DID");
        skip_needs_input!(
            report,
            AREA,
            "e911Validate",
            "requires a DID and a full address"
        );
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        // The manual-review provisioning variant is never fired: `e911Provision`
        // is the chosen provisioning path, and provisioning the same DID twice
        // would leak a second address.
        report.record(
            AREA,
            "fixture:e911ProvisionManually",
            Outcome::Skip("e911Provision is the chosen provisioning path".to_string()),
        );

        let cfg = &ctx.config.e911;

        // Provisioning reads the record back itself, so the DID's current
        // record is read only on a run that does not provision.
        if let Some(did) = &cfg.did
            && !cfg.provision
        {
            info_fixture(ctx, report, did).await;
        }

        let Some(address) = e911_address(cfg) else {
            report.record(
                AREA,
                "fixture:e911Validate",
                Outcome::Skip(
                    "--e911-did, address inputs and --e911-email not all set".to_string(),
                ),
            );
            report.record(
                AREA,
                "fixture:e911Provision",
                Outcome::Skip(
                    "--e911-did, address inputs and --e911-email not all set".to_string(),
                ),
            );
            return;
        };

        // Validate-only always runs: it provisions nothing.
        validate_fixture(ctx, report, &address).await;

        if !cfg.provision {
            report.record(
                AREA,
                "fixture:e911Provision",
                Outcome::Skip("--e911-provision not set (validate-only)".to_string()),
            );
            return;
        }

        provision_fixture(ctx, report, &address).await;
    }
}

/// A complete e911 address, present only when every required field was
/// supplied. Assembled from [`E911Config`]; an incomplete address yields `None`
/// so the fixture records skip (no input) rather than sending a partial call.
struct E911Address {
    did: String,
    full_name: String,
    street_number: u64,
    street_name: String,
    city: String,
    state: String,
    country: String,
    zip: String,
    email: String,
    language: Option<String>,
}

fn e911_address(cfg: &E911Config) -> Option<E911Address> {
    Some(E911Address {
        did: cfg.did.clone()?,
        full_name: cfg.full_name.clone()?,
        street_number: cfg.street_number?,
        street_name: cfg.street_name.clone()?,
        city: cfg.city.clone()?,
        state: cfg.state.clone()?,
        country: cfg.country.clone()?,
        zip: cfg.zip.clone()?,
        email: cfg.email.clone()?,
        language: cfg.language.clone(),
    })
}

/// Read the DID's current e911 record through the typed path with drift
/// diffing. A DID with no record answers `e911_disable`, which is a skip: the
/// run has nothing to read, not a defect.
async fn info_fixture(ctx: &AreaCtx<'_>, report: &mut Report, did: &str) {
    const LABEL: &str = "fixture:e911Info";
    let outcome = probe::<_, E911InfoResponse>(
        ctx.client,
        "e911Info",
        &E911InfoParams {
            did: Some(did.to_string()),
        },
        |r| Some(r.info.len()),
    )
    .await;

    if let ProbeOutcome::ApiError(status) = &outcome
        && status == ApiStatus::E911Disable.as_wire()
    {
        report.record(
            AREA,
            LABEL,
            Outcome::Skip("the DID has no e911 record (e911_disable)".to_string()),
        );
        return;
    }

    report.record_probe(AREA, LABEL, outcome);
}

async fn validate_fixture(ctx: &AreaCtx<'_>, report: &mut Report, address: &E911Address) {
    let result = ctx
        .client
        .e911_validate(&E911ValidateParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip_code: Some(address.zip.clone()),
            language: address.language.clone(),
            email: Some(address.email.clone()),
            ..Default::default()
        })
        .await;

    match result {
        Ok(_) => report.record(AREA, "fixture:e911Validate", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:e911Validate",
            Outcome::Fail(format!("e911Validate: {error}")),
        ),
    }
}

/// Provision the address, read it back typed, update it, then cancel. The
/// cancel is deferred immediately after a successful provision so it fires
/// exactly once on every path -- including when the read-back or update fails.
async fn provision_fixture(ctx: &AreaCtx<'_>, report: &mut Report, address: &E911Address) {
    let client = ctx.client;

    let provisioned = client
        .e911_provision(&E911ProvisionParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip_code: Some(address.zip.clone()),
            language: address.language.clone(),
            email: Some(address.email.clone()),
            ..Default::default()
        })
        .await;

    if let Err(error) = provisioned {
        return fail(
            report,
            "fixture:e911Provision",
            &format!("e911Provision: {error}"),
        );
    }

    report.record(AREA, "fixture:e911Provision", Outcome::Pass);

    let mut scope = Scope::new();
    let cancel_did = address.did.clone();
    scope.defer(format!("e911 did={cancel_did}"), move |client| {
        Box::pin(async move {
            let params = E911CancelParams {
                did: Some(cancel_did),
            };
            cancel_once_validated(
                || client.e911_cancel(&params),
                CANCEL_ATTEMPTS,
                CANCEL_RETRY_WAIT,
            )
            .await
        })
    });

    read_back::<_, E911InfoResponse>(
        client,
        report,
        AREA,
        "fixture:e911Info",
        &E911InfoParams {
            did: Some(address.did.clone()),
        },
        |r| Some(r.info.len()),
    )
    .await;

    match client
        .e911_update(&E911UpdateParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip_code: Some(address.zip.clone()),
            language: address.language.clone(),
            email: Some(address.email.clone()),
            ..Default::default()
        })
        .await
    {
        Ok(_) => report.record(AREA, "fixture:e911Update", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:e911Update",
            Outcome::Fail(format!("e911Update: {error}")),
        ),
    }

    for label in scope.cleanup(client).await {
        report.record(
            AREA,
            "cleanup",
            Outcome::Fail(format!("teardown failed for {label}")),
        );
    }
}

/// Send `cancel` while voip.ms answers `e911_pending` or the request fails in
/// transit for a reason repeating may cure, at most `attempts` times (at least
/// once), `wait` apart. `e911_disable` means the DID has no record left to
/// cancel, so it counts as done, which also covers a retry after a cancel that
/// landed but whose answer was lost. Any other answer is final.
async fn cancel_once_validated<F, Fut, T>(
    mut cancel: F,
    attempts: u32,
    wait: Duration,
) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 1;
    loop {
        match cancel().await {
            Err(Error::Api(ApiStatus::E911Disable)) => return Ok(()),
            Err(error) if attempt < attempts && worth_another_cancel(&error) => {
                println!(
                    "[info] e911Cancel attempt {attempt}/{attempts}: {error}; retrying in {}s",
                    wait.as_secs()
                );
                attempt += 1;
                tokio::time::sleep(wait).await;
            }
            result => return tolerate_absent(result),
        }
    }
}

fn worth_another_cancel(error: &Error) -> bool {
    matches!(error, Error::Api(ApiStatus::E911Pending))
        || error
            .transport()
            .is_some_and(|failure| failure.retry_outlook() != RetryOutlook::Futile)
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// Run [`cancel_once_validated`] against `answers`, one per call, and
    /// return its result with the number of calls it made.
    async fn run(
        answers: &[std::result::Result<(), ApiStatus>],
        attempts: u32,
    ) -> (anyhow::Result<()>, usize) {
        let calls = Cell::new(0);
        let result = cancel_once_validated(
            || {
                let answer = answers[calls.get()].clone().map_err(Error::Api);
                calls.set(calls.get() + 1);
                async move { answer }
            },
            attempts,
            Duration::ZERO,
        )
        .await;
        (result, calls.get())
    }

    #[tokio::test]
    async fn a_pending_cancel_is_retried_until_it_is_accepted() {
        let pending = Err(ApiStatus::E911Pending);
        let (result, calls) = run(&[pending.clone(), pending, Ok(())], 5).await;
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(calls, 3);
    }

    #[tokio::test]
    async fn a_cancel_still_pending_after_the_last_attempt_fails() {
        let pending = Err(ApiStatus::E911Pending);
        let (result, calls) = run(&[pending.clone(), pending.clone(), pending], 3).await;
        let error = result.expect_err("still pending");
        assert!(error.to_string().contains("e911_pending"), "{error}");
        assert_eq!(calls, 3);
    }

    #[tokio::test]
    async fn a_did_with_no_record_left_counts_as_cancelled() {
        let (result, calls) = run(&[Err(ApiStatus::E911Disable)], 5).await;
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(calls, 1);
    }

    #[tokio::test]
    async fn a_cancel_that_fails_in_transit_is_retried() {
        // Port 1 on loopback refuses the connection, which the crate
        // classifies as worth repeating.
        let refused = voip_ms::reqwest::Client::new()
            .get("http://127.0.0.1:1/")
            .send()
            .await
            .expect_err("nothing listens on port 1");
        let mut refused = Some(Error::from(refused));
        let calls = Cell::new(0);
        let result = cancel_once_validated(
            || {
                calls.set(calls.get() + 1);
                let answer = refused.take().map_or(Ok(()), Err);
                async move { answer }
            },
            5,
            Duration::ZERO,
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(calls.get(), 2);
    }

    #[tokio::test]
    async fn any_other_refusal_is_not_retried() {
        let (result, calls) = run(&[Err(ApiStatus::InvalidCredentials)], 5).await;
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
