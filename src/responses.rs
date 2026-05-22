use serde::Deserialize;
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
    pub current_balance: Option<String>,
    pub spent_total: Option<String>,
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
    pub pop: Option<String>,
    pub routing: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
