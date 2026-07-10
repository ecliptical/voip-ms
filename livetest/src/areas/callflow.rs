//! The `callflow` area: the call-routing building blocks -- call hunting, call
//! parking, callbacks, caller-ID filtering, DISAs, music on hold, ring groups,
//! recordings, time conditions, and a queue's static members. The list-all
//! reads probe cleanly; the id- or date-scoped reads (`getCallRecording`,
//! `getCallRecordings`, `getRecordingFile`, `getStaticMembers`) are skipped at
//! probe depth. The set/del/create writes are owned but run only at costly
//! depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Callflow;

#[async_trait(?Send)]
impl Area for Callflow {
    fn name(&self) -> &'static str {
        "callflow"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delCallHunting",
            "delCallParking",
            "delCallRecording",
            "delCallback",
            "delCallerIDFiltering",
            "delDISA",
            "delMusicOnHold",
            "delRecording",
            "delRingGroup",
            "delStaticMember",
            "delTimeCondition",
            "getCallHuntings",
            "getCallParking",
            "getCallRecording",
            "getCallRecordings",
            "getCallbacks",
            "getCallerIDFiltering",
            "getDISAs",
            "getMusicOnHold",
            "getRecordingFile",
            "getRecordings",
            "getRingGroups",
            "getStaticMembers",
            "getTimeConditions",
            "sendCallRecordingEmail",
            "setCallHunting",
            "setCallParking",
            "setCallback",
            "setCallerIDFiltering",
            "setDISA",
            "setMusicOnHold",
            "setRecording",
            "setRingGroup",
            "setStaticMember",
            "setTimeCondition",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "callflow";

        probe_list!(
            ctx,
            report,
            AREA,
            "getCallHuntings",
            GetCallHuntingsParams,
            GetCallHuntingsResponse,
            call_hunting
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallParking",
            GetCallParkingParams,
            GetCallParkingResponse,
            call_hunting
        );
        skip_needs_input!(
            report,
            AREA,
            "getCallRecording",
            "requires a call-recording id"
        );
        skip_needs_input!(report, AREA, "getCallRecordings", "requires a date window");
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallbacks",
            GetCallbacksParams,
            GetCallbacksResponse,
            callbacks
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallerIDFiltering",
            GetCallerIDFilteringParams,
            GetCallerIDFilteringResponse,
            filtering
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getDISAs",
            GetDISAsParams,
            GetDISAsResponse,
            disa
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getMusicOnHold",
            GetMusicOnHoldParams,
            GetMusicOnHoldResponse,
            music_on_hold
        );
        skip_needs_input!(report, AREA, "getRecordingFile", "requires a recording id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getRecordings",
            GetRecordingsParams,
            GetRecordingsResponse,
            recordings
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getRingGroups",
            GetRingGroupsParams,
            GetRingGroupsResponse,
            ring_groups
        );
        skip_needs_input!(report, AREA, "getStaticMembers", "requires a queue id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getTimeConditions",
            GetTimeConditionsParams,
            GetTimeConditionsResponse,
            timecondition
        );
    }
}
