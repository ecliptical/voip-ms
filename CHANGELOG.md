# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.0] - 2026-09-21

### Fixed

- The four methods that take a base64-encoded file are sent as a
  `multipart/form-data` POST instead of a GET, which is what makes them usable
  at all: `set_recording`, `send_fax_message`, `send_mms` (`media2`), and
  `add_lnp_file`. VoIP.ms's front end caps the request line at 8190 bytes, so a
  GET left roughly 8 kB for the whole parameter set -- about a third of a
  second of 8 kHz mono audio for `set_recording`, against 60,428 base64
  characters for a 2.8 second greeting. `add_lnp_file` is documented "Only
  accepted through POST request" and could not work over GET at any size.
  - Every other method is still a GET. The transport is decided per method from
    the presence of a base64 file parameter, so no call site changes and the
    218 methods that can stay observable in a log or proxy do.
  - The POST is `multipart/form-data` specifically.
    `application/x-www-form-urlencoded` reaches a SOAP handler on `rest.php`
    and comes back as an XML fault, which is what makes the API look GET-only
    on a first test.
  - A multipart call carries the credentials as form fields, so for those four
    methods the API password no longer appears in the request URL.
- **Breaking**: `GetTransactionHistoryResponseTransaction::date` is
  `Option<TransactionDate>` instead of `Option<chrono::NaiveDateTime>`. The row
  that aggregates communication charges over the requested window reports that
  window -- `2026-08-01 to 2026-08-31` -- in place of a timestamp, and a strict
  datetime failed the whole envelope on it, so a caller whose range covered any
  billed calls got an error instead of the transactions beside it.
  `TransactionDate` reads a timestamp (`At`), a bare date (`On`), or a range
  (`Period`), and keeps anything else verbatim in `Unrecognized`, so the next
  surprise in that field cannot cost a response.
  - The docs' Output block shows only a timestamp, so the extractor had nothing
    else to infer from. The type is assigned per struct in
    `TRANSACTION_DATE_RESPONSE_PATHS`, since `date` elsewhere is a point in time
    and never a range.
  - `getCharges` and `getDeposits` keep their `Option<chrono::NaiveDate>`. They
    are the same ledger kept for a reseller client, but neither takes a date
    range, so neither has a window to aggregate over and neither can report one.
- `TransportFailure::never_reached_upstream()` answers `false` for HTTP 408,
  where every other 4xx still answers `true`. The method claims the request
  *provably* never reached VoIP.ms, and 408 does not prove that: RFC 9110
  §15.5.9 defines it as the origin giving up on an incomplete request, which
  would be safe, but intermediaries widely return it for a slow *response*,
  where VoIP.ms may have acted and the reply been lost. A consumer gating an
  agent's retry advice on this told it that a 408 on a destructive,
  irreversible call had changed nothing, which invites a double order.
  `retry_outlook()` is unchanged -- 408 stays `AfterWaiting`, since "not now"
  is still the right reading of it.
- `cargo xtask check-flags` recognizes the `(Boolean: 1/0)` spelling the docs
  use alongside `1 = Enable / 0 = Disable`. The bare form names no value, so
  the `1=`/`0=` rule could not see it, and the audit reported "ok" while `cnam`,
  `sip_traffic`, and `setMusicOnHold`'s `volume` stayed integers.
- `cargo xtask dump-methods` reads `call_multipart_raw` as well as `call_raw`,
  so the four file-carrying methods stay in the wire-method list the live
  harness's completeness gate partitions. It has read only `call_raw` since
  before those methods moved to a POST in this release, so re-running it would
  have dropped them.
- The README's snippets compile. They named `SendSmsParams` (the type is
  `SendSMSParams`), pinned `voip-ms = "0.3"`, and claimed every `*Params` and
  `*Response` field is `Option<T>`, which stopped being true in 0.6.0. They are
  now compiled as doctests, so a call site shown there cannot drift from the
  surface again.

### Added

- `Client::call_multipart` and `Client::call_multipart_raw`: the multipart-POST
  counterparts of `Client::call` and `Client::call_raw`, for calling a method
  with a file payload that this crate hasn't been regenerated for. Same status
  handling as the GET pair, including how each treats an empty-collection
  status. `Client::call_multipart_raw_unchecked` pairs with
  `call_raw_unchecked` under the `unchecked-raw` feature, so diagnosing an
  unexpected status on a file method can use the transport that method needs.
- `requires_multipart(method)`: whether a wire method has to be a POST. The
  generated methods apply it themselves; it is public for a caller that
  dispatches by method name and so cannot otherwise tell.
- `attach_offset` completes the bare wall clocks in a record-listing envelope
  with the offset the request carried, the step the typed methods take before
  deserializing. Public because a `call_raw` caller needs it too: the raw
  envelope still reports its timestamps without the offset. It takes a
  `*`-wildcard path, not the RFC 6901 JSON pointer `Client::call_at` takes.
- `GET_CDR_TIMESTAMPS`, `GET_SMS_TIMESTAMPS`, `GET_MMS_TIMESTAMPS` and their
  three reseller siblings: the paths `attach_offset` needs for each method,
  emitted by the same codegen pass that types the fields.
- `TimezoneOffset::UTC` and `TimezoneOffset::to_fixed_offset`. A zone off the
  hour keeps its fraction through both (`Asia/Kolkata` sends `5.50` and its
  timestamps come back qualified `+05:30`).
- Every generated `*Params` and `*Response` struct derives `PartialEq` and
  `Eq`, so a test can compare a whole response and a consumer can dedupe or
  diff records without writing them out field by field.
- `NoParams`, the parameters of a method that takes none. Public so a caller
  reaching one of the eight parameterless methods by wire name through
  `Client::call_raw` has something to serialize.
- A `new` constructor on each `*Params` struct with between one and six fields
  the docs mark `(required)`, taking exactly those. Every field stays `Option`
  and struct-update syntax still works; the constructor only spares a caller
  from reading the API docs to learn which fields the method needs. Structs
  with more required fields than that (`AddLNPPortParams` has 12) get none: an
  unlabeled argument list that long reads worse than the struct literal. The
  offset ops' `timezone` is excluded -- the docs mark it required and the crate
  defaults it to UTC.
- An `examples/call_raw.rs` that calls any wire method by name with
  `key=value` parameters and prints the envelope. The typed methods answer what
  a value *is*; this answers what VoIP.ms actually sent, which is what settles a
  field whose documentation and response sample disagree. It picks the
  transport with `requires_multipart` and prints a non-`success` status rather
  than raising it, so an error envelope reads as easily as a successful one.
- `Client::api_username()`, so a consumer holding several clients (a reseller
  plus its sub-accounts) can label a log line from the client rather than
  carrying the username beside it. `Debug` already printed it.
- `FromStr` on `ApiStatus` and on every generated wire enum, infallible because
  each has an `Unknown` catch-all. `ApiStatus` also gained a `Success` variant:
  a typed response's `status` reports it, and it was previously the one status
  that landed in `Unknown`.
- `Seconds::as_u64`, `WaitTime::as_u64`, and `MaxMembers::as_u64`: the count,
  or `None` for the unbounded sentinel, so reading it does not need a match.
- `serde` is re-exported from the crate root. AGENTS.md and the 0.3.0 entry
  both said it already was; it was not, and the README's `call_at` example
  needed a separate dependency to compile.
- `cargo xtask check-types` reports a field a method family types one way to
  read and another way to write. It parses the emitted surface, so it describes
  what shipped rather than what the inputs say. It reports nothing today:
  every pair it found is corrected below, and the three meant to differ carry
  their reason in its `DELIBERATE` list.
- The live harness diffs every raw response against the key paths the typed
  surface models, and reports an `unmodeled` outcome for a key no `*Response`
  field claims. The existing raw-vs-typed probe could never have found `ip` and
  `useragent`: it fires only when a typed read *fails*, and an unknown key
  deserializes away without failing anything. `cargo xtask gen` emits the
  modeled paths into `livetest/src/response_fields.rs` alongside the structs and
  from the same shapes (`cargo xtask dump-fields` rebuilds that file alone), and
  the report prints the `additions` entry to paste. `getCDR` also moved to probe
  depth, so a read-only run sees a populated record -- the only place the
  per-record fields are visible at all.
- The response overrides gained an `additions` section, which appends a scalar
  field to an extracted shape (`{ "path": "cdr[].ip", "type": "string" }`). A
  docs-driven extractor cannot see an undocumented field by construction, and
  the alternative -- replacing the whole method's shape by hand -- would freeze
  it against later doc updates. Declaring a field the docs later pick up fails
  the codegen, so the stale entry gets deleted.

### Changed

- `reqwest`'s `multipart` feature is enabled. Feature selection is additive, so
  a consumer that names its own `reqwest` features keeps them and gains
  `multipart` -- and the dependencies it brings -- along with them.
- `Error::Api`'s `Display` renders the documented meaning beside the code:
  `API status: did_in_use (DID Number is already in use)`. An undocumented code
  still renders alone. `as_str()` and `ApiStatus`'s own `Display` are unchanged.
- `src/generated.rs` carries `#![allow(clippy::upper_case_acronyms)]`. The type
  names keep VoIP.ms's acronym casing (`GetDIDsInfoParams`, `SendSMSResponse`),
  which is deliberate but departs from C-CASE, so clippy reported this crate's
  types against a consumer's own build.
- **Breaking**: the six record-listing methods report their timestamps with the
  UTC offset the call asked for. `GetCDRResponseCDR::date`,
  `GetResellerCDRResponseCDR::date`, `GetSMSResponseSMS::date`,
  `GetMMSResponseSMS::date`, `GetResellerSMSResponseSMS::date`, and
  `GetResellerMMSResponseSMS::date` are now
  `Option<chrono::DateTime<chrono::FixedOffset>>` instead of
  `Option<chrono::NaiveDateTime>`. The crate already computed the offset VoIP.ms
  would apply and then dropped it, so a caller who passed a `timezone` got back
  a wall clock with no way to recover the zone. A consumer read one as UTC and
  reported a time that had already passed.
  - The type is a fixed offset, not a zone. VoIP.ms takes one number for the
    whole range, so a range straddling a DST transition comes back at the
    pre-transition offset throughout; a `DateTime<Tz>` would shift the far side
    by an offset the server never applied.
- **Breaking**: those methods send an explicit `timezone` on every call,
  defaulting to `TimezoneOffset::UTC` when the caller names no zone. Omitting it
  selects the account's configured zone, which nothing in the API reports -- a
  timestamp returned in it could only be guessed at. A caller who relied on the
  account default now gets UTC and should pass the zone it was set to.
- **Breaking**: `GetCDRResponseCDR` carries the `ip` and `useragent` fields as
  `Option<String>`. The struct is generated without `#[non_exhaustive]` and all
  its fields are public, so a downstream struct literal or an exhaustive
  destructure without `..` stops compiling.
  `getCDR` returns both on the wire, but the docs' Output
  block does not list them, so the extractor could not see them and they were
  discarded during deserialization. A live check identified them: `ip` is the
  originating client's public address and `useragent` its SIP User-Agent. Two
  SIP clients calling from one sub-account within the same minute reported
  distinct agents and a shared public address, so the pair describes the client,
  not the account.
  - An outbound call from a registered client carries both whether it connects
    or not: a 19-second billed call and a 0-second failure reported the same
    pair. They remain best-effort, though -- an inbound row, an internal echo
    test, and two outbound attempts the record cannot be told apart from ones
    that populated carried neither. An empty scalar folds to `None`, and a
    consumer cannot read an empty `ip` as "no device placed this call".
  - The observed `useragent` arrived truncated mid-token, so the value is not
    necessarily a complete User-Agent. That, and an address voip.ms is equally
    free to clip, is why both stay `String` rather than `IpAddr` or a parsed
    agent: a strict parse would fail the whole response.
- **Breaking**: `status` on every `*Response` is `ApiStatus` rather than
  `Option<String>`. By the time a typed call returns, the crate has already
  parsed the status and decided whether it is `success` or an empty-collection
  code, so handing back the raw string made the caller parse it again. The
  field is required, not optional: a response missing `status` is already an
  `Error::InvalidResponse` before deserialization. A record's own `status` (a
  fax's, a port's, an e911 record's) is unrelated and keeps its string type.
- **Breaking**: `ApiStatus::is_empty()` is `ApiStatus::is_empty_collection()`.
  On an enum, `is_empty` reads as "this status is blank", and it shadowed the
  `is_empty` a `Vec<ApiStatus>` has. The meaning is unchanged: the code says
  the requested collection has no entries.
- **Breaking**: every field a method family typed one way to read and another
  way to write now shares one type. The response side was
  inferred from the docs' sample output and the param side declared by the
  WSDL, and each side was internally consistent, so nothing caught the
  disagreement; a caller who listed a record and then updated it converted
  each field by hand. `cargo xtask check-types` is the tripwire from here on.
  - **Record ids become `u64`** on the write side, matching what every
    response already reported: `callback`, `call_hunting`, `client`,
    `conference`, `disa`, `filtering`, `forwarding`, `group`, `ivr`,
    `mailbox`, `member`, `phonebook`, `queue`, `recording`, `ring_group`,
    `sipuri`, `timecondition`, `voicemail`, plus `canada_routing`,
    `internal_dialtime`, `internal_extension`, `internal_voicemail`,
    `priority_weight`, `reseller_client`, `reseller_package`, the four
    recording slots (`agent_announcement`, `caller_announcement`,
    `voice_announcement`, `unavailable_message_recording`), and
    `setConference`'s 20 `sound_*` prompts. `reseller_nextbilling` becomes
    `Option<chrono::NaiveDate>`.
    - The recording slots are `u64` although some are documented as a code
      *or* the word `none`. Confirmed against the live API on a ring group:
      `none` and `0` are interchangeable going in, and the read side reports
      `0` either way, so `Some(0)` clears the slot and nothing is lost.
  - **Identifiers an all-digit doc sample made look numeric become `String`**
    on the read side: `zip`, `password`, `security_code`, `dtmf_digits`, and
    `callerid_prefix`. Each lost information as a number -- a US ZIP of
    `02134`, a voicemail PIN of `0123`, a dial string carrying `*` or `#`, and
    the `MIA [555]` prefix `getDIDsInfo` actually reports. The same sample
    artifact made four fax/e-mail `id` fields strings where their own params
    are integers; those become `u64` (`getEmailToFax`, `getFaxFolders`,
    `getFaxMessages`, `getFaxNumbersInfo`).
  - `cnam` and `sip_traffic` are documented `(Boolean: 1/0)` and were spread
    across `Option<u64>` and `Option<String>` on ten param structs that did not
    agree with each other. `Some(1)` becomes `Some(true)`; the `1`/`0` wire form
    comes from the serializer, as it does for every other flag.
  - `setForwarding`'s `pause` is `Option<rust_decimal::Decimal>`: it is
    documented "0 to 10 in increments of 0.5", which the previous `String`
    obscured and a `u64` could not hold.
  - `setQueue`'s `maximum_callers` is `Option<WaitTime>`, documented "1 to 60
    or 'unlimited'" -- the count or the sentinel, the same shape
    `maximum_wait_time` already had.
  - `report_hold_time_agent` is `Option<EstimatedHoldTimeAnnounce>` on both
    sides. Its `yes` sample made the extractor read it as a boolean, but
    `getReportEstimatedHoldTime` also offers `once`, which a `bool` drops.
  - `setMusicOnHold`'s `volume` is `Option<bool>`, the `1`/`0` quiet toggle the
    docs describe. `getMusicOnHold`'s same-named field stays `String` and is
    *not* that flag: it reports the rendition the toggle produced (`mp3` or
    `quietmp3`), which a live read settled -- the docs' `mp3` sample looked
    like a misaligned column and is not.
  - Three fields are deliberately left divergent, each recorded with its
    reason: `client` on `getClients` and `getDIDsInfo` (documented to accept an
    e-mail address or a sub-account name as well as the id), `recording` on the
    call-hunting pair (whose response reports the system name `default`), and
    `volume` above.
- **Breaking**: the 25 fields named `r#type` have descriptive names. The
  DID and toll-free searches take `search_type`, `searchVanity` takes
  `vanity_type`, the SMS and MMS params and responses take `direction` (as do
  `getCallRecording` and `getCallRecordings`), a porting attachment's is
  `file_type`, `getTransactionHistory`'s is `transaction_type`,
  `getDIDCountries`'s is `international_type`, and the five reference-data
  lookups take `code`. The wire name is unchanged.
- **Breaking**: the eight methods the WSDL declares no parameters for take no
  argument: `get_call_accounts`, `get_call_billing`, `get_ip`,
  `get_lnp_list_status`, `get_locations`, `get_provinces`, `get_states`, and
  `get_vpris`, each with its `*_raw` twin. Their empty `*Params` structs are
  gone. Should one grow a parameter upstream, the method gains an argument,
  which the generator makes visible as the breaking change it is.
- **Breaking**: `ClientBuilder::build()` returns `Client` rather than
  `Result<Client>`. Its only fallible step was parsing a string literal this
  crate owns, and a failure surfaced as `Error::InvalidResponse`, a variant
  about HTTP bodies. The default URL is parsed once into a `LazyLock`, and
  `Client::new` no longer documents a panic.
- **Breaking**: a type no longer carries a serde direction or a `Default`
  that nothing reaches. Both kinds cost something to keep: a serde impl is a
  wire contract that has to stay correct, and a `Default` manufactures a
  value. The purely structural derives (`Hash`, `PartialOrd`/`Ord`,
  `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`) cost nothing and are
  unchanged, so a consumer can still key a map by `ApiStatus` or sort a
  `TimezoneOffset`.
  - `ApiStatus` no longer
    implements `Serialize`, `TimezoneName` no longer implements `Serialize`
    (nothing writes a zone name; only `getTimezones` and `getVoicemails`
    report one), and `TimezoneOffset` no longer implements `Deserialize` (it
    only ever goes out, on the private wire twin). A generated wire enum now
    carries only the direction a field reaches it through, so
    `CallPickupBehavior` reads, and `DialingMode`, `SearchType` and
    `VanityType` write.
  - That also retires the 19 `#[allow(dead_code)]` attributes the generator
    emitted to hide the readers nothing called. An `allow` covering generated
    code is the shape of the problem rather than a fix for it.
  - `Default` is gone from every `*Response` and from `ApiStatus`. A response
    is received, never built, and a defaulted one claimed `status: Success`
    over empty fields; `Success` was the default only because the derive
    needed one, which is not a reason. `*Params` keep `Default` -- the
    struct-update idiom is built on it -- and the per-field
    `#[serde(default)]` is unaffected, since it defaults the field's own type.
  - Each is additive to restore, so a consumer who needs one can ask.
- **Breaking**: `Error::InvalidParams` carries a `ParamsError` rather than a
  `TimezoneOffsetError`. The variant name was general and its payload
  specific, so the next parameter check had nowhere to go.
  `ParamsError::Timezone` holds the previous payload, and `?` converts through
  both hops, so the next kind of validation is additive.

### Upgrading

The multipart change asks nothing of a call site: the four methods keep their
signatures and their `*Params` structs, and the transport is chosen inside
`Client`. A consumer that derives its own artifacts from this crate's method
surface -- a generated tool catalog, for instance -- should regenerate them,
since those four methods' doc comments now name their transport.

The timestamp change does ask something. Code reading `date` off any of the six
record-listing responses now holds a `DateTime<FixedOffset>`: call
`.naive_local()` for the previous wall-clock value, or keep the offset and drop
whatever local re-zoning stood in for it. Code that passed no `timezone` and
relied on the account's configured zone now receives UTC, and should pass the
zone that account is set to.

The rest is mechanical and the compiler finds all of it:

| Was | Is |
|---|---|
| `builder.build()?` | `builder.build()` |
| `status.is_empty()` | `status.is_empty_collection()` |
| `resp.status.as_deref() == Some("success")` | `resp.status == ApiStatus::Success` |
| `client.get_ip(&GetIPParams {})` | `client.get_ip()` |
| `SearchDIDsUSAParams { r#type, .. }` | `SearchDIDsUSAParams { search_type, .. }` |
| `GetSMSResponseSMS::r#type` | `GetSMSResponseSMS::direction` |
| `cnam: Some(1)` | `cnam: Some(true)` |
| `queue: Some("32208".into())` | `queue: Some(32208)` (and every other record id) |
| `pause: Some("1.5".into())` | `pause: Some(Decimal::from_str_exact("1.5")?)` |
| `maximum_callers: Some("10".into())` | `maximum_callers: Some(WaitTime::Value(10))` |
| `report_hold_time_agent: Some("yes".into())` | `report_hold_time_agent: Some(EstimatedHoldTimeAnnounce::Yes)` |
| `client.zip` as `u64` | `client.zip` as `String` (and `password`, `security_code`, `dtmf_digits`, `callerid_prefix`) |
| `transaction.date` as `NaiveDateTime` | `transaction.date.as_ref().and_then(TransactionDate::at)` for the same value |
| `Error::InvalidParams(e)` | `Error::InvalidParams(ParamsError::Timezone(e))`, and `ParamsError` is `#[non_exhaustive]`, so a `match` on it needs a wildcard arm |

## [0.12.2] - 2026-09-17

### Added

- `Error::transport()` classifies a failed request as a `TransportFailure`, and
  returns `None` for a failure that is not one. The classification and the two
  questions that hang off it are properties of HTTP and of the VoIP.ms API, not
  of any one consumer, and they were being re-derived per consumer -- including
  one hand-written copy that had already drifted.
  - `TransportFailure` is `Rejected(reqwest::StatusCode)`, `Timeout`, `Dns`,
    `Connect`, `Body`, or `Other`. `reqwest` is re-exported, so `StatusCode` in
    the public API costs callers no new dependency. Classification order is
    part of the contract: a status is read before the predicates, and `is_dns`
    before `is_connect`, since reqwest reports a resolution failure through the
    connect error that wraps it and both predicates answer true.
  - `TransportFailure::never_reached_upstream()` reports whether the request
    provably never reached VoIP.ms, so no account state can have changed. A 4xx
    counts -- the transport refused the request before VoIP.ms could act on it.
    A 5xx, a timeout, or an unreadable reply do not: VoIP.ms may have acted and
    the response been lost.
  - `TransportFailure::retry_outlook()` reports whether repeating the identical
    call is worth anything, as `RetryOutlook::Worthwhile`, `Futile`,
    `AfterWaiting`, or `Unknown`. This is a separate question from
    `never_reached_upstream()`: 429 and 408 are `AfterWaiting`, every other 4xx
    is `Futile`, and collapsing the two told one consumer that a stale proxy
    credential answering 401 to every call was safe to retry.
  - An allow-list rejection stays out of all of this. VoIP.ms answers it on a
    200 with `ApiStatus::IPNotEnabled` in the envelope, not as an HTTP 403, so
    `transport()` returns `None` for it -- the distinction consumers kept
    getting wrong.
- No `Display` for either type. Each consumer writes its own words: a model
  reading a retry decision and a person reading a terminal diagnostic want
  different sentences, and only the reading is shared.

The addition is purely additive. `Error::Http` keeps its shape and its inner
`reqwest::Error`, and the classification reads only the error's kind, so the
URL stripping that 0.12.1 added still holds -- covered by a test that classifies
a real failed request and asserts nothing it exposes carries the password.

## [0.12.1] - 2026-09-17

### Security

- The API password no longer reaches error or `Debug` output. The crate
  authenticates by query parameter, so every request URL carries a live
  `api_password`; two paths printed it verbatim.
  - `Error::Http` now wraps a `reqwest::Error` whose URL has been stripped.
    A `reqwest::Error` renders its URL from both `Display` and `Debug`, so any
    consumer writing `format!("{e}")`, `e.to_string()`, `%e`, `?e` or `{e:#}`
    printed the password. The stripping happens in the `From<reqwest::Error>`
    conversion, so it covers every fallible call site at once. The variant
    keeps its shape and its `source()`, and the inner error's classification
    (`status()`, `is_timeout()`, `is_connect()`, `is_body()`, `is_decode()`)
    is unaffected -- all of it reads the error's kind, not its URL.
  - `Client` and `ClientBuilder` implement `Debug` by hand instead of deriving
    it. The derive printed `api_password` directly, which no URL-based
    redaction could catch. The password renders as `<redacted>`;
    `api_username` is still shown (it identifies the account and authenticates
    nothing on its own), and `base_url` is shown with any `user:pass@` userinfo
    stripped, since a caller-supplied proxy URL may embed credentials that
    `Url`'s own `Display` prints verbatim.

### Changed

- Raised the `reqwest` floor to 0.13.5. `reqwest::Error::is_dns`, which
  consumers call on the error inside `Error::Http` to classify a failure, does
  not exist in the previous 0.13.4 floor.
- Raised the `rust_decimal` floor to 1.43, its current minor. Both bumps are
  semver-compatible for callers naming the re-exported `voip_ms::reqwest` /
  `voip_ms::rust_decimal`.

## [0.12.0] - 2026-07-21

### Changed

- Every timezone across the API surface is now a `chrono_tz::Tz` (re-exported
  as `voip_ms::chrono_tz`); the crate translates to each method's wire format
  internally. Breaking for callers that passed the previous `String` /
  `rust_decimal::Decimal` values.
  - **Record-listing offset methods** (`getCDR`, `getResellerCDR`, `getSMS`,
    `getMMS`, `getResellerSMS`, `getResellerMMS`): the public `timezone` is
    `Option<Tz>`. The wire wants a numeric UTC offset (`-12` to `13`), so each
    call resolves the zone's offset at the query start date (`date_from` /
    `from`) -- DST-aware, e.g. `America/New_York` sends `-5` in January and
    `-4` in July -- via a generated `*ParamsWire` twin. A zone with no start
    date to anchor the resolution, an unparseable start date, or an offset
    outside VoIP.ms's range (e.g. `Pacific/Kiritimati`, +14) fails the call
    with the new `Error::InvalidParams` before any request is sent. VoIP.ms
    applies one offset to the whole range, so rows across a DST boundary can
    still be off by an hour -- inherent to the API, not resolvable client-side.
  - **Named-zone params** (`createVoicemail` / `setVoicemail` /
    `getTimezones`): typed `Tz`, carried on the wire as the IANA name
    (`America/New_York`).
  - **Named-zone responses** (`getVoicemails`, `getTimezones`): typed the new
    `TimezoneName` -- `Known(Tz)` for a recognized zone, `Unrecognized(String)`
    verbatim otherwise. VoIP.ms's `getTimezones` catalog still lists legacy
    names the IANA database has dropped (`Asia/Beijing`, `US/Pacific-New`,
    `Factory`, the old Saudi `Riyadh87`/`88`/`89` zones,
    `Canada/East-Saskatchewan`) -- confirmed live, where a strict `Tz` failed
    the whole response on the first one encountered.
  - New `TimezoneOffset` -- the validated numeric wire form (`-12..=13`,
    fractional-hour capable) that zones resolve into via
    `TimezoneOffset::at(tz, date)`; public for callers that need the raw
    number. `Display` renders `UTC-05:00`.

### Fixed

- `GetMediaMMSResponse::media` is now a `HashMap<String, String>` (index ->
  media URL) instead of a scalar `String` plus mis-inferred `field_0`/`field_1`/
  `field_2` siblings. The API docs render the field garbled, so the extractor
  mis-inferred its shape; live, `getMediaMMS` returns `media` as an object
  (`{"1": "https://voip.ms/media/.../media.jpeg"}`), which the old type failed
  to deserialize. Confirmed by a live `sendMMS` -> `getMediaMMS` round trip.
- `OrderFAXNumberResponse::dids` is now a `Vec<String>` (the ordered fax
  numbers) instead of a scalar `String` plus mis-inferred `0`/`1`/`2` sibling
  fields -- the same garbled-doc mis-inference as `getMediaMMS`. Live, a
  successful `orderFaxNumber` returns `dids` as an array (`["5878831590"]`),
  which the old type failed to deserialize, so every successful fax order
  errored client-side after the number was already provisioned.

## [0.11.0] - 2026-07-17

### Changed

- `DelRingGroupParams` names its id field `ring_group` (was `ringgroup`),
  matching `GetRingGroupsParams` and `SetRingGroupParams` so a consumer who
  just listed or configured a ring group reuses the same field to delete it.
  The upstream `delRingGroup` wire parameter is still `ringgroup` (a
  `#[serde(rename)]` maps it back). Breaking for callers that named the old
  field.

## [0.10.2] - 2026-07-11

### Fixed

- Corrected the wire values of three hand-curated enums, audited against the
  live reference endpoints they document (`getVoicemailAttachmentFormats`,
  `getRingStrategies`, `getPlayInstructions`):
  - `EmailAttachmentFormat::Mp3` serializes/deserializes `wavmp3` (was the
    nonexistent `mp3`).
  - `RingStrategy` drops the phantom `Linear` and `WRandom` variants
    (`getRingStrategies` rejects both; only `ringall`, `leastrecent`,
    `fewestcalls`, `random`, `rrmemory` are valid).
  - `PlayInstructions` drops the phantom `DontSay` variant
    (`getPlayInstructions` accepts only `u` / `su`).

## [0.10.1] - 2026-07-11

### Fixed

- `getConference` reports an uncapped conference's `max_members` as the word
  `Unlimited`, which the `u64` typing could not deserialize. The field is now
  a `MaxMembers` (a count or `Unlimited`), so the response decodes.
- A method that answers a successful call with an empty body (e.g.
  `delConference`) no longer fails with a JSON parse error: an empty response
  body is treated as `{"status":"success"}`.

### Removed

- The `live_api_verify` example and its `live-api-verify.yaml` workflow, both
  superseded by the `livetest` workspace member. `livetest` covers the full API
  surface (all 222 methods across functional areas) with raw-vs-typed drift
  diffing, populated-fixture lifecycles, and a pre-flight sweep, where the
  example probed a fixed ~20 methods. See "Live API verification (`livetest`)"
  in DEVELOPMENT.md for how to run it.

## [0.10.0] - 2026-07-11

### Changed

- **Breaking:** the response identifier fields `DIDAdded` (`assignDIDvPRI`),
  `DIDRemoved` (`removeDIDvPRI`), and `deleted_did` (`cancelFaxNumber`) are
  `Option<String>` (were `Option<u64>`). Each reports the single DID the call
  acted on -- a phone number, which must stay a string for the same reasons as
  every other DID field (leading zeros, non-NANP forms, values beyond `i64`).
  The doc-sample extractor inferred `integer` from an all-digit sample, and the
  method-specific wire names missed the name-based phone-number override until
  now.
- **Breaking:** the `getCDR` / `getResellerCDR` response `uniqueid` is
  `Option<String>` (was `Option<u64>`). A call-record unique id can be
  alphanumeric (e.g. `12964421x41098i8c`), which no integer type can hold, so
  the old typing failed to deserialize a real value; the
  `getTransactionHistory` `uniqueid` was already `String`.

## [0.9.0] - 2026-07-10

### Changed

- **Breaking:** integer request params are `u64` (was `i64`), matching the
  response side. Every VoIP.ms integer param is a non-negative id or count
  (the documented `-1` sentinels are enum-typed), and ids read from responses
  are `u64`, so the old `i64` forced a cast on every get/set round-trip.
- **Breaking:** decimal request params are `rust_decimal::Decimal` (was
  `f64`). The affected params are money amounts (`charge`, `payment`,
  `setup`, `monthly`, `minute`) plus `timezone`; `Decimal` serializes the
  exact value, where `f64` could ship float artifacts on the two methods
  that move money.
- **Breaking:** `date_from` / `date_to` params are `chrono::NaiveDate` (was
  `String`); its `Serialize` emits the documented `YYYY-MM-DD` wire form.
  The crate's `chrono` dependency gains the `serde` feature for this.
- **Breaking:** generated field identifiers are snake_case. Wire names that
  are camelCase (`isMobile`, `rateCenter`, `emailToFax`) or run-together
  acronyms (`sipuri`) become idiomatic idents (`is_mobile`, `rate_center`,
  `email_to_fax`, `sip_uri`) with a serde `rename` preserving the wire form
  on both the params and response side; `#![allow(non_snake_case)]` is gone
  from the generated module. Params and responses now share one
  keyword-escaping ident helper, closing a latent gap where a param named
  `match` or `ref` would have emitted invalid Rust.
- **Breaking:** `addLNPPort`'s `locationType` param and `getLNPDetails`'s
  matching response field are the new `LocationType` enum
  (`Residential`/`Business`, wire `0`/`1`) instead of a bare number.
- Name-based field-type substitution no longer applies to collection-shaped
  response fields -- a scalar override can never stand in for a list/object,
  so reference catalogs (`getNAT`, `getPlayInstructions`) keep their
  structural types without needing `field_type_skip` entries (both entries
  removed).
- New `cargo xtask check-flags`: audits the hand-curated boolean-flag tables
  against the doc-mined parameter descriptions, reporting flag-like params
  not yet typed as `bool` and stale table entries. Finding `locationType`
  above was its first catch.
- **Breaking:** the DID-identifier rule below now covers every phone-number
  field: `number`, `phone_number`, `contact`, `destination`, and `stationid`
  join `did` in the generator's built-in String-override table
  (`PHONE_STRING_FIELDS`). Concretely, `SetCallbackParams.number` and
  `SetPhonebookParams.number` change from `Option<i64>` to `Option<String>` --
  the WSDL declared them `xsd:integer` even though `setPhonebook` documents
  `sip:2563` as a valid value, so a SIP phonebook entry was previously
  impossible to send and a get/set round-trip required a lossy parse (the
  matching response fields were already `String`). All previously patched
  response fields keep their exact types; the per-method patches that
  re-stated this rule are removed from `tools/api-response-overrides.json`,
  and `cargo xtask gen` now warns when a patch is shadowed by a field-name
  override.
- **Breaking:** DID identifier fields are now `String`, never numeric. The WSDL
  declared the `did` parameter of `getFaxNumbersInfo`,
  `getFaxNumbersPortability`, `setFaxNumberEmail`, `setFaxNumberInfo`, and
  `setFaxNumberURLCallback` as `xsd:integer`, so they were generated as
  `Option<i64>`; a DID is a phone number (an identifier, not a quantity) that
  can carry leading zeros and exceed `i64` range, so passing one lost
  information. They are now `Option<String>`, uniform with every other `did`.
  Likewise `getDIDvPRI`'s `dids` response field was inferred as `Vec<u64>` from
  a numeric-looking sample but holds DIDs; it is now `Vec<String>`.

### Fixed

- Register four undocumented empty-collection statuses that the live API returns
  for an empty list but that are absent from the API's error-code table:
  `no_emailtofax` (`getEmailToFax`), `no_folder` (`getFaxFolders`),
  `no_transactions` (`getTransactionHistory`), and `no_vpri` (`getVPRIs`). The
  typed methods now return an empty response for these instead of `Error::Api`.
  Found by auditing every list-returning method against the live API.

### Removed

- The `live_api_verify` example and its `live-api-verify.yaml` workflow, both
  superseded by the `livetest` workspace member. `livetest` covers the full API
  surface (all 222 methods across functional areas) with raw-vs-typed drift
  diffing, populated-fixture lifecycles, and a pre-flight sweep, where the
  example probed a fixed ~20 methods. See "Live API verification (`livetest`)"
  in DEVELOPMENT.md for how to run it.

## [0.8.0] - 2026-07-09

### Fixed

- `searchFaxAreaCodeCAN` and `searchFaxAreaCodeUSA`: `ratecenters` is a list of
  `{area_code, available, ratecenter}` objects, not a scalar. The doc sample was
  a mis-parsed `print_r` dump that flattened the array into an `array(` scalar
  plus a spurious `0` field, so an area code with matches failed with "expected
  string, number, or bool, got [{...}]". An area code with no matches returns
  `{"status":"success"}` with no `ratecenters` field, which now deserializes to
  an empty list.

## [0.7.0] - 2026-07-09

### Fixed

- Five response fields whose types did not match what the live API returns,
  each of which failed deserialization of an otherwise-successful call:
  - `getTerminationRates`: `route` is a list of `{value, description}` entries,
    not a single object.
  - `e911AddressTypes`: `types` is a list of `{value, description}` catalog
    entries, not flattened scalars (the doc sample was a mis-parsed `print_r`
    dump).
  - `getFaxNumbersInfo`: a number's `did` is a dotted string
    (`647.948.4755`), typed `String` instead of `u64`.
  - `getLNPListStatus`: `list_status` is a string-keyed `code => description`
    map (including an empty-string key), now a
    `HashMap<String, String>` instead of a scalar.
  - `getReportEstimatedHoldTime`: the `types` entries carry free-text
    `value`/`description` strings (e.g. `"once"` / `"Yes, only once"`), not
    yes/no booleans.
- Four more responses whose `print_r` doc sample was flattened into sibling
  scalars instead of the real nested shape:
  - `e911Info`: `info` is a nested object (`did`, `full_name`, address parts,
    …), not top-level scalars.
  - `getLNPList`, `getLNPNotes`, `getLNPAttachList`: `list` is a list of
    objects (`{portid, numbers, foc_date, status}` / `{note, date, time}` /
    `{attachid, type, size}`), not flattened scalars.
- DID and phone-number identifier fields retyped from `u64` to `String`
  (`did`, SMS/MMS `contact`, CDR `destination`, `phone_number`, `number`,
  fax `stationid`/`from`/`destination`, `deleted_did`, `DIDAdded`, …): these
  are identifiers, not quantities, and can carry a `+`, formatting, or a short
  code that failed integer parsing. Caller-ID / forward override fields
  (`callerid_number`, `callerid_override`, `default_e911`, `sms_forward`) are
  likewise `String` but fold voip.ms's `-1` "not set" sentinel (and empty) to
  `None`.

### Added

- A `map` response shape kind in the codegen, emitting a bare
  `HashMap<String, V>` that defaults to empty (absent means empty, matching the
  list convention), for reference catalogs whose keys are data rather than
  schema.
- `deserialize_opt_string_sentinel_none`, a string deserializer that folds the
  `-1`/empty "not set" sentinel to `None`, for caller-ID override fields.

## [0.6.0] - 2026-07-09

### Changed

- **Breaking:** every list-valued response field is now a bare `Vec<T>` that
  defaults to empty, instead of `Option<Vec<T>>`. VoIP.ms signals an empty
  collection by omitting the field (or via an `is_empty` status that strips the
  subtree), so absent and empty always meant the same thing -- the `Option` only
  added a `None` no caller could act on differently from `Some(vec![])`. Callers
  drop the `.unwrap_or_default()` / `.unwrap()` / `.as_ref()` dance and use the
  `Vec` directly (`.is_empty()`, `.iter()`, indexing).

### Fixed

- **Breaking:** `getNAT`, `getPlayInstructions`, and `getJoinWhenEmptyTypes`
  return a list of `{value, description}` option objects, not a scalar. Their
  response fields are now `Vec<…>` of a generated element struct with
  `value: Option<String>` and `description: Option<String>`:
  - `GetNATResponse.nat`: was `Option<Nat>`, now `Vec<GetNATResponseNAT>`.
  - `GetPlayInstructionsResponse.play_instructions`: was
    `Option<PlayInstructions>`, now
    `Vec<GetPlayInstructionsResponsePlayInstruction>`.
  - `GetJoinWhenEmptyTypesResponseType.value` / `.description`: were
    `Option<bool>`, now `Option<String>`.
  The name-based `Nat` / `PlayInstructions` enum substitution wrongly overrode
  the list-typed reference-listing fields, and the extractor mis-inferred the
  `yes`/`Yes` sample cells as booleans; a live `value` of `Strict` or an array
  payload then failed to deserialize. The `Nat` and `PlayInstructions` enums are
  unchanged and still type the corresponding scalar setting fields elsewhere.

## [0.5.0] - 2026-07-07

### Changed

- **Breaking:** `date` on `getVoicemailMessages` is now `Option<chrono::NaiveDateTime>`
  instead of `Option<chrono::NaiveDate>`. VoIP.ms returns a full timestamp for
  this field, e.g. `2023-06-26 15:37:05`, which failed to deserialize as a
  bare date with "trailing input". The new type matches the identical `date`
  field on `getCDR` and `getResellerCDR`.

## [0.4.0] - 2026-07-07

### Changed

- **Breaking:** `callerid` on `getVoicemailMessages`, `getFAXMessages`,
  `getPhonebook`, and `getCallerIDFiltering` is now `Option<String>` instead of
  `Option<u64>`. VoIP.ms returns the caller's display form for inbound caller ID
  -- a name and number in angle brackets, e.g. `NAME <4164442828>` -- which
  failed to deserialize as an integer with "invalid digit found in string". The
  new type matches the identical `callerid` on `getVoicemailTranscriptions`,
  `getCDR`, and `getResellerCDR`. A purely numeric caller ID still round-trips as
  its string form.

## [0.3.2] - 2026-07-06

### Fixed

- Response list fields (`getVoicemailMessageFile`'s `message`,
  `getRecordingFile`'s `recordings`, `getConferenceRecordingFile`'s `recording`,
  and every other generated `Option<Vec<_>>` field) failed to deserialize with
  "invalid type: map, expected a sequence" when VoIP.ms returned a single-row
  result as a bare object instead of a one-element array -- which the live API
  does for the fetch-one file methods. A tolerant single-or-sequence
  deserializer now backs every list field: an array is taken as-is, a lone
  object (or scalar) becomes a one-element `Vec`, and null / absent /
  empty-string stays `None`.

## [0.3.1] - 2026-06-25

### Fixed

- Boolean flag parameters `answered`, `noanswer`, `busy`, and `failed`
  (`getCDR` / `getResellerCDR`), `activate` (`signupClient`), `portout`
  (`cancelDID`), and `advanced` (`getBalance`) serialized as bare
  `true`/`false`. They are documented as `1`/`0` flags but were left out of the
  `FLAG_01_FIELDS` override, so they missed the `1`/`0` param `serialize_with`.
  They now serialize as `1`/`0` like every other flag.

## [0.3.0] - 2026-06-25

### Changed

- **Breaking:** Removed the `rustls-tls-webpki-roots` feature. reqwest 0.13.4's
  `rustls` feature verifies against the OS trust store via
  `rustls-platform-verifier` and no longer exposes an embedded-Mozilla-roots
  toggle. `rustls-tls-native-roots` (default) and `native-tls` remain; an image
  with no OS trust store now needs `rustls-no-provider` plus a hand-built
  `ClientConfig`. The minimum reqwest is raised to 0.13.4 accordingly.
- Dropped the direct `url` dependency; the crate uses `reqwest::Url`, and
  reqwest re-exports it.

- **Breaking:** Boolean-flag parameters and fields are now typed `bool` instead
  of `i64` / `String` / `f64`. Many VoIP.ms parameters documented as
  `1 = true, 0 = false` (or `yes`/`no`) were under-typed by the WSDL; they now
  take a plain `bool`. The `1`/`0` or `yes`/`no` the API requires (a bare `bool`
  serializes `true`/`false`, which these reject) is produced by a param
  `serialize_with`, and tolerant deserialization accepts
  `1`/`0`/`yes`/`no`/`true`/`false` as string, number, or JSON bool. Affected
  fields are listed in `FLAG_01_FIELDS` / `FLAG_YES_NO_FIELDS` in
  `xtask/src/field_overrides.rs`. Validate-only flags whose `false` is
  equivalent to absent (the `test` param) are plain `bool` (default `false`,
  omitted from the request when `false`); the rest are `Option<bool>` so an
  explicit `Some(false)` ("turn it off") stays distinct from `None` ("leave
  unchanged"). Callers passing a string (e.g. `enable: Some("1".to_string())`)
  must migrate to `enable: Some(true)`, and `test` to `test: true`.
- **Breaking:** Queue/announcement duration fields documented as a number of
  seconds *or* a no-limit word are now typed `Seconds` / `WaitTime` (hand-written
  enums holding a `u64` count or an unbounded sentinel) instead of `String`:
  `retry_timer`, `wrapup_time`, `member_delay`, `announce_round_seconds`,
  `frequency_announcement`, `announce_position_frecuency` (`Seconds`, sentinel
  `none`) and `maximum_wait_time` (`WaitTime`, sentinel `unlimited`). Callers
  pass `Some(Seconds::Value(30))` / `Some(Seconds::Unlimited)`. Both are
  re-exported from the crate root and deserialize tolerantly (number, numeric
  string, or sentinel word).
- **Breaking:** `ApiStatus` is now an enum instead of a `String` newtype. It
  has one variant per documented VoIP.ms `status` code (~475, e.g.
  `ApiStatus::InvalidCredentials`, `ApiStatus::APINotEnabled`,
  `ApiStatus::NoDID`) for ergonomic `match` arms, plus an
  `ApiStatus::Unknown(String)` catch-all that preserves any undocumented code
  verbatim. `ApiStatus::description()` returns the documented human-readable
  meaning (`None` for `Unknown`), `as_str()` returns the verbatim wire string,
  `is_documented()` reports whether the code is a known variant, and
  `from_wire()` / `From<String>` / `From<&str>` parse the wire string. Code
  matching on the old `ApiStatus(String)` tuple must migrate to the variants
  (or match `ApiStatus::Unknown(s)` / call `as_str()`). The enum is generated
  by `cargo xtask gen` from the new committed `tools/api-statuses.json`,
  extracted from the docs' global error-code table via the new
  `cargo xtask extract-statuses` subcommand.
- **Breaking:** The typed list methods no longer error when the collection is
  empty. VoIP.ms returns a distinct `no_*` status per list method when there
  are no entries (`no_sms`, `no_cdr`, `no_messages`, …); the typed `Client`
  methods now fold any such status (`ApiStatus::is_empty()`) into a successful
  response with the collection field `None`, instead of `Err(Error::Api(...))`.
  Code that matched `Err(Error::Api(ApiStatus::NoSMS))` (or the other empty
  codes) on a typed call must instead handle an `Ok` whose collection field is
  `None`/empty. The `*_raw` methods are unchanged and still surface the empty
  status as `Error::Api`. Codes that look like `no_*` but signal a real failure
  (`no_base64file`, `no_callstatus`, `no_change_billingtype`, `no_provision`,
  `no_provision_update`, `no_sequences`) still error. The classification lives
  in the new `empty_statuses` array of `tools/api-response-overrides.json`.

### Added

- `ApiStatus::is_empty()`, reporting whether a status means "the requested
  collection is empty" rather than a failure. Generated from the
  `empty_statuses` array in `tools/api-response-overrides.json`.

- `field_type_skip` section in `tools/api-response-overrides.json` to suppress
  the global field-name override for one struct -- on both the `*Params` and
  `*Response` side -- where a flag/enum name is reused for an unrelated value
  (used for `getVoicemails`' `urgent` message count and `getFaxMessages`'
  free-text `folder` name).
- `field_type_override` section in `tools/api-response-overrides.json` to assign
  one struct's field a specific enum, overriding the global field-name table
  (used for the `type` field, which means different things per method).
- Generated `Client` methods and `*Params` structs now carry the official
  per-method description as a doc comment (mined from the docs into the new
  `method_docs` section of `tools/api-responses.json`; ~218 of 222 methods
  have one).
- Crate-level docs now cover IP allow-listing (and the `getIP` exemption) and
  the REST wire format.
- Re-exported `chrono`, `reqwest`, `rust_decimal`, `serde_json`, and `serde`
  from the crate root. Their types appear in the public API, so callers can now
  name those types (and `match` on `Error::Http`) without declaring a separate,
  independently-versioned dependency.
- The substituted enum types (`DtmfMode`, `MessageType`, …) and the
  hand-written `Routing` / `Seconds` / `WaitTime` now derive `Hash`, so they can
  be used as `HashMap` / `HashSet` keys. (`Copy` is not derived: the
  `Unknown(String)` catch-all holds a `String`.)

### Fixed

- `getFaxMessages`' `folder` parameter and response field were mistyped as the
  `VoicemailFolder` enum by the global `folder` override. A fax folder is a
  free-text name (`SENT` / `ALL` / user-created via `setFaxFolder`) outside that
  variant set; both are now `String`.

## [0.1.3] - 2026-05-25

### Changed

- Switched generated method naming so typed responses are now the default:
  unsuffixed methods return generated `*Response` structs, and raw JSON
  access moved to explicit `*_raw` methods.
- Renamed the low-level `Client` helpers to match: `Client::call_typed`
  → [`Client::call`], `Client::call_typed_at` → [`Client::call_at`],
  and the prior raw-JSON `Client::call` → [`Client::call_raw`].
  Generated method bodies now call `self.call(...)` for the typed
  wrapper and `self.call_raw(...)` for the `*_raw` wrapper.
- Generated PascalCase type names now preserve acronym casing
  (`GetDIDsInfoParams`, `SendSMSResponse`, `GetDTMFModesResponse`,
  `GetVPRIsResponse`, etc.) instead of title-casing acronyms
  (`GetDidsInfoParams`, `SendSmsResponse`, …). The acronym table that
  drives snake_case method names is now also used to render PascalCase
  type names, including nested element types whose wire field name is
  lowercase (`GetDIDsInfoResponseDID`, `GetSMSResponseSMS`,
  `GetSIPURIsResponseSIPURI`).
- Smarter English singularization for nested element type names:
  `-xes` / `-zes` / `-ches` / `-shes` words drop the full `es`
  (`faxes` → `fax`, so the prior `GetFAXMessagesResponseFaxe` is now
  `GetFAXMessagesResponseFAX`); `-sses` words drop just the trailing
  `es` (`addresses` → `address`); plain `-ses` words like `phrases`
  fall through to the simple `-s` strip (the prior `RecognizedPhras`
  is now `RecognizedPhrase`).
- Updated examples and docs to match the typed-by-default API.
- README link references now use absolute docs.rs URLs so they render
  on crates.io and GitHub instead of relying on rustdoc intra-doc
  resolution.
- Improved typed response deserialization robustness for string-like fields
  that VoIP.ms sometimes emits as numbers or booleans.

### Added

- New public type [`Routing`] (and `RoutingParseError`) modeling the
  tagged `kind:value` strings VoIP.ms uses for call-routing fields
  (`account:`, `fwd:`, `vm:`, `sip:`, `sys:`, `grp:`, `queue:`, `ivr:`,
  `cb:`, `tc:`, `disa:`, `did:`, `phone:`, `none:`). Generated
  `*Params` and `*Response` structs now type 12 routing-related fields
  (`routing`, `failover_*`, `fail_over_routing_*`) as
  `Option<Routing>` instead of `Option<String>`. Unknown tags
  round-trip via a `Routing::Unknown { tag, value }` catch-all so
  forward compatibility is preserved.
- New public enums for documented VoIP.ms scalars: `DtmfMode`, `Nat`,
  `EmailAttachmentFormat`, `TranscriptionFormat`, `PlayInstructions`,
  `RingStrategy`, `RingGroupOrder`, `VoicemailFolder`, `QueueEmptyBehavior`
  (`join_when_empty` / `leave_when_empty`), `EstimatedHoldTimeAnnounce`,
  `CallPickupBehavior`, `RecordingSort`, `SearchType`, `VanityType`,
  `MessageType` (SMS/MMS direction), `DialingMode`, `TollFreeCarrier`, and
  `DidBillingType` (integer-coded). Each carries an `Unknown(String)`
  variant for values not in the documented set, so VoIP.ms adding new
  options never breaks deserialization. `QueueEmptyBehavior` and
  `EstimatedHoldTimeAnnounce` also correct a latent bug: the queue response
  fields were inferred as `bool` from a `yes`/`no` sample and would have
  dropped the third value (`strict` / `once`).
- The `type` field is now typed per struct: a search mode (`SearchType`) in
  the DID/toll-free search params, a vanity prefix (`VanityType`) in
  `searchVanity`, and a message direction (`MessageType`, wire `1`/`0`) in
  the SMS/MMS params and responses. Reference-data `type` lookups whose
  valid set comes from another endpoint stay `String`.
- Generated enum deserializers now accept the wire value as a JSON string,
  number, or bool (VoIP.ms returns the SMS `type` as a bare number), not
  only a string.
- Codegen overrides schema extended with top-level `enums` and
  `field_types` sections in `tools/api-response-overrides.json`. New
  enums can be added declaratively (name, variants, wire strings) and
  mapped to one or more field names without touching the generator
  source.
- Dry-run support for runnable examples:
  `VOIP_MS_DRY_RUN=true` for `get_balance`, `list_dids`, and `send_sms`;
  `LIVE_VERIFY_DRY_RUN=true` for `live_api_verify` smoke/extended flows.

## [0.1.2] - 2026-05-25

### Changed

- Replaced the small set of hand-written starter response types
  (`StatusResponse`, `GetBalanceResponse`, `GetDidsInfoResponse`) with a
  full generated `*Response` struct per method (222 in total). Shapes are
  inferred from the official VoIP.ms HTML docs by a new
  `cargo xtask extract-responses` extractor, with hand-edited corrections
  in `tools/api-response-overrides.json`. All response fields are
  `Option<T>` and tolerate string-or-number, `0/1`, `Y/N`, and
  date/datetime placeholder forms via custom deserializers in
  `src/responses.rs`.
- `src/responses.rs` no longer re-exports any types; it now contains only
  the shared `deserialize_opt_*` helpers used by `src/generated.rs`.
- Examples updated to consume the generated `*Response` structs.

### Added

- `cargo xtask extract-responses <html>` for refreshing
  `tools/api-responses.json` from a saved copy of `apidocs.php`.
- `tools/api-response-overrides.json` schema (see `xtask/src/overrides.rs`)
  supporting per-path scalar retypes and full shape replacement.
- `DEVELOPMENT.md` documents the HTML-refresh workflow.

## [0.1.1] - 2026-05-22

### Changed

- Added and documented typed response ergonomics more clearly across user and
  maintainer docs: raw `serde_json::Value` methods, generated `*_typed`
  methods, `call_typed` / `call_typed_at`, and starter partial typed response
  structs.
- Moved contributor and maintainer workflows out of `README.md` into a new
  `DEVELOPMENT.md` guide (regeneration, testing strategy, CI/CD behavior,
  and release process), keeping README focused on crate usage.

## [0.1.0] - 2026-05-22

### Changed

- Bumped `reqwest` to `0.13` and adjusted feature flag mappings to its
  reorganized TLS surface. User-facing feature names
  (`rustls-tls-native-roots`, `rustls-tls-webpki-roots`, `native-tls`)
  are unchanged.
- Dependabot auto-merge now skips `0.x → 0.y` Cargo updates, which are
  classified as `semver-minor` by Dependabot but are breaking under
  Cargo's SemVer interpretation. Those land as reviewed PRs.

### Added

- Initial release skeleton: async `Client` over `reqwest`, typed
  `*Params` request structs and `Client` methods for all 222 VoIP.ms
  REST operations, generated from `tools/server.wsdl` by the
  `xtask` workspace member (`cargo xtask gen`).
- `Client::call` for invoking methods not yet covered by the
  generator and for typed deserialization via `serde_json::from_value`.
- `Error::Http` / `Error::Api(ApiStatus)` / `Error::InvalidResponse`
  error surface.
- TLS feature flags: `rustls-tls-native-roots` (default),
  `rustls-tls-webpki-roots`, `native-tls`.
- Examples: `get_balance`, `send_sms`, `list_dids` (run with
  `VOIP_MS_USERNAME` / `VOIP_MS_PASSWORD` set).
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
  with coverage instrumentation and Dependabot auto-merge for
  patch/minor cargo updates.

[0.13.0]: https://github.com/ecliptical/voip-ms/compare/v0.12.2...HEAD
[0.12.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.12.2
[0.12.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.12.1
[0.12.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.12.0
[0.11.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.11.0
[0.10.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.10.2
[0.10.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.10.1
[0.10.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.10.0
[0.9.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.9.0
[0.8.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.8.0
[0.7.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.7.0
[0.6.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.6.0
[0.5.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.5.0
[0.4.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.4.0
[0.3.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.3.2
[0.3.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.3.1
[0.3.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.3.0
[0.1.3]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.3
[0.1.2]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.2
[0.1.1]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.1
[0.1.0]: https://github.com/ecliptical/voip-ms/releases/tag/v0.1.0
