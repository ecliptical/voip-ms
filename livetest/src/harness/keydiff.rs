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

use std::collections::BTreeSet;

use serde_json::Value;

/// Live key paths the modeled set does not cover, sorted and deduplicated.
///
/// `modeled` is one method's entry from the generated `RESPONSE_FIELDS` table,
/// in the override file's path grammar. Each reported path is in that grammar
/// too, so it pastes into an `additions` entry.
pub fn unmodeled_paths(raw: &Value, modeled: &[&str]) -> Vec<String> {
    let mut live = BTreeSet::new();
    collect(raw, &mut String::new(), &mut live);
    live.iter()
        .filter(|path| !modeled.iter().any(|spec| covers(spec, path)))
        .map(|path| normalize(path, modeled))
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// Walk the live envelope into the dotted path of every key it carries, with
/// `[]` marking a list element -- the same grammar the modeled table uses.
///
/// Every element of a list shares one path, since the crate models a list by one
/// element template and a key in the tenth row is as unmodeled as in the first.
/// The path is built in one reused buffer and cloned only for a path not yet
/// seen: the largest responses here repeat the same handful of keys over
/// thousands of rows, and formatting each row's keys afresh to discard them
/// again is the bulk of the work this function would otherwise do.
fn collect(value: &Value, path: &mut String, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let restore = path.len();
                if !path.is_empty() {
                    path.push('.');
                }

                path.push_str(key);
                if !out.contains(path.as_str()) {
                    out.insert(path.clone());
                }

                collect(inner, path, out);
                path.truncate(restore);
            }
        }
        Value::Array(items) => {
            let restore = path.len();
            path.push_str("[]");
            for item in items {
                collect(item, path, out);
            }

            path.truncate(restore);
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

/// One segment, where a `*` name stands for any key of a dynamic-key map.
///
/// The live segment may carry fewer `[]` than the spec: voip.ms collapses a
/// one-element list to the bare element, which is why every generated list field
/// deserializes through `deserialize_vec_from_single_or_seq`. An account holding
/// exactly one DID, message, or conference therefore reports `x.field` against a
/// modeled `x[].field`, and reading that as unmodeled would fail the run on
/// every such account. More `[]` than the spec is a genuine mismatch, since a
/// list where the crate models a scalar leaves its keys unread.
fn segment_covers(spec: &str, live: &str) -> bool {
    let (spec_name, spec_brackets) = split_brackets(spec);
    let (live_name, live_brackets) = split_brackets(live);
    (spec_name == "*" || spec_name == live_name) && spec_brackets.ends_with(live_brackets)
}

/// Put a live path back into the modeled grammar, position by position.
///
/// A live path names what one account happens to hold, and two things about it
/// are data rather than schema: a dynamic-key map's key, and whether voip.ms
/// folded a one-element list down to the bare element. Reporting either verbatim
/// yields a path `cargo xtask gen` rejects -- `list_status.ACT.foo` for the
/// first, `cdr.ip` where the table models `cdr[].ip` for the second -- so the
/// same undeclared field would print differently, and paste or not, depending on
/// how much data the account holds.
///
/// Wherever a modeled path covers the live one this far, its segment names the
/// position's schema form, `*` and `[]` included, so that segment is adopted
/// whole.
///
/// Several modeled paths can cover one position -- `cdr` names the field and
/// `cdr[].uniqueid` its element -- and the most bracketed of them wins, because
/// folding only ever drops a `[]` that the schema has.
fn normalize(path: &str, modeled: &[&str]) -> String {
    let live: Vec<&str> = path.split('.').collect();
    let mut out = live.clone();
    for (position, slot) in out.iter_mut().enumerate() {
        let covering = modeled
            .iter()
            .filter_map(|spec| {
                let spec_segments: Vec<&str> = spec.split('.').collect();
                let covers_prefix = spec_segments.len() > position
                    && spec_segments
                        .iter()
                        .zip(&live)
                        .take(position + 1)
                        .all(|(s, l)| segment_covers(s, l));
                covers_prefix.then(|| spec_segments[position])
            })
            .max_by_key(|segment| split_brackets(segment).1.len());

        if let Some(segment) = covering {
            *slot = segment;
        }
    }

    out.join(".")
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

    /// One record with the key set a live `getCDR` returns, including the two
    /// the docs' Output block omits.
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

    /// voip.ms collapses a one-element list to the bare element, so an account
    /// holding exactly one row reports `cdr.x` where the table models `cdr[].x`.
    /// Reading that as unmodeled failed the run on every single-row account.
    #[test]
    fn a_one_element_list_arriving_unwrapped_is_not_unmodeled() {
        let folded = json!({
            "status": "success",
            "cdr": { "uniqueid": "1", "date": "2026-09-21 17:40:40" }
        });
        assert!(unmodeled_paths(&folded, CDR_BEFORE).is_empty());
    }

    /// The folded form must not change what a finding is called: the same
    /// undeclared field has to print the path the overrides accept whether the
    /// account holds one row or a thousand. `overrides::tests::
    /// addition_appends_to_a_list_element` is the other half, pinning that
    /// `cdr[].ip` applies.
    #[test]
    fn a_finding_on_a_folded_list_reports_the_list_path() {
        let folded = json!({
            "status": "success",
            "cdr": { "uniqueid": "1", "ip": "203.0.113.7" }
        });
        assert_eq!(unmodeled_paths(&folded, CDR_BEFORE), ["cdr[].ip"]);
    }

    /// The reverse is a real mismatch: a list where the crate models a scalar
    /// leaves the element's keys unread.
    #[test]
    fn a_list_where_the_table_models_a_scalar_is_reported() {
        let nested = json!({ "status": "success", "cdr": [{ "uniqueid": [{ "x": "1" }] }] });
        assert_eq!(unmodeled_paths(&nested, CDR_BEFORE), ["cdr[].uniqueid[].x"]);
    }

    /// A new field under a dynamic-key map is one schema finding, not one per
    /// key the account happens to hold, and it has to paste into the overrides.
    #[test]
    fn a_new_field_under_a_map_reports_once_in_the_override_grammar() {
        let catalog = json!({
            "status": "success",
            "list_status": {
                "ACT": { "label": "Active", "added": "x" },
                "REJ": { "label": "Rejected", "added": "y" }
            }
        });
        let modeled = [
            "status",
            "list_status",
            "list_status.*",
            "list_status.*.label",
        ];
        assert_eq!(unmodeled_paths(&catalog, &modeled), ["list_status.*.added"]);
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
