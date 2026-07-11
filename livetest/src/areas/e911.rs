//! The `e911` area: emergency-service address provisioning for DIDs.
//! Costly-by-nature because provisioning carries a fee and records a physical
//! address (irreversible, sensitive), so it is excluded from the default set
//! until named. Only the address-type enumeration probes cleanly; `e911Info`
//! needs a DID and `e911Validate` a full address, so both are skipped at probe
//! depth. The provision/update/cancel writes are owned but run only at costly
//! depth.
//!
//! At `Depth::Costly`, the safe path runs first and unconditionally when a DID
//! plus a complete address is supplied: `e911Validate` checks the address
//! without provisioning anything. Provisioning is gated behind `--e911-provision`
//! -- only then does the fixture run `e911Provision -> e911Info -> e911Update`
//! and cancel via `e911Cancel`. Validation is deliberately preferred over
//! provisioning. `e911ProvisionManually` (the manual-review provisioning
//! variant) is not fired: `e911Provision` is the chosen provisioning path, and
//! provisioning the same DID twice would leak a second address.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::{Depth, E911Config};
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct E911;

const AREA: &str = "e911";

#[async_trait(?Send)]
impl Area for E911 {
    fn name(&self) -> &'static str {
        AREA
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

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        // The manual-review provisioning variant is never fired: `e911Provision`
        // is the chosen provisioning path, and provisioning the same DID twice
        // would leak a second address.
        report.record(
            AREA,
            "fixture:e911ProvisionManually",
            Outcome::Skip("e911Provision is the chosen provisioning path".to_string()),
        );

        let cfg = &ctx.config.e911;
        let Some(address) = e911_address(cfg) else {
            report.record(
                AREA,
                "fixture:e911Validate",
                Outcome::Skip("--e911-did and address inputs not set".to_string()),
            );
            report.record(
                AREA,
                "fixture:e911Provision",
                Outcome::Skip("--e911-did and address inputs not set".to_string()),
            );
            return;
        };

        // Validate-only always runs: it provisions nothing.
        validate_fixture(ctx, report, &address).await;

        if !cfg.provision {
            report.record(
                AREA,
                "fixture:e911Provision",
                Outcome::Skip("--e911-provision not set (validate-only)".to_string()),
            );
            return;
        }

        provision_fixture(ctx, report, &address).await;
    }
}

/// A complete e911 address, present only when every required field was
/// supplied. Assembled from [`E911Config`]; an incomplete address yields `None`
/// so the fixture records skip (no input) rather than sending a partial call.
struct E911Address {
    did: String,
    full_name: String,
    street_number: i64,
    street_name: String,
    city: String,
    state: String,
    country: String,
    zip: String,
    language: Option<String>,
}

fn e911_address(cfg: &E911Config) -> Option<E911Address> {
    Some(E911Address {
        did: cfg.did.clone()?,
        full_name: cfg.full_name.clone()?,
        street_number: cfg.street_number?,
        street_name: cfg.street_name.clone()?,
        city: cfg.city.clone()?,
        state: cfg.state.clone()?,
        country: cfg.country.clone()?,
        zip: cfg.zip.clone()?,
        language: cfg.language.clone(),
    })
}

async fn validate_fixture(ctx: &AreaCtx<'_>, report: &mut Report, address: &E911Address) {
    let result = ctx
        .client
        .e911_validate(&E911ValidateParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip: Some(address.zip.clone()),
            language: address.language.clone(),
            ..Default::default()
        })
        .await;

    match result {
        Ok(_) => report.record(AREA, "fixture:e911Validate", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:e911Validate",
            Outcome::Fail(format!("e911Validate: {error}")),
        ),
    }
}

/// Provision the address, read it back typed, update it, then cancel. The
/// cancel is deferred immediately after a successful provision so it fires
/// exactly once on every path -- including when the read-back or update fails.
async fn provision_fixture(ctx: &AreaCtx<'_>, report: &mut Report, address: &E911Address) {
    let client = ctx.client;

    let provisioned = client
        .e911_provision(&E911ProvisionParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip: Some(address.zip.clone()),
            language: address.language.clone(),
            ..Default::default()
        })
        .await;

    if let Err(error) = provisioned {
        return fail(
            report,
            "fixture:e911Provision",
            &format!("e911Provision: {error}"),
        );
    }

    report.record(AREA, "fixture:e911Provision", Outcome::Pass);

    let mut scope = Scope::new();
    let cancel_did = address.did.clone();
    scope.defer(format!("e911 did={cancel_did}"), move |client| {
        Box::pin(async move {
            client
                .e911_cancel(&E911CancelParams {
                    did: Some(cancel_did),
                })
                .await?;
            Ok(())
        })
    });

    read_back::<_, E911InfoResponse>(
        client,
        report,
        AREA,
        "fixture:e911Info",
        &E911InfoParams {
            did: Some(address.did.clone()),
        },
        |r| r.info.as_ref().map(|_| 1),
    )
    .await;

    match client
        .e911_update(&E911UpdateParams {
            did: Some(address.did.clone()),
            full_name: Some(address.full_name.clone()),
            street_number: Some(address.street_number),
            street_name: Some(address.street_name.clone()),
            city: Some(address.city.clone()),
            state: Some(address.state.clone()),
            country: Some(address.country.clone()),
            zip: Some(address.zip.clone()),
            language: address.language.clone(),
            ..Default::default()
        })
        .await
    {
        Ok(_) => report.record(AREA, "fixture:e911Update", Outcome::Pass),
        Err(error) => report.record(
            AREA,
            "fixture:e911Update",
            Outcome::Fail(format!("e911Update: {error}")),
        ),
    }

    for label in scope.cleanup(client).await {
        report.record(
            AREA,
            "cleanup",
            Outcome::Fail(format!("teardown failed for {label}")),
        );
    }
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}
