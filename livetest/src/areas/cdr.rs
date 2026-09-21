//! The `cdr` area: call detail records for the account and, for resellers,
//! their clients. Both methods require a date window (`date_from`/`date_to`),
//! which the harness supplies as a trailing 30 days. The scope is read-only.
//!
//! `getCDR` reads that window at every depth. The range is arbitrary, which is
//! why probe depth once skipped it, but the call is free and read-only, and a
//! *populated* response is the harness's highest-value one: it is where drift
//! has historically lived (call-date parsing), and it is the only place the key
//! diff sees the per-record fields -- two of which (`ip`, `useragent`) reached
//! production undeclared because nothing ever read a real record.
//!
//! At `Depth::Costly` the area then reads the same window through
//! `Client::get_cdr` itself at two named zones, which is the only live exercise
//! of the generated wire twin, of `TimezoneOffset::at`, and of the offset the
//! typed response claims. `getResellerCDR` is left skipped: it additionally
//! needs a reseller client id the harness has no fixture for.

use async_trait::async_trait;

use crate::areas::probe_macros::skip_needs_input;
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back_zoned;
use crate::harness::{Outcome, Report};
use voip_ms::chrono_tz::Tz;
use voip_ms::{Client, Error, GET_CDR_TIMESTAMPS, GetCDRParams, GetCDRResponse};

pub struct Cdr;

const AREA: &str = "cdr";

/// Zones the offset round-trip is checked at. `America/New_York` is a
/// whole-hour zone that shifts with DST, so its wire value depends on when the
/// run happens; `Asia/Kolkata` sits on a half hour, and only a live run can
/// show whether voip.ms keeps that half hour or truncates it away.
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

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        read_back_zoned::<_, GetCDRResponse>(
            ctx.client,
            report,
            AREA,
            "getCDR",
            &window_params(),
            GET_CDR_TIMESTAMPS,
            |r| Some(r.cdr.len()),
        )
        .await;

        skip_needs_input!(
            report,
            AREA,
            "getResellerCDR",
            "requires a reseller client id"
        );
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        // The probe already read a window like this at every depth, so what is
        // left here is the part only costly depth adds. Every read below shares
        // this one value, so the zones are compared over identical rows.
        let params = window_params();

        // One UTC read shared by every zone compared against it, rather than one
        // per zone over identical rows.
        let at_utc = typed_get_cdr(ctx.client, &params).await;
        for tz in ROUND_TRIP_ZONES {
            // Every zone reports under its own key whatever happens, so a
            // failed UTC read fails these checks rather than removing them --
            // a vanished key reads as a check that stopped existing.
            let outcome = match &at_utc {
                Ok(at_utc) => offset_round_trip(ctx.client, &params, at_utc, *tz).await,
                Err(outcome) => outcome.clone(),
            };

            report.record(AREA, &format!("fixture:getCDR:{tz}"), outcome);
        }
    }
}

/// A trailing 30 days ending today, with all four call statuses, since VoIP.ms
/// rejects a request naming none (`no_callstatus`).
///
/// Each caller gets its own window from the clock at that moment, so a run
/// crossing local midnight between the probe and the costly fixtures reads two
/// spans a day apart. That costs nothing here: the probe's read and the
/// round-trip's reads are each self-contained, and the round-trip compares zones
/// within one window it holds for the duration.
fn window_params() -> GetCDRParams {
    let today = voip_ms::chrono::Local::now().date_naive();
    GetCDRParams {
        date_from: Some(today - voip_ms::chrono::Duration::days(30)),
        date_to: Some(today),
        answered: Some(true),
        noanswer: Some(true),
        busy: Some(true),
        failed: Some(true),
        ..Default::default()
    }
}

/// Read the window at `tz` through `Client::get_cdr` and check that a record
/// names the same instant it does in `at_utc`.
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
async fn offset_round_trip(
    client: &Client,
    params: &GetCDRParams,
    at_utc: &GetCDRResponse,
    tz: Tz,
) -> Outcome {
    let zoned = GetCDRParams {
        timezone: Some(tz),
        ..params.clone()
    };
    let at_zone = match typed_get_cdr(client, &zoned).await {
        Ok(r) => r,
        Err(outcome) => return outcome,
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

/// `Client::get_cdr`, with a shape mismatch reported as drift rather than as a
/// plain failure.
///
/// Every other read in this harness goes through [`ProbeOutcome`] so that
/// raw-succeeded-typed-failed lands in the DRIFT bucket with the envelope to
/// paste into an override. The typed method cannot say that on its own:
/// `Error::InvalidResponse` covers a body that is not JSON and an envelope with
/// no `status` as well as the shape mismatch, and those two are transport-class
/// anomalies the probe deliberately keeps out of DRIFT. Re-fetching the raw
/// envelope separates them -- a raw call that succeeds proves the envelope was
/// well formed and the typed step is what failed.
async fn typed_get_cdr(client: &Client, params: &GetCDRParams) -> Result<GetCDRResponse, Outcome> {
    let error = match client.get_cdr(params).await {
        Ok(response) => return Ok(response),
        Err(Error::InvalidResponse(error)) => error,
        Err(error) => return Err(Outcome::Fail(format!("getCDR: {error}"))),
    };

    match client.get_cdr_raw(params).await {
        Ok(body) => Err(Outcome::Drift {
            error,
            raw_json: serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()),
        }),
        // The raw path rejecting the envelope too is what rules drift out. Any
        // other refetch failure answers nothing -- a timeout on the second
        // request says only that the second request timed out -- so it must not
        // read as "this is not drift" and send the operator elsewhere.
        Err(Error::InvalidResponse(raw_error)) => Err(Outcome::Fail(format!(
            "getCDR returned an envelope the raw path rejects too, so this is not \
             response-shape drift: typed `{error}`, raw `{raw_error}`"
        ))),
        Err(raw_error) => Err(Outcome::Fail(format!(
            "getCDR failed to deserialize and the refetch that would classify it \
             did not complete, so drift is neither shown nor ruled out: typed \
             `{error}`, refetch `{raw_error}`"
        ))),
    }
}
