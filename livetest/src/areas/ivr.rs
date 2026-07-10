//! The `ivr` area: interactive voice response menus. The list read probes
//! cleanly; the set/del writes are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Ivr;

#[async_trait(?Send)]
impl Area for Ivr {
    fn name(&self) -> &'static str {
        "ivr"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["delIVR", "getIVRs", "setIVR"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "ivr";

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
}
