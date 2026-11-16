---
paths:
  - "src/client.rs"
  - "src/form.rs"
  - "src/error.rs"
  - "tests/client.rs"
  - "tools/api-statuses.json"
---

# Client transport and errors

### 7. GET, except a method carrying a file

**Decision**: A call is a GET with query parameters, unless its parameters
include a base64-encoded file, in which case it is a `multipart/form-data`
POST carrying every parameter -- credentials and method name included -- as a
form field. Four methods qualify: `setRecording`, `sendFaxMessage`, `sendMMS`
(`media2`), and `addLNPFile`. `Client` decides per method from the generated
surface; a caller does not choose, and no parameter or builder option
overrides it. The one exception is `Client::call_multipart_raw` (and its
`_unchecked` twin), for an upload method this crate has not been regenerated
for and so cannot recognize.

**Rationale**: VoIP.ms documents and accepts both, but every documented example
is GET, and GET keeps the request observable in logs and proxies during
development. That observability is worth keeping for the 218 methods that can
have it, which is why the transport is targeted rather than switched wholesale.

A file parameter cannot have it. VoIP.ms's front end caps the request line at
8190 bytes (Apache's default `LimitRequestLine`): measured against
`rest.php`, a request line of about 8182 bytes answers 200 and one of about
8187 answers 414. After the credentials and the method name that leaves roughly
8 kB for the percent-encoded parameters, about a third of a second of 8 kHz
mono 16-bit audio -- a 2.8 second greeting is 60,428 base64 characters, more
than seven times over. `addLNPFile` is documented "Only accepted through POST
request", so for it no size works over GET. The risk this decision originally
weighed, many small parameters on `createSubAccount` / `setSubAccount` /
`setQueue`, really is mitigated by most of them being `None` (decision #3); one
large parameter is not, and that is the case the original reasoning did not
cover.

The POST has to be `multipart/form-data`. `rest.php` hands an
`application/x-www-form-urlencoded` POST to a SOAP handler, which answers with
an XML fault -- the trap that makes the API look GET-only on a first test.
Multipart is accepted at 100 kB and at 8 MB, and a read-only `getBalance` over
multipart returns `success`, so the API does not restrict the transport to
upload methods; this crate restricts it to the methods that need it.

Which methods those are is derived from the parameters, not from a second
document. `tools/server.wsdl` (decision #1) does not record `addLNPFile`'s
POST-only requirement -- that lives only in the HTML docs -- but every method
that needs POST has a base64 file parameter and every method with one needs
POST, so the parameter carries the whole rule. The four paths are listed in
`BASE64_FILE_PARAM_PATHS` (`xtask/src/field_overrides.rs`), from which the
generator emits `requires_multipart`. Every generated method calls `call` /
`call_raw`, and those (with `call_raw_unchecked` and the private `call_zoned`)
read the transport from `requires_multipart` by wire name, so the choice lives
in one place and a caller dispatching by name gets the same answer
as a generated method. `call_multipart_raw` is public for the same reason
`call_raw` is: a method this crate hasn't been regenerated for still needs a way
to be called, and for an upload method that way has to be a POST.

**How to apply**: When a new method takes a base64 file parameter, add its
`"wireMethod.field"` path to `BASE64_FILE_PARAM_PATHS` and regenerate.
`cargo xtask gen` has three outcomes for that table, all covered by unit tests
in `xtask/src/main.rs`:

* it **fails** on an entry naming a parameter the WSDL does not declare, since
  a path left behind by a docs revision would drop the method back onto a GET
  without a word;
* it **fails** on an entry naming an op that is also in `OFFSET_OPS`. An offset
  op sends its `*ParamsWire` twin, which has only ever gone out over GET; listing
  it would move that twin onto a multipart body without a word. Nothing overlaps
  today, and reconciling the wire twin with a multipart body is unexamined work,
  so the generator refuses rather than guesses;
* it **warns** when a parameter the docs describe as base64 is absent from the
  table. That is the tripwire for a fifth method appearing in a docs refresh;
  it warns rather than fails because the reading comes from mined HTML and
  needs a human to confirm the parameter really carries a file.

### Calling the wire API

The private `Client::send` is the single point that hits the network; it takes
the transport (decision #7) and returns the parsed envelope, and `Client::fetch`
adds the status classification on top. The public `call` and `call_raw` (and
`call_raw_unchecked`, behind the `unchecked-raw` feature) take the transport
from `Transport::for_method`, which reads the generated
`requires_multipart(method)`; the private `raw` helper holds the body
`call_raw` and `call_multipart_raw` share. `call_multipart_raw` and `call_multipart_raw_unchecked`
are the single explicit escape hatch, for an upload method this crate has not
been regenerated for, which `requires_multipart` answers `false` for. A caller
dispatching by wire name calls `call_raw` like a generated method does;
`requires_multipart(method)` answers the transport question without making the
call, `offset_timestamps(method)` names the timestamps the caller then
completes with `attach_offset` (decision #8), and `zone_timestamps(method)` the
ones it qualifies with `attach_zone` (decision #8a). Every generated method is a thin
wrapper over `call` or `call_raw`, whatever its transport:

```rust
pub async fn get_balance(&self, params: &GetBalanceParams) -> Result<GetBalanceResponse> {
  self.call("getBalance", params).await
}

pub async fn set_recording(&self, params: &SetRecordingParams) -> Result<SetRecordingResponse> {
  self.call("setRecording", params).await
}
```

A request's total time limit is the `reqwest::Client`'s, unless the `Client`
came from `with_timeout`, whose limit `send` sets on each request with
`RequestBuilder::timeout`. Running out of time is then an `Error::Http` that
`Error::transport` classifies; a timer around the call drops the request and
leaves no `Error`. reqwest replaces the client's total limit with the
request's rather than taking the smaller; a test in `tests/client.rs`
covers that.

Both transports render their fields through one serializer (`src/form.rs`), so
they differ in where a value rides and never in how it is written. That
serializer matches `serde_urlencoded`, which is what `reqwest`'s `.query()`
applies to a parameter set, and its tests assert the agreement value by value --
including the one arm where `Display` would diverge, a whole float, which the
query string renders `1.0` and `to_string` renders `1`. The one value where they
part is a `serde_json::Number` under serde_json's `arbitrary_precision` feature,
which serializes as a private struct holding the number's text.
`serde_urlencoded` refuses it like any struct; `form.rs` sends the text as it
arrives, because any crate in the build can turn the feature on, and the GET's
query string is built from `form.rs`'s fields, so `serde_urlencoded` never sees
the struct. CI runs the whole suite with the feature on. The multipart form was
originally built by reading back the query string the GET form produced, which
bought the same parity at the cost of encoding and decoding the whole payload:
roughly 30 MB of transient allocation for an 8 MB fax.

A parameter with no field rendering -- a nested value, or anything serde reports
as other than a scalar -- is `Error::InvalidParams(ParamsError::Unencodable)`
naming the parameter, and nothing is sent. Neither transport can carry one, so
this is not a property of the transport that happened to be chosen.

**No wire form rests on a `Display` impl that could change.** A wire form is a
contract with VoIP.ms; `Display` is free to render for a person, and in this
crate it does -- `Error` prints `API status: did_in_use (DID Number is already
in use)` where `ApiStatus` prints `did_in_use`, and `TransportFailure` has no
`Display` at all for the same reason (see "Transport-failure classification").
The risk is a wire form that is only *incidentally* a `Display` impl, so that
making the rendering friendlier for a log silently changes what is sent.

That rules out borrowing a *type's* rendering, not `to_string` as such. In
`form.rs` the integer arms use it, since a primitive integer's `Display` is its
decimal digits by specification and cannot drift; the float arms do not, since a
float's is fixed to something else (`1` for `1.0`, never an exponent) and so go
through `ryu`. `Routing` states its wire form as `Routing::to_wire`, which
`Serialize` calls and `FromStr` inverts, leaving its `Display` free to change --
the two render the same text today and only the first is a contract.
`TimezoneOffset` still serializes through `collect_str` over `rust_decimal`'s
`Display`, whose scale is load-bearing (`Asia/Kolkata` sends `5.50`).

**How to apply**: give a domain type that goes on the wire an explicit wire form
(`to_wire` / `as_wire`) and have `Serialize` call that. If a type's only route
to text is its `Display`, treat that as the signal to add one rather than to
borrow the rendering meant for a reader. `Display` stays in error messages, the
`Debug` impls, and log lines.

If a regeneration drift is ever needed (e.g. a method needs custom
encoding), break that one method out of the codegen with an explicit
skip-list and hand-write it in `src/client.rs`. Do not pollute
`generated.rs` with special cases.

### Error surfacing

Four variants, no more:

* `Error::Http` -- wraps `reqwest::Error`. Includes both transport-level
  failures and `error_for_status`'s non-2xx surfacing.
* `Error::Api(ApiStatus)` -- the response parsed as `{ "status": "..." }`
  with something other than a success status (`ApiStatus::is_success`) or an
  empty-collection code. `ApiStatus` is a generated enum
  with one PascalCase variant per documented code (~475 of them) plus an
  `Unknown(String)` catch-all, so a code VoIP.ms returns but hasn't
  documented is preserved verbatim rather than lost -- the variant set is
  documentation, not a closed contract. `ApiStatus::from_wire` /
  `as_wire` round-trip the wire string, `description()` returns the
  documented meaning (`None` for `Unknown`), and `is_documented()`
  reports whether it's a known variant. The enum, its impls, and the
  description table are emitted by `cargo xtask gen` from
  `tools/api-statuses.json`, which is extracted from the docs' global
  "Error Codes" table via `cargo xtask extract-statuses <html>`, followed
  by the codes the live API returns but that table omits
  (`undocumented_statuses` in `tools/api-response-overrides.json`). Because
  the docs ship a couple of codes capitalized (`Invalid_threshold`), the
  variant's `as_wire` preserves the wire casing while the variant
  *identifier* normalizes through the same acronym-aware PascalCase as
  method/type names (`no_did` → `NoDID`). `Display` on the *error* renders the
  code with its documented meaning (`API status: did_in_use (DID Number is
  already in use)`) so a log line says what went wrong; `Display` on
  `ApiStatus` itself stays the bare wire string, matching `as_wire`.

  `ApiStatus::Success` and `ApiStatus::SuccessTest` are the variants the table
  does not supply. They are synthesized by the generator, because a typed
  response's `status` field (decision 2) reports them and would otherwise land
  in `Unknown`. `success_test` is voip.ms's answer to a call with `test=1`:
  checked, nothing saved. Measured live, `test=true` is ignored and the call
  takes effect, which is why `test` is serialized `1`/`0`. `load_statuses`
  fails the run if a docs refresh starts listing either code, so no variant can
  be emitted twice.

  **Empty-collection statuses are not errors for typed calls.** VoIP.ms
  returns a distinct `no_*` status per list method when the list is empty
  (`no_sms`, `no_cdr`, `no_messages`, …). The typed `Client::call` (and so
  every unsuffixed generated method) folds any status for which
  `ApiStatus::is_empty_collection()` is true into a successful data-less response
  -- collection fields deserialize to `None` -- instead of `Error::Api`. The
  `*_raw` methods (and `call_raw`) deliberately keep the strict verbatim
  contract: they still surface an empty status as `Error::Api`, so the raw
  escape hatch reflects exactly what VoIP.ms returned. `check_status` in
  `src/client.rs` classifies the status; the two paths diverge in
  `call_raw` vs `call`. The classification is hand-curated in the
  `empty_statuses` array of `tools/api-response-overrides.json` and emitted
  into `ApiStatus::is_empty_collection()` by `cargo xtask gen`; codes that look like
  `no_*` but signal a real failure (`no_base64file`, `no_callstatus`,
  `no_change_billingtype`, `no_provision`, `no_provision_update`,
  `no_sequences`) are deliberately excluded. To reclassify, edit that array
  and regenerate -- an entry naming a status code absent from both
  `tools/api-statuses.json` and `undocumented_statuses` fails the codegen.
  **A single nested error code is the status.** The port-request methods
  answer every failure as `status: invalid` with the code that says which
  under `error` (`{"status": "invalid", "error": [{"code": "no_ports", ...}]}`),
  so `invalid` alone would hide both an empty list and a bad port id.
  `reported_status` in `src/client.rs` reads that one code as the status, for
  the raw and typed paths alike, and the typed path writes it back into the
  envelope's `status` so the response reports it. `invalid` with no code, or
  with several, stays `ApiStatus::Unknown("invalid")`. `no_ports` is in
  `undocumented_statuses` and `empty_statuses`, so an account that has never
  ported a number gets an empty `getLNPList` rather than an error.
* `Error::InvalidResponse(String)` -- the response was 2xx and JSON but
  didn't contain a `status` field. Should be rare; if it happens
  systematically for a method, that's a VoIP.ms-side break.
* `Error::InvalidParams(ParamsError)` -- the parameters could not be
  converted to their wire form, so no request was sent. `ParamsError` names
  which check failed: `Timezone(TimezoneOffsetError)` (see 5a's record-listing
  offsets), `InvalidDate { param, value }` (a record-listing `from` / `to`
  that is not a date, checked whether or not a zone is set), and
  `Unencodable(String)`, a parameter with no wire-field rendering (decision
  #7). The inner enum exists so the next parameter
  validation is additive: the variant name is general and a specific payload
  would have forced either a second `Error` variant or a breaking change. The
  timezone hop carries `#[from]`, so a generated `TryFrom<&*Params>` returning a
  `TimezoneOffsetError` still reaches `Error` through one `?`.

### Transport-failure classification

**Decision**: `Error::transport()` reduces a failure to a `TransportFailure`
(`Rejected(StatusCode)` / `Timeout` / `Dns` / `Connect` / `Body` / `Other`),
and `TransportFailure` answers two questions about it:
`never_reached_upstream()` (could account state have changed) and
`retry_outlook()` (is repeating the identical call worth anything, as a
`RetryOutlook`). Neither type implements `Display`.

**Rationale**: This is knowledge about HTTP and about this API, not about any
consumer's product, and every consumer of the crate was re-deriving it from
`Error::Http`'s inner `reqwest::Error` -- one by hand-copying another's
implementation, which drifted within a review cycle with nothing to catch it.
Two distinctions only this crate knows are the ones consumers got wrong: an
allow-list rejection is `Api(ApiStatus::IPNotEnabled)` on a 200 rather than an
HTTP 403, so it is not a transport failure at all; and reqwest reports a
resolution failure through the connect error that wraps it, so `is_dns` must be
read before `is_connect`. The two questions stay separate because a refusal
answers them differently -- a stale proxy credential answering 401 changed no
state *and* is futile to repeat, and collapsing them produced a retry loop. That
split is also why 408 is carved out of `never_reached_upstream`: RFC 9110 defines
it for an incomplete request, but an intermediary returning it for a slow
response is a 504 in 408's clothing, so the state claim is unprovable while
`AfterWaiting` still reads right.

Rendering is deliberately excluded: a model reading a retry decision and a
person reading a terminal diagnostic want different sentences, so a `Display`
here would pull that judgment into the wrong crate. This sits alongside
decision #6 -- the crate classifies, it still does not retry.

**Both enums are exhaustive, on purpose.** Neither `TransportFailure` nor
`RetryOutlook` carries `#[non_exhaustive]`, so a consumer can `match` on either
with no wildcard arm. A consumer that does gets a compile error when a variant
is added and has to decide what the new variant means in its own wording. With
`#[non_exhaustive]` every consumer would need a wildcard arm, and a new variant
would take that arm's fallback wording without anyone deciding it. The cost is
that adding a variant to either enum is a breaking change and ships in a
breaking release. Do not add `#[non_exhaustive]` to them. `ParamsError` is the
opposite case: it is `#[non_exhaustive]` so that a new parameter check is
additive.

**How to apply**: A new `TransportFailure` variant must be added to the arms of
both `never_reached_upstream` and `retry_outlook` (neither uses a wildcard) and
to the per-variant tests in `src/error.rs`, so it decides rather than inherits.
Classification order is part of the contract; the tests pin it, including that
a DNS failure does not read as `Connect`.
