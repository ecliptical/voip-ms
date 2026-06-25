//! Hand-edited corrections to the extracted response shapes.
//!
//! `tools/api-response-overrides.json` lets a maintainer either:
//!
//! * supply a full `shape` for a method the extractor couldn't parse
//!   (for example, methods whose Output block uses a non-standard
//!   `print_r` dialect or has no Output block at all), or
//! * patch one or more scalar types inside an otherwise-correct
//!   inferred shape (the doc samples often type-erase into one form —
//!   a phone number can look like an integer, a 0/1 flag can look like
//!   an integer — and the inferrer can't always tell).
//!
//! Paths use a small dotted grammar:
//!
//! * `field` — top-level field of the response object
//! * `obj.sub` — nested field
//! * `list[]` — refers to the element template of a list
//! * `list[].field` — field within a list element
//!
//! Patches only retype scalars. To restructure a subtree, use a full
//! shape replacement on the whole method.

use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use crate::extract::{ScalarTy, Shape};

#[derive(Debug, Deserialize)]
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
}

/// A user-defined enum to emit into the generated module.
#[derive(Debug, Deserialize)]
pub struct EnumDef {
    /// Optional doc comment placed above the emitted enum.
    #[serde(default)]
    pub doc: Option<String>,
    /// Variants in emission order.
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct MethodOverride {
    /// Full shape replacement. When present, takes precedence over the
    /// extracted shape (and over any `patches` on this method).
    #[serde(default)]
    pub shape: Option<JsonValue>,
    /// Scalar-type patches applied to the extracted shape.
    #[serde(default)]
    pub patches: Vec<Patch>,
}

#[derive(Debug, Deserialize)]
pub struct Patch {
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
        if !mo.patches.is_empty() {
            return Err("cannot combine `shape` with `patches` for the same method".into());
        }

        return Ok(Some(shape));
    }

    let Some(mut shape) = extracted else {
        if mo.patches.is_empty() {
            return Ok(None);
        }

        return Err(
            "patches given for a method with no extracted shape (supply `shape` instead)"
                .to_string(),
        );
    };

    for patch in &mo.patches {
        apply_patch(&mut shape, &patch.path, patch.ty)?;
    }

    Ok(Some(shape))
}

fn apply_patch(shape: &mut Shape, path: &str, ty: ScalarTy) -> Result<(), String> {
    let segments = parse_path(path)?;
    descend(shape, &segments, ty, path)
}

fn descend(shape: &mut Shape, segs: &[Seg], ty: ScalarTy, full_path: &str) -> Result<(), String> {
    let Some((head, rest)) = segs.split_first() else {
        return match shape {
            Shape::Scalar { ty: t, .. } => {
                *t = ty;
                Ok(())
            }
            _ => Err(format!(
                "override path `{full_path}` lands on a non-scalar node"
            )),
        };
    };

    match (head, shape) {
        (Seg::Field(name), Shape::Object(fields)) => {
            let slot = fields
                .iter_mut()
                .find(|(k, _)| k == name)
                .ok_or_else(|| format!("override path `{full_path}` field `{name}` not found"))?;
            descend(&mut slot.1, rest, ty, full_path)
        }
        (Seg::Element, Shape::List(inner)) => descend(inner, rest, ty, full_path),
        (Seg::Field(_), _) => Err(format!(
            "override path `{full_path}` expected object at `{head:?}`",
        )),
        (Seg::Element, _) => Err(format!("override path `{full_path}` expected list at `[]`",)),
    }
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
