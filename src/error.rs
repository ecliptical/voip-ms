use reqwest::StatusCode;

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
/// A GET authenticates by query parameter, so its request URL carries a live
/// `api_password`, and a `reqwest::Error` renders its URL verbatim from both
/// `Display` and `Debug` -- `format!("{e}")` on a wrapped error would print the
/// password. Stripping in the conversion rather than at the call sites makes
/// that hold for every `?` that produces an [`Error`], including ones added
/// later. The URL is the only thing dropped: the error's kind, status, and
/// source chain all survive. A multipart POST keeps `api_password` out of the
/// URL by carrying it in the body, but its URL is still the caller-supplied
/// base URL, which may embed `user:pass@` userinfo that `Url` renders verbatim
/// -- so stripping is load-bearing on every transport, not only on GET.
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e.without_url())
    }
}

impl Error {
    /// The transport-level classification of this failure, or `None` when the
    /// failure is not a transport one.
    ///
    /// Only [`Error::Http`] classifies. [`Error::Api`] and
    /// [`Error::InvalidResponse`] both mean the exchange completed and VoIP.ms
    /// answered -- an allow-list rejection arrives as
    /// `Api(`[`ApiStatus::IPNotEnabled`]`)` on a 200, not as an HTTP 403 --
    /// and [`Error::InvalidParams`] means nothing was ever sent.
    pub fn transport(&self) -> Option<TransportFailure> {
        match self {
            Self::Http(e) => Some(TransportFailure::classify(e)),
            Self::InvalidResponse(_) | Self::Api(_) | Self::InvalidParams(_) => None,
        }
    }
}

/// How a request to the VoIP.ms API failed beneath the API itself: the HTTP
/// exchange, or the connection that would have carried it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportFailure {
    /// VoIP.ms, or an edge in front of it, answered with an error status.
    Rejected(StatusCode),

    /// The exchange did not finish within the deadline.
    Timeout,

    /// The host name did not resolve.
    Dns,

    /// No connection to the host could be established.
    Connect,

    /// The request landed and its reply could not be read (body or decode).
    Body,

    /// A failure none of the other variants describe.
    Other,
}

impl TransportFailure {
    /// Reduces a `reqwest::Error` to the one variant that describes it.
    ///
    /// The order the readings are taken in is load-bearing. A status is a
    /// distinct kind of failure rather than a refinement of the others, so it
    /// is read first. A resolution failure answers both `is_dns` and
    /// `is_connect` -- reqwest reports it through the connect error that wraps
    /// it -- so the narrower reading has to come first to survive.
    fn classify(e: &reqwest::Error) -> Self {
        if let Some(status) = e.status() {
            Self::Rejected(status)
        } else if e.is_timeout() {
            Self::Timeout
        } else if e.is_dns() {
            Self::Dns
        } else if e.is_connect() {
            Self::Connect
        } else if e.is_body() || e.is_decode() {
            Self::Body
        } else {
            Self::Other
        }
    }

    /// Whether the request provably never reached VoIP.ms, so no account state
    /// can have changed.
    ///
    /// A 4xx other than 408 counts: the transport refused the request before
    /// VoIP.ms could act on it. `false` is the conservative answer rather than a
    /// claim that something did happen -- a 5xx, a 408, an expired deadline, or
    /// an unreadable reply each leave open that VoIP.ms acted and the response
    /// was lost.
    pub fn never_reached_upstream(self) -> bool {
        match self {
            Self::Connect | Self::Dns => true,
            // 408 is the one 4xx whose meaning depends on who sent it. RFC 9110
            // §15.5.9 defines it as the origin giving up on an incomplete
            // request, which would prove nothing was acted on, but
            // intermediaries widely return it for a slow *response* -- a 504 in
            // 408's clothing, where VoIP.ms may well have acted.
            Self::Rejected(status) => {
                status.is_client_error() && status != StatusCode::REQUEST_TIMEOUT
            }

            Self::Timeout | Self::Body | Self::Other => false,
        }
    }

    /// Whether repeating the identical call is worth anything.
    pub fn retry_outlook(self) -> RetryOutlook {
        match self {
            Self::Connect | Self::Dns => RetryOutlook::Worthwhile,
            Self::Rejected(status)
                if status == StatusCode::TOO_MANY_REQUESTS
                    || status == StatusCode::REQUEST_TIMEOUT =>
            {
                RetryOutlook::AfterWaiting
            }

            Self::Rejected(status) if status.is_client_error() => RetryOutlook::Futile,
            Self::Rejected(_) | Self::Timeout | Self::Body | Self::Other => RetryOutlook::Unknown,
        }
    }
}

/// What repeating an identical failed call can be expected to achieve.
///
/// A separate question from [`TransportFailure::never_reached_upstream`], and a
/// refusal answers the two differently: a stale proxy credential answering 401
/// changed no state and is still futile to repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetryOutlook {
    /// Transient: the same call may well work.
    Worthwhile,

    /// Refused on terms repeating will not change; something must be fixed.
    Futile,

    /// Refused for arriving too often or too slowly; may work after a pause.
    AfterWaiting,

    /// May have landed, so what to do turns on the caller's operation.
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::future::Future;
    use std::net::TcpListener;
    use std::time::Duration;

    use rust_decimal::Decimal;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::types::TimezoneOffset;

    const SECRET: &str = "sup3r-s3cret-api-p4ssword";

    /// The endpoint `Client` calls under `base`, with [`SECRET`] in the query
    /// string as the `api_password`, mirroring how `Client` authenticates.
    fn secret_url(base: &str) -> String {
        format!("{base}/api/v1/rest.php?api_username=u&api_password={SECRET}")
    }

    /// An HTTP client whose own `deadline` bounds every request made with it.
    fn bounded_client(deadline: Duration) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(deadline)
            .build()
            .expect("build test HTTP client")
    }

    /// Awaits `request` under an outer bound. `cargo test` has no per-test
    /// timeout, so a peer that neither answers nor refuses would hang the whole
    /// binary with no failing assertion.
    async fn settled<T>(request: impl Future<Output = T>) -> T {
        tokio::time::timeout(Duration::from_secs(10), request)
            .await
            .expect("request settled within the outer timeout")
    }

    /// A `reqwest::Error` from a request that could not connect.
    ///
    /// The request targets a loopback port that was bound and released, so the
    /// connection is refused rather than routed anywhere.
    async fn refused_request_error() -> reqwest::Error {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("loopback addr");
        drop(listener);

        settled(
            bounded_client(Duration::from_secs(5))
                .get(secret_url(&format!("http://{addr}")))
                .send(),
        )
        .await
        .expect_err("a released loopback port must refuse the connection")
    }

    /// A `reqwest::Error` from a request a server answered with a 503.
    async fn rejected_request_error() -> reqwest::Error {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        settled(
            bounded_client(Duration::from_secs(5))
                .get(secret_url(&server.uri()))
                .send(),
        )
        .await
        .expect("the mock server answers")
        .error_for_status()
        .expect_err("a 503 must surface as an error status")
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
        let (is_timeout, is_dns, is_connect, is_body, is_decode, status) = (
            raw.is_timeout(),
            raw.is_dns(),
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
        assert_eq!(inner.is_dns(), is_dns);
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

    /// Fails every name, standing in for an unresolvable host without reaching
    /// a real resolver, whose answer for a reserved name is not guaranteed.
    struct UnresolvableHost;

    impl reqwest::dns::Resolve for UnresolvableHost {
        fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
            Box::pin(async { Err("no such host".into()) })
        }
    }

    #[tokio::test]
    async fn error_status_classifies_as_rejected() {
        let err = Error::from(rejected_request_error().await);

        assert_eq!(
            err.transport(),
            Some(TransportFailure::Rejected(StatusCode::SERVICE_UNAVAILABLE))
        );
    }

    #[tokio::test]
    async fn expired_deadline_classifies_as_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
            .mount(&server)
            .await;

        let raw = settled(
            bounded_client(Duration::from_millis(250))
                .get(secret_url(&server.uri()))
                .send(),
        )
        .await
        .expect_err("the client deadline must expire before the delayed reply");

        assert_eq!(
            Error::from(raw).transport(),
            Some(TransportFailure::Timeout)
        );
    }

    #[tokio::test]
    async fn unresolved_host_classifies_as_dns_not_connect() {
        let http = reqwest::Client::builder()
            .dns_resolver(UnresolvableHost)
            .timeout(Duration::from_secs(5))
            .build()
            .expect("build test HTTP client");

        let raw = settled(http.get(secret_url("http://voip.ms.invalid")).send())
            .await
            .expect_err("an unresolvable host must fail the request");

        // The premise for ordering Dns ahead of Connect: reqwest reports a
        // resolution failure through the connect error that wraps it, so both
        // predicates answer true and only the order decides which is read.
        assert!(raw.is_dns(), "expected a DNS error, got: {raw}");
        assert!(raw.is_connect(), "expected the wrapping connect error");

        assert_eq!(Error::from(raw).transport(), Some(TransportFailure::Dns));
    }

    #[tokio::test]
    async fn refused_connection_classifies_as_connect() {
        let raw = refused_request_error().await;
        assert!(!raw.is_dns(), "the address was numeric, got: {raw}");

        assert_eq!(
            Error::from(raw).transport(),
            Some(TransportFailure::Connect)
        );
    }

    #[tokio::test]
    async fn unreadable_reply_classifies_as_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("<html>not the envelope</html>"),
            )
            .mount(&server)
            .await;

        let raw = settled(async {
            bounded_client(Duration::from_secs(5))
                .get(secret_url(&server.uri()))
                .send()
                .await
                .expect("the mock server answers")
                .json::<serde_json::Value>()
                .await
        })
        .await
        .expect_err("HTML must not decode as the JSON envelope");

        assert_eq!(Error::from(raw).transport(), Some(TransportFailure::Body));
    }

    #[tokio::test]
    async fn unrecognized_failure_classifies_as_other() {
        let server = MockServer::start().await;
        let target = secret_url(&server.uri());
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", target.as_str()))
            .mount(&server)
            .await;

        let raw = settled(bounded_client(Duration::from_secs(5)).get(&target).send())
            .await
            .expect_err("a redirect onto itself must exhaust the redirect policy");

        assert_eq!(Error::from(raw).transport(), Some(TransportFailure::Other));
    }

    /// The recurring downstream mistake is reading an allow-list rejection as
    /// an HTTP 403. VoIP.ms answers 200 and reports it in the envelope, so it
    /// is an API status and classifies as no transport failure at all.
    #[test]
    fn non_transport_failures_do_not_classify() {
        assert_eq!(Error::Api(ApiStatus::IPNotEnabled).transport(), None);
        assert_eq!(
            Error::InvalidResponse("missing status field".into()).transport(),
            None
        );

        let out_of_range = TimezoneOffset::new(Decimal::from(14))
            .expect_err("+14 is outside the range VoIP.ms accepts");
        assert_eq!(Error::InvalidParams(out_of_range).transport(), None);
    }

    #[test]
    fn never_reached_upstream_per_variant() {
        use TransportFailure as T;

        assert!(T::Connect.never_reached_upstream());
        assert!(T::Dns.never_reached_upstream());

        // A 4xx is the transport refusing the request; a 5xx leaves open that
        // VoIP.ms acted and the response was lost.
        assert!(T::Rejected(StatusCode::UNAUTHORIZED).never_reached_upstream());
        assert!(T::Rejected(StatusCode::NOT_FOUND).never_reached_upstream());
        assert!(!T::Rejected(StatusCode::BAD_GATEWAY).never_reached_upstream());
        assert!(!T::Rejected(StatusCode::SERVICE_UNAVAILABLE).never_reached_upstream());

        // 408 is carved out of that: an intermediary returning it for a slow
        // response is indistinguishable here from the RFC-correct incomplete
        // request, so the claim is not provable.
        assert!(!T::Rejected(StatusCode::REQUEST_TIMEOUT).never_reached_upstream());

        assert!(!T::Timeout.never_reached_upstream());
        assert!(!T::Body.never_reached_upstream());
        assert!(!T::Other.never_reached_upstream());
    }

    #[test]
    fn retry_outlook_per_variant() {
        use RetryOutlook as R;
        use TransportFailure as T;

        assert_eq!(
            T::Rejected(StatusCode::TOO_MANY_REQUESTS).retry_outlook(),
            R::AfterWaiting
        );
        // 408 answers this question and `never_reached_upstream` differently:
        // whether it landed is unknown, but "not now" still reads right.
        assert_eq!(
            T::Rejected(StatusCode::REQUEST_TIMEOUT).retry_outlook(),
            R::AfterWaiting
        );

        // Every other 4xx refuses on terms an identical repeat cannot change.
        assert_eq!(
            T::Rejected(StatusCode::UNAUTHORIZED).retry_outlook(),
            R::Futile
        );
        assert_eq!(
            T::Rejected(StatusCode::FORBIDDEN).retry_outlook(),
            R::Futile
        );
        assert_eq!(
            T::Rejected(StatusCode::NOT_FOUND).retry_outlook(),
            R::Futile
        );

        assert_eq!(T::Connect.retry_outlook(), R::Worthwhile);
        assert_eq!(T::Dns.retry_outlook(), R::Worthwhile);

        assert_eq!(
            T::Rejected(StatusCode::BAD_GATEWAY).retry_outlook(),
            R::Unknown
        );
        assert_eq!(T::Timeout.retry_outlook(), R::Unknown);
        assert_eq!(T::Body.retry_outlook(), R::Unknown);
        assert_eq!(T::Other.retry_outlook(), R::Unknown);
    }

    /// Classification reads the error's kind, never its URL, so neither the
    /// verdicts nor anything rendered alongside them can carry the password.
    #[tokio::test]
    async fn classifying_an_error_does_not_reveal_the_password() {
        for err in [
            Error::from(refused_request_error().await),
            Error::from(rejected_request_error().await),
        ] {
            let failure = err.transport().expect("a transport failure");
            let verdicts = (failure.never_reached_upstream(), failure.retry_outlook());

            assert!(
                !format!("{failure:?}").contains(SECRET),
                "the classification leaked the password: {failure:?}"
            );
            assert!(
                !format!("{verdicts:?}").contains(SECRET),
                "a verdict leaked the password: {verdicts:?}"
            );
            assert!(
                !err.to_string().contains(SECRET) && !format!("{err:?}").contains(SECRET),
                "the classified error leaked the password: {err:?}"
            );
        }
    }
}
