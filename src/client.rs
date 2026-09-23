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
    /// The envelope is what VoIP.ms sent, so its timestamps are wall clocks with
    /// no offset. For the record-listing methods (`getCDR`, `getSMS`, …) send an
    /// explicit `timezone`, then qualify the envelope with
    /// [`attach_offset`](crate::attach_offset) over the paths
    /// [`offset_timestamps`](crate::offset_timestamps) answers for the method.
    /// For a method whose timestamps are in a named zone, use
    /// [`attach_zone`](crate::attach_zone) over the paths
    /// [`zone_timestamps`](crate::zone_timestamps) answers:
    ///
    /// ```no_run
    /// # async fn example(client: &voip_ms::Client, method: &str) -> Result<(), Box<dyn std::error::Error>> {
    /// use voip_ms::{
    ///     TimezoneOffset, attach_offset, attach_zone, chrono::NaiveDate, chrono_tz::Tz,
    ///     offset_timestamps, serde_json::json, zone_timestamps,
    /// };
    ///
    /// let day = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
    /// let timezone = TimezoneOffset::for_window(Tz::America__Vancouver, day)?;
    /// let mut envelope = client
    ///     .call_raw(
    ///         method,
    ///         &json!({ "date_from": day, "date_to": "2026-09-16", "timezone": timezone }),
    ///     )
    ///     .await?;
    /// if let Some(timestamps) = offset_timestamps(method) {
    ///     attach_offset(&mut envelope, timezone, timestamps);
    /// } else if let Some(zoned) = zone_timestamps(method) {
    ///     // `None` for a mailbox's zone, which the caller reads from
    ///     // `getVoicemails`.
    ///     if let Some(zone) = zoned.zone.zone() {
    ///         attach_zone(&mut envelope, zone, zoned.paths);
    ///     }
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
        self.call_qualified(method, params, |_| {}).await
    }

    /// [`Client::call`], letting `qualify` complete the envelope before it is
    /// deserialized.
    async fn call_qualified<P, T>(
        &self,
        method: &str,
        params: &P,
        qualify: impl FnOnce(&mut Value),
    ) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let (mut body, _empty) = self
            .fetch(method, params, Transport::for_method(method))
            .await?;
        qualify(&mut body);
        serde_json::from_value(body)
            .map_err(|e| Error::InvalidResponse(format!("failed to deserialize response: {e}")))
    }

    /// Issue a request and deserialize the response, first qualifying the
    /// timestamps `timestamps` reaches with [`attach_offset`] for the
    /// `timezone` the request carried.
    ///
    /// Empty-collection statuses fold into an empty response, as in
    /// [`Client::call`].
    pub(crate) async fn call_zoned<P, T>(
        &self,
        method: &str,
        params: &P,
        timezone: crate::TimezoneOffset,
        timestamps: &[&str],
    ) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.call_qualified(method, params, |body| {
            attach_offset(body, timezone, timestamps);
        })
        .await
    }

    /// Issue a request and deserialize the response, first qualifying the wall
    /// clocks `timestamps` reaches in `zone`.
    ///
    /// For a method whose timestamps are rendered in a named zone the request
    /// cannot choose and the response does not report, such as a mailbox's own
    /// `timezone`. Each entry is a path in the form [`attach_zone`] takes, and
    /// the call is still one request: the zone comes from the caller.
    ///
    /// Empty-collection statuses fold into an empty response, as in
    /// [`Client::call`].
    pub(crate) async fn call_in_zone<P, T>(
        &self,
        method: &str,
        params: &P,
        zone: chrono_tz::Tz,
        timestamps: &[&str],
    ) -> Result<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.call_qualified(method, params, |body| {
            attach_zone(body, zone, timestamps);
        })
        .await
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

/// Qualify the timestamps `timestamps` reaches in a record-listing envelope
/// that was requested with `timezone`, so each parses as the instant it names.
///
/// The record-listing methods (`getCDR`, `getSMS`, …) report each timestamp
/// as its [`SERVER_ZONE`](crate::SERVER_ZONE) wall clock moved by
/// `timezone + 5` hours, with no offset. That equals the wall clock at
/// UTC+`timezone` only outside DST, so the number sent is not an offset to
/// attach. This moves each value back, resolves it in `SERVER_ZONE`, and
/// writes it with the offset that zone was at then (`-04:00` or `-05:00`).
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
/// A moved-back wall clock that has no single offset in `SERVER_ZONE` -- the
/// hour it repeats when clocks fall back -- is written as that wall clock
/// followed by the zone's name (`2026-11-01 01:30:00 America/Toronto`) and
/// reads as [`WallClock::Bare`](crate::WallClock::Bare) holding it, so every
/// row of a response is on the same clock. A record-listing field refuses a
/// value that carries neither an offset nor that name, which is what one read
/// without this step looks like. A value is left exactly as it arrived when it
/// is blank, when it already names an offset, and when it is not a
/// `YYYY-MM-DD HH:MM:SS` wall clock.
///
/// ```
/// use voip_ms::{TimezoneOffset, attach_offset, serde_json::json};
///
/// // A call at 16:28:34 UTC, read with `timezone=0` in September.
/// let mut body = json!({ "cdr": [
///     { "date": "2026-09-23 17:28:34" },
///     { "date": "2026-11-01 06:30:00" },
/// ] });
/// attach_offset(&mut body, TimezoneOffset::UTC, &["/cdr/*/date"]);
/// assert_eq!(body["cdr"][0]["date"], "2026-09-23 12:28:34-04:00");
/// // 01:30 Eastern happens twice that night.
/// assert_eq!(body["cdr"][1]["date"], "2026-11-01 01:30:00 America/Toronto");
/// ```
pub fn attach_offset(body: &mut Value, timezone: crate::TimezoneOffset, timestamps: &[&str]) {
    let shift = chrono::TimeDelta::seconds(timezone.shift_seconds());
    qualify_timestamps(body, timestamps, |wall| {
        let local = parse_wall(wall)?.checked_sub_signed(shift)?;
        Some(match resolve_in(crate::SERVER_ZONE, local) {
            Some(at) => Qualified::At(at),
            None => Qualified::InServerZone(local),
        })
    });
}

/// Qualify the bare wall clocks `timestamps` reaches in `body` with the UTC
/// offset `zone` was at when each one happened, so each parses as the instant
/// it names.
///
/// `getVoicemailMessages` renders a message's `date` in the mailbox's current
/// `timezone` setting (`getVoicemails`' `timezone`) at the time it is read, and
/// names neither the zone nor an offset. Given that zone, every value resolves,
/// including one recorded while the mailbox had a different setting, since
/// VoIP.ms renders the stored instant afresh on each read.
///
/// Paths take the form [`attach_offset`] documents, and
/// [`zone_timestamps`](crate::zone_timestamps) answers each such method's
/// paths from its wire name.
///
/// Each value is resolved on its own, so rows on either side of a DST change
/// get different offsets. A value is left alone when it is blank, when it
/// already names an offset, when it is not a `YYYY-MM-DD HH:MM:SS` wall clock,
/// and when the wall clock is ambiguous in `zone` (the repeated hour when
/// clocks fall back) or does not exist in it (the hour skipped when they spring
/// forward). Choosing either side of an ambiguous hour would be a guess, and a
/// value left bare still deserializes, as [`WallClock::Bare`](crate::WallClock::Bare).
///
/// ```
/// use voip_ms::{attach_zone, chrono_tz::America::Toronto, serde_json::json};
///
/// let mut body = json!({ "messages": [
///     { "date": "2026-09-22 18:47:40" },
///     { "date": "2026-11-01 01:30:00" },
/// ] });
/// attach_zone(&mut body, Toronto, &["/messages/*/date"]);
/// assert_eq!(body["messages"][0]["date"], "2026-09-22 18:47:40-04:00");
/// // 01:30 happens twice in Toronto that night, so it stays bare.
/// assert_eq!(body["messages"][1]["date"], "2026-11-01 01:30:00");
/// ```
pub fn attach_zone(body: &mut Value, zone: chrono_tz::Tz, timestamps: &[&str]) {
    qualify_timestamps(body, timestamps, |wall| {
        resolve_in(zone, parse_wall(wall)?).map(Qualified::At)
    });
}

/// What the qualification walk writes in place of a bare wall clock.
enum Qualified {
    /// An instant, written with its offset.
    At(chrono::DateTime<chrono::FixedOffset>),
    /// A [`SERVER_ZONE`](crate::SERVER_ZONE) wall clock with no single offset
    /// there, written with the zone's name.
    InServerZone(chrono::NaiveDateTime),
}

/// A trimmed `YYYY-MM-DD HH:MM:SS` wall clock.
fn parse_wall(wall: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(wall, crate::responses::DATETIME_WIRE_FORMAT).ok()
}

/// `local` as an instant in `zone`, with the offset in force then, or `None`
/// when `zone` repeats or skips that wall clock or is at an offset with a
/// seconds part there (a zone's pre-standard local mean time, which the wire
/// spelling cannot carry, since it stops at minutes).
fn resolve_in(
    zone: chrono_tz::Tz,
    local: chrono::NaiveDateTime,
) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    use chrono::TimeZone;

    let at = zone.from_local_datetime(&local).single()?.fixed_offset();
    (at.offset().local_minus_utc() % 60 == 0).then_some(at)
}

/// Rewrite every string `timestamps` reaches in `body` that holds a wall clock
/// with no offset as what `qualify` gives for it, skipping blanks and values
/// that already name an offset.
///
/// `qualify` is handed the trimmed value, and a value it answers `None` for is
/// left exactly as it arrived.
fn qualify_timestamps(
    body: &mut Value,
    timestamps: &[&str],
    mut qualify: impl FnMut(&str) -> Option<Qualified>,
) {
    for path in timestamps {
        qualify_at(body, path.trim_start_matches('/'), &mut qualify);
    }
}

/// Walk one [`attach_offset`] path, qualifying the string it lands on.
fn qualify_at<F>(value: &mut Value, path: &str, qualify: &mut F)
where
    F: FnMut(&str) -> Option<Qualified>,
{
    use std::fmt::Write as _;

    let Some((segment, rest)) = path.split_once('/') else {
        if let Some(Value::String(s)) = value.get_mut(path) {
            let wall = s.trim();
            if wall.is_empty() || crate::responses::names_offset(wall) {
                return;
            }

            let Some(qualified) = qualify(wall) else {
                return;
            };

            s.clear();
            // Writing to a `String` cannot fail.
            let _ = match qualified {
                Qualified::At(at) => write!(
                    s,
                    "{}",
                    at.format(crate::responses::OFFSET_DATETIME_WIRE_FORMAT)
                ),
                Qualified::InServerZone(local) => write!(
                    s,
                    "{} {}",
                    local.format(crate::responses::DATETIME_WIRE_FORMAT),
                    crate::SERVER_ZONE.name()
                ),
            };
        }

        return;
    };

    if segment != "*" {
        if let Some(child) = value.get_mut(segment) {
            qualify_at(child, rest, qualify);
        }

        return;
    }

    match value {
        Value::Array(items) => {
            for item in items {
                qualify_at(item, rest, qualify);
            }
        }

        // VoIP.ms returns a one-element list as a bare object, which
        // `deserialize_vec_from_single_or_seq` accepts on the way in.
        other => qualify_at(other, rest, qualify),
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

    const CDR_DATES: &[&str] = &["/cdr/*/date"];

    fn number(hours: &str) -> crate::TimezoneOffset {
        crate::TimezoneOffset::new(rust_decimal::Decimal::from_str_exact(hours).unwrap()).unwrap()
    }

    /// Qualify one record-listing wall clock read with `timezone` and return
    /// what it became.
    fn read_with(timezone: &str, wall: &str) -> Value {
        let mut body = serde_json::json!({ "cdr": [{ "date": wall }] });
        attach_offset(&mut body, number(timezone), CDR_DATES);
        body["cdr"][0]["date"].clone()
    }

    /// One call, placed at 16:28:34 UTC on 2026-09-23 and read back live with
    /// six different `timezone` values, is one instant whatever was sent.
    #[test]
    fn attach_offset_undoes_the_measured_shift() {
        for (timezone, reported) in [
            ("0", "2026-09-23 17:28:34"),
            ("-4", "2026-09-23 13:28:34"),
            ("-5", "2026-09-23 12:28:34"),
            ("5.5", "2026-09-23 22:58:34"),
            ("13", "2026-09-24 06:28:34"),
            ("-12", "2026-09-23 05:28:34"),
        ] {
            assert_eq!(
                read_with(timezone, reported),
                "2026-09-23 12:28:34-04:00",
                "timezone={timezone}"
            );
        }
    }

    #[test]
    fn attach_offset_resolves_each_row_in_the_server_zone() {
        // Outside DST the fixed shift makes `timezone=0` report UTC. That is
        // inferred from DST-season readings: this asserts the model, not a
        // winter measurement.
        assert_eq!(
            read_with("0", "2026-01-15 17:00:00"),
            "2026-01-15 12:00:00-05:00"
        );
        // Rows either side of the spring-forward change get their own offset.
        assert_eq!(
            read_with("-5", "2026-03-08 01:59:59"),
            "2026-03-08 01:59:59-05:00"
        );
        assert_eq!(
            read_with("-5", "2026-03-08 03:00:00"),
            "2026-03-08 03:00:00-04:00"
        );
    }

    #[test]
    fn attach_offset_writes_the_repeated_hour_on_the_server_clock() {
        // 06:30 at `timezone=0` moves back to 01:30 Eastern on the night clocks
        // fall back, which happens twice. It keeps the Eastern wall clock, as
        // its resolved neighbors do, and names the zone so a record-listing
        // field can tell it from a value that was never qualified.
        assert_eq!(
            read_with("0", "2026-11-01 06:30:00"),
            "2026-11-01 01:30:00 America/Toronto"
        );
    }

    #[test]
    fn attach_offset_reaches_every_record_in_a_list() {
        let mut body = serde_json::json!({
            "status": "success",
            "cdr": [
                { "date": "2026-09-16 19:14:35" },
                { "date": "2026-09-16 20:02:00" },
            ],
        });
        attach_offset(&mut body, number("0"), CDR_DATES);
        assert_eq!(body["cdr"][0]["date"], "2026-09-16 14:14:35-04:00");
        assert_eq!(body["cdr"][1]["date"], "2026-09-16 15:02:00-04:00");
        assert_eq!(body["status"], "success");
    }

    #[test]
    fn attach_offset_reaches_a_bare_record() {
        // A one-row list arrives as the object itself.
        let mut body = serde_json::json!({ "sms": { "date": "2026-03-30 10:24:16" } });
        attach_offset(&mut body, number("-5"), &["/sms/*/date"]);
        assert_eq!(body["sms"]["date"], "2026-03-30 10:24:16-04:00");
    }

    #[test]
    fn attach_offset_leaves_absent_and_non_string_values_alone() {
        let mut body = serde_json::json!({
            "cdr": [{ "seconds": "11" }, { "date": null }, { "date": 0 }],
        });
        attach_offset(&mut body, number("0"), &["/cdr/*/date", "/missing/*/date"]);
        assert_eq!(body["cdr"][0].get("date"), None);
        assert_eq!(body["cdr"][1]["date"], serde_json::Value::Null);
        assert_eq!(body["cdr"][2]["date"], 0);
    }

    #[test]
    fn attach_offset_leaves_blank_qualified_and_unreadable_values_alone() {
        // A blank has no wall clock to qualify, and one bad value fails the
        // whole response, so rewriting it would lose every other record too.
        for wall in [
            "",
            "   ",
            "2026-09-16 15:14:35-05:00",
            "2026-09-16T15:14:35Z",
            "2026-09-16 15:14:35+0530",
            "0000-00-00 00:00:00",
            "2026-09-16",
        ] {
            assert_eq!(read_with("0", wall), wall);
        }
    }

    const MESSAGE_DATES: &[&str] = &["/messages/*/date"];

    /// Qualify one wall clock in `America/Toronto` and return what it became.
    fn in_toronto(wall: &str) -> Value {
        let mut body = serde_json::json!({ "messages": [{ "date": wall }] });
        attach_zone(&mut body, chrono_tz::America::Toronto, MESSAGE_DATES);
        body["messages"][0]["date"].clone()
    }

    #[test]
    fn attach_zone_resolves_each_row_at_its_own_instant() {
        // Toronto springs forward at 02:00 on 2026-03-08 and falls back at 02:00
        // on 2026-11-01, so rows either side of each change get different
        // offsets from the same zone.
        assert_eq!(
            in_toronto("2026-03-08 01:59:59"),
            "2026-03-08 01:59:59-05:00"
        );
        assert_eq!(
            in_toronto("2026-03-08 03:00:00"),
            "2026-03-08 03:00:00-04:00"
        );
        assert_eq!(
            in_toronto("2026-11-01 00:59:59"),
            "2026-11-01 00:59:59-04:00"
        );
        assert_eq!(
            in_toronto("2026-11-01 02:00:00"),
            "2026-11-01 02:00:00-05:00"
        );
    }

    #[test]
    fn attach_zone_leaves_a_repeated_or_skipped_wall_clock_bare() {
        // 01:30 happens twice when clocks fall back; either offset is a guess.
        assert_eq!(in_toronto("2026-11-01 01:30:00"), "2026-11-01 01:30:00");
        // 02:30 never happens when they spring forward.
        assert_eq!(in_toronto("2026-03-08 02:30:00"), "2026-03-08 02:30:00");
    }

    #[test]
    fn attach_zone_leaves_blank_qualified_and_unreadable_values_alone() {
        assert_eq!(in_toronto(""), "");
        assert_eq!(in_toronto("   "), "   ");
        assert_eq!(
            in_toronto("2026-09-22 18:47:40+02:00"),
            "2026-09-22 18:47:40+02:00"
        );
        assert_eq!(in_toronto("0000-00-00 00:00:00"), "0000-00-00 00:00:00");
        assert_eq!(in_toronto("2026-09-22"), "2026-09-22");
    }

    /// Both helpers rewrite a value they qualify in full, so padding goes, and
    /// leave one they do not qualify exactly as it arrived.
    #[test]
    fn both_helpers_trim_what_they_qualify_and_nothing_else() {
        assert_eq!(
            in_toronto(" 2026-09-22 18:47:40 "),
            "2026-09-22 18:47:40-04:00"
        );
        assert_eq!(in_toronto(" 2026-11-01 01:30:00 "), " 2026-11-01 01:30:00 ");
        assert_eq!(
            read_with("-5", " 2026-09-16 15:14:35 "),
            "2026-09-16 15:14:35-04:00"
        );
    }

    #[test]
    fn attach_zone_reaches_a_bare_record() {
        let mut body = serde_json::json!({ "messages": { "date": "2026-09-22 18:47:40" } });
        attach_zone(&mut body, chrono_tz::Pacific::Honolulu, MESSAGE_DATES);
        assert_eq!(body["messages"]["date"], "2026-09-22 18:47:40-10:00");
    }

    #[test]
    fn attach_zone_writes_utc_as_a_zero_offset() {
        let mut body = serde_json::json!({ "messages": [{ "date": "2026-09-22 22:47:40" }] });
        attach_zone(&mut body, chrono_tz::UTC, MESSAGE_DATES);
        assert_eq!(body["messages"][0]["date"], "2026-09-22 22:47:40+00:00");
    }

    #[test]
    fn attach_zone_leaves_a_local_mean_time_offset_bare() {
        // Toronto's local mean time before 1895 is -05:17:32, which the wire
        // spelling cannot carry to the second.
        assert_eq!(in_toronto("1880-01-01 12:00:00"), "1880-01-01 12:00:00");
    }
}
