use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Generic status envelope with unknown payload fields preserved.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for getBalance responses.
///
/// Unknown top-level and nested fields are preserved in extra maps.
#[derive(Debug, Clone, Deserialize)]
pub struct GetBalanceResponse {
    pub status: String,
    pub balance: Balance,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for the balance object in getBalance responses.
#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub current_balance: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub spent_total: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub calls_total: Option<u64>,
    pub time_total: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub spent_today: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub calls_today: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub time_today: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for getDIDsInfo responses.
///
/// Unknown top-level and nested fields are preserved in extra maps.
#[derive(Debug, Clone, Deserialize)]
pub struct GetDidsInfoResponse {
    #[serde(default)]
    pub dids: Vec<DidInfo>,
    pub status: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for DID entries from getDIDsInfo.
#[derive(Debug, Clone, Deserialize)]
pub struct DidInfo {
    pub did: Option<String>,
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub pop: Option<u64>,
    pub routing: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub voicemail_threshold: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub dialtime: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub cnam: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub e911: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub record_calls: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub inbound_dialing_mode: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub billing_type: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_opt_date")]
    pub next_billing: Option<NaiveDate>,
    #[serde(default, deserialize_with = "deserialize_opt_datetime")]
    pub order_date: Option<NaiveDateTime>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub reseller_account: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_opt_date")]
    pub reseller_next_billing: Option<NaiveDate>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub reseller_monthly: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub reseller_minute: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_decimal_from_string_or_number"
    )]
    pub reseller_setup: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_available: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub transcribe: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub transcription_start_delay: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub transcription_sentiment: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub transcription_summary: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub transcription_redaction: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub mms_available: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_email_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_forward_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_url_callback_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub sms_url_callback_retry: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub webhook_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub dialmode: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub smpp_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_bool_from_string_number_or_yn"
    )]
    pub sms_sipaccount_enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn deserialize_opt_decimal_from_string_or_number<'de, D>(
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
        Some(Value::String(s)) => Decimal::from_str(&s)
            .map(Some)
            .map_err(|e| D::Error::custom(format!("invalid decimal string {s}: {e}"))),
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

fn deserialize_opt_u64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| D::Error::custom(format!("number cannot be represented as u64: {n}"))),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Ok(None);
            }
            s.parse::<u64>()
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid integer string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

fn deserialize_opt_bool_from_string_number_or_yn<'de, D>(
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

fn deserialize_opt_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
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

fn deserialize_opt_datetime<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
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
