/// Result type returned by all [`Client`](crate::Client) methods.
pub type Result<T> = std::result::Result<T, Error>;

/// The [`ApiStatus`] enum and its impls are generated from the official API
/// docs' error-code table; see [`crate::generated`].
pub use crate::generated::ApiStatus;

/// Errors returned by the VoIP.ms client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport or HTTP-level failure.
    ///
    /// The inner error carries no URL: the `From<reqwest::Error>` conversion
    /// strips it. `#[source]` stands in for the `#[from]` that conversion
    /// replaces, keeping [`Error::source`](std::error::Error::source) walkable
    /// into reqwest's own cause chain.
    #[error("HTTP error: {0}")]
    Http(#[source] reqwest::Error),

    /// The response did not contain the expected JSON envelope.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// The API responded with a non-`success` status, surfaced as a typed
    /// [`ApiStatus`] variant (or [`ApiStatus::Unknown`] for a code this crate
    /// doesn't recognize).
    #[error("API status: {0}")]
    Api(ApiStatus),

    /// The request parameters could not be converted to their wire form
    /// before sending -- e.g. a record-listing `timezone` whose UTC offset
    /// cannot be resolved or falls outside the range VoIP.ms accepts.
    #[error("invalid parameters: {0}")]
    InvalidParams(#[from] crate::types::TimezoneOffsetError),
}

/// Wraps the error with its request URL stripped.
///
/// The API authenticates by query parameter, so every request URL carries a
/// live `api_password`, and a `reqwest::Error` renders its URL verbatim from
/// both `Display` and `Debug` -- `format!("{e}")` on a wrapped error would
/// print the password. Stripping in the conversion rather than at the call
/// sites makes that hold for every `?` that produces an [`Error`], including
/// ones added later. The URL is the only thing dropped: the error's kind,
/// status, and source chain all survive.
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e.without_url())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::net::TcpListener;
    use std::time::Duration;

    use super::*;

    const SECRET: &str = "sup3r-s3cret-api-p4ssword";

    /// A `reqwest::Error` from a request whose URL carries [`SECRET`] as an
    /// `api_password` query parameter, mirroring how `Client` authenticates.
    ///
    /// The request targets a loopback port that was bound and released, so the
    /// connection is refused rather than routed anywhere.
    async fn refused_request_error() -> reqwest::Error {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("loopback addr");
        drop(listener);

        let url = format!("http://{addr}/api/v1/rest.php?api_username=u&api_password={SECRET}");
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("build test HTTP client");

        // `cargo test` has no per-test timeout, so a host that drops packets
        // rather than refusing would hang the binary with no failing assertion.
        tokio::time::timeout(Duration::from_secs(10), http.get(&url).send())
            .await
            .expect("request settled within the outer timeout")
            .expect_err("a released loopback port must refuse the connection")
    }

    /// Guards the premise of [`http_conversion_strips_the_url`]: if reqwest
    /// ever stops embedding the URL, the stripping becomes dead code and this
    /// fails rather than leaving a redaction test that passes vacuously.
    #[tokio::test]
    async fn raw_reqwest_error_leaks_the_password() {
        let raw = refused_request_error().await;
        assert!(
            raw.to_string().contains(SECRET),
            "expected the raw error to embed its URL, got: {raw}"
        );
        assert!(
            format!("{raw:?}").contains(SECRET),
            "expected the raw error's Debug to embed its URL, got: {raw:?}"
        );
    }

    #[tokio::test]
    async fn http_conversion_strips_the_url() {
        let err = Error::from(refused_request_error().await);

        assert!(
            !err.to_string().contains(SECRET),
            "Display leaked the password: {err}"
        );
        assert!(
            !format!("{err:?}").contains(SECRET),
            "Debug leaked the password: {err:?}"
        );
        assert!(
            !format!("{err:#}").contains(SECRET),
            "alternate Display leaked the password: {err:#}"
        );
    }

    /// The downstream contract: consumers classify these errors instead of
    /// formatting them, and every predicate reads the error's kind, not its URL.
    #[tokio::test]
    async fn conversion_preserves_classification_and_source() {
        let raw = refused_request_error().await;
        let (is_timeout, is_connect, is_body, is_decode, status) = (
            raw.is_timeout(),
            raw.is_connect(),
            raw.is_body(),
            raw.is_decode(),
            raw.status(),
        );
        assert!(is_connect, "expected a connect error, got: {raw}");

        let Error::Http(inner) = Error::from(raw) else {
            panic!("a reqwest::Error must convert to Error::Http");
        };

        assert_eq!(inner.is_timeout(), is_timeout);
        assert_eq!(inner.is_connect(), is_connect);
        assert_eq!(inner.is_body(), is_body);
        assert_eq!(inner.is_decode(), is_decode);
        assert_eq!(inner.status(), status);
        assert!(inner.url().is_none(), "the URL must be gone");

        // A consumer walks this chain to surface the underlying cause text
        // ("tcp connect error: ...") in a user-facing diagnostic, so the chain
        // must stay both non-empty and free of the secret at every frame.
        let err = Error::Http(inner);
        let mut frames = 0;
        let mut source = err.source();
        while let Some(frame) = source {
            assert!(
                !frame.to_string().contains(SECRET),
                "source frame {frames} leaked the password: {frame}"
            );
            frames += 1;
            source = frame.source();
        }

        assert!(frames > 0, "Error::source() must reach the reqwest error");
    }
}
