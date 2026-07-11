use reqwest::Url;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{ApiStatus, Error, Result};

/// Default base URL for the VoIP.ms REST API.
pub const DEFAULT_BASE_URL: &str = "https://voip.ms/api/v1/rest.php";

/// Async client for the VoIP.ms REST API.
///
/// Clients are cheap to clone; the underlying [`reqwest::Client`] uses an
/// internal connection pool that is shared across clones.
#[derive(Debug, Clone)]
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

    /// Issue the GET request for `method` and return the parsed JSON body
    /// together with its classified status, without rejecting empty-collection
    /// statuses. The two callers differ only in how they treat that case.
    async fn fetch<P>(&self, method: &str, params: &P) -> Result<(Value, Option<ApiStatus>)>
    where
        P: Serialize + ?Sized,
    {
        let response = self
            .http
            .get(self.base_url.clone())
            .query(&[
                ("api_username", self.api_username.as_str()),
                ("api_password", self.api_password.as_str()),
                ("method", method),
            ])
            .query(params)
            .send()
            .await?
            .error_for_status()?;

        let body: Value = response.json().await?;
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
        let (body, empty) = self.fetch(method, params).await?;
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
    /// unexpected error status.
    ///
    /// Gated behind the `unchecked-raw` feature.
    #[cfg(feature = "unchecked-raw")]
    pub async fn call_raw_unchecked<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let response = self
            .http
            .get(self.base_url.clone())
            .query(&[
                ("api_username", self.api_username.as_str()),
                ("api_password", self.api_password.as_str()),
                ("method", method),
            ])
            .query(params)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
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
        let (body, _empty) = self.fetch(method, params).await?;
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
        let (body, empty) = self.fetch(method, params).await?;
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

/// Builder for [`Client`].
#[derive(Debug)]
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
