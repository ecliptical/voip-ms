//! The `subaccount` area: sub-accounts, their SIP URIs, and the account's
//! configured locations. The sub-account, SIP-URI, and location lists probe
//! cleanly; the registration-status read needs a specific account, so it is
//! skipped at probe depth. The create/set/del writes are owned but run only at
//! costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Subaccount;

#[async_trait(?Send)]
impl Area for Subaccount {
    fn name(&self) -> &'static str {
        "subaccount"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "createSubAccount",
            "delLocation",
            "delSIPURI",
            "delSubAccount",
            "getLocations",
            "getRegistrationStatus",
            "getSIPURIs",
            "getSubAccounts",
            "setLocation",
            "setSIPURI",
            "setSubAccount",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "subaccount";

        probe_list!(
            ctx,
            report,
            AREA,
            "getLocations",
            GetLocationsParams,
            GetLocationsResponse,
            locations
        );
        skip_needs_input!(
            report,
            AREA,
            "getRegistrationStatus",
            "requires a specific account"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getSIPURIs",
            GetSIPURIsParams,
            GetSIPURIsResponse,
            sipuris
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getSubAccounts",
            GetSubAccountsParams,
            GetSubAccountsResponse,
            accounts
        );
    }
}
