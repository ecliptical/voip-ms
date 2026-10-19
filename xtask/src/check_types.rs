//! `cargo xtask check-types`: report a field that a method family types one
//! way to read and another way to write, and a request field ident typed one
//! way in one `*Params` struct and another way in the next.
//!
//! The response side is inferred from the docs' sample output and the param
//! side is declared by the WSDL, and the two disagreed on seven `subAccount`
//! fields and on `cnam` -- so a caller who listed a sub-account and then
//! updated it had to convert each one by hand. Nothing caught that, because
//! each side is internally consistent.
//!
//! This walks the emitted surface in `src/generated.rs` (what shipped, not what
//! the inputs say) and compares every `Set*Params` / `Create*Params` /
//! `Update*Params` field against the same-named field on the `Get*Response`
//! structs of the same family. A family is the method name minus its leading
//! verb, singularized, so `setSubAccount` and `getSubAccounts` share one.
//!
//! The family comparison cannot see a code that one method looks up and
//! another takes (`getProtocols` and `createSubAccount`), since the two are in
//! different families. So it also compares every `*Params` field against every
//! other `*Params` field of the same ident: a consumer that derives one
//! parameter schema per ident, or a caller passing a value from one method to
//! the next, sees each ident as one type.
//!
//! Reports and exits successfully by default, since a finding wants human
//! judgment about which of the two types is right. `--deny` makes any finding
//! a non-zero exit, which is how CI holds the surface at zero.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use syn::{Fields, Item, Type};

use crate::{acronyms_sorted, camel_to_snake, repo_root};

/// Method-name verbs whose struct writes account state.
const WRITE_VERBS: &[&str] = &["set", "create", "update"];

/// The verb whose struct reads it back.
const READ_VERB: &str = "get";

/// `family.field` pairs whose two types are meant to differ.
///
/// * `voicemail.timezone` -- strict `chrono_tz::Tz` to write and the tolerant
///   `TimezoneName` to read, because voip.ms still reports legacy zone names
///   the IANA database dropped while the crate must never send one.
/// * `music_on_hold.volume` -- the param is the documented `1`/`0` quiet
///   toggle, while the response reports the rendition that resulted (`mp3` /
///   `quietmp3`). Two different things sharing a name, confirmed live.
const DELIBERATE: &[&str] = &["music_on_hold.volume", "voicemail.timezone"];

/// `*Params` field idents meant to carry more than one type across methods.
///
/// * `folder` -- a `voip_ms::VoicemailFolder` everywhere except
///   `getFaxMessages`, whose folder is a fax folder: `SENT`, `ALL` or one a
///   caller created with `setFaxFolder`, which no fixed set of variants can
///   hold.
/// * `recording` -- a recording id everywhere except `setCallHunting`, which
///   also takes the system recording `default` and reports it back verbatim.
///   Confirmed live: a call hunting created with `recording=default` read back
///   `default`, one created with a recording id read back the id.
const DELIBERATE_PARAMS: &[&str] = &["folder", "recording"];

/// The type a field carries for comparison, with a response's
/// `voip_ms::Reported<T>` wrapper removed.
///
/// Reading `voip_ms::Reported<T>` where the param writes a bare `T` is the crate-wide
/// rule rather than an exception: a param is written and cannot receive a value
/// this crate does not understand, a response can. Comparing the wrapper
/// against the bare type would report every date field that appears on both
/// sides, which is a finding about the rule and not about the field.
fn unwrap_reported(rust_type: &str) -> &str {
    rust_type
        .strip_prefix("crate::Reported<")
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(rust_type)
}

/// One emitted field: where it sits and what it is typed as.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Field {
    struct_name: String,
    rust_type: String,
}

pub fn cmd_check_types(args: &[String]) -> Result<(), String> {
    let deny = args.iter().any(|a| a == "--deny");
    let generated = repo_root().join("src").join("generated.rs");
    let text =
        fs::read_to_string(&generated).map_err(|e| format!("read {}: {e}", generated.display()))?;
    let file = syn::parse_file(&text).map_err(|e| format!("parse {}: {e}", generated.display()))?;

    let mut writes: BTreeMap<(String, String), BTreeSet<Field>> = BTreeMap::new();
    let mut reads: BTreeMap<(String, String), BTreeSet<Field>> = BTreeMap::new();
    for (name, fields) in structs(&file) {
        let Some((verb, family)) = split_family(&name) else {
            continue;
        };

        let side = if WRITE_VERBS.contains(&verb.as_str()) {
            &mut writes
        } else if verb == READ_VERB {
            &mut reads
        } else {
            continue;
        };

        for (ident, rust_type) in fields {
            side.entry((family.clone(), ident))
                .or_default()
                .insert(Field {
                    struct_name: name.clone(),
                    rust_type,
                });
        }
    }

    let params = params_by_ident(&file);
    let mut findings = 0usize;
    for (field, typed) in mixed_params(&params, DELIBERATE_PARAMS) {
        findings += 1;
        println!("params.{field}:");
        for f in typed {
            println!("  {} -> {}", f.struct_name, f.rust_type);
        }
    }

    for ((family, field), written) in &writes {
        let Some(read) = reads.get(&(family.clone(), field.clone())) else {
            continue;
        };

        if DELIBERATE.contains(&format!("{family}.{field}").as_str()) {
            continue;
        }

        let types: BTreeSet<&str> = written
            .iter()
            .chain(read)
            .map(|f| unwrap_reported(&f.rust_type))
            .collect();
        if types.len() < 2 {
            continue;
        }

        findings += 1;
        println!("{family}.{field}:");
        for f in written.iter().chain(read) {
            println!("  {} -> {}", f.struct_name, f.rust_type);
        }
    }

    if findings == 0 {
        println!(
            "ok: every get/set field pair agrees on its type, and so does every params \
             field sharing an ident"
        );
        return Ok(());
    }

    let summary = format!(
        "{findings} field(s) typed two ways: `family.field` to read and to write, \
         `params.field` in two params structs. Each forces a caller to convert a value \
         by hand on its way from one method to another; fix with a field-name entry in \
         xtask/src/field_overrides.rs or a FIELD_IDENT_OVERRIDE rename in \
         xtask/src/main.rs, or record why the two differ in this command's DELIBERATE \
         or DELIBERATE_PARAMS list."
    );
    if deny {
        return Err(summary);
    }

    println!("\n{summary}");
    Ok(())
}

/// Every struct with named fields, as its name and its `(ident, type)` pairs
/// with `Option` removed and collection-typed fields left out.
///
/// A collection is left out because a root response's payload list often
/// shares its name with the record id it holds (`GetDISAsResponse::disa`
/// against `SetDISAParams::disa`). They are not the same field, and no
/// override could make them one type.
fn structs(file: &syn::File) -> Vec<(String, Vec<(String, String)>)> {
    file.items
        .iter()
        .filter_map(|item| {
            let Item::Struct(s) = item else {
                return None;
            };

            let Fields::Named(named) = &s.fields else {
                return None;
            };

            let fields = named
                .named
                .iter()
                .filter_map(|f| {
                    let ident = f.ident.as_ref()?.to_string();
                    Some((ident, unwrap_option(&render_type(&f.ty))))
                })
                .filter(|(_, rust_type)| !is_collection(rust_type))
                .collect();
            Some((s.ident.to_string(), fields))
        })
        .collect()
}

/// Every `*Params` field, grouped by ident.
fn params_by_ident(file: &syn::File) -> BTreeMap<String, BTreeSet<Field>> {
    let mut params: BTreeMap<String, BTreeSet<Field>> = BTreeMap::new();
    for (name, fields) in structs(file) {
        if !name.ends_with("Params") {
            continue;
        }

        for (ident, rust_type) in fields {
            params.entry(ident).or_default().insert(Field {
                struct_name: name.clone(),
                rust_type,
            });
        }
    }

    params
}

/// The idents in `params` typed more than one way, less those in `deliberate`.
fn mixed_params<'a>(
    params: &'a BTreeMap<String, BTreeSet<Field>>,
    deliberate: &[&str],
) -> Vec<(&'a str, &'a BTreeSet<Field>)> {
    params
        .iter()
        .filter(|(field, _)| !deliberate.contains(&field.as_str()))
        .filter(|(_, typed)| {
            typed
                .iter()
                .map(|f| f.rust_type.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        })
        .map(|(field, typed)| (field.as_str(), typed))
        .collect()
}

/// Split an emitted struct name into `(leading verb, family)`, where the
/// family is the rest of the method name singularized -- `SetSubAccountParams`
/// and `GetSubAccountsResponseAccount` both yield `("set"/"get",
/// "sub_account")`.
///
/// Returns `None` for a struct that is neither, and for a method name with no
/// word after its verb.
fn split_family(struct_name: &str) -> Option<(String, String)> {
    let method = struct_name.strip_suffix("Params").or_else(|| {
        struct_name
            .split("Response")
            .next()
            .filter(|s| !s.is_empty())
    })?;
    if method == struct_name && !struct_name.contains("Response") {
        return None;
    }

    let acronyms = acronyms_sorted();
    let snake = camel_to_snake(method, &acronyms);
    let mut tokens = snake.split('_');
    let verb = tokens.next()?.to_string();
    let rest: Vec<String> = tokens.map(singular).collect();
    if rest.is_empty() {
        return None;
    }

    Some((verb, rest.join("_")))
}

/// Drop a trailing plural `s` so `sub_accounts` and `sub_account` land in one
/// family. Deliberately naive: it runs on single lowercase tokens of a method
/// name, where the only plurals are regular ones.
fn singular(token: &str) -> String {
    match token.strip_suffix('s') {
        Some(stem) if !stem.is_empty() => stem.to_string(),
        _ => token.to_string(),
    }
}

/// Whether a rendered type is a list or a map, which a scalar override can
/// never stand in for.
fn is_collection(rendered: &str) -> bool {
    rendered.starts_with("Vec<") || rendered.starts_with("std::collections::HashMap<")
}

/// `Option<T>` reduced to `T`; every other type is left alone. The two sides
/// wrap differently (a param is always optional, a response list is not), and
/// the wrapper is not the disagreement being looked for.
fn unwrap_option(rendered: &str) -> String {
    rendered
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(rendered)
        .to_string()
}

/// A type as the generated source spells it. Every emitted field is a path
/// type, so anything else is rendered as `?` rather than compared.
fn render_type(ty: &Type) -> String {
    let Type::Path(path) = ty else {
        return "?".to_string();
    };

    path.path
        .segments
        .iter()
        .map(|seg| {
            let mut out = seg.ident.to_string();
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                let inner: Vec<String> = args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(render_type(t)),
                        _ => None,
                    })
                    .collect();
                out.push('<');
                out.push_str(&inner.join(", "));
                out.push('>');
            }

            out
        })
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_get_and_a_set_of_one_record_share_a_family() {
        assert_eq!(
            split_family("SetSubAccountParams"),
            Some(("set".into(), "sub_account".into()))
        );
        assert_eq!(
            split_family("CreateSubAccountParams"),
            Some(("create".into(), "sub_account".into()))
        );
        assert_eq!(
            split_family("GetSubAccountsResponseAccount"),
            Some(("get".into(), "sub_account".into()))
        );
        // The acronym tokenizer keeps the plural and the singular together.
        assert_eq!(
            split_family("SetDIDInfoParams"),
            Some(("set".into(), "did_info".into()))
        );
        assert_eq!(
            split_family("GetDIDsInfoResponseDID"),
            Some(("get".into(), "did_info".into()))
        );
    }

    #[test]
    fn a_struct_that_is_neither_is_skipped() {
        assert_eq!(split_family("NoParams"), None);
        assert_eq!(split_family("ApiStatus"), None);
    }

    #[test]
    fn a_params_ident_typed_two_ways_is_reported_unless_deliberate() {
        let file: syn::File = syn::parse_str(
            "pub struct GetCodesParams { pub code: Option<String>, pub id: Option<u64>, \
                 pub note: Option<String>, pub kind: Option<String> }
             pub struct SetThingParams { pub code: Option<u64>, pub id: Option<u64>, \
                 pub note: Option<u64>, pub kind: Option<String> }
             pub struct GetThingsResponseThing { pub id: Option<String> }",
        )
        .unwrap();
        let params = params_by_ident(&file);

        let reported: Vec<&str> = mixed_params(&params, &["note"])
            .into_iter()
            .map(|(field, _)| field)
            .collect();
        assert_eq!(reported, ["code"]);

        let reported: Vec<&str> = mixed_params(&params, &[])
            .into_iter()
            .map(|(field, _)| field)
            .collect();
        assert_eq!(reported, ["code", "note"]);
    }

    #[test]
    fn option_is_not_the_disagreement() {
        assert_eq!(unwrap_option("Option<u64>"), "u64");
        assert_eq!(
            unwrap_option("Vec<GetDIDsInfoResponseDID>"),
            "Vec<GetDIDsInfoResponseDID>"
        );
    }
}
