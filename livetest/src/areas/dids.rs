//! The `dids` area: the account's DIDs and vPRIs, plus the availability
//! searches. The list-all reads (`getDIDsInfo`, `getBackOrders`, `getVPRIs`)
//! and the toll-free availability searches probe cleanly; the geography- or
//! id-scoped lookups (`getDIDsCAN`/`USA`, the international catalogs, `getDIDvPRI`,
//! and the criteria-driven searches) are skipped at probe depth. The ordering
//! and routing writes are owned but run only at costly depth. Kept in the
//! default (free) set: DID ordering is costly by depth, not the area's read
//! nature, and `getDIDsInfo` is the harness's bread-and-butter probe.
//!
//! At `Depth::Costly`, opting into `--order-test-did` runs a fully
//! self-contained order -> configure -> read -> cancel fixture over one CAN
//! DID (routed to `none:` so it needs no other fixture), independent of the
//! sms/mms area's own `--test-did`: a dedicated, never-cancelled number they
//! use instead, so the two costly paths can be exercised separately. Teardown
//! is ledger-driven: the ledger records the ordered number immediately after a
//! successful order and only forgets it once `cancelDID` confirms, and
//! [`sweep`](Area::sweep) reconciles any number a crashed prior run left
//! recorded but never cancelled.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::read_back;
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Dids;

const AREA: &str = "dids";
const LEDGER_KIND: &str = "did";

#[async_trait(?Send)]
impl Area for Dids {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "assignDIDvPRI",
            "backOrderDIDCAN",
            "backOrderDIDUSA",
            "cancelDID",
            "connectDID",
            "getBackOrders",
            "getDIDsCAN",
            "getDIDsInfo",
            "getDIDsInternationalGeographic",
            "getDIDsInternationalNational",
            "getDIDsInternationalTollFree",
            "getDIDsUSA",
            "getDIDvPRI",
            "getVPRIs",
            "orderDID",
            "orderDIDInternationalGeographic",
            "orderDIDInternationalNational",
            "orderDIDInternationalTollFree",
            "orderDIDVirtual",
            "orderTollFree",
            "orderVanity",
            "removeDIDvPRI",
            "searchDIDsCAN",
            "searchDIDsUSA",
            "searchTollFreeCanUS",
            "searchTollFreeUSA",
            "searchVanity",
            "setDIDBillingType",
            "setDIDInfo",
            "setDIDPOP",
            "setDIDRouting",
            "unconnectDID",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getBackOrders",
            GetBackOrdersParams,
            GetBackOrdersResponse,
            back_orders
        );
        skip_needs_input!(report, AREA, "getDIDsCAN", "requires a province");
        probe_list!(
            ctx,
            report,
            AREA,
            "getDIDsInfo",
            GetDIDsInfoParams,
            GetDIDsInfoResponse,
            dids
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalGeographic",
            "requires a country id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalNational",
            "requires a country id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalTollFree",
            "requires a country id"
        );
        skip_needs_input!(report, AREA, "getDIDsUSA", "requires a state");
        skip_needs_input!(report, AREA, "getDIDvPRI", "requires a vPRI id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getVPRIs",
            GetVPRIsParams,
            GetVPRIsResponse,
            vpri
        );
        skip_needs_input!(
            report,
            AREA,
            "searchDIDsCAN",
            "requires search type and query"
        );
        skip_needs_input!(
            report,
            AREA,
            "searchDIDsUSA",
            "requires search type and query"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "searchTollFreeCanUS",
            SearchTollFreeCANUSParams,
            SearchTollFreeCANUSResponse,
            dids
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "searchTollFreeUSA",
            SearchTollFreeUSAParams,
            SearchTollFreeUSAResponse,
            dids
        );
        skip_needs_input!(
            report,
            AREA,
            "searchVanity",
            "requires vanity type and query"
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        // The ledger, not an in-account marker, is authoritative for DIDs: a
        // free-text marker field isn't safe on a resource whose number gets
        // reassigned to other customers after cancellation. Reconcile any
        // entry a crashed prior run recorded but never cancelled.
        let entries = match ctx.ledger.entries_for_account() {
            Ok(entries) => entries,
            Err(error) => {
                report.record(
                    AREA,
                    "sweep:did:enumerate",
                    Outcome::Fail(format!("reading ledger: {error:#}")),
                );
                return SweepResult {
                    unreconciled: vec!["did: ledger read failed".to_string()],
                };
            }
        };

        let mut unreconciled = Vec::new();
        for entry in entries.into_iter().filter(|e| e.kind == LEDGER_KIND) {
            match reconcile_ledger_did(ctx, report, &entry.id).await {
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

        if !ctx.config.order_test_did {
            report.record(
                AREA,
                "fixture:orderDID",
                Outcome::Skip("--order-test-did not set".to_string()),
            );
            return;
        }

        order_fixture_did(ctx, report).await;
    }
}

/// Look up a ledger-recorded DID still on the account and cancel it; remove
/// the ledger entry only once `cancelDID` confirms. Returns `Ok(())` when the
/// account ends up without the number (already gone, or freshly cancelled),
/// `Err(label)` when it's still present and cancellation failed.
async fn reconcile_ledger_did(
    ctx: &AreaCtx<'_>,
    report: &mut Report,
    did: &str,
) -> std::result::Result<(), String> {
    let client = ctx.client;

    let still_present = match client
        .get_dids_info(&GetDIDsInfoParams {
            did: Some(did.to_string()),
            ..Default::default()
        })
        .await
    {
        // `no_did` is a registered empty status, so an absent DID already
        // arrives here as `Ok` with an empty `dids` list, not `Err`.
        Ok(resp) => resp.dids.iter().any(|d| d.did.as_deref() == Some(did)),
        Err(error) => {
            report.record(
                AREA,
                "sweep:did:reconfirm",
                Outcome::Fail(format!("getDIDsInfo for ledger DID {did}: {error:#}")),
            );
            return Err(format!("did {did}: lookup failed"));
        }
    };

    if !still_present {
        // Gone from the account already (a prior run's cancel landed but the
        // ledger write didn't) -- the ledger entry is stale, drop it.
        let _ = ctx.ledger.remove(LEDGER_KIND, did);
        return Ok(());
    }

    report.record(
        AREA,
        "sweep:did:found",
        Outcome::Skip(format!("reclaiming ledger-recorded DID {did}")),
    );

    match client
        .cancel_did(&CancelDIDParams {
            did: Some(did.to_string()),
            ..Default::default()
        })
        .await
    {
        Ok(_) => {
            let _ = ctx.ledger.remove(LEDGER_KIND, did);
            println!("[sweep] {AREA}/did: cancelled ledger-recorded DID {did}");
            Ok(())
        }
        Err(error) => {
            report.record(
                AREA,
                "sweep:did:delete",
                Outcome::Fail(format!("cancelDID for ledger DID {did}: {error:#}")),
            );
            Err(format!("did {did}: cancel failed"))
        }
    }
}

/// Search for one purchasable CAN DID, order it (routed to `none:` so the
/// fixture needs no other resource), enable SMS, read it back, then defer its
/// cancellation. The ledger entry is appended the instant the order succeeds
/// and removed only after `cancelDID` confirms, so a crash between the two
/// leaves a record [`Area::sweep`] can reconcile on the next run.
async fn order_fixture_did(ctx: &AreaCtx<'_>, report: &mut Report) {
    let client = ctx.client;

    let did = match find_purchasable_did(ctx).await {
        Ok(Some(did)) => did,
        Ok(None) => {
            return fail(
                report,
                "fixture:searchDIDsCAN",
                "no purchasable DID found for the configured province/query",
            );
        }
        Err(error) => {
            return fail(
                report,
                "fixture:searchDIDsCAN",
                &format!("searchDIDsCAN: {error}"),
            );
        }
    };

    let pop = match find_server_pop(ctx).await {
        Ok(Some(pop)) => pop,
        Ok(None) => return fail(report, "fixture:getServersInfo", "no server POP available"),
        Err(error) => {
            return fail(
                report,
                "fixture:getServersInfo",
                &format!("getServersInfo: {error}"),
            );
        }
    };

    let ordered = client
        .order_did(&OrderDIDParams {
            did: Some(did.clone()),
            routing: Some(Routing::None),
            pop: Some(pop),
            dialtime: Some(60),
            cnam: Some(0),
            billing_type: Some(DidBillingType::PerMinute),
            ..Default::default()
        })
        .await;

    if let Err(error) = ordered {
        return fail(report, "fixture:orderDID", &format!("orderDID: {error}"));
    }

    // Record ownership before anything else can run: from this line on, a
    // crash is reconciled by `sweep` on the next invocation rather than
    // silently leaking a paid number.
    if let Err(error) = ctx.ledger.append(LEDGER_KIND, &did, ctx.token.as_str()) {
        report.record(
            AREA,
            "fixture:orderDID",
            Outcome::Fail(format!(
                "orderDID succeeded but ledger append failed: {error:#}"
            )),
        );
    }

    report.record(AREA, "fixture:orderDID", Outcome::Pass);

    let mut scope = Scope::new();
    let cancel_did = did.clone();
    scope.defer(format!("did={cancel_did}"), move |client| {
        Box::pin(async move {
            client
                .cancel_did(&CancelDIDParams {
                    did: Some(cancel_did),
                    ..Default::default()
                })
                .await?;
            Ok(())
        })
    });

    if let Err(error) = client
        .set_sms(&SetSMSParams {
            did: Some(did.clone()),
            enable: Some(true),
            ..Default::default()
        })
        .await
    {
        report.record(
            AREA,
            "fixture:setSMS",
            Outcome::Fail(format!("setSMS: {error}")),
        );
    } else {
        report.record(AREA, "fixture:setSMS", Outcome::Pass);
    }

    read_back::<_, GetDIDsInfoResponse>(
        client,
        report,
        AREA,
        "fixture:getDIDsInfo",
        &GetDIDsInfoParams {
            did: Some(did.clone()),
            ..Default::default()
        },
        |r| Some(r.dids.len()),
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
                "cancelDID confirmed but ledger remove failed: {error:#}"
            )),
        );
    }
}

async fn find_purchasable_did(ctx: &AreaCtx<'_>) -> voip_ms::Result<Option<String>> {
    let resp = ctx
        .client
        .search_dids_can(&SearchDIDsCANParams {
            province: Some(ctx.config.did_search_province.clone()),
            r#type: Some(SearchType::Contains),
            query: Some(ctx.config.did_search_query.clone()),
        })
        .await?;

    Ok(resp.dids.into_iter().find_map(|d| d.did))
}

async fn find_server_pop(ctx: &AreaCtx<'_>) -> voip_ms::Result<Option<u64>> {
    let resp = ctx
        .client
        .get_servers_info(&GetServersInfoParams::default())
        .await?;

    Ok(resp
        .servers
        .iter()
        .find(|s| s.server_recommended == Some(true))
        .or_else(|| resp.servers.first())
        .and_then(|s| s.server_pop))
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}
