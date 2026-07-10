//! The `mms` area: MMS messages and their media. The message list probes
//! cleanly; fetching a message's media needs an MMS id, so it is skipped at
//! probe depth. `sendMMS`/`deleteMMS` are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Mms;

#[async_trait(?Send)]
impl Area for Mms {
    fn name(&self) -> &'static str {
        "mms"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["deleteMMS", "getMMS", "getMediaMMS", "sendMMS"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "mms";

        probe_list!(
            ctx,
            report,
            AREA,
            "getMMS",
            GetMMSParams,
            GetMMSResponse,
            sms
        );
        skip_needs_input!(report, AREA, "getMediaMMS", "requires an MMS id");
    }
}
