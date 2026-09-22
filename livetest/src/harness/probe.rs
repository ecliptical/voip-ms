//! The raw-vs-typed drift probe.
//!
//! Every drift bug in this crate's history is the same shape: the live API
//! returns JSON that the generated `*Response` type can't deserialize, turning
//! a *successful* upstream call into a serde error. The probe isolates exactly
//! that step -- fetch the raw envelope, then attempt the typed deserialization
//! over it -- so a failure is unambiguously attributable to response-shape
//! drift (raw ok, typed fails) rather than to the network or a real API error.
//!
//! A typed deserialization that *succeeds* still proves nothing about fidelity,
//! so the same raw envelope also goes through the key diff in
//! [`super::keydiff`], which reports what the typed shape silently dropped --
//! and the deserialized value is scanned for the tolerant types' catch-all
//! variants, which is what a *value* the crate cannot read now looks like.
//!
//! Those three checks answer three different questions: drift is "the crate
//! could not read the envelope", unmodeled is "the crate dropped part of it",
//! degraded is "the crate read it but did not understand one value".

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use voip_ms::{Client, Error, TimezoneOffset, attach_offset};

use crate::harness::keydiff;
use crate::response_fields;

/// The result of probing one method.
pub enum ProbeOutcome {
    /// Raw succeeded and the typed shape deserialized. `element_count` is set
    /// when the response's primary payload is a list, for logging;
    /// `unmodeled` holds the live key paths no `*Response` field claims.
    Ok {
        element_count: Option<usize>,
        unmodeled: Vec<String>,
        degraded: Vec<String>,
    },
    /// Raw succeeded but the typed deserialization failed: a drift bug.
    Drift { error: String, raw_json: String },
    /// The API returned a non-success (non-empty) status -- a real API error,
    /// not drift.
    ApiError(String),
    /// A transport/HTTP error -- not drift.
    Transport(String),
}

/// Probe one method by name, given its params and the deserialization target.
///
/// The raw JSON envelope is fetched over whichever transport the method
/// requires; the typed shape `T` is then deserialized from a clone of that
/// value. `count` extracts an optional element count from the deserialized
/// value for logging (return `None` for non-list responses).
pub async fn probe<P, T>(
    client: &Client,
    method: &str,
    params: &P,
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Sync,
    T: DeserializeOwned + Debug,
{
    probe_qualified(client, method, params, |_| {}, count).await
}

/// A record-listing request: the params as they go on the wire, and the offset
/// they ask for.
///
/// The two are one value because they have to agree. The public params carry an
/// IANA zone, the wire wants the numeric offset its typed `Client` method
/// resolves inside a private wire twin, and the response's wall clocks are only
/// instants once that same offset goes back on. Passing them separately lets a
/// caller ask for `+05:30` and stamp `+00:00`, which deserializes, passes, and
/// is wrong by five and a half hours -- the failure this crate's zoned
/// timestamps exist to rule out.
pub struct ZonedRequest {
    params: Value,
    offset: TimezoneOffset,
}

impl ZonedRequest {
    /// `params` with the numeric `timezone` the wire twin would have set, which
    /// a call-by-name cannot reach.
    ///
    /// Params that already name a zone are rejected rather than overwritten:
    /// silently replacing `Asia/Kolkata` with `offset` would send one zone,
    /// stamp another, and report a pass for a zone never asked about -- the
    /// divergence keeping the two halves in one value is here to prevent.
    pub fn new(params: &impl Serialize, offset: TimezoneOffset) -> Result<Self, String> {
        match serde_json::to_value(params) {
            Ok(Value::Object(fields)) if fields.contains_key("timezone") => Err(format!(
                "params already name a timezone ({}); pass the zone as the offset instead",
                fields["timezone"]
            )),
            Ok(Value::Object(mut fields)) => {
                fields.insert("timezone".into(), json!(offset));
                Ok(Self {
                    params: Value::Object(fields),
                    offset,
                })
            }

            Ok(other) => Err(format!("params are not an object: {other}")),
            Err(error) => Err(format!("params do not serialize: {error}")),
        }
    }

    /// The params as sent, for a failure capture that must not describe a
    /// request that was never made.
    pub fn params(&self) -> &Value {
        &self.params
    }
}

/// Probe a record-listing method.
///
/// `getCDR` and the `getSMS` / `getMMS` family shift their timestamps by the
/// numeric `timezone` the request carries and then report the shifted wall
/// clock without it, so the request's own offset goes back on before the typed
/// step -- left alone, every unqualified timestamp would read as drift.
/// `timestamps` are the paths [`voip_ms::attach_offset`] takes, which the crate
/// emits per method as `GET_CDR_TIMESTAMPS` and its siblings.
pub async fn probe_zoned<T>(
    client: &Client,
    method: &str,
    request: &ZonedRequest,
    timestamps: &[&str],
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    T: DeserializeOwned + Debug,
{
    let fixed = request.offset.to_fixed_offset();
    probe_qualified(
        client,
        method,
        &request.params,
        |body| attach_offset(body, fixed, timestamps),
        count,
    )
    .await
}

/// [`probe_zoned`] over default params at UTC: the probe-depth form, where a
/// record-listing method is called with no filters.
///
/// Params that will not serialize are a bug in this harness rather than drift
/// in the API, so they classify as `Transport` like the other param failure
/// [`probe`] can hit.
pub async fn probe_zoned_default<P, T>(
    client: &Client,
    method: &str,
    timestamps: &[&str],
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Default,
    T: DeserializeOwned + Debug,
{
    match ZonedRequest::new(&P::default(), TimezoneOffset::UTC) {
        Ok(request) => probe_zoned(client, method, &request, timestamps, count).await,
        Err(error) => ProbeOutcome::Transport(error),
    }
}

/// The shared probe body, reached by every probe here: fetch the raw envelope,
/// let `qualify` complete it, then deserialize `T` over the result. The raw
/// envelope is what a drift report shows, so `qualify`'s edits stay out of it.
///
/// The fetch goes through [`Client::call_raw_by_name`], which takes the
/// transport the method requires. A probe holds a wire name, not a generated
/// method, so it cannot inherit the transport from the call site -- and a file
/// method sent as a GET would fail on request-line length, reading as a
/// transport error rather than as this function's mistake.
async fn probe_qualified<P, T>(
    client: &Client,
    method: &str,
    params: &P,
    qualify: impl Fn(&mut Value),
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Sync,
    T: DeserializeOwned + Debug,
{
    let raw = match client.call_raw_by_name(method, params).await {
        Ok(value) => value,
        // An empty-collection status is the typed path's empty-list case, not a
        // failure: both raw forms surface it verbatim, where the typed `call`
        // folds it into an empty response. Mirror the typed semantics so an
        // empty account never reads as an API error -- there is simply nothing
        // to deserialize.
        Err(Error::Api(status)) if status.is_empty_collection() => {
            return ProbeOutcome::Ok {
                element_count: Some(0),
                unmodeled: Vec::new(),
                degraded: Vec::new(),
            };
        }
        Err(Error::Api(status)) => return ProbeOutcome::ApiError(status.to_string()),
        Err(Error::Http(e)) => return ProbeOutcome::Transport(e.to_string()),
        Err(Error::InvalidResponse(e)) => {
            // 2xx JSON with no `status` field -- treat as a transport-class
            // anomaly, not drift (drift is a shape mismatch on a valid envelope).
            return ProbeOutcome::Transport(format!("invalid response: {e}"));
        }
        // Params failed wire conversion before any request went out (e.g. a
        // timezone that couldn't resolve); a harness bug, not API drift.
        Err(e @ Error::InvalidParams(_)) => return ProbeOutcome::Transport(e.to_string()),
    };

    // Against the raw envelope, not the qualified one: `qualify` completes
    // values the server left bare, and a key it had to add would otherwise read
    // as one the API sent and the crate drops.
    //
    // A method with no typed shape at all has nothing to compare against, so an
    // absent *or empty* modeled set must not read as "every key is unmodeled".
    let unmodeled = match response_fields::modeled_paths(method) {
        Some(modeled) if !modeled.is_empty() => keydiff::unmodeled_paths(&raw, modeled),
        _ => Vec::new(),
    };

    let mut qualified = raw.clone();
    qualify(&mut qualified);
    match serde_json::from_value::<T>(qualified) {
        Ok(typed) => ProbeOutcome::Ok {
            degraded: degraded_values(&typed),
            element_count: count(&typed),
            unmodeled,
        },
        Err(error) => ProbeOutcome::Drift {
            error: error.to_string(),
            raw_json: pretty(&raw),
        },
    }
}

/// The catch-all variants a tolerant type parks a value it could not read in.
/// `Unknown {` is [`voip_ms::Routing`]'s, which is a struct variant; the rest
/// are tuple variants.
const CATCH_ALL_VARIANTS: &[&str] = &["Unreadable(", "Unrecognized(", "Unknown(", "Unknown {"];

/// The values a deserialized response could not read, as `Debug` renders them.
///
/// Tolerance is what stopped one odd value failing a whole envelope, and it
/// took this probe's only signal with it: a re-spelled date now deserializes
/// *successfully* into a catch-all variant, so the typed read no longer fails
/// and `keydiff` stays quiet because the key is modeled. Without this, the
/// harness would pass silently on the very input that opened issue #28.
///
/// It reads `Debug` because that is the only uniform view of a `*Response`:
/// the type carries no `Serialize` (a response is received, never built), and
/// a per-type accessor would mean touching all 222 of them.
fn degraded_values(typed: &impl Debug) -> Vec<String> {
    let rendered = format!("{typed:?}");
    let mut found: Vec<String> = Vec::new();
    for variant in CATCH_ALL_VARIANTS {
        let mut rest = rendered.as_str();
        while let Some(at) = rest.find(variant) {
            let tail = &rest[at..];
            let end = tail
                .find(['(', '{'].as_slice())
                .map_or(tail.len(), |i| i + 1);
            let payload_end = tail[end..]
                .find([')', '}'].as_slice())
                .map_or(tail.len(), |i| end + i + 1);
            found.push(tail[..payload_end].to_string());
            rest = &rest[at + variant.len()..];
        }
    }

    found.sort();
    found.dedup();
    found
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use voip_ms::{Reported, TransactionDate, chrono::NaiveDate};

    /// The signal this replaces: before tolerance, these values failed the
    /// typed read and the probe reported drift. They now deserialize, so the
    /// catch-all variant is the only thing left that says the wire changed.
    #[test]
    fn a_catch_all_variant_is_what_drift_looks_like_now() {
        let found = degraded_values(&vec![
            Some(Reported::Unreadable("08/10/2026".to_string())),
            Some(Reported::Parsed(
                NaiveDate::from_ymd_opt(2026, 10, 8).unwrap(),
            )),
        ]);
        assert_eq!(found, ["Unreadable(\"08/10/2026\")"]);
    }

    /// Every tolerant family reports, not just the date wrapper: the issue this
    /// harness missed was a `TransactionDate`, and a zone name or a routing tag
    /// degrades the same way.
    #[test]
    fn every_tolerant_family_reports() {
        let found = degraded_values(&(
            TransactionDate::Unrecognized("whenever".to_string()),
            voip_ms::Routing::Unknown {
                tag: "zzz".to_string(),
                value: "1".to_string(),
            },
        ));
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().any(|f| f.starts_with("Unrecognized(")));
        assert!(found.iter().any(|f| f.starts_with("Unknown {")));
    }

    /// A response the crate read end to end has nothing to report, so an
    /// ordinary run stays quiet.
    #[test]
    fn a_fully_read_response_reports_nothing() {
        let clean = vec![Some(Reported::Parsed(
            NaiveDate::from_ymd_opt(2026, 10, 8).unwrap(),
        ))];
        assert!(degraded_values(&clean).is_empty());
    }
}
