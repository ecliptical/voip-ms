//! The `forwarding` area: call-forwarding entries. The list read probes
//! cleanly; the set/del writes are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Forwarding;

#[async_trait(?Send)]
impl Area for Forwarding {
    fn name(&self) -> &'static str {
        "forwarding"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["delForwarding", "getForwardings", "setForwarding"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "forwarding";

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
}
