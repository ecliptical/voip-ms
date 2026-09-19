//! The `cdr` area: call detail records for the account and, for resellers,
//! their clients. Both methods require a date window (`date_from`/`date_to`),
//! which the harness can't supply at probe depth without choosing an arbitrary
//! range, so both are skipped there. The scope is read-only.
//!
//! At `Depth::Costly` the area supplies a real trailing-30-day window and
//! reads `getCDR` back through the typed probe -- a populated response is
//! where historical drift (e.g. call-date parsing) has lived, and the window
//! costs nothing to try regardless of whether any other costly fixture placed
//! a call today. It then reads the same window through `Client::get_cdr`
//! itself at two named zones, which is the only live exercise of the generated
//! wire twin, of `TimezoneOffset::at`, and of the offset the typed response
//! claims. `getResellerCDR` is left skipped: it additionally needs a reseller
//! client id the harness has no fixture for.

use async_trait::async_trait;

use crate::areas::probe_macros::skip_needs_input;
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back_zoned;
use crate::harness::{Outcome, Report};
use voip_ms::chrono_tz::Tz;
use voip_ms::{Client, GET_CDR_TIMESTAMPS, GetCDRParams, GetCDRResponse};

pub struct Cdr;

const AREA: &str = "cdr";

/// Zones the offset round-trip is checked at. `America/New_York` is a whole
/// hour and `Asia/Kolkata` a half hour: the wire carries `-4` for the first and
/// `5.50` for the second, and only a live run can show whether voip.ms keeps
/// the half hour or truncates it to `5`.
const ROUND_TRIP_ZONES: &[Tz] = &[
    voip_ms::chrono_tz::America::New_York,
    voip_ms::chrono_tz::Asia::Kolkata,
];

#[async_trait(?Send)]
impl Area for Cdr {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["getCDR", "getResellerCDR"]
    }

    async fn probe(&self, _ctx: &AreaCtx<'_>, report: &mut Report) {
        skip_needs_input!(report, AREA, "getCDR", "requires a date window");
        skip_needs_input!(report, AREA, "getResellerCDR", "requires a date window");
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let today = voip_ms::chrono::Local::now().date_naive();
        let date_from = today - voip_ms::chrono::Duration::days(30);
        let date_to = today;
        let params = GetCDRParams {
            date_from: Some(date_from),
            date_to: Some(date_to),
            // At least one call-status filter is required or VoIP.ms rejects
            // the request with `no_callstatus`.
            answered: Some(true),
            noanswer: Some(true),
            busy: Some(true),
            failed: Some(true),
            ..Default::default()
        };

        read_back_zoned::<_, GetCDRResponse>(
            ctx.client,
            report,
            AREA,
            "fixture:getCDR",
            &params,
            GET_CDR_TIMESTAMPS,
            |r| Some(r.cdr.len()),
        )
        .await;

        for tz in ROUND_TRIP_ZONES {
            let outcome = offset_round_trip(ctx.client, &params, *tz).await;
            report.record(AREA, &format!("fixture:getCDR:{tz}"), outcome);
        }

        report.record(
            AREA,
            "getResellerCDR",
            Outcome::Skip("requires a reseller client id".to_string()),
        );
    }
}

/// Read the same window twice through `Client::get_cdr`, at UTC and at `tz`,
/// and check that a record names the same instant both times.
///
/// The records are the same calls, so the instant cannot move with the zone the
/// caller asked for. It moves if voip.ms applies an offset other than the one
/// sent, which is what the typed `DateTime<FixedOffset>` would then be
/// asserting wrongly -- the case a fractional zone raises, since nothing else
/// confirms the server keeps a half hour.
///
/// Exercising it through the typed method rather than a call-by-name is the
/// point: this is the only live path through the generated `*ParamsWire`
/// conversion and the generated timestamp paths.
async fn offset_round_trip(client: &Client, params: &GetCDRParams, tz: Tz) -> Outcome {
    let at_utc = match client.get_cdr(params).await {
        Ok(r) => r,
        Err(error) => return Outcome::Fail(format!("getCDR at UTC: {error}")),
    };

    let zoned = GetCDRParams {
        timezone: Some(tz),
        ..params.clone()
    };
    let at_zone = match client.get_cdr(&zoned).await {
        Ok(r) => r,
        Err(error) => return Outcome::Fail(format!("getCDR at {tz}: {error}")),
    };

    let mut compared = 0;
    for utc_record in &at_utc.cdr {
        let (Some(id), Some(utc_date)) = (utc_record.uniqueid.as_deref(), utc_record.date) else {
            continue;
        };

        let Some(zone_date) = at_zone
            .cdr
            .iter()
            .find(|r| r.uniqueid.as_deref() == Some(id))
            .and_then(|r| r.date)
        else {
            continue;
        };

        if zone_date != utc_date {
            return Outcome::Fail(format!(
                "call {id} reads {utc_date} at UTC but {zone_date} at {tz}; voip.ms did not \
                 apply the offset the request carried, so the reported zone is wrong"
            ));
        }

        compared += 1;
    }

    if compared == 0 {
        return Outcome::Skip(format!(
            "no call in the window carries a date and an id at both UTC and {tz}"
        ));
    }

    Outcome::Pass
}
