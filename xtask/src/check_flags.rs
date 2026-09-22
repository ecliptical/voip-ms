//! `cargo xtask check-flags`: audit the hand-curated boolean-flag tables in
//! `field_overrides.rs` against the doc-mined parameter descriptions.
//!
//! Two directions:
//!
//! * a param whose description reads boolean-like (`1=Enable / 0=Disable` or
//!   `yes/no`) but that no override types -- a candidate for
//!   `FLAG_01_FIELDS` / `FLAG_YES_NO_FIELDS`;
//! * a flag-table entry matching no known param or response field -- dead
//!   weight left behind by a docs revision.
//!
//! Reports and exits successfully by default, since a finding needs human
//! judgment (some 0/1-valued params are enums, not flags). `--deny` makes any
//! finding a non-zero exit, which is how CI holds the tables at zero. Run it
//! after each `extract-responses` refresh.

use std::collections::BTreeSet;
use std::fs;

use crate::extract::Shape;
use crate::field_overrides::{FLAG_01_FIELDS, FLAG_YES_NO_FIELDS, Table};
use crate::{CLIENT_FIELDS, acronyms_sorted, camel_to_pascal, overrides, repo_root, wsdl};

pub fn cmd_check_flags(args: &[String]) -> Result<(), String> {
    let deny = args.iter().any(|a| a == "--deny");
    let root = repo_root();
    let wsdl_path = root.join("tools").join("server.wsdl");
    let responses_path = root.join("tools").join("api-responses.json");
    let overrides_path = root.join("tools").join("api-response-overrides.json");

    let wsdl_text =
        fs::read_to_string(&wsdl_path).map_err(|e| format!("read {}: {e}", wsdl_path.display()))?;
    let wsdl = wsdl::parse_wsdl(&wsdl_text)?;
    let overrides_doc = overrides::load(&overrides_path)?;
    let param_docs = crate::load_param_docs(&responses_path)?;

    // The same override coverage `gen` applies: built-ins plus every field
    // declared enum-typed. Only presence matters here, not the target type.
    let mut table = Table::with_builtins();
    for field in overrides_doc.field_types.keys() {
        table.insert(field.clone(), Default::default());
    }

    let acronyms = acronyms_sorted();
    let mut candidates: Vec<(String, String, &'static str, String)> = Vec::new();
    let mut known_fields: BTreeSet<String> = BTreeSet::new();
    for op in &wsdl.operations {
        let Some(fields) = wsdl.types.get(&format!("{op}Input")) else {
            continue;
        };

        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        for (fname, _) in fields {
            if CLIENT_FIELDS.contains(&fname.as_str()) {
                continue;
            }

            known_fields.insert(fname.clone());
            let path = format!("{struct_name}.{fname}");
            let covered = overrides_doc.field_type_override.contains_key(&path)
                || (table.get(fname).is_some() && !overrides_doc.field_type_skip.contains(&path));
            if covered {
                continue;
            }

            let Some(doc) = param_docs.get(op).and_then(|d| d.get(fname)) else {
                continue;
            };

            if let Some(kind) = flag_kind(doc) {
                candidates.push((op.clone(), fname.clone(), kind, doc.clone()));
            }
        }
    }

    // Response field names count toward "still in use" -- many flag entries
    // (`listened`, `urgent`, ...) appear only on the response side.
    let shapes = crate::load_response_shapes(&responses_path, &overrides_doc, &wsdl)?;
    for shape in shapes.values() {
        collect_field_names(shape, &mut known_fields);
    }

    let dead: Vec<&&str> = FLAG_01_FIELDS
        .iter()
        .chain(FLAG_YES_NO_FIELDS.iter())
        .filter(|name| !known_fields.contains(**name))
        .collect();

    if candidates.is_empty() && dead.is_empty() {
        println!("ok: flag tables and doc-mined candidates agree");
        return Ok(());
    }

    if !candidates.is_empty() {
        println!(
            "{} flag-like param(s) not typed as bool (candidates for the \
             FLAG_* tables in xtask/src/field_overrides.rs):",
            candidates.len(),
        );
        for (method, field, kind, doc) in &candidates {
            let excerpt: String = doc.chars().take(90).collect();
            println!("  {method}.{field} ({kind}): {excerpt}");
        }
    }

    if !dead.is_empty() {
        println!(
            "{} flag-table entr(ies) matching no known param or response field \
             (possibly stale):",
            dead.len(),
        );
        for name in &dead {
            println!("  {name}");
        }
    }

    if deny {
        return Err(format!(
            "{} flag candidate(s) and {} stale entr(ies); type each as a flag or an \
             enum, or remove the entry",
            candidates.len(),
            dead.len(),
        ));
    }

    Ok(())
}

/// Classify a param description as boolean-like: `yes/no` toggles, the bare
/// `Boolean: 1/0` the docs also use, or value lists offering exactly
/// `1=`/`0=` (a `2=`/`3=` alternative means a real enum, not a flag).
/// Whitespace-insensitive, so `1 = Enable` matches too.
///
/// The `Boolean: 1/0` spelling names no value, so the `1=`/`0=` rule cannot
/// see it. That is how `cnam` and `sip_traffic` stayed integers while every
/// neighboring flag was a `bool`.
fn flag_kind(doc: &str) -> Option<&'static str> {
    let squished: String = doc
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if squished.contains("yes/no") {
        return Some("yes/no");
    }

    if squished.contains("boolean:1/0") || squished.contains("boolean:0/1") {
        return Some("1/0");
    }

    if squished.contains("1=")
        && squished.contains("0=")
        && !squished.contains("2=")
        && !squished.contains("3=")
    {
        return Some("1/0");
    }

    None
}

fn collect_field_names(shape: &Shape, out: &mut BTreeSet<String>) {
    match shape {
        Shape::Object(fields) => {
            for (name, sub) in fields {
                out.insert(name.clone());
                collect_field_names(sub, out);
            }
        }
        Shape::List(inner) | Shape::Map(inner) => collect_field_names(inner, out),
        Shape::Scalar { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::flag_kind;

    #[test]
    fn reads_the_spellings_the_docs_use() {
        assert_eq!(flag_kind("CNAM for the DID (Boolean: 1/0)"), Some("1/0"));
        assert_eq!(
            flag_kind("Encrypted SIP Traffic (Boolean: 1/0)"),
            Some("1/0")
        );
        assert_eq!(
            flag_kind("Enable SMS (1 = Enable / 0 = Disable)"),
            Some("1/0")
        );
        assert_eq!(flag_kind("Record calls (yes/no)"), Some("yes/no"));

        // A third value makes it an enum, and a plain description is neither.
        assert_eq!(flag_kind("Mode (0 = off / 1 = on / 2 = auto)"), None);
        assert_eq!(flag_kind("Destination Number (Example: 5551234568)"), None);
        // `getSMS`'s `type` names what each side means, so it reads as the
        // direction enum it is rather than as a flag.
        assert_eq!(
            flag_kind("Filter SMSs by Type (Boolean: 1 = received / 0 = sent)"),
            Some("1/0"),
            "the 1=/0= rule still fires; the field_type_override is what excludes it"
        );
    }
}
