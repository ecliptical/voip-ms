//! Walks merged response `Shape` trees and emits typed `*Response`
//! structs into `src/generated.rs`.
//!
//! Naming:
//! * Top-level type for method `getBalance` → `GetBalanceResponse`.
//! * Nested object types are named by path: the `balance` sub-object of
//!   `getBalance` becomes `GetBalanceResponseBalance`.
//! * List elements drop a trailing plural `s`/`es` where it doesn't
//!   collide: the elements of `dids: [...]` in `getDIDsInfo` become
//!   `GetDIDsInfoResponseDid`.
//!
//! All fields are `Option<T>` so unexpected omissions don't fail
//! deserialization. Scalar types map to:
//!
//! | Inferred type | Rust type        | Deserializer                                       |
//! |---------------|------------------|----------------------------------------------------|
//! | `string`      | `String`         | (default)                                          |
//! | `integer`     | `u64`            | `deserialize_opt_u64_from_string_or_number`        |
//! | `decimal`     | `rust_decimal::Decimal` | `deserialize_opt_decimal_from_string_or_number` |
//! | `bool_yn` / `bool_01` | `bool`   | `deserialize_opt_bool_from_string_number_or_yn`    |
//! | `date`        | `chrono::NaiveDate`     | `deserialize_opt_date`                      |
//! | `datetime`    | `chrono::NaiveDateTime` | `deserialize_opt_datetime`                  |

use std::collections::BTreeMap;

use crate::extract::{ScalarTy, Shape};
use crate::{acronyms_sorted, camel_to_pascal};

/// Render all `*Response` structs for the methods in `responses`.
///
/// `method_names` enumerates methods in the canonical (WSDL) order so
/// the output is stable across runs.
pub fn emit_response_structs(
    method_names: &[String],
    responses: &BTreeMap<String, Shape>,
) -> String {
    let acronyms = acronyms_sorted();
    let mut out = String::new();
    for op in method_names {
        let Some(shape) = responses.get(op) else {
            continue;
        };

        let pascal = camel_to_pascal(op, &acronyms);
        let root = format!("{pascal}Response");
        let mut emitter = Emitter::new();
        emitter.emit_struct(&root, shape);

        out.push_str(&format!(
            "\n/// Response body for [`Client::{}_typed`] (wire method `{op}`).\n",
            crate::camel_to_snake(op, &acronyms),
        ));
        out.push_str(&emitter.into_text());
    }

    out
}

struct Emitter {
    /// Structs emitted in dependency-friendly order (children appended
    /// before any later sibling that references them).
    structs: Vec<String>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            structs: Vec::new(),
        }
    }

    fn into_text(self) -> String {
        self.structs.join("\n")
    }

    /// Emit a struct named `name` whose body comes from `shape`.
    /// `shape` should be either an Object (record) or a List (top-level
    /// list — wraps as `{ items: Vec<…> }`); a scalar at the root is
    /// promoted into a single-field record `{ value: T }`.
    fn emit_struct(&mut self, name: &str, shape: &Shape) {
        match shape {
            Shape::Object(fields) => self.emit_record(name, fields),
            Shape::List(inner) => {
                let inner_ty = self.field_type(name, "items", inner);
                let body = format!(
                    "#[derive(Debug, Clone, Default, serde::Deserialize)]\n\
                     pub struct {name} {{\n    \
                         #[serde(default)]\n    \
                         pub items: Vec<{inner_ty}>,\n\
                     }}\n",
                );
                self.structs.push(body);
            }
            Shape::Scalar { .. } => {
                let inner_ty = self.scalar_rust_type(shape);
                let deser = scalar_deserializer(shape);
                let attrs = render_field_attrs(deser);
                let body = format!(
                    "#[derive(Debug, Clone, Default, serde::Deserialize)]\n\
                     pub struct {name} {{\n\
                         {attrs}    pub value: Option<{inner_ty}>,\n\
                     }}\n",
                );
                self.structs.push(body);
            }
        }
    }

    fn emit_record(&mut self, name: &str, fields: &[(String, Shape)]) {
        // The PHP `print_r` source occasionally emits the same key twice
        // at the same nesting level (typically a `[status]` shown for
        // both a generic and a specific success state). Dedupe by name,
        // keeping the first occurrence — the second is virtually always
        // a duplicate sample of the same field.
        let mut seen = std::collections::HashSet::new();
        let mut deduped: Vec<&(String, Shape)> = Vec::with_capacity(fields.len());
        for entry in fields {
            if seen.insert(entry.0.as_str()) {
                deduped.push(entry);
            }
        }

        let mut body = String::new();
        body.push_str("#[derive(Debug, Clone, Default, serde::Deserialize)]\n");
        body.push_str(&format!("pub struct {name} {{\n"));
        for (fname, sub) in deduped {
            let rust_ident = rust_field_ident(fname);
            let rust_ty = self.field_type(name, fname, sub);
            let deser = scalar_deserializer(sub);
            let attrs = render_field_attrs(deser);
            if rust_ident == *fname {
                body.push_str(&attrs);
                body.push_str(&format!("    pub {rust_ident}: Option<{rust_ty}>,\n"));
            } else {
                body.push_str("    #[serde(default");
                if let Some(d) = deser {
                    body.push_str(&format!(", deserialize_with = \"{d}\""));
                }
                body.push_str(&format!(", rename = \"{fname}\")]\n"));
                body.push_str(&format!("    pub {rust_ident}: Option<{rust_ty}>,\n"));
            }
        }
        body.push_str("}\n");
        self.structs.push(body);
    }

    /// Type to use for a field inside a record/list. Side effect: if
    /// the field is itself an Object or a List-of-Object, a child
    /// struct is emitted first.
    fn field_type(&mut self, parent: &str, fname: &str, shape: &Shape) -> String {
        match shape {
            Shape::Scalar { .. } => self.scalar_rust_type(shape),
            Shape::Object(_) => {
                let child = nested_type_name(parent, fname);
                self.emit_struct(&child, shape);
                child
            }
            Shape::List(inner) => {
                let elem_ty = match &**inner {
                    Shape::Scalar { .. } => self.scalar_rust_type(inner),
                    Shape::Object(_) => {
                        let child = element_type_name(parent, fname);
                        self.emit_struct(&child, inner);
                        child
                    }
                    Shape::List(_) => {
                        let child = element_type_name(parent, fname);
                        self.emit_struct(&child, inner);
                        child
                    }
                };
                format!("Vec<{elem_ty}>")
            }
        }
    }

    fn scalar_rust_type(&self, shape: &Shape) -> String {
        match shape {
            Shape::Scalar { ty, .. } => match ty {
                ScalarTy::Integer => "u64".into(),
                ScalarTy::Decimal => "rust_decimal::Decimal".into(),
                ScalarTy::BoolYn | ScalarTy::Bool01 => "bool".into(),
                ScalarTy::Date => "chrono::NaiveDate".into(),
                ScalarTy::DateTime => "chrono::NaiveDateTime".into(),
                ScalarTy::String | ScalarTy::Empty => "String".into(),
            },
            _ => "serde_json::Value".into(),
        }
    }
}

fn render_field_attrs(deser: Option<&'static str>) -> String {
    match deser {
        None => "    #[serde(default)]\n".into(),
        Some(d) => format!("    #[serde(default, deserialize_with = \"{d}\")]\n"),
    }
}

fn scalar_deserializer(shape: &Shape) -> Option<&'static str> {
    let Shape::Scalar { ty, .. } = shape else {
        return None;
    };

    match ty {
        ScalarTy::Integer => Some("crate::responses::deserialize_opt_u64_from_string_or_number"),
        ScalarTy::Decimal => {
            Some("crate::responses::deserialize_opt_decimal_from_string_or_number")
        }
        ScalarTy::BoolYn | ScalarTy::Bool01 => {
            Some("crate::responses::deserialize_opt_bool_from_string_number_or_yn")
        }
        ScalarTy::Date => Some("crate::responses::deserialize_opt_date"),
        ScalarTy::DateTime => Some("crate::responses::deserialize_opt_datetime"),
        ScalarTy::String | ScalarTy::Empty => None,
    }
}

fn nested_type_name(parent: &str, fname: &str) -> String {
    let acronyms = acronyms_sorted();
    format!("{parent}{}", camel_to_pascal(fname, &acronyms))
}

fn element_type_name(parent: &str, fname: &str) -> String {
    let acronyms = acronyms_sorted();
    let singular = singularize(fname);
    format!("{parent}{}", camel_to_pascal(&singular, &acronyms))
}

/// Naive English singularizer good enough for the field names voip.ms
/// uses (`dids` → `did`, `members` → `member`, `entries` → `entry`).
fn singularize(s: &str) -> String {
    if let Some(stem) = s.strip_suffix("ies") {
        return format!("{stem}y");
    }
    if let Some(stem) = s.strip_suffix("ses") {
        return format!("{stem}s");
    }
    if let Some(stem) = s.strip_suffix('s')
        && !stem.is_empty()
    {
        return stem.to_string();
    }

    s.to_string()
}

/// `type` is reserved; `match` and a couple of others may show up in
/// future API additions. Add as needed.
fn rust_field_ident(name: &str) -> String {
    if name.is_empty() {
        return "field_empty".into();
    }

    let safe = matches!(
        name,
        "type"
            | "match"
            | "fn"
            | "mod"
            | "ref"
            | "use"
            | "loop"
            | "move"
            | "box"
            | "where"
            | "self"
            | "Self"
            | "static"
            | "trait"
            | "true"
            | "false"
            | "as"
            | "async"
            | "await"
            | "dyn"
            | "enum"
            | "extern"
            | "impl"
            | "in"
            | "let"
            | "pub"
            | "return"
            | "struct"
            | "super"
            | "unsafe"
            | "while"
            | "yield"
            | "if"
            | "else"
            | "for"
            | "break"
            | "continue"
            | "const"
            | "crate"
    );
    if safe {
        return format!("r#{name}");
    }

    if is_rust_identifier(name) {
        return name.to_string();
    }

    // Numeric or otherwise non-identifier names: prefix with `field_`
    // and replace any remaining unsafe chars with `_`.
    let mut out = String::with_capacity(name.len() + 6);
    out.push_str("field_");
    for c in name.chars() {
        if is_ident_char(c) {
            out.push(c);
        } else {
            out.push('_');
        }
    }

    out
}

fn is_rust_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(is_ident_char)
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}
