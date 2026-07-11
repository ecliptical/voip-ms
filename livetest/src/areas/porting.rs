//! The `porting` area: local number portability (LNP) port requests, their
//! details, notes, attachments, and status. Costly-by-nature because a port
//! submission commits to moving a number between carriers, so it is excluded
//! from the default set until named. Only the account-wide port-status summary
//! probes cleanly; every other read needs a port id, attachment id, or DID, so
//! they are skipped at probe depth. `addLNPPort`/`addLNPFile` are owned but run
//! only at costly depth.
//!
//! At `Depth::Costly` the read side runs whenever its id/DID input is supplied:
//! `getPortability` (a non-committing portability check for a DID) and the
//! id-scoped `getLNP*` reads. `addLNPPort` has no dry-run mode, so it is gated
//! behind `--submit-port` plus a complete `--port-*` detail set -- the flag and
//! the completeness of the inputs are the only safety. `addLNPFile` needs a
//! base64 attachment the harness has no input for, so it records skip (no
//! input).

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_scalar, skip_needs_input};
use crate::config::{Depth, PortingConfig};
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Porting;

const AREA: &str = "porting";

#[async_trait(?Send)]
impl Area for Porting {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::CostlyByNature
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "addLNPFile",
            "addLNPPort",
            "getLNPAttach",
            "getLNPAttachList",
            "getLNPDetails",
            "getLNPList",
            "getLNPListStatus",
            "getLNPNotes",
            "getLNPStatus",
            "getPortability",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        skip_needs_input!(report, AREA, "getLNPAttach", "requires an attachment id");
        skip_needs_input!(report, AREA, "getLNPAttachList", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPDetails", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPList", "requires a port id");
        probe_scalar!(
            ctx,
            report,
            AREA,
            "getLNPListStatus",
            GetLNPListStatusParams,
            GetLNPListStatusResponse
        );
        skip_needs_input!(report, AREA, "getLNPNotes", "requires a port id");
        skip_needs_input!(report, AREA, "getLNPStatus", "requires a port id");
        skip_needs_input!(report, AREA, "getPortability", "requires a DID");
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let cfg = &ctx.config.porting;

        // Read-side, non-committing: run whenever the DID/port-id input is
        // present.
        match &cfg.portability_did {
            Some(did) => {
                read_back::<_, GetPortabilityResponse>(
                    ctx.client,
                    report,
                    AREA,
                    "fixture:getPortability",
                    &GetPortabilityParams {
                        did: Some(did.clone()),
                    },
                    |r| Some(r.plans.len()),
                )
                .await;
            }
            None => skip_no_input(report, "fixture:getPortability"),
        }

        port_id_reads(ctx, report, cfg.port_id).await;

        // getLNPAttach needs an attachment id the harness has no input for.
        skip_no_input(report, "fixture:getLNPAttach");

        // Mutators: `addLNPPort` has no dry-run, so it is gated behind
        // `--submit-port` plus complete detail inputs.
        submit_port_fixture(ctx, report, cfg).await;

        // addLNPFile needs a base64 attachment the harness has no input for.
        skip_no_input(report, "fixture:addLNPFile");
    }
}

/// The id-scoped read side. Each fires only when `--port-id` is supplied,
/// otherwise records skip (no input).
async fn port_id_reads(ctx: &AreaCtx<'_>, report: &mut Report, port_id: Option<u64>) {
    let Some(portid) = port_id else {
        for label in [
            "fixture:getLNPDetails",
            "fixture:getLNPStatus",
            "fixture:getLNPNotes",
            "fixture:getLNPList",
            "fixture:getLNPAttachList",
        ] {
            skip_no_input(report, label);
        }

        return;
    };

    read_back::<_, GetLNPDetailsResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getLNPDetails",
        &GetLNPDetailsParams {
            portid: Some(portid),
        },
        |r| Some(r.numbers.len()),
    )
    .await;

    read_back::<_, GetLNPStatusResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getLNPStatus",
        &GetLNPStatusParams {
            portid: Some(portid),
        },
        |_| None,
    )
    .await;

    read_back::<_, GetLNPNotesResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getLNPNotes",
        &GetLNPNotesParams {
            portid: Some(portid),
        },
        |r| Some(r.list.len()),
    )
    .await;

    read_back::<_, GetLNPListResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getLNPList",
        &GetLNPListParams {
            portid: Some(portid),
            ..Default::default()
        },
        |r| Some(r.list.len()),
    )
    .await;

    read_back::<_, GetLNPAttachListResponse>(
        ctx.client,
        report,
        AREA,
        "fixture:getLNPAttachList",
        &GetLNPAttachListParams {
            portid: Some(portid),
        },
        |r| Some(r.list.len()),
    )
    .await;
}

async fn submit_port_fixture(ctx: &AreaCtx<'_>, report: &mut Report, cfg: &PortingConfig) {
    if !cfg.submit {
        report.record(
            AREA,
            "fixture:addLNPPort",
            Outcome::Skip("--submit-port not set".to_string()),
        );
        return;
    }

    let Some(detail) = port_detail(cfg) else {
        skip_no_input(report, "fixture:addLNPPort");
        return;
    };

    let result = ctx
        .client
        .add_lnp_port(&AddLNPPortParams {
            port_type: Some(detail.port_type),
            numbers: Some(detail.numbers.clone()),
            statement_name: Some(detail.statement_name.clone()),
            provider_name: Some(detail.provider_name.clone()),
            provider_account: Some(detail.provider_account.clone()),
            first_name: Some(detail.first_name.clone()),
            last_name: Some(detail.last_name.clone()),
            address1: Some(detail.address.clone()),
            city: Some(detail.city.clone()),
            state: Some(detail.state.clone()),
            zip: Some(detail.zip.clone()),
            country: Some(detail.country.clone()),
            notes: Some(ctx.token.marker(0)),
            ..Default::default()
        })
        .await;

    match result {
        Ok(resp) => {
            report.record(AREA, "fixture:addLNPPort", Outcome::Pass);
            if let Some(port) = resp.port {
                println!("[info] {AREA}/addLNPPort: submitted port {port}");
            }
        }
        Err(error) => report.record(
            AREA,
            "fixture:addLNPPort",
            Outcome::Fail(format!("addLNPPort: {error}")),
        ),
    }
}

/// The hard-required `addLNPPort` fields, present only when every one was
/// supplied. An incomplete set yields `None` so the fixture records skip (no
/// input) rather than submitting a partial, rejected port.
struct PortDetail {
    port_type: u64,
    numbers: String,
    statement_name: String,
    provider_name: String,
    provider_account: String,
    first_name: String,
    last_name: String,
    address: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

fn port_detail(cfg: &PortingConfig) -> Option<PortDetail> {
    Some(PortDetail {
        port_type: cfg.port_type?,
        numbers: cfg.numbers.clone()?,
        statement_name: cfg.statement_name.clone()?,
        provider_name: cfg.provider_name.clone()?,
        provider_account: cfg.provider_account.clone()?,
        first_name: cfg.first_name.clone()?,
        last_name: cfg.last_name.clone()?,
        address: cfg.address.clone()?,
        city: cfg.city.clone()?,
        state: cfg.state.clone()?,
        zip: cfg.zip.clone()?,
        country: cfg.country.clone()?,
    })
}

fn skip_no_input(report: &mut Report, label: &str) {
    report.record(AREA, label, Outcome::Skip("no input".to_string()));
}
