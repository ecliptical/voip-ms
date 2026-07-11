//! The `reseller` area: reseller client management and per-client
//! balance/message/CDR reporting. Costly-by-nature because it owns
//! `signupClient` and the client-threshold controls that move reseller money,
//! so it is excluded from the default set until named. The client list, the
//! package catalog, and the reseller SMS/MMS lists probe cleanly; the
//! per-client package/threshold/balance reads need a client id, so they are
//! skipped at probe depth. `getResellerCDR` lives in the `cdr` area. The
//! set/del/signup writes are owned but run only at costly depth.
//!
//! At `Depth::Costly` the per-client reads run when `--reseller-client-id` is
//! supplied. `signupClient` creates a real billable client and has no dry-run,
//! so it is gated behind `--signup-reseller-client` plus a complete
//! `--signup-*` detail set, and creates the client inactive (`activate=0`) as a
//! conservative default. `setClient`/`setClientThreshold`/`delClient` mutate or
//! destroy an existing client with no dry-run and no dedicated input, so they
//! record skip (no input).

use async_trait::async_trait;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::{Depth, ResellerConfig};
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::probe::{ProbeOutcome, probe};
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Reseller;

const AREA: &str = "reseller";

#[async_trait(?Send)]
impl Area for Reseller {
    fn name(&self) -> &'static str {
        AREA
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
        probe_reseller::<_, GetResellerMMSResponse>(
            ctx,
            report,
            "getResellerMMS",
            &GetResellerMMSParams::default(),
            |r| Some(r.sms.len()),
        )
        .await;
        probe_reseller::<_, GetResellerSMSResponse>(
            ctx,
            report,
            "getResellerSMS",
            &GetResellerSMSParams::default(),
            |r| Some(r.sms.len()),
        )
        .await;
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let cfg = &ctx.config.reseller;

        client_id_reads(ctx, report, cfg.client_id.as_deref()).await;

        // signupClient is the one mutator wired here; it is gated and creates
        // the client inactive.
        signup_fixture(ctx, report, cfg).await;

        // The remaining mutators modify or delete an existing client with no
        // dry-run and no dedicated input; a real client id alone is not license
        // to overwrite or destroy it.
        for label in [
            "fixture:setClient",
            "fixture:setClientThreshold",
            "fixture:delClient",
        ] {
            skip_no_input(report, label);
        }
    }
}

/// The per-client reads. Each fires only when `--reseller-client-id` is
/// supplied, otherwise records skip (no input).
async fn client_id_reads(ctx: &AreaCtx<'_>, report: &mut Report, client_id: Option<&str>) {
    let Some(client) = client_id else {
        for label in [
            "fixture:getClientPackages",
            "fixture:getClientThreshold",
            "fixture:getResellerBalance",
        ] {
            skip_no_input(report, label);
        }

        return;
    };

    read_back::<_, GetClientPackagesResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getClientPackages",
        &GetClientPackagesParams {
            client: Some(client.to_string()),
        },
        |r| Some(r.packages.len()),
    )
    .await;

    read_back::<_, GetClientThresholdResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getClientThreshold",
        &GetClientThresholdParams {
            client: Some(client.to_string()),
        },
        |r| r.threshold_information.as_ref().map(|_| 1),
    )
    .await;

    read_back::<_, GetResellerBalanceResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getResellerBalance",
        &GetResellerBalanceParams {
            client: Some(client.to_string()),
        },
        |r| r.balance.as_ref().map(|_| 1),
    )
    .await;
}

async fn signup_fixture(ctx: &AreaCtx<'_>, report: &mut Report, cfg: &ResellerConfig) {
    if !cfg.signup {
        report.record(
            AREA,
            "fixture:signupClient",
            Outcome::Skip("--signup-reseller-client not set".to_string()),
        );
        return;
    }

    let Some(detail) = signup_detail(cfg) else {
        skip_no_input(report, "fixture:signupClient");
        return;
    };

    let result = ctx
        .client
        .signup_client(&SignupClientParams {
            firstname: Some(detail.first_name.clone()),
            lastname: Some(detail.last_name.clone()),
            email: Some(detail.email.clone()),
            confirm_email: Some(detail.email.clone()),
            password: Some(detail.password.clone()),
            confirm_password: Some(detail.password.clone()),
            address: Some(detail.address.clone()),
            city: Some(detail.city.clone()),
            state: Some(detail.state.clone()),
            country: Some(detail.country.clone()),
            zip: Some(detail.zip.clone()),
            phone_number: Some(detail.phone.clone()),
            activate: Some(false),
            ..Default::default()
        })
        .await;

    match result {
        Ok(resp) => {
            report.record(AREA, "fixture:signupClient", Outcome::Pass);
            if let Some(client) = resp.client {
                println!("[info] {AREA}/signupClient: created client {client} (inactive)");
            }
        }
        Err(error) => report.record(
            AREA,
            "fixture:signupClient",
            Outcome::Fail(format!("signupClient: {error}")),
        ),
    }
}

/// The hard-required `signupClient` fields, present only when every one was
/// supplied. An incomplete set yields `None` so the fixture records skip (no
/// input) rather than submitting a partial, rejected signup.
struct SignupDetail {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    address: String,
    city: String,
    state: String,
    country: String,
    zip: String,
    phone: String,
}

fn signup_detail(cfg: &ResellerConfig) -> Option<SignupDetail> {
    Some(SignupDetail {
        first_name: cfg.first_name.clone()?,
        last_name: cfg.last_name.clone()?,
        email: cfg.email.clone()?,
        password: cfg.password.clone()?,
        address: cfg.address.clone()?,
        city: cfg.city.clone()?,
        state: cfg.state.clone()?,
        country: cfg.country.clone()?,
        zip: cfg.zip.clone()?,
        phone: cfg.phone.clone()?,
    })
}

fn skip_no_input(report: &mut Report, label: &str) {
    report.record(AREA, label, Outcome::Skip("no input".to_string()));
}

/// Probe a reseller list method, folding `invalid_client` into a Skip. That
/// status here means the run account is not a reseller (issue #18) -- an
/// account-capability limitation, not a code or drift defect -- so the whole
/// reseller area is inapplicable rather than failing. Any other outcome (drift,
/// a different API error, transport) is recorded verbatim.
async fn probe_reseller<P, T>(
    ctx: &AreaCtx<'_>,
    report: &mut Report,
    method: &str,
    params: &P,
    count: impl Fn(&T) -> Option<usize>,
) where
    P: Serialize + Sync,
    T: DeserializeOwned,
{
    let outcome = probe::<P, T>(ctx.client, method, params, count).await;
    if let ProbeOutcome::ApiError(status) = &outcome
        && status == &ApiStatus::InvalidClient.to_string()
    {
        report.record(
            AREA,
            method,
            Outcome::Skip("account is not a reseller (invalid_client)".to_string()),
        );
        return;
    }

    report.record_probe(AREA, method, outcome);
}
