use reqwest::{IntoUrl, Url, multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::fmt;

use crate::error::{ApiStatus, Error, Result};

/// Default base URL for the VoIP.ms REST API.
pub const DEFAULT_BASE_URL: &str = "https://voip.ms/api/v1/rest.php";

/// A URL built only to be discarded: the multipart form reads the query string
/// `reqwest` serializes the parameters into, and never sends the request.
const SCRATCH_URL: &str = "http://form.invalid/";

/// Where a request carries its parameters.
#[derive(Clone, Copy)]
enum Transport {
    /// A GET with the parameters on the query string.
    Get,
    /// A POST with the parameters as `multipart/form-data` fields. Required
    /// for a base64 file payload, which overruns the 8190-byte request line
    /// the API's front end accepts. `application/x-www-form-urlencoded` is not
    /// an alternative: `rest.php` hands one to a SOAP handler, which answers
    /// with an XML fault.
    MultipartPost,
}

/// Async client for the VoIP.ms REST API.
///
/// Clients are cheap to clone; the underlying [`reqwest::Client`] uses an
/// internal connection pool that is shared across clones.
///
/// A call is a GET carrying its parameters on the query string, except one
/// whose parameters include a base64-encoded file, which is a
/// `multipart/form-data` POST.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    api_username: String,
    api_password: String,
}

impl Client {
    /// Build a new client with the default base URL and a default
    /// [`reqwest::Client`]. Use [`Client::builder`] for more control.
    ///
    /// # Panics
    ///
    /// Panics if the default base URL fails to parse, which would indicate a
    /// bug in this crate.
    pub fn new(api_username: impl Into<String>, api_password: impl Into<String>) -> Self {
        Self::builder(api_username, api_password)
            .build()
            .expect("default VoIP.ms base URL must parse")
    }

    /// Start building a client with custom HTTP client or base URL.
    pub fn builder(
        api_username: impl Into<String>,
        api_password: impl Into<String>,
    ) -> ClientBuilder {
        ClientBuilder {
            http: None,
            base_url: None,
            api_username: api_username.into(),
            api_password: api_password.into(),
        }
    }

    /// The GET form of a request to `url`: credentials, `method`, and `params`
    /// on the query string.
    fn get_request<U, P>(&self, url: U, method: &str, params: &P) -> reqwest::RequestBuilder
    where
        U: IntoUrl,
        P: Serialize + ?Sized,
    {
        self.http
            .get(url)
            .query(&[
                ("api_username", self.api_username.as_str()),
                ("api_password", self.api_password.as_str()),
                ("method", method),
            ])
            .query(params)
    }

    /// The request's parameters as multipart fields, read back out of the query
    /// string the GET form serializes them into. The two transports differ in
    /// where a value rides, not in how it is encoded, and a parameter no
    /// `Serialize` can put on a query string fails here as it would there.
    fn multipart_form<P>(&self, method: &str, params: &P) -> Result<multipart::Form>
    where
        P: Serialize + ?Sized,
    {
        let scratch = self.get_request(SCRATCH_URL, method, params).build()?;
        let mut form = multipart::Form::new();
        for (name, value) in scratch.url().query_pairs() {
            form = form.text(name.into_owned(), value.into_owned());
        }

        Ok(form)
    }

    /// Issue the request for `method` over `transport` and return its parsed
    /// JSON body, without classifying the `status` field.
    async fn send<P>(&self, method: &str, params: &P, transport: Transport) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let request = match transport {
            Transport::Get => self.get_request(self.base_url.clone(), method, params),
            Transport::MultipartPost => self
                .http
                .post(self.base_url.clone())
                .multipart(self.multipart_form(method, params)?),
        };

        let text = request.send().await?.error_for_status()?.text().await?;
        // Some methods (e.g. delConference) answer a successful call with an
        // empty body instead of a `{"status":"success"}` envelope; treat that
        // as success rather than a JSON parse error.
        if text.trim().is_empty() {
            return Ok(json!({ "status": "success" }));
        }

        serde_json::from_str(&text)
            .map_err(|e| Error::InvalidResponse(format!("response body is not JSON: {e}")))
    }

    /// Issue the request for `method` over `transport` and return the parsed
    /// JSON body together with its classified status, without rejecting
    /// empty-collection statuses. Callers differ only in how they treat that
    /// case.
    async fn fetch<P>(
        &self,
        method: &str,
        params: &P,
        transport: Transport,
    ) -> Result<(Value, Option<ApiStatus>)>
    where
        P: Serialize + ?Sized,
    {
        let body = self.send(method, params, transport).await?;
        let empty = check_status(&body)?;
        Ok((body, empty))
    }

    /// Issue a request for `method` with the given typed parameters and
    /// return the full JSON response body as a [`serde_json::Value`].
    ///
    /// The `status` field is inspected: any value other than `success`
    /// causes an [`Error::Api`] -- including the empty-collection statuses
    /// ([`ApiStatus::is_empty`], e.g. `no_sms`). This is the verbatim escape
    /// hatch: it surfaces exactly what VoIP.ms returned. The typed
    /// [`Client::call`] instead folds those into an empty response.
    ///
    /// This is the low-level raw call used by every generated `*_raw`
    /// method on [`Client`]. Reach for it directly when VoIP.ms adds a
    /// method this crate hasn't been regenerated for; otherwise prefer
    /// the typed [`Client::call`] or one of the per-method wrappers.
    pub async fn call_raw<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let (body, empty) = self.fetch(method, params, Transport::Get).await?;
        if let Some(status) = empty {
            return Err(Error::Api(status));
        }
        Ok(body)
    }

    /// Issue a `multipart/form-data` POST for `method` and return the full
    /// JSON response body as a [`serde_json::Value`].
    ///
    /// The multipart counterpart of [`Client::call_raw`], with the same
    /// verbatim contract on the `status` field. Every parameter travels as a
    /// form field -- credentials and `method` included -- so nothing is bounded
    /// by the 8190-byte request line the API's front end accepts, which a
    /// base64 file payload overruns many times over. The multipart encoding is
    /// not interchangeable with `application/x-www-form-urlencoded`: `rest.php`
    /// hands one of those to a SOAP handler and answers with an XML fault.
    pub async fn call_multipart_raw<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let (body, empty) = self.fetch(method, params, Transport::MultipartPost).await?;
        if let Some(status) = empty {
            return Err(Error::Api(status));
        }
        Ok(body)
    }

    /// Issue a request for `method` and return the raw JSON response body
    /// verbatim, *without* classifying its `status` field -- a non-`success`
    /// status is returned as-is in the body rather than as an [`Error::Api`].
    /// Only a transport failure or a missing-JSON body is an `Err`.
    ///
    /// Unlike [`Client::call_raw`], this surfaces the whole envelope even for a
    /// genuine error status, so a caller can inspect exactly what the server
    /// returned (the status plus any diagnostic fields). Prefer [`Client::call`]
    /// or [`Client::call_raw`] for normal use; reach for this to diagnose an
    /// unexpected error status. An empty body reads as `{"status":"success"}`;
    /// a non-empty body that isn't JSON is an [`Error::InvalidResponse`].
    ///
    /// Gated behind the `unchecked-raw` feature.
    #[cfg(feature = "unchecked-raw")]
    pub async fn call_raw_unchecked<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        self.send(method, params, Transport::Get).await
    }

    /// Issue a request and deserialize the full JSON response body into `T`.
    ///
    /// Like [`Client::call_raw`], a non-`success` status is returned as
    /// [`Error::Api`] -- except an empty-collection status
    /// ([`ApiStatus::is_empty`]), which deserializes into `T` with its
    /// collection fields defaulting to `None` rather than erroring.
    pub async fn call<P, T>(&self, method: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (body, _empty) = self.fetch(method, params, Transport::Get).await?;
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// Issue a `multipart/form-data` POST for `method` and deserialize the full
    /// JSON response body into `T`.
    ///
    /// The multipart counterpart of [`Client::call`], with the same handling of
    /// an empty-collection status. See [`Client::call_multipart_raw`] for what
    /// the transport changes and why a base64 file payload needs it.
    pub async fn call_multipart<P, T>(&self, method: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (body, _empty) = self.fetch(method, params, Transport::MultipartPost).await?;
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// Issue a request and deserialize a JSON subtree selected by JSON pointer.
    ///
    /// Use this when the API wraps the interesting data under a known key
    /// (e.g. `/balance` or `/dids`).
    ///
    /// As with [`Client::call`], an empty-collection status
    /// ([`ApiStatus::is_empty`]) is not an error; it carries no data subtree,
    /// so the pointer resolves to JSON `null` and `T`'s fields default to
    /// `None`.
    pub async fn call_at<P, T>(&self, method: &str, params: &P, pointer: &str) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (body, empty) = self.fetch(method, params, Transport::Get).await?;
        let subtree = match body.pointer(pointer) {
            Some(v) => v.clone(),
            None if empty.is_some() => Value::Null,
            None => {
                return Err(Error::InvalidResponse(format!(
                    "response missing JSON pointer `{pointer}` for method `{method}`"
                )));
            }
        };

        serde_json::from_value(subtree).map_err(|e| {
            Error::InvalidResponse(format!(
                "failed to deserialize JSON pointer `{pointer}` for method `{method}`: {e}"
            ))
        })
    }

    /// The base URL this client posts to.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

/// Stands in for the API password wherever a client is formatted.
const REDACTED: &str = "<redacted>";

/// `url` with any `user:pass@` userinfo removed.
fn without_userinfo(url: &Url) -> Url {
    let mut url = url.clone();
    // Both setters fail only on a cannot-be-a-base URL, which has no userinfo
    // to strip in the first place.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url
}

// `Debug` on `Client` and `ClientBuilder` is hand-written rather than derived
// because a derive prints `api_password` verbatim, and one `{:?}` reaching a
// log or a downstream error message leaks a live account password. The other
// two fields are deliberate:
//
// * `api_username` is shown. The account email says which account a client
//   speaks for -- the reason to format one at all -- and authenticates nothing
//   on its own.
// * `base_url` is shown with userinfo stripped. A caller may point the client
//   at a proxy whose URL embeds `user:pass@`, which `Url`'s own `Display`
//   prints verbatim.
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base_url = without_userinfo(&self.base_url);
        f.debug_struct("Client")
            .field("http", &self.http)
            .field("base_url", &base_url.as_str())
            .field("api_username", &self.api_username)
            .field("api_password", &REDACTED)
            .finish()
    }
}

/// Builder for [`Client`].
pub struct ClientBuilder {
    http: Option<reqwest::Client>,
    base_url: Option<Url>,
    api_username: String,
    api_password: String,
}

impl ClientBuilder {
    /// Use a custom [`reqwest::Client`] (e.g. with a proxy, custom timeouts,
    /// or custom TLS configuration).
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    /// Override the API base URL. The default is [`DEFAULT_BASE_URL`].
    pub fn base_url(mut self, url: Url) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Finalize the builder.
    pub fn build(self) -> Result<Client> {
        let base_url = match self.base_url {
            Some(u) => u,
            None => Url::parse(DEFAULT_BASE_URL).map_err(|e| {
                Error::InvalidResponse(format!("default base URL failed to parse: {e}"))
            })?,
        };
        let http = self.http.unwrap_or_default();
        Ok(Client {
            http,
            base_url,
            api_username: self.api_username,
            api_password: self.api_password,
        })
    }
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base_url = self
            .base_url
            .as_ref()
            .map(|u| without_userinfo(u).to_string());
        f.debug_struct("ClientBuilder")
            .field("http", &self.http)
            .field("base_url", &base_url)
            .field("api_username", &self.api_username)
            .field("api_password", &REDACTED)
            .finish()
    }
}

/// Classify a response's `status` field.
///
/// Returns `Ok(None)` for `success`, `Err(Error::Api)` for a genuine failure,
/// and `Ok(Some(status))` for an empty-collection status
/// ([`ApiStatus::is_empty`], e.g. `no_sms`) -- VoIP.ms's per-method "the list
/// is empty" code. Whether that case is an error is left to the caller:
/// [`Client::call_raw`] surfaces it verbatim, while the typed
/// [`Client::call`] folds it into an empty response.
fn check_status(body: &Value) -> Result<Option<ApiStatus>> {
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidResponse("response missing `status` field".into()))?;
    if status == "success" {
        return Ok(None);
    }
    let status = ApiStatus::from_wire(status);
    if status.is_empty() {
        Ok(Some(status))
    } else {
        Err(Error::Api(status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_default_base_url() {
        let c = Client::new("user", "pass");
        assert_eq!(c.base_url().as_str(), DEFAULT_BASE_URL);
    }

    #[test]
    fn builder_overrides_base_url_and_http_client() {
        let url = Url::parse("https://example.test/api").unwrap();
        let c = Client::builder("u", "p")
            .base_url(url.clone())
            .http_client(reqwest::Client::new())
            .build()
            .unwrap();
        assert_eq!(c.base_url(), &url);
    }

    #[test]
    fn clone_shares_configuration() {
        let c = Client::new("u", "p");
        let c2 = c.clone();
        assert_eq!(c.base_url(), c2.base_url());
    }

    #[test]
    fn debug_redacts_the_password() {
        let c = Client::new("user@example.com", "hunter2");
        let rendered = format!("{c:?}");

        assert!(!rendered.contains("hunter2"), "Client Debug: {rendered}");
        assert!(rendered.contains(REDACTED), "Client Debug: {rendered}");
        assert!(
            rendered.contains("user@example.com"),
            "Client Debug should still identify the account: {rendered}"
        );

        let b = Client::builder("user@example.com", "hunter2");
        let rendered = format!("{b:?}");

        assert!(
            !rendered.contains("hunter2"),
            "ClientBuilder Debug: {rendered}"
        );
        assert!(
            rendered.contains(REDACTED),
            "ClientBuilder Debug: {rendered}"
        );
    }

    #[test]
    fn debug_strips_base_url_userinfo() {
        let url = Url::parse("https://proxyuser:proxypass@example.test/api").unwrap();
        let c = Client::builder("u", "p")
            .base_url(url.clone())
            .build()
            .unwrap();

        let rendered = format!("{c:?}");
        assert!(!rendered.contains("proxypass"), "Client Debug: {rendered}");
        assert!(!rendered.contains("proxyuser"), "Client Debug: {rendered}");

        let rendered = format!("{:?}", Client::builder("u", "p").base_url(url.clone()));
        assert!(
            !rendered.contains("proxypass"),
            "ClientBuilder Debug: {rendered}"
        );

        // Stripping is presentational only -- the client still requests the URL
        // it was given, credentials included.
        assert_eq!(c.base_url(), &url);
    }

    #[test]
    fn check_status_classifies_each_case() {
        assert!(matches!(
            check_status(&serde_json::json!({"status": "success"})),
            Ok(None)
        ));
        assert!(matches!(
            check_status(&serde_json::json!({"status": "no_sms"})),
            Ok(Some(_))
        ));
        assert!(matches!(
            check_status(&serde_json::json!({"status": "invalid_credentials"})),
            Err(Error::Api(_))
        ));
        assert!(matches!(
            check_status(&serde_json::json!({})),
            Err(Error::InvalidResponse(_))
        ));
    }
}
