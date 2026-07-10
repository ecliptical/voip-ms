//! The `conference` area: conference bridges and their members. The bridge and
//! member lists probe cleanly; the recording reads require a conference id (and
//! `getConferenceRecordingFile` a recording id too), so they are skipped at
//! probe depth. The set/del/member writes are owned but run only at costly
//! depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Conference;

#[async_trait(?Send)]
impl Area for Conference {
    fn name(&self) -> &'static str {
        "conference"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "addMemberToConference",
            "delConference",
            "delConferenceMember",
            "delMemberFromConference",
            "getConference",
            "getConferenceMembers",
            "getConferenceRecordingFile",
            "getConferenceRecordings",
            "setConference",
            "setConferenceMember",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "conference";

        probe_list!(
            ctx,
            report,
            AREA,
            "getConference",
            GetConferenceParams,
            GetConferenceResponse,
            conference
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getConferenceMembers",
            GetConferenceMembersParams,
            GetConferenceMembersResponse,
            members
        );
        skip_needs_input!(
            report,
            AREA,
            "getConferenceRecordingFile",
            "requires a conference and recording id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getConferenceRecordings",
            "requires a conference id"
        );
    }
}
