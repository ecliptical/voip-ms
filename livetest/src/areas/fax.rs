//! The `fax` area: fax numbers, folders, messages, and email-to-fax mappings.
//! Costly-by-nature because it owns `orderFaxNumber` and the per-page
//! `sendFaxMessage` (money), so it is excluded from the default set until
//! named. The folder/message/number/email-to-fax lists probe cleanly; the
//! reads that need a message id, a DID, or an area code are skipped at probe
//! depth. The order/send/set/del writes are owned but run only at costly depth.
//!
//! At `Depth::Costly`, opting into `--order-test-fax` runs a self-contained
//! order -> configure -> read -> cancel fixture over one CAN fax number,
//! mirroring the dids area's `--order-test-did`. Teardown is ledger-driven and
//! conservative: the ledger records the ordered number the instant the order
//! succeeds and only forgets it once `cancelFaxNumber` confirms, and
//! [`sweep`](Area::sweep) reconciles any number a crashed prior run left
//! recorded but never cancelled. A fax number's cancel keys off the numeric
//! record `id`, not the DID, so both the fixture and the sweep re-derive it
//! from `getFaxNumbersInfo` -- cancelling only a number this account still
//! holds.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::read_back;
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Fax;

const AREA: &str = "fax";
const LEDGER_KIND: &str = "fax";

#[async_trait(?Send)]
impl Area for Fax {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "cancelFaxNumber",
            "connectFAX",
            "delEmailToFax",
            "delFaxFolder",
            "deleteFaxMessage",
            "getEmailToFax",
            "getFaxFolders",
            "getFaxMessagePDF",
            "getFaxMessages",
            "getFaxNumbersInfo",
            "getFaxNumbersPortability",
            "mailFaxMessagePDF",
            "moveFaxMessage",
            "orderFaxNumber",
            "searchFaxAreaCodeCAN",
            "searchFaxAreaCodeUSA",
            "sendFaxMessage",
            "setEmailToFax",
            "setFaxFolder",
            "setFaxNumberEmail",
            "setFaxNumberInfo",
            "setFaxNumberURLCallback",
            "unconnectFAX",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getEmailToFax",
            GetEmailToFAXParams,
            GetEmailToFAXResponse,
            emailToFax
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxFolders",
            GetFAXFoldersParams,
            GetFAXFoldersResponse,
            folders
        );
        skip_needs_input!(
            report,
            AREA,
            "getFaxMessagePDF",
            "requires a fax message id"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxMessages",
            GetFAXMessagesParams,
            GetFAXMessagesResponse,
            faxes
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxNumbersInfo",
            GetFAXNumbersInfoParams,
            GetFAXNumbersInfoResponse,
            numbers
        );
        skip_needs_input!(report, AREA, "getFaxNumbersPortability", "requires a DID");
        skip_needs_input!(
            report,
            AREA,
            "searchFaxAreaCodeCAN",
            "requires an area code"
        );
        skip_needs_input!(
            report,
            AREA,
            "searchFaxAreaCodeUSA",
            "requires an area code"
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        // Like DIDs, a fax number has no safe free-text marker field and its
        // number can be reassigned after cancellation, so the ledger is
        // authoritative. Reconcile any entry a crashed prior run recorded but
        // never cancelled.
        let entries = match ctx.ledger.entries_for_account() {
            Ok(entries) => entries,
            Err(error) => {
                report.record(
                    AREA,
                    "sweep:fax:enumerate",
                    Outcome::Fail(format!("reading ledger: {error:#}")),
                );
                return SweepResult {
                    unreconciled: vec!["fax: ledger read failed".to_string()],
                };
            }
        };

        let mut unreconciled = Vec::new();
        for entry in entries.into_iter().filter(|e| e.kind == LEDGER_KIND) {
            match reconcile_ledger_fax(ctx, report, &entry.id).await {
                Ok(()) => {}
                Err(label) => unreconciled.push(label),
            }
        }

        SweepResult { unreconciled }
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        if !ctx.config.order_test_fax {
            report.record(
                AREA,
                "fixture:orderFaxNumber",
                Outcome::Skip("--order-test-fax not set".to_string()),
            );
            return;
        }

        order_fixture_fax(ctx, report).await;
    }
}

/// Look up a ledger-recorded fax number still on the account and cancel it;
/// remove the ledger entry only once `cancelFaxNumber` confirms. Returns
/// `Ok(())` when the account ends up without the number (already gone, or
/// freshly cancelled), `Err(label)` when it's still present and cancellation
/// failed.
async fn reconcile_ledger_fax(
    ctx: &AreaCtx<'_>,
    report: &mut Report,
    did: &str,
) -> std::result::Result<(), String> {
    let record_id = match lookup_fax_record_id(ctx, did).await {
        // Gone from the account already (a prior run's cancel landed but the
        // ledger write didn't) -- the ledger entry is stale, drop it.
        Ok(None) => {
            let _ = ctx.ledger.remove(LEDGER_KIND, did);
            return Ok(());
        }
        Ok(Some(id)) => id,
        Err(error) => {
            report.record(
                AREA,
                "sweep:fax:reconfirm",
                Outcome::Fail(format!("getFaxNumbersInfo for ledger fax {did}: {error:#}")),
            );
            return Err(format!("fax {did}: lookup failed"));
        }
    };

    report.record(
        AREA,
        "sweep:fax:found",
        Outcome::Skip(format!("reclaiming ledger-recorded fax number {did}")),
    );

    match ctx
        .client
        .cancel_fax_number(&CancelFAXNumberParams {
            id: Some(record_id),
            ..Default::default()
        })
        .await
    {
        Ok(_) => {
            let _ = ctx.ledger.remove(LEDGER_KIND, did);
            println!("[sweep] {AREA}/fax: cancelled ledger-recorded fax number {did}");
            Ok(())
        }
        Err(error) => {
            report.record(
                AREA,
                "sweep:fax:delete",
                Outcome::Fail(format!("cancelFaxNumber for ledger fax {did}: {error:#}")),
            );
            Err(format!("fax {did}: cancel failed"))
        }
    }
}

/// Order one CAN fax number, configure its notification email, read it back,
/// then defer its cancellation. The ledger entry is appended the instant the
/// order succeeds and removed only after `cancelFaxNumber` confirms, so a crash
/// between the two leaves a record [`Area::sweep`] can reconcile on the next
/// run.
async fn order_fixture_fax(ctx: &AreaCtx<'_>, report: &mut Report) {
    let client = ctx.client;

    let location = match find_available_fax_location(ctx).await {
        Ok(Some(location)) => location,
        Ok(None) => {
            return fail(
                report,
                "fixture:getFaxRateCentersCAN",
                "no available fax rate center for the configured province",
            );
        }
        Err(error) => {
            return fail(
                report,
                "fixture:getFaxRateCentersCAN",
                &format!("getFaxRateCentersCAN: {error}"),
            );
        }
    };

    let ordered = client
        .order_fax_number(&OrderFAXNumberParams {
            location: Some(location as i64),
            quantity: Some(1),
            ..Default::default()
        })
        .await;

    let did = match ordered {
        Ok(resp) => match resp.dids.filter(|d| !d.is_empty()) {
            Some(dids) => first_did(&dids),
            None => {
                return fail(
                    report,
                    "fixture:orderFaxNumber",
                    "orderFaxNumber returned no DID",
                );
            }
        },
        Err(error) => {
            return fail(
                report,
                "fixture:orderFaxNumber",
                &format!("orderFaxNumber: {error}"),
            );
        }
    };

    // Record ownership before anything else can run: from this line on, a
    // crash is reconciled by `sweep` on the next invocation rather than
    // silently leaking a paid number.
    if let Err(error) = ctx.ledger.append(LEDGER_KIND, &did, ctx.token.as_str()) {
        report.record(
            AREA,
            "fixture:orderFaxNumber",
            Outcome::Fail(format!(
                "orderFaxNumber succeeded but ledger append failed: {error:#}"
            )),
        );
    }

    report.record(AREA, "fixture:orderFaxNumber", Outcome::Pass);

    let mut scope = Scope::new();
    // The cancel keys off the numeric record id, which `getFaxNumbersInfo`
    // supplies -- resolve it inside the teardown so it reflects the account at
    // cleanup time, and cancel only a number still present.
    let cancel_did = did.clone();
    scope.defer(format!("fax={cancel_did}"), move |client| {
        Box::pin(async move { cancel_fax_by_did(client, &cancel_did).await })
    });

    if let Err(error) = client
        .set_fax_number_info(&SetFAXNumberInfoParams {
            did: Some(did.clone()),
            email: Some(ctx.token.marker(0)),
            ..Default::default()
        })
        .await
    {
        report.record(
            AREA,
            "fixture:setFaxNumberInfo",
            Outcome::Fail(format!("setFaxNumberInfo: {error}")),
        );
    } else {
        report.record(AREA, "fixture:setFaxNumberInfo", Outcome::Pass);
    }

    read_back::<_, GetFAXNumbersInfoResponse>(
        client,
        report,
        AREA,
        "fixture:getFaxNumbersInfo",
        &GetFAXNumbersInfoParams {
            did: Some(did.clone()),
        },
        |r| Some(r.numbers.len()),
    )
    .await;

    let failures = scope.cleanup(client).await;
    if !failures.is_empty() {
        for label in failures {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }

        // The cancel didn't confirm: leave the ledger entry in place so the
        // next run's sweep retries it, rather than losing track of a live
        // paid number.
        return;
    }

    if let Err(error) = ctx.ledger.remove(LEDGER_KIND, &did) {
        report.record(
            AREA,
            "cleanup",
            Outcome::Fail(format!(
                "cancelFaxNumber confirmed but ledger remove failed: {error:#}"
            )),
        );
    }
}

/// Cancel a fax number by its DID: look up its numeric record id and, if the
/// number is still on the account, cancel it. A number already gone is a
/// success (nothing to cancel).
async fn cancel_fax_by_did(client: &Client, did: &str) -> anyhow::Result<()> {
    let Some(id) = fax_record_id(client, did).await? else {
        return Ok(());
    };

    client
        .cancel_fax_number(&CancelFAXNumberParams {
            id: Some(id),
            ..Default::default()
        })
        .await?;
    Ok(())
}

/// The numeric record id for a fax number, resolved through the harness context
/// (so the sweep's error path can distinguish an empty account from a failure).
async fn lookup_fax_record_id(ctx: &AreaCtx<'_>, did: &str) -> voip_ms::Result<Option<i64>> {
    fax_record_id(ctx.client, did).await
}

/// Resolve a fax number's numeric record `id` from `getFaxNumbersInfo`. Returns
/// `None` when the number is not on the account. `NoNumbers` is a registered
/// empty status, so an absent number arrives as `Ok` with an empty list.
async fn fax_record_id(client: &Client, did: &str) -> voip_ms::Result<Option<i64>> {
    let resp = client
        .get_fax_numbers_info(&GetFAXNumbersInfoParams {
            did: Some(did.to_string()),
        })
        .await?;

    Ok(resp
        .numbers
        .into_iter()
        .find(|n| n.did.as_deref() == Some(did))
        .and_then(|n| n.id)
        .and_then(|id| id.parse::<i64>().ok()))
}

/// Find one available CAN fax rate center in the configured province and return
/// its `location` id (the input `orderFaxNumber` needs).
async fn find_available_fax_location(ctx: &AreaCtx<'_>) -> voip_ms::Result<Option<u64>> {
    let resp = ctx
        .client
        .get_fax_rate_centers_can(&GetFAXRateCentersCANParams {
            province: Some(ctx.config.fax_search_province.clone()),
        })
        .await?;

    Ok(resp
        .ratecenters
        .into_iter()
        .find(|r| r.available == Some(true) && r.location.is_some())
        .and_then(|r| r.location))
}

/// `orderFaxNumber`'s `dids` field can carry more than one comma-separated
/// number; the fixture orders `quantity=1`, so take the first.
fn first_did(dids: &str) -> String {
    dids.split(',').next().unwrap_or(dids).trim().to_string()
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}
