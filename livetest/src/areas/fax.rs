//! The `fax` area: fax numbers, folders, messages, and email-to-fax mappings.
//! Costly-by-nature because it owns `orderFaxNumber` and the per-page
//! `sendFaxMessage` (money), so it is excluded from the default set until
//! named. The folder/message/number/email-to-fax lists probe cleanly; the
//! reads that need a message id, a DID, or an area code are skipped at probe
//! depth. The order/send/set/del writes are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Fax;

#[async_trait(?Send)]
impl Area for Fax {
    fn name(&self) -> &'static str {
        "fax"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "cancelFaxNumber",
            "connectFAX",
            "delEmailToFax",
            "delFaxFolder",
            "deleteFaxMessage",
            "getEmailToFax",
            "getFaxFolders",
            "getFaxMessagePDF",
            "getFaxMessages",
            "getFaxNumbersInfo",
            "getFaxNumbersPortability",
            "mailFaxMessagePDF",
            "moveFaxMessage",
            "orderFaxNumber",
            "searchFaxAreaCodeCAN",
            "searchFaxAreaCodeUSA",
            "sendFaxMessage",
            "setEmailToFax",
            "setFaxFolder",
            "setFaxNumberEmail",
            "setFaxNumberInfo",
            "setFaxNumberURLCallback",
            "unconnectFAX",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "fax";

        probe_list!(
            ctx,
            report,
            AREA,
            "getEmailToFax",
            GetEmailToFAXParams,
            GetEmailToFAXResponse,
            emailToFax
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxFolders",
            GetFAXFoldersParams,
            GetFAXFoldersResponse,
            folders
        );
        skip_needs_input!(
            report,
            AREA,
            "getFaxMessagePDF",
            "requires a fax message id"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxMessages",
            GetFAXMessagesParams,
            GetFAXMessagesResponse,
            faxes
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxNumbersInfo",
            GetFAXNumbersInfoParams,
            GetFAXNumbersInfoResponse,
            numbers
        );
        skip_needs_input!(report, AREA, "getFaxNumbersPortability", "requires a DID");
        skip_needs_input!(
            report,
            AREA,
            "searchFaxAreaCodeCAN",
            "requires an area code"
        );
        skip_needs_input!(
            report,
            AREA,
            "searchFaxAreaCodeUSA",
            "requires an area code"
        );
    }
}
