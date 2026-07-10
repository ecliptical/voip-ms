//! The `voicemail` area: voicemail boxes and their setups, messages, and
//! transcriptions. The box and setup lists probe cleanly; the message,
//! message-file, and transcription reads need a mailbox (and folder/message
//! number, or a date window), so they are skipped at probe depth. The
//! create/set/del/mark/move writes are owned but run only at costly depth.
//! (Voicemail attachment formats and folder enumerations are reference-scope
//! lookups and live in the `reference` area.)

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Voicemail;

#[async_trait(?Send)]
impl Area for Voicemail {
    fn name(&self) -> &'static str {
        "voicemail"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "createVoicemail",
            "delMessages",
            "delVoicemail",
            "getVoicemailMessageFile",
            "getVoicemailMessages",
            "getVoicemailSetups",
            "getVoicemailTranscriptions",
            "getVoicemails",
            "markListenedVoicemailMessage",
            "markUrgentVoicemailMessage",
            "moveFolderVoicemailMessage",
            "sendVoicemailEmail",
            "setDIDVoicemail",
            "setVoicemail",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "voicemail";

        skip_needs_input!(
            report,
            AREA,
            "getVoicemailMessageFile",
            "requires a mailbox, folder, and message number"
        );
        skip_needs_input!(report, AREA, "getVoicemailMessages", "requires a mailbox");
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemailSetups",
            GetVoicemailSetupsParams,
            GetVoicemailSetupsResponse,
            voicemailsetups
        );
        skip_needs_input!(
            report,
            AREA,
            "getVoicemailTranscriptions",
            "requires an account and mailbox"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemails",
            GetVoicemailsParams,
            GetVoicemailsResponse,
            voicemails
        );
    }
}
