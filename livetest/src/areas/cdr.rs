//! The `cdr` area: call detail records for the account and, for resellers,
//! their clients. Both methods require a date window (`date_from`/`date_to`),
//! which the harness can't supply at probe depth without choosing an arbitrary
//! range, so both are skipped there. The scope is read-only.
//!
//! At `Depth::Costly` the area supplies a real trailing-30-day window and
//! reads `getCDR` back through the typed probe -- a populated response is
//! where historical drift (e.g. call-date parsing) has lived, and the window
//! costs nothing to try regardless of whether any other costly fixture placed
//! a call today. `getResellerCDR` is left skipped: it additionally needs a
//! reseller client id the harness has no fixture for.

use async_trait::async_trait;

use crate::areas::probe_macros::skip_needs_input;
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::{Outcome, Report};
use voip_ms::{GetCDRParams, GetCDRResponse};

pub struct Cdr;

const AREA: &str = "cdr";

#[async_trait(?Send)]
impl Area for Cdr {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["getCDR", "getResellerCDR"]
    }

    async fn probe(&self, _ctx: &AreaCtx<'_>, report: &mut Report) {
        skip_needs_input!(report, AREA, "getCDR", "requires a date window");
        skip_needs_input!(report, AREA, "getResellerCDR", "requires a date window");
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let today = voip_ms::chrono::Local::now().date_naive();
        let date_from = today - voip_ms::chrono::Duration::days(30);
        let date_to = today;

        read_back::<_, GetCDRResponse>(
            ctx.client,
            report,
            AREA,
            "fixture:getCDR",
            &GetCDRParams {
                date_from: Some(date_from),
                date_to: Some(date_to),
                timezone: Some(voip_ms::rust_decimal::Decimal::ZERO),
                ..Default::default()
            },
            |r| Some(r.cdr.len()),
        )
        .await;

        report.record(
            AREA,
            "getResellerCDR",
            Outcome::Skip("requires a reseller client id".to_string()),
        );
    }
}
