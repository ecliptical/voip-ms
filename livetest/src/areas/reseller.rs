//! The `reseller` area: reseller client management and per-client
//! balance/message/CDR reporting. Costly-by-nature because it owns
//! `signupClient` and the client-threshold controls that move reseller money,
//! so it is excluded from the default set until named. The client list, the
//! package catalog, and the reseller SMS/MMS lists probe cleanly; the
//! per-client package/threshold/balance reads need a client id, so they are
//! skipped at probe depth. `getResellerCDR` lives in the `cdr` area. The
//! set/del/signup writes are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Reseller;

#[async_trait(?Send)]
impl Area for Reseller {
    fn name(&self) -> &'static str {
        "reseller"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delClient",
            "getClientPackages",
            "getClientThreshold",
            "getClients",
            "getPackages",
            "getResellerBalance",
            "getResellerMMS",
            "getResellerSMS",
            "setClient",
            "setClientThreshold",
            "signupClient",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "reseller";

        skip_needs_input!(report, AREA, "getClientPackages", "requires a client id");
        skip_needs_input!(report, AREA, "getClientThreshold", "requires a client id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getClients",
            GetClientsParams,
            GetClientsResponse,
            clients
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getPackages",
            GetPackagesParams,
            GetPackagesResponse,
            packages
        );
        skip_needs_input!(report, AREA, "getResellerBalance", "requires a client id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getResellerMMS",
            GetResellerMMSParams,
            GetResellerMMSResponse,
            sms
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getResellerSMS",
            GetResellerSMSParams,
            GetResellerSMSResponse,
            sms
        );
    }
}
