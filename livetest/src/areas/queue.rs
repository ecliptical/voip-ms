//! The `queue` area: call queues and their estimated-hold-time report. Both
//! reads probe cleanly; the set/del writes are owned but run only at costly
//! depth. (A queue's static members belong to the `callflow` area, per the API
//! scope split.)

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Queue;

#[async_trait(?Send)]
impl Area for Queue {
    fn name(&self) -> &'static str {
        "queue"
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
        const AREA: &str = "queue";

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
}
