//! Hand-written domain types used in place of `String` in selected
//! generated request and response fields.
//!
//! Types here are wired into `src/generated.rs` by `xtask` through the
//! field-name override table in `xtask/src/field_overrides.rs`.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::de::{Deserializer, Error as DeError, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A VoIP.ms routing target encoded on the wire as `tag:payload`.
///
/// VoIP.ms uses this `tag:payload` scheme across all routing-like fields
/// (`routing`, `failover_busy`, `routing_match`, the `fail_over_routing_*`
/// family, …). Documented tags are mapped to named variants; anything else
/// is preserved verbatim in [`Routing::Unknown`] so that VoIP.ms adding a
/// new tag does not break deserialization or round-tripping.
///
/// `none:` (no routing) is represented by [`Routing::None`].
///
/// # Wire format
///
/// * `account:100001_VoIP` → [`Routing::Account`]
/// * `fwd:15555` → [`Routing::Forward`]
/// * `vm:101` → [`Routing::Voicemail`]
/// * `sip:user@host` → [`Routing::Sip`]
/// * `sys:5` → [`Routing::System`]
/// * `grp:42` → [`Routing::Group`]
/// * `queue:7` → [`Routing::Queue`]
/// * `ivr:3` → [`Routing::Ivr`]
/// * `cb:2359` → [`Routing::Callback`]
/// * `tc:11` → [`Routing::TimeCondition`]
/// * `disa:1` → [`Routing::Disa`]
/// * `did:5551234567` → [`Routing::Did`]
/// * `phone:5551234567` → [`Routing::Phone`]
/// * `none:` → [`Routing::None`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routing {
    /// No routing (wire: `none:`).
    None,
    /// Sub-account by name (wire: `account:NAME`).
    Account(String),
    /// Forwarding entry by id (wire: `fwd:ID`).
    Forward(String),
    /// Voicemail box (wire: `vm:MAILBOX`).
    Voicemail(String),
    /// External SIP URI (wire: `sip:user@host`).
    Sip(String),
    /// System recording / system action (wire: `sys:ID`).
    System(String),
    /// Ring group by id (wire: `grp:ID`).
    Group(String),
    /// Queue by id (wire: `queue:ID`).
    Queue(String),
    /// IVR menu by id (wire: `ivr:ID`).
    Ivr(String),
    /// Callback entry by id (wire: `cb:ID`).
    Callback(String),
    /// Time condition by id (wire: `tc:ID`).
    TimeCondition(String),
    /// DISA entry by id (wire: `disa:ID`).
    Disa(String),
    /// DID number (wire: `did:NUMBER`).
    Did(String),
    /// Outbound phone number (wire: `phone:NUMBER`).
    Phone(String),
    /// Any tag this crate doesn't recognize. The original wire form is
    /// preserved as `tag:value` so it round-trips unchanged.
    Unknown { tag: String, value: String },
}

impl Routing {
    /// The wire tag (the substring before the `:`).
    pub fn tag(&self) -> &str {
        match self {
            Routing::None => "none",
            Routing::Account(_) => "account",
            Routing::Forward(_) => "fwd",
            Routing::Voicemail(_) => "vm",
            Routing::Sip(_) => "sip",
            Routing::System(_) => "sys",
            Routing::Group(_) => "grp",
            Routing::Queue(_) => "queue",
            Routing::Ivr(_) => "ivr",
            Routing::Callback(_) => "cb",
            Routing::TimeCondition(_) => "tc",
            Routing::Disa(_) => "disa",
            Routing::Did(_) => "did",
            Routing::Phone(_) => "phone",
            Routing::Unknown { tag, .. } => tag,
        }
    }

    /// The wire payload (the substring after the `:`).
    pub fn value(&self) -> &str {
        match self {
            Routing::None => "",
            Routing::Account(v)
            | Routing::Forward(v)
            | Routing::Voicemail(v)
            | Routing::Sip(v)
            | Routing::System(v)
            | Routing::Group(v)
            | Routing::Queue(v)
            | Routing::Ivr(v)
            | Routing::Callback(v)
            | Routing::TimeCondition(v)
            | Routing::Disa(v)
            | Routing::Did(v)
            | Routing::Phone(v) => v,
            Routing::Unknown { value, .. } => value,
        }
    }
}

/// The one place a [`Routing`]'s `tag:payload` text is written.
///
/// It is a separate type rather than [`Routing`]'s own `Display` so that
/// `Display` can be changed -- made friendlier for a log, say -- without
/// changing what every routing field sends. Being a `Display` itself is what
/// lets [`Serialize`] stream it through `collect_str` with no intermediate
/// `String`.
struct Wire<'a>(&'a Routing);

impl fmt::Display for Wire<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.0.tag(), self.0.value())
    }
}

impl Routing {
    /// The `tag:payload` string VoIP.ms carries, the inverse of
    /// [`Routing::from_str`].
    ///
    /// This is the wire form, stated once and separately from [`Display`]. The
    /// two render the same text today, and the separation is what lets that stop
    /// being true without the wire form following along.
    ///
    /// [`Display`]: std::fmt::Display
    /// [`Routing::from_str`]: std::str::FromStr::from_str
    pub fn to_wire(&self) -> String {
        Wire(self).to_string()
    }
}

/// Renders the wire form ([`Routing::to_wire`]), which is what a reader of a
/// routing field expects to see. Nothing on the wire depends on this impl.
impl fmt::Display for Routing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Wire(self).fmt(f)
    }
}

/// Parse a `tag:value` string into a [`Routing`].
///
/// An empty string is rejected; use [`Option::None`] in the surrounding
/// struct to represent an absent value.
impl FromStr for Routing {
    type Err = RoutingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (tag, value) = s.split_once(':').ok_or(RoutingParseError::MissingColon)?;

        Ok(match tag {
            "none" => Routing::None,
            "account" => Routing::Account(value.into()),
            "fwd" => Routing::Forward(value.into()),
            "vm" => Routing::Voicemail(value.into()),
            "sip" => Routing::Sip(value.into()),
            "sys" => Routing::System(value.into()),
            "grp" => Routing::Group(value.into()),
            "queue" => Routing::Queue(value.into()),
            "ivr" => Routing::Ivr(value.into()),
            "cb" => Routing::Callback(value.into()),
            "tc" => Routing::TimeCondition(value.into()),
            "disa" => Routing::Disa(value.into()),
            "did" => Routing::Did(value.into()),
            "phone" => Routing::Phone(value.into()),
            other => Routing::Unknown {
                tag: other.to_string(),
                value: value.to_string(),
            },
        })
    }
}

/// Error from parsing a [`Routing`] from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingParseError {
    /// The input contained no `:` separator.
    MissingColon,
}

impl fmt::Display for RoutingParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoutingParseError::MissingColon => {
                f.write_str("routing string is missing required `:` separator")
            }
        }
    }
}

impl std::error::Error for RoutingParseError {}

impl Serialize for Routing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&Wire(self))
    }
}

impl<'de> Deserialize<'de> for Routing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RoutingVisitor;

        impl<'de> Visitor<'de> for RoutingVisitor {
            type Value = Routing;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a VoIP.ms routing string of the form `tag:value`")
            }

            fn visit_str<E>(self, v: &str) -> Result<Routing, E>
            where
                E: DeError,
            {
                Routing::from_str(v).map_err(E::custom)
            }

            fn visit_string<E>(self, v: String) -> Result<Routing, E>
            where
                E: DeError,
            {
                Routing::from_str(&v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(RoutingVisitor)
    }
}

/// A duration in seconds, or an "unbounded" sentinel.
///
/// Several VoIP.ms queue/announcement fields take a number of seconds *or* a
/// word meaning no limit (`none` / `unlimited`), so a bare `u64` can't hold the
/// sentinel. [`Seconds`] serializes the sentinel as `none`; [`WaitTime`] as
/// `unlimited` (the word `maximum_wait_time` documents). Both deserialize
/// tolerantly: a number, a numeric string, or either sentinel word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seconds {
    /// A concrete number of seconds.
    Value(u64),
    /// No limit (wire: `none`).
    Unlimited,
}

/// A wait time in seconds, or unlimited.
///
/// Like [`Seconds`] but serializes the unbounded case as `unlimited`, the word
/// `maximum_wait_time` documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitTime {
    /// A concrete number of seconds.
    Value(u64),
    /// No limit (wire: `unlimited`).
    Unlimited,
}

/// A conference member cap, or unlimited.
///
/// `getConference` reports `max_members` as a count or the word `Unlimited`
/// when the conference has no cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxMembers {
    /// A concrete member cap.
    Value(u64),
    /// No cap (wire: `Unlimited`).
    Unlimited,
}

macro_rules! impl_seconds {
    ($name:ident, $unlimited_wire:literal, $expecting:literal) => {
        impl $name {
            /// The count, or `None` for the unbounded sentinel.
            pub fn as_u64(&self) -> Option<u64> {
                match self {
                    $name::Value(v) => Some(*v),
                    $name::Unlimited => None,
                }
            }
        }

        impl From<u64> for $name {
            fn from(v: u64) -> Self {
                $name::Value(v)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self {
                    $name::Value(v) => serializer.serialize_str(&v.to_string()),
                    $name::Unlimited => serializer.serialize_str($unlimited_wire),
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct SecondsVisitor;

                impl<'de> Visitor<'de> for SecondsVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str($expecting)
                    }

                    fn visit_u64<E>(self, v: u64) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        Ok($name::Value(v))
                    }

                    fn visit_str<E>(self, v: &str) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        let t = v.trim();
                        match t.to_ascii_lowercase().as_str() {
                            "none" | "unlimited" => Ok($name::Unlimited),
                            _ => t
                                .parse::<u64>()
                                .map($name::Value)
                                .map_err(|_| E::custom(format!("invalid seconds value {v}"))),
                        }
                    }
                }

                deserializer.deserialize_any(SecondsVisitor)
            }
        }
    };
}

impl_seconds!(Seconds, "none", "a number of seconds or `none`");
impl_seconds!(WaitTime, "unlimited", "a number of seconds or `unlimited`");
impl_seconds!(MaxMembers, "Unlimited", "a member count or `Unlimited`");

/// The number of hours the record-listing `timezone` parameter carries.
///
/// The `getCDR`, `getResellerCDR`, `getSMS`, `getMMS`, `getResellerSMS`, and
/// `getResellerMMS` methods take a `timezone` parameter that VoIP.ms documents
/// as "adjust the times of the records according to Timezone (Numeric: -12 to
/// 13)". It is distinct from the `getTimezones` / voicemail `timezone`, which is
/// a named zone (`America/New_York`).
///
/// **It is not a UTC offset during daylight saving time.** VoIP.ms records its
/// timestamps as wall clocks in [`SERVER_ZONE`], and a request carrying `n`
/// reports each one shifted by `n + 5` hours, as if that zone were always
/// UTC-05:00; the `date_from` / `date_to` days are matched on the shifted
/// value. During DST that is the wall clock at UTC+`n+1`. Measured against the
/// live API on 2026-09-23 with `n` of `-12`, `-5`, `-4`, `0`, `5.5` and `13`.
///
/// Outside DST the same fixed shift gives UTC+`n`. That is inferred from the
/// September readings, not measured: every record on the account measured was
/// made during DST, and a read made in winter could apply a different base.
///
/// Callers hold a [`chrono_tz::Tz`] on those methods' `timezone` field, and the
/// crate sends [`TimezoneOffset::for_window`] of it at the query's start date
/// (or its end date when there is no start), the number whose shifted days are
/// that zone's days. The reported timestamps are qualified by undoing the shift
/// and resolving each one in [`SERVER_ZONE`], so they name their instant
/// whatever was sent.
///
/// Wraps a [`Decimal`] constrained to `-12..=13`. [`TimezoneOffset::new`]
/// rejects out-of-range values so a nonsensical number never reaches the wire.
///
/// # Wire format
///
/// Serializes as a bare number (`-5`, `5.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimezoneOffset(Decimal);

impl TimezoneOffset {
    /// The inclusive range of numbers VoIP.ms accepts, in hours.
    const MIN: i64 = -12;
    const MAX: i64 = 13;

    /// Zero, sent as `0`. Outside DST it reports UTC wall clocks; during DST,
    /// UTC+01:00 ones.
    pub const UTC: Self = Self(Decimal::ZERO);

    /// The fixed offset VoIP.ms's shift assumes for [`SERVER_ZONE`], in hours.
    const SHIFT_BASE: i64 = 5;

    /// Construct an offset, rejecting a value outside `-12..=13` hours.
    pub fn new(hours: impl Into<Decimal>) -> Result<Self, TimezoneOffsetError> {
        let hours = hours.into();
        if hours < Decimal::from(Self::MIN) || hours > Decimal::from(Self::MAX) {
            return Err(TimezoneOffsetError::OutOfRange(hours));
        }

        Ok(Self(hours))
    }

    /// The UTC offset of `tz` at local noon on `date`, in hours.
    ///
    /// Noon sits clear of the midnight DST fold. A zone whose offset exceeds
    /// `-12..=13` (e.g. `Pacific/Kiritimati`, +14) returns
    /// [`TimezoneOffsetError::OutOfRange`]. This is the zone's offset, not the
    /// number to send; see [`TimezoneOffset::for_window`].
    pub fn at(tz: chrono_tz::Tz, date: chrono::NaiveDate) -> Result<Self, TimezoneOffsetError> {
        Self::new(Self::offset_at_noon(tz, date)?)
    }

    /// The number to send so that a query starting on `date` reports `tz`'s
    /// days: `tz`'s UTC offset minus [`SERVER_ZONE`]'s, less five hours, both
    /// taken at local noon in `tz`.
    ///
    /// VoIP.ms applies one number to the whole range, so a range that crosses a
    /// DST change in either zone is out by an hour from that change on.
    ///
    /// The result must fall in `-12..=13`, or this returns
    /// [`TimezoneOffsetError::OutOfRange`] rather than a window an hour off.
    /// During Eastern DST a UTC-12 zone (`Etc/GMT+12`) needs `-13` and is
    /// refused, where `Pacific/Kiritimati` (+14) needs `13` and fits; outside
    /// DST it is the other way round.
    pub fn for_window(
        tz: chrono_tz::Tz,
        date: chrono::NaiveDate,
    ) -> Result<Self, TimezoneOffsetError> {
        use chrono::Offset;

        let noon = Self::noon_in(tz, date)?;
        let server = noon
            .with_timezone(&SERVER_ZONE)
            .offset()
            .fix()
            .local_minus_utc();
        let caller = noon.offset().fix().local_minus_utc();
        // A request carrying `n` reports `UTC + server + n + 5`; `n` is chosen
        // so that equals `UTC + caller`.
        let hours =
            Decimal::from(caller - server) / Decimal::from(3600) - Decimal::from(Self::SHIFT_BASE);
        Self::new(hours)
    }

    /// Local noon on `date` in `tz`.
    fn noon_in(
        tz: chrono_tz::Tz,
        date: chrono::NaiveDate,
    ) -> Result<chrono::DateTime<chrono_tz::Tz>, TimezoneOffsetError> {
        use chrono::TimeZone;

        let noon = date
            .and_hms_opt(12, 0, 0)
            .expect("12:00:00 is a valid time");
        tz.from_local_datetime(&noon)
            .earliest()
            .ok_or(TimezoneOffsetError::UnresolvableInstant)
    }

    /// `tz`'s UTC offset at local noon on `date`, in hours. Whole where the
    /// offset is on the hour; a fractional zone (India, +5:30) keeps the
    /// remainder.
    fn offset_at_noon(
        tz: chrono_tz::Tz,
        date: chrono::NaiveDate,
    ) -> Result<Decimal, TimezoneOffsetError> {
        use chrono::Offset;

        let seconds = Self::noon_in(tz, date)?.offset().fix().local_minus_utc();
        Ok(Decimal::from(seconds) / Decimal::from(3600))
    }

    /// How far VoIP.ms moves a [`SERVER_ZONE`] wall clock for a request
    /// carrying this number: `n + 5` hours, in seconds.
    pub(crate) fn shift_seconds(&self) -> i64 {
        use rust_decimal::prelude::ToPrimitive;

        ((self.0 + Decimal::from(Self::SHIFT_BASE)) * Decimal::from(3600))
            .round()
            .to_i64()
            .expect("a number within -12..=13 hours shifts by a whole number of seconds")
    }

    /// The number, in hours.
    pub fn hours(&self) -> Decimal {
        self.0
    }
}

/// The zone VoIP.ms records its timestamps in: US/Canada Eastern, observing DST.
///
/// Measured against the live API with an independent reference instant for
/// each: an SMS, a call, a call recording (whose id embeds a Unix timestamp), a
/// DID order, a fax, an MMS and a SIP registration. Every one reported the
/// Eastern wall clock at UTC-04:00 in September. VoIP.ms documents its servers
/// as Eastern time following the US and Canadian DST rules.
pub const SERVER_ZONE: chrono_tz::Tz = chrono_tz::America::Toronto;

/// Where a method's response timestamps get the named zone they are rendered
/// in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampZone {
    /// [`SERVER_ZONE`].
    Server,
    /// A zone the response does not name and the caller must supply, such as a
    /// voicemail box's own `timezone`.
    Supplied,
}

impl TimestampZone {
    /// The zone, or `None` when the caller has to supply it.
    pub fn zone(&self) -> Option<chrono_tz::Tz> {
        match self {
            TimestampZone::Server => Some(SERVER_ZONE),
            TimestampZone::Supplied => None,
        }
    }
}

/// A method's response timestamps rendered in a named zone, and the paths that
/// reach them in the form [`attach_zone`](crate::attach_zone) takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneTimestamps {
    pub zone: TimestampZone,
    pub paths: &'static [&'static str],
}

impl fmt::Display for TimezoneOffset {
    /// Renders the number as sent (`-5`, `5.5`). It is not labeled as a UTC
    /// offset, since during DST it is not one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.normalize().fmt(f)
    }
}

/// Error from constructing a [`TimezoneOffset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimezoneOffsetError {
    /// The number fell outside the `-12..=13` range VoIP.ms accepts. Returned
    /// by [`TimezoneOffset::new`], by [`TimezoneOffset::at`] for a zone whose
    /// offset exceeds that range (e.g. `Pacific/Kiritimati`, +14), and by
    /// [`TimezoneOffset::for_window`] for a zone whose window needs a number
    /// outside it (e.g. a UTC-12 zone during Eastern DST).
    OutOfRange(Decimal),
    /// The chosen instant does not exist in the zone (a DST spring-forward
    /// gap), so no offset could be resolved.
    UnresolvableInstant,
    /// A `timezone` zone was given without a query date to resolve its window
    /// at: neither the start date (`date_from` / `from`) nor the end date
    /// (`date_to` / `to`).
    MissingQueryDate,
    /// A query date string (`from` / `to`) did not parse as a `YYYY-MM-DD`
    /// date.
    InvalidQueryDate,
}

impl fmt::Display for TimezoneOffsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimezoneOffsetError::OutOfRange(v) => {
                write!(f, "timezone {v} is outside the range -12 to 13")
            }

            TimezoneOffsetError::UnresolvableInstant => {
                f.write_str("timezone offset could not be resolved at the given date")
            }

            TimezoneOffsetError::MissingQueryDate => f.write_str(
                "timezone requires a query date (date_from / from or date_to / to) to resolve \
                 its window",
            ),
            TimezoneOffsetError::InvalidQueryDate => {
                f.write_str("query date is not a YYYY-MM-DD date")
            }
        }
    }
}

impl std::error::Error for TimezoneOffsetError {}

impl Serialize for TimezoneOffset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

/// A named time zone as VoIP.ms reports it: a parsed [`chrono_tz::Tz`] when
/// the bundled IANA database recognizes the name, or the verbatim wire string
/// when it does not.
///
/// VoIP.ms's `getTimezones` reference catalog still lists a handful of legacy
/// names the IANA database has since dropped (`Asia/Beijing`,
/// `US/Pacific-New`, `Factory`, ...), and a long-lived mailbox may carry one;
/// preserving them beats failing the whole response. Parsing never fails --
/// an unrecognized name lands in [`TimezoneName::Unrecognized`] and
/// round-trips unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimezoneName {
    /// A zone the bundled IANA database recognizes.
    Known(chrono_tz::Tz),
    /// A name it does not recognize, preserved verbatim.
    Unrecognized(String),
}

impl TimezoneName {
    /// The recognized zone, or `None` for a legacy name.
    pub fn tz(&self) -> Option<chrono_tz::Tz> {
        match self {
            TimezoneName::Known(tz) => Some(*tz),
            TimezoneName::Unrecognized(_) => None,
        }
    }

    /// The zone name as VoIP.ms spells it.
    pub fn name(&self) -> &str {
        match self {
            TimezoneName::Known(tz) => tz.name(),
            TimezoneName::Unrecognized(s) => s,
        }
    }
}

impl fmt::Display for TimezoneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for TimezoneName {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<chrono_tz::Tz>()
            .map(TimezoneName::Known)
            .unwrap_or_else(|_| TimezoneName::Unrecognized(s.to_string())))
    }
}

impl From<chrono_tz::Tz> for TimezoneName {
    fn from(tz: chrono_tz::Tz) -> Self {
        TimezoneName::Known(tz)
    }
}

impl<'de> Deserialize<'de> for TimezoneName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NameVisitor;

        impl<'de> Visitor<'de> for NameVisitor {
            type Value = TimezoneName;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an IANA time zone name")
            }

            fn visit_str<E>(self, v: &str) -> Result<TimezoneName, E>
            where
                E: DeError,
            {
                let Ok(name) = TimezoneName::from_str(v);
                Ok(name)
            }
        }

        deserializer.deserialize_str(NameVisitor)
    }
}

/// What VoIP.ms reported for a field: the parsed value, or the text it sent
/// when that text does not parse.
///
/// Response dates wear this. A `*Response` is one value built from one
/// envelope, so a deserializer that fails on a single field fails the whole
/// read -- one unreadable date costs every record beside it, and the caller
/// gets an error instead of the rows that were fine. That is the shape of the
/// break this crate has now paid for twice, in legacy zone names and in a
/// transaction-history window.
///
/// [`Reported::Unreadable`] keeps the value instead of discarding it, which
/// answering `None` would do. The difference matters twice over: the caller can
/// still see and salvage what arrived, and "unreadable" stays distinguishable
/// from "absent", so the live drift harness can still tell that VoIP.ms sent
/// something this crate does not model.
///
/// Absence is the surrounding [`Option`], not a variant here: a field VoIP.ms
/// omits is `None`, and so is one carrying a blank or a `0000-00-00`
/// placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reported<T> {
    /// The value, as this crate reads it.
    Parsed(T),
    /// The text VoIP.ms sent, kept because it does not parse.
    Unreadable(String),
}

impl<T> Reported<T> {
    /// The value, or `None` when VoIP.ms sent something unreadable.
    pub fn value(&self) -> Option<&T> {
        match self {
            Reported::Parsed(v) => Some(v),
            Reported::Unreadable(_) => None,
        }
    }

    /// The value, consuming the wrapper.
    pub fn into_value(self) -> Option<T> {
        match self {
            Reported::Parsed(v) => Some(v),
            Reported::Unreadable(_) => None,
        }
    }

    /// The text VoIP.ms sent, when it could not be read.
    pub fn unreadable(&self) -> Option<&str> {
        match self {
            Reported::Parsed(_) => None,
            Reported::Unreadable(s) => Some(s),
        }
    }
}

impl<T: Copy> Reported<T> {
    /// The value, copied out of the wrapper. The shorthand for the [`chrono`]
    /// types these fields hold, which are all [`Copy`].
    pub fn get(&self) -> Option<T> {
        self.value().copied()
    }
}

impl<T: fmt::Display> fmt::Display for Reported<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reported::Parsed(v) => v.fmt(f),
            Reported::Unreadable(s) => f.write_str(s),
        }
    }
}

/// A wall clock reported without an offset, rendered in a named zone the
/// response does not name.
///
/// [`WallClock::Zoned`] when that zone is known: the instant, carrying the
/// offset the zone was at then. [`WallClock::Bare`] when it is not, and also
/// when the wall clock is ambiguous in the zone (the repeated hour when clocks
/// fall back) or does not exist in it (the hour skipped when they spring
/// forward), since either offset would be a guess.
///
/// Two values are equal only when they report the same wall clock with the same
/// offset. `DateTime`'s own equality compares instants alone, so
/// `18:47:40-04:00` and `22:47:40+00:00` would otherwise compare equal while
/// reporting different wall clocks.
///
/// [`Display`](std::fmt::Display) renders the wire spelling, with the offset
/// appended for a zoned value (`2026-09-22 18:47:40-04:00`). A named-zone field
/// reads either form back as the same value. The offset is written to the
/// minute, which is all the wire carries.
#[derive(Debug, Clone, Copy)]
pub enum WallClock {
    /// The instant, with the UTC offset in force in the zone at that moment.
    Zoned(DateTime<FixedOffset>),
    /// The wall clock in the zone it was rendered in, with no offset.
    Bare(NaiveDateTime),
}

impl PartialEq for WallClock {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WallClock::Zoned(a), WallClock::Zoned(b)) => a == b && a.offset() == b.offset(),
            (WallClock::Bare(a), WallClock::Bare(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for WallClock {}

impl WallClock {
    /// The wall clock in the value's zone: the zone a zoned value was resolved
    /// in, or the one a bare value was rendered in. For a record-listing
    /// timestamp that is [`SERVER_ZONE`] either way, not the shifted wall clock
    /// VoIP.ms reported.
    pub fn local(&self) -> NaiveDateTime {
        match self {
            WallClock::Zoned(at) => at.naive_local(),
            WallClock::Bare(at) => *at,
        }
    }

    /// The instant, or `None` for a wall clock no offset was attached to.
    pub fn zoned(&self) -> Option<DateTime<FixedOffset>> {
        match self {
            WallClock::Zoned(at) => Some(*at),
            WallClock::Bare(_) => None,
        }
    }
}

impl fmt::Display for WallClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WallClock::Zoned(at) => write!(
                f,
                "{}",
                at.format(crate::responses::OFFSET_DATETIME_WIRE_FORMAT)
            ),
            WallClock::Bare(at) => {
                write!(f, "{}", at.format(crate::responses::DATETIME_WIRE_FORMAT))
            }
        }
    }
}

/// The `date` a `getTransactionHistory` row carries: when it posted, or the
/// window it summarizes.
///
/// Most rows name a point in time. The report also ends with a synthesized row
/// per usage-metered charge, totaling it over the range the call asked for, and
/// that row puts the range itself in the same field
/// (`2026-08-01 to 2026-08-31`), which no single [`chrono`] type holds. Such a
/// row has no transaction to name and reports `uniqueid` as the literal `n/a`.
/// Parsing never fails: a form no variant covers lands in
/// [`TransactionDate::Unrecognized`] and round-trips unchanged, so one
/// unreadable row cannot fail the whole response.
///
/// [`Display`](std::fmt::Display) renders every variant in the canonical wire
/// spelling, which is what VoIP.ms sends but not always the bytes that arrived:
/// parsing trims, [`chrono`] accepts unpadded components it renders padded, and
/// a window bounded by timestamps keeps only the days. Only
/// [`TransactionDate::Unrecognized`] holds the value as received, trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionDate {
    /// The instant the row posted (wire: `2016-06-03 00:03:46`).
    At(NaiveDateTime),
    /// The day the row posted, with no time of day (wire: `2010-10-29`). Kept
    /// distinct from [`TransactionDate::At`] rather than read as midnight,
    /// which would invent a time of day and render it back with one.
    On(NaiveDate),
    /// The window an aggregate row summarizes (wire: `<from> to <to>`).
    Period { from: NaiveDate, to: NaiveDate },
    /// A form no other variant covers, preserved verbatim.
    Unrecognized(String),
}

/// Separates the two dates of a [`TransactionDate::Period`] on the wire.
const PERIOD_SEPARATOR: &str = " to ";

/// One bound of a [`TransactionDate::Period`]. A bound may arrive as a bare
/// date or as a timestamp; only the day bounds the window, so a timestamp
/// contributes its date and the time of day is dropped.
fn period_bound(s: &str) -> Option<NaiveDate> {
    let trimmed = s.trim();
    trimmed.parse::<NaiveDate>().ok().or_else(|| {
        NaiveDateTime::parse_from_str(trimmed, crate::responses::DATETIME_WIRE_FORMAT)
            .ok()
            .map(|at| at.date())
    })
}

impl TransactionDate {
    /// The instant, or `None` unless the row reported one to the second.
    pub fn at(&self) -> Option<NaiveDateTime> {
        match self {
            TransactionDate::At(at) => Some(*at),
            _ => None,
        }
    }

    /// The calendar day the row posted, whether it named a time of day or not.
    /// `None` for a span, which covers many days, and for an unrecognized
    /// value.
    pub fn date(&self) -> Option<NaiveDate> {
        match self {
            TransactionDate::At(at) => Some(at.date()),
            TransactionDate::On(on) => Some(*on),
            _ => None,
        }
    }

    /// The span as `(from, to)`, or `None` when the row names a point in time.
    pub fn period(&self) -> Option<(NaiveDate, NaiveDate)> {
        match self {
            TransactionDate::Period { from, to } => Some((*from, *to)),
            _ => None,
        }
    }
}

impl fmt::Display for TransactionDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionDate::At(at) => {
                write!(f, "{}", at.format(crate::responses::DATETIME_WIRE_FORMAT))
            }
            TransactionDate::On(on) => write!(f, "{on}"),
            TransactionDate::Period { from, to } => write!(f, "{from}{PERIOD_SEPARATOR}{to}"),
            TransactionDate::Unrecognized(s) => f.write_str(s),
        }
    }
}

impl FromStr for TransactionDate {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((from, to)) = s.split_once(PERIOD_SEPARATOR) {
            return Ok(match (period_bound(from), period_bound(to)) {
                // A window runs forward. A reversed pair is not one, so it is
                // left unread rather than handing a caller a negative width
                // with nothing to signal it.
                (Some(from), Some(to)) if from <= to => TransactionDate::Period { from, to },
                _ => TransactionDate::Unrecognized(s.to_string()),
            });
        }

        if let Ok(at) = NaiveDateTime::parse_from_str(s, crate::responses::DATETIME_WIRE_FORMAT) {
            return Ok(TransactionDate::At(at));
        }

        Ok(match s.parse::<NaiveDate>() {
            Ok(on) => TransactionDate::On(on),
            Err(_) => TransactionDate::Unrecognized(s.to_string()),
        })
    }
}

impl<'de> Deserialize<'de> for TransactionDate {
    /// Reads the same wire forms the response field does: the text trimmed, and
    /// a bare number or bool taken as its text rather than rejected.
    ///
    /// One difference is inherent and not a disagreement: the field is
    /// `Option<TransactionDate>`, so absence -- an empty value or a zero-date
    /// placeholder -- is its `None`. A bare `TransactionDate` has no absent
    /// form, so it keeps those in [`TransactionDate::Unrecognized`] rather than
    /// inventing one.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DateVisitor;

        impl DateVisitor {
            fn parse<E: DeError>(text: &str) -> Result<TransactionDate, E> {
                let Ok(date) = text.trim().parse::<TransactionDate>();
                Ok(date)
            }
        }

        impl<'de> Visitor<'de> for DateVisitor {
            type Value = TransactionDate;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a transaction date: a timestamp, a date, or `<from> to <to>`")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<TransactionDate, E> {
                Self::parse(v)
            }

            fn visit_i64<E: DeError>(self, v: i64) -> Result<TransactionDate, E> {
                Self::parse(&v.to_string())
            }

            fn visit_u64<E: DeError>(self, v: u64) -> Result<TransactionDate, E> {
                Self::parse(&v.to_string())
            }

            fn visit_f64<E: DeError>(self, v: f64) -> Result<TransactionDate, E> {
                Self::parse(&v.to_string())
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<TransactionDate, E> {
                Self::parse(&v.to_string())
            }
        }

        deserializer.deserialize_any(DateVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_tags() {
        assert_eq!(Routing::from_str("none:").unwrap(), Routing::None);
        assert_eq!(
            Routing::from_str("account:100001_VoIP").unwrap(),
            Routing::Account("100001_VoIP".into()),
        );
        assert_eq!(
            Routing::from_str("fwd:15555").unwrap(),
            Routing::Forward("15555".into()),
        );
        assert_eq!(
            Routing::from_str("vm:101").unwrap(),
            Routing::Voicemail("101".into()),
        );
        assert_eq!(
            Routing::from_str("cb:2359").unwrap(),
            Routing::Callback("2359".into()),
        );
    }

    #[test]
    fn preserves_unknown_tags() {
        let r = Routing::from_str("future:abc").unwrap();
        assert_eq!(
            r,
            Routing::Unknown {
                tag: "future".into(),
                value: "abc".into(),
            },
        );
        assert_eq!(r.to_string(), "future:abc");
    }

    #[test]
    fn sip_value_can_contain_colons() {
        // Split is on the FIRST colon so sip URIs survive intact.
        let r = Routing::from_str("sip:5552223333@sip.voip.ms:5060").unwrap();
        assert_eq!(r, Routing::Sip("5552223333@sip.voip.ms:5060".into()));
        assert_eq!(r.to_wire(), "sip:5552223333@sip.voip.ms:5060");
    }

    #[test]
    fn the_wire_form_is_what_serde_sends_and_from_str_accepts() {
        // `to_wire` is the contract, not `Display`: the two agree today, and
        // this is what has to keep holding if one of them changes.
        for r in [
            Routing::None,
            Routing::Account("100001_VoIP".into()),
            Routing::Sip("5552223333@sip.voip.ms:5060".into()),
            Routing::Unknown {
                tag: "future".into(),
                value: "abc".into(),
            },
        ] {
            let wire = r.to_wire();
            assert_eq!(
                Routing::from_str(&wire).unwrap(),
                r,
                "the wire form must parse back to the value that produced it"
            );
            assert_eq!(
                serde_json::to_string(&r).unwrap(),
                serde_json::to_string(&wire).unwrap(),
                "serde sends the wire form"
            );
            // They agree today. Changing `Display` is then a deliberate edit
            // here, not something a reader of the doc has to take on faith.
            assert_eq!(r.to_string(), wire, "Display renders the wire form today");
        }
    }

    #[test]
    fn rejects_missing_colon() {
        assert_eq!(
            Routing::from_str("nocolon"),
            Err(RoutingParseError::MissingColon),
        );
    }

    #[test]
    fn round_trips_through_serde() {
        let r = Routing::Forward("19998887777".into());
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"fwd:19998887777\"");
        let back: Routing = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn deserialize_none() {
        let r: Routing = serde_json::from_str("\"none:\"").unwrap();
        assert_eq!(r, Routing::None);
    }

    #[test]
    fn seconds_serializes_value_and_sentinel() {
        assert_eq!(
            serde_json::to_string(&Seconds::Value(30)).unwrap(),
            "\"30\""
        );
        assert_eq!(
            serde_json::to_string(&Seconds::Unlimited).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&WaitTime::Unlimited).unwrap(),
            "\"unlimited\""
        );
    }

    #[test]
    fn seconds_deserializes_number_string_and_sentinels() {
        // A bare number, a numeric string, and either sentinel word all parse.
        assert_eq!(
            serde_json::from_str::<Seconds>("45").unwrap(),
            Seconds::Value(45)
        );
        assert_eq!(
            serde_json::from_str::<Seconds>("\"45\"").unwrap(),
            Seconds::Value(45)
        );
        for s in ["\"none\"", "\"NONE\"", "\"unlimited\""] {
            assert_eq!(
                serde_json::from_str::<Seconds>(s).unwrap(),
                Seconds::Unlimited,
                "{s}"
            );
        }
        // WaitTime shares the tolerant parse.
        assert_eq!(
            serde_json::from_str::<WaitTime>("\"unlimited\"").unwrap(),
            WaitTime::Unlimited
        );
    }

    /// Reading the count is the common thing to do with one of these, and a
    /// caller should not have to match to do it.
    #[test]
    fn seconds_like_types_expose_their_count() {
        assert_eq!(Seconds::Value(30).as_u64(), Some(30));
        assert_eq!(Seconds::Unlimited.as_u64(), None);
        assert_eq!(WaitTime::Value(45).as_u64(), Some(45));
        assert_eq!(WaitTime::Unlimited.as_u64(), None);
        assert_eq!(MaxMembers::Value(40).as_u64(), Some(40));
        assert_eq!(MaxMembers::Unlimited.as_u64(), None);
    }

    #[test]
    fn max_members_handles_count_and_capital_unlimited() {
        // The wire sends a numeric string or the capitalized word `Unlimited`.
        assert_eq!(
            serde_json::from_str::<MaxMembers>("\"40\"").unwrap(),
            MaxMembers::Value(40)
        );
        assert_eq!(
            serde_json::from_str::<MaxMembers>("\"Unlimited\"").unwrap(),
            MaxMembers::Unlimited
        );
        // Serialize round-trips the exact wire form, capital `U` included.
        assert_eq!(
            serde_json::to_string(&MaxMembers::Unlimited).unwrap(),
            "\"Unlimited\""
        );
        assert_eq!(
            serde_json::to_string(&MaxMembers::Value(40)).unwrap(),
            "\"40\""
        );
    }

    #[test]
    fn timezone_offset_rejects_out_of_range() {
        assert!(TimezoneOffset::new(-12).is_ok());
        assert!(TimezoneOffset::new(13).is_ok());
        assert_eq!(
            TimezoneOffset::new(14),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(14)))
        );
        assert_eq!(
            TimezoneOffset::new(-13),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(-13)))
        );
    }

    #[test]
    fn timezone_offset_serializes_as_bare_number() {
        assert_eq!(
            serde_json::to_string(&TimezoneOffset::new(-5).unwrap()).unwrap(),
            "\"-5\""
        );
    }

    /// `Display` is the number, not a `UTC±HH:MM` label, which during DST would
    /// name a zone the timestamps are not in.
    #[test]
    fn timezone_offset_displays_the_number() {
        assert_eq!(TimezoneOffset::new(-5).unwrap().to_string(), "-5");
        assert_eq!(TimezoneOffset::new(13).unwrap().to_string(), "13");
        assert_eq!(TimezoneOffset::UTC.to_string(), "0");
        // `for_window` computes Kolkata in July as 5.5 + 4 - 5 = 4.50; the
        // trailing zero is scale, not a different number.
        let kolkata = TimezoneOffset::for_window(
            chrono_tz::Asia::Kolkata,
            chrono::NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
        )
        .unwrap();
        assert_eq!(kolkata.to_string(), "4.5");
    }

    #[test]
    fn timezone_offset_at_resolves_dst() {
        use chrono::NaiveDate;
        let jan = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let jul = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        // America/New_York: EST (-5) in winter, EDT (-4) in summer.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::New_York, jan).unwrap(),
            TimezoneOffset::new(-5).unwrap()
        );
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::New_York, jul).unwrap(),
            TimezoneOffset::new(-4).unwrap()
        );
        // Arizona does not observe DST: -7 year-round.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::Phoenix, jul).unwrap(),
            TimezoneOffset::new(-7).unwrap()
        );
        // UTC is 0.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::UTC, jan).unwrap(),
            TimezoneOffset::new(0).unwrap()
        );
    }

    #[test]
    fn timezone_name_parses_known_and_preserves_legacy() {
        let known: TimezoneName = "America/New_York".parse().unwrap();
        assert_eq!(known, TimezoneName::Known(chrono_tz::America::New_York));
        assert_eq!(known.tz(), Some(chrono_tz::America::New_York));
        assert_eq!(known.name(), "America/New_York");

        // voip.ms's catalog still lists zone names the IANA database dropped;
        // they must survive verbatim instead of failing the parse.
        let legacy: TimezoneName = "Asia/Beijing".parse().unwrap();
        assert_eq!(legacy, TimezoneName::Unrecognized("Asia/Beijing".into()));
        assert_eq!(legacy.tz(), None);
        assert_eq!(legacy.name(), "Asia/Beijing");
        assert_eq!(legacy.to_string(), "Asia/Beijing");
    }

    /// Nothing writes a zone name -- only `getTimezones` and `getVoicemails`
    /// report one -- so the type reads and never serializes.
    #[test]
    fn timezone_name_deserializes_known_and_legacy() {
        for name in ["America/New_York", "US/Pacific-New"] {
            let back: TimezoneName = serde_json::from_str(&format!("\"{name}\"")).unwrap();
            assert_eq!(back, name.parse::<TimezoneName>().unwrap());
            assert_eq!(back.name(), name);
        }
    }

    /// Every form has to come back out as VoIP.ms spelled it. That round-trip
    /// is why `On` exists rather than a midnight `At`: reading a bare date as a
    /// timestamp would invent a time of day and render it back with one.
    #[test]
    fn transaction_date_round_trips_every_form() {
        for wire in [
            "2016-06-03 00:03:46",
            "2010-10-29",
            "2026-08-01 to 2026-08-31",
            "2026-08-07 to 2026-08-07",
            "whenever",
        ] {
            let Ok(parsed) = wire.parse::<TransactionDate>();
            assert_eq!(parsed.to_string(), wire);
        }
    }

    #[test]
    fn transaction_date_separates_a_point_in_time_from_a_span() {
        let day = NaiveDate::from_ymd_opt(2016, 6, 3).unwrap();
        let Ok(at) = "2016-06-03 00:03:46".parse::<TransactionDate>();
        assert_eq!(at.at(), Some(day.and_hms_opt(0, 3, 46).unwrap()));
        assert_eq!(at.date(), Some(day));
        assert_eq!(at.period(), None);

        // A date-only row has a day but no instant.
        let Ok(on) = "2010-10-29".parse::<TransactionDate>();
        assert_eq!(on.at(), None);
        assert_eq!(on.date(), NaiveDate::from_ymd_opt(2010, 10, 29));
        assert_eq!(on.period(), None);

        let Ok(period) = "2026-08-01 to 2026-09-01".parse::<TransactionDate>();
        assert_eq!(
            period.period(),
            Some((
                NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
            ))
        );
        assert_eq!(period.at(), None);
        assert_eq!(period.date(), None);
    }

    /// A half-parsed span is unrecognized rather than silently half-read: one
    /// side alone says nothing about what the row covers. A reversed one is
    /// not a window at all, so it keeps the value rather than reporting a
    /// negative width.
    #[test]
    fn transaction_date_keeps_a_malformed_span_verbatim() {
        for wire in [
            "2026-08-01 to never",
            "not-a-date to 2026-08-31",
            "2026-08-31 to 2026-08-01",
        ] {
            let Ok(parsed) = wire.parse::<TransactionDate>();
            assert_eq!(parsed, TransactionDate::Unrecognized(wire.to_string()));
        }
    }

    /// Only the day bounds a window, so a bound given to the second still
    /// reads as one rather than falling to `Unrecognized`, where `period()`
    /// would answer `None` and a caller aggregating by window would skip the
    /// row.
    #[test]
    fn transaction_date_reads_a_span_bounded_by_timestamps() {
        let Ok(parsed) = "2026-08-01 00:00:00 to 2026-08-31 23:59:59".parse::<TransactionDate>();
        assert_eq!(
            parsed.period(),
            Some((
                NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
            ))
        );
    }

    /// The type is response-only, so it carries `Deserialize` and no
    /// `Serialize` -- and a raw envelope has to be readable through it.
    #[test]
    fn transaction_date_deserializes_every_form() {
        for wire in [
            "2016-06-03 00:03:46",
            "2010-10-29",
            "2026-08-01 to 2026-08-31",
            "whenever",
        ] {
            let back: TransactionDate = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(back, wire.parse::<TransactionDate>().unwrap());
            assert_eq!(back.to_string(), wire);
        }
    }

    /// The advertised use is reading one out of a raw envelope, which is
    /// exactly where the untidy forms show up -- so the impl has to read them
    /// the way the response field does, not just the canonical four.
    #[test]
    fn transaction_date_deserializes_the_untidy_forms_too() {
        let from = |v: serde_json::Value| serde_json::from_value::<TransactionDate>(v).unwrap();
        assert_eq!(
            from(serde_json::json!(" 2010-10-29 ")),
            TransactionDate::On(NaiveDate::from_ymd_opt(2010, 10, 29).unwrap())
        );
        assert_eq!(
            from(serde_json::json!(0)),
            TransactionDate::Unrecognized("0".to_string())
        );
        assert_eq!(
            from(serde_json::json!(true)),
            TransactionDate::Unrecognized("true".to_string())
        );
    }

    #[test]
    fn reported_reads_both_variants_through_every_accessor() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let parsed = Reported::Parsed(day);
        assert_eq!(parsed.value(), Some(&day));
        assert_eq!(parsed.get(), Some(day));
        assert_eq!(parsed.unreadable(), None);
        assert_eq!(parsed.clone().into_value(), Some(day));
        assert_eq!(parsed.to_string(), "2026-10-08");

        let unreadable: Reported<NaiveDate> = Reported::Unreadable("08/10/2026".to_string());
        assert_eq!(unreadable.value(), None);
        assert_eq!(unreadable.get(), None);
        assert_eq!(unreadable.unreadable(), Some("08/10/2026"));
        assert_eq!(unreadable.clone().into_value(), None);
        // The arm the examples print through: a degraded date still renders as
        // what VoIP.ms sent rather than as nothing.
        assert_eq!(unreadable.to_string(), "08/10/2026");
    }

    /// `Display` has to write what the response field reads back as the same
    /// value, in both forms.
    #[test]
    fn wall_clock_display_round_trips_through_the_field() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(
                default,
                deserialize_with = "crate::responses::deserialize_opt_reported_wall_clock"
            )]
            date: Option<Reported<WallClock>>,
        }

        let wall = NaiveDate::from_ymd_opt(2026, 9, 22)
            .unwrap()
            .and_hms_opt(18, 47, 40)
            .unwrap();
        for (value, rendered) in [
            (WallClock::Bare(wall), "2026-09-22 18:47:40"),
            (
                WallClock::Zoned(
                    DateTime::parse_from_rfc3339("2026-09-22T18:47:40-04:00").unwrap(),
                ),
                "2026-09-22 18:47:40-04:00",
            ),
            (
                WallClock::Zoned(
                    DateTime::parse_from_rfc3339("2026-09-22T18:47:40+05:30").unwrap(),
                ),
                "2026-09-22 18:47:40+05:30",
            ),
        ] {
            assert_eq!(value.to_string(), rendered);
            let row: Row = serde_json::from_value(serde_json::json!({ "date": rendered })).unwrap();
            assert_eq!(row.date, Some(Reported::Parsed(value)));
        }
    }

    #[test]
    fn zoned_wall_clocks_are_equal_only_with_the_same_offset() {
        let at = |s: &str| WallClock::Zoned(DateTime::parse_from_rfc3339(s).unwrap());
        // The same instant, reported as two different wall clocks.
        assert_ne!(
            at("2026-09-22T18:47:40-04:00"),
            at("2026-09-22T22:47:40+00:00")
        );
        assert_eq!(
            at("2026-09-22T18:47:40-04:00"),
            at("2026-09-22T18:47:40-04:00")
        );
        assert_ne!(
            at("2026-09-22T18:47:40-04:00"),
            WallClock::Bare(
                NaiveDate::from_ymd_opt(2026, 9, 22)
                    .unwrap()
                    .and_hms_opt(18, 47, 40)
                    .unwrap()
            )
        );
    }

    #[test]
    fn wall_clock_reports_its_local_time_either_way() {
        let wall = NaiveDate::from_ymd_opt(2026, 9, 22)
            .unwrap()
            .and_hms_opt(18, 47, 40)
            .unwrap();
        let at = DateTime::parse_from_rfc3339("2026-09-22T18:47:40-04:00").unwrap();

        assert_eq!(WallClock::Zoned(at).local(), wall);
        assert_eq!(WallClock::Zoned(at).zoned(), Some(at));
        assert_eq!(WallClock::Bare(wall).local(), wall);
        assert_eq!(WallClock::Bare(wall).zoned(), None);
    }

    /// The number is the zone's offset minus Eastern's, less five, so it
    /// matches the zone's UTC offset outside Eastern DST and is one less during
    /// it.
    #[test]
    fn timezone_offset_for_window_compensates_for_eastern_dst() {
        use chrono::NaiveDate;

        let jan = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let jul = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        let n = |tz, date| TimezoneOffset::for_window(tz, date).map(|o| o.hours());
        let hours = |s: &str| Ok(Decimal::from_str_exact(s).unwrap());

        assert_eq!(n(chrono_tz::UTC, jan), hours("0"));
        assert_eq!(n(chrono_tz::UTC, jul), hours("-1"));
        // Eastern itself shifts by nothing all year.
        assert_eq!(n(SERVER_ZONE, jan), hours("-5"));
        assert_eq!(n(SERVER_ZONE, jul), hours("-5"));
        assert_eq!(n(chrono_tz::America::Vancouver, jul), hours("-8"));
        // A zone with no DST of its own moves by one when Eastern does.
        assert_eq!(n(chrono_tz::Asia::Kolkata, jan), hours("5.5"));
        assert_eq!(n(chrono_tz::Asia::Kolkata, jul), hours("4.5"));
        // +14 fits only while Eastern is on DST.
        assert_eq!(n(chrono_tz::Pacific::Kiritimati, jul), hours("13"));
        assert_eq!(
            n(chrono_tz::Pacific::Kiritimati, jan),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(14)))
        );
        // UTC-12 is refused during Eastern DST, where its days need -13, rather
        // than sent as -12 and matched an hour off; outside DST it fits.
        assert_eq!(
            n(chrono_tz::Etc::GMTPlus12, jul),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(-13)))
        );
        assert_eq!(n(chrono_tz::Etc::GMTPlus12, jan), hours("-12"));
    }

    #[test]
    fn timezone_offset_shift_is_the_number_plus_five_hours() {
        assert_eq!(TimezoneOffset::UTC.shift_seconds(), 5 * 3600);
        assert_eq!(TimezoneOffset::new(-5).unwrap().shift_seconds(), 0);
        assert_eq!(
            TimezoneOffset::new(Decimal::from_str_exact("5.5").unwrap())
                .unwrap()
                .shift_seconds(),
            10 * 3600 + 1800
        );
    }

    #[test]
    fn timezone_offset_at_handles_sub_hour_and_out_of_range() {
        use chrono::NaiveDate;
        let day = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        // India is UTC+5:30 -- a fractional offset survives.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::Asia::Kolkata, day).unwrap(),
            TimezoneOffset::new(Decimal::from_str_exact("5.5").unwrap()).unwrap()
        );
        // Kiritimati is UTC+14, outside voip.ms's -12..=13 range.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::Pacific::Kiritimati, day),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(14)))
        );
    }
}
