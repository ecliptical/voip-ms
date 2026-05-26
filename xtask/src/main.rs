//! Code generator for `src/generated.rs`.
//!
//! For each WSDL `<operation>`, emits:
//!   * A `*Params` request struct with one `Option<T>` field per WSDL input
//!     element (`api_username`/`api_password` excluded; they come from the
//!     `Client`).
//!   * A method on `Client` that calls the underlying REST endpoint.
//!
//! Run from the repository root:
//!     cargo xtask gen

mod extract;
mod field_overrides;
mod overrides;
mod response_codegen;
mod wsdl;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs, io};

use extract::Shape;
use wsdl::Wsdl;

/// Acronyms recognized when converting camelCase wire names to snake_case
/// Rust identifiers. The tokenizer tries the longest match first.
const ACRONYMS: &[&str] = &[
    "DISAs", "DIDs", "IVRs", "VPRIs", "URIs", "CDRs", "IPs", "PRIs", "DID", "IVR", "LNP", "CDR",
    "SIP", "SMS", "USA", "CAN", "FAX", "CNAM", "MMS", "DISA", "RTP", "DTMF", "ANI", "API", "PIN",
    "NAT", "URL", "CSV", "JSON", "XML", "PRI", "URI", "VPRI", "vPRI", "PDF", "POP", "IP", "TZ",
    "DST", "US", "ID",
];

/// Fields that come from the `Client`, not the per-method request struct.
const CLIENT_FIELDS: &[&str] = &["api_username", "api_password"];

fn xsd_to_rust(t: &str) -> &'static str {
    match t {
        "xsd:string" => "String",
        "xsd:integer" => "i64",
        "xsd:boolean" => "bool",
        "xsd:decimal" => "f64",
        _ => "String",
    }
}

pub(crate) fn acronyms_sorted() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = ACRONYMS.to_vec();
    v.sort_by_key(|s| std::cmp::Reverse(s.len()));
    v
}

/// A single token produced by [`tokenize`]. Acronyms preserve their
/// canonical (mixed/upper) casing from [`ACRONYMS`] so PascalCase
/// emission can reuse it verbatim; ordinary words are stored as
/// lowercase fragments.
#[derive(Debug, Clone)]
pub(crate) enum Token {
    Acronym(&'static str),
    Word(String),
}

impl Token {
    fn lowercase(&self) -> String {
        match self {
            Token::Acronym(a) => a.to_ascii_lowercase(),
            Token::Word(w) => w.clone(),
        }
    }

    fn pascal(&self) -> String {
        match self {
            Token::Acronym(a) => (*a).to_string(),
            Token::Word(w) => {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            }
        }
    }
}

/// Locate a canonical acronym whose lowercase form equals `lower`.
/// Iterates `acronyms` in caller-provided order so longest-first
/// preference (set up by [`acronyms_sorted`]) wins on ties between
/// `vPRI`/`VPRI`-style variants.
fn acronym_for_lower(acronyms: &[&'static str], lower: &str) -> Option<&'static str> {
    acronyms
        .iter()
        .copied()
        .find(|a| a.eq_ignore_ascii_case(lower) && a.len() == lower.len())
}

/// Try to decompose a lowercase fragment into a chain of one or more
/// acronyms (longest-first, case-insensitive). Returns `Some(chain)`
/// only when the entire string is consumed by acronyms, with no
/// leftover characters — that conservative rule avoids false positives
/// like turning `users` into `US`+`ers`.
pub(crate) fn decompose_into_acronyms(
    acronyms: &[&'static str],
    lower: &str,
) -> Option<Vec<&'static str>> {
    if lower.is_empty() {
        return None;
    }

    let mut chain = Vec::new();
    let mut i = 0;
    while i < lower.len() {
        let rest = &lower[i..];
        let found = acronyms
            .iter()
            .copied()
            .find(|a| a.len() <= rest.len() && a.eq_ignore_ascii_case(&rest[..a.len()]));

        match found {
            Some(a) => {
                chain.push(a);
                i += a.len();
            }
            None => return None,
        }
    }

    Some(chain)
}

fn tokenize(s: &str, acronyms: &[&'static str]) -> Vec<Token> {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, tokens: &mut Vec<Token>| {
        if cur.is_empty() {
            return;
        }

        let taken = std::mem::take(cur);
        if let Some(a) = acronym_for_lower(acronyms, &taken) {
            tokens.push(Token::Acronym(a));
        } else if let Some(chain) = decompose_into_acronyms(acronyms, &taken) {
            for a in chain {
                tokens.push(Token::Acronym(a));
            }
        } else {
            tokens.push(Token::Word(taken));
        }
    };

    let mut i = 0;
    while i < n {
        let rest = &s[i..];
        if let Some(a) = acronyms.iter().copied().find(|a| rest.starts_with(*a)) {
            flush(&mut cur, &mut tokens);
            tokens.push(Token::Acronym(a));
            i += a.len();
            continue;
        }
        let c = bytes[i] as char;
        if c == '_' || c == '-' {
            flush(&mut cur, &mut tokens);
        } else if c.is_ascii_uppercase() {
            flush(&mut cur, &mut tokens);
            cur.push(c.to_ascii_lowercase());
        } else {
            cur.push(c);
        }
        i += 1;
    }

    flush(&mut cur, &mut tokens);
    tokens
}

pub(crate) fn camel_to_snake(s: &str, acronyms: &[&'static str]) -> String {
    tokenize(s, acronyms)
        .iter()
        .map(Token::lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

pub(crate) fn camel_to_pascal(s: &str, acronyms: &[&'static str]) -> String {
    tokenize(s, acronyms).iter().map(Token::pascal).collect()
}

/// `type` is a Rust keyword; rename to `r#type`.
fn rust_field_name(name: &str) -> String {
    if name == "type" {
        "r#type".to_string()
    } else {
        name.to_string()
    }
}

fn emit(
    wsdl: &Wsdl,
    responses: &BTreeMap<String, Shape>,
    table: &field_overrides::Table,
    enum_decls: &str,
) -> String {
    let acronyms = acronyms_sorted();
    let mut out = String::new();
    out.push_str(
        "// @generated by xtask from tools/server.wsdl + tools/api-responses.json.\n\
         // DO NOT EDIT — regenerate with `cargo xtask gen`.\n\
         \n\
         #![allow(clippy::too_many_arguments)]\n\
         #![allow(non_snake_case)]\n\
         \n\
         use serde::Serialize;\n\
         use serde_json::Value;\n\
         \n\
         use crate::client::Client;\n\
         use crate::error::Result;\n",
    );

    out.push_str(enum_decls);

    for op in &wsdl.operations {
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let input_name = format!("{op}Input");
        let empty = Vec::new();
        let fields = wsdl.types.get(&input_name).unwrap_or(&empty);

        out.push_str(&format!(
            "\n/// Parameters for [`Client::{}`] (wire method `{op}`).\n",
            camel_to_snake(op, &acronyms),
        ));
        out.push_str("#[derive(Debug, Default, Clone, Serialize)]\n");
        let body_fields: Vec<&(String, String)> = fields
            .iter()
            .filter(|(n, _)| !CLIENT_FIELDS.contains(&n.as_str()))
            .collect();
        if body_fields.is_empty() {
            out.push_str(&format!("pub struct {struct_name} {{}}\n"));
            continue;
        }
        out.push_str(&format!("pub struct {struct_name} {{\n"));
        for (fname, ftype) in body_fields {
            let rust_ty = match table.get(fname) {
                Some(o) => o.rust_type.clone(),
                None => xsd_to_rust(ftype).to_string(),
            };
            let ident = rust_field_name(fname);
            out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n");
            out.push_str(&format!("    pub {ident}: Option<{rust_ty}>,\n"));
        }
        out.push_str("}\n");
    }

    out.push_str(&response_codegen::emit_response_structs(
        &wsdl.operations,
        responses,
        table,
    ));

    out.push_str("\nimpl Client {\n");
    for op in &wsdl.operations {
        let method = camel_to_snake(op, &acronyms);
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let response_name = format!("{}Response", camel_to_pascal(op, &acronyms));
        out.push_str(&format!(
            "    /// Call the `{op}` API method and deserialize into [`{response_name}`].\n    \
             pub async fn {method}(&self, params: &{struct_name}) -> Result<{response_name}> {{\n        \
                 self.call(\"{op}\", params).await\n    \
             }}\n\n\
             /// Call the `{op}` API method and return the raw JSON envelope.\n    \
             pub async fn {method}_raw(&self, params: &{struct_name}) -> Result<Value> {{\n        \
                 self.call_raw(\"{op}\", params).await\n    \
             }}\n\n"
        ));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
        .to_path_buf()
}

/// Snake-cased name used for the per-enum `deserialize_opt_*` helper
/// emitted into `generated.rs`.
fn enum_deserializer_path(enum_name: &str) -> String {
    let acronyms = acronyms_sorted();
    format!("deserialize_opt_{}", camel_to_snake(enum_name, &acronyms))
}

/// Emit Rust enum declarations (plus their (de)serializer helpers) for
/// every user-defined enum in the overrides JSON.
fn emit_enums(enums: &std::collections::HashMap<String, overrides::EnumDef>) -> String {
    let mut names: Vec<&String> = enums.keys().collect();
    names.sort();
    let mut out = String::new();
    for name in names {
        let def = &enums[name];
        out.push('\n');
        if let Some(doc) = &def.doc {
            for line in doc.lines() {
                out.push_str(&format!("/// {line}\n"));
            }
        } else {
            out.push_str(&format!(
                "/// Voip.ms `{name}` enum. Variants are documented values; any\n\
                 /// unrecognized wire string is preserved verbatim in [`{name}::Unknown`].\n",
            ));
        }
        out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
        out.push_str(&format!("pub enum {name} {{\n"));
        for v in &def.variants {
            if let Some(doc) = &v.doc {
                for line in doc.lines() {
                    out.push_str(&format!("    /// {line}\n"));
                }
            }
            out.push_str(&format!("    {},\n", v.name));
        }
        out.push_str("    /// Any wire value this crate doesn't recognize.\n");
        out.push_str("    Unknown(String),\n");
        out.push_str("}\n\n");

        // as_wire
        out.push_str(&format!("impl {name} {{\n"));
        out.push_str("    /// The wire string for this variant.\n");
        out.push_str("    pub fn as_wire(&self) -> &str {\n");
        out.push_str("        match self {\n");
        for v in &def.variants {
            out.push_str(&format!(
                "            {name}::{} => {:?},\n",
                v.name, v.wire
            ));
        }
        out.push_str(&format!("            {name}::Unknown(s) => s.as_str(),\n"));
        out.push_str("        }\n    }\n\n");
        out.push_str("    /// Parse a wire string. Unknown values are preserved.\n");
        out.push_str("    pub fn from_wire(s: &str) -> Self {\n");
        out.push_str("        match s {\n");
        for v in &def.variants {
            out.push_str(&format!(
                "            {:?} => {name}::{},\n",
                v.wire, v.name
            ));
        }
        out.push_str(&format!(
            "            other => {name}::Unknown(other.to_string()),\n"
        ));
        out.push_str("        }\n    }\n}\n\n");

        // Display
        out.push_str(&format!(
            "impl std::fmt::Display for {name} {{\n    \
                 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        \
                     f.write_str(self.as_wire())\n    \
                 }}\n\
             }}\n\n"
        ));

        // Serialize
        out.push_str(&format!(
            "impl serde::Serialize for {name} {{\n    \
                 fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {{\n        \
                     s.serialize_str(self.as_wire())\n    \
                 }}\n\
             }}\n\n"
        ));

        // Deserialize
        out.push_str(&format!(
            "impl<'de> serde::Deserialize<'de> for {name} {{\n    \
                 fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {{\n        \
                     let s = <String as serde::Deserialize>::deserialize(d)?;\n        \
                     Ok({name}::from_wire(&s))\n    \
                 }}\n\
             }}\n\n"
        ));

        // deserialize_opt helper
        let helper = enum_deserializer_path(name);
        out.push_str(&format!(
            "pub(crate) fn {helper}<'de, D>(d: D) -> std::result::Result<Option<{name}>, D::Error>\n\
             where D: serde::Deserializer<'de> {{\n    \
                 let opt = <Option<String> as serde::Deserialize>::deserialize(d)?;\n    \
                 Ok(opt.and_then(|s| {{\n        \
                     let t = s.trim();\n        \
                     if t.is_empty() {{ None }} else {{ Some({name}::from_wire(t)) }}\n    \
                 }}))\n\
             }}\n"
        ));
    }
    out
}

fn cmd_gen() -> Result<(), String> {
    let root = repo_root();
    let wsdl_path = root.join("tools").join("server.wsdl");
    let responses_path = root.join("tools").join("api-responses.json");
    let overrides_path = root.join("tools").join("api-response-overrides.json");
    let out_path = root.join("src").join("generated.rs");

    let text =
        fs::read_to_string(&wsdl_path).map_err(|e| format!("read {}: {e}", wsdl_path.display()))?;
    let wsdl = wsdl::parse_wsdl(&text)?;

    // Sanity checks (warnings only, mirror gen.py).
    let missing: Vec<&String> = wsdl
        .operations
        .iter()
        .filter(|op| !wsdl.types.contains_key(&format!("{op}Input")))
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "warning: {} operations missing input type: {:?}",
            missing.len(),
            &missing[..missing.len().min(5)],
        );
    }

    let mut unknown = BTreeSet::new();
    for op in &wsdl.operations {
        if let Some(fields) = wsdl.types.get(&format!("{op}Input")) {
            for (_, t) in fields {
                if !matches!(
                    t.as_str(),
                    "xsd:string" | "xsd:integer" | "xsd:boolean" | "xsd:decimal"
                ) {
                    unknown.insert(t.clone());
                }
            }
        }
    }
    if !unknown.is_empty() {
        eprintln!("warning: unmapped XSD types: {unknown:?}");
    }

    let overrides_doc = overrides::load(&overrides_path)?;
    overrides_doc.check_version()?;

    let responses = load_response_shapes(&responses_path, &overrides_doc, &wsdl)?;

    // Build the field-name override table by combining the built-in
    // routing entries with anything declared in `field_types`.
    let mut table = field_overrides::Table::with_builtins();
    for (field, enum_name) in &overrides_doc.field_types {
        if !overrides_doc.enums.contains_key(enum_name) {
            return Err(format!(
                "field_types maps `{field}` to unknown enum `{enum_name}`"
            ));
        }
        let deser = enum_deserializer_path(enum_name);
        table.insert(
            field.clone(),
            field_overrides::FieldOverride {
                rust_type: enum_name.clone(),
                response_deserializer: Some(deser),
            },
        );
    }

    let enum_decls = emit_enums(&overrides_doc.enums);
    let rendered = emit(&wsdl, &responses, &table, &enum_decls);
    fs::write(&out_path, &rendered).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "wrote {} ({} methods, {} typed responses)",
        out_path.display(),
        wsdl.operations.len(),
        responses.len(),
    );

    match Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&out_path)
        .status()
    {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("warning: rustfmt exited with {s}; run `cargo fmt` manually"),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("warning: rustfmt not found on PATH; run `cargo fmt` manually");
        }
        Err(e) => eprintln!("warning: rustfmt failed ({e}); run `cargo fmt` manually"),
    }

    Ok(())
}

/// Read `tools/api-responses.json`, apply overrides, and return a
/// per-method `Shape` keyed by wire method name.
fn load_response_shapes(
    responses_path: &Path,
    overrides_doc: &overrides::OverridesDoc,
    wsdl: &Wsdl,
) -> Result<BTreeMap<String, Shape>, String> {
    let mut shapes: BTreeMap<String, Shape> = BTreeMap::new();
    if responses_path.exists() {
        let text = fs::read_to_string(responses_path)
            .map_err(|e| format!("read {}: {e}", responses_path.display()))?;
        let doc: extract::Document = serde_json::from_str(&text)
            .map_err(|e| format!("parse {}: {e}", responses_path.display()))?;
        for (name, value) in &doc.methods {
            let shape = Shape::from_json(value)
                .map_err(|e| format!("{name} in {}: {e}", responses_path.display()))?;
            shapes.insert(name.clone(), shape);
        }
    } else {
        eprintln!(
            "warning: {} missing — skipping typed response generation",
            responses_path.display()
        );
    }

    let known: BTreeSet<&str> = wsdl.operations.iter().map(String::as_str).collect();
    for (name, mo) in &overrides_doc.methods {
        if !known.contains(name.as_str()) {
            eprintln!("warning: overrides reference unknown method `{name}`; skipping");
            continue;
        }
        let extracted = shapes.remove(name);
        if let Some(shape) = overrides::apply(extracted, mo)? {
            shapes.insert(name.clone(), shape);
        }
    }

    Ok(shapes)
}

fn cmd_extract(args: &[String]) -> Result<(), String> {
    let html = args.first().ok_or_else(|| {
        "extract-responses requires the path to the saved API doc HTML".to_string()
    })?;

    let html_path = PathBuf::from(html);
    let out_path = repo_root().join("tools").join("api-responses.json");
    extract::cmd_extract_responses(&html_path, &out_path)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("gen");
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();
    let res = match cmd {
        "gen" => cmd_gen(),
        "extract-responses" => cmd_extract(&rest),
        other => Err(format!(
            "unknown subcommand `{other}` (expected `gen` or `extract-responses`)"
        )),
    };

    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
