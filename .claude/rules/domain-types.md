---
paths:
  - "src/types.rs"
  - "src/responses.rs"
  - "xtask/src/field_overrides.rs"
  - "xtask/src/overrides.rs"
  - "xtask/src/check_types.rs"
  - "xtask/src/check_flags.rs"
  - "tools/api-response-overrides.json"
  - "tests/generated_enums.rs"
---

# Domain types and unreadable values

### 2 (continued). Response values the crate cannot read

All response fields are `Option<T>` with `#[serde(default)]` so that
VoIP.ms adding, removing, or omitting a field never breaks
deserialization. Numbers, booleans (`0/1`, `Y/N`), dates, and decimals
arrive as JSON strings from the API; the deserializers in
`src/responses.rs` (`deserialize_opt_*`) normalize both string and
native-typed forms and treat `"0000-00-00"` placeholders as `None`.

**A value the crate cannot read costs its own field, not the response.** A
`*Response` is one value built from one envelope, so a deserializer that errors
on a single field discards every record beside it -- the caller gets an error
instead of the rows that were fine. Every response date is therefore
[`crate::Reported<T>`] (`Parsed(T)` or `Unreadable(String)`), and every
substituted enum carries `Unknown(String)`. The wrapper keeps the text rather
than answering `None`, which would discard it: the caller can still salvage what
arrived, and "unreadable" stays distinguishable from "absent", which is what the
live harness reads. Params are the other way round -- they are written, never
received, so they keep the bare `chrono` type, which is why `FieldOverride`
carries a `response_rust_type` beside `rust_type`.

The rule is that an unexpected **value** degrades and an unexpected **contract**
does not. The timestamp readers are where the two meet. In a named-zone field a
value with no offset is part of the contract, and reads as
[`crate::WallClock::Bare`], which claims no zone, rather than as UTC, which
would invent one. In a record-listing field it is a broken contract, since
there it is a wall clock shifted by the `timezone` sent that `attach_offset`
never qualified, so `deserialize_opt_record_listing_timestamp` refuses it
(decision #8). A JSON list or object where a scalar belongs is a shape, and no
scalar type can stand in for one, so that is rejected too.

**The class is not closed.** Seven readers still fail the whole envelope on a
well-shaped string they cannot read. Four are scalar helpers in
`src/responses.rs`: `deserialize_opt_u64_from_string_or_number` (`"1,234"`),
`deserialize_opt_decimal_from_string_or_number` (`"$1.00"`),
`deserialize_opt_bool_from_string_number_or_yn` (`"maybe"`) and
`deserialize_opt_routing`. The other three are the sentinel types in
`src/types.rs` -- [`crate::Seconds`], [`crate::WaitTime`] and
[`crate::MaxMembers`] -- which accept a number, a numeric string or their own
sentinel word and reject everything else, so a `getQueues` row reporting
`announce_frequency` as `"every 30s"` costs the response. Those reach
`maximum_wait_time`, `maximum_callers`, `max_members` and the `SECONDS_FIELDS`
set. That is the whole remaining exposure, and it is not hypothetical: `GetTransactionHistoryResponseTransaction::amount` is a strict
`Decimal` on the same row whose `uniqueid` VoIP.ms already reports as the
literal `n/a`. Extending `Reported<T>` to them is the same change made here for
dates, and wants the same thing first -- an observation, or a decision recorded
as one.

The one exception is the envelope's own `status`, which is a required
[`ApiStatus`] on each top-level `*Response`. `Client::fetch` has already
required the field to exist and classified it before a typed call returns, so
handing back a raw string would make the caller parse it a second time to
learn whether it was `success` or an empty-collection code. `ApiStatus` carries
`Success` and `SuccessTest` variants for that reason -- they are synthesized by
the generator (`SUCCESS_STATUSES`), since the docs' error-code table lists only
errors. A nested record's same-named field (a fax's, a port's, an e911
record's) is unrelated and keeps its inferred type; `response_codegen.rs`
distinguishes them by whether the struct is the method's root.

### 5a. Domain types substituted by field name

**Decision**: A small set of stringly-typed fields are upgraded to
domain types during codegen, driven by the field's snake_case name (not
its method). Two override mechanisms feed the same substitution table
in `xtask/src/field_overrides.rs`:

* **Built-in substitutions** (hand-written in `field_overrides.rs`):
  the 12 routing-related fields (`routing`, `failover_busy`,
  `failover_noanswer`, `failover_unreachable`, plus the
  `fail_over_routing_*` variants used by queues) map to
  [`crate::Routing`], a tagged enum hand-written in `src/types.rs`
  that parses VoIP.ms's `kind:value` strings (`account:100001_VoIP`,
  `fwd:1026`, `sip:87149`, `recording:289572`, `none:`, …). Routing
  changes shape rarely and benefits from a custom `FromStr` (only the
  first `:` is the separator, so a value containing `:` survives). Every
  header but `sys` and `none` names a record by its id, `sip` included:
  confirmed live, `setDIDRouting` stores `sip:` followed by a `getSIPURIs`
  id and answers `invalid_sipuri` to `sip:user@example.com`.
* **Seconds-or-sentinel durations** map to [`crate::Seconds`] (six queue /
  announcement fields in `SECONDS_FIELDS`) or [`crate::WaitTime`]
  (`maximum_wait_time`), hand-written enums that hold a `u64` count *or* an
  unbounded sentinel -- VoIP.ms documents these as a number of seconds or a
  word (`none` / `unlimited`), which a bare `u64` can't represent. They carry
  their own (de)serialization (tolerant of a number, a numeric string, or
  either sentinel word).
* **Timezones are `chrono_tz::Tz` everywhere**, translated per wire contract.
  The field name `timezone` means two incompatible things upstream, so the
  assignments are per-struct (injected into the `field_type_override` map in
  `main.rs`, since the JSON section is validated enum-only), not name-based:
  - *Named-zone* params (`createVoicemail` / `setVoicemail` / `getTimezones` --
    `field_overrides.rs::NAMED_ZONE_TZ_PARAM_PATHS`) are strict `Tz`, carried
    on the wire via `serialize_opt_tz`. *Named-zone responses*
    (`getVoicemails`, `getTimezones` -- `NAMED_ZONE_TZ_RESPONSE_PATHS`) are the
    tolerant [`crate::TimezoneName`] (`Known(Tz)` or `Unrecognized(String)`)
    via `deserialize_opt_from_wire_text`: voip.ms's live `getTimezones` catalog
    lists legacy names the IANA database has dropped (`Asia/Beijing`,
    `US/Pacific-New`, `Factory`, old Saudi `Riyadh87`/`88`/`89`,
    `Canada/East-Saskatchewan`), and a strict `Tz` failed the whole response on
    the first one -- confirmed by `cargo run -p livetest`, not by the wiremock
    suite (whose fixtures never included a legacy name).
  - *Record-listing* `timezone` params (`getCDR` / `getResellerCDR` / `getSMS`
    / `getMMS` / `getResellerSMS` / `getResellerMMS` -- the `OFFSET_OPS` table
    in `main.rs`) want a number of hours (`-12..=13`), which the WSDL
    under-types inconsistently (`xsd:decimal` on the CDR pair, `xsd:string` on
    the SMS/MMS four). The public field is still `Option<Tz>`; the generator
    emits a private `*ParamsWire` twin plus a `TryFrom<&*Params>` that picks the
    number with `TimezoneOffset::for_query`, and routes both generated method
    bodies through it. `for_query` takes `for_window` of the zone (UTC when
    none is named) at the query start date (`date_from` / `from`), or at the end
    date (`date_to` / `to`) when there is no start. A named zone with neither
    date (`TimezoneOffsetError::MissingQueryDate`), a date string that does not
    parse (`ParamsError::InvalidDate`, named zone or not, naming the param,
    whether or not the other date is valid), or a number outside the range is
    `Error::InvalidParams` before any request is sent. The public
    struct still derives `Serialize` -- there `timezone` emits the IANA name
    (what a log should show); only the wire twin carries the number, so a raw
    `call_raw` caller picks the number itself with `for_query`. The rules live
    in `for_query` rather than in the generated body so that a raw caller and
    the typed method cannot choose differently.

    **The number is not a UTC offset during DST.** VoIP.ms records timestamps
    as [`crate::SERVER_ZONE`] wall clocks (US/Canada Eastern, observing DST)
    and reports each shifted by `timezone + 5` hours, as if that zone were
    always UTC-5; the `date_from` / `date_to` days are matched on the shifted
    value. Measured on 2026-09-23 against a call placed at 16:28:34 UTC, whose
    recording id embeds that Unix time: `timezone` `-12`, `-5`, `-4`, `0`,
    `5.5` and `13` reported `05:28:34`, `12:28:34`, `13:28:34`, `17:28:34`,
    `22:58:34` and `06:28:34` (the next day), and at `13` the call was listed
    under 2026-09-24. So during DST `n` reports UTC+`n+1`. `for_window` sends
    the caller's offset minus the server zone's, less five, so the window is the
    caller's days at the start date. A caller who names no zone gets UTC days.

    That outside DST the same fixed base gives UTC+`n` is **inferred, not
    measured**: every record on the test account was made during DST, and every
    read was made during it. If the shift were different for a winter record or
    a winter read, every winter row would be qualified wrong. The unit tests
    that assert winter values assert this model. Settling it takes reads made
    after 2026-11-01 of a record with a known instant, one made before the
    change (such as the call above) and one made after it.

    A number outside `-12..=13` is refused rather than clamped, so a zone's
    window is either right or an error. During Eastern DST a UTC-12 zone needs
    `-13`, which 0.13 sent as `-12` (and matched an hour off); outside DST
    `Pacific/Kiritimati` (+14) needs `14`.

    Omitting the parameter is not a fallback to some account zone; there is no
    such setting. The portal's account and contact pages have no zone field.
    `getCDR` and `getResellerCDR` answer `invalid_timezone` when it is omitted,
    and `getSMS` / `getMMS` treat an omitted one as `-5`. The reseller SMS/MMS
    pair was not measured: the account has no reseller client to query. The
    crate always sends one because the window is then the caller's.
* **Boolean flags** map to `bool`, registered in the `FLAG_01_FIELDS` /
  `FLAG_YES_NO_FIELDS` consts of `xtask/src/field_overrides.rs`. Many
  parameters VoIP.ms documents as `1 = true, 0 = false` (or `yes`/`no`) are
  under-typed by the WSDL as `xsd:integer` / `xsd:string`, so the extractor
  would emit `i64` / `String` and leak the wire encoding. The wire form lives
  in a serializer, not the type: the override carries a `param_serializer`
  (`serialize_opt_flag_01` / `serialize_opt_flag_yes_no` in `src/responses.rs`)
  emitted as `serialize_with` on the param, since a bare `bool` serializes to
  `true`/`false`, which these parameters reject. Responses use the existing
  tolerant `deserialize_opt_bool_from_string_number_or_yn`, accepting
  `1`/`0`/`yes`/`no`/`true`/`false` as string, number, or bool. A validate-only
  flag whose `false` means the same as absent (the `test` param) sets
  `param_skip_if` so it's emitted as plain `bool` (default `false`, omitted from
  the request when `false`) rather than `Option<bool>`.
* **Phone-number identifier fields** stay `String` on both the param and
  response side (`PHONE_STRING_FIELDS` in `xtask/src/field_overrides.rs`:
  `did`, `number`, `phone_number`, `contact`, `destination`, `stationid`).
  A phone number is an identifier, never a quantity -- it can carry leading
  zeros, exceed `i64` range, or hold a SIP form (`sip:2563` in
  `setPhonebook`) -- but both the WSDL (`xsd:integer` on the fax `did` and
  `setCallback`/`setPhonebook` `number` params) and the extractor (an
  all-digit doc sample infers `integer`) under-type them, so the override
  forces `String` globally instead of patching method-by-method. The
  response side keeps the tolerant string deserializer since VoIP.ms may
  ship the value as a bare JSON number. Deliberately excluded: the plural
  `dids` (sometimes a list of numeric vPRI ids) and `from` (a date filter
  in the `getSMS`-family params, an email in `getEmailToFax`'s response).
  A `patches` entry under one of these fields leaves the Rust type alone but
  still corrects the type and sample in the JSON shapes, so it stays.
* **Calendar dates** (`date_from`, `date_to`, `reseller_nextbilling` in
  `DATE_FIELDS`) map to [`chrono::NaiveDate`], whose own `Serialize` emits the
  documented `YYYY-MM-DD` wire form. The bare `date` field is excluded -- it is
  a datetime in some responses and a date in others, so no single type fits.
  `getTransactionHistory` is the case where one field holds a point in time and
  a range: [`crate::TransactionDate`] (`TRANSACTION_DATE_RESPONSE_PATHS`,
  assigned per struct for the same reason the timezones are) carries a timestamp
  (`At`), a bare date (`On`), or a range (`Period`, wire `2026-08-01 to
  2026-08-31`), with an `Unrecognized(String)` catch-all. The doc sample shows
  only a timestamp, so the extractor inferred `datetime`, and a live range
  failed the whole envelope -- the same shape of break as the legacy zone names,
  found the same way.

  **The range is the requested window, not a billing period.** The report ends
  with a synthesized row per usage-metered charge -- CNAM queries, communication
  charges -- totaling that charge over the range the caller asked for, and the
  row carries the range itself in place of a timestamp. The customer portal
  shows it directly: a search from 2026-04-01 to 2026-09-22 ends in a
  `CNAM Queries` row dated `2026-04-01 to 2026-09-22`, the search range
  verbatim. The row is synthesized rather than recorded, so it has no
  transaction to name and reports `uniqueid` as the literal `n/a`, which is the
  cheapest way for a consumer to tell it from a real one. Production logs agree:
  four distinct range values inside one four-minute session as the caller varied
  the window, two of them (`2026-08-07 to 2026-08-07`,
  `2026-08-01 to 2026-08-07`) aligning to no billing period at all.

  That is what keeps the type off `getCharges` and `getDeposits`, which are the
  same ledger kept for a reseller client. Neither takes a date range -- `client`
  is their only parameter -- so neither has a window to aggregate over and
  neither can produce the row; both stay `NaiveDate`.

  The reasoning generalizes: a range in a `date` field is a property of a
  *report that totals something over a window*, so the methods to suspect are
  the ones taking `date_from` / `date_to`. Walking that set leaves nothing else
  to fix. The `OFFSET_OPS` six are zoned (decision #8);
  `getCallTranscriptions` and `getVoicemailTranscriptions` report `date` as
  `String` and cannot fail on any value; `getCallRecordings` has no `date`.
  `getConferenceRecordings` and `getVoicemailMessages` are the two that share
  the shape and type `date` as a point in time (`NaiveDateTime`, and
  `WallClock` per decision #8a): each lists individual records and totals
  nothing, so there is no per-charge sum for a window row to carry -- a
  recording and a voicemail each happened at an instant. Revisit that only if
  one of them grows a summary row.

  `On` is a separate variant rather than a midnight `At` because folding a bare
  date into a timestamp would invent a time of day and render it back with one,
  so `Display` would report a precision the wire never carried.
* **Numeric ids the WSDL under-types as strings** (`U64_FIELDS`, plus
  `setConference`'s 20 prompt slots in `CONFERENCE_PROMPT_FIELDS`) map to
  `u64`. This is the class where the two inference sources disagreed
  wholesale: the doc samples gave the `get` side `u64` while the WSDL declared
  the `set`/`create` side `xsd:string`, so a caller who listed a record and
  then updated it converted each field by hand. Only the param side moves --
  every response already reported a number -- so no response gains a way to
  fail. Entries are the *wire* name, which is why `ring_group` is listed twice
  (`delRingGroup` spells it `ringgroup`). `id` names a different record in
  every method and is not always a number, so it is typed per struct instead
  (`NUMERIC_ID_PARAM_PATHS`): `getBackOrders` filters by the back order id its
  response reports as a number. `delLocation`'s `id` is an internal extension
  location, confirmed live to be a number (a location created with
  `setLocation` was `1713`, the built-in one is `0`), and its ident is
  `internal_extension_location` (`FIELD_IDENT_OVERRIDE`), the sub-account
  field that takes the same value; `location_id` already names an
  international DID location. `setCallerIDFiltering`'s `filter` is listed
  there too: it is the caller ID filter id the rest of the family takes and
  reports as `filtering`, so it is `u64` and its ident is `filtering`
  (`FIELD_IDENT_OVERRIDE`).

  The lookup codes (`protocol`, `device_type`, `lock_international`, `route`,
  `balance_management`, `carrier`, `voicemail_setup`) are in the same table
  for the opposite disagreement: the WSDL types the field that takes the code
  `xsd:integer` and the `get*` lookup's filter `xsd:string`. Both sides
  document the same code ("Code for a specific Protocol (Example: 3)" against
  "Values from getProtocols"), and the lookup reports it as a number in
  `value`, so a code read from the lookup is sent on without a conversion.
* **Identifiers an all-digit sample made look numeric** (`zip`, `password`,
  `security_code`, `dtmf_digits`, `callerid_prefix` in
  `IDENTIFIER_STRING_FIELDS`) map to `String`, the same reasoning as
  `PHONE_STRING_FIELDS`: a ZIP of `02134`, a PIN of `0123`, a dial string with
  `*` or `#`, and a prefix voip.ms reports as `MIA [555]` all lose information
  as a number. Here the *response* side moves, and only toward tolerance.
* **`pause` and the reseller fees** (`DECIMAL_FIELDS`) map to
  [`rust_decimal::Decimal`]. `setForwarding` documents `pause` as "0 to 10 in
  increments of 0.5", which `u64` cannot hold and the response already
  reported as a decimal. `minute`, `monthly` and `setup` are a reseller
  client's rate and fees, documented alike on `connectDID`, `connectFAX` and
  the seven orders ("Example: 3.50"); the WSDL declared them `xsd:decimal` on
  the first two and `xsd:string` on the orders, and a whole-dollar doc sample
  made three response fields `u64`, which a fee with cents would fail.
* **`maximum_callers`** joins `maximum_wait_time` in `WAIT_TIME_FIELDS` as
  [`crate::WaitTime`]. The type is selected by its sentinel's wire spelling,
  not by its name: the queue's caller cap is documented "1 to 60 or
  'unlimited'", which is `WaitTime`'s exact wire form, where `Seconds` writes
  `none` and `MaxMembers` a capitalized `Unlimited`.

  Some fields resist this alignment. `client` is `u64` except on `getClients`
  and `getDIDsInfo`, whose parameter is documented to accept an e-mail address
  or a sub-account name as well as the id; there it stays `String` and its
  ident is `client_filter`, since it holds a different set of values.
  `recording` is `u64` except on the call-hunting pair, which takes the system
  recording `default` as well as a recording id and reports either back
  verbatim (confirmed live by creating one of each), so it stays `String` and
  is in `check-types`'s `DELIBERATE_PARAMS`. Both use `field_type_skip` for the
  exceptions. `music_on_hold`'s `volume` is two different things sharing a
  name and is recorded in `check-types`'s `DELIBERATE` list: the param is the
  documented `1`/`0` quiet toggle, the response reports the rendition it
  produced (`mp3` / `quietmp3`).

  Four questions the documents could not settle were answered against the
  live API rather than guessed, using `cargo run --example call_raw`:
  `getReportEstimatedHoldTime` really does offer `once` beside `yes`/`no`, so
  `report_hold_time_agent` is the enum and not the `bool` its sample implied;
  a recording slot accepts `none` and `0` interchangeably and always reports
  `0`, so `u64` loses nothing; `volume=1` stores the quiet rendition while `0`
  (or anything else) stores the normal one; and a mailbox created from
  `digits=01` comes back as `1`, so the leading zero the docs' example shows is
  normalized away and `mailbox` is a number rather than an identifier.
* **Declarative enum overrides** in
  `tools/api-response-overrides.json` under the new `enums` (variant
  list with wire strings) and `field_types` (field-name → enum-name)
  sections. The generator emits the enum type (deriving `Debug`, `Clone`,
  `PartialEq`, `Eq` -- not `Copy`, since the `Unknown(String)` catch-all holds
  a `String`), `as_wire` / `from_wire`, `Display`, and substitutes the field's
  type in every `*Params` and `*Response` struct that has that field.

  **Only the serde direction a field reaches it through is emitted**, tracked
  in `EnumSides` while the structs render: `Serialize` for an enum some
  `*Params` writes, `Deserialize` for one some `*Response` reads, both for the
  15 that are both. A response field names no per-enum helper: it reads through
  the generic `deserialize_opt_from_wire_text::<Enum, _>`, the same reader
  `TimezoneName` uses, and the seconds-or-sentinel types go through
  `deserialize_opt_via::<T, _>` the same way. The same rule retired `ApiStatus`'s `Serialize`,
  `TimezoneName`'s `Serialize`, and `TimezoneOffset`'s `Deserialize`. Used
  for `DtmfMode`, `Nat`, `EmailAttachmentFormat`,
  `TranscriptionFormat`, `PlayInstructions`, `RingStrategy`,
  `RingGroupOrder`, `VoicemailFolder`, `QueueEmptyBehavior`,
  `EstimatedHoldTimeAnnounce`, `CallPickupBehavior`, `RecordingSort`,
  `DialingMode`, `TollFreeCarrier`, `DidBillingType`, and `LocationType`.
  Integer-coded enums (`1`/`2`, `-1`) work the same way -- the generated
  deserializer accepts the wire value as a JSON string, number, or bool.

Both kinds of substituted enum carry an `Unknown(String)` (or
`Unknown { tag, value }` for `Routing`) catch-all so VoIP.ms adding
a new variant or shipping an unexpected value never breaks
deserialization.

Field-name substitution is global but shape-aware: on the response side it
applies only to scalar-shaped fields, since a substituted scalar type can
never stand in for a list or object -- so a reference catalog returned
under an overridden field name (`getNAT`'s `nat`, `getPlayInstructions`'s
`play_instructions`) keeps its structural type automatically. Beyond that,
two JSON sections handle fields whose name means different things in
different structs:

* `field_type_skip` (`["StructName.field"]`) suppresses the name-based
  override for one struct -- on both the `*Params` and `*Response` side --
  keeping its WSDL/inferred/patched type. Two cases use it:
  `GetVoicemailsResponseVoicemail.urgent` is a *count*, not the per-message
  flag; and `getFaxMessages`'s `folder` is a free-text fax-folder name
  (`SENT` / `ALL` / user-created via `setFaxFolder`), not one of the fixed
  [`VoicemailFolder`] variants the global `folder` mapping would impose.
* `field_type_override` (`{"StructName.field": "EnumName"}`) is the
  assigning complement: it types one struct's field as a specific enum,
  overriding both the inferred type and any `field_types` entry. The `type`
  field needs this -- it's a search mode in `SearchVanityParams`, a message
  direction in `GetSMSResponseSMS`, and a reference-data code elsewhere, so
  no single global mapping fits. A per-struct entry wins over the global
  table on both the param and response side.

**Rationale**: Field names like `routing`, `dtmf_mode`, and `nat` mean
the same thing across every method they appear on. Substituting by
field name keeps the override table tiny and avoids per-method
duplication. Hand-written types stay in `src/types.rs` for cases that
need custom parsing; routine `set of fixed strings` enums are declared
in JSON to keep the generator the source of truth.

**How to apply**: For a new closed-set scalar (e.g. a `priority` field
with documented values `low`/`normal`/`high`), add an entry to `enums`
and a `field_types` mapping in `tools/api-response-overrides.json` and
regenerate. For a new boolean flag, add its field name to
`FLAG_01_FIELDS` or `FLAG_YES_NO_FIELDS` in
`xtask/src/field_overrides.rs` (no JSON or new type needed) --
`cargo xtask check-flags` audits those tables against the doc-mined
parameter descriptions and reports both uncovered flag-like params and
stale entries. It reads three spellings (`yes/no`, `Boolean: 1/0`, and a
`1=`/`0=` value list); the second was added after `cnam` and `sip_traffic`
stayed integers through an audit that reported "ok", because the bare form
names no value for the `1=`/`0=` rule to match. For a scalar
that needs structured parsing (multi-part value, custom validation),
hand-write it in `src/types.rs`, register the field names in
`xtask/src/field_overrides.rs::ROUTING_FIELDS`-style const, and add the
deserializer to `src/responses.rs`.

`cargo xtask check-types` is the complementary audit: it parses
`src/generated.rs` and reports a field a method family types one way on the
`set`/`create` side and another on the `get` side. It also reports a `*Params`
field ident typed one way in one struct and another way in the next, which the
family comparison cannot see when a code is looked up in one family and taken
in another (`getProtocols` and `createSubAccount`); a consumer that derives
one parameter schema per ident sees such a field with two types. A pair meant
to differ there goes in `DELIBERATE_PARAMS` (`folder`: a `VoicemailFolder`
everywhere but `getFaxMessages`, whose fax folders include ones the caller
named; `recording`: an id everywhere but `setCallHunting`, which also takes
`default`), and a field that holds a different set of values gets its own ident
instead. Both audits print and exit zero by default, since a finding wants
human judgment about which of the two types is right; both take `--deny`, which
turns a finding into a non-zero exit, and CI runs them that way. So "it reports
nothing" is a rule rather than a note here. A pair whose two types are meant to
differ goes in its `DELIBERATE` list with the reason, which is the record of
*why* rather than a way to silence it. A
collection type is excluded from the comparison: a root response's payload
list often shares its name with the record id it holds
(`GetDISAsResponse::disa` against `SetDISAParams::disa`), and no override
could make those one type.
