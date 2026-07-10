//! The `forwarding` area: call-forwarding entries. The list read probes
//! cleanly; at `Lifecycle` depth the area runs a create -> read -> delete
//! fixture over a forwarding entry (marker in its `description`) and its
//! [`sweep`](Area::sweep) reclaims marker-bearing entries from prior runs.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Forwarding;

const AREA: &str = "forwarding";

#[async_trait(?Send)]
impl Area for Forwarding {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["delForwarding", "getForwardings", "setForwarding"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getForwardings",
            GetForwardingsParams,
            GetForwardingsResponse,
            forwardings
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        sweep_orphans(
            report,
            AREA,
            "forwarding",
            || list_orphans(client),
            |id| del_forwarding(client, id),
        )
        .await
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        forwarding_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn forwarding_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let description = ctx.token.marker(0);

    let created = client
        .set_forwarding(&SetForwardingParams {
            phone_number: Some("15555550103".to_string()),
            description: Some(description),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.forwarding {
            Some(id) => id as i64,
            None => {
                report.record(
                    AREA,
                    "fixture:setForwarding",
                    Outcome::Fail("setForwarding succeeded without an id".to_string()),
                );
                return;
            }
        },
        Err(error) => {
            report.record(
                AREA,
                "fixture:setForwarding",
                Outcome::Fail(format!("setForwarding: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:setForwarding", Outcome::Pass);
    scope.defer(format!("forwarding id={id}"), move |client| {
        Box::pin(async move {
            client
                .del_forwarding(&DelForwardingParams {
                    forwarding: Some(id),
                })
                .await?;
            Ok(())
        })
    });

    read_back::<_, GetForwardingsResponse>(
        client,
        report,
        AREA,
        "fixture:getForwardings",
        &GetForwardingsParams::default(),
        |r| Some(r.forwardings.len()),
    )
    .await;
}

async fn list_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetForwardingsResponse = client
        .get_forwardings(&GetForwardingsParams::default())
        .await?;
    Ok(resp
        .forwardings
        .into_iter()
        .filter(|f| owned(&f.description))
        .filter_map(|f| {
            f.forwarding.map(|id| Orphan {
                label: format!("forwarding id={id}"),
                id: id as i64,
            })
        })
        .collect())
}

async fn del_forwarding(client: &Client, id: i64) -> anyhow::Result<()> {
    client
        .del_forwarding(&DelForwardingParams {
            forwarding: Some(id),
        })
        .await?;
    Ok(())
}
