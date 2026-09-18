use reqwest::Url;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::fmt;

use crate::error::{ApiStatus, Error, Result};

/// Default base URL for the VoIP.ms REST API.
pub const DEFAULT_BASE_URL: &str = "https://voip.ms/api/v1/rest.php";

/// Async client for the VoIP.ms REST API.
///
/// Clients are cheap to clone; the underlying [`reqwest::Client`] uses an
/// internal connection pool that is shared across clones.
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

        let text = response.text().await?;
        // Some methods (e.g. delConference) answer a successful call with an
        // empty body instead of a `{"status":"success"}` envelope; treat that
        // as success rather than a JSON parse error.
        let body: Value = if text.trim().is_empty() {
            json!({ "status": "success" })
        } else {
            serde_json::from_str(&text)
                .map_err(|e| Error::InvalidResponse(format!("response body is not JSON: {e}")))?
        };
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
    /// unexpected error status. An empty body reads as `{"status":"success"}`;
    /// a non-empty body that isn't JSON is an [`Error::InvalidResponse`].
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

        let text = response.text().await?;
        if text.trim().is_empty() {
            return Ok(json!({ "status": "success" }));
        }

        serde_json::from_str(&text)
            .map_err(|e| Error::InvalidResponse(format!("response body is not JSON: {e}")))
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

    /// Issue a request and deserialize the response, first attaching `offset`
    /// to the timestamps `timestamps` reaches.
    ///
    /// The record-listing methods shift their timestamps by the UTC offset the
    /// request carried and then report the shifted wall clock without it, so a
    /// value only names an instant once the offset is put back. Each entry is a
    /// path in the form [`attach_offset`] takes.
    ///
    /// Empty-collection statuses fold into an empty response, as in
    /// [`Client::call`].
    pub(crate) async fn call_zoned<P, T>(
        &self,
        method: &str,
        params: &P,
        offset: chrono::FixedOffset,
        timestamps: &[&str],
    ) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (mut body, _empty) = self.fetch(method, params).await?;
        attach_offset(&mut body, offset, timestamps);
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// The base URL this client posts to.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

/// Attach `offset` to the bare wall-clock timestamps `timestamps` reaches in
/// `body`, so each parses as the instant it names.
///
/// The record-listing methods (`getCDR`, `getSMS`, …) report their timestamps
/// in the UTC offset the request asked for but leave the offset off the value.
/// The typed methods attach it before deserializing; a [`Client::call_raw`]
/// caller on one of those methods needs the same step, which is why this is
/// public.
///
/// Each entry is a `/`-separated path into `body` in which a `*` segment stands
/// for every element of a list -- `/cdr/*/date` reaches the `date` of every
/// record. A `*` also matches the bare object VoIP.ms returns in place of a
/// one-element list. A value that already names a zone is left alone.
///
/// ```
/// use voip_ms::{attach_offset, chrono::FixedOffset, serde_json::json};
///
/// let mut body = json!({ "cdr": [{ "date": "2026-09-16 15:14:35" }] });
/// attach_offset(&mut body, FixedOffset::west_opt(4 * 3600).unwrap(), &["/cdr/*/date"]);
/// assert_eq!(body["cdr"][0]["date"], "2026-09-16 15:14:35-04:00");
/// ```
pub fn attach_offset(body: &mut Value, offset: chrono::FixedOffset, timestamps: &[&str]) {
    for path in timestamps {
        attach_at(body, path.trim_start_matches('/'), &offset.to_string());
    }
}

/// Walk one [`attach_offset`] path, appending `suffix` to the string it lands on.
fn attach_at(value: &mut Value, path: &str, suffix: &str) {
    let Some((segment, rest)) = path.split_once('/') else {
        if let Some(Value::String(s)) = value.get_mut(path)
            && !names_zone(s)
        {
            s.push_str(suffix);
        }

        return;
    };

    if segment != "*" {
        if let Some(child) = value.get_mut(segment) {
            attach_at(child, rest, suffix);
        }

        return;
    }

    match value {
        Value::Array(items) => {
            for item in items {
                attach_at(item, rest, suffix);
            }
        }

        // VoIP.ms returns a one-element list as a bare object, which
        // `deserialize_vec_from_single_or_seq` accepts on the way in.
        other => attach_at(other, rest, suffix),
    }
}

/// Whether a timestamp already names a UTC offset (`Z`, `-04:00`, `+0530`).
/// VoIP.ms reports none today; leaving such a value alone keeps a second
/// suffix off it should that change.
fn names_zone(s: &str) -> bool {
    let s = s.trim_end();
    s.ends_with('Z')
        || s.rsplit_once(['+', '-']).is_some_and(|(head, zone)| {
            !head.is_empty()
                && (zone.len() == 4 || zone.len() == 5)
                && zone.chars().all(|c| c.is_ascii_digit() || c == ':')
        })
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

    /// UTC-04:00, the offset `America/New_York` resolves to in September.
    fn west4() -> chrono::FixedOffset {
        chrono::FixedOffset::west_opt(4 * 3600).expect("4 hours is a valid offset")
    }

    #[test]
    fn attach_offset_reaches_every_record_in_a_list() {
        let mut body = serde_json::json!({
            "status": "success",
            "cdr": [
                { "date": "2026-09-16 15:14:35" },
                { "date": "2026-09-16 16:02:00" },
            ],
        });
        attach_offset(&mut body, west4(), &["/cdr/*/date"]);
        assert_eq!(body["cdr"][0]["date"], "2026-09-16 15:14:35-04:00");
        assert_eq!(body["cdr"][1]["date"], "2026-09-16 16:02:00-04:00");
        assert_eq!(body["status"], "success");
    }

    #[test]
    fn attach_offset_reaches_a_bare_record() {
        // A one-row list arrives as the object itself.
        let mut body = serde_json::json!({ "sms": { "date": "2026-03-30 10:24:16" } });
        attach_offset(&mut body, west4(), &["/sms/*/date"]);
        assert_eq!(body["sms"]["date"], "2026-03-30 10:24:16-04:00");
    }

    #[test]
    fn attach_offset_leaves_absent_and_non_string_values_alone() {
        let mut body = serde_json::json!({
            "cdr": [{ "seconds": "11" }, { "date": null }, { "date": 0 }],
        });
        attach_offset(&mut body, west4(), &["/cdr/*/date", "/missing/*/date"]);
        assert_eq!(body["cdr"][0].get("date"), None);
        assert_eq!(body["cdr"][1]["date"], serde_json::Value::Null);
        assert_eq!(body["cdr"][2]["date"], 0);
    }

    #[test]
    fn attach_offset_leaves_a_value_that_already_names_a_zone() {
        let mut body = serde_json::json!({
            "cdr": [
                { "date": "2026-09-16 15:14:35-05:00" },
                { "date": "2026-09-16T15:14:35Z" },
                { "date": "2026-09-16 15:14:35+0530" },
            ],
        });
        attach_offset(&mut body, west4(), &["/cdr/*/date"]);
        assert_eq!(body["cdr"][0]["date"], "2026-09-16 15:14:35-05:00");
        assert_eq!(body["cdr"][1]["date"], "2026-09-16T15:14:35Z");
        assert_eq!(body["cdr"][2]["date"], "2026-09-16 15:14:35+0530");
    }
}
