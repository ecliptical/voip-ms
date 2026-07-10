//! The `reference` area: static lookup tables and rate/ratecenter catalogs
//! (`getCountries`, `getStates`, codec/protocol/auth enumerations, carrier and
//! rate-center lists, ...). All read-only. The parameterless lookups probe
//! cleanly at every depth with no fixtures; the few that require a
//! province/state/type/package selector are skipped at probe depth (they need
//! an input the harness can't supply without a fixture). This area historically
//! surfaced list-element drift (`getNAT`, `getPlayInstructions`,
//! `getJoinWhenEmptyTypes` in 0.6.0).

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

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
            "getCallAccounts",
            "getCallBilling",
            "getCallTypes",
            "getCarriers",
            "getCountries",
            "getDIDCountries",
            "getDTMFModes",
            "getDeviceTypes",
            "getFaxProvinces",
            "getFaxRateCentersCAN",
            "getFaxRateCentersUSA",
            "getFaxStates",
            "getInternationalTypes",
            "getJoinWhenEmptyTypes",
            "getLanguages",
            "getLocales",
            "getLockInternational",
            "getNAT",
            "getPlayInstructions",
            "getProtocols",
            "getProvinces",
            "getRateCentersCAN",
            "getRateCentersUSA",
            "getRates",
            "getRingStrategies",
            "getRoutes",
            "getStates",
            "getTerminationRates",
            "getTimezones",
            "getVoicemailAttachmentFormats",
            "getVoicemailFolders",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "reference";

        probe_list!(
            ctx,
            report,
            AREA,
            "getAllowedCodecs",
            GetAllowedCodecsParams,
            GetAllowedCodecsResponse,
            allowed_codecs
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getAuthTypes",
            GetAuthTypesParams,
            GetAuthTypesResponse,
            auth_types
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallAccounts",
            GetCallAccountsParams,
            GetCallAccountsResponse,
            accounts
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallBilling",
            GetCallBillingParams,
            GetCallBillingResponse,
            call_billing
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallTypes",
            GetCallTypesParams,
            GetCallTypesResponse,
            call_types
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCarriers",
            GetCarriersParams,
            GetCarriersResponse,
            carriers
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCountries",
            GetCountriesParams,
            GetCountriesResponse,
            countries
        );
        skip_needs_input!(
            report,
            AREA,
            "getDIDCountries",
            "requires an international DID type"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getDTMFModes",
            GetDTMFModesParams,
            GetDTMFModesResponse,
            dtmf_modes
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getDeviceTypes",
            GetDeviceTypesParams,
            GetDeviceTypesResponse,
            device_types
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxProvinces",
            GetFAXProvincesParams,
            GetFAXProvincesResponse,
            provinces
        );
        skip_needs_input!(report, AREA, "getFaxRateCentersCAN", "requires a province");
        skip_needs_input!(report, AREA, "getFaxRateCentersUSA", "requires a state");
        probe_list!(
            ctx,
            report,
            AREA,
            "getFaxStates",
            GetFAXStatesParams,
            GetFAXStatesResponse,
            states
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getInternationalTypes",
            GetInternationalTypesParams,
            GetInternationalTypesResponse,
            types
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getJoinWhenEmptyTypes",
            GetJoinWhenEmptyTypesParams,
            GetJoinWhenEmptyTypesResponse,
            types
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getLanguages",
            GetLanguagesParams,
            GetLanguagesResponse,
            languages
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getLocales",
            GetLocalesParams,
            GetLocalesResponse,
            locales
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getLockInternational",
            GetLockInternationalParams,
            GetLockInternationalResponse,
            lock_international
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getNAT",
            GetNATParams,
            GetNATResponse,
            nat
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getPlayInstructions",
            GetPlayInstructionsParams,
            GetPlayInstructionsResponse,
            play_instructions
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getProtocols",
            GetProtocolsParams,
            GetProtocolsResponse,
            protocols
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getProvinces",
            GetProvincesParams,
            GetProvincesResponse,
            provinces
        );
        skip_needs_input!(report, AREA, "getRateCentersCAN", "requires a province");
        skip_needs_input!(report, AREA, "getRateCentersUSA", "requires a state");
        skip_needs_input!(report, AREA, "getRates", "requires a package");
        probe_list!(
            ctx,
            report,
            AREA,
            "getRingStrategies",
            GetRingStrategiesParams,
            GetRingStrategiesResponse,
            strategies
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getRoutes",
            GetRoutesParams,
            GetRoutesResponse,
            routes
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getStates",
            GetStatesParams,
            GetStatesResponse,
            states
        );
        skip_needs_input!(
            report,
            AREA,
            "getTerminationRates",
            "requires a route and query"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getTimezones",
            GetTimezonesParams,
            GetTimezonesResponse,
            timezones
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemailAttachmentFormats",
            GetVoicemailAttachmentFormatsParams,
            GetVoicemailAttachmentFormatsResponse,
            email_attachment_formats
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemailFolders",
            GetVoicemailFoldersParams,
            GetVoicemailFoldersResponse,
            folders
        );
    }
}
