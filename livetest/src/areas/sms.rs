//! The `sms` area: SMS messages. The message list probes cleanly;
//! `sendSMS`/`setSMS`/`deleteSMS` are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Sms;

#[async_trait(?Send)]
impl Area for Sms {
    fn name(&self) -> &'static str {
        "sms"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["deleteSMS", "getSMS", "sendSMS", "setSMS"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "sms";

        probe_list!(
            ctx,
            report,
            AREA,
            "getSMS",
            GetSMSParams,
            GetSMSResponse,
            sms
        );
    }
}
