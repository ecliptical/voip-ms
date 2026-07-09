//! Field-name → Rust type substitutions applied by the generator.
//!
//! Some VoIP.ms fields are documented as `String` in the WSDL but
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
#[derive(Debug, Clone, Default)]
pub struct FieldOverride {
    /// Fully-qualified Rust type to substitute for `String`.
    pub rust_type: String,
    /// Optional `serialize_with` path for param use. Needed when the
    /// substituted type doesn't itself serialize to the wire form
    /// VoIP.ms expects -- e.g. a plain `bool` whose flag must travel as
    /// `1`/`0` rather than `true`/`false`. Types that carry their own
    /// `Serialize` (like [`crate::Routing`]) leave this `None`.
    pub param_serializer: Option<String>,
    /// When set, the param field is emitted as plain `T` (not
    /// `Option<T>`) and skipped on the wire when equal to its default
    /// via this `skip_serializing_if` path. Used for true-only flags
    /// (`test`) where `false` carries no meaning distinct from absent.
    pub param_skip_if: Option<String>,
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

/// Boolean flags VoIP.ms encodes on the wire as `1` / `0`. Typed as `bool`
/// instead of the `i64` / `String` / `f64` the WSDL declares; the `1`/`0` wire
/// form comes from a param `serialize_with`, since a bare `bool` would serialize
/// as `true`/`false`, which these parameters reject. Documented as
/// `1 = true, 0 = false` (or `1=Enable / 0=Disable`).
const FLAG_01_FIELDS: &[&str] = &[
    "activate",
    "advanced",
    "answered",
    "burst_enabled",
    "busy",
    "diversion_header",
    "dont_charge_monthly",
    "dont_charge_setup",
    "email_attach_file",
    "email_enable",
    "email_enabled",
    "enable",
    "enable_internal_cnam",
    "enable_ip_restriction",
    "enable_pop_restriction",
    "enabled",
    "failed",
    "fax_to_sip_enabled",
    "isMobile",
    "isPartial",
    "noanswer",
    "portout",
    "record_calls",
    "security_code_enabled",
    "send_bye",
    "send_email_enabled",
    "skip_password",
    "smpp_enabled",
    "sms_email_enabled",
    "sms_forward_enable",
    "sms_forward_enabled",
    "sms_sipaccount_enabled",
    "sms_url_callback_enabled",
    "sms_url_callback_retry",
    "transcribe",
    "url_callback_enable",
    "url_callback_retry",
    "url_enabled",
];

/// Boolean flags VoIP.ms encodes on the wire as `yes` / `no`. Typed as `bool`
/// instead of `String`, with the `yes`/`no` wire form from a param
/// `serialize_with`. These are the conference, queue, and voicemail toggles
/// documented as `(yes/no)`.
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

/// Queue/announcement durations documented as a number of seconds *or* the
/// word `none` (no limit / no delay). Typed as [`crate::Seconds`], which holds
/// the count or an unbounded sentinel; `maximum_wait_time` uses the word
/// `unlimited` instead and is typed [`crate::WaitTime`] separately.
const SECONDS_FIELDS: &[&str] = &[
    "announce_position_frecuency",
    "announce_round_seconds",
    "frequency_announcement",
    "member_delay",
    "retry_timer",
    "wrapup_time",
];

/// Caller-ID / forward phone-number override fields. They are phone-number
/// identifiers -- not integers -- so a formatted or non-NANP value must
/// survive, but voip.ms signals "not set" with a `-1` sentinel (or empty),
/// which a real caller ID never is. Typed `String` with a deserializer that
/// folds `-1`/empty to `None`.
const CALLERID_OVERRIDE_FIELDS: &[&str] = &[
    "callerid_number",
    "callerid_override",
    "default_e911",
    "sms_forward",
];

fn builtin() -> Vec<(&'static str, FieldOverride)> {
    let routing = FieldOverride {
        rust_type: "crate::Routing".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_routing".into()),
        ..Default::default()
    };
    let tolerant_bool = "crate::responses::deserialize_opt_bool_from_string_number_or_yn";
    let flag_01 = FieldOverride {
        rust_type: "bool".into(),
        param_serializer: Some("crate::responses::serialize_opt_flag_01".into()),
        response_deserializer: Some(tolerant_bool.into()),
        ..Default::default()
    };
    let flag_yes_no = FieldOverride {
        rust_type: "bool".into(),
        param_serializer: Some("crate::responses::serialize_opt_flag_yes_no".into()),
        response_deserializer: Some(tolerant_bool.into()),
        ..Default::default()
    };
    // `test` is a request-only validate-only flag: its docs uniformly say
    // "set to true if testing... no changes are made", so `false` carries no
    // meaning distinct from absent. Emitted as plain `bool`, skipped when false.
    let flag_test = FieldOverride {
        rust_type: "bool".into(),
        param_serializer: Some("crate::responses::serialize_flag_01".into()),
        param_skip_if: Some("crate::responses::is_false".into()),
        ..Default::default()
    };
    // Seconds / WaitTime carry their own Serialize, like Routing -- no
    // param_serializer needed.
    let seconds = FieldOverride {
        rust_type: "crate::Seconds".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_seconds".into()),
        ..Default::default()
    };
    let wait_time = FieldOverride {
        rust_type: "crate::WaitTime".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_wait_time".into()),
        ..Default::default()
    };
    // Phone-number override: `String` (a formatted / non-NANP caller ID must
    // survive) with a deserializer that folds the `-1`/empty "not set" sentinel
    // to `None`. Params are already `Option<String>`, so `rust_type` is a no-op
    // there; only the response deserializer changes.
    let callerid_override = FieldOverride {
        rust_type: "String".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_string_sentinel_none".into(),
        ),
        ..Default::default()
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
        .chain(std::iter::once(("test", flag_test)))
        .chain(SECONDS_FIELDS.iter().map(|name| (*name, seconds.clone())))
        .chain(std::iter::once(("maximum_wait_time", wait_time)))
        .chain(
            CALLERID_OVERRIDE_FIELDS
                .iter()
                .map(|name| (*name, callerid_override.clone())),
        )
        .collect()
}
