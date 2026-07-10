//! The `e911` area: emergency-service address provisioning for DIDs.
//! Costly-by-nature because provisioning carries a fee and records a physical
//! address (irreversible, sensitive), so it is excluded from the default set
//! until named. Only the address-type enumeration probes cleanly; `e911Info`
//! needs a DID and `e911Validate` a full address, so both are skipped at probe
//! depth. The provision/update/cancel writes are owned but run only at costly
//! depth.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct E911;

#[async_trait(?Send)]
impl Area for E911 {
    fn name(&self) -> &'static str {
        "e911"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "e911AddressTypes",
            "e911Cancel",
            "e911Info",
            "e911Provision",
            "e911ProvisionManually",
            "e911Update",
            "e911Validate",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "e911";

        probe_list!(
            ctx,
            report,
            AREA,
            "e911AddressTypes",
            E911AddressTypesParams,
            E911AddressTypesResponse,
            types
        );
        skip_needs_input!(report, AREA, "e911Info", "requires an e911-enabled DID");
        skip_needs_input!(
            report,
            AREA,
            "e911Validate",
            "requires a DID and a full address"
        );
    }
}
