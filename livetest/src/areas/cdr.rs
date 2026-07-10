//! The `cdr` area: call detail records for the account and, for resellers,
//! their clients. Both methods require a date window (`date_from`/`date_to`),
//! which the harness can't supply at probe depth without choosing an arbitrary
//! range, so both are skipped there. The scope is read-only.

use async_trait::async_trait;

use crate::areas::probe_macros::skip_needs_input;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};

pub struct Cdr;

#[async_trait(?Send)]
impl Area for Cdr {
    fn name(&self) -> &'static str {
        "cdr"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["getCDR", "getResellerCDR"]
    }

    async fn probe(&self, _ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "cdr";

        skip_needs_input!(report, AREA, "getCDR", "requires a date window");
        skip_needs_input!(report, AREA, "getResellerCDR", "requires a date window");
    }
}
