//! The `porting` area: local number portability (LNP) port requests, their
//! details, notes, attachments, and status. Costly-by-nature because a port
//! submission commits to moving a number between carriers, so it is excluded
//! from the default set until named. Only the account-wide port-status summary
//! probes cleanly; every other read needs a port id, attachment id, or DID, so
//! they are skipped at probe depth. `addLNPPort`/`addLNPFile` are owned but run
//! only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_scalar, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Porting;

#[async_trait(?Send)]
impl Area for Porting {
    fn name(&self) -> &'static str {
        "porting"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "addLNPFile",
            "addLNPPort",
            "getLNPAttach",
            "getLNPAttachList",
            "getLNPDetails",
            "getLNPList",
            "getLNPListStatus",
            "getLNPNotes",
            "getLNPStatus",
            "getPortability",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "porting";

        skip_needs_input!(report, AREA, "getLNPAttach", "requires an attachment id");
        skip_needs_input!(report, AREA, "getLNPAttachList", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPDetails", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPList", "requires a port id");
        probe_scalar!(
            ctx,
            report,
            AREA,
            "getLNPListStatus",
            GetLNPListStatusParams,
            GetLNPListStatusResponse
        );
        skip_needs_input!(report, AREA, "getLNPNotes", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPStatus", "requires a port id");
        skip_needs_input!(report, AREA, "getPortability", "requires a DID");
    }
}
