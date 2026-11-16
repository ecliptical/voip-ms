---
paths:
  - "src/client.rs"
  - "src/types.rs"
  - "xtask/src/main.rs"
  - "xtask/src/response_codegen.rs"
  - "livetest/src/areas/cdr.rs"
  - "livetest/src/areas/voicemail.rs"
  - "livetest/src/harness/probe.rs"
---

# Response timestamps

### 8. Record-listing timestamps are qualified by undoing the server's shift

**Decision**: The six methods that take a `timezone` number (decision #5a's
`OFFSET_OPS`) type their response timestamp as `Reported<WallClock>`. The typed
method sends the number `TimezoneOffset::for_query` picks, then, before
deserializing, moves each reported wall clock back by `timezone + 5` hours and
resolves it in [`crate::SERVER_ZONE`], via `Client::call_zoned` and the public
`attach_offset`. A value comes back `WallClock::Zoned` with the offset the
server zone was at then (`-04:00` or `-05:00`).

**Rationale**: VoIP.ms reports these timestamps as server-zone wall clocks
shifted by a fixed `timezone + 5` hours (decision #5a), so the number sent is
not the offset they are in. 0.13 attached the number sent as the offset, which
is an hour wrong for every row recorded during DST: the call at 16:28:34 UTC
read back at `timezone=0` as `17:28:34+00:00`. One downstream consumer had
already read an unqualified `2026-09-18T14:44:11` as UTC and reported a
registration time that had passed, which is what the typing exists to prevent.

The shift is fixed arithmetic, so it can be undone exactly for any number sent,
including a fractional one (`5.5` reports `22:58:34` for that call). What
remains is the server-zone wall clock, and resolving it per row gives each row
the offset in force at its own instant, so a range that crosses a DST change
comes back right on both sides. The one wall clock that cannot be resolved is
the hour the server zone repeats when clocks fall back. `attach_offset` writes
that value as its moved-back server-zone wall clock followed by the zone's name
(`2026-11-01 01:30:00 America/Toronto`), and it reads as `WallClock::Bare` --
which is why the type is `WallClock` rather than `DateTime<FixedOffset>`. It is
written on the server clock rather than left shifted so that every row's
`.local()` is on the same clock; left shifted, it would sort five hours away
from its neighbors at `timezone=0`.

The zone name is also what keeps the guard. `deserialize_opt_record_listing_timestamp`
reads a value with an offset as `Zoned` and one ending in the server zone's
name as `Bare`. It refuses a `YYYY-MM-DD HH:MM:SS` wall clock with neither:
that is a shifted wall clock that was never qualified, which a raw caller who
skipped `attach_offset` would otherwise read as if it meant something. Any other
text is an odd value that `attach_offset` left as it arrived, and it degrades to
`Reported::Unreadable`, so one malformed date costs its own row and not the
envelope. The guard cannot catch `attach_offset` given a number other than the
one sent.

Qualifying is a step on the JSON, not a `Deserialize` impl: serde has no access
to the request, so it cannot know the number sent. `attach_offset` is public
because a `call_raw` caller needs the same step (`livetest`'s `probe_zoned` is
one), and it takes the `TimezoneOffset` sent rather than an offset to attach.
Each method's paths reach a caller through `offset_timestamps(method)`, emitted
by the same codegen pass that retypes the fields -- a raw caller reading them
out of a generated method body would be copying something that moves with the
response shape. It is the one public route: the per-method `*_TIMESTAMPS`
consts the typed methods pass to `call_zoned` are private, so there is no
second spelling of the same paths to keep in step. It is a lookup and not a
step inside `call_raw`: the raw calls return exactly what VoIP.ms sent, and a
qualified envelope would no longer be that.

`attach_offset` leaves a blank value alone. A blank is one record's missing
timestamp, which the deserializers fold to `None`; rewriting it would produce a
string that parses as nothing, and since one unparseable value fails the whole
envelope, that would turn a single missing timestamp into the loss of every
record beside it.

**The live check is instant equality across numbers.** The `cdr` area's
`fixture:getCDR:<zone>` reads one window at UTC and at a named zone (a
whole-hour and a half-hour one) through `Client::get_cdr` and fails if a
record's instant moves. It passed on 2026-09-23. It cannot catch a shift that is
wrong by the same amount for every number -- 0.13's error was exactly that --
so the absolute reference is the recorded call: typed `get_cdr` reports it as
`12:28:34-04:00`.

The other `NaiveDateTime` response fields are typed through decision #8a
where they were measured. Six are recorded in the server zone and typed as
`WallClock`, and three stay `NaiveDateTime` because nothing independent of the
API could date them.

**How to apply**: `cargo xtask gen` derives the fields from the response shapes:
every `datetime` scalar under an `OFFSET_OPS` method is retyped and its path
emitted as that method's `*_TIMESTAMPS` const and `offset_timestamps` arm. Three things fail the run rather
than degrade quietly, because each would leave a field silently naive while the
build stayed green:

* an offset op whose response declares no timestamp at all (a docs refresh that
  dropped the field);
* a `datetime` the walk cannot name, which is any of: a bare one as a list
  element or map value, since a path ends at a field name; one inside a map,
  whose values `attach_offset`'s `*` does not reach, because over an object `*`
  already means the bare single record VoIP.ms sends for a one-element list; and
  one under a collection nested directly inside another, where the path
  `attach_offset` would walk and the struct the emitter wraps it in stop
  agreeing. Each needs the path form, the emitter, or both extended first, which
  is a decision rather than a default. The guards ask whether the shape holds a
  timestamp before refusing, so a nested list of strings is not an error;
* a missing response shape for an op that sends an offset.

Note the asymmetry the second case fixes: the "no timestamp at all" check only
fires when *every* timestamp is missed, so a response that grows a second
timestamp somewhere unaddressable would otherwise pass.

A response that is not a record is not in that set. `emit_struct` promotes one
into a one-field record (`value`, `items`, `entries`), `timestamp_fields`
synthesizes the same field, and that field goes through the override table like
any other.

All three have to agree on the *name*, not just on the shape, and twice now they
have not: a promoted `value` the emitter typed from the raw shape because its
arm never consulted the resolver, and an `items` element the walk singularized
to `Item` while the emitter called it `Items`. Both emitted a bare
`NaiveDateTime` under a path `call_zoned` rewrites, so every typed call on that
method failed at runtime on a generator run that reported success -- quieter, and
so worse, than the refusal each replaced. Whatever names an element struct must
be one function called from both sides (`element_type_name`), and the tests
assert the emitted type rather than the walk's return value, because the walk
agreeing with itself is exactly what both failures looked like.

Naming the element through `element_type_name` costs one shape: a response that
is a list of lists. `emit_struct` hands the whole list to `field_type`, so the
inner list becomes an element struct of its own (`{ items: Vec<T> }`) where the
wire has a bare array, and serde rejects it. That shape is unsupported by
construction now rather than by accident, which is the trade that makes the root
case agree with the field case. Nothing returns one -- `src/generated.rs` has no
`pub items:` field at all, so no response is a top-level list -- and a docs
refresh that produced one would need both the emitter and the walk taught about
it together.

### 8a. Timestamps in a named zone are qualified in that zone

**Decision**: A response timestamp that is a wall clock in a known named zone
is `Option<Reported<WallClock>>`, where [`crate::WallClock`] is
`Zoned(DateTime<FixedOffset>)` or `Bare(NaiveDateTime)`. The `ZONE_OPS` table in
`xtask/src/main.rs` names each method and where its zone comes from
(`ZoneSource`):

* **`Server`** -- [`crate::SERVER_ZONE`]. The plain typed method qualifies
  each value itself through `Client::call_in_zone`: `getRegistrationStatus`
  (`register_next`), `getCallRecordings` / `getCallRecording` (`datetime`),
  `getDIDsInfo` (`order_date`), `getFaxMessages` (`date`) and `getMediaMMS`
  (`date`).
* **`Supplied`** -- a zone the caller holds. `getVoicemailMessages`' `date` is
  in the mailbox's own `timezone`. The plain `get_voicemail_messages` leaves
  every value bare, and `get_voicemail_messages_in_zone(params, tz)` qualifies
  them.

A raw caller does the same with the public `attach_zone` over the paths
`zone_timestamps(method)` answers, which also names the source
([`crate::ZoneTimestamps`], [`crate::TimestampZone`]). Both are emitted from
`ZONE_OPS`, the same way `offset_timestamps` is emitted from `OFFSET_OPS`.

**Rationale (server zone)**: each `Server` method was measured on 2026-09-23,
during DST, against an instant taken independently of the API, and each
reported the Eastern wall clock at UTC-04:00:

| Method | Reference | Reported |
|---|---|---|
| `getRegistrationStatus` | baresip registered at 14:40:30 UTC with a 600 s interval | `register_next` `10:50:30` |
| `getCallRecordings`, `getCallRecording` | call at 16:28:34 UTC; the recording id embeds Unix time `1790180914` | `datetime` `12:28:34` |
| `getDIDsInfo` | DID ordered between 17:19:00 and 17:19:32 UTC | `order_date` `13:19:31` |
| `getFaxMessages` | fax sent between 17:24:15 and 17:24:33 UTC | `date` `13:24:33` |
| `getMediaMMS` | MMS sent between 17:20:47 and 17:21:40 UTC | `date` `13:21:39` |

A fixed UTC-5 would have read an hour earlier in every row. The probe DID and
fax number were canceled afterwards, and the DID the call went to had
`record_calls` switched on for the call and back off.

Three fields stay `NaiveDateTime`, unmeasured, because no reference could be
created without an effect the API cannot undo:

* `GetBackOrdersResponseBackOrder::order_date` -- a back order cannot be
  canceled through the API and may later buy a number.
* `GetLNPDetailsResponse::date` -- a port request is a real filing with the
  losing carrier.
* `GetConferenceRecordingsResponseRecording::date` -- no API call turns on
  conference recording. A DID routed with the undocumented `cfb:`
  ([`crate::Routing::Conference`]) dials into a conference, so with recording
  turned on in the portal a call to it could be the reference.

Every measured table agreed, so these are likely `SERVER_ZONE` as well; adding
one to `ZONE_OPS` wants a measurement, not the pattern.

**Rationale (mailbox zone)**: Measured against the live API on 2026-09-22 and
2026-09-23 with the test account and `examples/call_raw.rs`:

* The response names no zone. Every message carries exactly `callerid`, `date`,
  `duration`, `folder`, `listened`, `mailbox`, `message_num` and `urgent`, and
  the request takes only `mailbox`, `folder`, `date_from` and `date_to`, so it
  cannot choose one either.
* `date` is the stored instant, rendered in the mailbox's *current* `timezone`
  setting at the time it is read. Mailbox 101's message 17 read `12:47:40`
  while the mailbox was set to `Pacific/Honolulu` and `18:47:40` after it went
  back to `America/Toronto`. Message 14 read `2026-08-24 21:25:11` under
  Toronto and `2026-08-25 03:25:11` under `Europe/Berlin`. So a caller holding
  the mailbox's current zone resolves every row correctly, including rows
  recorded under an earlier setting.
* `date_from` / `date_to` are **not** matched in the mailbox's zone. Under
  Toronto, message 14 (01:25 UTC on 2026-08-25) matched `2026-08-24` and not
  `2026-08-25`, so the window is not UTC. Under Berlin the same message read
  `2026-08-25 03:25:11` and still matched `2026-08-24` only. So the window is
  Eastern days. No message on the account fell between 04:00 and 05:00 UTC
  during DST, the one hour that separates `SERVER_ZONE` days from fixed UTC-5
  ones, so which of the two it is was not measured. The finding is written on
  both params through `ZoneOp::param_notes`, and the run fails if an entry
  names a param the WSDL does not declare.

Each row is resolved at its own instant, so rows on either side of a DST change
get different offsets. A wall clock the zone repeats when clocks fall back is
ambiguous, and one it skips when they spring forward should not occur. Both read
as `Bare`, because choosing a side would be a guess, and so does one at an
offset with a seconds part (a zone's pre-standard local mean time), which the
wire spelling cannot carry since it stops at minutes. A value that already names
an offset (`names_offset`), a blank, and text that is not a
`YYYY-MM-DD HH:MM:SS` wall clock are left as they arrived.

`attach_zone` and `attach_offset` share one walk. Each only resolves a value --
`attach_offset` after moving it back by the shift -- and the walk rewrites it in
place, trimmed: as the instant with its offset, or, for a wall clock with no
single offset, as that wall clock followed by the zone's name
(`2026-11-01 01:30:00 America/Toronto`). Both helpers write that one form, so a
server-zone field and a record-listing field agree on the unresolvable hour.
Both kinds of field resolve the value in the named zone again, so it reads as
`Bare` -- and a zone name that does fix the instant (`... 01:30:00 UTC`) reads
as `Zoned` rather than being dropped. A record-listing field accepts only the
server zone's name. A named-zone field also reads the plain spelling, which is
what an unqualified call leaves.

`WallClock`'s equality is hand-written. `DateTime<FixedOffset>`'s own `==`
compares instants and ignores the offset, so a derived one would call
`18:47:40-04:00` and `22:47:40+00:00` equal while they report different wall
clocks, and a test could not catch `attach_zone` choosing the wrong offset.

A named-zone field accepts a bare value rather than refusing it, unlike a
record-listing field. A bare value there comes from a call that supplied no zone
or from a row left unresolved, and is the wall clock in the zone it was
rendered in, so it is part of the field's contract rather than a break in it.
`Bare` claims no zone where reading it as UTC would invent one. Unreadable text
still degrades to `Reported::Unreadable`.

The crate still makes one request per method call. Looking the mailbox zone up
with `getVoicemails` inside `get_voicemail_messages_in_zone` would double the
requests, and a failed `getVoicemails` would then fail a message read. The
caller can fetch the mailbox once for many reads.

**How to apply**: A new method whose timestamps are wall clocks in a named zone
goes in `ZONE_OPS` with its `ZoneSource`: `Server` only once an independent
reference instant has been compared against it during DST, `Supplied` with a
doc fragment naming where the caller reads the zone. `cargo xtask gen` fails
when a zone op is also an offset op, when it takes no parameters (the template
for those qualifies nothing), when its response has no timestamp, or when a
`param_notes` entry names an undeclared param. livetest's `probe` qualifies a
`Server` method on its own, and the `voicemail` area probes
`getVoicemailMessages` against the mailboxes `getVoicemails` reports, qualified
in each mailbox's zone, until one returns a message, so the key diff has rows to
compare or the method is skipped.
