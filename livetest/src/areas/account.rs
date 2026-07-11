//! The `account` area: balance, deposits, charges, transaction history, server
//! info, and the account's egress IP. Costly-by-nature because it owns the
//! money-moving `addCharge`/`addPayment` methods (reachable only at costly
//! depth), so it is excluded from the default set until named. The read surface
//! is free: balance and server enumerations probe cleanly; the charge/deposit
//! and transaction-history reports are skipped at probe depth because they
//! require a client id or a date window.
//!
//! At `Depth::Costly`, the input-gated reads run when their id/date-window is
//! supplied. `addCharge`/`addPayment` fire only with an explicit amount plus a
//! client id, and even then always in the API's `test` (dry-run) mode -- the
//! harness never moves real money. Absence of an amount is the first safety;
//! the unconditional `test=1` is the second.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, probe_scalar, skip_needs_input};
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Account;

const AREA: &str = "account";

#[async_trait(?Send)]
impl Area for Account {
    fn name(&self) -> &'static str {
        AREA
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

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let cfg = &ctx.config.account;

        // Input-gated reads: read-only, run whenever their required id or
        // date-window is supplied.
        match cfg.client_id {
            Some(client) => {
                read_back::<_, GetChargesResponse>(
                    ctx.client,
                    report,
                    AREA,
                    "fixture:getCharges",
                    &GetChargesParams {
                        client: Some(client.to_string()),
                    },
                    |r| Some(r.charges.len()),
                )
                .await;

                read_back::<_, GetDepositsResponse>(
                    ctx.client,
                    report,
                    AREA,
                    "fixture:getDeposits",
                    &GetDepositsParams {
                        client: Some(client.to_string()),
                    },
                    |r| Some(r.deposits.len()),
                )
                .await;
            }
            None => {
                skip_no_input(report, "fixture:getCharges");
                skip_no_input(report, "fixture:getDeposits");
            }
        }

        match (&cfg.transaction_date_from, &cfg.transaction_date_to) {
            (Some(from), Some(to)) => {
                read_back::<_, GetTransactionHistoryResponse>(
                    ctx.client,
                    report,
                    AREA,
                    "fixture:getTransactionHistory",
                    &GetTransactionHistoryParams {
                        date_from: Some(*from),
                        date_to: Some(*to),
                    },
                    |r| Some(r.transactions.len()),
                )
                .await;
            }
            _ => skip_no_input(report, "fixture:getTransactionHistory"),
        }

        // getCallTranscriptions needs a sub-account name plus a date window,
        // neither of which the harness supplies here.
        skip_no_input(report, "fixture:getCallTranscriptions");

        // Money-movers: an explicit amount plus a client id, always in the
        // API's dry-run (`test`) mode so no real money moves.
        payment_fixture(ctx, report).await;
        charge_fixture(ctx, report).await;
    }
}

async fn payment_fixture(ctx: &AreaCtx<'_>, report: &mut Report) {
    let cfg = &ctx.config.account;
    let (Some(client), Some(amount)) = (cfg.client_id, cfg.payment_amount) else {
        skip_no_input(report, "fixture:addPayment");
        return;
    };

    let result = ctx
        .client
        .add_payment(&AddPaymentParams {
            client: Some(client),
            payment: Some(amount),
            description: Some(ctx.token.marker(0)),
            test: true,
        })
        .await;

    match result {
        Ok(_) => report.record(AREA, "fixture:addPayment", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:addPayment",
            Outcome::Fail(format!("addPayment (test): {error}")),
        ),
    }
}

async fn charge_fixture(ctx: &AreaCtx<'_>, report: &mut Report) {
    let cfg = &ctx.config.account;
    let (Some(client), Some(amount)) = (cfg.client_id, cfg.charge_amount) else {
        skip_no_input(report, "fixture:addCharge");
        return;
    };

    let result = ctx
        .client
        .add_charge(&AddChargeParams {
            client: Some(client),
            charge: Some(amount),
            description: Some(ctx.token.marker(1)),
            test: true,
        })
        .await;

    match result {
        Ok(_) => report.record(AREA, "fixture:addCharge", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:addCharge",
            Outcome::Fail(format!("addCharge (test): {error}")),
        ),
    }
}

fn skip_no_input(report: &mut Report, label: &str) {
    report.record(AREA, label, Outcome::Skip("no input".to_string()));
}
