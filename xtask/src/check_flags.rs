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
//! Purely advisory: findings need human judgment (some 0/1-valued params are
//! enums, not flags), so the command reports and exits successfully either
//! way. Run it after each `extract-responses` refresh.

use std::collections::BTreeSet;
use std::fs;

use crate::extract::Shape;
use crate::field_overrides::{FLAG_01_FIELDS, FLAG_YES_NO_FIELDS, Table};
use crate::{CLIENT_FIELDS, acronyms_sorted, camel_to_pascal, overrides, repo_root, wsdl};

pub fn cmd_check_flags() -> Result<(), String> {
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

    Ok(())
}

/// Classify a param description as boolean-like: `yes/no` toggles, or value
/// lists offering exactly `1=`/`0=` (a `2=`/`3=` alternative means a real
/// enum, not a flag). Whitespace-insensitive, so `1 = Enable` matches too.
fn flag_kind(doc: &str) -> Option<&'static str> {
    let squished: String = doc
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if squished.contains("yes/no") {
        return Some("yes/no");
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
