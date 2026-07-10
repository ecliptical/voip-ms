//! The `reference` area: static lookup tables (`getCountries`, `getStates`,
//! codec/protocol/auth enumerations, ...). All read-only, all parameterless
//! (every param is optional), so they probe cleanly at every depth with no
//! fixtures and nothing to sweep. This area proves the probe pipeline
//! end-to-end and historically surfaced list-element drift (`getNAT`,
//! `getPlayInstructions`, `getJoinWhenEmptyTypes` in 0.6.0).

use async_trait::async_trait;

use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::{Report, probe};
use voip_ms::*;

/// Probe one reference method: build default params, call typed-over-raw, and
/// count the single list field. Keeps the ~15 near-identical probes to one
/// line each.
macro_rules! probe_ref {
    ($ctx:expr, $report:expr, $wire:literal, $params:ty, $resp:ty, $field:ident) => {{
        let outcome = probe::<$params, $resp>($ctx.client, $wire, &<$params>::default(), |r| {
            Some(r.$field.len())
        })
        .await;
        $report.record_probe("reference", $wire, outcome);
    }};
}

pub struct Reference;

#[async_trait(?Send)]
impl Area for Reference {
    fn name(&self) -> &'static str {
        "reference"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "getAllowedCodecs",
            "getAuthTypes",
            "getCallTypes",
            "getCountries",
            "getDTMFModes",
            "getDeviceTypes",
            "getInternationalTypes",
            "getJoinWhenEmptyTypes",
            "getLanguages",
            "getLocales",
            "getNAT",
            "getPlayInstructions",
            "getProtocols",
            "getProvinces",
            "getRingStrategies",
            "getRoutes",
            "getStates",
            "getTimezones",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_ref!(
            ctx,
            report,
            "getAllowedCodecs",
            GetAllowedCodecsParams,
            GetAllowedCodecsResponse,
            allowed_codecs
        );
        probe_ref!(
            ctx,
            report,
            "getAuthTypes",
            GetAuthTypesParams,
            GetAuthTypesResponse,
            auth_types
        );
        probe_ref!(
            ctx,
            report,
            "getCallTypes",
            GetCallTypesParams,
            GetCallTypesResponse,
            call_types
        );
        probe_ref!(
            ctx,
            report,
            "getCountries",
            GetCountriesParams,
            GetCountriesResponse,
            countries
        );
        probe_ref!(
            ctx,
            report,
            "getDTMFModes",
            GetDTMFModesParams,
            GetDTMFModesResponse,
            dtmf_modes
        );
        probe_ref!(
            ctx,
            report,
            "getDeviceTypes",
            GetDeviceTypesParams,
            GetDeviceTypesResponse,
            device_types
        );
        probe_ref!(
            ctx,
            report,
            "getInternationalTypes",
            GetInternationalTypesParams,
            GetInternationalTypesResponse,
            types
        );
        probe_ref!(
            ctx,
            report,
            "getJoinWhenEmptyTypes",
            GetJoinWhenEmptyTypesParams,
            GetJoinWhenEmptyTypesResponse,
            types
        );
        probe_ref!(
            ctx,
            report,
            "getLanguages",
            GetLanguagesParams,
            GetLanguagesResponse,
            languages
        );
        probe_ref!(
            ctx,
            report,
            "getLocales",
            GetLocalesParams,
            GetLocalesResponse,
            locales
        );
        probe_ref!(ctx, report, "getNAT", GetNATParams, GetNATResponse, nat);
        probe_ref!(
            ctx,
            report,
            "getPlayInstructions",
            GetPlayInstructionsParams,
            GetPlayInstructionsResponse,
            play_instructions
        );
        probe_ref!(
            ctx,
            report,
            "getProtocols",
            GetProtocolsParams,
            GetProtocolsResponse,
            protocols
        );
        probe_ref!(
            ctx,
            report,
            "getProvinces",
            GetProvincesParams,
            GetProvincesResponse,
            provinces
        );
        probe_ref!(
            ctx,
            report,
            "getRingStrategies",
            GetRingStrategiesParams,
            GetRingStrategiesResponse,
            strategies
        );
        probe_ref!(
            ctx,
            report,
            "getRoutes",
            GetRoutesParams,
            GetRoutesResponse,
            routes
        );
        probe_ref!(
            ctx,
            report,
            "getStates",
            GetStatesParams,
            GetStatesResponse,
            states
        );
        probe_ref!(
            ctx,
            report,
            "getTimezones",
            GetTimezonesParams,
            GetTimezonesResponse,
            timezones
        );
    }
}
