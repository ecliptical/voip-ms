/// Result type returned by all [`Client`](crate::Client) methods.
pub type Result<T> = std::result::Result<T, Error>;

/// The [`ApiStatus`] enum and its impls are generated from the official API
/// docs' error-code table; see [`crate::generated`].
pub use crate::generated::ApiStatus;

/// Errors returned by the voip.ms client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport or HTTP-level failure.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The response did not contain the expected JSON envelope.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// The API responded with a non-`success` status, surfaced as a typed
    /// [`ApiStatus`] variant (or [`ApiStatus::Unknown`] for a code this crate
    /// doesn't recognize).
    #[error("API status: {0}")]
    Api(ApiStatus),
}
