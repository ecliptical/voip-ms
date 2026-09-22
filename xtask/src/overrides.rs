//! Hand-edited corrections to the extracted response shapes.
//!
//! `tools/api-response-overrides.json` lets a maintainer either:
//!
//! * supply a full `shape` for a method the extractor couldn't parse
//!   (for example, methods whose Output block uses a non-standard
//!   `print_r` dialect or has no Output block at all), or
//! * patch one or more scalar types inside an otherwise-correct
//!   inferred shape (the doc samples often type-erase into one form --
//!   a phone number can look like an integer, a 0/1 flag can look like
//!   an integer -- and the inferrer can't always tell), or
//! * add a scalar field the live API returns but the docs never list,
//!   which the extractor cannot see by construction.
//!
//! Paths use a small dotted grammar:
//!
//! * `field` -- top-level field of the response object
//! * `obj.sub` -- nested field
//! * `list[]` -- refers to the element template of a list
//! * `list[].field` -- field within a list element
//! * `map.*` -- the value template of a dynamic-key map, whose own keys are
//!   data rather than schema
//!
//! Patches only retype scalars, and additions only create them. To
//! restructure a subtree, use a full shape replacement on the whole method.

use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use crate::extract::{ScalarTy, Shape};

/// The path segment naming a dynamic-key map's value template, matching what
/// `dump-fields` emits and the live harness reports.
const MAP_WILDCARD: &str = "*";

#[derive(Deserialize)]
pub struct OverridesDoc {
    /// Schema version. Currently always `1`; bump and branch on it if
    /// the format ever changes incompatibly.
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub methods: HashMap<String, MethodOverride>,
    /// Named enums emitted into `src/generated.rs`. The key is the
    /// emitted Rust type name (PascalCase).
    #[serde(default)]
    pub enums: HashMap<String, EnumDef>,
    /// Field-name → enum-name table. Each named field across every
    /// generated `*Params` and `*Response` struct gets typed as
    /// `Option<EnumName>` instead of `Option<String>`.
    #[serde(default)]
    pub field_types: HashMap<String, String>,
    /// Per-struct field paths the field-name override table must not
    /// touch. The substitution in `field_types` (and the built-in
    /// routing/flag tables) keys on field *name* and applies to every
    /// `*Params`/`*Response` struct that has it -- correct when the name
    /// means the same thing everywhere, but a few response structs reuse
    /// a flag name for an unrelated value (e.g. `GetVoicemailsResponse
    /// Voicemail.urgent` is a *count* of urgent messages, not the
    /// per-message urgent flag). Each entry is an emitted-struct name and
    /// field, `"StructName.field"`, where the field keeps its inferred /
    /// patched type instead of the name-based override.
    #[serde(default)]
    pub field_type_skip: Vec<String>,
    /// Per-struct field → enum-name table: the assigning complement to
    /// [`Self::field_type_skip`]. Where one field name means different
    /// things in different structs (the `type` field is a search mode in
    /// `SearchVanityParams`, a message direction in `GetSMSResponseSMS`, a
    /// reference-data code elsewhere), a global `field_types` entry can't
    /// apply. Each key is an emitted-struct name and field,
    /// `"StructName.field"`, mapped to a declared enum; that one struct's
    /// field is typed as the enum, overriding both the inferred type and
    /// any name-based `field_types` entry.
    #[serde(default)]
    pub field_type_override: HashMap<String, String>,
    /// Wire status codes that mean "the requested collection is empty,"
    /// not a failure. VoIP.ms returns a distinct `no_*` status for each
    /// list method when the list has no entries (`no_sms`, `no_cdr`,
    /// `no_messages`, ...). The generator emits these into
    /// `voip_ms::ApiStatus::is_empty_collection`; the client treats an empty status as a
    /// successful empty response (all collection fields deserialize to
    /// `None`) instead of an `voip_ms::Error::Api`. Codes that look like `no_*`
    /// but signal a real failure (`no_base64file`, `no_callstatus`,
    /// `no_provision`, ...) are deliberately omitted.
    #[serde(default)]
    pub empty_statuses: Vec<String>,
}

/// A user-defined enum to emit into the generated module.
#[derive(Deserialize)]
pub struct EnumDef {
    /// Optional doc comment placed above the emitted enum.
    #[serde(default)]
    pub doc: Option<String>,
    /// Variants in emission order.
    pub variants: Vec<EnumVariant>,
}

#[derive(Deserialize)]
pub struct EnumVariant {
    /// PascalCase Rust variant name.
    pub name: String,
    /// Wire string. Required.
    pub wire: String,
    /// Optional per-variant doc comment.
    #[serde(default)]
    pub doc: Option<String>,
}

impl OverridesDoc {
    pub fn check_version(&self) -> Result<(), String> {
        if self.version != 0 && self.version != 1 {
            return Err(format!(
                "unsupported overrides schema version {} (expected 1)",
                self.version
            ));
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct MethodOverride {
    /// Full shape replacement. When present, takes precedence over the
    /// extracted shape (and over any `patches` or `additions` on this
    /// method).
    #[serde(default)]
    pub shape: Option<JsonValue>,
    /// Scalar-type patches applied to the extracted shape.
    #[serde(default)]
    pub patches: Vec<Patch>,
    /// Scalar fields appended to the extracted shape, applied before
    /// [`Self::patches`].
    #[serde(default)]
    pub additions: Vec<Addition>,
}

#[derive(Deserialize)]
pub struct Patch {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: ScalarTy,
}

/// An undocumented response field, declared by hand from an observed live
/// response. The path's final segment is the new field's wire name; its
/// parent must already exist in the extracted shape.
///
/// Declaring a field the extracted shape already carries is an error: the
/// docs have caught up, so the entry is stale and belongs deleted.
#[derive(Deserialize)]
pub struct Addition {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: ScalarTy,
}

pub fn load(path: &Path) -> Result<OverridesDoc, String> {
    if !path.exists() {
        return Ok(OverridesDoc {
            version: 0,
            methods: HashMap::new(),
            enums: HashMap::new(),
            field_types: HashMap::new(),
            field_type_skip: Vec::new(),
            field_type_override: HashMap::new(),
            empty_statuses: Vec::new(),
        });
    }

    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Apply `mo` on top of the extractor's shape for one method.
///
/// Returns the patched shape, or a new one when `mo.shape` is set.
pub fn apply(extracted: Option<Shape>, mo: &MethodOverride) -> Result<Option<Shape>, String> {
    if let Some(repr) = &mo.shape {
        let shape = Shape::from_json(repr)?;
        if !mo.patches.is_empty() || !mo.additions.is_empty() {
            return Err(
                "cannot combine `shape` with `patches` or `additions` for the same method".into(),
            );
        }

        return Ok(Some(shape));
    }

    let Some(mut shape) = extracted else {
        if mo.patches.is_empty() && mo.additions.is_empty() {
            return Ok(None);
        }

        return Err(
            "patches or additions given for a method with no extracted shape (supply `shape` \
             instead)"
                .to_string(),
        );
    };

    for addition in &mo.additions {
        apply_addition(&mut shape, &addition.path, addition.ty)?;
    }

    for patch in &mo.patches {
        apply_patch(&mut shape, &patch.path, patch.ty)?;
    }

    Ok(Some(shape))
}

fn apply_patch(shape: &mut Shape, path: &str, ty: ScalarTy) -> Result<(), String> {
    let segments = parse_path(path)?;
    match resolve(shape, &segments, path)? {
        Shape::Scalar { ty: slot, .. } => {
            *slot = ty;
            Ok(())
        }
        _ => Err(format!("override path `{path}` lands on a non-scalar node")),
    }
}

fn apply_addition(shape: &mut Shape, path: &str, ty: ScalarTy) -> Result<(), String> {
    let mut segments = parse_path(path)?;
    let Some(Seg::Field(name)) = segments.pop() else {
        return Err(format!(
            "addition path `{path}` must end in the new field's name"
        ));
    };

    let Shape::Object(fields) = resolve(shape, &segments, path)? else {
        return Err(format!(
            "addition path `{path}` expected object at its parent"
        ));
    };

    if fields.iter().any(|(k, _)| *k == name) {
        return Err(format!(
            "addition path `{path}` is already in the extracted shape; the docs now \
             list it, so delete the addition"
        ));
    }

    fields.push((
        name,
        Shape::Scalar {
            ty,
            sample: String::new(),
        },
    ));

    Ok(())
}

/// Walk `segs` from `shape` and hand back the node they name.
fn resolve<'a>(
    shape: &'a mut Shape,
    segs: &[Seg],
    full_path: &str,
) -> Result<&'a mut Shape, String> {
    let mut node = shape;
    for seg in segs {
        node = match (seg, node) {
            (Seg::Field(name), Shape::Object(fields)) => fields
                .iter_mut()
                .find(|(k, _)| k == name)
                .map(|slot| &mut slot.1)
                .ok_or_else(|| format!("override path `{full_path}` field `{name}` not found"))?,
            (Seg::Element, Shape::List(inner)) => inner.as_mut(),
            // `*` addresses a dynamic-key map's value template. Its keys are
            // data, so this is the only way to name a position inside one --
            // and the live harness reports findings under a map that way.
            (Seg::Field(name), Shape::Map(value)) if name == MAP_WILDCARD => value.as_mut(),
            (Seg::Field(_), _) => {
                return Err(format!(
                    "override path `{full_path}` expected object at `{seg:?}`"
                ));
            }
            (Seg::Element, _) => {
                return Err(format!("override path `{full_path}` expected list at `[]`"));
            }
        };
    }

    Ok(node)
}

#[derive(Debug)]
enum Seg {
    Field(String),
    Element,
}

fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
    let mut out = Vec::new();
    for part in path.split('.') {
        if part.is_empty() {
            return Err(format!("override path `{path}` has empty segment"));
        }

        let (name, list_depth) = split_brackets(part)?;
        if !name.is_empty() {
            out.push(Seg::Field(name.to_string()));
        }

        for _ in 0..list_depth {
            out.push(Seg::Element);
        }
    }

    Ok(out)
}

fn split_brackets(part: &str) -> Result<(&str, usize), String> {
    let bytes = part.as_bytes();
    let mut split_at = bytes.len();
    let mut depth = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if split_at == bytes.len() {
                split_at = i;
            }

            if i + 1 >= bytes.len() || bytes[i + 1] != b']' {
                return Err(format!("override path segment `{part}` expects `[]`"));
            }

            depth += 1;
            i += 2;
        } else if depth > 0 {
            return Err(format!(
                "override path segment `{part}` has text after `[]`"
            ));
        } else {
            i += 1;
        }
    }

    Ok((&part[..split_at], depth))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cdr_shape() -> Shape {
        Shape::Object(vec![(
            "cdr".to_string(),
            Shape::List(Box::new(Shape::Object(vec![(
                "uniqueid".to_string(),
                Shape::Scalar {
                    ty: ScalarTy::String,
                    sample: "128238059".to_string(),
                },
            )]))),
        )])
    }

    fn addition(path: &str) -> MethodOverride {
        MethodOverride {
            shape: None,
            patches: Vec::new(),
            additions: vec![Addition {
                path: path.to_string(),
                ty: ScalarTy::String,
            }],
        }
    }

    fn element_fields(shape: &Shape) -> &[(String, Shape)] {
        let Shape::Object(top) = shape else {
            panic!("expected object")
        };
        let Shape::List(element) = &top[0].1 else {
            panic!("expected list")
        };
        let Shape::Object(fields) = element.as_ref() else {
            panic!("expected object")
        };

        fields
    }

    #[test]
    fn addition_appends_to_a_list_element() {
        let shape = apply(Some(cdr_shape()), &addition("cdr[].ip"))
            .unwrap()
            .unwrap();
        let names: Vec<&str> = element_fields(&shape)
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(names, ["uniqueid", "ip"]);
    }

    #[test]
    fn addition_of_an_extracted_field_is_an_error() {
        let err = apply(Some(cdr_shape()), &addition("cdr[].uniqueid")).unwrap_err();
        assert!(err.contains("already in the extracted shape"), "{err}");
    }

    #[test]
    fn addition_under_a_missing_parent_is_an_error() {
        let err = apply(Some(cdr_shape()), &addition("calls[].ip")).unwrap_err();
        assert!(err.contains("field `calls` not found"), "{err}");
    }

    /// A finding under a dynamic-key map is reported as `map.*.field`, so that
    /// path has to be one an addition can actually use.
    #[test]
    fn addition_reaches_a_maps_value_template() {
        let shape = Shape::Object(vec![(
            "list_status".to_string(),
            Shape::Map(Box::new(Shape::Object(vec![(
                "label".to_string(),
                Shape::Scalar {
                    ty: ScalarTy::String,
                    sample: "Active".to_string(),
                },
            )]))),
        )]);

        let applied = apply(Some(shape), &addition("list_status.*.added"))
            .unwrap()
            .unwrap();
        let Shape::Object(top) = &applied else {
            panic!("expected object")
        };
        let Shape::Map(value) = &top[0].1 else {
            panic!("expected map")
        };
        let Shape::Object(fields) = value.as_ref() else {
            panic!("expected object")
        };
        let names: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["label", "added"]);
    }

    #[test]
    fn addition_onto_a_scalar_is_an_error() {
        let err = apply(Some(cdr_shape()), &addition("cdr[].uniqueid.ip")).unwrap_err();
        assert!(err.contains("expected object"), "{err}");
    }

    #[test]
    fn a_full_shape_replacement_rejects_additions() {
        let mut mo = addition("cdr[].ip");
        mo.shape = Some(serde_json::json!({ "kind": "object", "fields": [] }));
        let err = apply(Some(cdr_shape()), &mo).unwrap_err();
        assert!(err.contains("cannot combine"), "{err}");
    }
}
