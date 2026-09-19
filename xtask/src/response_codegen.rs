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
use crate::field_overrides::Resolver;
use crate::{acronyms_sorted, camel_to_pascal};

/// Render all `*Response` structs for the methods in `responses`.
///
/// `method_names` enumerates methods in the canonical (WSDL) order so
/// the output is stable across runs.
pub fn emit_response_structs(
    method_names: &[String],
    responses: &BTreeMap<String, Shape>,
    resolver: &Resolver,
) -> String {
    let acronyms = acronyms_sorted();
    let mut out = String::new();
    for op in method_names {
        let Some(shape) = responses.get(op) else {
            continue;
        };

        let pascal = camel_to_pascal(op, &acronyms);
        let root = format!("{pascal}Response");
        let mut emitter = Emitter::new(resolver);
        emitter.emit_struct(&root, shape);

        out.push_str(&format!(
            "\n/// Response body for [`Client::{}`] (wire method `{op}`).\n",
            crate::camel_to_snake(op, &acronyms),
        ));

        out.push_str(&emitter.into_text());
    }

    out
}

/// Where one response timestamp lands in the emitted structs, and the path
/// that reaches its JSON value.
#[derive(Debug)]
pub struct TimestampField {
    /// `"StructName.field"`, the key a per-struct type override is registered
    /// under.
    pub struct_path: String,
    /// The `/`-separated path `crate::attach_offset` takes, with `*` for every
    /// element of a list.
    pub json_path: String,
}

/// Every `datetime` scalar in `op`'s response shape, named the way
/// [`emit_response_structs`] emits it.
///
/// A `datetime` the walk cannot name is an error rather than a silent omission:
/// the field would still be emitted, as a bare `NaiveDateTime`, and the offset
/// the request carried would be dropped for it alone -- the miss the
/// `fields.is_empty()` check in `cmd_gen` cannot see, because some other field
/// covered it.
pub fn timestamp_fields(op: &str, shape: &Shape) -> Result<Vec<TimestampField>, String> {
    let acronyms = acronyms_sorted();
    let root = format!("{}Response", camel_to_pascal(op, &acronyms));
    let mut found = Vec::new();
    // A scalar response is emitted as a one-field record (`emit_struct`), so a
    // bare timestamp at the root is reachable under that field's name.
    if let Shape::Scalar {
        ty: ScalarTy::DateTime,
        ..
    } = shape
    {
        found.push(TimestampField {
            struct_path: format!("{root}.value"),
            json_path: "/value".into(),
        });
        return Ok(found);
    }

    collect_timestamps(&root, "", shape, &mut found)?;
    Ok(found)
}

fn collect_timestamps(
    struct_name: &str,
    json_prefix: &str,
    shape: &Shape,
    out: &mut Vec<TimestampField>,
) -> Result<(), String> {
    let fields = match shape {
        Shape::Object(fields) => fields,
        // Reached only through a collection's element shape, since the root is
        // handled by `timestamp_fields`. A bare timestamp here is an element of
        // a list or a value of a map, and `attach_offset`'s path form ends at a
        // field name, so there is nothing to name it with.
        Shape::Scalar {
            ty: ScalarTy::DateTime,
            ..
        } => {
            return Err(format!(
                "{struct_name} holds a bare timestamp at `{json_prefix}` rather than one \
                 under a field, which `attach_offset` has no path form for"
            ));
        }

        // A collection directly inside another. `attach_offset` would in fact
        // reach `/x/*/*/date`, but the emitter wraps a nested list in a record
        // of its own, so the path and the struct it lands on stop agreeing.
        // Supporting it is a decision about both, not a default.
        Shape::List(_) | Shape::Map(_) => {
            return Err(format!(
                "{struct_name} at `{json_prefix}` nests a collection directly inside another, \
                 which this walk does not name"
            ));
        }

        Shape::Scalar { .. } => return Ok(()),
    };

    // Mirrors `emit_record`'s dedupe, so a key the `print_r` source repeats
    // yields the one field the struct actually has.
    let mut seen = std::collections::HashSet::new();
    for (fname, sub) in fields {
        if !seen.insert(fname.as_str()) {
            continue;
        }

        let json_path = format!("{json_prefix}/{fname}");
        match sub {
            Shape::Scalar {
                ty: ScalarTy::DateTime,
                ..
            } => out.push(TimestampField {
                struct_path: format!("{struct_name}.{fname}"),
                json_path,
            }),
            Shape::Object(_) => {
                collect_timestamps(&nested_type_name(struct_name, fname), &json_path, sub, out)?;
            }

            Shape::List(inner) => collect_timestamps(
                &element_type_name(struct_name, fname),
                &format!("{json_path}/*"),
                inner,
                out,
            )?,
            // `attach_offset`'s `*` means "every element of a list", and over an
            // object it means the bare single record VoIP.ms sends in place of a
            // one-element list -- so it cannot also mean "every value of a map".
            // No offset op returns a map today; one that did would need the path
            // syntax extended first, which is a decision, not a default.
            Shape::Map(inner) => {
                let mut inside = Vec::new();
                let walked = collect_timestamps(
                    &element_type_name(struct_name, fname),
                    &format!("{json_path}/*"),
                    inner,
                    &mut inside,
                );
                // Either outcome means the map holds a timestamp: a walk that
                // named some, or one that failed trying. Both report against the
                // map, since that is the shape with no path form, not whatever
                // the recursion happened to be looking at when it gave up.
                let found = match walked {
                    Ok(()) if inside.is_empty() => continue,
                    Ok(()) => inside
                        .iter()
                        .map(|f| f.struct_path.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    Err(inner_error) => inner_error,
                };

                return Err(format!(
                    "{struct_name}.{fname} is a map whose values carry a timestamp ({found}); \
                     `attach_offset` has no path form that reaches it"
                ));
            }

            Shape::Scalar { .. } => {}
        }
    }

    Ok(())
}

struct Emitter<'a> {
    /// Structs emitted in dependency-friendly order (children appended
    /// before any later sibling that references them).
    structs: Vec<String>,
    resolver: &'a Resolver<'a>,
}

impl<'a> Emitter<'a> {
    fn new(resolver: &'a Resolver<'a>) -> Self {
        Self {
            structs: Vec::new(),
            resolver,
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

            Shape::Map(_) => {
                let map_ty = self.field_type(name, "entries", shape);
                let body = format!(
                    "#[derive(Debug, Clone, Default, serde::Deserialize)]\n\
                     pub struct {name} {{\n    \
                         #[serde(default, deserialize_with = \"crate::responses::deserialize_map_from_object\")]\n    \
                         pub entries: {map_ty},\n\
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

        let acronyms = acronyms_sorted();
        let mut body = String::new();
        body.push_str("#[derive(Debug, Clone, Default, serde::Deserialize)]\n");
        body.push_str(&format!("pub struct {name} {{\n"));
        for (fname, sub) in deduped {
            let rust_ident = crate::rust_field_ident(fname, &acronyms);
            // The name-based table only applies to scalar-shaped fields: a
            // substituted scalar type can never stand in for a list/object.
            let override_ = self
                .resolver
                .resolve(name, fname, matches!(sub, Shape::Scalar { .. }));
            let rust_ty = match override_ {
                Some(o) => o.rust_type.clone(),
                None => self.field_type(name, fname, sub),
            };
            let deser = match override_ {
                Some(o) => o.response_deserializer.as_deref(),
                None => field_deserializer(sub),
            };
            // Collection fields (list, map) are emitted as a bare `Vec<T>` /
            // `HashMap<K, V>`, defaulting to empty: VoIP.ms signals an empty
            // collection by omitting the field (or via an `is_empty` status that
            // strips the subtree), so absent and empty carry the same meaning --
            // an `Option` would only add a never-actionable `None`. A
            // `field_type` override always retypes to a scalar, so it keeps the
            // `Option<T>` form.
            let bare_collection =
                override_.is_none() && matches!(sub, Shape::List(_) | Shape::Map(_));
            let field_ty = if bare_collection {
                rust_ty
            } else {
                format!("Option<{rust_ty}>")
            };
            let attrs = render_field_attrs(deser);
            if rust_ident.trim_start_matches("r#") == *fname {
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
                    Shape::List(_) | Shape::Map(_) => {
                        let child = element_type_name(parent, fname);
                        self.emit_struct(&child, inner);
                        child
                    }
                };

                format!("Vec<{elem_ty}>")
            }

            Shape::Map(value) => {
                let value_ty = match &**value {
                    Shape::Scalar { .. } => self.scalar_rust_type(value),
                    Shape::Object(_) | Shape::List(_) | Shape::Map(_) => {
                        let child = element_type_name(parent, fname);
                        self.emit_struct(&child, value);
                        child
                    }
                };

                format!("std::collections::HashMap<String, {value_ty}>")
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
        Shape::Map(_) => Some("crate::responses::deserialize_map_from_object"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn datetime() -> Shape {
        Shape::Scalar {
            ty: ScalarTy::DateTime,
            sample: "2026-09-16 15:14:35".into(),
        }
    }

    fn text() -> Shape {
        Shape::Scalar {
            ty: ScalarTy::String,
            sample: "success".into(),
        }
    }

    fn object(fields: &[(&str, Shape)]) -> Shape {
        Shape::Object(
            fields
                .iter()
                .map(|(n, s)| ((*n).to_string(), s.clone()))
                .collect(),
        )
    }

    #[test]
    fn names_a_timestamp_in_a_list_of_records() {
        let shape = object(&[
            ("status", text()),
            (
                "cdr",
                Shape::List(Box::new(object(&[("date", datetime())]))),
            ),
        ]);
        let found = timestamp_fields("getCDR", &shape).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].struct_path, "GetCDRResponseCDR.date");
        assert_eq!(found[0].json_path, "/cdr/*/date");
    }

    #[test]
    fn names_a_timestamp_on_the_response_itself() {
        let shape = object(&[("status", text()), ("date", datetime())]);
        let found = timestamp_fields("getCDR", &shape).unwrap();
        assert_eq!(found[0].struct_path, "GetCDRResponse.date");
        assert_eq!(found[0].json_path, "/date");
    }

    #[test]
    fn names_a_scalar_response_under_the_field_it_is_emitted_as() {
        // `emit_struct` promotes a scalar response to `{ value: T }`.
        let found = timestamp_fields("getCDR", &datetime()).unwrap();
        assert_eq!(found[0].struct_path, "GetCDRResponse.value");
        assert_eq!(found[0].json_path, "/value");
    }

    #[test]
    fn ignores_a_response_with_no_timestamp() {
        let shape = object(&[
            ("status", text()),
            ("media", Shape::List(Box::new(text()))),
            ("codes", Shape::Map(Box::new(text()))),
        ]);
        assert!(timestamp_fields("getCDR", &shape).unwrap().is_empty());
    }

    // The rest are shapes that must fail the run rather than emit a field the
    // offset would be silently dropped for.

    #[test]
    fn rejects_a_timestamp_nested_two_collections_deep() {
        let shape = object(&[(
            "x",
            Shape::List(Box::new(Shape::List(Box::new(object(&[(
                "date",
                datetime(),
            )]))))),
        )]);
        let error = timestamp_fields("getCDR", &shape).unwrap_err();
        assert!(error.contains("nests a collection"), "{error}");
    }

    #[test]
    fn rejects_a_list_of_bare_timestamps() {
        let shape = object(&[("dates", Shape::List(Box::new(datetime())))]);
        let error = timestamp_fields("getCDR", &shape).unwrap_err();
        assert!(error.contains("bare timestamp"), "{error}");
    }

    #[test]
    fn rejects_a_map_whose_values_carry_a_timestamp() {
        let shape = object(&[(
            "byId",
            Shape::Map(Box::new(object(&[("date", datetime())]))),
        )]);
        let error = timestamp_fields("getCDR", &shape).unwrap_err();
        assert!(
            error.contains("is a map whose values carry a timestamp"),
            "{error}"
        );
    }

    #[test]
    fn rejects_a_map_of_bare_timestamps_naming_the_map() {
        let shape = object(&[("byId", Shape::Map(Box::new(datetime())))]);
        let error = timestamp_fields("getCDR", &shape).unwrap_err();
        assert!(error.contains("byId is a map"), "{error}");
    }
}
