use reqwest::Url;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{ApiStatus, Error, Result};

/// Default base URL for the voip.ms REST API.
pub const DEFAULT_BASE_URL: &str = "https://voip.ms/api/v1/rest.php";

/// Async client for the voip.ms REST API.
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
            .expect("default voip.ms base URL must parse")
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

    /// Issue a request for `method` with the given typed parameters and
    /// return the full JSON response body as a [`serde_json::Value`].
    ///
    /// The `status` field is inspected: any value other than `success`
    /// causes an [`Error::Api`].
    ///
    /// This is the low-level raw call used by every generated `*_raw`
    /// method on [`Client`]. Reach for it directly when voip.ms adds a
    /// method this crate hasn't been regenerated for; otherwise prefer
    /// the typed [`Client::call`] or one of the per-method wrappers.
    pub async fn call_raw<P>(&self, method: &str, params: &P) -> Result<Value>
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
        check_status(&body)?;
        Ok(body)
    }

    /// Issue a request and deserialize the full JSON response body into `T`.
    ///
    /// This applies the same status validation as [`Client::call_raw`]:
    /// any non-`success` status is returned as [`Error::Api`].
    pub async fn call<P, T>(&self, method: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let body = self.call_raw(method, params).await?;
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// Issue a request and deserialize a JSON subtree selected by JSON pointer.
    ///
    /// Use this when the API wraps the interesting data under a known key
    /// (e.g. `/balance` or `/dids`).
    pub async fn call_at<P, T>(&self, method: &str, params: &P, pointer: &str) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let body = self.call_raw(method, params).await?;
        let subtree = body.pointer(pointer).cloned().ok_or_else(|| {
            Error::InvalidResponse(format!(
                "response missing JSON pointer `{pointer}` for method `{method}`"
            ))
        })?;

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

fn check_status(body: &Value) -> Result<()> {
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidResponse("response missing `status` field".into()))?;
    if status == "success" {
        Ok(())
    } else {
        Err(Error::Api(ApiStatus(status.to_owned())))
    }
}
