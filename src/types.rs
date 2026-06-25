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

/// Parse a voip.ms boolean-like wire value (`1`/`0`, `yes`/`no`, `true`/`false`,
/// and the single-letter forms), case-insensitively. Returns `None` for an
/// empty string so an absent value reads as absent rather than an error.
fn parse_wire_bool(s: &str) -> Option<Result<bool, String>> {
    let normalized = s.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }
    Some(match normalized.as_str() {
        "1" | "Y" | "YES" | "TRUE" | "T" => Ok(true),
        "0" | "N" | "NO" | "FALSE" | "F" => Ok(false),
        _ => Err(format!("invalid boolean-like value {s}")),
    })
}

/// A boolean voip.ms flag that travels on the wire as `1` / `0`.
///
/// Many voip.ms parameters are documented as `1 = true, 0 = false` (e.g.
/// `email_enable`, `url_callback_enable`, `isPartial`, `test`) yet would
/// otherwise type as `i64` or `String`. Wrapping `bool` keeps the public API a
/// plain boolean while serializing to the integer voip.ms expects -- a default
/// `bool` would serialize as `true`/`false`, which these parameters do not
/// accept.
///
/// Deserialization is tolerant: `1`/`0`, `yes`/`no`, `true`/`false` (and the
/// single-letter forms), as a string, number, or JSON bool, all parse, since
/// voip.ms is inconsistent about which it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag01(pub bool);

/// A boolean voip.ms flag that travels on the wire as `yes` / `no`.
///
/// The conference, queue, and voicemail toggles (`admin`, `say_callerid`,
/// `talk_detection`, `transcription`, …) are documented as `yes`/`no` strings.
/// Like [`Flag01`] this wraps `bool` for an ergonomic API while serializing the
/// lowercase word voip.ms expects.
///
/// Deserialization shares [`Flag01`]'s tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagYesNo(pub bool);

macro_rules! impl_flag {
    ($name:ident, $true_wire:literal, $false_wire:literal, $expecting:literal) => {
        impl From<bool> for $name {
            fn from(b: bool) -> Self {
                $name(b)
            }
        }

        impl From<$name> for bool {
            fn from(f: $name) -> bool {
                f.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(if self.0 { $true_wire } else { $false_wire })
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FlagVisitor;

                impl<'de> Visitor<'de> for FlagVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str($expecting)
                    }

                    fn visit_bool<E>(self, v: bool) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        Ok($name(v))
                    }

                    fn visit_u64<E>(self, v: u64) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        Ok($name(v != 0))
                    }

                    fn visit_i64<E>(self, v: i64) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        Ok($name(v != 0))
                    }

                    fn visit_str<E>(self, v: &str) -> Result<$name, E>
                    where
                        E: DeError,
                    {
                        match parse_wire_bool(v) {
                            Some(Ok(b)) => Ok($name(b)),
                            Some(Err(e)) => Err(E::custom(e)),
                            None => Err(E::custom("empty boolean-like value")),
                        }
                    }
                }

                deserializer.deserialize_any(FlagVisitor)
            }
        }
    };
}

impl_flag!(Flag01, "1", "0", "a 1/0 voip.ms flag");
impl_flag!(FlagYesNo, "yes", "no", "a yes/no voip.ms flag");

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
    fn flag01_serializes_to_integer_string() {
        assert_eq!(serde_json::to_string(&Flag01(true)).unwrap(), "\"1\"");
        assert_eq!(serde_json::to_string(&Flag01(false)).unwrap(), "\"0\"");
    }

    #[test]
    fn flag_yes_no_serializes_to_word() {
        assert_eq!(serde_json::to_string(&FlagYesNo(true)).unwrap(), "\"yes\"");
        assert_eq!(serde_json::to_string(&FlagYesNo(false)).unwrap(), "\"no\"");
    }

    #[test]
    fn flags_deserialize_tolerantly() {
        // voip.ms returns flags as strings, numbers, or bools, in mixed casing.
        for s in ["\"1\"", "1", "\"yes\"", "\"YES\"", "true", "\"true\""] {
            assert_eq!(
                serde_json::from_str::<Flag01>(s).unwrap(),
                Flag01(true),
                "{s}"
            );
            assert_eq!(
                serde_json::from_str::<FlagYesNo>(s).unwrap(),
                FlagYesNo(true),
                "{s}"
            );
        }
        for s in ["\"0\"", "0", "\"no\"", "\"NO\"", "false", "\"false\""] {
            assert_eq!(
                serde_json::from_str::<Flag01>(s).unwrap(),
                Flag01(false),
                "{s}"
            );
            assert_eq!(
                serde_json::from_str::<FlagYesNo>(s).unwrap(),
                FlagYesNo(false),
                "{s}"
            );
        }
    }

    #[test]
    fn flags_convert_from_bool() {
        assert_eq!(Flag01::from(true), Flag01(true));
        assert_eq!(FlagYesNo::from(false), FlagYesNo(false));
        assert!(bool::from(Flag01(true)));
    }
}
