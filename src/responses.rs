//! Custom serde (de)serializers used by generated `*Params` and
//! `*Response` structs.
//!
//! The VoIP.ms API frequently returns numbers, booleans, dates, and
//! decimals as JSON strings (and occasionally as JSON numbers for the
//! same field across different methods). These helpers normalize both
//! forms — and treat empty / `"0000-00-00"` / `"0000-00-00 00:00:00"`
//! placeholders as `None` — into Rust types.
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

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Value;
use std::str::FromStr;

use crate::types::{MaxMembers, Routing, Seconds, WaitTime};

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
            let normalized = s.trim().to_ascii_uppercase();
            if normalized.is_empty() {
                return Ok(None);
            }
            match normalized.as_str() {
                "1" | "Y" | "YES" | "TRUE" | "T" => Ok(Some(true)),
                "0" | "N" | "NO" | "FALSE" | "F" => Ok(Some(false)),
                _ => Err(D::Error::custom(format!("invalid boolean-like string {s}"))),
            }
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected bool, string, or number, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "0000-00-00" {
                return Ok(None);
            }
            NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid date {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected date string, got {other}"
        ))),
    }
}

pub(crate) fn deserialize_opt_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "0000-00-00 00:00:00" {
                return Ok(None);
            }
            NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
                .map(Some)
                .map_err(|e| D::Error::custom(format!("invalid datetime {s}: {e}")))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected datetime string, got {other}"
        ))),
    }
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

pub(crate) fn deserialize_opt_seconds<'de, D>(deserializer: D) -> Result<Option<Seconds>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_via::<Seconds, D>(deserializer)
}

pub(crate) fn deserialize_opt_wait_time<'de, D>(
    deserializer: D,
) -> Result<Option<WaitTime>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_via::<WaitTime, D>(deserializer)
}

pub(crate) fn deserialize_opt_max_members<'de, D>(
    deserializer: D,
) -> Result<Option<MaxMembers>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_opt_via::<MaxMembers, D>(deserializer)
}

/// Deserialize an optional value via the target type's own `Deserialize`,
/// mapping JSON null and the empty string to `None`. Shared by the
/// seconds-or-sentinel helpers, whose types accept a number, a numeric string,
/// or a sentinel word but should still read an absent/empty field as `None`.
fn deserialize_opt_via<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn opt_date_folds_zero_placeholder_and_rejects_bad() {
        let call = deserialize_opt_date::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00")).unwrap(), None);
        assert_eq!(
            call(json!("2024-03-15")).unwrap(),
            Some(NaiveDate::from_ymd_opt(2024, 3, 15).unwrap())
        );
        assert!(call(json!("15/03/2024")).is_err());
        assert!(call(json!(20240315)).is_err());
    }

    #[test]
    fn opt_datetime_folds_zero_placeholder_and_rejects_bad() {
        let call = deserialize_opt_datetime::<serde_json::Value>;
        assert_eq!(call(json!(null)).unwrap(), None);
        assert_eq!(call(json!("")).unwrap(), None);
        assert_eq!(call(json!("0000-00-00 00:00:00")).unwrap(), None);
        assert_eq!(
            call(json!("2024-03-15 08:30:00")).unwrap(),
            Some(
                NaiveDate::from_ymd_opt(2024, 3, 15)
                    .unwrap()
                    .and_hms_opt(8, 30, 0)
                    .unwrap()
            )
        );
        assert!(call(json!("2024-03-15")).is_err());
        assert!(call(json!(0)).is_err());
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
        let sec = deserialize_opt_seconds::<serde_json::Value>;
        assert_eq!(sec(json!(null)).unwrap(), None);
        assert_eq!(sec(json!("  ")).unwrap(), None);
        assert_eq!(sec(json!(30)).unwrap(), Some(Seconds::Value(30)));
        assert_eq!(sec(json!("none")).unwrap(), Some(Seconds::Unlimited));
        assert!(sec(json!("garbage")).is_err());

        let wt = deserialize_opt_wait_time::<serde_json::Value>;
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
