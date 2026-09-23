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

mod check_flags;
mod check_types;
mod dump_fields;
mod dump_methods;
mod extract;
mod field_overrides;
mod overrides;
mod response_codegen;
mod wsdl;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
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

/// Field identifier renames, keyed by `(struct name, wire field name)` mapping
/// to the Rust field identifier to emit instead of the one
/// [`rust_field_ident`] derives. The wire name still travels on the wire (a
/// `#[serde(rename)]` falls out because the ident now differs from it). Applies
/// on both the `*Params` and the `*Response` side.
///
/// Two things earn an entry:
///
/// * genuine cross-method inconsistency in the upstream WSDL, where one method
///   names an id differently than its siblings and a consumer who just read or
///   set the value naturally reuses the sibling's name;
/// * a wire field named `type`, which [`rust_field_ident`] can only escape as
///   `r#type`. What it holds differs per struct -- a search mode, a message
///   direction, a file format -- so each gets the word for what it is.
const FIELD_IDENT_OVERRIDE: &[(&str, &str, &str)] = &[
    // `delRingGroup` names the id `ringgroup` while `getRingGroups` and
    // `setRingGroup` use `ring_group`; align the delete with the family so a
    // caller who just listed or configured a group reuses the same field.
    ("DelRingGroupParams", "ringgroup", "ring_group"),
    // The reference-data lookups: `type` narrows the catalog to one entry, and
    // the docs call the value a code ("Code for a specific Address Type").
    ("E911AddressTypesParams", "type", "code"),
    ("GetAuthTypesParams", "type", "code"),
    ("GetInternationalTypesParams", "type", "code"),
    ("GetJoinWhenEmptyTypesParams", "type", "code"),
    ("GetReportEstimatedHoldTimeParams", "type", "code"),
    // Not a country code: it selects which kind of international DID to list.
    ("GetDIDCountriesParams", "type", "international_type"),
    // The DID / toll-free searches: how the pattern is matched.
    ("SearchDIDsCANParams", "type", "search_type"),
    ("SearchDIDsUSAParams", "type", "search_type"),
    ("SearchTollFreeCANUSParams", "type", "search_type"),
    ("SearchTollFreeUSAParams", "type", "search_type"),
    ("SearchVanityParams", "type", "vanity_type"),
    // Messaging: sent or received.
    ("GetMMSParams", "type", "direction"),
    ("GetResellerMMSParams", "type", "direction"),
    ("GetResellerSMSParams", "type", "direction"),
    ("GetSMSParams", "type", "direction"),
    ("GetMMSResponseSMS", "type", "direction"),
    ("GetResellerMMSResponseSMS", "type", "direction"),
    ("GetResellerSMSResponseSMS", "type", "direction"),
    ("GetSMSResponseSMS", "type", "direction"),
    // Call recordings report `Incoming` / `Outgoing`.
    ("GetCallRecordingResponse", "type", "direction"),
    ("GetCallRecordingsResponseRecording", "type", "direction"),
    // A porting attachment's `type` is its file format (`pdf`).
    ("GetLNPAttachResponse", "type", "file_type"),
    ("GetLNPAttachListResponseList", "type", "file_type"),
    ("GetLNPDetailsResponseAttachment", "type", "file_type"),
    (
        "GetTransactionHistoryResponseTransaction",
        "type",
        "transaction_type",
    ),
];

/// The Rust field identifier for a wire field, applying any
/// [`FIELD_IDENT_OVERRIDE`] and otherwise deriving it with [`rust_field_ident`].
pub(crate) fn field_ident(struct_name: &str, fname: &str, acronyms: &[&'static str]) -> String {
    FIELD_IDENT_OVERRIDE
        .iter()
        .find(|(s, f, _)| *s == struct_name && *f == fname)
        .map(|(_, _, ident)| (*ident).to_string())
        .unwrap_or_else(|| rust_field_ident(fname, acronyms))
}

/// A record-listing method whose `timezone` param is a number (`-12..=13`) on
/// the wire but a named `chrono_tz::Tz` in the public params.
///
/// For each of these, the generator emits a private `*ParamsWire` twin struct
/// (identical fields, `timezone` as a `crate::TimezoneOffset`) plus a
/// `TryFrom<&*Params>` that picks the number with
/// `TimezoneOffset::for_window` at the query start date, and routes the
/// `Client` method bodies through it. The public struct still derives
/// `Serialize` -- there `timezone` emits the IANA name, which is what a log or
/// JSON dump should show; only the wire twin carries the number.
///
/// VoIP.ms reports each timestamp as its server-zone wall clock shifted by
/// `timezone + 5` hours. Every `datetime` scalar in these methods' response
/// shapes becomes a `crate::Reported<crate::WallClock>`, and the typed method
/// routes through `Client::call_zoned`, which undoes the shift for the number
/// sent and resolves each row in `crate::SERVER_ZONE`.
struct OffsetOp {
    /// The wire method (e.g. `getCDR`).
    wire: &'static str,
    /// The sibling start-date param that anchors the DST resolution.
    start_field: &'static str,
    /// The end-date param, which anchors it when there is no start date.
    end_field: &'static str,
    /// Whether the date params are `chrono::NaiveDate` (the CDR methods) rather
    /// than `'YYYY-MM-DD'` strings (the SMS/MMS methods).
    start_is_date: bool,
}

const OFFSET_OPS: &[OffsetOp] = &[
    OffsetOp {
        wire: "getCDR",
        start_field: "date_from",
        end_field: "date_to",
        start_is_date: true,
    },
    OffsetOp {
        wire: "getResellerCDR",
        start_field: "date_from",
        end_field: "date_to",
        start_is_date: true,
    },
    OffsetOp {
        wire: "getSMS",
        start_field: "from",
        end_field: "to",
        start_is_date: false,
    },
    OffsetOp {
        wire: "getMMS",
        start_field: "from",
        end_field: "to",
        start_is_date: false,
    },
    OffsetOp {
        wire: "getResellerSMS",
        start_field: "from",
        end_field: "to",
        start_is_date: false,
    },
    OffsetOp {
        wire: "getResellerMMS",
        start_field: "from",
        end_field: "to",
        start_is_date: false,
    },
];

fn offset_op(wire: &str) -> Option<&'static OffsetOp> {
    OFFSET_OPS.iter().find(|o| o.wire == wire)
}

/// A method whose response timestamps are wall clocks rendered in a named zone
/// that the request cannot choose and the response does not report.
///
/// Every `datetime` scalar in its response shape becomes a
/// `crate::Reported<crate::WallClock>`, which reads a bare wall clock as well
/// as a qualified one, and a `zone_timestamps` arm names the paths and the zone
/// source for a raw caller's `attach_zone`.
struct ZoneOp {
    /// The wire method (e.g. `getVoicemailMessages`).
    wire: &'static str,
    source: ZoneSource,
    /// A measured fact about how a param relates to the zone, appended to the
    /// param's mined doc as `(param, note)`.
    param_notes: &'static [(&'static str, &'static str)],
}

/// Where a [`ZoneOp`]'s zone comes from, which decides the methods emitted.
enum ZoneSource {
    /// `crate::SERVER_ZONE`. The plain typed method qualifies the timestamps
    /// itself, through `Client::call_in_zone`.
    Server,
    /// A zone the caller supplies, described by a doc fragment completing
    /// "VoIP.ms renders the timestamps in". The plain typed method leaves the
    /// timestamps bare, and an `*_in_zone` sibling takes the zone.
    Supplied(&'static str),
}

/// How `getVoicemailMessages` matches its date window, measured against the
/// live API: with the mailbox set to `America/Toronto` a message at
/// `2026-08-24 21:25:11` (01:25 UTC the next day) matched `2026-08-24` and not
/// `2026-08-25`, so the window is not UTC; with the mailbox moved to
/// `Europe/Berlin` the same message read `2026-08-25 03:25:11` and still
/// matched `2026-08-24` only, so the window is not the mailbox's zone either.
/// Both readings fit Eastern days, and no message on the account fell in the
/// hour that would tell DST-observing Eastern from a fixed UTC-5.
const VOICEMAIL_WINDOW_NOTE: &str = "Not matched in the mailbox's zone, so a message can \
     match a day other than the one its reported `date` shows. The window was measured \
     to be Eastern days, not UTC ones; whether those days observe DST, as \
     `SERVER_ZONE` does, was not measured.";

/// Each [`ZoneSource::Server`] entry was measured against the live API during
/// DST with an independent reference instant, and each reported the
/// `SERVER_ZONE` wall clock (UTC-04:00), not a fixed UTC-05:00:
///
/// * `getRegistrationStatus`: a SIP registration with a 600 s interval made at
///   14:40:30 UTC reported `register_next` `10:50:30`.
/// * `getCallRecordings` / `getCallRecording`: a call at 16:28:34 UTC reported
///   `datetime` `12:28:34`, and the recording's id embeds that Unix time.
/// * `getDIDsInfo`: a DID ordered between 17:19:00 and 17:19:32 UTC reported
///   `order_date` `13:19:31`.
/// * `getFaxMessages`: a fax sent between 17:24:15 and 17:24:33 UTC reported
///   `date` `13:24:33`.
/// * `getMediaMMS`: an MMS sent between 17:20:47 and 17:21:40 UTC reported
///   `date` `13:21:39`.
///
/// `getBackOrders`, `getLNPDetails` and `getConferenceRecordings` are absent
/// because they were not measured: a back order cannot be canceled through
/// the API, a port request is a real carrier filing, and a conference
/// recording cannot be enabled through the API.
const ZONE_OPS: &[ZoneOp] = &[
    ZoneOp {
        wire: "getCallRecording",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getCallRecordings",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getDIDsInfo",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getFaxMessages",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getMediaMMS",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getRegistrationStatus",
        source: ZoneSource::Server,
        param_notes: &[],
    },
    ZoneOp {
        wire: "getVoicemailMessages",
        source: ZoneSource::Supplied(
            "the mailbox's current `timezone` setting (`getVoicemails`' `timezone`), \
             at the time they are read",
        ),
        param_notes: &[
            ("date_from", VOICEMAIL_WINDOW_NOTE),
            ("date_to", VOICEMAIL_WINDOW_NOTE),
        ],
    },
];

fn zone_op(wire: &str) -> Option<&'static ZoneOp> {
    ZONE_OPS.iter().find(|o| o.wire == wire)
}

/// The const naming an offset op's response timestamp paths
/// (`getCDR` -> `GET_CDR_TIMESTAMPS`).
fn timestamps_const_name(wire: &str, acronyms: &[&'static str]) -> String {
    format!(
        "{}_TIMESTAMPS",
        camel_to_snake(wire, acronyms).to_uppercase()
    )
}

/// Emit the per-op `*_TIMESTAMPS` consts, each documented as the path form of
/// `attach`, the public helper that qualifies them. The typed methods pass them
/// to `Client::call_zoned` or `Client::call_in_zone`. They are private: the
/// lookups [`emit_offset_timestamps`] and [`emit_zone_timestamps`] write are the
/// one public route to the same paths.
fn emit_timestamp_consts(
    timestamps: &BTreeMap<String, Vec<String>>,
    attach: &str,
    acronyms: &[&'static str],
) -> String {
    let mut out = String::new();
    for (op, paths) in timestamps {
        let rendered = paths
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "\n/// Paths to the timestamps in a `{op}` response, in the form\n\
             /// [`{attach}`](crate::{attach}) takes.\n\
             const {}: &[&str] = &[{rendered}];\n",
            timestamps_const_name(op, acronyms),
        ));
    }

    out
}

/// Emit the public `offset_timestamps` lookup, answering each offset op's wire
/// name with the const [`emit_timestamp_consts`] writes for it.
fn emit_offset_timestamps(
    zoned_timestamps: &BTreeMap<String, Vec<String>>,
    acronyms: &[&'static str],
) -> String {
    let doc = "\n/// The paths [`attach_offset`](crate::attach_offset) needs to qualify\n\
               /// `method`'s response timestamps, or `None` for a method whose response\n\
               /// reports none shifted by the `timezone` the request carried.\n\
               ///\n\
               /// `Some` for exactly the record-listing methods. A generated method\n\
               /// qualifies them on its own; a raw envelope, such as\n\
               /// [`Client::call_raw`] returns, reports them as shifted wall clocks.\n\
               ///\n\
               /// **A method this answers `Some` for must be sent an explicit\n\
               /// `timezone`**, and `attach_offset` must be given the same number:\n\
               /// the shift it undoes is the one that number caused. `getCDR` and\n\
               /// `getResellerCDR` reject a request that omits it, and `getSMS` /\n\
               /// `getMMS` treat an omitted one as `-5`.\n\
               ///\n\
               /// Like [`requires_multipart`], it answers only for the methods this crate\n\
               /// was generated from: a method VoIP.ms has added since answers `None`.\n";
    let arms = zoned_timestamps
        .keys()
        .map(|op| (op.as_str(), timestamps_const_name(op, acronyms)))
        .collect::<Vec<_>>();
    emit_timestamp_lookup("offset_timestamps", doc, "&'static [&'static str]", &arms)
}

/// Emit the public `zone_timestamps` lookup, answering each zone op's wire name
/// with its zone source and the const [`emit_timestamp_consts`] writes for it.
fn emit_zone_timestamps(
    zone_timestamps: &BTreeMap<String, Vec<String>>,
    acronyms: &[&'static str],
) -> String {
    let doc = "\n/// Where `method`'s response timestamps get their zone and the paths\n\
               /// [`attach_zone`](crate::attach_zone) needs to qualify them, or `None` for a\n\
               /// method whose response reports none in a named zone.\n\
               ///\n\
               /// `Some` for exactly the methods whose response timestamps are wall clocks\n\
               /// in a named zone the request cannot choose and the response does not\n\
               /// name. A raw envelope, such as [`Client::call_raw`] returns, reports them\n\
               /// bare: pass the zone ([`TimestampZone::zone`](crate::TimestampZone::zone),\n\
               /// or the one the caller holds for\n\
               /// [`TimestampZone::Supplied`](crate::TimestampZone::Supplied)) and these\n\
               /// paths to `attach_zone` before deserializing.\n\
               ///\n\
               /// Like [`requires_multipart`], it answers only for the methods this crate\n\
               /// was generated from: a method VoIP.ms has added since answers `None`.\n";
    let arms = zone_timestamps
        .keys()
        .map(|op| {
            let zone = match zone_op(op).map(|z| &z.source) {
                Some(ZoneSource::Server) => "Server",
                Some(ZoneSource::Supplied(_)) => "Supplied",
                None => unreachable!("{op} has zone timestamps but is not a zone op"),
            };
            (
                op.as_str(),
                format!(
                    "crate::ZoneTimestamps {{ zone: crate::TimestampZone::{zone}, paths: {} }}",
                    timestamps_const_name(op, acronyms)
                ),
            )
        })
        .collect::<Vec<_>>();
    emit_timestamp_lookup("zone_timestamps", doc, "crate::ZoneTimestamps", &arms)
}

/// Emit a public `name(method) -> Option<ty>` lookup answering each wire name in
/// `arms` with its expression, under `doc`.
fn emit_timestamp_lookup(name: &str, doc: &str, ty: &str, arms: &[(&str, String)]) -> String {
    if arms.is_empty() {
        // A `match` holding only the wildcard arm trips clippy's
        // `match_single_binding` in a consumer's build.
        return format!(
            "{doc}pub fn {name}(_method: &str) -> Option<{ty}> {{\n    \
                 None\n\
             }}\n"
        );
    }

    let arms = arms
        .iter()
        .map(|(op, expr)| format!("        {op:?} => Some({expr}),\n"))
        .collect::<String>();
    format!(
        "{doc}pub fn {name}(method: &str) -> Option<{ty}> {{\n    \
             match method {{\n\
             {arms}        \
                 _ => None,\n    \
             }}\n\
         }}\n"
    )
}

/// What [`base64_file_params`] read out of the table and the mined docs.
#[derive(Debug)]
struct Base64FileParams {
    /// File parameters by wire method, as the emitter consumes them.
    by_op: BTreeMap<String, Vec<String>>,
    /// `wireMethod.field` paths the docs describe as base64 that the table does
    /// not list. A value rather than a printed warning, so it can be asserted
    /// on; `cmd_gen` is what reports it.
    unlisted: Vec<String>,
}

/// Group `paths` by wire method for the emitter, failing on one the WSDL has no
/// field for or one naming an offset op.
///
/// A parameter the docs call base64 that `paths` omits is returned in
/// [`Base64FileParams::unlisted`] rather than failing: that reading comes from
/// mined HTML and wants a human to confirm it is really a file.
fn base64_file_params(
    wsdl: &Wsdl,
    param_docs: &ParamDocs,
    paths: &[&str],
) -> Result<Base64FileParams, String> {
    let mut by_op: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in paths {
        let (op, field) = path
            .rsplit_once('.')
            .filter(|(op, field)| !op.is_empty() && !field.is_empty())
            .ok_or_else(|| {
                format!("BASE64_FILE_PARAM_PATHS entry `{path}` must be `wireMethod.field`")
            })?;
        let declared = wsdl
            .types
            .get(&format!("{op}Input"))
            .is_some_and(|fields| fields.iter().any(|(name, _)| name == field));
        if !declared {
            return Err(format!(
                "BASE64_FILE_PARAM_PATHS entry `{path}` names no input field of `{op}`; \
                 correct or remove it in xtask/src/field_overrides.rs"
            ));
        }

        // Neither check above catches this: the entry is well-formed and
        // listed, and the op would silently start posting its `*ParamsWire`
        // twin as a multipart form, a combination nothing has exercised.
        if offset_op(op).is_some() {
            return Err(format!(
                "BASE64_FILE_PARAM_PATHS entry `{path}` names an offset op, whose wire twin \
                 has only ever been sent over GET; confirm it survives a multipart body \
                 before listing it"
            ));
        }

        by_op.entry(op.to_string()).or_default().push(field.into());
    }

    let mut unlisted = Vec::new();
    for (op, fields) in param_docs {
        for (field, doc) in fields {
            let listed = by_op.get(op).is_some_and(|fs| fs.contains(field));
            if !listed && documents_base64(doc) {
                unlisted.push(format!("{op}.{field}"));
            }
        }
    }

    Ok(Base64FileParams { by_op, unlisted })
}

/// Whether a parameter description documents a base64-encoded value.
/// Whitespace-insensitive, so `Base 64` and `Base64` both read.
fn documents_base64(doc: &str) -> bool {
    let squished: String = doc
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    squished.contains("base64")
}

/// Params fields the docs mark `(required)` that a `new` constructor must not
/// ask for, keyed `"wireMethod.field"`.
///
/// The offset ops' `timezone` is the only case: the docs mark it required, and
/// the crate sends UTC when a caller names no zone, so a constructor demanding
/// one would contradict the method it builds for.
const REQUIRED_CTOR_SKIP: &[&str] = &[
    "getCDR.timezone",
    "getMMS.timezone",
    "getResellerCDR.timezone",
    "getResellerMMS.timezone",
    "getResellerSMS.timezone",
    "getSMS.timezone",
];

/// The most required fields a positional constructor stays readable at.
/// `addLNPPort` has 12 and `createVoicemail` 11; past this bound an unlabeled
/// argument list reads worse than the struct literal, so no `new` is emitted
/// and the caller writes the fields out.
const MAX_CTOR_FIELDS: usize = 6;

/// Emit `impl {struct_name} { pub fn new(..) }` taking the parameters the
/// mined docs mark `(required)`, or nothing when there are none or too many.
///
/// Every field stays `Option` and the struct-update pattern keeps working; the
/// constructor only spares a caller from guessing which fields VoIP.ms needs.
fn emit_params_constructor(
    struct_name: &str,
    op: &str,
    body_fields: &[&(String, String)],
    param_docs: &ParamDocs,
    resolver: &field_overrides::Resolver,
    acronyms: &[&'static str],
) -> String {
    let docs = param_docs.get(op);
    let required: Vec<(String, String, bool)> = body_fields
        .iter()
        .copied()
        .filter(|(fname, _)| {
            !REQUIRED_CTOR_SKIP.contains(&format!("{op}.{fname}").as_str())
                && docs
                    .and_then(|d| d.get(fname))
                    .is_some_and(|doc| doc.contains("(required)"))
        })
        .map(|(fname, ftype)| {
            let override_ = resolver.resolve(struct_name, fname, true);
            let ty = match override_ {
                Some(o) => o.rust_type.clone(),
                None => xsd_to_rust(ftype).to_string(),
            };
            let bare = override_.is_some_and(|o| o.param_skip_if.is_some());
            (field_ident(struct_name, fname, acronyms), ty, bare)
        })
        .collect();
    if required.is_empty() || required.len() > MAX_CTOR_FIELDS {
        return String::new();
    }

    let args = required
        .iter()
        .map(|(ident, ty, _)| {
            if ty == "String" {
                format!("{ident}: impl Into<String>")
            } else {
                format!("{ident}: {ty}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut assignments = String::new();
    for (ident, ty, bare) in &required {
        let value = if ty == "String" {
            format!("{ident}.into()")
        } else {
            ident.clone()
        };
        if *bare {
            assignments.push_str(&format!("            {ident}: {value},\n"));
        } else {
            assignments.push_str(&format!("            {ident}: Some({value}),\n"));
        }
    }

    // An all-required struct has nothing left for the struct-update to fill,
    // and `clippy::needless_update` says so.
    if required.len() < body_fields.len() {
        assignments.push_str("            ..Default::default()\n");
    }

    let named: Vec<String> = required
        .iter()
        .map(|(ident, _, _)| format!("`{ident}`"))
        .collect();
    let listed = match named.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, head)) => format!("{} and {last}", head.join(", ")),
        None => unreachable!("an empty list returned above"),
    };

    let mut out = format!("\nimpl {struct_name} {{\n");
    // Through `render_doc` for the wrapping, which is also why the text carries
    // no intra-doc link: it escapes brackets, since the mined parameter
    // descriptions it usually renders are full of prose like `[Required]`.
    render_doc(
        &mut out,
        "    ",
        &format!(
            "A `{struct_name}` with the parameters the VoIP.ms docs mark as required: \
             {listed}. Every other field keeps its default, so struct-update syntax \
             still fills in the rest."
        ),
    );
    out.push_str("    ///\n");
    render_doc(
        &mut out,
        "    ",
        "The marking is the docs' word, not the API's: a field asked for here may still \
         be optional in practice, and one left out may turn out to be needed.",
    );
    out.push_str(&format!(
        "    pub fn new({args}) -> Self {{\n        \
             Self {{\n{assignments}        \
             }}\n    \
         }}\n\
         }}\n"
    ));
    out
}

/// Doc emitted on the offset ops' public `timezone` field in place of the
/// mined upstream text, which describes the numeric wire form ("Numeric: -12
/// to 13") the public `Tz` field is not.
const OFFSET_TIMEZONE_DOC: &str = "IANA time zone whose days the date range means \
     (Example: 'America/New_York'); resolved at the query start date, or the end date when \
     there is no start, to the number VoIP.ms needs for that window, since it shifts its \
     Eastern wall clocks as if Eastern were always UTC-5. Omit for UTC days. The reported \
     timestamps are qualified in the server zone whatever is sent.";

/// Rust type for a WSDL param type. Integers map to `u64`, matching the
/// response side: every VoIP.ms integer param is a non-negative id or count
/// (the documented `-1` sentinels are enum-typed via `field_types`), and the
/// ids read from responses are `u64`, so `i64` would force a cast on every
/// round-trip. Decimals map to `rust_decimal::Decimal` -- the decimal params
/// are money amounts (`charge`, `payment`, `setup`, ...), which `f64` would
/// serialize with float artifacts.
fn xsd_to_rust(t: &str) -> &'static str {
    match t {
        "xsd:string" => "String",
        "xsd:integer" => "u64",
        "xsd:boolean" => "bool",
        "xsd:decimal" => "rust_decimal::Decimal",
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

/// Rust identifier for a wire field name: snake_cased through the
/// acronym-aware tokenizer (`isMobile` -> `is_mobile`, `rateCenter` ->
/// `rate_center`), then keyword-escaped (`type` -> `r#type`); a name that
/// still isn't identifier-shaped (e.g. all digits) gains a `field_` prefix
/// with unsafe characters replaced. Callers emit a serde `rename` back to
/// the wire name whenever the result (minus any `r#`) differs from it.
pub(crate) fn rust_field_ident(name: &str, acronyms: &[&'static str]) -> String {
    if name.is_empty() {
        return "field_empty".into();
    }

    let snake = camel_to_snake(name, acronyms);
    if is_rust_keyword(&snake) {
        return format!("r#{snake}");
    }

    if is_rust_identifier(&snake) {
        return snake;
    }

    let mut out = String::with_capacity(snake.len() + 6);
    out.push_str("field_");
    for c in snake.chars() {
        if is_ident_char(c) {
            out.push(c);
        } else {
            out.push('_');
        }
    }

    out
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
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
    )
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

/// The one status the error-code table does not list, because it is not an
/// error. It is a variant so a typed response's `status` field names the
/// ordinary case instead of landing in `voip_ms::ApiStatus::Unknown`.
const SUCCESS_STATUS: (&str, &str) = ("success", "The request succeeded");

/// Emit the `ApiStatus` enum: a `Success` variant, one PascalCase variant per
/// documented wire code (carrying its description as a doc comment), and an
/// `Unknown(String)` catch-all, with
/// `as_wire`/`from_wire`/`description`/`is_documented`/`is_empty_collection`
/// and the `FromStr`/`Display`/`Deserialize` impls. The wire strings are preserved verbatim (including the rare
/// capitalized codes); only the variant *identifiers* are normalized.
fn emit_statuses(statuses: &[(String, String)], empty: &BTreeSet<String>) -> String {
    if statuses.is_empty() {
        return String::new();
    }

    let acronyms = acronyms_sorted();
    // `Success` leads the list so every match arm below covers it without a
    // special case; it is the one status that is not an error, and the
    // error-code table does not list it.
    let variants: Vec<(String, String, String)> = std::iter::once((
        "Success".to_string(),
        SUCCESS_STATUS.0.to_string(),
        SUCCESS_STATUS.1.to_string(),
    ))
    .chain(statuses.iter().map(|(code, desc)| {
        (
            status_variant_name(code, &acronyms),
            code.clone(),
            desc.clone(),
        )
    }))
    .collect();

    let mut out = String::new();

    // Enum declaration.
    out.push_str(
        "\n/// A `status` returned by the VoIP.ms API.\n\
         ///\n\
         /// Every documented error code from the official API docs' global\n\
         /// error-code table is a variant, alongside [`ApiStatus::Success`];\n\
         /// [`ApiStatus::description`] returns a variant's documented meaning. The\n\
         /// set of codes is documentation, not a stable contract -- a code VoIP.ms\n\
         /// returns but hasn't documented is preserved verbatim in\n\
         /// [`ApiStatus::Unknown`] rather than lost.\n\
         ///\n\
         /// ```\n\
         /// # use voip_ms::ApiStatus;\n\
         /// let status = ApiStatus::from_wire(\"invalid_credentials\");\n\
         /// assert_eq!(status, ApiStatus::InvalidCredentials);\n\
         /// assert_eq!(status.as_wire(), \"invalid_credentials\");\n\
         /// assert_eq!(status.description(), Some(\"Username or Password is incorrect\"));\n\
         /// assert!(status.is_documented());\n\
         ///\n\
         /// let unknown = \"some_new_code\".parse::<ApiStatus>().unwrap();\n\
         /// assert_eq!(unknown, ApiStatus::Unknown(\"some_new_code\".to_string()));\n\
         /// assert_eq!(unknown.description(), None);\n\
         /// assert!(!unknown.is_documented());\n\
         /// ```\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\n\
         pub enum ApiStatus {\n",
    );
    for (variant, code, desc) in &variants {
        out.push_str(&format!("    /// `{code}` -- {desc}\n"));
        out.push_str(&format!("    {variant},\n"));
    }
    out.push_str("    /// A `status` value not present in the documented table,\n");
    out.push_str("    /// preserved verbatim.\n");
    out.push_str("    Unknown(String),\n");
    out.push_str("}\n\n");

    out.push_str("impl ApiStatus {\n");

    // as_wire
    out.push_str("    /// The verbatim wire `status` string.\n");
    out.push_str("    pub fn as_wire(&self) -> &str {\n");
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

    // is_empty_collection
    let empty_variants: Vec<&String> = variants
        .iter()
        .filter(|(_, code, _)| empty.contains(code))
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
    out.push_str("    pub fn is_empty_collection(&self) -> bool {\n");
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
                 f.write_str(self.as_wire())\n    \
             }\n\
         }\n\n",
    );

    // FromStr -- infallible, so `.parse()` reaches the same place `from_wire`
    // does for code that is generic over `FromStr`.
    out.push_str(
        "impl std::str::FromStr for ApiStatus {\n    \
             type Err = std::convert::Infallible;\n\n    \
             fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {\n        \
                 Ok(ApiStatus::from_wire(s))\n    \
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
    resolver: &field_overrides::Resolver,
    enums: &std::collections::HashMap<String, overrides::EnumDef>,
    statuses: &[(String, String)],
    empty_statuses: &BTreeSet<String>,
    zoned_timestamps: &BTreeMap<String, Vec<String>>,
    zone_timestamps: &BTreeMap<String, Vec<String>>,
    base64_file_params: &BTreeMap<String, Vec<String>>,
) -> Result<String, String> {
    let acronyms = acronyms_sorted();
    // Which side each declared enum lands on, collected while the structs are
    // rendered so only the serde direction a field actually uses is emitted.
    let mut enum_sides = EnumSides::default();
    let mut out = String::new();
    out.push_str(
        "// @generated by xtask from tools/server.wsdl + tools/api-responses.json.\n\
         // DO NOT EDIT -- regenerate with `cargo xtask gen`.\n\
         \n\
         // The type names keep VoIP.ms's own acronym casing (`GetDIDsInfoParams`,\n\
         // `SendSMSResponse`) so a name reads the same here as in the API docs. That\n\
         // departs from C-CASE, which clippy reports against a consumer's own build\n\
         // when they run it over this crate's types.\n\
         #![allow(clippy::upper_case_acronyms)]\n\
         \n\
         use serde::Serialize;\n\
         use serde_json::Value;\n\
         \n\
         use crate::client::Client;\n\
         use crate::error::Result;\n\
         \n\
         /// The parameters of a method that takes none.\n\
         #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]\n\
         pub struct NoParams;\n",
    );

    let mut body = String::new();
    for op in &wsdl.operations {
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let input_name = format!("{op}Input");
        let empty = Vec::new();
        let fields = wsdl.types.get(&input_name).unwrap_or(&empty);
        let body_fields: Vec<&(String, String)> = fields
            .iter()
            .filter(|(n, _)| !CLIENT_FIELDS.contains(&n.as_str()))
            .collect();
        // A method with no parameters gets no struct: its generated method takes
        // no argument. Should the WSDL grow one, the method gains an argument --
        // a breaking change the generator makes visible.
        if body_fields.is_empty() {
            continue;
        }

        body.push('\n');
        if let Some(desc) = method_docs.get(op) {
            render_method_doc(&mut body, "", desc);
            body.push_str("///\n");
        }
        body.push_str(&format!(
            "/// Parameters for [`Client::{}`] (wire method `{op}`).\n",
            camel_to_snake(op, &acronyms),
        ));
        body.push_str("#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]\n");
        body.push_str(&format!("pub struct {struct_name} {{\n"));
        let docs = param_docs.get(op);
        for (fname, ftype) in body_fields.iter().copied() {
            // Params are WSDL scalars, so the name-based table always applies.
            let override_ = resolver.resolve(&struct_name, fname, true);
            let rust_ty = match override_ {
                Some(o) => o.rust_type.clone(),
                None => xsd_to_rust(ftype).to_string(),
            };

            enum_sides.note_param(enums, &rust_ty);

            let ident = field_ident(&struct_name, fname, &acronyms);
            let rename = (ident.trim_start_matches("r#") != fname).then_some(fname);
            let mined = docs.and_then(|d| d.get(fname));
            if offset_op(op).is_some() && fname == "timezone" {
                // The mined doc describes the numeric wire form the public
                // `Tz` field is not, and its `(required)` marker is the one
                // `REQUIRED_CTOR_SKIP` overrules: the field defaults to UTC.
                render_doc(&mut body, "    ", OFFSET_TIMEZONE_DOC);
            } else if let Some(desc) = mined {
                render_doc(&mut body, "    ", desc);
            }

            if let Some((_, note)) =
                zone_op(op).and_then(|z| z.param_notes.iter().find(|(f, _)| *f == fname))
            {
                if mined.is_some() {
                    body.push_str("    ///\n");
                }

                render_doc(&mut body, "    ", note);
            }

            // A `param_skip_if` override emits the field unwrapped (plain `T`,
            // skipped at its default); otherwise it's `Option<T>` skipped when
            // `None`. A `param_serializer` supplies the wire form for a type
            // whose own `Serialize` is wrong (a `bool` flag wanting `1`/`0`).
            let param_serializer = override_.and_then(|o| o.param_serializer.as_deref());
            let skip_if = override_.and_then(|o| o.param_skip_if.as_deref());
            body.push_str("    #[serde(skip_serializing_if = \"");
            body.push_str(skip_if.unwrap_or("Option::is_none"));
            body.push('"');
            if let Some(ser) = param_serializer {
                body.push_str(&format!(", serialize_with = \"{ser}\""));
            }
            if let Some(wire) = rename {
                body.push_str(&format!(", rename = \"{wire}\""));
            }
            body.push_str(")]\n");
            match skip_if {
                Some(_) => body.push_str(&format!("    pub {ident}: {rust_ty},\n")),
                None => body.push_str(&format!("    pub {ident}: Option<{rust_ty}>,\n")),
            }
        }
        body.push_str("}\n");
        body.push_str(&emit_params_constructor(
            &struct_name,
            op,
            &body_fields,
            param_docs,
            resolver,
            &acronyms,
        ));

        if let Some(off) = offset_op(op) {
            body.push_str(&emit_offset_wire(
                op,
                off,
                &struct_name,
                &body_fields,
                resolver,
                &acronyms,
            ));
        }
    }

    let mut response_enums = BTreeSet::new();
    let responses_text = response_codegen::emit_response_structs(
        &wsdl.operations,
        responses,
        resolver,
        &mut response_enums,
    )?;
    for ty in &response_enums {
        enum_sides.note_response(enums, ty);
    }

    // Now that both sides have been rendered, the enum declarations can carry
    // only the serde direction some field reaches them through.
    out.push_str(&emit_enums(enums, &enum_sides));
    out.push_str(&emit_statuses(statuses, empty_statuses));
    out.push_str(&emit_timestamp_consts(
        zoned_timestamps,
        "attach_offset",
        &acronyms,
    ));
    out.push_str(&emit_timestamp_consts(
        zone_timestamps,
        "attach_zone",
        &acronyms,
    ));
    out.push_str(&body);
    out.push_str(&responses_text);

    out.push_str(&emit_requires_multipart(base64_file_params));
    out.push_str(&emit_offset_timestamps(zoned_timestamps, &acronyms));
    out.push_str(&emit_zone_timestamps(zone_timestamps, &acronyms));
    out.push_str("\nimpl Client {\n");
    for op in &wsdl.operations {
        let method = camel_to_snake(op, &acronyms);
        let struct_name = format!("{}Params", camel_to_pascal(op, &acronyms));
        let response_name = format!("{}Response", camel_to_pascal(op, &acronyms));
        let takes_params = wsdl.types.get(&format!("{op}Input")).is_some_and(|fields| {
            fields
                .iter()
                .any(|(n, _)| !CLIENT_FIELDS.contains(&n.as_str()))
        });
        if let Some(desc) = method_docs.get(op) {
            render_method_doc(&mut out, "    ", desc);
            out.push_str("    ///\n");
        }
        if !takes_params {
            // No `*Params` struct exists for this method, so the call site
            // would otherwise have to name an empty one.
            out.push_str(&format!(
                "    /// Call the `{op}` API method and deserialize into [`{response_name}`].\n    \
                 ///\n    \
                 /// The method takes no parameters.\n    \
                 pub async fn {method}(&self) -> Result<{response_name}> {{\n        \
                     self.call(\"{op}\", &NoParams).await\n    \
                 }}\n\n\
                 /// Call the `{op}` API method and return the raw JSON envelope.\n    \
                 ///\n    \
                 /// The method takes no parameters.\n    \
                 pub async fn {method}_raw(&self) -> Result<Value> {{\n        \
                     self.call_raw(\"{op}\", &NoParams).await\n    \
                 }}\n\n"
            ));
            continue;
        }

        if offset_op(op).is_some() {
            // Route through the wire twin, resolving `timezone` (a `Tz`) to the
            // window number at the query start date, then undo the shift that
            // number caused on the wall clocks the response reports.
            // Only the const's name is emitted, but the entry is looked up
            // anyway: a method whose const `emit_timestamp_consts` never wrote
            // would otherwise surface as an unresolved name inside a 20k-line
            // generated file, long after the run reported success.
            assert!(
                zoned_timestamps.contains_key(op),
                "{op} is an offset op with no collected response timestamps"
            );
            let paths = timestamps_const_name(op, &acronyms);
            out.push_str(&format!(
                "    /// Call the `{op}` API method and deserialize into [`{response_name}`].\n    \
                 ///\n    \
                 /// `timezone` chooses whose days the date range means, and defaults to\n    \
                 /// UTC: the number sent is\n    \
                 /// [`TimezoneOffset::for_window`](crate::TimezoneOffset::for_window) of\n    \
                 /// it at the query start date, and a zone that cannot be resolved is\n    \
                 /// [`Error::InvalidParams`](crate::Error::InvalidParams). Each reported\n    \
                 /// timestamp is qualified in [`SERVER_ZONE`](crate::SERVER_ZONE) with the\n    \
                 /// offset in force then, whatever was sent; one the zone repeats when\n    \
                 /// clocks fall back stays [`WallClock::Bare`](crate::WallClock::Bare).\n    \
                 pub async fn {method}(&self, params: &{struct_name}) -> Result<{response_name}> {{\n        \
                     let wire = {struct_name}Wire::try_from(params)?;\n        \
                     self.call_zoned(\"{op}\", &wire, wire.timezone, {paths}).await\n    \
                 }}\n\n\
                 /// Call the `{op}` API method and return the raw JSON envelope.\n    \
                 ///\n    \
                 /// The `timezone` sent is chosen as for the typed method. The envelope\n    \
                 /// reports its timestamps shifted by it, without an offset:\n    \
                 /// [`attach_offset`](crate::attach_offset) given the same number\n    \
                 /// qualifies them, over the paths\n    \
                 /// [`offset_timestamps`](crate::offset_timestamps) answers for `{op}`.\n    \
                 pub async fn {method}_raw(&self, params: &{struct_name}) -> Result<Value> {{\n        \
                     self.call_raw(\"{op}\", &{struct_name}Wire::try_from(params)?).await\n    \
                 }}\n\n"
            ));
            continue;
        }

        // `call` / `call_raw` pick the multipart transport for a method in
        // `requires_multipart` themselves; the sentence only documents it.
        let sentence = base64_file_params
            .get(op)
            .map(|fields| multipart_doc_sentence(fields))
            .unwrap_or_default();
        let zop = zone_op(op);
        if zop.is_some() {
            // Same assertion as the offset ops, for the same reason.
            assert!(
                zone_timestamps.contains_key(op),
                "{op} is a zone op with no collected response timestamps"
            );
        }

        let raw_zone = match zop {
            Some(_) => format!(
                "///\n    \
                 /// The envelope reports its timestamps as bare wall clocks:\n    \
                 /// [`attach_zone`](crate::attach_zone) qualifies them, over the paths\n    \
                 /// [`zone_timestamps`](crate::zone_timestamps) answers for `{op}`.\n    "
            ),
            None => String::new(),
        };
        let (typed_zone, typed_body) = match zop.map(|z| &z.source) {
            Some(ZoneSource::Server) => (
                "///\n    \
                 /// Each reported timestamp is qualified in\n    \
                 /// [`SERVER_ZONE`](crate::SERVER_ZONE), the zone VoIP.ms records it in,\n    \
                 /// with the offset in force then. One the zone repeats when clocks fall\n    \
                 /// back stays [`WallClock::Bare`](crate::WallClock::Bare).\n    "
                    .to_string(),
                format!(
                    "self.call_in_zone(\"{op}\", params, crate::SERVER_ZONE, {})",
                    timestamps_const_name(op, &acronyms)
                ),
            ),
            Some(ZoneSource::Supplied(_)) => (
                format!(
                    "///\n    \
                     /// The reported timestamps name no offset, so each reads as\n    \
                     /// [`WallClock::Bare`](crate::WallClock::Bare).\n    \
                     /// [`Client::{method}_in_zone`] qualifies them.\n    "
                ),
                format!("self.call(\"{op}\", params)"),
            ),
            None => (String::new(), format!("self.call(\"{op}\", params)")),
        };
        out.push_str(&format!(
            "    /// Call the `{op}` API method and deserialize into [`{response_name}`].\n    \
             {sentence}{typed_zone}\
             pub async fn {method}(&self, params: &{struct_name}) -> Result<{response_name}> {{\n        \
                 {typed_body}.await\n    \
             }}\n\n\
             /// Call the `{op}` API method and return the raw JSON envelope.\n    \
             {sentence}{raw_zone}\
             pub async fn {method}_raw(&self, params: &{struct_name}) -> Result<Value> {{\n        \
                 self.call_raw(\"{op}\", params).await\n    \
             }}\n\n"
        ));

        if let Some(
            zop @ ZoneOp {
                source: ZoneSource::Supplied(zone),
                ..
            },
        ) = zop
        {
            out.push_str(&emit_in_zone_method(
                zop.wire,
                zone,
                &method,
                &struct_name,
                &response_name,
                &sentence,
                &acronyms,
            ));
        }
    }

    out.push_str("}\n");
    Ok(out)
}

/// Emit the `*_in_zone` method of `op`, a [`ZoneSource::Supplied`] op whose
/// zone `source` describes: the typed call, with each reported timestamp
/// qualified in a zone the caller passes. `sentence` is the multipart note the
/// plain and raw methods carry, empty for a GET.
fn emit_in_zone_method(
    op: &str,
    source: &str,
    method: &str,
    struct_name: &str,
    response_name: &str,
    sentence: &str,
    acronyms: &[&'static str],
) -> String {
    let paths = timestamps_const_name(op, acronyms);
    // Wrapped here rather than in the template, since the fragment's length is
    // the table's. It carries no links for `render_doc` to escape.
    let mut zone = String::new();
    render_doc(
        &mut zone,
        "    ",
        &format!(
            "VoIP.ms renders the timestamps in {source}, and names neither the zone nor \
             an offset."
        ),
    );

    format!(
        "    /// Call the `{op}` API method and deserialize into [`{response_name}`],\n    \
         /// qualifying each reported timestamp with its offset in `zone`.\n    \
         {sentence}\
         ///\n\
         {zone}    \
         /// Pass that zone: each timestamp becomes\n    \
         /// [`WallClock::Zoned`](crate::WallClock::Zoned) with the offset in force\n    \
         /// at that instant, and one the zone repeats or skips at a DST change\n    \
         /// stays [`WallClock::Bare`](crate::WallClock::Bare). This is still one\n    \
         /// request; the zone is not looked up.\n    \
         pub async fn {method}_in_zone(\n        \
             &self,\n        \
             params: &{struct_name},\n        \
             zone: chrono_tz::Tz,\n    \
         ) -> Result<{response_name}> {{\n        \
             self.call_in_zone(\"{op}\", params, zone, {paths}).await\n    \
         }}\n\n"
    )
}

/// The `///` lines naming why a method posts, agreeing in number with however
/// many file parameters it has. The table allows an op more than one; none has
/// two today, which is why this is a function rather than an inline format.
fn multipart_doc_sentence(fields: &[String]) -> String {
    let named = fields
        .iter()
        .map(|f| format!("`{f}`"))
        .collect::<Vec<_>>()
        .join(" / ");
    let (noun, verb, pronoun) = if fields.len() == 1 {
        ("parameter", "does", "it")
    } else {
        ("parameters", "do", "them")
    };

    format!(
        "///\n    \
         /// Sent as a `multipart/form-data` POST: the base64 {named} {noun} {verb}\n    \
         /// not fit the request line a GET would carry {pronoun} on.\n    "
    )
}

/// Emit the public `requires_multipart` predicate over the table's keys.
fn emit_requires_multipart(base64_file_params: &BTreeMap<String, Vec<String>>) -> String {
    if base64_file_params.is_empty() {
        // `matches!(method, )` does not parse, so the empty table needs a body
        // of its own rather than an arm list with nothing in it.
        return "\n/// Whether `method` must be sent as a `multipart/form-data` POST rather than\n\
                /// a GET. No method carries a base64 file parameter, so nothing does.\n\
                pub fn requires_multipart(_method: &str) -> bool {\n    false\n}\n"
            .to_string();
    }

    let arms = base64_file_params
        .keys()
        .map(|op| format!("{op:?}"))
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "\n/// Whether `method` must be sent as a `multipart/form-data` POST rather than\n\
         /// a GET, because one of its parameters carries a base64-encoded file that\n\
         /// would overrun the request line a query string rides on.\n\
         ///\n\
         /// [`Client::call_raw`] and the other generic calls apply this to the\n\
         /// wire-method name they are given. It is public for a caller that needs\n\
         /// the answer without making the call.\n\
         ///\n\
         /// **It answers only for the methods this crate was generated from.** The\n\
         /// names are a fixed table, so a method VoIP.ms has added since answers\n\
         /// `false` rather than reporting that it cannot say -- and `false` is the\n\
         /// wrong answer for an upload method. Regenerate, or call\n\
         /// [`Client::call_multipart_raw`] for it.\n\
         pub fn requires_multipart(method: &str) -> bool {{\n    \
             matches!(method, {arms})\n\
         }}\n"
    )
}

/// Emit the private `*ParamsWire` twin for an [`OffsetOp`]: the same fields as
/// the public struct, with `timezone` as the resolved numeric
/// `crate::TimezoneOffset` -- not optional, since a request with no zone asks
/// for UTC -- plus the `TryFrom<&*Params>` that resolves the public `Tz` at the
/// query start date.
fn emit_offset_wire(
    op: &str,
    off: &OffsetOp,
    struct_name: &str,
    body_fields: &[&(String, String)],
    resolver: &field_overrides::Resolver,
    acronyms: &[&'static str],
) -> String {
    let wire_name = format!("{struct_name}Wire");
    let mut out = String::new();

    out.push_str(&format!(
        "\n/// Wire form of [`{struct_name}`]: `timezone` resolved to the number\n\
         /// `{op}` expects.\n\
         #[derive(Serialize)]\n\
         struct {wire_name} {{\n"
    ));
    for (fname, ftype) in body_fields.iter().copied() {
        let ident = field_ident(struct_name, fname, acronyms);
        let rename = (ident.trim_start_matches("r#") != fname).then_some(fname);
        if fname == "timezone" {
            out.push_str("    timezone: crate::TimezoneOffset,\n");
            continue;
        }

        // Resolve overrides as the public struct so wire fields match it
        // exactly (same types, serializers, renames).
        let override_ = resolver.resolve(struct_name, fname, true);
        let rust_ty = match override_ {
            Some(o) => o.rust_type.clone(),
            None => xsd_to_rust(ftype).to_string(),
        };
        let param_serializer = override_.and_then(|o| o.param_serializer.as_deref());
        match override_.and_then(|o| o.param_skip_if.as_deref()) {
            Some(skip_if) => {
                out.push_str(&format!("    #[serde(skip_serializing_if = \"{skip_if}\""));
                if let Some(ser) = param_serializer {
                    out.push_str(&format!(", serialize_with = \"{ser}\""));
                }
                if let Some(wire) = rename {
                    out.push_str(&format!(", rename = \"{wire}\""));
                }
                out.push_str(")]\n");
                out.push_str(&format!("    {ident}: {rust_ty},\n"));
            }
            None => {
                out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\"");
                if let Some(ser) = param_serializer {
                    out.push_str(&format!(", serialize_with = \"{ser}\""));
                }
                if let Some(wire) = rename {
                    out.push_str(&format!(", rename = \"{wire}\""));
                }
                out.push_str(")]\n");
                out.push_str(&format!("    {ident}: Option<{rust_ty}>,\n"));
            }
        }
    }
    out.push_str("}\n\n");

    let start_ident = field_ident(struct_name, off.start_field, acronyms);
    out.push_str(&format!(
        "impl TryFrom<&{struct_name}> for {wire_name} {{\n    \
             type Error = crate::ParamsError;\n\n    \
             fn try_from(p: &{struct_name}) -> std::result::Result<Self, Self::Error> {{\n"
    ));
    // The number is chosen for the window at the start date, or at the end
    // date when there is no start, in the named zone or in UTC when none is
    // named. A named zone needs one of the two to resolve at; with no zone and
    // neither date there is no window to match, and the reported timestamps
    // are qualified for whatever number is sent. A date string that does not
    // parse is an error whether or not a zone is named, and whether or not the
    // other date is valid, so both are parsed before one is chosen.
    let end_ident = field_ident(struct_name, off.end_field, acronyms);
    if off.start_is_date {
        out.push_str(&format!(
            "        let day = p.{start_ident}.or(p.{end_ident});\n"
        ));
    } else {
        let (start_wire, end_wire) = (off.start_field, off.end_field);
        out.push_str(&format!(
            "        let parse = |param: &'static str, d: Option<&str>| {{\n            \
                 d.map(str::trim)\n                \
                     .filter(|d| !d.is_empty())\n                \
                     .map(|d| {{\n                    \
                         d.parse::<chrono::NaiveDate>()\n                        \
                             .map_err(|_| crate::ParamsError::InvalidDate {{\n                            \
                                 param,\n                            \
                                 value: d.to_string(),\n                        \
                             }})\n                \
                     }})\n                \
                     .transpose()\n        \
             }};\n        \
             let {start_ident} = parse({start_wire:?}, p.{start_ident}.as_deref())?;\n        \
             let {end_ident} = parse({end_wire:?}, p.{end_ident}.as_deref())?;\n        \
             let day = {start_ident}.or({end_ident});\n"
        ));
    }

    out.push_str(
        "        let timezone = match (p.timezone, day) {\n            \
             (tz, Some(day)) => {\n                \
                 crate::TimezoneOffset::for_window(tz.unwrap_or(chrono_tz::UTC), day)?\n            \
             }\n            \
             (Some(_), None) => {\n                \
                 return Err(crate::types::TimezoneOffsetError::MissingQueryDate.into());\n            \
             }\n            \
             (None, None) => crate::TimezoneOffset::UTC,\n        \
         };\n",
    );

    out.push_str("        Ok(Self {\n");
    for (fname, ftype) in body_fields.iter().copied() {
        let ident = field_ident(struct_name, fname, acronyms);
        if fname == "timezone" {
            out.push_str("            timezone,\n");
            continue;
        }

        let rust_ty = match resolver.resolve(struct_name, fname, true) {
            Some(o) => o.rust_type.clone(),
            None => xsd_to_rust(ftype).to_string(),
        };
        if is_copy_ty(&rust_ty) {
            out.push_str(&format!("            {ident}: p.{ident},\n"));
        } else {
            out.push_str(&format!("            {ident}: p.{ident}.clone(),\n"));
        }
    }
    out.push_str("        })\n    }\n}\n");
    out
}

/// Whether a generated param type is `Copy`, so the wire-twin conversion can
/// move it instead of tripping `clippy::clone_on_copy`. The declared-enum
/// types (e.g. `MessageType`) are not listed: their `Unknown(String)`
/// catch-all makes them `Clone`-only.
fn is_copy_ty(t: &str) -> bool {
    matches!(
        t,
        "bool"
            | "u64"
            | "chrono::NaiveDate"
            | "rust_decimal::Decimal"
            | "chrono_tz::Tz"
            | "crate::TimezoneOffset"
            | "crate::Seconds"
            | "crate::WaitTime"
            | "crate::MaxMembers"
    )
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
        .to_path_buf()
}

/// Record a per-struct type assignment, refusing a second one that disagrees.
///
/// Five tables feed this map -- the JSON `field_type_override`,
/// `NAMED_ZONE_TZ_PARAM_PATHS`, `NAMED_ZONE_TZ_RESPONSE_PATHS`,
/// `TRANSACTION_DATE_RESPONSE_PATHS` and the `OFFSET_OPS` zoned timestamps --
/// and `BTreeMap::insert` is last-writer-wins, so a path reached by two of them
/// would take whichever ran last and drop the other on a run that reported
/// success. `getTransactionHistory` is the live example: it takes a date
/// window, so a `timezone` parameter appearing on it would put it in
/// `OFFSET_OPS`, whose loop runs after the range assignment and would replace
/// it with a zoned timestamp.
///
/// The whole override is compared, not just `rust_type`: two tables can agree
/// on the Rust type and disagree on the response type or the deserializer, and
/// resolving that by loop order is the same silent loss. Re-asserting an
/// identical override stays allowed, since only a disagreement is a
/// contradiction.
fn assign_field_type(
    assignments: &mut BTreeMap<String, field_overrides::FieldOverride>,
    path: String,
    ov: field_overrides::FieldOverride,
) -> Result<(), String> {
    if let Some(existing) = assignments.get(&path)
        && *existing != ov
    {
        return Err(format!(
            "`{path}` is assigned two different overrides, `{existing:?}` and `{ov:?}`; \
             the tables in `gen` disagree and one of the entries has to go"
        ));
    }

    assignments.insert(path, ov);
    Ok(())
}

/// The `deserialize_with` path for a response field holding a declared enum.
fn enum_deserializer_path(enum_name: &str) -> String {
    format!("crate::responses::deserialize_opt_from_wire_text::<{enum_name}, _>")
}

/// Which serde directions each declared enum is actually reached through.
///
/// An enum on a param is only ever written, one on a response only ever read,
/// and most are both. Emitting the unused direction would ship a wire contract
/// nothing exercises -- and the reader helper for a param-only enum was dead
/// code the generator had to `#[allow]` to keep the build quiet.
#[derive(Default)]
struct EnumSides {
    serialize: BTreeSet<String>,
    deserialize: BTreeSet<String>,
}

impl EnumSides {
    /// Record `rust_ty` as reached from a `*Params` field, if it names a
    /// declared enum. A non-enum type is ignored, so callers pass any type.
    fn note_param(
        &mut self,
        enums: &std::collections::HashMap<String, overrides::EnumDef>,
        rust_ty: &str,
    ) {
        if enums.contains_key(rust_ty) {
            self.serialize.insert(rust_ty.to_string());
        }
    }

    /// Record `rust_ty` as reached from a `*Response` field.
    fn note_response(
        &mut self,
        enums: &std::collections::HashMap<String, overrides::EnumDef>,
        rust_ty: &str,
    ) {
        if enums.contains_key(rust_ty) {
            self.deserialize.insert(rust_ty.to_string());
        }
    }
}

/// Emit Rust enum declarations for every user-defined enum in the overrides
/// JSON, each carrying only the serde direction `sides` says a field reaches
/// it through.
fn emit_enums(
    enums: &std::collections::HashMap<String, overrides::EnumDef>,
    sides: &EnumSides,
) -> String {
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

        out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
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

        // FromStr -- infallible, since `Unknown` absorbs anything `from_wire`
        // does not recognize. It exists so `.parse()` works for code generic
        // over `FromStr`.
        out.push_str(&format!(
            "impl std::str::FromStr for {name} {{\n    \
                 type Err = std::convert::Infallible;\n\n    \
                 fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {{\n        \
                     Ok({name}::from_wire(s))\n    \
                 }}\n\
             }}\n\n"
        ));

        // Serialize -- only for an enum some `*Params` field writes.
        if sides.serialize.contains(name) {
            out.push_str(&format!(
                "impl serde::Serialize for {name} {{\n    \
                     fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {{\n        \
                         s.serialize_str(self.as_wire())\n    \
                     }}\n\
                 }}\n\n"
            ));
        }

        // Deserialize -- only for an enum some `*Response` field reads. It is
        // tolerant of the string / number / bool wire forms voip.ms mixes.
        if sides.deserialize.contains(name) {
            out.push_str(&format!(
                "impl<'de> serde::Deserialize<'de> for {name} {{\n    \
                     fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {{\n        \
                         let s = crate::responses::deserialize_enum_wire_string(d)?;\n        \
                         Ok({name}::from_wire(&s))\n    \
                     }}\n\
                 }}\n\n"
            ));
        }
    }
    out
}

/// Write emitted Rust to `path`, formatted, or write nothing at all.
///
/// Both gates run before the write, because an emitter defect that rendered
/// broken Rust used to exit zero: the text is parsed, then formatted, and only
/// the result reaches the file. Formatting through a pipe rather than over the
/// written file is what keeps that true of a `rustfmt` failure as well as of a
/// parse failure -- a run that formats in place has already replaced one of the
/// three outputs by the time it can refuse, leaving the tree half-regenerated
/// and the next `cargo fmt --check` failing on a file nobody edited. A missing
/// `rustfmt` is still only a warning, so a machine without one can regenerate.
///
/// Neither gate sees inside a macro invocation -- `matches!(method, )` is valid
/// tokens to both, and is the defect that prompted them. An emitted `matches!`
/// arm list that can be empty therefore needs a branch of its own plus a test,
/// as [`emit_requires_multipart`] has; these gates cover the structural rest.
/// What covers everything is a build, which CI reaches by regenerating and then
/// compiling.
pub(crate) fn write_rust(path: &Path, rendered: &str) -> Result<(), String> {
    syn::parse_file(rendered).map_err(|e| {
        format!(
            "emitted Rust for {} does not parse ({e}); the file was not written",
            path.display()
        )
    })?;

    let formatted = rustfmt(rendered)
        .map_err(|e| format!("{e}; {} was not written", path.display()))?
        .unwrap_or_else(|| rendered.to_string());

    fs::write(path, formatted).map_err(|e| format!("write {}: {e}", path.display()))
}

/// `rendered` as `rustfmt` formats it, so a regen leaves no churn for
/// `cargo fmt --check`. `None` when no `rustfmt` is installed.
///
/// `rustfmt`'s own diagnostic goes to the terminal, since why it refused is
/// most of what the gate is worth.
fn rustfmt(rendered: &str) -> Result<Option<String>, String> {
    rustfmt_with(rendered, Stdio::inherit())
}

/// [`rustfmt`], with its diagnostic sent where `stderr` says rather than
/// inherited.
fn rustfmt_with(rendered: &str, stderr: Stdio) -> Result<Option<String>, String> {
    let mut child = match Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(stderr)
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("warning: rustfmt not found on PATH; run `cargo fmt` manually");
            return Ok(None);
        }
        Err(e) => return Err(format!("rustfmt could not be started ({e})")),
    };

    // The input runs to a megabyte, past what a pipe buffers, so it is fed from
    // a thread: writing it here while nothing drains stdout deadlocks the pair.
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "rustfmt stdin was not piped".to_string())?;
    let input = rendered.to_string();
    let feed = std::thread::spawn(move || stdin.write_all(input.as_bytes()));

    let out = child
        .wait_with_output()
        .map_err(|e| format!("rustfmt could not be run ({e})"))?;
    feed.join()
        .map_err(|_| "the thread feeding rustfmt panicked".to_string())?
        .map_err(|e| format!("rustfmt stopped reading its input ({e})"))?;

    if !out.status.success() {
        return Err(format!(
            "rustfmt rejected the emitted Rust ({})",
            out.status
        ));
    }

    String::from_utf8(out.stdout)
        .map(Some)
        .map_err(|e| format!("rustfmt returned something that is not UTF-8 ({e})"))
}

/// The post-override response shapes, for a tool that needs what `gen` renders
/// without rendering it. Shares `gen`'s inputs so the two can't disagree.
pub(crate) fn load_shapes_for_tools() -> Result<BTreeMap<String, Shape>, String> {
    let root = repo_root();
    let wsdl_path = root.join("tools").join("server.wsdl");
    let text =
        fs::read_to_string(&wsdl_path).map_err(|e| format!("read {}: {e}", wsdl_path.display()))?;
    let wsdl = wsdl::parse_wsdl(&text)?;

    let overrides_doc = overrides::load(&root.join("tools").join("api-response-overrides.json"))?;
    overrides_doc.check_version()?;

    load_response_shapes(
        &root.join("tools").join("api-responses.json"),
        &overrides_doc,
        &wsdl,
    )
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
    // Before anything is written: this run emits the harness's key-path table
    // from these same shapes, and a shape that table cannot describe must stop
    // the run rather than half-finish it.
    dump_fields::validate_roots(&responses)?;
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

    // A patch or addition whose leaf field has a field-name override is dead
    // weight: the override supplies both the Rust type and the deserializer, so
    // the declared scalar type is never consulted. Warn so the entry gets fixed
    // (unless a `field_type_skip` on that field name keeps some struct on the
    // inferred/patched type, in which case the entry may still be live). An
    // addition goes through the same resolver as any other scalar field, so it
    // is shadowed in exactly the same way.
    let skipped_fields: BTreeSet<&str> = field_type_skip
        .iter()
        .filter_map(|entry| entry.rsplit_once('.').map(|(_, f)| f))
        .collect();
    for (method, mo) in &overrides_doc.methods {
        let declared = mo
            .patches
            .iter()
            .map(|patch| ("patch", &patch.path))
            .chain(mo.additions.iter().map(|add| ("addition", &add.path)));
        for (kind, path) in declared {
            let leaf = path.rsplit('.').next().unwrap_or(path);
            // A path ending in `[]` retypes a list *element*, which the
            // field-name table never touches.
            if leaf.ends_with("[]") {
                continue;
            }
            if table.get(leaf).is_some() && !skipped_fields.contains(leaf) {
                eprintln!(
                    "warning: {method}: {kind} `{path}` is shadowed by the \
                     field-name override for `{leaf}`; its declared type is ignored",
                );
            }
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
        assign_field_type(
            &mut field_type_override,
            path.clone(),
            field_overrides::FieldOverride {
                rust_type: enum_name.clone(),
                response_deserializer: Some(enum_deserializer_path(enum_name)),
                ..Default::default()
            },
        )?;
    }

    // Timezone assignments, hand-written here rather than in the JSON (whose
    // `field_type_override` values are validated as declared enums): named-zone
    // params are strict `chrono_tz::Tz`; named-zone response fields are the
    // tolerant `crate::TimezoneName` (voip.ms still reports legacy names the
    // IANA database dropped); and the offset ops' public `timezone` is `Tz`
    // (its numeric wire form is produced by the `*ParamsWire` twin -- the
    // public field serializes as the readable IANA name).
    let acronyms = acronyms_sorted();
    for path in field_overrides::NAMED_ZONE_TZ_PARAM_PATHS {
        assign_field_type(
            &mut field_type_override,
            (*path).to_string(),
            field_overrides::tz_param_override(),
        )?;
    }

    for path in field_overrides::NAMED_ZONE_TZ_RESPONSE_PATHS {
        assign_field_type(
            &mut field_type_override,
            (*path).to_string(),
            field_overrides::tz_response_override(),
        )?;
    }

    // A transaction-history row's `date` is a point in time or the window the
    // row summarizes, so it is typed per struct here for the same reason -- the
    // JSON section takes declared enums only.
    for path in field_overrides::TRANSACTION_DATE_RESPONSE_PATHS {
        assign_field_type(
            &mut field_type_override,
            (*path).to_string(),
            field_overrides::transaction_date_override(),
        )?;
    }

    // Each offset op's response reports its timestamps in the offset the
    // request carried, so they are typed with one instead of as a bare wall
    // clock, and the generated method is handed the paths that reach them.
    let mut zoned_timestamps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for op in OFFSET_OPS {
        field_type_override.insert(
            format!("{}Params.timezone", camel_to_pascal(op.wire, &acronyms)),
            field_overrides::tz_param_override(),
        );

        let shape = responses.get(op.wire).ok_or_else(|| {
            format!(
                "{} sends a timezone offset but has no response shape to qualify",
                op.wire
            )
        })?;
        let fields = response_codegen::timestamp_fields(op.wire, shape)?;
        if fields.is_empty() {
            return Err(format!(
                "{}'s response declares no timestamp field, so the offset it sends \
                 would qualify nothing; correct OFFSET_OPS or the response shape",
                op.wire
            ));
        }

        for f in &fields {
            assign_field_type(
                &mut field_type_override,
                f.struct_path.clone(),
                field_overrides::record_listing_timestamp_override(),
            )?;
        }

        zoned_timestamps.insert(
            op.wire.to_string(),
            fields.into_iter().map(|f| f.json_path).collect(),
        );
    }

    // Each zone op's response reports its timestamps in a named zone the caller
    // supplies, so they are typed to hold a bare or a qualified wall clock, and
    // the `*_in_zone` method is handed the paths that reach them.
    let mut zone_timestamps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for op in ZONE_OPS {
        // An offset op's timestamps already carry the offset the request sent;
        // qualifying them a second time in a zone would contradict it.
        if offset_op(op.wire).is_some() {
            return Err(format!(
                "{} is in both OFFSET_OPS and ZONE_OPS; its timestamps can be \
                 qualified one way, not both",
                op.wire
            ));
        }

        // A note on a param the WSDL does not declare would never render, and
        // the measured fact it records would vanish on a run that succeeded.
        let declared = wsdl.types.get(&format!("{}Input", op.wire));
        // A method with no params of its own is emitted through a separate
        // template that calls `call` with no `*Params` struct, so it would get
        // neither the server-zone qualification nor an `*_in_zone` sibling.
        if !declared.is_some_and(|fields| {
            fields
                .iter()
                .any(|(n, _)| !CLIENT_FIELDS.contains(&n.as_str()))
        }) {
            return Err(format!(
                "{} is a zone op but takes no parameters, so its typed method \
                 could not qualify its timestamps",
                op.wire
            ));
        }

        for (param, _) in op.param_notes {
            if !declared.is_some_and(|fields| fields.iter().any(|(n, _)| n == param)) {
                return Err(format!(
                    "ZONE_OPS notes `{}.{param}`, which the WSDL does not declare",
                    op.wire
                ));
            }
        }

        let shape = responses.get(op.wire).ok_or_else(|| {
            format!(
                "{} is a zone op but has no response shape to qualify",
                op.wire
            )
        })?;
        let fields = response_codegen::timestamp_fields(op.wire, shape)?;
        if fields.is_empty() {
            return Err(format!(
                "{}'s response declares no timestamp field, so a zone would qualify \
                 nothing; correct ZONE_OPS or the response shape",
                op.wire
            ));
        }

        for f in &fields {
            assign_field_type(
                &mut field_type_override,
                f.struct_path.clone(),
                field_overrides::wall_clock_override(),
            )?;
        }

        zone_timestamps.insert(
            op.wire.to_string(),
            fields.into_iter().map(|f| f.json_path).collect(),
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

    let base64_file_params =
        base64_file_params(&wsdl, &param_docs, field_overrides::BASE64_FILE_PARAM_PATHS)?;
    for path in &base64_file_params.unlisted {
        eprintln!(
            "warning: {path} is documented as base64 but is absent from \
             BASE64_FILE_PARAM_PATHS; a file payload does not fit the request \
             line a GET puts it on"
        );
    }

    let resolver = field_overrides::Resolver {
        table: &table,
        per_struct: &field_type_override,
        skip: &field_type_skip,
    };
    let rendered = emit(
        &wsdl,
        &responses,
        &param_docs,
        &method_docs,
        &resolver,
        &overrides_doc.enums,
        &statuses,
        &empty_statuses,
        &zoned_timestamps,
        &zone_timestamps,
        &base64_file_params.by_op,
    )?;
    write_rust(&out_path, &rendered)?;
    println!(
        "wrote {} ({} methods, {} method descriptions, {} typed responses, \
         {} status codes)",
        out_path.display(),
        wsdl.operations.len(),
        method_docs.len(),
        responses.len(),
        statuses.len(),
    );

    // From the same shapes, so the harness's key-diff table can't fall behind
    // the structs it describes.
    dump_fields::write_table(&responses)?;

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
    // `Success` is synthesized (see SUCCESS_STATUS), so a docs refresh that
    // started listing it would emit the variant twice.
    seen.insert("Success".to_string(), SUCCESS_STATUS.0.to_string());
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
        "check-flags" => check_flags::cmd_check_flags(&rest),
        "check-types" => check_types::cmd_check_types(&rest),
        "dump-methods" => dump_methods::cmd_dump_methods(),
        "dump-fields" => dump_fields::cmd_dump_fields(),
        other => Err(format!(
            "unknown subcommand `{other}` \
             (expected `gen`, `extract-responses`, `extract-statuses`, \
             `check-flags`, `check-types`, `dump-methods`, or `dump-fields`)"
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A WSDL with one ordinary op carrying a `file` param and one offset op.
    fn wsdl_fixture() -> Wsdl {
        let mut types = std::collections::HashMap::new();
        types.insert(
            "setRecordingInput".to_string(),
            vec![
                ("file".to_string(), "xsd:string".to_string()),
                ("name".to_string(), "xsd:string".to_string()),
            ],
        );
        types.insert(
            "getCDRInput".to_string(),
            vec![("date_from".to_string(), "xsd:string".to_string())],
        );
        Wsdl {
            operations: vec!["setRecording".to_string(), "getCDR".to_string()],
            types,
        }
    }

    #[test]
    fn base64_file_params_groups_listed_paths_by_op() {
        let found =
            base64_file_params(&wsdl_fixture(), &ParamDocs::new(), &["setRecording.file"]).unwrap();
        assert!(
            found.unlisted.is_empty(),
            "nothing is documented as base64 here"
        );
        let grouped = found.by_op;
        assert_eq!(
            grouped.get("setRecording").map(Vec::as_slice),
            Some(&["file".to_string()][..])
        );
    }

    #[test]
    fn base64_file_params_rejects_a_path_the_wsdl_does_not_declare() {
        // A path left behind by a docs revision would otherwise drop the method
        // back onto a GET without a word.
        let err = base64_file_params(&wsdl_fixture(), &ParamDocs::new(), &["setRecording.gone"])
            .unwrap_err();
        assert!(err.contains("names no input field"), "{err}");

        let err = base64_file_params(&wsdl_fixture(), &ParamDocs::new(), &["nodot"]).unwrap_err();
        assert!(err.contains("must be `wireMethod.field`"), "{err}");
    }

    #[test]
    fn base64_file_params_rejects_an_offset_op() {
        // An offset op's wire twin has only been sent over GET, so listing one
        // here would move it onto multipart with nothing having checked that.
        let err = base64_file_params(&wsdl_fixture(), &ParamDocs::new(), &["getCDR.date_from"])
            .unwrap_err();
        assert!(err.contains("names an offset op"), "{err}");
    }

    #[test]
    fn base64_file_params_reports_a_documented_file_param_absent_from_the_table() {
        // The tripwire for a fifth method arriving in a docs refresh: reported
        // as a value, so `cmd_gen`'s warning has something to be a warning
        // about and this branch is not taken on faith.
        let mut docs = ParamDocs::new();
        docs.entry("setRecording".to_string())
            .or_default()
            .insert("file".to_string(), "Base64 encoded file".to_string());
        docs.entry("sendFaxMessage".to_string())
            .or_default()
            .insert("file".to_string(), "must be encoded in Base64".to_string());

        // `setRecording.file` is listed, `sendFaxMessage.file` is not.
        let found = base64_file_params(&wsdl_fixture(), &docs, &["setRecording.file"]).unwrap();
        assert_eq!(found.unlisted, vec!["sendFaxMessage.file".to_string()]);
    }

    /// The emitted predicate, parsed as an item so a malformed one is a failure
    /// here rather than a compile error in a file the run already wrote.
    fn parsed_predicate(table: &BTreeMap<String, Vec<String>>) -> (String, syn::ItemFn) {
        let rendered = emit_requires_multipart(table);
        let item: syn::ItemFn = syn::parse_str(&rendered)
            .unwrap_or_else(|e| panic!("emitted predicate does not parse ({e}): {rendered}"));
        (rendered, item)
    }

    fn table_of(ops: &[&str]) -> BTreeMap<String, Vec<String>> {
        ops.iter()
            .map(|op| ((*op).to_string(), vec!["file".to_string()]))
            .collect()
    }

    #[test]
    fn requires_multipart_matches_every_op_in_the_table() {
        let (rendered, item) = parsed_predicate(&table_of(&["sendMMS", "setRecording"]));

        assert_eq!(item.sig.ident, "requires_multipart");
        // Both arms, in the table's own order, and joined so one name cannot
        // stand in for the other.
        assert!(
            rendered.contains(r#"matches!(method, "sendMMS" | "setRecording")"#),
            "{rendered}"
        );
    }

    #[test]
    fn requires_multipart_over_an_empty_table_still_parses() {
        // `matches!(method, )` is not valid Rust, so the empty table takes a
        // branch of its own. Nothing exercises that branch today -- four ops are
        // listed -- which is exactly why it is asserted rather than assumed.
        let (rendered, item) = parsed_predicate(&BTreeMap::new());

        assert_eq!(item.sig.ident, "requires_multipart");
        assert!(!rendered.contains("matches!"), "{rendered}");
        // The parameter is `_method`: an unused binding would warn in a
        // consumer's build, and the generated module is not theirs to silence.
        assert!(
            rendered.contains("fn requires_multipart(_method: &str)"),
            "{rendered}"
        );
    }

    fn parsed_offset_lookup(table: &BTreeMap<String, Vec<String>>) -> (String, syn::ItemFn) {
        let rendered = emit_offset_timestamps(table, &acronyms_sorted());
        let item: syn::ItemFn = syn::parse_str(&rendered)
            .unwrap_or_else(|e| panic!("emitted lookup does not parse ({e}): {rendered}"));
        (rendered, item)
    }

    #[test]
    fn offset_timestamps_answers_each_op_with_its_own_const() {
        let table: BTreeMap<String, Vec<String>> = [
            ("getCDR".to_string(), vec!["/cdr/*/date".to_string()]),
            ("getSMS".to_string(), vec!["/sms/*/date".to_string()]),
        ]
        .into_iter()
        .collect();
        let (rendered, item) = parsed_offset_lookup(&table);

        assert_eq!(item.sig.ident, "offset_timestamps");
        // The arm names the const rather than repeating its paths, so the two
        // cannot disagree.
        assert!(
            rendered.contains(r#""getCDR" => Some(GET_CDR_TIMESTAMPS),"#),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#""getSMS" => Some(GET_SMS_TIMESTAMPS),"#),
            "{rendered}"
        );
        assert!(rendered.contains("_ => None,"), "{rendered}");
        assert!(!rendered.contains("/cdr/*/date"), "{rendered}");
    }

    #[test]
    fn zone_timestamps_answers_each_op_with_its_source_and_const() {
        let table: BTreeMap<String, Vec<String>> = [
            (
                "getVoicemailMessages".to_string(),
                vec!["/messages/*/date".to_string()],
            ),
            (
                "getDIDsInfo".to_string(),
                vec!["/dids/*/order_date".to_string()],
            ),
        ]
        .into_iter()
        .collect();
        let rendered = emit_zone_timestamps(&table, &acronyms_sorted());
        let item: syn::ItemFn = syn::parse_str(&rendered)
            .unwrap_or_else(|e| panic!("emitted lookup does not parse ({e}): {rendered}"));

        assert_eq!(item.sig.ident, "zone_timestamps");
        // Each arm names the op's source from `ZONE_OPS` and its const, so the
        // paths are spelled once.
        assert!(
            rendered.contains(
                "\"getVoicemailMessages\" => Some(crate::ZoneTimestamps { zone: \
                 crate::TimestampZone::Supplied, paths: GET_VOICEMAIL_MESSAGES_TIMESTAMPS }),"
            ),
            "{rendered}"
        );
        assert!(
            rendered.contains(
                "\"getDIDsInfo\" => Some(crate::ZoneTimestamps { zone: \
                 crate::TimestampZone::Server, paths: GET_DIDS_INFO_TIMESTAMPS }),"
            ),
            "{rendered}"
        );
        assert!(!rendered.contains("/messages/*/date"), "{rendered}");
    }

    #[test]
    fn a_timestamp_const_names_the_helper_that_takes_it() {
        let table: BTreeMap<String, Vec<String>> = [(
            "getVoicemailMessages".to_string(),
            vec!["/messages/*/date".to_string()],
        )]
        .into_iter()
        .collect();
        let rendered = emit_timestamp_consts(&table, "attach_zone", &acronyms_sorted());

        assert!(
            rendered.contains("[`attach_zone`](crate::attach_zone) takes."),
            "{rendered}"
        );
        assert!(
            rendered.contains(
                r#"const GET_VOICEMAIL_MESSAGES_TIMESTAMPS: &[&str] = &["/messages/*/date"];"#
            ),
            "{rendered}"
        );
    }

    /// A method whose timestamps are qualified in a zone must not also be one
    /// whose request carries an offset; `cmd_gen` refuses the overlap, and this
    /// keeps the committed tables from reaching it.
    #[test]
    fn no_op_is_both_an_offset_op_and_a_zone_op() {
        for op in ZONE_OPS {
            assert!(
                offset_op(op.wire).is_none(),
                "{} is in both tables",
                op.wire
            );
        }
    }

    #[test]
    fn offset_timestamps_over_an_empty_table_still_parses() {
        let (rendered, item) = parsed_offset_lookup(&BTreeMap::new());

        assert_eq!(item.sig.ident, "offset_timestamps");
        assert!(!rendered.contains("match"), "{rendered}");
        assert!(
            rendered.contains("fn offset_timestamps(_method: &str)"),
            "{rendered}"
        );
    }

    #[test]
    fn write_rust_refuses_output_that_does_not_parse() {
        // The class the gate closes: an emitter defect that renders broken Rust
        // must stop the run, not leave the file on disk with a warning.
        // Named per process so two concurrent runs cannot share the file.
        let dir = std::env::temp_dir().join(format!("voip-ms-xtask-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a temp dir for the refusal case");
        let path = dir.join("broken.rs");
        let _ = fs::remove_file(&path);

        let err = write_rust(&path, "pub fn broken() -> bool { false").unwrap_err();
        assert!(err.contains("does not parse"), "{err}");
        assert!(!path.exists(), "the file must not have been written");

        // The rustfmt gate refuses too. Driven directly because anything
        // rustfmt rejects, `syn` rejects first, so no literal reaches this arm
        // through `write_rust`. Its own diagnostic is silenced here: inherited,
        // it prints a bare `error:` line into an otherwise green test run.
        let err = rustfmt_with("fn f() { let _ = |&:| (); }\n", Stdio::null()).unwrap_err();
        assert!(err.contains("rustfmt rejected"), "{err}");

        write_rust(&path, "pub fn fine() -> bool {\n    false\n}\n")
            .expect("valid Rust is written");
        assert!(path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_macro_body_is_opaque_to_the_parse_gate() {
        // Both parsers take a macro invocation as tokens, so the defect that
        // prompted the gate -- an arm list rendered empty -- reads as valid Rust
        // to each of them. That is why the empty-table branch above exists and
        // is asserted, rather than left to the gate.
        assert!(syn::parse_file("fn f(m: &str) -> bool { matches!(m, ) }").is_ok());
    }

    #[test]
    fn multipart_doc_sentence_agrees_with_the_number_of_fields() {
        // The plural branch has no entry to exercise it today: every op in the
        // table has exactly one file parameter.
        let one = [String::from("file")];
        let two = [String::from("media1"), String::from("media2")];
        assert!(multipart_doc_sentence(&one).contains("`file` parameter does"));
        assert!(multipart_doc_sentence(&one).contains("carry it on"));
        assert!(multipart_doc_sentence(&two).contains("`media1` / `media2` parameters do"));
        assert!(multipart_doc_sentence(&two).contains("carry them on"));
    }

    #[test]
    fn documents_base64_reads_the_spellings_the_docs_use() {
        assert!(documents_base64("Base64 encoded file (required)"));
        assert!(documents_base64("Base 64 code of the file to be attached"));
        assert!(documents_base64(
            "The file must be encoded in Base64, and in one of the following formats"
        ));
        assert!(documents_base64(
            "Base 64 image encode (Example: data:image/png;base64,iVBOR...)"
        ));
        assert!(!documents_base64(
            "Url to media file (Example: 'https://voip.ms/x.jpg')"
        ));
    }

    #[test]
    fn two_tables_assigning_one_field_different_types_fails_the_run() {
        let mut assignments = BTreeMap::new();
        let path = "GetTransactionHistoryResponseTransaction.date".to_string();
        assign_field_type(
            &mut assignments,
            path.clone(),
            field_overrides::transaction_date_override(),
        )
        .unwrap();

        let err = assign_field_type(
            &mut assignments,
            path.clone(),
            field_overrides::wall_clock_override(),
        )
        .unwrap_err();
        assert!(err.contains("assigned two different overrides"), "{err}");

        // Re-asserting the same type is not a disagreement, so a path listed
        // twice in one table is harmless rather than a build failure.
        assign_field_type(
            &mut assignments,
            path,
            field_overrides::transaction_date_override(),
        )
        .unwrap();
    }
}
