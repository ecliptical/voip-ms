use reqwest::{Url, multipart};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::fmt;
use std::sync::LazyLock;

use crate::error::{ApiStatus, Error, ParamsError, Result};
use crate::form;

/// Default base URL for the VoIP.ms REST API.
pub const DEFAULT_BASE_URL: &str = "https://voip.ms/api/v1/rest.php";

/// [`DEFAULT_BASE_URL`] parsed once, so building a client cannot fail on a
/// literal this crate controls.
static DEFAULT_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse(DEFAULT_BASE_URL).expect("the default base URL is a literal and must parse")
});

/// Where a request carries its parameters.
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

impl Transport {
    /// The transport `method` travels over, read from the generated table. A
    /// method absent from it is a GET.
    fn for_method(method: &str) -> Self {
        if crate::requires_multipart(method) {
            Self::MultipartPost
        } else {
            Self::Get
        }
    }
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
    pub fn new(api_username: impl Into<String>, api_password: impl Into<String>) -> Self {
        Self::builder(api_username, api_password).build()
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

    /// Every field the request carries: credentials, `method`, then `params`.
    ///
    /// Rendered once for both transports, so a value rides differently on each
    /// but is written the same way on both. A parameter with no field rendering
    /// -- a nested structure, or a value that is not a scalar -- is
    /// [`Error::InvalidParams`] and nothing is sent.
    ///
    /// Everything this function can borrow, it borrows: the three fixed names,
    /// the credentials, and the method. Only a rendered parameter value has to
    /// be owned, since `serde` hands one over for the length of a call. A GET
    /// is the common case and it pays for the fields it does not need owned.
    fn wire_fields<'a, P>(
        &'a self,
        method: &'a str,
        params: &P,
    ) -> Result<Vec<(Cow<'static, str>, Cow<'a, str>)>>
    where
        P: Serialize + ?Sized,
    {
        let mut fields = vec![
            (
                Cow::Borrowed("api_username"),
                Cow::Borrowed(self.api_username.as_str()),
            ),
            (
                Cow::Borrowed("api_password"),
                Cow::Borrowed(self.api_password.as_str()),
            ),
            (Cow::Borrowed("method"), Cow::Borrowed(method)),
        ];
        fields.extend(
            form::to_fields(params)
                .map_err(|e| Error::InvalidParams(ParamsError::Unencodable(e.into_message())))?
                .into_iter()
                .map(|(name, value)| (name, Cow::Owned(value))),
        );

        Ok(fields)
    }

    /// Issue the request for `method` over `transport` and return its parsed
    /// JSON body, without classifying the `status` field.
    async fn send<P>(&self, method: &str, params: &P, transport: Transport) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let fields = self.wire_fields(method, params)?;
        let request = match transport {
            Transport::Get => self.http.get(self.base_url.clone()).query(&fields),
            // A form part owns what it carries, so the three borrowed fields are
            // copied here rather than on every GET.
            Transport::MultipartPost => {
                let form = fields
                    .into_iter()
                    .fold(multipart::Form::new(), |form, (name, value)| {
                        form.text(name, value.into_owned())
                    });
                self.http.post(self.base_url.clone()).multipart(form)
            }
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

    /// [`Client::fetch`], with an empty-collection status rejected as
    /// [`Error::Api`] like any other non-`success` one.
    async fn raw<P>(&self, method: &str, params: &P, transport: Transport) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        let (body, empty) = self.fetch(method, params, transport).await?;
        if let Some(status) = empty {
            return Err(Error::Api(status));
        }

        Ok(body)
    }

    /// Issue a request for `method` with the given typed parameters and
    /// return the full JSON response body as a [`serde_json::Value`].
    ///
    /// The `status` field is inspected: any value other than `success`
    /// causes an [`Error::Api`] -- including the empty-collection statuses
    /// ([`ApiStatus::is_empty_collection`], e.g. `no_sms`). This is the verbatim escape
    /// hatch: it surfaces exactly what VoIP.ms returned. The typed
    /// [`Client::call`] instead folds those into an empty response.
    ///
    /// This is the low-level raw call used by every generated `*_raw`
    /// method on [`Client`]. Reach for it directly when holding a wire method
    /// name rather than calling a generated method, or when VoIP.ms adds a
    /// method this crate hasn't been regenerated for; otherwise prefer the
    /// typed [`Client::call`] or one of the per-method wrappers.
    ///
    /// The transport is chosen for the caller from `method`. A method in the
    /// generated table whose parameters carry a base64 file
    /// ([`requires_multipart`](crate::requires_multipart)) is a
    /// `multipart/form-data` POST, since the file does not fit the request line
    /// a GET puts it on; every other method is a GET. A method this crate has
    /// not been regenerated for is absent from the table and so is sent as a
    /// GET whatever parameters it takes: an upload method the crate has never
    /// seen needs [`Client::call_multipart_raw`].
    ///
    /// ```no_run
    /// # async fn example(client: &voip_ms::Client) -> voip_ms::Result<()> {
    /// use voip_ms::serde_json::json;
    ///
    /// // A multipart POST, because `setRecording` carries a file.
    /// let envelope = client
    ///     .call_raw("setRecording", &json!({ "name": "greeting", "file": "UklGRg==" }))
    ///     .await?;
    /// # let _ = envelope;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The envelope is what VoIP.ms sent, so for the record-listing methods
    /// (`getCDR`, `getSMS`, …) its timestamps are wall clocks with no offset,
    /// and the typed response deserializers refuse them. Send those methods an
    /// explicit `timezone`, then complete the envelope with
    /// [`attach_offset`](crate::attach_offset) over the paths
    /// [`offset_timestamps`](crate::offset_timestamps) answers for the method:
    ///
    /// ```no_run
    /// # async fn example(client: &voip_ms::Client, method: &str) -> Result<(), Box<dyn std::error::Error>> {
    /// use voip_ms::{TimezoneOffset, attach_offset, offset_timestamps, serde_json::json};
    ///
    /// let offset = TimezoneOffset::new(-4)?;
    /// let mut envelope = client
    ///     .call_raw(
    ///         method,
    ///         &json!({ "date_from": "2026-09-01", "date_to": "2026-09-16", "timezone": offset }),
    ///     )
    ///     .await?;
    /// if let Some(timestamps) = offset_timestamps(method) {
    ///     attach_offset(&mut envelope, offset.to_fixed_offset(), timestamps);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_raw<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        self.raw(method, params, Transport::for_method(method))
            .await
    }

    /// Issue a `multipart/form-data` POST for `method` and return the full
    /// JSON response body as a [`serde_json::Value`].
    ///
    /// The one call that takes the transport from its caller: for an upload
    /// method this crate has not been regenerated for, which
    /// [`requires_multipart`](crate::requires_multipart) answers `false` for
    /// and [`Client::call_raw`] would therefore send as a GET. A method in the
    /// generated table needs none of this; `call_raw` already posts the ones
    /// that carry a file.
    ///
    /// The `status` field is classified as in [`Client::call_raw`]. Every
    /// parameter travels as a form field -- credentials and `method` included
    /// -- so nothing is bounded by the 8190-byte request line the API's front
    /// end accepts, which a base64 file payload overruns many times over. The
    /// multipart encoding is not interchangeable with
    /// `application/x-www-form-urlencoded`: `rest.php` hands one of those to a
    /// SOAP handler and answers with an XML fault.
    pub async fn call_multipart_raw<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        self.raw(method, params, Transport::MultipartPost).await
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
    /// The transport is chosen from `method` as in [`Client::call_raw`], so a
    /// dump of what a method answered goes out the same way its typed call did.
    ///
    /// Gated behind the `unchecked-raw` feature.
    #[cfg(feature = "unchecked-raw")]
    pub async fn call_raw_unchecked<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        self.send(method, params, Transport::for_method(method))
            .await
    }

    /// The same unclassified envelope as [`Client::call_raw_unchecked`], over a
    /// `multipart/form-data` POST: for an upload method this crate has not been
    /// regenerated for, as [`Client::call_multipart_raw`] is.
    ///
    /// Gated behind the `unchecked-raw` feature.
    #[cfg(feature = "unchecked-raw")]
    pub async fn call_multipart_raw_unchecked<P>(&self, method: &str, params: &P) -> Result<Value>
    where
        P: Serialize + ?Sized,
    {
        self.send(method, params, Transport::MultipartPost).await
    }

    /// Issue a request and deserialize the full JSON response body into `T`.
    ///
    /// Like [`Client::call_raw`], a non-`success` status is returned as
    /// [`Error::Api`] -- except an empty-collection status
    /// ([`ApiStatus::is_empty_collection`]), which deserializes into `T` with its
    /// collection fields defaulting to `None` rather than erroring. The
    /// transport is chosen from `method` as in [`Client::call_raw`].
    pub async fn call<P, T>(&self, method: &str, params: &P) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (body, _empty) = self
            .fetch(method, params, Transport::for_method(method))
            .await?;
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
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
        let (mut body, _empty) = self
            .fetch(method, params, Transport::for_method(method))
            .await?;
        attach_offset(&mut body, offset, timestamps);
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// The base URL this client posts to.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// The account this client authenticates as.
    pub fn api_username(&self) -> &str {
        &self.api_username
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
/// one-element list.
///
/// [`offset_timestamps`](crate::offset_timestamps) answers each record-listing
/// method's paths from its wire name, so a raw caller names the method rather
/// than spelling the paths out.
///
/// A blank value and one that already names a zone are both left alone -- the
/// first has no wall clock to qualify and stays the empty placeholder the
/// deserializers fold to `None`.
///
/// ```
/// use voip_ms::{attach_offset, chrono::FixedOffset, serde_json::json};
///
/// let mut body = json!({ "cdr": [{ "date": "2026-09-16 15:14:35" }] });
/// attach_offset(&mut body, FixedOffset::west_opt(4 * 3600).unwrap(), &["/cdr/*/date"]);
/// assert_eq!(body["cdr"][0]["date"], "2026-09-16 15:14:35-04:00");
/// ```
pub fn attach_offset(body: &mut Value, offset: chrono::FixedOffset, timestamps: &[&str]) {
    let suffix = offset.to_string();
    for path in timestamps {
        attach_at(body, path.trim_start_matches('/'), &suffix);
    }
}

/// Walk one [`attach_offset`] path, appending `suffix` to the string it lands on.
fn attach_at(value: &mut Value, path: &str, suffix: &str) {
    let Some((segment, rest)) = path.split_once('/') else {
        if let Some(Value::String(s)) = value.get_mut(path)
            && !s.trim().is_empty()
            && !crate::responses::names_offset(s)
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
    pub fn build(self) -> Client {
        Client {
            http: self.http.unwrap_or_default(),
            base_url: self.base_url.unwrap_or_else(|| DEFAULT_URL.clone()),
            api_username: self.api_username,
            api_password: self.api_password,
        }
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
/// ([`ApiStatus::is_empty_collection`], e.g. `no_sms`) -- VoIP.ms's per-method "the list
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
    if status.is_empty_collection() {
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
            .build();
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
        let c = Client::builder("u", "p").base_url(url.clone()).build();

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
    fn attach_offset_leaves_a_blank_value_alone() {
        // A blank has no wall clock to qualify. Suffixing it would produce
        // `-04:00`, which parses as nothing -- and since one bad value fails
        // the whole response, that would lose every other record too.
        let mut body = serde_json::json!({
            "cdr": [{ "date": "" }, { "date": "   " }],
        });
        attach_offset(&mut body, west4(), &["/cdr/*/date"]);
        assert_eq!(body["cdr"][0]["date"], "");
        assert_eq!(body["cdr"][1]["date"], "   ");
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
