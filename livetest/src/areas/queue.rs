//! The `queue` area: call queues and their estimated-hold-time report. Both
//! reads probe cleanly; at `Lifecycle` depth the area runs a create -> read ->
//! delete fixture over a queue (marker in its `queue_name`) and its
//! [`sweep`](Area::sweep) reclaims marker-bearing queues from prior runs. (A
//! queue's static members belong to the `callflow` area, per the API scope
//! split; callflow's own sweep reclaims the throwaway queues its member fixture
//! stands up, so this area is not relied on to cover them.)

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{
    Orphan, owned, queue_number, read_back, required_queue_params, sweep_orphans, tolerate_absent,
};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Queue;

const AREA: &str = "queue";

#[async_trait(?Send)]
impl Area for Queue {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delQueue",
            "getQueues",
            "getReportEstimatedHoldTime",
            "setQueue",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getQueues",
            GetQueuesParams,
            GetQueuesResponse,
            queues
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getReportEstimatedHoldTime",
            GetReportEstimatedHoldTimeParams,
            GetReportEstimatedHoldTimeResponse,
            types
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        sweep_orphans(
            report,
            AREA,
            "queue",
            || list_orphans(client),
            |id| del_queue(client, id),
        )
        .await
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        queue_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn queue_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let queue_name = ctx.token.marker(0);

    let created = client
        .set_queue(&SetQueueParams {
            queue_name: Some(queue_name),
            ..required_queue_params(queue_number(ctx.token.as_str(), 0))
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.queue {
            Some(id) => id,
            None => {
                report.record(
                    AREA,
                    "fixture:setQueue",
                    Outcome::Fail("setQueue succeeded without an id".to_string()),
                );
                return;
            }
        },
        // A queue number the account can't provision (no internal-extension
        // range) is rejected with `invalid_number` -- an account limitation,
        // not a harness defect, so skip rather than fail.
        Err(Error::Api(ApiStatus::InvalidNumber)) => {
            report.record(
                AREA,
                "fixture:setQueue",
                Outcome::Skip("account has no provisionable queue number".to_string()),
            );
            return;
        }
        Err(error) => {
            report.record(
                AREA,
                "fixture:setQueue",
                Outcome::Fail(format!("setQueue: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:setQueue", Outcome::Pass);
    scope.defer(format!("queue id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(client.del_queue(&DelQueueParams { queue: Some(id) }).await)
        })
    });

    read_back::<_, GetQueuesResponse>(
        client,
        report,
        AREA,
        "fixture:getQueues",
        &GetQueuesParams::default(),
        |r| Some(r.queues.len()),
    )
    .await;
}

async fn list_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetQueuesResponse = client.get_queues(&GetQueuesParams::default()).await?;
    Ok(resp
        .queues
        .into_iter()
        .filter(|q| owned(&q.queue_name))
        .filter_map(|q| {
            q.queue.map(|id| Orphan {
                label: format!("queue id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_queue(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_queue(&DelQueueParams { queue: Some(id) })
        .await?;
    Ok(())
}
