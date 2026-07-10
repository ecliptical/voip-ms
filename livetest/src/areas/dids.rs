//! The `dids` area: the account's DIDs and vPRIs, plus the availability
//! searches. The list-all reads (`getDIDsInfo`, `getBackOrders`, `getVPRIs`)
//! and the toll-free availability searches probe cleanly; the geography- or
//! id-scoped lookups (`getDIDsCAN`/`USA`, the international catalogs, `getDIDvPRI`,
//! and the criteria-driven searches) are skipped at probe depth. The ordering
//! and routing writes are owned but run only at costly depth. Kept in the
//! default (free) set: DID ordering is costly by depth, not the area's read
//! nature, and `getDIDsInfo` is the harness's bread-and-butter probe.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Dids;

#[async_trait(?Send)]
impl Area for Dids {
    fn name(&self) -> &'static str {
        "dids"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "assignDIDvPRI",
            "backOrderDIDCAN",
            "backOrderDIDUSA",
            "cancelDID",
            "connectDID",
            "getBackOrders",
            "getDIDsCAN",
            "getDIDsInfo",
            "getDIDsInternationalGeographic",
            "getDIDsInternationalNational",
            "getDIDsInternationalTollFree",
            "getDIDsUSA",
            "getDIDvPRI",
            "getVPRIs",
            "orderDID",
            "orderDIDInternationalGeographic",
            "orderDIDInternationalNational",
            "orderDIDInternationalTollFree",
            "orderDIDVirtual",
            "orderTollFree",
            "orderVanity",
            "removeDIDvPRI",
            "searchDIDsCAN",
            "searchDIDsUSA",
            "searchTollFreeCanUS",
            "searchTollFreeUSA",
            "searchVanity",
            "setDIDBillingType",
            "setDIDInfo",
            "setDIDPOP",
            "setDIDRouting",
            "unconnectDID",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "dids";

        probe_list!(
            ctx,
            report,
            AREA,
            "getBackOrders",
            GetBackOrdersParams,
            GetBackOrdersResponse,
            back_orders
        );
        skip_needs_input!(report, AREA, "getDIDsCAN", "requires a province");
        probe_list!(
            ctx,
            report,
            AREA,
            "getDIDsInfo",
            GetDIDsInfoParams,
            GetDIDsInfoResponse,
            dids
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalGeographic",
            "requires a country id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalNational",
            "requires a country id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDsInternationalTollFree",
            "requires a country id"
        );
        skip_needs_input!(report, AREA, "getDIDsUSA", "requires a state");
        skip_needs_input!(report, AREA, "getDIDvPRI", "requires a vPRI id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getVPRIs",
            GetVPRIsParams,
            GetVPRIsResponse,
            vpri
        );
        skip_needs_input!(
            report,
            AREA,
            "searchDIDsCAN",
            "requires search type and query"
        );
        skip_needs_input!(
            report,
            AREA,
            "searchDIDsUSA",
            "requires search type and query"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "searchTollFreeCanUS",
            SearchTollFreeCANUSParams,
            SearchTollFreeCANUSResponse,
            dids
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "searchTollFreeUSA",
            SearchTollFreeUSAParams,
            SearchTollFreeUSAResponse,
            dids
        );
        skip_needs_input!(
            report,
            AREA,
            "searchVanity",
            "requires vanity type and query"
        );
    }
}
