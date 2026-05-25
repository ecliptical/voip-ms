//! Custom serde deserializers used by generated `*Response` structs.
//!
//! The voip.ms API frequently returns numbers, booleans, dates, and
//! decimals as JSON strings (and occasionally as JSON numbers for the
//! same field across different methods). These helpers normalize both
//! forms — and treat empty / `"0000-00-00"` / `"0000-00-00 00:00:00"`
//! placeholders as `None` — into Rust types.
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
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::str::FromStr;

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
