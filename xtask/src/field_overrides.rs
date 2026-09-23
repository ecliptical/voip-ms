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
//!    types like `voip_ms::Routing` whose semantics span many fields.
//! 2. The `field_types` and `enums` sections of
//!    `tools/api-response-overrides.json`, loaded into a runtime
//!    [`Table`] alongside the built-ins. This is how data-driven
//!    enum substitutions (`dtmf_mode`, `nat`, …) reach the generator.

use std::collections::{BTreeMap, BTreeSet, HashMap};

/// How a particular field name should be typed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldOverride {
    /// Fully-qualified Rust type to substitute for `String`.
    pub rust_type: String,
    /// Optional `serialize_with` path for param use. Needed when the
    /// substituted type doesn't itself serialize to the wire form
    /// VoIP.ms expects -- e.g. a plain `bool` whose flag must travel as
    /// `1`/`0` rather than `true`/`false`. Types that carry their own
    /// `Serialize` (like `voip_ms::Routing`) leave this `None`.
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
    /// Rust type to substitute on the *response* side only, when it differs
    /// from [`Self::rust_type`]. A param is written and so cannot receive a
    /// value this crate does not understand; a response can, which is why the
    /// date fields read as `voip_ms::Reported<T>` and write as the bare `T`.
    pub response_rust_type: Option<String>,
}

impl FieldOverride {
    /// The type to emit on the response side: [`Self::response_rust_type`]
    /// where the two sides differ, otherwise [`Self::rust_type`].
    pub fn response_type(&self) -> &str {
        self.response_rust_type
            .as_deref()
            .unwrap_or(&self.rust_type)
    }
}

/// Runtime table of field-name overrides. Built from both built-in
/// entries and the overrides JSON.
#[derive(Default)]
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

/// Resolves which override (if any) applies to one emitted struct field,
/// combining the three override sources with their precedence: a per-struct
/// assignment (`field_type_override`) wins outright; otherwise the name-based
/// table applies -- but only to scalar-shaped fields, and not where a
/// `field_type_skip` entry suppresses it for this struct.
pub struct Resolver<'a> {
    /// Name-based override table (built-ins + `field_types` enums).
    pub table: &'a Table,
    /// `"StructName.field"` -> override assignments (`field_type_override`).
    pub per_struct: &'a BTreeMap<String, FieldOverride>,
    /// `"StructName.field"` paths where the name-based table is suppressed.
    pub skip: &'a BTreeSet<String>,
}

impl<'a> Resolver<'a> {
    /// The override for `struct_name.fname`. `name_based` says whether the
    /// name-based table may apply: params (always WSDL scalars) pass `true`;
    /// response fields pass "is this field scalar-shaped" -- a substituted
    /// scalar type can never stand in for a list/object/map, so collection
    /// fields (e.g. the reference catalogs `getNAT` and
    /// `getPlayInstructions` return under an overridden field name) keep
    /// their structural type without needing a skip entry.
    pub fn resolve(
        &self,
        struct_name: &str,
        fname: &str,
        name_based: bool,
    ) -> Option<&'a FieldOverride> {
        let path = format!("{struct_name}.{fname}");
        if let Some(o) = self.per_struct.get(&path) {
            return Some(o);
        }

        if !name_based || self.skip.contains(&path) {
            return None;
        }

        self.table.get(fname)
    }
}

/// Field names that should be typed as `voip_ms::Routing` instead of
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
pub(crate) const FLAG_01_FIELDS: &[&str] = &[
    "activate",
    "advanced",
    "answered",
    "burst_enabled",
    "busy",
    "cnam",
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
    "sip_traffic",
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
    // `setMusicOnHold`'s "quiet volume" toggle. Confirmed live: `1` stores
    // the quiet rendition and `0` (or anything else) the normal one. The
    // response field of the same name is *not* this flag -- it reports the
    // rendition that resulted (`mp3` / `quietmp3`) -- so it carries a
    // `field_type_skip`.
    "volume",
];

/// Boolean flags VoIP.ms encodes on the wire as `yes` / `no`. Typed as `bool`
/// instead of `String`, with the `yes`/`no` wire form from a param
/// `serialize_with`. These are the conference, queue, and voicemail toggles
/// documented as `(yes/no)`.
pub(crate) const FLAG_YES_NO_FIELDS: &[&str] = &[
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
/// word `none` (no limit / no delay). Typed as `voip_ms::Seconds`, which holds
/// the count or an unbounded sentinel; `maximum_wait_time` uses the word
/// `unlimited` instead and is typed `voip_ms::WaitTime` separately.
const SECONDS_FIELDS: &[&str] = &[
    "announce_position_frecuency",
    "announce_round_seconds",
    "frequency_announcement",
    "member_delay",
    "retry_timer",
    "wrapup_time",
];

/// The named-zone `timezone` params: IANA zone names (`America/New_York`)
/// stored on a mailbox or selecting a `getTimezones` catalog entry. Typed
/// `chrono_tz::Tz` per struct rather than by field name, because the same
/// field name on the CDR / SMS / MMS record-listing methods is a different
/// contract entirely -- a number of hours -- handled by the `OFFSET_OPS`
/// wire transform in `main.rs`. Keyed `"StructName.field"`.
pub(crate) const NAMED_ZONE_TZ_PARAM_PATHS: &[&str] = &[
    "CreateVoicemailParams.timezone",
    "SetVoicemailParams.timezone",
    "GetTimezonesParams.timezone",
];

/// The named-zone *response* fields. Typed `voip_ms::TimezoneName` rather than
/// `Tz`: voip.ms still reports legacy names the IANA database has dropped
/// (`Asia/Beijing`, `US/Pacific-New`, `Factory`, ...), so a response value
/// must be able to carry an unrecognized name verbatim. Params stay strict
/// `Tz` -- the crate never *sends* a name it can't resolve.
pub(crate) const NAMED_ZONE_TZ_RESPONSE_PATHS: &[&str] = &[
    "GetVoicemailsResponseVoicemail.timezone",
    "GetTimezonesResponseTimezone.value",
];

/// The [`FieldOverride`] typing a param as `chrono_tz::Tz`: the IANA name
/// travels on the wire via `serialize_opt_tz`.
pub(crate) fn tz_param_override() -> FieldOverride {
    FieldOverride {
        rust_type: "chrono_tz::Tz".into(),
        param_serializer: Some("crate::responses::serialize_opt_tz".into()),
        ..Default::default()
    }
}

/// The [`FieldOverride`] for a response timestamp whose zone is known only
/// outside the response: a `voip_ms::WallClock`, zoned once `attach_zone` or
/// `attach_offset` has qualified it and bare otherwise.
pub(crate) fn wall_clock_override() -> FieldOverride {
    FieldOverride {
        rust_type: "crate::Reported<crate::WallClock>".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_reported_wall_clock".into()),
        ..Default::default()
    }
}

/// The [`FieldOverride`] typing a response field as `voip_ms::TimezoneName`,
/// which preserves names the IANA database doesn't recognize.
pub(crate) fn tz_response_override() -> FieldOverride {
    FieldOverride {
        rust_type: "crate::TimezoneName".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_from_wire_text::<crate::TimezoneName, _>".into(),
        ),
        ..Default::default()
    }
}

/// The response fields typed `voip_ms::TransactionDate`. A
/// `getTransactionHistory` row reports either when it posted or a span
/// (`2026-08-01 to 2026-08-31`) in the same field, which the doc sample's lone
/// timestamp does not show and a strict `NaiveDateTime` fails the whole
/// response on. Assigned per struct, since `date` elsewhere is a point in time
/// and never a span. Keyed `"StructName.field"`.
///
/// The range is the *requested window*: the report totals each usage-metered
/// charge over the range the call asked for, and that row carries the range in
/// place of a timestamp. `getCharges` and `getDeposits` are the same ledger for
/// a reseller client and are deliberately absent, because neither takes a date
/// range and so neither has a window to report.
pub(crate) const TRANSACTION_DATE_RESPONSE_PATHS: &[&str] =
    &["GetTransactionHistoryResponseTransaction.date"];

/// The [`FieldOverride`] typing a response field as `voip_ms::TransactionDate`,
/// which carries a timestamp, a bare date, a date span, or an unrecognized
/// value verbatim.
pub(crate) fn transaction_date_override() -> FieldOverride {
    FieldOverride {
        rust_type: "crate::TransactionDate".into(),
        response_deserializer: Some("crate::responses::deserialize_opt_transaction_date".into()),
        ..Default::default()
    }
}

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

/// Phone-number identifier fields. A phone number is an identifier, never a
/// quantity -- so it must stay `String`: it can carry leading zeros and exceed
/// `i64` range, and any parse to a number loses information. The WSDL and the
/// doc-sample extractor both under-type these (the WSDL declares the fax
/// methods' `did` and the `setCallback`/`setPhonebook` `number` params as
/// `xsd:integer`; the extractor sees an all-digit sample and infers
/// `integer`), so the override forces `String` uniformly on both the param
/// and response side. Every field with one of these names carries a phone
/// number in every method that has it. `DIDAdded` / `DIDRemoved` /
/// `deleted_did` each report the single DID an `assignDIDvPRI`,
/// `removeDIDvPRI`, or `cancelFaxNumber` call acted on -- a DID, not the count
/// the past-participle name suggests.
///
/// Entries are the *wire* field name (the resolver matches before the
/// snake-case ident is derived), so a camelCase wire name is written as-is
/// (`DIDAdded`, not `did_added`). Deliberately excluded:
///
/// * the plural `dids` -- variously a list of DID objects or of numeric vPRI
///   ids in responses, which a blanket `String` override would wrongly
///   flatten;
/// * `from` -- a date filter in the `getSMS`-family params but an email
///   address in `getEmailToFax`'s response, so it keeps per-method handling.
const PHONE_STRING_FIELDS: &[&str] = &[
    "DIDAdded",
    "DIDRemoved",
    "contact",
    "deleted_did",
    "destination",
    "did",
    "number",
    "phone_number",
    "stationid",
];

/// Opaque identifier fields that arrive as an all-digit sample -- so the
/// extractor infers `integer` -- but are not numbers: a `getCDR` /
/// `getResellerCDR` call's `uniqueid` can be alphanumeric (e.g.
/// `12964421x41098i8c`), which no integer type can hold. Forced to `String`
/// with the same tolerant deserializer as [`PHONE_STRING_FIELDS`], since the
/// wire value may still be a bare number.
const ID_STRING_FIELDS: &[&str] = &["uniqueid"];

/// Params whose value is a base64-encoded file, keyed `"wireMethod.field"`.
/// A method with one of these is sent as a `multipart/form-data` POST; every
/// other method is a GET.
///
/// A GET would put the value on a query string, which the 8190-byte request
/// line (Apache's default `LimitRequestLine`) leaves roughly 8 kB of -- about a
/// third of a second of 8 kHz mono audio for `setRecording`. `addLNPFile` is
/// documented "Only accepted through POST request", so no size works for it.
pub(crate) const BASE64_FILE_PARAM_PATHS: &[&str] = &[
    "addLNPFile.file",
    "sendFaxMessage.file",
    "sendMMS.media2",
    "setRecording.file",
];

/// Calendar-date fields, documented uniformly as `'YYYY-MM-DD'`
/// (`Example: '2010-11-30'`). Typed `chrono::NaiveDate`, whose own
/// `Serialize` emits exactly that wire form, instead of the WSDL's
/// `xsd:string` -- on the param side, which is the only side that writes. A
/// response reads the same field as `voip_ms::Reported<chrono::NaiveDate>`,
/// like every other response date.
/// `reseller_nextbilling` is here so `getSubAccounts` and
/// `setSubAccount` agree on it; the other two are the date-range filters. The
/// bare `date` field is deliberately excluded -- it is a datetime in some
/// responses (`getLNPDetails`) and a date in others, so no single type fits.
const DATE_FIELDS: &[&str] = &["date_from", "date_to", "reseller_nextbilling"];

/// Numeric id / code fields the WSDL declares as `xsd:string` on the write
/// side while the doc samples infer `integer` on the read side, so a caller who
/// listed a record and then updated it had to convert each one by hand. Typed
/// `u64` on both sides, which is what every response already reported -- only
/// the param side moves, so no response gains a way to fail.
///
/// Most are voip.ms's own record ids, documented uniformly as "ID for a
/// specific X (Example: 4636)". Entries are the *wire* field name, which is
/// why `ring_group` appears twice: `delRingGroup` spells it `ringgroup` (see
/// `FIELD_IDENT_OVERRIDE`). `international_route` is listed although it already
/// agreed, so the two route codes cannot drift apart again.
///
/// The recording-code slots (`agent_announcement`, `caller_announcement`,
/// `voice_announcement`, `unavailable_message_recording`) are here although
/// some document "a recording code *or* the word `none`". Confirmed live on a
/// ring group: `none` and `0` are interchangeable on the way in, and the read
/// side reports `0` either way, so `u64` loses nothing and `Some(0)` clears
/// the slot.
///
/// `mailbox` is here rather than in [`IDENTIFIER_STRING_FIELDS`] although
/// `createVoicemail` documents its `digits` as "Example: 01". Confirmed live:
/// creating a box with `digits=01` yields mailbox `1`, so voip.ms normalizes
/// the leading zero away and there is none to preserve.
const U64_FIELDS: &[&str] = &[
    "agent_announcement",
    "call_hunting",
    "callback",
    "caller_announcement",
    "canada_routing",
    "client",
    "conference",
    "disa",
    "filtering",
    "forwarding",
    "group",
    "internal_dialtime",
    "internal_extension",
    "internal_voicemail",
    "international_route",
    "ivr",
    "mailbox",
    "member",
    "phonebook",
    "priority_weight",
    "queue",
    "recording",
    "reseller_client",
    "reseller_package",
    "ring_group",
    "ringgroup",
    "sipuri",
    "timecondition",
    "unavailable_message_recording",
    "voice_announcement",
    "voicemail",
];

/// `setConference`'s 20 prompt slots, each documented as "the recording
/// played when ... (Values from getRecordings)" and reported by
/// `getConference` as a numeric code. Same correction as [`U64_FIELDS`],
/// listed apart only because they are one family with one rationale.
///
/// Unlike the queue's `agent_announcement` / `voice_announcement`, none
/// documents the word `none` as an alternative, so an integer holds every
/// documented value.
const CONFERENCE_PROMPT_FIELDS: &[&str] = &[
    "sound_error_menu",
    "sound_get_pin",
    "sound_has_joined",
    "sound_has_left",
    "sound_invalid_pin",
    "sound_join",
    "sound_kicked",
    "sound_leave",
    "sound_locked",
    "sound_locked_now",
    "sound_muted",
    "sound_only_one",
    "sound_only_person",
    "sound_other_in_party",
    "sound_participants_muted",
    "sound_participants_unmuted",
    "sound_place_into_conference",
    "sound_there_are",
    "sound_unlocked_now",
    "sound_unmuted",
];

/// Fields that are identifiers or free text, not quantities, but that an
/// all-digit doc sample made the extractor infer as `integer` on the response
/// side. Forced to `String` everywhere, which is what the param side already
/// was, so the read side only becomes more tolerant.
///
/// Each would lose information as a number:
///
/// * `zip` -- a US ZIP has leading zeros (`02134`) and a Canadian postal code
///   is alphanumeric (`M5V 3A8`);
/// * `password` -- a voicemail PIN of `0123` is not 123;
/// * `security_code` -- `setEmailToFax` documents it as "an alphanumeric code";
/// * `dtmf_digits` -- a dial string carries `*`, `#`, and pause characters;
/// * `callerid_prefix` -- `getDIDsInfo` reports `MIA [555]`.
const IDENTIFIER_STRING_FIELDS: &[&str] = &[
    "callerid_prefix",
    "dtmf_digits",
    "password",
    "security_code",
    "zip",
];

/// Fractional-second durations. `setForwarding`'s `pause` is documented
/// "Example: 1.5 / Values: 0 to 10 in increments of 0.5", which the WSDL
/// declares `xsd:string` while the response sample infers `decimal`.
const DECIMAL_FIELDS: &[&str] = &["pause"];

/// Counts documented as a number *or* the word `unlimited`, which is
/// `voip_ms::WaitTime`'s exact wire contract -- the type is selected by that
/// spelling rather than by its name, since `Seconds` writes `none` and
/// `MaxMembers` writes a capitalized `Unlimited`.
const WAIT_TIME_FIELDS: &[&str] = &["maximum_wait_time", "maximum_callers"];

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
        response_deserializer: Some(
            "crate::responses::deserialize_opt_via::<crate::Seconds, _>".into(),
        ),
        ..Default::default()
    };
    let wait_time = FieldOverride {
        rust_type: "crate::WaitTime".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_via::<crate::WaitTime, _>".into(),
        ),
        ..Default::default()
    };
    // getConference reports `max_members` as a count or the word `Unlimited`;
    // like Seconds/WaitTime it carries its own Serialize, so no param_serializer.
    let max_members = FieldOverride {
        rust_type: "crate::MaxMembers".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_via::<crate::MaxMembers, _>".into(),
        ),
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
    // NaiveDate's own Serialize emits the `%Y-%m-%d` wire form, so no
    // param_serializer. The response side reads a `Reported`, like every other
    // response date: the param is written and cannot receive a surprise, the
    // response can.
    let date = FieldOverride {
        rust_type: "chrono::NaiveDate".into(),
        response_rust_type: Some("crate::Reported<chrono::NaiveDate>".into()),
        response_deserializer: Some("crate::responses::deserialize_opt_reported_date".into()),
        ..Default::default()
    };
    // A phone number stays `String` on both the param and response side. On
    // the response side VoIP.ms may send it as a bare JSON number, so it keeps
    // the tolerant string deserializer -- dropping it would reintroduce drift
    // on a numeric wire value.
    let phone_string = FieldOverride {
        rust_type: "String".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_string_from_string_number_or_bool".into(),
        ),
        ..Default::default()
    };
    // A numeric id whose wire form is a bare number either way, so the param
    // side needs no serializer; only the response side has to tolerate the
    // numeric string voip.ms may send instead.
    let numeric_id = FieldOverride {
        rust_type: "u64".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_u64_from_string_or_number".into(),
        ),
        ..Default::default()
    };
    // Decimal's own Serialize emits the bare number the wire wants, so no
    // param_serializer; the response side tolerates a numeric string.
    let decimal = FieldOverride {
        rust_type: "rust_decimal::Decimal".into(),
        response_deserializer: Some(
            "crate::responses::deserialize_opt_decimal_from_string_or_number".into(),
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
        .chain(
            WAIT_TIME_FIELDS
                .iter()
                .map(|name| (*name, wait_time.clone())),
        )
        .chain(std::iter::once(("max_members", max_members)))
        .chain(
            CALLERID_OVERRIDE_FIELDS
                .iter()
                .map(|name| (*name, callerid_override.clone())),
        )
        .chain(DATE_FIELDS.iter().map(|name| (*name, date.clone())))
        .chain(
            U64_FIELDS
                .iter()
                .chain(CONFERENCE_PROMPT_FIELDS.iter())
                .map(|name| (*name, numeric_id.clone())),
        )
        .chain(DECIMAL_FIELDS.iter().map(|name| (*name, decimal.clone())))
        .chain(
            PHONE_STRING_FIELDS
                .iter()
                .chain(ID_STRING_FIELDS.iter())
                .chain(IDENTIFIER_STRING_FIELDS.iter())
                .map(|name| (*name, phone_string.clone())),
        )
        .collect()
}
