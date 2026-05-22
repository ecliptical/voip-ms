use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as DeError;
use serde_json::Value;
use std::collections::BTreeMap;

/// Generic status envelope with unknown payload fields preserved.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for `getBalance` responses.
///
/// Unknown top-level and nested fields are preserved in `extra` maps.
#[derive(Debug, Clone, Deserialize)]
pub struct GetBalanceResponse {
    pub status: String,
    pub balance: Balance,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for the `balance` object in `getBalance` responses.
#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    #[serde(
        default,
        deserialize_with = "deserialize_opt_f64_from_string_or_number"
    )]
    pub current_balance: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_f64_from_string_or_number"
    )]
    pub spent_total: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub calls_total: Option<u64>,
    pub time_total: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_f64_from_string_or_number"
    )]
    pub spent_today: Option<f64>,
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

/// Partial typed shape for `getDIDsInfo` responses.
///
/// Unknown top-level and nested fields are preserved in `extra` maps.
#[derive(Debug, Clone, Deserialize)]
pub struct GetDidsInfoResponse {
    #[serde(default)]
    pub dids: Vec<DidInfo>,
    pub status: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Partial typed shape for DID entries from `getDIDsInfo`.
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
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub cnam: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub e911: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub sms_available: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64_from_string_or_number"
    )]
    pub mms_available: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn deserialize_opt_f64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_f64()
            .map(Some)
            .ok_or_else(|| D::Error::custom(format!("number cannot be represented as f64: {n}"))),
        Some(Value::String(s)) => s
            .parse::<f64>()
            .map(Some)
            .map_err(|e| D::Error::custom(format!("invalid numeric string `{s}`: {e}"))),
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
        Some(Value::String(s)) => s
            .parse::<u64>()
            .map(Some)
            .map_err(|e| D::Error::custom(format!("invalid integer string `{s}`: {e}"))),
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}
