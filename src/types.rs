//! Hand-written domain types used in place of `String` in selected
//! generated request and response fields.
//!
//! Types here are wired into `src/generated.rs` by `xtask` through the
//! field-name override table in `xtask/src/field_overrides.rs`.

use serde::de::{Deserializer, Error as DeError, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A voip.ms routing target encoded on the wire as `tag:payload`.
///
/// Voip.ms uses this `tag:payload` scheme across all routing-like fields
/// (`routing`, `failover_busy`, `routing_match`, the `fail_over_routing_*`
/// family, …). Documented tags are mapped to named variants; anything else
/// is preserved verbatim in [`Routing::Unknown`] so that voip.ms adding a
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
                f.write_str("a voip.ms routing string of the form `tag:value`")
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
/// Several voip.ms queue/announcement fields take a number of seconds *or* a
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
}
