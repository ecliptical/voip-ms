//! The `ivr` area: interactive voice response menus. The list read probes
//! cleanly; at `Lifecycle` depth the area runs a create -> read -> delete
//! fixture over an IVR (marker in its `name`) and its [`sweep`](Area::sweep)
//! reclaims marker-bearing menus from prior runs.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans, tolerate_absent};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Ivr;

const AREA: &str = "ivr";

#[async_trait(?Send)]
impl Area for Ivr {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["delIVR", "getIVRs", "setIVR"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getIVRs",
            GetIVRsParams,
            GetIVRsResponse,
            ivrs
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        sweep_orphans(
            report,
            AREA,
            "ivr",
            || list_orphans(client),
            |id| del_ivr(client, id),
        )
        .await
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        ivr_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn ivr_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(0);

    // `recording`, `voicemailsetup`, and `choices` are `(required)` and take
    // account-specific codes ("Values from getRecordings"/`getVoicemailSetups`);
    // these are the conventional defaults (code 1) and a single hangup choice --
    // best-effort until a live run confirms them. A rejection surfaces as a Fail
    // naming the offending field (e.g. `missing_recording`).
    let created = client
        .set_ivr(&SetIVRParams {
            name: Some(name),
            timeout: Some(10),
            language: Some("en".to_string()),
            recording: Some(1),
            voicemailsetup: Some(1),
            choices: Some("1=sys:hangup".to_string()),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.ivr {
            Some(id) => id,
            None => {
                report.record(
                    AREA,
                    "fixture:setIVR",
                    Outcome::Fail("setIVR succeeded without an id".to_string()),
                );
                return;
            }
        },
        Err(error) => {
            report.record(
                AREA,
                "fixture:setIVR",
                Outcome::Fail(format!("setIVR: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:setIVR", Outcome::Pass);
    scope.defer(format!("ivr id={id}"), move |client| {
        Box::pin(
            async move { tolerate_absent(client.del_ivr(&DelIVRParams { ivr: Some(id) }).await) },
        )
    });

    read_back::<_, GetIVRsResponse>(
        client,
        report,
        AREA,
        "fixture:getIVRs",
        &GetIVRsParams::default(),
        |r| Some(r.ivrs.len()),
    )
    .await;
}

async fn list_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetIVRsResponse = client.get_ivrs(&GetIVRsParams::default()).await?;
    Ok(resp
        .ivrs
        .into_iter()
        .filter(|i| owned(&i.name))
        .filter_map(|i| {
            i.ivr.map(|id| Orphan {
                label: format!("ivr id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_ivr(client: &Client, id: u64) -> anyhow::Result<()> {
    client.del_ivr(&DelIVRParams { ivr: Some(id) }).await?;
    Ok(())
}
