//! `cargo xtask check-types`: report a field that a method family types one
//! way to read and another way to write.
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
/// * `client.client` -- a reseller client id (`u64`) everywhere except
///   `getClients` and `getDIDsInfo`, whose parameter is documented to accept an
///   e-mail address or a sub-account name instead, so those two stay `String`.
/// * `music_on_hold.volume` -- the param is the documented `1`/`0` quiet
///   toggle, while the response reports the rendition that resulted (`mp3` /
///   `quietmp3`). Two different things sharing a name, confirmed live.
const DELIBERATE: &[&str] = &[
    "client.client",
    "music_on_hold.volume",
    "voicemail.timezone",
];

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
    for item in &file.items {
        let Item::Struct(s) = item else {
            continue;
        };

        let name = s.ident.to_string();
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

        let Fields::Named(named) = &s.fields else {
            continue;
        };

        for f in &named.named {
            let Some(ident) = &f.ident else {
                continue;
            };

            let rust_type = unwrap_option(&render_type(&f.ty));
            // A root response's payload list often shares its name with the
            // record id it holds (`GetDISAsResponse::disa` against
            // `SetDISAParams::disa`). They are not the same field, and no
            // override could make them one type.
            if is_collection(&rust_type) {
                continue;
            }

            side.entry((family.clone(), ident.to_string()))
                .or_default()
                .insert(Field {
                    struct_name: name.clone(),
                    rust_type,
                });
        }
    }

    let mut findings = 0usize;
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
        println!("ok: every get/set field pair agrees on its type");
        return Ok(());
    }

    let summary = format!(
        "{findings} field(s) typed differently to read than to write. Each forces a \
         caller who listed a record and then updated it to convert by hand; fix with a \
         field-name entry in xtask/src/field_overrides.rs, or record why the two differ \
         in this command's DELIBERATE list."
    );
    if deny {
        return Err(summary);
    }

    println!("\n{summary}");
    Ok(())
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
    fn option_is_not_the_disagreement() {
        assert_eq!(unwrap_option("Option<u64>"), "u64");
        assert_eq!(
            unwrap_option("Vec<GetDIDsInfoResponseDID>"),
            "Vec<GetDIDsInfoResponseDID>"
        );
    }
}
