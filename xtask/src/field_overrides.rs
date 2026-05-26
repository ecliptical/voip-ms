//! Field-name → Rust type substitutions applied by the generator.
//!
//! Some voip.ms fields are documented as `String` in the WSDL but
//! actually carry a small structured value (e.g. a `tag:value` routing
//! string, or a low-cardinality enum). When the wire encoding is stable
//! across methods, we type the field as a richer Rust type instead of
//! `String`. Substitutions are keyed by field name and apply uniformly
//! to every `*Params` and `*Response` struct.
//!
//! Two layers contribute entries:
//!
//! 1. The hard-coded [`builtin`] table — used for hand-written domain
//!    types like [`crate::Routing`] whose semantics span many fields.
//! 2. The `field_types` and `enums` sections of
//!    `tools/api-response-overrides.json`, loaded into a runtime
//!    [`Table`] alongside the built-ins. This is how data-driven
//!    enum substitutions (`dtmf_mode`, `nat`, …) reach the generator.

use std::collections::HashMap;

/// How a particular field name should be typed.
#[derive(Debug, Clone)]
pub struct FieldOverride {
    /// Fully-qualified Rust type to substitute for `String`.
    pub rust_type: String,
    /// Optional `deserialize_with` path for response use. The
    /// referenced function must accept `Option<T>` and treat empty /
    /// absent inputs as `None`.
    pub response_deserializer: Option<String>,
}

/// Runtime table of field-name overrides. Built from both built-in
/// entries and the overrides JSON.
#[derive(Debug, Default)]
pub struct Table {
    entries: HashMap<String, FieldOverride>,
}

impl Table {
    /// Build a table seeded with the built-in entries.
    pub fn with_builtins() -> Self {
        let mut t = Self::default();
        for (name, ov) in builtin() {
            t.entries.insert(name.to_string(), ov);
        }
        t
    }

    /// Insert or replace an override. Used by the codegen to add
    /// enum-typed fields declared in the overrides JSON.
    pub fn insert(&mut self, field: impl Into<String>, ov: FieldOverride) {
        self.entries.insert(field.into(), ov);
    }

    pub fn get(&self, name: &str) -> Option<&FieldOverride> {
        self.entries.get(name)
    }
}

/// Field names that should be typed as [`crate::Routing`] instead of
/// `String`. All of these encode a `tag:value` routing target.
const ROUTING_FIELDS: &[&str] = &[
    "routing",
    "routing_match",
    "routing_nomatch",
    "failover_busy",
    "failover_noanswer",
    "failover_unreachable",
    "fail_over_routing_full",
    "fail_over_routing_timeout",
    "fail_over_routing_join_empty",
    "fail_over_routing_join_unavail",
    "fail_over_routing_leave_empty",
    "fail_over_routing_leave_unavail",
];

fn builtin() -> Vec<(&'static str, FieldOverride)> {
    let routing = FieldOverride {
        rust_type: "crate::Routing".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_routing".into()),
    };
    ROUTING_FIELDS
        .iter()
        .map(|name| (*name, routing.clone()))
        .collect()
}
