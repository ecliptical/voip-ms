//! The `account` area: balance, deposits, charges, transaction history, server
//! info, and the account's egress IP. Costly-by-nature because it owns the
//! money-moving `addCharge`/`addPayment` methods (reachable only at costly
//! depth), so it is excluded from the default set until named. The read surface
//! is free: balance and server enumerations probe cleanly; the charge/deposit
//! and transaction-history reports are skipped at probe depth because they
//! require a client id or a date window.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, probe_scalar, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Account;

#[async_trait(?Send)]
impl Area for Account {
    fn name(&self) -> &'static str {
        "account"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "addCharge",
            "addPayment",
            "getBalance",
            "getBalanceManagement",
            "getCallTranscriptions",
            "getCharges",
            "getDeposits",
            "getIP",
            "getServersInfo",
            "getTransactionHistory",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "account";

        probe_scalar!(
            ctx,
            report,
            AREA,
            "getBalance",
            GetBalanceParams,
            GetBalanceResponse
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getBalanceManagement",
            GetBalanceManagementParams,
            GetBalanceManagementResponse,
            balance_management
        );
        skip_needs_input!(
            report,
            AREA,
            "getCallTranscriptions",
            "requires a date window"
        );
        skip_needs_input!(report, AREA, "getCharges", "requires a client id");
        skip_needs_input!(report, AREA, "getDeposits", "requires a client id");
        // getIP is exercised by the pre-run connectivity check; probing it again
        // here would double-count it, so the area owns it without re-probing.
        report.record(
            AREA,
            "getIP",
            crate::harness::Outcome::Skip("covered by the connectivity pre-check".to_string()),
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getServersInfo",
            GetServersInfoParams,
            GetServersInfoResponse,
            servers
        );
        skip_needs_input!(
            report,
            AREA,
            "getTransactionHistory",
            "requires a date window"
        );
    }
}
