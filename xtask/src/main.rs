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
        let a = acronyms
            .iter()
            .copied()
            .find(|a| a.len() <= rest.len() && a.eq_ignore_ascii_case(&rest[..a.len()]))?;

        chain.push(a);
        i += a.len();
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

/// Per-method parameter descriptions, keyed by wire method name then
/// wire parameter name. Empty when no extract is present.
type ParamDocs = BTreeMap<String, BTreeMap<String, String>>;

/// Per-method one-line descriptions, keyed by wire method name. Most
/// methods carry one in the docs (~218 of 222); the rest are absent.
type MethodDocs = BTreeMap<String, String>;

/// Render a possibly multi-line method description as `///` lines at the
/// given indent, wrapping each source line independently so bullet breaks
/// are preserved.
fn render_method_doc(out: &mut String, indent: &str, text: &str) {
    for line in text.lines() {
        render_doc(out, indent, line);
    }
}

/// Wrap a description as one or more `///` lines at the given indent,
/// hard-wrapping long lines so rustfmt doesn't have to.
fn render_doc(out: &mut String, indent: &str, text: &str) {
    const WIDTH: usize = 80;
    let mut line = String::new();
    let mut flush = |line: &mut String| {
        if !line.is_empty() {
            out.push_str(&format!("{indent}/// {}\n", escape_doc_line(line)));
            line.clear();
        }
    };
    for raw_word in text.split_whitespace() {
        let word = sanitize_doc_word(raw_word);
        let prospective = if line.is_empty() {
            word.len()
        } else {
            line.len() + 1 + word.len()
        };

        if !line.is_empty() && indent.len() + 4 + prospective > WIDTH {
            flush(&mut line);
        }

        if !line.is_empty() {
            line.push(' ');
        }

        line.push_str(&word);
    }

    flush(&mut line);
}

/// Make a single word of mined doc text safe for rustdoc, which parses doc
/// comments as Markdown:
///
/// * a bare `http(s)://…` URL is wrapped as an `<…>` autolink (rustdoc
///   warns on bare URLs);
/// * `[` / `]` are backslash-escaped so prose like `[Required]` or
///   `[Optional]` isn't parsed as a (broken) shortcut intra-doc link.
///
/// URLs are checked first so their own characters aren't bracket-escaped.
fn sanitize_doc_word(word: &str) -> String {
    let trimmed = word.trim_start_matches(['(', '\'', '"']);
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        // Wrap just the URL token, preserving any leading/trailing prose
        // punctuation (quotes, parens) around it.
        let lead_len = word.len() - trimmed.len();
        let (lead, rest) = word.split_at(lead_len);
        let url_end = rest.find(['\'', '"', ')', ',']).unwrap_or(rest.len());
        let (url, trail) = rest.split_at(url_end);
        return format!("{lead}<{url}>{trail}");
    }

    if word.contains('[') || word.contains(']') {
        return word.replace('[', "\\[").replace(']', "\\]");
    }

    word.to_string()
}

/// Backslash-escape a leading Markdown list marker (`- `, `* `, `+ `, or
/// `N. `) so a wrapped doc line isn't parsed as a lazy list continuation
/// (clippy::doc_lazy_continuation). The VoIP.ms source uses these
/// characters as plain prose punctuation, not Markdown.
fn escape_doc_line(line: &str) -> String {
    let bytes = line.as_bytes();
    let starts_bullet =
        matches!(bytes.first(), Some(b'-' | b'*' | b'+')) && matches!(bytes.get(1), Some(b' '));
    let starts_ordered = {
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        digits > 0
            && matches!(bytes.get(digits), Some(b'.'))
            && matches!(bytes.get(digits + 1), Some(b' '))
    };

    if starts_bullet || starts_ordered {
        format!("\\{line}")
    } else {
        line.to_string()
    }
}

/// The PascalCase variant identifier for a wire status code, using the
/// same acronym-aware conversion as method/type names (`invalid_credentials`
/// → `InvalidCredentials`, `no_did` → `NoDID`, `api_not_enabled` →
/// `APINotEnabled`). The rare capitalized wire codes (`Invalid_threshold`)
/// lower-case fine through the tokenizer, so the variant matches its
/// lowercase-sibling form.
fn status_variant_name(code: &str, acronyms: &[&'static str]) -> String {
    camel_to_pascal(code, acronyms)
}

/// Emit the `ApiStatus` enum: one PascalCase variant per documented wire
/// code (carrying its description as a doc comment) plus an `Unknown(String)`
/// catch-all, with `as_str`/`from_wire`/`description`/`is_documented` and the
/// `Display`/`Serialize`/`Deserialize`/`From<String>` impls. The wire strings
/// are preserved verbatim (including the rare capitalized codes); only the
/// variant *identifiers* are normalized.
fn emit_statuses(statuses: &[(String, String)], empty: &BTreeSet<String>) -> String {
    if statuses.is_empty() {
        return String::new();
    }

    let acronyms = acronyms_sorted();
    let variants: Vec<(String, &String, &String)> = statuses
        .iter()
        .map(|(code, desc)| (status_variant_name(code, &acronyms), code, desc))
        .collect();

    let mut out = String::new();

    // Enum declaration.
    out.push_str(
        "\n/// A non-success `status` returned by the VoIP.ms API.\n\
         ///\n\
         /// Every documented error code from the official API docs' global\n\
         /// error-code table is a variant; [`ApiStatus::description`] returns its\n\
         /// documented meaning. The set of codes is documentation, not a stable\n\
         /// contract — a code VoIP.ms returns but hasn't documented is preserved\n\
         /// verbatim in [`ApiStatus::Unknown`] rather than lost.\n\
         ///\n\
         /// ```\n\
         /// # use voip_ms::ApiStatus;\n\
         /// let status = ApiStatus::from_wire(\"invalid_credentials\");\n\
         /// assert_eq!(status, ApiStatus::InvalidCredentials);\n\
         /// assert_eq!(status.as_str(), \"invalid_credentials\");\n\
         /// assert_eq!(status.description(), Some(\"Username or Password is incorrect\"));\n\
         /// assert!(status.is_documented());\n\
         ///\n\
         /// let unknown = ApiStatus::from_wire(\"some_new_code\");\n\
         /// assert_eq!(unknown, ApiStatus::Unknown(\"some_new_code\".to_string()));\n\
         /// assert_eq!(unknown.description(), None);\n\
         /// assert!(!unknown.is_documented());\n\
         /// ```\n\
         #[derive(Debug, Clone, PartialEq, Eq, Hash)]\n\
         pub enum ApiStatus {\n",
    );
    for (variant, code, desc) in &variants {
        out.push_str(&format!("    /// `{code}` — {desc}\n"));
        out.push_str(&format!("    {variant},\n"));
    }
    out.push_str("    /// A `status` value not present in the documented table,\n");
    out.push_str("    /// preserved verbatim.\n");
    out.push_str("    Unknown(String),\n");
    out.push_str("}\n\n");

    out.push_str("impl ApiStatus {\n");

    // as_str
    out.push_str("    /// The verbatim wire `status` string.\n");
    out.push_str("    pub fn as_str(&self) -> &str {\n");
    out.push_str("        match self {\n");
    for (variant, code, _) in &variants {
        out.push_str(&format!("            ApiStatus::{variant} => {code:?},\n"));
    }
    out.push_str("            ApiStatus::Unknown(s) => s.as_str(),\n");
    out.push_str("        }\n    }\n\n");

    // from_wire
    out.push_str("    /// Parse a wire `status` string. Unknown values are preserved\n");
    out.push_str("    /// in [`ApiStatus::Unknown`].\n");
    out.push_str("    pub fn from_wire(s: &str) -> Self {\n");
    out.push_str("        match s {\n");
    for (variant, code, _) in &variants {
        out.push_str(&format!("            {code:?} => ApiStatus::{variant},\n"));
    }
    out.push_str("            other => ApiStatus::Unknown(other.to_string()),\n");
    out.push_str("        }\n    }\n\n");

    // description
    out.push_str("    /// The human-readable description of this status from the\n");
    out.push_str("    /// VoIP.ms docs, or `None` for [`ApiStatus::Unknown`].\n");
    out.push_str("    pub fn description(&self) -> Option<&'static str> {\n");
    out.push_str("        match self {\n");
    for (variant, _, desc) in &variants {
        out.push_str(&format!(
            "            ApiStatus::{variant} => Some({desc:?}),\n"
        ));
    }
    out.push_str("            ApiStatus::Unknown(_) => None,\n");
    out.push_str("        }\n    }\n\n");

    // is_documented
    out.push_str("    /// Whether this status is a documented code (not\n");
    out.push_str("    /// [`ApiStatus::Unknown`]).\n");
    out.push_str("    pub fn is_documented(&self) -> bool {\n");
    out.push_str("        !matches!(self, ApiStatus::Unknown(_))\n");
    out.push_str("    }\n\n");

    // is_empty
    let empty_variants: Vec<&String> = variants
        .iter()
        .filter(|(_, code, _)| empty.contains(*code))
        .map(|(variant, _, _)| variant)
        .collect();
    out.push_str("    /// Whether this status means \"the requested collection is empty,\"\n");
    out.push_str("    /// rather than a failure. VoIP.ms returns a distinct `no_*` status\n");
    out.push_str("    /// for each list method when the list has no entries; the typed\n");
    out.push_str("    /// `Client` methods treat such a status as a successful empty\n");
    out.push_str("    /// response (collection fields deserialize to `None`) instead of an\n");
    out.push_str("    /// [`crate::Error::Api`], while the `*_raw` methods still surface it\n");
    out.push_str("    /// verbatim. Codes that look like `no_*` but signal a real failure\n");
    out.push_str("    /// (`no_base64file`, `no_callstatus`, `no_provision`, ...) are not\n");
    out.push_str("    /// included.\n");
    out.push_str("    pub fn is_empty(&self) -> bool {\n");
    if empty_variants.is_empty() {
        out.push_str("        false\n");
    } else {
        out.push_str("        matches!(\n            self,\n");
        let arms: Vec<String> = empty_variants
            .iter()
            .map(|v| format!("            ApiStatus::{v}"))
            .collect();
        out.push_str(&arms.join("\n                | "));
        out.push_str("\n        )\n");
    }
    out.push_str("    }\n}\n\n");

    // Display
    out.push_str(
        "impl std::fmt::Display for ApiStatus {\n    \
             fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        \
                 f.write_str(self.as_str())\n    \
             }\n\
         }\n\n",
    );

    // From<String> / From<&str> — keep the prior `ApiStatus::from(String)`
    // ergonomics working against the new enum.
    out.push_str(
        "impl From<String> for ApiStatus {\n    \
             fn from(s: String) -> Self {\n        \
                 ApiStatus::from_wire(&s)\n    \
             }\n\
         }\n\n\
         impl From<&str> for ApiStatus {\n    \
             fn from(s: &str) -> Self {\n        \
                 ApiStatus::from_wire(s)\n    \
             }\n\
         }\n\n",
    );

    // Serialize
    out.push_str(
        "impl serde::Serialize for ApiStatus {\n    \
             fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {\n        \
                 s.serialize_str(self.as_str())\n    \
             }\n\
         }\n\n",
    );

    // Deserialize
    out.push_str(
        "impl<'de> serde::Deserialize<'de> for ApiStatus {\n    \
             fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {\n        \
                 let s = <String as serde::Deserialize>::deserialize(d)?;\n        \
                 Ok(ApiStatus::from_wire(&s))\n    \
             }\n\
         }\n",
    );

    out
}

#[allow(clippy::too_many_arguments)]
fn emit(
    wsdl: &Wsdl,
    responses: &BTreeMap<String, Shape>,
    param_docs: &ParamDocs,
    method_docs: &MethodDocs,
    table: &field_overrides::Table,
    field_type_skip: &BTreeSet<String>,
    field_type_override: &BTreeMap<String, field_overrides::FieldOverride>,
    enum_decls: &str,
    statuses: &[(String, String)],
    empty_statuses: &BTreeSet<String>,
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
    out.push_str(&emit_statuses(statuses, empty_statuses));

    for op in &wsdl.operations {
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let input_name = format!("{op}Input");
        let empty = Vec::new();
        let fields = wsdl.types.get(&input_name).unwrap_or(&empty);

        out.push('\n');
        if let Some(desc) = method_docs.get(op) {
            render_method_doc(&mut out, "", desc);
            out.push_str("///\n");
        }
        out.push_str(&format!(
            "/// Parameters for [`Client::{}`] (wire method `{op}`).\n",
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
        let docs = param_docs.get(op);
        for (fname, ftype) in body_fields {
            // A per-struct `field_type_override` wins outright; otherwise the
            // name-based table applies, unless `field_type_skip` suppresses it
            // for this same-named-but-unrelated field (keeping the WSDL type).
            let path = format!("{struct_name}.{fname}");
            let override_ = if let Some(o) = field_type_override.get(&path) {
                Some(o)
            } else if field_type_skip.contains(&path) {
                None
            } else {
                table.get(fname)
            };
            let rust_ty = match override_ {
                Some(o) => o.rust_type.clone(),
                None => xsd_to_rust(ftype).to_string(),
            };

            let ident = rust_field_name(fname);
            if let Some(desc) = docs.and_then(|d| d.get(fname)) {
                render_doc(&mut out, "    ", desc);
            }

            // A `param_skip_if` override emits the field unwrapped (plain `T`,
            // skipped at its default); otherwise it's `Option<T>` skipped when
            // `None`. A `param_serializer` supplies the wire form for a type
            // whose own `Serialize` is wrong (a `bool` flag wanting `1`/`0`).
            let param_serializer = override_.and_then(|o| o.param_serializer.as_deref());
            match override_.and_then(|o| o.param_skip_if.as_deref()) {
                Some(skip_if) => {
                    out.push_str(&format!("    #[serde(skip_serializing_if = \"{skip_if}\""));
                    if let Some(ser) = param_serializer {
                        out.push_str(&format!(", serialize_with = \"{ser}\""));
                    }
                    out.push_str(")]\n");
                    out.push_str(&format!("    pub {ident}: {rust_ty},\n"));
                }
                None => {
                    out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\"");
                    if let Some(ser) = param_serializer {
                        out.push_str(&format!(", serialize_with = \"{ser}\""));
                    }
                    out.push_str(")]\n");
                    out.push_str(&format!("    pub {ident}: Option<{rust_ty}>,\n"));
                }
            }
        }
        out.push_str("}\n");
    }

    out.push_str(&response_codegen::emit_response_structs(
        &wsdl.operations,
        responses,
        table,
        field_type_skip,
        field_type_override,
    ));

    out.push_str("\nimpl Client {\n");
    for op in &wsdl.operations {
        let method = camel_to_snake(op, &acronyms);
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let response_name = format!("{}Response", camel_to_pascal(op, &acronyms));
        if let Some(desc) = method_docs.get(op) {
            render_method_doc(&mut out, "    ", desc);
            out.push_str("    ///\n");
        }
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

        out.push_str("#[derive(Debug, Clone, PartialEq, Eq, Hash)]\n");
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

        // Deserialize -- tolerant of string / number / bool wire forms.
        out.push_str(&format!(
            "impl<'de> serde::Deserialize<'de> for {name} {{\n    \
                 fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {{\n        \
                     let s = crate::responses::deserialize_enum_wire_string(d)?;\n        \
                     Ok({name}::from_wire(&s))\n    \
                 }}\n\
             }}\n\n"
        ));

        // deserialize_opt helper -- same tolerance, empty / absent -> None.
        // `allow(dead_code)`: emitted for every enum, but a param-only enum
        // (used only via `Serialize`) never references its response helper.
        let helper = enum_deserializer_path(name);
        out.push_str(&format!(
            "#[allow(dead_code)]\n\
             pub(crate) fn {helper}<'de, D>(d: D) -> std::result::Result<Option<{name}>, D::Error>\n\
             where D: serde::Deserializer<'de> {{\n    \
                 let opt = crate::responses::deserialize_opt_string_from_string_number_or_bool(d)?;\n    \
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
    let param_docs = load_param_docs(&responses_path)?;
    let method_docs = load_method_docs(&responses_path)?;

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
                ..Default::default()
            },
        );
    }

    // `"StructName.field"` paths where the name-based override table is
    // suppressed for one struct (a same-named-but-unrelated field).
    let field_type_skip: BTreeSet<String> = overrides_doc.field_type_skip.iter().cloned().collect();
    for entry in &field_type_skip {
        let field = entry
            .rsplit_once('.')
            .map(|(_, f)| f)
            .filter(|f| !f.is_empty())
            .ok_or_else(|| format!("field_type_skip entry `{entry}` must be `StructName.field`"))?;
        if table.get(field).is_none() {
            return Err(format!(
                "field_type_skip entry `{entry}` names field `{field}`, which has no override to skip"
            ));
        }
    }

    // `"StructName.field" -> EnumName`: assign one struct's field a specific
    // enum type, overriding the inferred type and any name-based `field_types`.
    let mut field_type_override: BTreeMap<String, field_overrides::FieldOverride> = BTreeMap::new();
    for (path, enum_name) in &overrides_doc.field_type_override {
        if path
            .rsplit_once('.')
            .filter(|(_, f)| !f.is_empty())
            .is_none()
        {
            return Err(format!(
                "field_type_override key `{path}` must be `StructName.field`"
            ));
        }
        if !overrides_doc.enums.contains_key(enum_name) {
            return Err(format!(
                "field_type_override `{path}` maps to unknown enum `{enum_name}`"
            ));
        }
        field_type_override.insert(
            path.clone(),
            field_overrides::FieldOverride {
                rust_type: enum_name.clone(),
                response_deserializer: Some(enum_deserializer_path(enum_name)),
                ..Default::default()
            },
        );
    }

    let statuses = load_statuses(&root.join("tools").join("api-statuses.json"))?;

    // An empty-status code that no longer appears in the status table (a
    // typo, or a code the docs dropped) would silently never match, so fail
    // loudly instead.
    let empty_statuses: BTreeSet<String> = overrides_doc.empty_statuses.iter().cloned().collect();
    let known: BTreeSet<&str> = statuses.iter().map(|(code, _)| code.as_str()).collect();
    for code in &empty_statuses {
        if !known.contains(code.as_str()) {
            return Err(format!(
                "empty_statuses references unknown status code `{code}`"
            ));
        }
    }

    let enum_decls = emit_enums(&overrides_doc.enums);
    let rendered = emit(
        &wsdl,
        &responses,
        &param_docs,
        &method_docs,
        &table,
        &field_type_skip,
        &field_type_override,
        &enum_decls,
        &statuses,
        &empty_statuses,
    );
    fs::write(&out_path, &rendered).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    println!(
        "wrote {} ({} methods, {} method descriptions, {} typed responses, \
         {} status codes)",
        out_path.display(),
        wsdl.operations.len(),
        method_docs.len(),
        responses.len(),
        statuses.len(),
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

/// Read the `param_docs` section of `tools/api-responses.json`. Missing
/// file or missing section yields an empty map — doc comments are
/// purely additive, so codegen proceeds without them.
fn load_param_docs(responses_path: &Path) -> Result<ParamDocs, String> {
    if !responses_path.exists() {
        return Ok(ParamDocs::new());
    }

    let text = fs::read_to_string(responses_path)
        .map_err(|e| format!("read {}: {e}", responses_path.display()))?;
    let doc: extract::Document = serde_json::from_str(&text)
        .map_err(|e| format!("parse {}: {e}", responses_path.display()))?;

    let mut out = ParamDocs::new();
    for (method, value) in &doc.param_docs {
        let Some(obj) = value.as_object() else {
            continue;
        };

        let inner: BTreeMap<String, String> = obj
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();
        if !inner.is_empty() {
            out.insert(method.clone(), inner);
        }
    }

    Ok(out)
}

/// Read the `method_docs` section of `tools/api-responses.json`. Missing
/// file or section yields an empty map — these doc comments are additive.
fn load_method_docs(responses_path: &Path) -> Result<MethodDocs, String> {
    if !responses_path.exists() {
        return Ok(MethodDocs::new());
    }

    let text = fs::read_to_string(responses_path)
        .map_err(|e| format!("read {}: {e}", responses_path.display()))?;
    let doc: extract::Document = serde_json::from_str(&text)
        .map_err(|e| format!("parse {}: {e}", responses_path.display()))?;

    Ok(doc
        .method_docs
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect())
}

/// Read `tools/api-statuses.json` into ordered `(code, description)`
/// pairs. A missing file yields an empty list — status constants are
/// additive, so codegen proceeds without them (with a warning).
fn load_statuses(path: &Path) -> Result<Vec<(String, String)>, String> {
    if !path.exists() {
        eprintln!(
            "warning: {} missing — skipping status-code generation",
            path.display()
        );
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let doc: extract::StatusDocument =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;

    // Guard against duplicate variant names — two distinct wire codes that
    // collapse to the same PascalCase identifier would fail to compile.
    let acronyms = acronyms_sorted();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for entry in &doc.statuses {
        let ident = status_variant_name(&entry.code, &acronyms);
        if let Some(prev) = seen.insert(ident.clone(), entry.code.clone()) {
            return Err(format!(
                "duplicate status variant `{ident}` (from codes `{prev}` and `{}`)",
                entry.code
            ));
        }
    }

    Ok(doc
        .statuses
        .into_iter()
        .map(|e| (e.code, e.description))
        .collect())
}

fn cmd_extract(args: &[String]) -> Result<(), String> {
    let html = args.first().ok_or_else(|| {
        "extract-responses requires the path to the saved API doc HTML".to_string()
    })?;

    let html_path = PathBuf::from(html);
    let out_path = repo_root().join("tools").join("api-responses.json");
    extract::cmd_extract_responses(&html_path, &out_path)
}

fn cmd_extract_statuses(args: &[String]) -> Result<(), String> {
    let html = args.first().ok_or_else(|| {
        "extract-statuses requires the path to the saved API doc HTML".to_string()
    })?;

    let html_path = PathBuf::from(html);
    let out_path = repo_root().join("tools").join("api-statuses.json");
    extract::cmd_extract_statuses(&html_path, &out_path)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("gen");
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();
    let res = match cmd {
        "gen" => cmd_gen(),
        "extract-responses" => cmd_extract(&rest),
        "extract-statuses" => cmd_extract_statuses(&rest),
        other => Err(format!(
            "unknown subcommand `{other}` \
             (expected `gen`, `extract-responses`, or `extract-statuses`)"
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
