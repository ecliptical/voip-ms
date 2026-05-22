use reqwest::Url;
use serde::Serialize;
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

    /// Issue a request for `method` with the given typed parameters.
    ///
    /// Returns the full JSON response body as a [`serde_json::Value`]. The
    /// `status` field is inspected: any value other than `success` causes an
    /// [`Error::Api`].
    ///
    /// This is the low-level call used by every generated method on
    /// [`Client`]. Reach for it directly when:
    ///
    /// * voip.ms adds a method this crate hasn't been regenerated for, or
    /// * you want to deserialize the response into your own type.
    ///
    /// For the typed deserialization case, use [`serde_json::from_value`] on
    /// the returned [`Value`].
    pub async fn call<P>(&self, method: &str, params: &P) -> Result<Value>
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
