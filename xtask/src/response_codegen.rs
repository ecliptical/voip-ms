//! Walks merged response `Shape` trees and emits typed `*Response`
//! structs into `src/generated.rs`.
//!
//! Naming:
//! * Top-level type for method `getBalance` → `GetBalanceResponse`.
//! * Nested object types are named by path: the `balance` sub-object of
//!   `getBalance` becomes `GetBalanceResponseBalance`.
//! * List elements drop a trailing plural `s`/`es` where it doesn't
//!   collide: the elements of `dids: [...]` in `getDIDsInfo` become
//!   `GetDIDsInfoResponseDID`.
//!
//! Scalar and object fields are `Option<T>` so unexpected omissions don't fail
//! deserialization; list fields are a bare `Vec<T>` defaulting to empty, since
//! VoIP.ms signals an empty collection by omitting the field. Scalar types map
//! to:
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
use crate::field_overrides::{FieldOverride, Table};
use crate::{acronyms_sorted, camel_to_pascal};

/// Render all `*Response` structs for the methods in `responses`.
///
/// `method_names` enumerates methods in the canonical (WSDL) order so
/// the output is stable across runs.
pub fn emit_response_structs(
    method_names: &[String],
    responses: &BTreeMap<String, Shape>,
    table: &Table,
    field_type_skip: &std::collections::BTreeSet<String>,
    field_type_override: &BTreeMap<String, FieldOverride>,
) -> String {
    let acronyms = acronyms_sorted();
    let mut out = String::new();
    for op in method_names {
        let Some(shape) = responses.get(op) else {
            continue;
        };

        let pascal = camel_to_pascal(op, &acronyms);
        let root = format!("{pascal}Response");
        let mut emitter = Emitter::new(table, field_type_skip, field_type_override);
        emitter.emit_struct(&root, shape);

        out.push_str(&format!(
            "\n/// Response body for [`Client::{}`] (wire method `{op}`).\n",
            crate::camel_to_snake(op, &acronyms),
        ));

        out.push_str(&emitter.into_text());
    }

    out
}

struct Emitter<'a> {
    /// Structs emitted in dependency-friendly order (children appended
    /// before any later sibling that references them).
    structs: Vec<String>,
    table: &'a Table,
    /// `"StructName.field"` paths where the field-name override table is
    /// suppressed (the field keeps its inferred/patched type).
    field_type_skip: &'a std::collections::BTreeSet<String>,
    /// `"StructName.field"` paths assigned a specific enum type, overriding
    /// the inferred type and any name-based override.
    field_type_override: &'a BTreeMap<String, FieldOverride>,
}

impl<'a> Emitter<'a> {
    fn new(
        table: &'a Table,
        field_type_skip: &'a std::collections::BTreeSet<String>,
        field_type_override: &'a BTreeMap<String, FieldOverride>,
    ) -> Self {
        Self {
            structs: Vec::new(),
            table,
            field_type_skip,
            field_type_override,
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
                         #[serde(default, deserialize_with = \"crate::responses::deserialize_vec_from_single_or_seq\")]\n    \
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
            // A per-struct `field_type_override` wins outright; otherwise the
            // name-based table applies, unless `field_type_skip` suppresses it
            // for this same-named-but-unrelated field.
            let path = format!("{name}.{fname}");
            let override_ = if let Some(o) = self.field_type_override.get(&path) {
                Some(o.clone())
            } else if self.field_type_skip.contains(&path) {
                None
            } else {
                self.table.get(fname).cloned()
            };
            let rust_ty = match &override_ {
                Some(o) => o.rust_type.clone(),
                None => self.field_type(name, fname, sub),
            };
            let deser = match &override_ {
                Some(o) => o.response_deserializer.as_deref(),
                None => field_deserializer(sub),
            };
            // List fields are emitted as a bare `Vec<T>`, defaulting to empty:
            // VoIP.ms signals an empty collection by omitting the field (or via
            // an `is_empty` status that strips the subtree), so absent and empty
            // carry the same meaning -- an `Option` would only add a
            // never-actionable `None`. A `field_type` override always retypes to
            // a scalar, so it keeps the `Option<T>` form.
            let bare_vec = override_.is_none() && matches!(sub, Shape::List(_));
            let field_ty = if bare_vec {
                rust_ty
            } else {
                format!("Option<{rust_ty}>")
            };
            let attrs = render_field_attrs(deser);
            if rust_ident == *fname {
                body.push_str(&attrs);
                body.push_str(&format!("    pub {rust_ident}: {field_ty},\n"));
            } else {
                body.push_str("    #[serde(default");
                if let Some(d) = deser {
                    body.push_str(&format!(", deserialize_with = \"{d}\""));
                }

                body.push_str(&format!(", rename = \"{fname}\")]\n"));
                body.push_str(&format!("    pub {rust_ident}: {field_ty},\n"));
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

fn render_field_attrs(deser: Option<&str>) -> String {
    match deser {
        None => "    #[serde(default)]\n".into(),
        Some(d) => format!("    #[serde(default, deserialize_with = \"{d}\")]\n"),
    }
}

/// The `deserialize_with` a response field uses given its shape: scalars get
/// their type-coercing helper; lists get the single-or-sequence helper (VoIP.ms
/// returns a one-row list as a bare object); objects deserialize structurally.
fn field_deserializer(shape: &Shape) -> Option<&'static str> {
    match shape {
        Shape::List(_) => Some("crate::responses::deserialize_vec_from_single_or_seq"),
        _ => scalar_deserializer(shape),
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
        ScalarTy::String | ScalarTy::Empty => {
            Some("crate::responses::deserialize_opt_string_from_string_number_or_bool")
        }
    }
}

fn nested_type_name(parent: &str, fname: &str) -> String {
    let acronyms = acronyms_sorted();
    format!("{parent}{}", camel_to_pascal(fname, &acronyms))
}

fn element_type_name(parent: &str, fname: &str) -> String {
    let acronyms = acronyms_sorted();
    let singular = singularize(fname, &acronyms);
    format!("{parent}{}", camel_to_pascal(&singular, &acronyms))
}

/// Naive English singularizer good enough for the field names VoIP.ms
/// uses (`dids` → `did`, `members` → `member`, `entries` → `entry`).
///
/// Preserves words whose lowercase form is itself an acronym chain
/// (e.g. `sms`, `mms`) so they aren't stripped to a non-acronym stem
/// (`sm`, `mm`).
fn singularize(s: &str, acronyms: &[&'static str]) -> String {
    let lower = s.to_ascii_lowercase();

    if let Some(stem) = s.strip_suffix("ies") {
        return format!("{stem}y");
    }

    // Words ending in -sses (addresses, classes, businesses) drop just "es"
    // to yield -ss. Plain "-ses" without the double s is usually
    // "<vowel>se" + "s" (phrases, houses, uses) and the trailing "s"
    // strip below handles it correctly.
    if lower.ends_with("sses")
        && let Some(stem) = s.strip_suffix("es")
    {
        return stem.to_string();
    }

    // Words ending in -xes / -zes / -ches / -shes drop the full "es"
    // (faxes → fax, boxes → box, matches → match, dishes → dish).
    if (lower.ends_with("xes")
        || lower.ends_with("zes")
        || lower.ends_with("ches")
        || lower.ends_with("shes"))
        && let Some(stem) = s.strip_suffix("es")
    {
        return stem.to_string();
    }

    if let Some(stem) = s.strip_suffix('s')
        && !stem.is_empty()
    {
        let stem_lower = stem.to_ascii_lowercase();
        let full_is_acronym = crate::decompose_into_acronyms(acronyms, &lower).is_some();
        let stem_is_acronym = crate::decompose_into_acronyms(acronyms, &stem_lower).is_some();
        if full_is_acronym && !stem_is_acronym {
            return s.to_string();
        }

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
