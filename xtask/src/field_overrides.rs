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

/// Boolean flags voip.ms encodes on the wire as `1` / `0`. Typed as
/// [`crate::Flag01`] instead of the `i64` / `String` / `f64` the WSDL declares.
/// Documented as `1 = true, 0 = false` (or `1=Enable / 0=Disable`).
const FLAG_01_FIELDS: &[&str] = &[
    "diversion_header",
    "dont_charge_monthly",
    "dont_charge_setup",
    "email_attach_file",
    "email_enable",
    "email_enabled",
    "enable",
    "enabled",
    "isMobile",
    "isPartial",
    "security_code_enabled",
    "send_email_enabled",
    "sms_forward_enable",
    "sms_sipaccount_enabled",
    "test",
    "url_callback_enable",
    "url_callback_retry",
];

/// Boolean flags voip.ms encodes on the wire as `yes` / `no`. Typed as
/// [`crate::FlagYesNo`] instead of `String`. These are the conference, queue,
/// and voicemail toggles documented as `(yes/no)`.
const FLAG_YES_NO_FIELDS: &[&str] = &[
    "admin",
    "announce_join_leave",
    "announce_only_user",
    "announce_user_count",
    "attach_message",
    "delete_message",
    "drop_silence",
    "jitter_buffer",
    "listened",
    "quiet",
    "ring_inuse",
    "say_callerid",
    "say_time",
    "start_muted",
    "talk_detection",
    "thankyou_for_your_patience",
    "transcription",
    "transcription_redaction",
    "transcription_sentiment",
    "transcription_summary",
    "urgent",
];

fn builtin() -> Vec<(&'static str, FieldOverride)> {
    let routing = FieldOverride {
        rust_type: "crate::Routing".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_routing".into()),
    };
    let flag_01 = FieldOverride {
        rust_type: "crate::Flag01".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_flag01".into()),
    };
    let flag_yes_no = FieldOverride {
        rust_type: "crate::FlagYesNo".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_flag_yes_no".into()),
    };

    ROUTING_FIELDS
        .iter()
        .map(|name| (*name, routing.clone()))
        .chain(FLAG_01_FIELDS.iter().map(|name| (*name, flag_01.clone())))
        .chain(
            FLAG_YES_NO_FIELDS
                .iter()
                .map(|name| (*name, flag_yes_no.clone())),
        )
        .collect()
}
