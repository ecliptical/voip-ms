use std::fmt;

/// Result type returned by all [`Client`](crate::Client) methods.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the voip.ms client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport or HTTP-level failure.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The response did not contain the expected JSON envelope.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// The API responded with a non-`success` status. The contained string is
    /// the verbatim `status` value, e.g. `invalid_credentials`,
    /// `missing_method`, `api_not_enabled`.
    #[error("API status: {0}")]
    Api(ApiStatus),
}

/// A non-success status returned by the voip.ms API.
///
/// voip.ms surfaces all method-specific error conditions through the `status`
/// field of the JSON response. The set of values varies per method and is not
/// stable across versions, so we keep this as a thin wrapper over the wire
/// string. See the official voip.ms API documentation for per-method status
/// values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiStatus(pub String);

impl ApiStatus {
    /// The verbatim `status` string from the response.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ApiStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ApiStatus {
    fn from(s: String) -> Self {
        ApiStatus(s)
    }
}
