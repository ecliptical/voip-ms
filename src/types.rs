//! Hand-written domain types used in place of `String` in selected
//! generated request and response fields.
//!
//! Types here are wired into `src/generated.rs` by `xtask` through the
//! field-name override table in `xtask/src/field_overrides.rs`.

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl fmt::Display for Routing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.tag(), self.value())
    }
}

/// Parse a `tag:value` string into a [`Routing`].
///
/// An empty string is rejected; use [`Option::None`] in the surrounding
/// struct to represent an absent value.
impl FromStr for Routing {
    type Err = RoutingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (tag, value) = match s.find(':') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => return Err(RoutingParseError::MissingColon),
        };

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
        serializer.collect_str(self)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaxMembers {
    /// A concrete member cap.
    Value(u64),
    /// No cap (wire: `Unlimited`).
    Unlimited,
}

macro_rules! impl_seconds {
    ($name:ident, $unlimited_wire:literal, $expecting:literal) => {
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

/// A UTC offset in hours, the wire form of the record-listing `timezone` knob.
///
/// The `getCDR`, `getResellerCDR`, `getSMS`, `getMMS`, `getResellerSMS`, and
/// `getResellerMMS` methods take a `timezone` parameter that VoIP.ms documents
/// as "adjust the times of the records according to Timezone (Numeric: -12 to
/// 13)". It is a whole- or fractional-hour offset from UTC, not an IANA zone
/// name -- distinct from the `getTimezones` / voicemail `timezone`, which is a
/// named zone (`America/New_York`). Passing `-5` returns timestamps in
/// UTC-05:00; omitting it leaves them in the account's configured timezone.
///
/// Callers hold a [`chrono_tz::Tz`] on those methods' `timezone` field; the
/// crate resolves it to this offset at the query's start date via
/// [`TimezoneOffset::at`] before sending. Construct one directly only to bypass
/// zone resolution and pin a fixed numeric offset.
///
/// Wraps a [`Decimal`] constrained to `-12..=13`. [`TimezoneOffset::new`]
/// rejects out-of-range values so a nonsensical offset never reaches the wire.
///
/// # Wire format
///
/// Serializes as a bare number (`-5`, `5.5`); deserializes tolerantly from a
/// JSON number or a numeric string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimezoneOffset(Decimal);

impl TimezoneOffset {
    /// The inclusive range VoIP.ms accepts, in hours from UTC.
    const MIN: i64 = -12;
    const MAX: i64 = 13;

    /// Construct an offset, rejecting a value outside `-12..=13` hours.
    pub fn new(hours: impl Into<Decimal>) -> Result<Self, TimezoneOffsetError> {
        let hours = hours.into();
        if hours < Decimal::from(Self::MIN) || hours > Decimal::from(Self::MAX) {
            return Err(TimezoneOffsetError::OutOfRange(hours));
        }

        Ok(Self(hours))
    }

    /// The UTC offset of `tz` on `date`, the value VoIP.ms wants for a query
    /// starting that day.
    ///
    /// VoIP.ms takes a single numeric offset for a whole date range, but a
    /// zone's offset shifts across DST boundaries, so the offset is fixed at
    /// one instant -- local noon on `date`, chosen to sit clear of the
    /// midnight DST fold. A zone whose offset exceeds `-12..=13` (e.g.
    /// `Pacific/Kiritimati`, +14) has no VoIP.ms representation and returns
    /// [`TimezoneOffsetError::OutOfRange`].
    pub fn at(tz: chrono_tz::Tz, date: chrono::NaiveDate) -> Result<Self, TimezoneOffsetError> {
        use chrono::{Offset, TimeZone};

        let noon = date
            .and_hms_opt(12, 0, 0)
            .expect("12:00:00 is a valid time");
        let seconds = tz
            .from_local_datetime(&noon)
            .earliest()
            .or_else(|| tz.from_local_datetime(&noon).latest())
            .map(|dt| dt.offset().fix().local_minus_utc())
            .ok_or(TimezoneOffsetError::UnresolvableInstant)?;
        // Whole hours where the offset is on the hour (the common case);
        // fractional zones (e.g. India +5:30) keep the remainder.
        let hours = Decimal::from(seconds) / Decimal::from(3600);
        Self::new(hours)
    }

    /// The offset in hours from UTC.
    pub fn hours(&self) -> Decimal {
        self.0
    }
}

impl fmt::Display for TimezoneOffset {
    /// Renders the offset as a signed `UTC±HH:MM` label (`UTC-05:00`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0.is_sign_negative() { '-' } else { '+' };
        let abs = self.0.abs();
        let whole = abs.trunc();
        let minutes = ((abs - whole) * Decimal::from(60)).round();
        write!(f, "UTC{sign}{whole:02}:{minutes:02}")
    }
}

impl FromStr for TimezoneOffset {
    type Err = TimezoneOffsetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hours =
            Decimal::from_str_exact(s.trim()).map_err(|_| TimezoneOffsetError::NotNumeric)?;
        Self::new(hours)
    }
}

impl TryFrom<i64> for TimezoneOffset {
    type Error = TimezoneOffsetError;

    fn try_from(hours: i64) -> Result<Self, Self::Error> {
        Self::new(Decimal::from(hours))
    }
}

/// Error from constructing a [`TimezoneOffset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimezoneOffsetError {
    /// The value fell outside the `-12..=13` hour range VoIP.ms accepts. Also
    /// returned by [`TimezoneOffset::at`] for a zone whose offset exceeds that
    /// range (e.g. `Pacific/Kiritimati`, +14).
    OutOfRange(Decimal),
    /// The string was not a number.
    NotNumeric,
    /// The chosen instant does not exist in the zone (a DST spring-forward
    /// gap), so no offset could be resolved.
    UnresolvableInstant,
    /// A `timezone` zone was given without the query start date that anchors
    /// its DST resolution (`date_from` / `from`), so there is no instant to
    /// resolve the offset at.
    MissingStartDate,
    /// The query start date string did not parse as a `YYYY-MM-DD` date, so
    /// the zone's offset could not be resolved at it.
    InvalidStartDate,
}

impl fmt::Display for TimezoneOffsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimezoneOffsetError::OutOfRange(v) => {
                write!(f, "timezone offset {v} is outside the range -12 to 13")
            }

            TimezoneOffsetError::NotNumeric => f.write_str("timezone offset is not a number"),
            TimezoneOffsetError::UnresolvableInstant => {
                f.write_str("timezone offset could not be resolved at the given date")
            }

            TimezoneOffsetError::MissingStartDate => f.write_str(
                "timezone requires the query start date (date_from / from) to resolve its offset",
            ),
            TimezoneOffsetError::InvalidStartDate => {
                f.write_str("query start date is not a YYYY-MM-DD date")
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

impl<'de> Deserialize<'de> for TimezoneOffset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OffsetVisitor;

        impl<'de> Visitor<'de> for OffsetVisitor {
            type Value = TimezoneOffset;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a UTC-offset number in hours between -12 and 13")
            }

            fn visit_i64<E>(self, v: i64) -> Result<TimezoneOffset, E>
            where
                E: DeError,
            {
                TimezoneOffset::try_from(v).map_err(E::custom)
            }

            fn visit_u64<E>(self, v: u64) -> Result<TimezoneOffset, E>
            where
                E: DeError,
            {
                let v = i64::try_from(v).map_err(|_| E::custom("timezone offset out of range"))?;
                TimezoneOffset::try_from(v).map_err(E::custom)
            }

            fn visit_f64<E>(self, v: f64) -> Result<TimezoneOffset, E>
            where
                E: DeError,
            {
                let d = Decimal::try_from(v)
                    .map_err(|_| E::custom("timezone offset is not a valid number"))?;
                TimezoneOffset::new(d).map_err(E::custom)
            }

            fn visit_str<E>(self, v: &str) -> Result<TimezoneOffset, E>
            where
                E: DeError,
            {
                TimezoneOffset::from_str(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(OffsetVisitor)
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl Serialize for TimezoneName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
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
        assert_eq!(r.to_string(), "sip:5552223333@sip.voip.ms:5060");
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
        assert!(TimezoneOffset::try_from(-12).is_ok());
        assert!(TimezoneOffset::try_from(13).is_ok());
        assert_eq!(
            TimezoneOffset::try_from(14),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(14)))
        );
        assert_eq!(
            TimezoneOffset::try_from(-13),
            Err(TimezoneOffsetError::OutOfRange(Decimal::from(-13)))
        );
    }

    #[test]
    fn timezone_offset_deserializes_number_and_string() {
        // Number, integer string, and fractional string all parse.
        assert_eq!(
            serde_json::from_str::<TimezoneOffset>("-5").unwrap(),
            TimezoneOffset::try_from(-5).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<TimezoneOffset>("\"-5\"").unwrap(),
            TimezoneOffset::try_from(-5).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<TimezoneOffset>("\"5.5\"").unwrap(),
            TimezoneOffset::new(Decimal::from_str_exact("5.5").unwrap()).unwrap()
        );
        // Out-of-range fails at deserialize time.
        assert!(serde_json::from_str::<TimezoneOffset>("99").is_err());
    }

    #[test]
    fn timezone_offset_serializes_as_bare_number() {
        assert_eq!(
            serde_json::to_string(&TimezoneOffset::try_from(-5).unwrap()).unwrap(),
            "\"-5\""
        );
    }

    #[test]
    fn timezone_offset_displays_utc_label() {
        assert_eq!(
            TimezoneOffset::try_from(-5).unwrap().to_string(),
            "UTC-05:00"
        );
        assert_eq!(
            TimezoneOffset::try_from(13).unwrap().to_string(),
            "UTC+13:00"
        );
        assert_eq!(
            TimezoneOffset::new(Decimal::from_str_exact("5.5").unwrap())
                .unwrap()
                .to_string(),
            "UTC+05:30"
        );
    }

    #[test]
    fn timezone_offset_at_resolves_dst() {
        use chrono::NaiveDate;
        let jan = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let jul = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        // America/New_York: EST (-5) in winter, EDT (-4) in summer.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::New_York, jan).unwrap(),
            TimezoneOffset::try_from(-5).unwrap()
        );
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::New_York, jul).unwrap(),
            TimezoneOffset::try_from(-4).unwrap()
        );
        // Arizona does not observe DST: -7 year-round.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::America::Phoenix, jul).unwrap(),
            TimezoneOffset::try_from(-7).unwrap()
        );
        // UTC is 0.
        assert_eq!(
            TimezoneOffset::at(chrono_tz::UTC, jan).unwrap(),
            TimezoneOffset::try_from(0).unwrap()
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

    #[test]
    fn timezone_name_round_trips_through_serde() {
        for name in ["America/New_York", "US/Pacific-New"] {
            let parsed: TimezoneName = name.parse().unwrap();
            let json = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, format!("\"{name}\""));
            let back: TimezoneName = serde_json::from_str(&json).unwrap();
            assert_eq!(back, parsed);
        }
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
