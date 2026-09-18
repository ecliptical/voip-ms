//! The raw-vs-typed drift probe.
//!
//! Every drift bug in this crate's history is the same shape: the live API
//! returns JSON that the generated `*Response` type can't deserialize, turning
//! a *successful* upstream call into a serde error. The probe isolates exactly
//! that step -- fetch the raw envelope, then attempt the typed deserialization
//! over it -- so a failure is unambiguously attributable to response-shape
//! drift (raw ok, typed fails) rather than to the network or a real API error.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use voip_ms::{Client, Error, TimezoneOffset, attach_offset};

/// The result of probing one method.
pub enum ProbeOutcome {
    /// Raw succeeded and the typed shape deserialized. `element_count` is set
    /// when the response's primary payload is a list, for logging.
    Ok { element_count: Option<usize> },
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
/// `call_raw` yields the raw JSON envelope; the typed shape `T` is then
/// deserialized from a clone of that value. `count` extracts an optional
/// element count from the deserialized value for logging (return `None` for
/// non-list responses).
pub async fn probe<P, T>(
    client: &Client,
    method: &str,
    params: &P,
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Sync,
    T: DeserializeOwned,
{
    probe_qualified(client, method, params, |_| {}, count).await
}

/// Probe a record-listing method the way its typed `Client` method calls it.
///
/// `getCDR` and the `getSMS` / `getMMS` family shift their timestamps by the
/// numeric `timezone` the request carries and then report the shifted wall
/// clock without it. The typed method sends that offset through a private wire
/// twin and attaches it back before deserializing, and a call-by-name reaches
/// neither half: left alone the raw call would ask for the account's own zone
/// and the typed step would then reject every unqualified timestamp as drift.
/// Both halves are reproduced here at UTC, the offset every probe in this
/// harness wants.
///
/// `timestamps` are the paths [`voip_ms::attach_offset`] takes.
pub async fn probe_zoned<P, T>(
    client: &Client,
    method: &str,
    params: &P,
    timestamps: &[&str],
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Sync,
    T: DeserializeOwned,
{
    let offset = TimezoneOffset::UTC;
    let mut wire = match serde_json::to_value(params) {
        Ok(Value::Object(fields)) => fields,
        Ok(other) => {
            return ProbeOutcome::Transport(format!("params are not an object: {other}"));
        }

        Err(error) => {
            return ProbeOutcome::Transport(format!("params do not serialize: {error}"));
        }
    };

    wire.insert("timezone".into(), json!(offset));
    let fixed = offset.to_fixed_offset();
    probe_qualified(
        client,
        method,
        &Value::Object(wire),
        |body| attach_offset(body, fixed, timestamps),
        count,
    )
    .await
}

/// The shared probe body: fetch the raw envelope, let `qualify` complete it,
/// then deserialize `T` over the result. The raw envelope is what a drift
/// report shows, so `qualify`'s edits stay out of it.
async fn probe_qualified<P, T>(
    client: &Client,
    method: &str,
    params: &P,
    qualify: impl Fn(&mut Value),
    count: impl Fn(&T) -> Option<usize>,
) -> ProbeOutcome
where
    P: Serialize + Sync,
    T: DeserializeOwned,
{
    let raw = match client.call_raw(method, params).await {
        Ok(value) => value,
        // An empty-collection status is the typed path's empty-list case, not a
        // failure: `call_raw` surfaces it verbatim but the typed `call` folds it
        // into an empty response. Mirror the typed semantics so an empty account
        // never reads as an API error -- there is simply nothing to deserialize.
        Err(Error::Api(status)) if status.is_empty() => {
            return ProbeOutcome::Ok {
                element_count: Some(0),
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

    let mut qualified = raw.clone();
    qualify(&mut qualified);
    match serde_json::from_value::<T>(qualified) {
        Ok(typed) => ProbeOutcome::Ok {
            element_count: count(&typed),
        },
        Err(error) => ProbeOutcome::Drift {
            error: error.to_string(),
            raw_json: pretty(&raw),
        },
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
