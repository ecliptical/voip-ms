//! The live-vs-spec key diff.
//!
//! The drift probe answers "can the typed shape read what the API sent?". This
//! answers the opposite question -- "did the API send something the typed shape
//! has no field for?" -- which no amount of deserializing can detect: the
//! generated structs carry no `deny_unknown_fields`, so an unmodeled key is
//! dropped in silence. Both `getCDR`'s `ip` and `useragent` reached production
//! that way.
//!
//! The comparison is one-directional on purpose. A key on the wire with no
//! modeled path is a fidelity gap in the crate. The reverse -- a modeled path
//! absent from the response -- is normal: VoIP.ms omits fields an account has
//! no data for, so it would report on every run and mean nothing.

use serde_json::Value;

/// Live key paths the modeled set does not cover, sorted and deduplicated.
///
/// `modeled` is one method's entry from the generated `RESPONSE_FIELDS` table,
/// in the override file's path grammar.
pub fn unmodeled_paths(raw: &Value, modeled: &[&str]) -> Vec<String> {
    let mut live = Vec::new();
    collect(raw, "", &mut live);
    live.sort();
    live.dedup();
    live.retain(|path| !modeled.iter().any(|spec| covers(spec, path)));
    live
}

/// Walk the live envelope into the dotted path of every key it carries, with
/// `[]` marking a list element -- the same grammar the modeled table uses.
fn collect(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };

                out.push(path.clone());
                collect(inner, &path, out);
            }
        }
        // Every element shares one path: the crate models a list by one element
        // template, and a key in the tenth row is as unmodeled as in the first.
        Value::Array(items) => {
            let path = format!("{prefix}[]");
            for item in items {
                collect(item, &path, out);
            }
        }
        _ => {}
    }
}

/// Whether a modeled path covers a live one, segment by segment.
fn covers(spec: &str, live: &str) -> bool {
    let mut spec_segments = spec.split('.');
    let mut live_segments = live.split('.');
    loop {
        match (spec_segments.next(), live_segments.next()) {
            (None, None) => return true,
            (Some(s), Some(l)) if segment_covers(s, l) => {}
            _ => return false,
        }
    }
}

/// One segment, where a `*` name stands for any key of a dynamic-key map. The
/// `[]` suffixes still have to agree: a list position is schema, not data.
fn segment_covers(spec: &str, live: &str) -> bool {
    let (spec_name, spec_brackets) = split_brackets(spec);
    let (live_name, live_brackets) = split_brackets(live);
    spec_brackets == live_brackets && (spec_name == "*" || spec_name == live_name)
}

fn split_brackets(segment: &str) -> (&str, &str) {
    match segment.find('[') {
        Some(i) => segment.split_at(i),
        None => (segment, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The modeled paths for `getCDR` before the two fields were declared.
    const CDR_BEFORE: &[&str] = &[
        "status",
        "cdr",
        "cdr[].account",
        "cdr[].call_logs",
        "cdr[].callerid",
        "cdr[].date",
        "cdr[].description",
        "cdr[].destination",
        "cdr[].destination_type",
        "cdr[].disposition",
        "cdr[].duration",
        "cdr[].rate",
        "cdr[].seconds",
        "cdr[].total",
        "cdr[].uniqueid",
    ];

    /// One record with the key set a live `getCDR` returns, per issue #21.
    fn live_cdr() -> Value {
        json!({
            "status": "success",
            "cdr": [{
                "account": "100000_VoIP",
                "call_logs": "",
                "callerid": "\"John Doe\" <5551112222>",
                "date": "2026-09-15 20:05:27",
                "description": "Inbound DID",
                "destination": "5551234567",
                "destination_type": "IN:CAN",
                "disposition": "ANSWERED",
                "duration": "00:00:03",
                "ip": "",
                "rate": "0.00900000",
                "seconds": "3",
                "total": "0.00090000",
                "uniqueid": "4832494463",
                "useragent": ""
            }]
        })
    }

    #[test]
    fn reports_the_keys_the_typed_shape_dropped() {
        assert_eq!(
            unmodeled_paths(&live_cdr(), CDR_BEFORE),
            ["cdr[].ip", "cdr[].useragent"]
        );
    }

    #[test]
    fn reports_nothing_once_the_fields_are_modeled() {
        let modeled: Vec<&str> = CDR_BEFORE
            .iter()
            .copied()
            .chain(["cdr[].ip", "cdr[].useragent"])
            .collect();
        assert!(unmodeled_paths(&live_cdr(), &modeled).is_empty());
    }

    #[test]
    fn a_modeled_path_the_response_omits_is_not_reported() {
        let sparse = json!({ "status": "success", "cdr": [{ "uniqueid": "1" }] });
        assert!(unmodeled_paths(&sparse, CDR_BEFORE).is_empty());
    }

    #[test]
    fn every_row_is_checked_not_just_the_first() {
        let rows = json!({
            "status": "success",
            "cdr": [{ "uniqueid": "1" }, { "uniqueid": "2", "ip": "203.0.113.7" }]
        });
        assert_eq!(unmodeled_paths(&rows, CDR_BEFORE), ["cdr[].ip"]);
    }

    #[test]
    fn a_dynamic_key_map_matches_any_key() {
        let catalog = json!({
            "status": "success",
            "list_status": { "ACT": "Active", "REJ": "Rejected" }
        });
        let modeled = ["status", "list_status", "list_status.*"];
        assert!(unmodeled_paths(&catalog, &modeled).is_empty());
    }

    /// A method missing from the table silently opts out of the diff, so a
    /// regen that forgets `cargo xtask dump-fields` fails here instead.
    #[test]
    fn every_wire_method_has_modeled_paths() {
        let missing: Vec<&str> = crate::wire_methods::WIRE_METHODS
            .iter()
            .copied()
            .filter(|m| crate::response_fields::modeled_paths(m).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "no modeled paths for {missing:?}; run `cargo xtask dump-fields`"
        );
    }

    /// `modeled_paths` looks the method up by binary search.
    #[test]
    fn the_table_is_sorted_by_method() {
        let names: Vec<&str> = crate::response_fields::RESPONSE_FIELDS
            .iter()
            .map(|(name, _)| *name)
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn a_nested_object_key_is_reported_with_its_full_path() {
        let nested = json!({ "status": "success", "info": { "ip": "203.0.113.7" } });
        let modeled = ["status", "info"];
        assert_eq!(unmodeled_paths(&nested, &modeled), ["info.ip"]);
    }
}
