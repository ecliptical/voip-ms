//! Custom serde (de)serializers used by generated `*Params` and
//! `*Response` structs.
//!
//! The VoIP.ms API frequently returns numbers, booleans, dates, and
//! decimals as JSON strings (and occasionally as JSON numbers for the
//! same field across different methods). These helpers normalize both
//! forms -- and treat empty / `"0000-00-00"` / `"0000-00-00 00:00:00"`
//! placeholders as `None` -- into Rust types.
//!
//! A few `bool` params also need a serializer: VoIP.ms rejects the
//! `true`/`false` a bare `bool` would emit, expecting `1`/`0` or
//! `yes`/`no`. The `serialize_*_flag_*` helpers supply that wire form.
//!
//! Some endpoints also emit `-1` as a sentinel for "not configured" in
//! fields that are otherwise unsigned identifiers. For optional unsigned
//! fields, `-1` and `"-1"` are normalized to `None`.
//!
//! These are wired up by `xtask` into `src/generated.rs`. Hand-written
//! call sites can also reference them via the `crate::responses::*`
//! module path.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Value;
use std::convert::Infallible;
use std::str::FromStr;

use crate::types::{Reported, Routing, TransactionDate, WallClock};

/// Deserialize a wire value (string, number, or bool) into its string form.
///
/// VoIP.ms returns enum-typed fields inconsistently as a JSON string (`"1"`,
/// `"yes"`) or a bare number / bool (`1`, `true`); generated enum
/// `Deserialize` impls route through this so `from_wire` always gets a string.
pub(crate) fn deserialize_enum_wire_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        other => Err(D::Error::custom(format!(
            "expected string, number, or bool, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_string_from_string_number_or_bool<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        Some(Value::Number(n)) => Ok(Some(n.to_string())),
        Some(Value::Bool(b)) => Ok(Some(b.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "expected string, number, or bool, got {other}"
        ))),
    }
}

/// Deserialize an optional caller-ID / phone-number override into its string
/// form, folding voip.ms's `-1` "not set" sentinel (and empty) to `None`.
///
/// These override fields (a sub-account's `callerid_number`, a forwarding's
/// `callerid_override`, `default_e911`, `sms_forward`) are phone-number
/// identifiers -- not integers -- so they must be `String` to survive a
/// formatted or non-NANP value; but voip.ms signals "unset" with `-1` (or an
/// empty string), which a real caller ID never is, so both collapse to `None`.
pub(crate) fn deserialize_opt_string_sentinel_none<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let s = match value {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(s)) => s,
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => {
            return Err(D::Error::custom(format!(
                "expected string, number, or bool, got {other}"
            )));
        }
    };

    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "-1" {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

pub(crate) fn deserialize_opt_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => Decimal::from_str(&n.to_string())
            .map(Some)
            .map_err(|e| D::Error::custom(format!("invalid decimal {n}: {e}"))),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                return Ok(None);
            }
            Decimal::from_str(&s)
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid decimal string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_u64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => {
            if n.as_i64() == Some(-1) {
                return Ok(None);
            }
            n.as_u64().map(Some).ok_or_else(|| {
                D::Error::custom(format!("number cannot be represented as u64: {n}"))
            })
        }
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "-1" {
                return Ok(None);
            }
            trimmed
                .parse::<u64>()
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid integer string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_bool_from_string_number_or_yn<'de, D>(
    deserializer: D,
) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(b)),
        Some(Value::Number(n)) => {
            n.as_u64().map(|v| v != 0).map(Some).ok_or_else(|| {
                D::Error::custom(format!("number cannot be represented as u64: {n}"))
            })
        }
        Some(Value::String(s)) => {
            let word = s.trim();
            if word.is_empty() {
                return Ok(None);
            }

            let spelled =
                |spellings: &[&str]| spellings.iter().any(|w| word.eq_ignore_ascii_case(w));
            if spelled(&["1", "Y", "YES", "TRUE", "T"]) {
                Ok(Some(true))
            } else if spelled(&["0", "N", "NO", "FALSE", "F"]) {
                Ok(Some(false))
            } else {
                Err(D::Error::custom(format!("invalid boolean-like string {s}")))
            }
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected bool, string, or number, got {other}"
        ))),
    }
}

/// The wire spelling of a VoIP.ms timestamp.
pub(crate) const DATETIME_WIRE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// A VoIP.ms timestamp with the offset [`crate::attach_offset`] and
/// [`crate::attach_zone`] append (`2026-09-16 15:14:35-04:00`).
pub(crate) const OFFSET_DATETIME_WIRE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%:z";

/// The wire spelling of a VoIP.ms calendar date.
const DATE_WIRE_FORMAT: &str = "%Y-%m-%d";

/// Whether a trimmed wire value carries no date: absent, or the zero-date
/// placeholder.
///
/// The placeholder is recognized by its date rather than by an exact match,
/// because either precision reaches either kind of field -- a `NaiveDate` field
/// can receive `0000-00-00 00:00:00` and a `NaiveDateTime` field a bare
/// `0000-00-00`, and an exact comparison folds one spelling while failing the
/// whole envelope on the other.
fn is_blank_or_zero_date(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed.starts_with("0000-00-00")
}

/// Whether a trimmed wire value *is* the zero-date placeholder, rather than
/// merely starting with it.
///
/// [`is_blank_or_zero_date`] tests the prefix, which is right where the whole
/// value is one timestamp: the offset a record-listing value carries is
/// appended to the placeholder too. It is wrong where the value can be a range,
/// so `deserialize_opt_transaction_date` asks this instead.
fn is_zero_date(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed == "0000-00-00" || trimmed == "0000-00-00 00:00:00"
}

/// Whether a timestamp already names a UTC offset (`Z`, `-04:00`, `+0530`).
///
/// Both sides of the record-listing contract ask this question, and they have
/// to agree: [`crate::attach_offset`] skips a value that already names one, and
/// the deserializer rejects one that does not. Two predicates would disagree at
/// the edges -- a value one skips and the other refuses costs the whole
/// envelope, and a value one suffixes and the other accepts is corrupted and
/// then reported as unreadable -- so this is the single answer, called from
/// both.
///
/// The offset is looked for after the last `T` or space, so the date's own
/// hyphens cannot be mistaken for its sign. The digit count is deliberately
/// loose (`-4:00` as well as `-04:00`): being generous here is safe, since a
/// value this accepts and chrono rejects degrades, while one this refuses fails
/// the envelope.
pub(crate) fn names_offset(s: &str) -> bool {
    let s = s.trim();
    if s.ends_with('Z') || s.ends_with('z') {
        return true;
    }

    let Some(at) = s.rfind(['T', ' ']) else {
        return false;
    };

    s[at..].rsplit_once(['+', '-']).is_some_and(|(head, zone)| {
        !head.is_empty()
            && !zone.is_empty()
            && zone.chars().all(|c| c.is_ascii_digit() || c == ':')
            && zone.chars().filter(char::is_ascii_digit).count() <= 4
    })
}

/// Deserialize a timestamp that names its UTC offset.
///
/// The record-listing methods (`getCDR`, `getSMS`, …) report a wall clock in
/// the offset the request asked for but leave the offset off the value;
/// [`crate::attach_offset`] puts it back before this parses it.
///
/// A value with **no offset** is rejected rather than read as UTC: an
/// unqualified timestamp silently taken for an absolute one is the whole
/// failure this typing exists to prevent, and that is a broken contract rather
/// than an odd value. A value that *has* an offset but does not parse is an odd
/// value, so it degrades into [`Reported::Unreadable`] and costs its own field
/// -- these are the highest-row-count methods in the API, so failing the
/// envelope there costs the most.
pub(crate) fn deserialize_opt_datetime_offset<'de, D>(
    deserializer: D,
) -> Result<Option<Reported<DateTime<FixedOffset>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(text) = opt_wire_text(deserializer)? else {
        return Ok(None);
    };

    if is_blank_or_zero_date(&text) {
        return Ok(None);
    }

    if !names_offset(&text) {
        return Err(D::Error::custom(format!(
            "record-listing timestamp {text} names no UTC offset"
        )));
    }

    Ok(Some(match parse_offset_datetime(&text) {
        Some(at) => Reported::Parsed(at),
        None => Reported::Unreadable(text),
    }))
}

/// A timestamp that names its offset, in the wire spelling with the offset
/// appended or as RFC 3339.
fn parse_offset_datetime(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(s, OFFSET_DATETIME_WIRE_FORMAT)
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .ok()
}

/// Deserialize a timestamp that may or may not name its UTC offset into a
/// [`WallClock`], keeping the wire text when it does not parse.
///
/// Unlike [`deserialize_opt_datetime_offset`], a bare wall clock is accepted:
/// [`crate::attach_zone`] leaves one bare when no zone was supplied or when the
/// wall clock is ambiguous or nonexistent in the zone, so a bare value is part
/// of this field's contract rather than a break in it.
pub(crate) fn deserialize_opt_reported_wall_clock<'de, D>(
    deserializer: D,
) -> Result<Option<Reported<WallClock>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_reported(deserializer, |s| {
        if names_offset(s) {
            parse_offset_datetime(s).map(WallClock::Zoned)
        } else {
            NaiveDateTime::parse_from_str(s, DATETIME_WIRE_FORMAT)
                .ok()
                .map(WallClock::Bare)
        }
    })
}

pub(crate) fn deserialize_opt_routing<'de, D>(deserializer: D) -> Result<Option<Routing>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Routing::from_str(trimmed)
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid routing string {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected routing string, got {other}"
        ))),
    }
}

/// Deserialize an optional response date, keeping the wire text when it does
/// not parse.
///
/// `parse` is the strict reading; whatever it rejects is kept verbatim in
/// [`Reported::Unreadable`] rather than failing the deserialization. A `*Response` is one value
/// built from one envelope, so erroring on a single field discards every record
/// beside it -- the break this crate has already paid for twice. Keeping the
/// text rather than answering `None` leaves the value salvageable and leaves
/// "unreadable" distinguishable from "absent", which is what the live drift
/// harness reads.
fn deserialize_opt_reported<'de, T, D>(
    deserializer: D,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<Reported<T>>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(text) = opt_wire_text(deserializer)? else {
        return Ok(None);
    };

    if is_blank_or_zero_date(&text) {
        return Ok(None);
    }

    Ok(Some(match parse(&text) {
        Some(value) => Reported::Parsed(value),
        None => Reported::Unreadable(text),
    }))
}

/// Deserialize a response field's calendar date, keeping the wire text when it
/// does not parse. See [`deserialize_opt_reported`].
pub(crate) fn deserialize_opt_reported_date<'de, D>(
    deserializer: D,
) -> Result<Option<Reported<NaiveDate>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_reported(deserializer, |s| {
        NaiveDate::parse_from_str(s, DATE_WIRE_FORMAT).ok()
    })
}

/// Deserialize a response field's timestamp, keeping the wire text when it does
/// not parse. See [`deserialize_opt_reported`].
pub(crate) fn deserialize_opt_reported_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<Reported<NaiveDateTime>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_reported(deserializer, |s| {
        NaiveDateTime::parse_from_str(s, DATETIME_WIRE_FORMAT).ok()
    })
}

/// The trimmed wire text of an optional scalar field, or `None` when the field
/// is absent or blank.
///
/// A number or a bool renders to its text rather than being rejected: VoIP.ms
/// sends the same field as a string on one method and a bare scalar on another,
/// so the spelling is not a contract. A list or an object is rejected, because
/// that is a shape, and no scalar type can stand in for one.
fn opt_wire_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let text = match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(s)) => s,
        Some(v @ (Value::Number(_) | Value::Bool(_))) => v.to_string(),
        Some(other) => {
            return Err(D::Error::custom(format!(
                "expected string, number, or bool, got {other}"
            )));
        }
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(trimmed.to_string()))
}

/// Deserialize an optional field into a type that parses infallibly from the
/// wire text.
///
/// Every type reaching this carries a catch-all variant, so it holds whatever
/// arrived: failing instead would cost the record, the envelope, and every row
/// beside it over one odd value, which is what these types exist to prevent.
/// Placeholder folding is *not* done here -- it is a date's contract, not every
/// caller's, and folding a `0000-00-00 to ...` range would discard the fact
/// that a range was reported at all.
pub(crate) fn deserialize_opt_from_wire_text<'de, T, D>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    T: FromStr<Err = Infallible>,
    D: Deserializer<'de>,
{
    let Some(text) = opt_wire_text(deserializer)? else {
        return Ok(None);
    };

    let Ok(parsed) = text.parse::<T>();
    Ok(Some(parsed))
}

/// Deserialize a transaction-history row's `date` into a [`TransactionDate`]:
/// a timestamp or a bare date when the row names a point in time, a pair of
/// dates when it names a window, the value verbatim when it is none of those,
/// and `None` for absent / empty / the placeholder.
pub(crate) fn deserialize_opt_transaction_date<'de, D>(
    deserializer: D,
) -> Result<Option<TransactionDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(text) = opt_wire_text(deserializer)? else {
        return Ok(None);
    };

    // The placeholder is the whole value, not a prefix of it: this is the one
    // field that can report a range, and `0000-00-00 to 2026-08-31` says a
    // window was reported even though its start is the zero date. Folding on
    // the prefix would make that indistinguishable from an absent field, which
    // is the information `TransactionDate` exists to keep.
    if is_zero_date(&text) {
        return Ok(None);
    }

    let Ok(date) = text.parse::<TransactionDate>();
    Ok(Some(date))
}

/// Deserialize an optional value via the target type's own `Deserialize`,
/// mapping JSON null and a blank string to `None`, so a type with no blank
/// spelling of its own still reads an empty field as absent. Unlike
/// [`deserialize_opt_from_wire_text`], a value `T` rejects fails the field.
pub(crate) fn deserialize_opt_via<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(v) => T::deserialize(v).map(Some).map_err(D::Error::custom),
    }
}

/// Deserialize a list field that VoIP.ms may return either as a JSON array or,
/// when the method yields a single row, as a bare unwrapped object. VoIP.ms's
/// `print_r`-derived output collapses a one-element list to the element itself
/// (e.g. `getVoicemailMessageFile`'s `message` comes back as an object, not a
/// one-element array), so every generated list field accepts both wire forms: an
/// array becomes the `Vec` as-is, a lone value becomes a one-element `Vec`, and
/// null / absent / empty-string is the empty `Vec` -- an omitted or empty
/// collection is not distinguished from a present-but-empty one.
pub(crate) fn deserialize_vec_from_single_or_seq<'de, T, D>(
    deserializer: D,
) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|v| T::deserialize(v).map_err(D::Error::custom))
            .collect(),
        Some(one) => T::deserialize(one)
            .map(|v| vec![v])
            .map_err(D::Error::custom),
    }
}

/// Deserialize a string-keyed map from a JSON object, tolerating absence.
///
/// VoIP.ms returns a `code => description` catalog (e.g. `getLNPListStatus`'s
/// `list_status`) as a JSON object whose keys are data, not schema. Absent,
/// `null`, and an empty string all yield an empty map -- the same
/// absent-is-empty convention the list helper follows. A non-object, non-empty
/// value is an error.
pub(crate) fn deserialize_map_from_object<'de, K, V, D>(
    deserializer: D,
) -> Result<std::collections::HashMap<K, V>, D::Error>
where
    K: Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
    V: Deserialize<'de>,
    D: Deserializer<'de>,
{
    use serde::de::IntoDeserializer as _;

    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(std::collections::HashMap::new()),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(std::collections::HashMap::new()),
        Some(Value::Object(entries)) => entries
            .into_iter()
            .map(|(k, v)| {
                let key = K::deserialize(k.into_deserializer())
                    .map_err(|e: serde::de::value::Error| D::Error::custom(e))?;
                let val = V::deserialize(v).map_err(D::Error::custom)?;
                Ok((key, val))
            })
            .collect(),
        Some(other) => Err(D::Error::custom(format!("expected object, got {other}"))),
    }
}

/// `skip_serializing_if` predicate for true-only flag params (`test`): a
/// `false` value is equivalent to absent and is left off the wire.
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

/// Serialize an optional `1`/`0` flag param: `Some(true)` → `"1"`,
/// `Some(false)` → `"0"`. `None` would be skipped before reaching here, so it
/// serializes nothing.
pub(crate) fn serialize_opt_flag_01<S>(v: &Option<bool>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(b) => serialize_flag_01(b, s),
        None => s.serialize_none(),
    }
}

/// Serialize an optional `yes`/`no` flag param: `Some(true)` → `"yes"`,
/// `Some(false)` → `"no"`. `None` would be skipped before reaching here.
pub(crate) fn serialize_opt_flag_yes_no<S>(v: &Option<bool>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(true) => s.serialize_str("yes"),
        Some(false) => s.serialize_str("no"),
        None => s.serialize_none(),
    }
}

/// Serialize a `1`/`0` flag param: `true` → `"1"`, `false` → `"0"`.
pub(crate) fn serialize_flag_01<S>(v: &bool, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(if *v { "1" } else { "0" })
}

/// Serialize an optional named-zone param as its IANA name (`America/New_York`),
/// the form the voicemail and `getTimezones` methods expect. `None` would be
/// skipped before reaching here.
pub(crate) fn serialize_opt_tz<S>(v: &Option<Tz>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(tz) => s.serialize_str(tz.name()),
        None => s.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Seconds, TimezoneName, WaitTime};
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::HashMap;

    // The (de)serializers here are `deserialize_with` callbacks generic over
    // `D: Deserializer`, and `serde_json::Value` is a `Deserializer`, so each
    // is driven by passing a `json!(..)` value straight in -- the same wire
    // shapes the generated code routes through them.

    #[test]
    fn enum_wire_string_coerces_scalars_and_rejects_composites() {
        let call = deserialize_enum_wire_string::<serde_json::Value>;
        assert_eq!(call(json!("yes")).unwrap(), "yes");
        assert_eq!(call(json!(1)).unwrap(), "1");
        assert_eq!(call(json!(true)).unwrap(), "true");
        assert!(call(json!([1, 2])).is_err());
        assert!(call(json!({"a": 1})).is_err());
    }

    #[test]
    fn opt_string_folds_empty_and_coerces_scalars() {
        let call = deserialize_opt_string_from_string_number_or_bool::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("   ")).unwrap(), None);
        assert_eq!(call(json!("hi")).unwrap(), Some("hi".to_string()));
        assert_eq!(call(json!(42)).unwrap(), Some("42".to_string()));
        assert_eq!(call(json!(false)).unwrap(), Some("false".to_string()));
        assert!(call(json!(["x"])).is_err());
    }

    #[test]
    fn opt_string_sentinel_none_maps_minus_one_and_scalars() {
        let call = deserialize_opt_string_sentinel_none::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("-1")).unwrap(), None);
        assert_eq!(call(json!(-1)).unwrap(), None);
        assert_eq!(
            call(json!("5551234567")).unwrap(),
            Some("5551234567".to_string())
        );
        assert_eq!(call(json!(1000)).unwrap(), Some("1000".to_string()));
        assert_eq!(call(json!(true)).unwrap(), Some("true".to_string()));
        assert!(call(json!({})).is_err());
    }

    #[test]
    fn opt_decimal_from_string_or_number() {
        let call = deserialize_opt_decimal_from_string_or_number::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(
            call(json!("3.50")).unwrap(),
            Some(Decimal::from_str("3.50").unwrap())
        );
        assert_eq!(
            call(json!(2)).unwrap(),
            Some(Decimal::from_str("2").unwrap())
        );
        assert!(call(json!("not-a-number")).is_err());
        assert!(call(json!(true)).is_err());
    }

    #[test]
    fn opt_u64_folds_minus_one_and_rejects_bad_input() {
        let call = deserialize_opt_u64_from_string_or_number::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("-1")).unwrap(), None);
        assert_eq!(call(json!(-1)).unwrap(), None);
        assert_eq!(call(json!(7)).unwrap(), Some(7));
        assert_eq!(call(json!("42")).unwrap(), Some(42));
        // A negative other than -1 cannot be a u64.
        assert!(call(json!(-2)).is_err());
        assert!(call(json!("abc")).is_err());
        assert!(call(json!(true)).is_err());
    }

    #[test]
    fn opt_bool_accepts_all_documented_forms() {
        let call = deserialize_opt_bool_from_string_number_or_yn::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        for t in [
            json!(true),
            json!(1),
            json!("1"),
            json!("y"),
            json!("YES"),
            json!("true"),
            json!("t"),
        ] {
            assert_eq!(call(t.clone()).unwrap(), Some(true), "{t}");
        }

        for f in [
            json!(false),
            json!(0),
            json!("0"),
            json!("n"),
            json!("NO"),
            json!("false"),
            json!("f"),
        ] {
            assert_eq!(call(f.clone()).unwrap(), Some(false), "{f}");
        }

        assert!(call(json!("maybe")).is_err());
        assert!(call(json!(-1)).is_err());
        assert!(call(json!([1])).is_err());
    }

    #[test]
    fn opt_reported_date_folds_placeholders_and_keeps_what_it_cannot_read() {
        let call = deserialize_opt_reported_date::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00")).unwrap(), None);
        assert_eq!(
            call(json!("2024-03-15")).unwrap(),
            Some(Reported::Parsed(
                NaiveDate::from_ymd_opt(2024, 3, 15).unwrap()
            ))
        );
        // Unreadable keeps the value instead of erroring, so one odd date
        // costs its own field and not the records beside it.
        assert_eq!(
            call(json!("15/03/2024")).unwrap(),
            Some(Reported::Unreadable("15/03/2024".to_string()))
        );
        assert_eq!(
            call(json!(20240315)).unwrap(),
            Some(Reported::Unreadable("20240315".to_string()))
        );
    }

    #[test]
    fn opt_reported_datetime_folds_placeholders_and_keeps_what_it_cannot_read() {
        let call = deserialize_opt_reported_datetime::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00 00:00:00")).unwrap(), None);
        assert_eq!(
            call(json!("2024-03-15 08:30:00")).unwrap(),
            Some(Reported::Parsed(
                NaiveDate::from_ymd_opt(2024, 3, 15)
                    .unwrap()
                    .and_hms_opt(8, 30, 0)
                    .unwrap()
            ))
        );
        // A date where a timestamp was documented is exactly the drift this
        // wrapper exists for: it is kept, not guessed at and not discarded.
        assert_eq!(
            call(json!("2024-03-15")).unwrap(),
            Some(Reported::Unreadable("2024-03-15".to_string()))
        );
        assert_eq!(
            call(json!(0)).unwrap(),
            Some(Reported::Unreadable("0".to_string()))
        );
    }

    #[test]
    fn opt_datetime_offset_requires_a_zone() {
        let call = deserialize_opt_datetime_offset::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        // The placeholder is recognized by its date: the offset is attached to
        // it like any other value.
        assert_eq!(call(json!("0000-00-00 00:00:00")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00 00:00:00-04:00")).unwrap(), None);
        assert_eq!(
            call(json!("2024-03-15 08:30:00-04:00")).unwrap(),
            Some(Reported::Parsed(
                DateTime::parse_from_rfc3339("2024-03-15T08:30:00-04:00").unwrap()
            ))
        );
        assert_eq!(
            call(json!("2024-03-15T08:30:00Z")).unwrap(),
            Some(Reported::Parsed(
                DateTime::parse_from_rfc3339("2024-03-15T08:30:00+00:00").unwrap()
            ))
        );
        // An unqualified wall clock is rejected rather than read as UTC: the
        // offset is the contract these methods are typed around, and inventing
        // one is the failure that typing exists to prevent.
        assert!(call(json!("2024-03-15 08:30:00")).is_err());
        assert!(call(json!("2024-03-15")).is_err());
        // A bare number reaches the same judgment as any other text rather than
        // being rejected for its JSON type: it names no offset, so it fails the
        // contract like an unqualified string, not because it was not a string.
        assert!(call(json!(0)).is_err());
        // A shape is still rejected: no timestamp can stand in for one.
        assert!(call(json!({"a": 1})).is_err());
        assert!(call(json!([1])).is_err());
        // A value that *carries* an offset but does not parse is an odd value,
        // not a broken contract, so it costs its own field instead of every
        // row in what are the API's longest responses.
        assert_eq!(
            call(json!("2026-02-30 00:00:00-04:00")).unwrap(),
            Some(Reported::Unreadable(
                "2026-02-30 00:00:00-04:00".to_string()
            ))
        );
    }

    #[test]
    fn opt_reported_wall_clock_reads_a_bare_and_a_qualified_value() {
        let call = deserialize_opt_reported_wall_clock::<serde_json::Value>;
        let wall = NaiveDate::from_ymd_opt(2026, 9, 22)
            .unwrap()
            .and_hms_opt(18, 47, 40)
            .unwrap();
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00 00:00:00")).unwrap(), None);
        // A bare wall clock is part of this field's contract, unlike the
        // record-listing timestamps, so it reads rather than failing.
        assert_eq!(
            call(json!("2026-09-22 18:47:40")).unwrap(),
            Some(Reported::Parsed(WallClock::Bare(wall)))
        );
        assert_eq!(
            call(json!("2026-09-22 18:47:40-04:00")).unwrap(),
            Some(Reported::Parsed(WallClock::Zoned(
                DateTime::parse_from_rfc3339("2026-09-22T18:47:40-04:00").unwrap()
            )))
        );
        assert_eq!(
            call(json!("2026-09-22T22:47:40Z")).unwrap(),
            Some(Reported::Parsed(WallClock::Zoned(
                DateTime::parse_from_rfc3339("2026-09-22T22:47:40+00:00").unwrap()
            )))
        );
        // Neither form parses: the text is kept, as for every response date.
        assert_eq!(
            call(json!("2026-09-22")).unwrap(),
            Some(Reported::Unreadable("2026-09-22".to_string()))
        );
        assert_eq!(
            call(json!("2026-02-30 00:00:00-04:00")).unwrap(),
            Some(Reported::Unreadable(
                "2026-02-30 00:00:00-04:00".to_string()
            ))
        );
        assert!(call(json!(["2026-09-22 18:47:40"])).is_err());
    }

    /// The offset check reads the time portion, so the date's own hyphens
    /// never look like one and a zone-less value cannot slip through.
    #[test]
    fn names_offset_reads_the_time_not_the_date() {
        assert!(names_offset("2024-03-15 08:30:00-04:00"));
        assert!(names_offset("2024-03-15 08:30:00+0530"));
        assert!(names_offset("2024-03-15T08:30:00Z"));
        // Unpadded and short forms count: `attach_offset` must skip exactly
        // what this accepts, and suffixing one of these would corrupt it.
        assert!(names_offset("2024-03-15 19:14:35-4:00"));
        assert!(names_offset("2024-03-15 19:14:35-05"));
        assert!(!names_offset("2024-03-15 08:30:00"));
        assert!(!names_offset("2024-03-15"));
        assert!(!names_offset("2024-03-15 08:30:00-oops"));
    }

    #[test]
    fn opt_transaction_date_reads_a_timestamp_a_date_a_span_and_anything_else() {
        let call = deserialize_opt_transaction_date::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00 00:00:00")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00")).unwrap(), None);
        // The placeholder is the whole value here, so a window whose start is
        // the zero date still reports as a window rather than as absence.
        assert_eq!(
            call(json!("0000-00-00 to 2026-08-31")).unwrap(),
            Some(TransactionDate::Unrecognized(
                "0000-00-00 to 2026-08-31".to_string()
            ))
        );
        // A date with no time of day stays a date rather than gaining a midnight.
        assert_eq!(
            call(json!("2010-10-29")).unwrap(),
            Some(TransactionDate::On(
                NaiveDate::from_ymd_opt(2010, 10, 29).unwrap()
            ))
        );
        assert_eq!(
            call(json!("2016-06-03 00:03:46")).unwrap(),
            Some(TransactionDate::At(
                NaiveDate::from_ymd_opt(2016, 6, 3)
                    .unwrap()
                    .and_hms_opt(0, 3, 46)
                    .unwrap()
            ))
        );
        assert_eq!(
            call(json!("2026-08-01 to 2026-08-31")).unwrap(),
            Some(TransactionDate::Period {
                from: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                to: NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
            })
        );
        // No form parses, so the value survives instead of failing the record
        // it belongs to -- and every record beside it.
        assert_eq!(
            call(json!("whenever")).unwrap(),
            Some(TransactionDate::Unrecognized("whenever".to_string()))
        );
        // Nor does a shape that is not a string: the sibling fields of this
        // struct all accept a bare number, so this one cannot be the single
        // field that fails the envelope over one.
        assert_eq!(
            call(json!(0)).unwrap(),
            Some(TransactionDate::Unrecognized("0".to_string()))
        );
        assert_eq!(
            call(json!(true)).unwrap(),
            Some(TransactionDate::Unrecognized("true".to_string()))
        );
    }

    /// Both zero-date spellings reach both kinds of field, so each deserializer
    /// folds the one its own format cannot parse as well as its own.
    #[test]
    fn every_date_deserializer_folds_both_zero_date_spellings() {
        let date = deserialize_opt_reported_date::<serde_json::Value>;
        assert_eq!(date(json!("0000-00-00")).unwrap(), None);
        assert_eq!(date(json!("0000-00-00 00:00:00")).unwrap(), None);

        let datetime = deserialize_opt_reported_datetime::<serde_json::Value>;
        assert_eq!(datetime(json!("0000-00-00 00:00:00")).unwrap(), None);
        assert_eq!(datetime(json!("0000-00-00")).unwrap(), None);

        let offset = deserialize_opt_datetime_offset::<serde_json::Value>;
        assert_eq!(offset(json!("0000-00-00")).unwrap(), None);
        assert_eq!(offset(json!("0000-00-00 00:00:00-04:00")).unwrap(), None);
    }

    /// The zone reader tolerates a scalar spelling and rejects a shape, which
    /// is the rule the date readers follow.
    #[test]
    fn opt_timezone_name_tolerates_a_scalar_and_rejects_a_shape() {
        let call = deserialize_opt_from_wire_text::<TimezoneName, serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("  ")).unwrap(), None);
        assert_eq!(
            call(json!("America/New_York")).unwrap(),
            Some(TimezoneName::Known(chrono_tz::America::New_York))
        );
        // A legacy name the IANA database dropped survives verbatim.
        assert_eq!(
            call(json!("US/Pacific-New")).unwrap(),
            Some(TimezoneName::Unrecognized("US/Pacific-New".to_string()))
        );
        assert_eq!(
            call(json!(5)).unwrap(),
            Some(TimezoneName::Unrecognized("5".to_string()))
        );
        // A JSON object is a shape, and no zone name can stand in for one.
        assert!(call(json!({"a": 1})).is_err());
        assert!(call(json!(["America/New_York"])).is_err());
    }

    #[test]
    fn opt_routing_folds_empty_and_rejects_non_string() {
        let call = deserialize_opt_routing::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(
            call(json!("fwd:15555")).unwrap(),
            Some(Routing::Forward("15555".into()))
        );
        // A routing string missing its `:` separator is a parse error.
        assert!(call(json!("nocolon")).is_err());
        assert!(call(json!(5)).is_err());
    }

    #[test]
    fn opt_seconds_and_wait_time_via_helper() {
        let sec = deserialize_opt_via::<Seconds, serde_json::Value>;
        assert_eq!(sec(json!(null)).unwrap(), None);
        assert_eq!(sec(json!("  ")).unwrap(), None);
        assert_eq!(sec(json!(30)).unwrap(), Some(Seconds::Value(30)));
        assert_eq!(sec(json!("none")).unwrap(), Some(Seconds::Unlimited));
        assert!(sec(json!("garbage")).is_err());

        let wt = deserialize_opt_via::<WaitTime, serde_json::Value>;
        assert_eq!(wt(json!("unlimited")).unwrap(), Some(WaitTime::Unlimited));
        assert_eq!(wt(json!("45")).unwrap(), Some(WaitTime::Value(45)));
    }

    // The list and map helpers take a real `Deserializer`, so they are driven
    // through a wrapper struct field, matching generated usage.

    #[derive(Deserialize)]
    struct VecWrap {
        #[serde(default, deserialize_with = "deserialize_vec_from_single_or_seq")]
        items: Vec<u64>,
    }

    #[test]
    fn vec_from_single_or_seq_coerces_all_shapes() {
        let empty: VecWrap = serde_json::from_value(json!({})).unwrap();
        assert_eq!(empty.items, Vec::<u64>::new());
        let null: VecWrap = serde_json::from_value(json!({"items": null})).unwrap();
        assert_eq!(null.items, Vec::<u64>::new());
        let blank: VecWrap = serde_json::from_value(json!({"items": ""})).unwrap();
        assert_eq!(blank.items, Vec::<u64>::new());
        let one: VecWrap = serde_json::from_value(json!({"items": 7})).unwrap();
        assert_eq!(one.items, vec![7]);
        let many: VecWrap = serde_json::from_value(json!({"items": [1, 2, 3]})).unwrap();
        assert_eq!(many.items, vec![1, 2, 3]);
        assert!(serde_json::from_value::<VecWrap>(json!({"items": "x"})).is_err());
    }

    #[derive(Deserialize)]
    struct MapWrap {
        #[serde(default, deserialize_with = "deserialize_map_from_object")]
        entries: HashMap<String, String>,
    }

    #[test]
    fn map_from_object_tolerates_absence_and_rejects_non_object() {
        let empty: MapWrap = serde_json::from_value(json!({})).unwrap();
        assert!(empty.entries.is_empty());
        let null: MapWrap = serde_json::from_value(json!({"entries": null})).unwrap();
        assert!(null.entries.is_empty());
        let blank: MapWrap = serde_json::from_value(json!({"entries": ""})).unwrap();
        assert!(blank.entries.is_empty());
        let full: MapWrap =
            serde_json::from_value(json!({"entries": {"1": "New", "2": "Old"}})).unwrap();
        assert_eq!(full.entries.get("1").map(String::as_str), Some("New"));
        assert!(serde_json::from_value::<MapWrap>(json!({"entries": [1, 2]})).is_err());
    }

    #[derive(serde::Serialize)]
    struct FlagWrap {
        #[serde(serialize_with = "serialize_opt_flag_01")]
        a: Option<bool>,
        #[serde(serialize_with = "serialize_opt_flag_yes_no")]
        b: Option<bool>,
        #[serde(serialize_with = "serialize_flag_01")]
        c: bool,
    }

    #[test]
    fn flag_serializers_emit_wire_forms_including_none() {
        let some = FlagWrap {
            a: Some(true),
            b: Some(false),
            c: true,
        };
        assert_eq!(
            serde_json::to_value(&some).unwrap(),
            json!({"a": "1", "b": "no", "c": "1"})
        );
        let none = FlagWrap {
            a: None,
            b: None,
            c: false,
        };
        assert_eq!(
            serde_json::to_value(&none).unwrap(),
            json!({"a": null, "b": null, "c": "0"})
        );
    }

    #[test]
    fn is_false_predicate() {
        assert!(is_false(&false));
        assert!(!is_false(&true));
    }
}
