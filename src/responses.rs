//! Custom serde (de)serializers used by generated `*Params` and
//! `*Response` structs.
//!
//! The VoIP.ms API frequently returns numbers, booleans, dates, and
//! decimals as JSON strings (and occasionally as JSON numbers for the
//! same field across different methods). These helpers normalize both
//! forms — and treat empty / `"0000-00-00"` / `"0000-00-00 00:00:00"`
//! placeholders as `None` — into Rust types.
//!
//! A few `bool` params also need a serializer: VoIP.ms rejects the
//! `true`/`false` a bare `bool` would emit, expecting `1`/`0` or
//! `yes`/`no`. The `serialize_*_flag_*` helpers supply that wire form.
//!
//! Some endpoints also emit `-1` as a sentinel for "not configured" in
//! fields that are otherwise unsigned identifiers. For optional unsigned
//! fields, `-1` and `"-1"` are normalized to `None`.
//!
//! These are wired up by `xtask` into `src/generated.rs`. Hand-written
//! call sites can also reference them via the `crate::responses::*`
//! module path.

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Value;
use std::str::FromStr;

use crate::types::{Routing, Seconds, WaitTime};

/// Deserialize a wire value (string, number, or bool) into its string form.
///
/// VoIP.ms returns enum-typed fields inconsistently as a JSON string (`"1"`,
/// `"yes"`) or a bare number / bool (`1`, `true`); generated enum
/// `Deserialize` impls route through this so `from_wire` always gets a string.
pub(crate) fn deserialize_enum_wire_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        other => Err(D::Error::custom(format!(
            "expected string, number, or bool, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_string_from_string_number_or_bool<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        Some(Value::Number(n)) => Ok(Some(n.to_string())),
        Some(Value::Bool(b)) => Ok(Some(b.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "expected string, number, or bool, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Decimal::from_str(&n.to_string())
            .map(Some)
            .map_err(|e| D::Error::custom(format!("invalid decimal {n}: {e}"))),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Ok(None);
            }
            Decimal::from_str(&s)
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid decimal string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_u64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => {
            if n.as_i64() == Some(-1) {
                return Ok(None);
            }
            n.as_u64().map(Some).ok_or_else(|| {
                D::Error::custom(format!("number cannot be represented as u64: {n}"))
            })
        }
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "-1" {
                return Ok(None);
            }
            trimmed
                .parse::<u64>()
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid integer string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_bool_from_string_number_or_yn<'de, D>(
    deserializer: D,
) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(b)),
        Some(Value::Number(n)) => {
            n.as_u64().map(|v| v != 0).map(Some).ok_or_else(|| {
                D::Error::custom(format!("number cannot be represented as u64: {n}"))
            })
        }
        Some(Value::String(s)) => {
            let normalized = s.trim().to_ascii_uppercase();
            if normalized.is_empty() {
                return Ok(None);
            }
            match normalized.as_str() {
                "1" | "Y" | "YES" | "TRUE" | "T" => Ok(Some(true)),
                "0" | "N" | "NO" | "FALSE" | "F" => Ok(Some(false)),
                _ => Err(D::Error::custom(format!("invalid boolean-like string {s}"))),
            }
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected bool, string, or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "0000-00-00" {
                return Ok(None);
            }
            NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid date {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected date string, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "0000-00-00 00:00:00" {
                return Ok(None);
            }
            NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid datetime {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected datetime string, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_routing<'de, D>(deserializer: D) -> Result<Option<Routing>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Routing::from_str(trimmed)
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid routing string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected routing string, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_seconds<'de, D>(deserializer: D) -> Result<Option<Seconds>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_via::<Seconds, D>(deserializer)
}

pub(crate) fn deserialize_opt_wait_time<'de, D>(
    deserializer: D,
) -> Result<Option<WaitTime>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_via::<WaitTime, D>(deserializer)
}

/// Deserialize an optional value via the target type's own `Deserialize`,
/// mapping JSON null and the empty string to `None`. Shared by the
/// seconds-or-sentinel helpers, whose types accept a number, a numeric string,
/// or a sentinel word but should still read an absent/empty field as `None`.
fn deserialize_opt_via<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(v) => T::deserialize(v).map(Some).map_err(D::Error::custom),
    }
}

/// Deserialize an optional list field that VoIP.ms may return either as a JSON
/// array or, when the method yields a single row, as a bare unwrapped object.
/// VoIP.ms's `print_r`-derived output collapses a one-element list to the
/// element itself (e.g. `getVoicemailMessageFile`'s `message` comes back as an
/// object, not a one-element array), so every generated list field accepts both
/// wire forms: an array becomes the `Vec` as-is, a lone value becomes a
/// one-element `Vec`, and null / absent / empty-string is `None`.
pub(crate) fn deserialize_opt_vec_from_single_or_seq<'de, T, D>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|v| T::deserialize(v).map_err(D::Error::custom))
            .collect::<Result<Vec<T>, _>>()
            .map(Some),
        Some(one) => T::deserialize(one)
            .map(|v| Some(vec![v]))
            .map_err(D::Error::custom),
    }
}

/// `skip_serializing_if` predicate for true-only flag params (`test`): a
/// `false` value is equivalent to absent and is left off the wire.
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

/// Serialize an optional `1`/`0` flag param: `Some(true)` → `"1"`,
/// `Some(false)` → `"0"`. `None` would be skipped before reaching here, so it
/// serializes nothing.
pub(crate) fn serialize_opt_flag_01<S>(v: &Option<bool>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(b) => serialize_flag_01(b, s),
        None => s.serialize_none(),
    }
}

/// Serialize an optional `yes`/`no` flag param: `Some(true)` → `"yes"`,
/// `Some(false)` → `"no"`. `None` would be skipped before reaching here.
pub(crate) fn serialize_opt_flag_yes_no<S>(v: &Option<bool>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(true) => s.serialize_str("yes"),
        Some(false) => s.serialize_str("no"),
        None => s.serialize_none(),
    }
}

/// Serialize a `1`/`0` flag param: `true` → `"1"`, `false` → `"0"`.
pub(crate) fn serialize_flag_01<S>(v: &bool, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(if *v { "1" } else { "0" })
}
