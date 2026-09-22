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
const CATCH_ALL_VARIANTS: &[&str] = &["Unreadable", "Unrecognized", "Unknown"];

/// Degraded values this harness expects and does not report.
///
/// Not every catch-all is drift. VoIP.ms's `getTimezones` catalog still lists
/// names the IANA database dropped, and a long-lived mailbox can carry one, so
/// those land in `Unrecognized` on every run by design. Without an allowlist,
/// an account holding one would exit non-zero forever and the operator's only
/// move would be to ignore the exit code -- which is the signal this check
/// exists to give. Same idea as `check-types`'s `DELIBERATE`: a *new* degraded
/// value is the finding.
///
/// Matched against the rendered payload, so an entry is the value VoIP.ms
/// sends, not the variant holding it.
const EXPECTED_DEGRADED: &[&str] = &[
    "Asia/Beijing",
    "Canada/East-Saskatchewan",
    "Factory",
    "Riyadh87",
    "Riyadh88",
    "Riyadh89",
    "US/Pacific-New",
];

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
/// a per-type accessor would mean touching all 222 of them. Reading `Debug`
/// costs two things this guards against: a variant name appearing inside a
/// string field would false-positive, so a match must start at a token
/// boundary; and a payload can contain the delimiter that closes it, so the
/// end is found by depth rather than by the first one.
fn degraded_values(typed: &impl Debug) -> Vec<String> {
    let rendered = format!("{typed:?}");
    let bytes = rendered.as_bytes();
    let mut found: Vec<String> = Vec::new();

    for (at, _) in Scan::new(bytes).filter(|(_, b)| *b == b'(' || *b == b'{') {
        // `Debug` renders a tuple variant as `Name(..)` and a struct variant as
        // `Name { .. }`, so the name ends either at the delimiter or one space
        // before it.
        let head = rendered[..at].strip_suffix(' ').unwrap_or(&rendered[..at]);
        let Some(variant) = CATCH_ALL_VARIANTS.iter().find(|v| head.ends_with(**v)) else {
            continue;
        };

        // A variant name is preceded by a delimiter, a space or `::` -- never by
        // a letter -- which keeps a nested type name from matching its suffix.
        let start = head.len() - variant.len();
        if rendered[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }

        let Some(end) = matching_delimiter(bytes, at) else {
            continue;
        };

        found.push(rendered[start..=end].to_string());
    }

    found.retain(|value| !EXPECTED_DEGRADED.iter().any(|known| value.contains(known)));
    found.sort();
    found.dedup();
    found
}

/// The bytes of a `Debug` rendering that are structure rather than content:
/// everything outside a string literal.
///
/// A variant name only means a variant where `Debug` wrote it. The same text
/// inside a field value is something VoIP.ms sent -- a `description` quoting an
/// error, a CNAM -- and reporting it would fail a live run over a string.
struct Scan<'a> {
    bytes: &'a [u8],
    at: usize,
    in_string: bool,
    escaped: bool,
}

impl<'a> Scan<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: 0,
            in_string: false,
            escaped: false,
        }
    }
}

impl Iterator for Scan<'_> {
    type Item = (usize, u8);

    fn next(&mut self) -> Option<(usize, u8)> {
        while self.at < self.bytes.len() {
            let at = self.at;
            let b = self.bytes[at];
            self.at += 1;

            if self.escaped {
                self.escaped = false;
                continue;
            }

            match b {
                b'\\' if self.in_string => self.escaped = true,
                b'"' => self.in_string = !self.in_string,
                _ if self.in_string => {}
                _ => return Some((at, b)),
            }
        }

        None
    }
}

/// The index of the delimiter closing the one at `open`, counting nesting so a
/// payload holding its own closer is not cut short.
fn matching_delimiter(bytes: &[u8], open: usize) -> Option<usize> {
    let close = if bytes[open] == b'(' { b')' } else { b'}' };
    let mut depth = 0usize;
    for (at, b) in Scan::new(bytes) {
        if at < open {
            continue;
        }

        if b == bytes[open] {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }

    None
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

    /// The payload is reported whole. A value carrying the delimiter that
    /// closes it is exactly the kind of surprise worth reporting accurately,
    /// and a truncated one is pasted into an override as something VoIP.ms
    /// never sent.
    #[test]
    fn a_payload_holding_its_own_delimiter_is_not_truncated() {
        let found = degraded_values(&Reported::<NaiveDate>::Unreadable(
            "2026-08-01 (approx)".to_string(),
        ));
        assert_eq!(found, ["Unreadable(\"2026-08-01 (approx)\")"]);
    }

    /// A variant name inside a string field is field content, not a variant.
    /// Reporting it would fail a run over a `description` that happens to
    /// mention one.
    #[test]
    fn a_variant_name_inside_a_field_value_is_not_a_finding() {
        // A tuple rather than a struct: the point is a variant name sitting in
        // a *string* inside a composite, which is how a `description` or a CNAM
        // echoing an error would reach the scan.
        let row = (
            "description",
            "carrier reported Unknown(code 3)".to_string(),
        );
        let found = degraded_values(&row);
        assert!(found.is_empty(), "{found:?}");
    }

    /// A legacy zone name is permanent and by design, so it must not make every
    /// live run exit non-zero -- the operator would have to ignore the exit code
    /// to keep working, which is the signal this check exists to give.
    #[test]
    fn an_expected_degraded_value_is_not_reported() {
        let found = degraded_values(&voip_ms::TimezoneName::Unrecognized(
            "US/Pacific-New".to_string(),
        ));
        assert!(found.is_empty(), "{found:?}");

        // A zone name that is not on the list still reports: the allowlist is
        // the known cases, not the whole family.
        let novel = degraded_values(&voip_ms::TimezoneName::Unrecognized(
            "Mars/Olympus".to_string(),
        ));
        assert_eq!(novel.len(), 1, "{novel:?}");
    }
}
