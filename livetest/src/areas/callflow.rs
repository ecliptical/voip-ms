//! The `callflow` area: the call-routing building blocks -- call hunting, call
//! parking, callbacks, caller-ID filtering, DISAs, music on hold, ring groups,
//! recordings, time conditions, and a queue's static members. The list-all
//! reads probe cleanly; the id- or date-scoped reads (`getCallRecording`,
//! `getCallRecordings`, `getRecordingFile`, `getStaticMembers`) are skipped at
//! probe depth.
//!
//! At `Lifecycle` depth the area runs create -> read -> delete fixtures over the
//! free, self-contained resources it owns: a callback, a DISA, a ring group, a
//! time condition, a queue static member, and a recording. The static member
//! depends on a queue, so the fixture creates a throwaway marker-bearing queue
//! to hang it on and tears the member down before the queue (LIFO). Its
//! [`sweep`](Area::sweep) reclaims marker-bearing leftovers of each.
//!
//! The recording fixture is the only file upload in the harness, and so the
//! only live exercise of the multipart transport: its generated WAV is larger
//! than the request line a GET fits.

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{
    Orphan, owned, queue_number, read_back, required_queue_params, sweep_orphans, tolerate_absent,
};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Callflow;

const AREA: &str = "callflow";

/// The request line voip.ms accepts, Apache's default `LimitRequestLine`. A GET
/// carries its whole request there, so a payload past this could not have been
/// sent as one. Compared against base64 characters, which is what a query
/// string would have had to hold.
const REQUEST_LINE_BYTES: usize = 8_190;

/// Seconds of audio the fixture uploads. Read by the upload itself and by the
/// tests that assert the payload is too big for a request line, so shrinking it
/// cannot quietly invalidate them.
const FIXTURE_SECONDS: u32 = 2;

/// How much of an upload's *duration* must survive voip.ms's re-encode. The one
/// re-encode on record trimmed 9%; half leaves room for a lossier pass while
/// still failing a body that arrived cut. Duration rather than bytes, since a
/// codec change moves the byte count without losing audio.
const MIN_STORED_FRACTION: f32 = 0.5;

#[async_trait(?Send)]
impl Area for Callflow {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delCallHunting",
            "delCallParking",
            "delCallRecording",
            "delCallback",
            "delCallerIDFiltering",
            "delDISA",
            "delMusicOnHold",
            "delRecording",
            "delRingGroup",
            "delStaticMember",
            "delTimeCondition",
            "getCallHuntings",
            "getCallParking",
            "getCallRecording",
            "getCallRecordings",
            "getCallbacks",
            "getCallerIDFiltering",
            "getDISAs",
            "getMusicOnHold",
            "getRecordingFile",
            "getRecordings",
            "getRingGroups",
            "getStaticMembers",
            "getTimeConditions",
            "sendCallRecordingEmail",
            "setCallHunting",
            "setCallParking",
            "setCallback",
            "setCallerIDFiltering",
            "setDISA",
            "setMusicOnHold",
            "setRecording",
            "setRingGroup",
            "setStaticMember",
            "setTimeCondition",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallHuntings",
            GetCallHuntingsParams,
            GetCallHuntingsResponse,
            call_hunting
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallParking",
            GetCallParkingParams,
            GetCallParkingResponse,
            call_hunting
        );
        skip_needs_input!(
            report,
            AREA,
            "getCallRecording",
            "requires a call-recording id"
        );
        skip_needs_input!(report, AREA, "getCallRecordings", "requires a date window");
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallbacks",
            GetCallbacksParams,
            GetCallbacksResponse,
            callbacks
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getCallerIDFiltering",
            GetCallerIDFilteringParams,
            GetCallerIDFilteringResponse,
            filtering
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getDISAs",
            GetDISAsParams,
            GetDISAsResponse,
            disa
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getMusicOnHold",
            GetMusicOnHoldParams,
            GetMusicOnHoldResponse,
            music_on_hold
        );
        skip_needs_input!(report, AREA, "getRecordingFile", "requires a recording id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getRecordings",
            GetRecordingsParams,
            GetRecordingsResponse,
            recordings
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getRingGroups",
            GetRingGroupsParams,
            GetRingGroupsResponse,
            ring_groups
        );
        skip_needs_input!(report, AREA, "getStaticMembers", "requires a queue id");
        probe_list!(
            ctx,
            report,
            AREA,
            "getTimeConditions",
            GetTimeConditionsParams,
            GetTimeConditionsResponse,
            timecondition
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        let mut unreconciled = Vec::new();

        // The static-member fixture hangs its member off a throwaway
        // marker-bearing queue, and a queue has no account-wide member listing
        // to reclaim members directly -- so the reclaim for both is deleting the
        // queue, expecting the member to go with it. Done here rather than
        // leaning on the `queue` area's sweep so callflow is self-contained when
        // run alone (`--areas callflow`). If a queue delete is refused while a
        // member still hangs on it, the failed delete surfaces as a non-clean
        // sweep that blocks the run -- not a silent leak.
        //
        // A recording is reclaimed last because, unlike the others, it is
        // referenceable -- `setIVR.recording`, `setMusicOnHold.recordings`, a
        // ring group's caller announcement, queue announcements -- and
        // `delRecording` on a referenced recording is refused, which fails the
        // sweep and aborts the run. No fixture in this area creates such a
        // reference, so the ordering is defense in depth rather than a fix for
        // a known case; the cross-area one that did exist is prevented at its
        // source, by the `ivr` fixture passing over marker-bearing recordings.
        // The remaining resources are independent and their order is
        // immaterial.
        for result in [
            sweep_orphans(
                report,
                AREA,
                "callback",
                || list_callback_orphans(client),
                |id| del_callback(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "disa",
                || list_disa_orphans(client),
                |id| del_disa(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "ringgroup",
                || list_ring_group_orphans(client),
                |id| del_ring_group(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "timecondition",
                || list_time_condition_orphans(client),
                |id| del_time_condition(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "staticmember-queue",
                || list_dep_queue_orphans(client),
                |id| del_queue(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "recording",
                || list_recording_orphans(client),
                |id| del_recording(client, id),
            )
            .await,
        ] {
            unreconciled.extend(result.unreconciled);
        }

        SweepResult { unreconciled }
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        callback_fixture(ctx, report, &mut scope).await;
        disa_fixture(ctx, report, &mut scope).await;
        ring_group_fixture(ctx, report, &mut scope).await;
        time_condition_fixture(ctx, report, &mut scope).await;
        static_member_fixture(ctx, report, &mut scope).await;
        recording_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn callback_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let description = ctx.token.marker(0);

    let created = client
        .set_callback(&SetCallbackParams {
            description: Some(description),
            number: Some("15555550100".into()),
            // `delay_before` must be non-zero; the API treats 0 as absent
            // (`missing_delay_before`).
            delay_before: Some(1),
            response_timeout: Some(5),
            digit_timeout: Some(5),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.callback {
            Some(id) => id,
            None => return fail(report, "fixture:setCallback", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setCallback",
                &format!("setCallback: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setCallback", Outcome::Pass);
    scope.defer(format!("callback id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_callback(&DelCallbackParams { callback: Some(id) })
                    .await,
            )
        })
    });

    read_back::<_, GetCallbacksResponse>(
        client,
        report,
        AREA,
        "fixture:getCallbacks",
        &GetCallbacksParams::default(),
        |r| Some(r.callbacks.len()),
    )
    .await;
}

async fn disa_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(1);

    let created = client
        .set_disa(&SetDISAParams {
            name: Some(name),
            pin: Some(1234),
            digit_timeout: Some(5),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.disa {
            Some(id) => id,
            None => return fail(report, "fixture:setDISA", "no id returned"),
        },
        Err(error) => return fail(report, "fixture:setDISA", &format!("setDISA: {error}")),
    };

    report.record(AREA, "fixture:setDISA", Outcome::Pass);
    scope.defer(format!("disa id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(client.del_disa(&DelDISAParams { disa: Some(id) }).await)
        })
    });

    read_back::<_, GetDISAsResponse>(
        client,
        report,
        AREA,
        "fixture:getDISAs",
        &GetDISAsParams::default(),
        |r| Some(r.disa.len()),
    )
    .await;
}

async fn ring_group_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.short_marker(2);

    // `members` and `voicemail` are `(required)`. `members` takes routing
    // headers to real resources (`account:<sub>`); a nonexistent sub is rejected
    // (`invalid_mailbox`), so discover a real one and skip if the account has
    // none. `voicemail` is a box id or the `0` ("no voicemail") sentinel -- the
    // word `none` is not accepted.
    let sub_account = match client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await
    {
        Ok(resp) => resp.accounts.into_iter().find_map(|a| a.account),
        Err(error) => {
            return fail(
                report,
                "fixture:setRingGroup",
                &format!("getSubAccounts (for member): {error}"),
            );
        }
    };
    let Some(sub_account) = sub_account else {
        return report.record(
            AREA,
            "fixture:setRingGroup",
            Outcome::Skip("requires a sub-account to use as a member".to_string()),
        );
    };

    let created = client
        .set_ring_group(&SetRingGroupParams {
            name: Some(name),
            members: Some(format!("account:{sub_account}")),
            voicemail: Some("0".into()),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.ring_group {
            Some(id) => id,
            None => return fail(report, "fixture:setRingGroup", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setRingGroup",
                &format!("setRingGroup: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setRingGroup", Outcome::Pass);
    scope.defer(format!("ringgroup id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_ring_group(&DelRingGroupParams {
                        ring_group: Some(id),
                    })
                    .await,
            )
        })
    });

    read_back::<_, GetRingGroupsResponse>(
        client,
        report,
        AREA,
        "fixture:getRingGroups",
        &GetRingGroupsParams::default(),
        |r| Some(r.ring_groups.len()),
    )
    .await;
}

async fn time_condition_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(3);

    let created = client
        .set_time_condition(&SetTimeConditionParams {
            name: Some(name),
            // `Routing::None` serializes to the header `none:`, which
            // `setTimeCondition` rejects with `invalid_routing_header`; a
            // system action (`sys:hangup`) is a valid target that needs no
            // dependent resource. Best-effort until a live run confirms it.
            routing_match: Some(Routing::System("hangup".into())),
            routing_nomatch: Some(Routing::System("hangup".into())),
            starthour: Some("09".to_string()),
            startminute: Some("00".to_string()),
            endhour: Some("17".to_string()),
            endminute: Some("00".to_string()),
            // Weekday conditions are day abbreviations (`mon`..`sun`), not
            // numbers -- a numeric value is rejected (`invalid_weekdaystart`).
            weekdaystart: Some("mon".to_string()),
            weekdayend: Some("fri".to_string()),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.timecondition {
            Some(id) => id,
            None => return fail(report, "fixture:setTimeCondition", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setTimeCondition",
                &format!("setTimeCondition: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setTimeCondition", Outcome::Pass);
    scope.defer(format!("timecondition id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_time_condition(&DelTimeConditionParams {
                        timecondition: Some(id),
                    })
                    .await,
            )
        })
    });

    read_back::<_, GetTimeConditionsResponse>(
        client,
        report,
        AREA,
        "fixture:getTimeConditions",
        &GetTimeConditionsParams::default(),
        |r| Some(r.timecondition.len()),
    )
    .await;
}

/// A static member hangs off a queue, so the fixture stands up a throwaway
/// marker-bearing queue first, hangs the member on it, and defers teardown of
/// the member *then* the queue -- LIFO cleanup enforces the dependency order.
async fn static_member_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let queue_name = ctx.token.marker(4);

    let queue_id = match client
        .set_queue(&SetQueueParams {
            queue_name: Some(queue_name),
            ..required_queue_params(queue_number(ctx.token.as_str(), 1))
        })
        .await
    {
        Ok(resp) => match resp.queue {
            Some(id) => id,
            None => return fail(report, "fixture:setQueue(dep)", "no id returned"),
        },
        // A queue number the account can't provision (no internal-extension
        // range) is rejected with `invalid_number`; that is an account
        // limitation, not a harness defect, so skip rather than fail.
        Err(Error::Api(ApiStatus::InvalidNumber)) => {
            return report.record(
                AREA,
                "fixture:setQueue(dep)",
                Outcome::Skip("account has no provisionable queue number".to_string()),
            );
        }
        Err(error) => {
            return fail(
                report,
                "fixture:setQueue(dep)",
                &format!("setQueue dependency: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setQueue(dep)", Outcome::Pass);
    scope.defer(format!("queue(dep) id={queue_id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_queue(&DelQueueParams {
                        queue: Some(queue_id),
                    })
                    .await,
            )
        })
    });

    let member_name = ctx.token.marker(5);
    let created = client
        .set_static_member(&SetStaticMemberParams {
            queue: Some(queue_id),
            member_name: Some(member_name),
            member: Some("15555550101".to_string()),
            priority: Some(1),
            ..Default::default()
        })
        .await;

    let member_id = match created {
        Ok(resp) => match resp.member {
            Some(id) => id,
            None => return fail(report, "fixture:setStaticMember", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setStaticMember",
                &format!("setStaticMember: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setStaticMember", Outcome::Pass);
    scope.defer(format!("staticmember id={member_id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_static_member(&DelStaticMemberParams {
                        member: Some(member_id),
                        queue: Some(queue_id),
                    })
                    .await,
            )
        })
    });

    read_back::<_, GetStaticMembersResponse>(
        client,
        report,
        AREA,
        "fixture:getStaticMembers",
        &GetStaticMembersParams {
            queue: Some(queue_id.to_string()),
            ..Default::default()
        },
        |r| Some(r.members.len()),
    )
    .await;
}

/// Upload a generated WAV through `setRecording`, read it back through
/// `getRecordingFile`, and delete it.
///
/// The payload is the point: at 8 kHz mono 16-bit, two seconds of audio is
/// 32 kB, over 42 kB base64-encoded, five times [`REQUEST_LINE_BYTES`] -- so
/// this is the only live coverage of the multipart transport.
///
/// The read-back goes through [`stored_wav`], which is given the uploaded size
/// so it can judge what came back against it. `getRecordingFile` is called
/// directly rather than through [`read_back`], whose drift diff looks at the
/// response shape and not at the bytes the shape carries.
async fn recording_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.short_marker(6);
    let wav = tone_wav(FIXTURE_SECONDS);

    let created = client
        .set_recording(&SetRecordingParams {
            name: Some(name),
            file: Some(BASE64.encode(&wav)),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.recording {
            Some(id) => id,
            None => return fail(report, "fixture:setRecording", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setRecording",
                &format!("setRecording: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setRecording", Outcome::Pass);
    scope.defer(format!("recording id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_recording(&DelRecordingParams {
                        recording: Some(id),
                    })
                    .await,
            )
        })
    });

    read_back::<_, GetRecordingsResponse>(
        client,
        report,
        AREA,
        "fixture:getRecordings",
        &GetRecordingsParams {
            recording: Some(id.to_string()),
        },
        |r| Some(r.recordings.len()),
    )
    .await;

    match client
        .get_recording_file(&GetRecordingFileParams {
            recording: Some(id.to_string()),
        })
        .await
    {
        Ok(resp) => match resp.recordings.first().and_then(|r| r.data.as_deref()) {
            Some(data) => report.record(
                AREA,
                "fixture:getRecordingFile",
                stored_wav(data, FIXTURE_SECONDS),
            ),
            None => fail(report, "fixture:getRecordingFile", "no file data returned"),
        },
        Err(error) => fail(
            report,
            "fixture:getRecordingFile",
            &format!("getRecordingFile: {error}"),
        ),
    }
}

/// Whether the stored recording still holds the audio that was uploaded.
///
/// Compared as duration, not as bytes. voip.ms re-encodes what it stores, and
/// how many bytes a given number of seconds occupies is a property of the codec
/// it picks: a re-encode of this fixture's 16-bit PCM to G.711, the ordinary
/// telephony codec, halves the byte count while losing no audio at all. A
/// byte-count floor cannot tell that apart from a request body that arrived cut
/// in half, so it would fail an intact upload.
///
/// Catches a truncation that removes more than `1 - `[`MIN_STORED_FRACTION`] of
/// the audio. A smaller cut passes, which is the price of leaving a re-encode
/// room to trim -- the one on record took 9% off the duration.
fn stored_wav(data: &str, uploaded_secs: u32) -> Outcome {
    let bytes = match BASE64.decode(data) {
        Ok(bytes) => bytes,
        Err(error) => return Outcome::Fail(format!("stored file is not base64: {error}")),
    };

    let Some(stored_secs) = wav_duration_secs(&bytes) else {
        return Outcome::Fail(format!(
            "stored file is not a readable RIFF/WAVE container ({} bytes)",
            bytes.len()
        ));
    };

    let floor = uploaded_secs as f32 * MIN_STORED_FRACTION;
    if stored_secs >= floor {
        Outcome::Pass
    } else {
        Outcome::Fail(format!(
            "stored recording is {stored_secs:.2} s against a {uploaded_secs} s upload, \
             under the {floor:.2} s floor: the upload arrived incomplete"
        ))
    }
}

/// The audio duration a WAV carries, in seconds, or `None` if the bytes are not
/// a WAV this can read.
///
/// Taken from the container rather than from its size. `fmt `'s
/// average-bytes-per-second field is defined for every WAV format, so dividing
/// the `data` chunk's length by it yields a duration without assuming which
/// codec voip.ms stored.
fn wav_duration_secs(bytes: &[u8]) -> Option<f32> {
    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }

    let mut bytes_per_sec: Option<u32> = None;
    let mut data_len: Option<usize> = None;
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = bytes.get(pos..pos + 4)?;
        let declared = u32::from_le_bytes(bytes.get(pos + 4..pos + 8)?.try_into().ok()?) as usize;
        let body = pos + 8;
        match id {
            // wFormatTag, nChannels, nSamplesPerSec, then nAvgBytesPerSec.
            b"fmt " => {
                let field = bytes.get(body + 8..body + 12)?;
                bytes_per_sec = Some(u32::from_le_bytes(field.try_into().ok()?));
            }
            // A truncated file can declare more than it carries; take what is
            // actually there, since that is what was received.
            b"data" => data_len = Some(declared.min(bytes.len().saturating_sub(body))),
            _ => {}
        }

        // Chunks are padded to an even length.
        pos = body + declared + (declared & 1);
    }

    match (bytes_per_sec, data_len) {
        (Some(rate), Some(len)) if rate > 0 => Some(len as f32 / rate as f32),
        _ => None,
    }
}

/// A RIFF/WAVE container holding `seconds` of a 440 Hz tone at 8 kHz mono
/// 16-bit PCM -- the format voip.ms stores a recording in, so nothing about the
/// upload depends on its conversion. A tone rather than silence, so a stored
/// file that lost its payload is still distinguishable from one that kept it.
fn tone_wav(seconds: u32) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 8_000;
    const BYTES_PER_FRAME: u32 = 2;
    // RIFF header through the `data` chunk's length field, less the leading
    // `RIFF` tag and the length field itself -- what the RIFF length counts.
    const HEADER_LEN: u32 = 36;

    let frames = seconds * SAMPLE_RATE;
    let data_len = frames * BYTES_PER_FRAME;
    // The 8 the RIFF length excludes: its own tag and length field.
    let mut wav = Vec::with_capacity((HEADER_LEN + 8 + data_len) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(HEADER_LEN + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk length
    wav.extend_from_slice(&1u16.to_le_bytes()); // format: uncompressed PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // channels
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * BYTES_PER_FRAME).to_le_bytes()); // bytes per second
    wav.extend_from_slice(&(BYTES_PER_FRAME as u16).to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for frame in 0..frames {
        let phase = std::f32::consts::TAU * 440.0 * frame as f32 / SAMPLE_RATE as f32;
        // Quarter scale: loud enough to survive a re-encode, quiet enough not
        // to clip.
        let sample = (phase.sin() * f32::from(i16::MAX) / 4.0) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    wav
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}

async fn list_callback_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetCallbacksResponse = client.get_callbacks(&GetCallbacksParams::default()).await?;
    Ok(resp
        .callbacks
        .into_iter()
        .filter(|c| owned(&c.description))
        .filter_map(|c| {
            c.callback.map(|id| Orphan {
                label: format!("callback id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_callback(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_callback(&DelCallbackParams { callback: Some(id) })
        .await?;
    Ok(())
}

async fn list_disa_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetDISAsResponse = client.get_disas(&GetDISAsParams::default()).await?;
    Ok(resp
        .disa
        .into_iter()
        .filter(|d| owned(&d.name))
        .filter_map(|d| {
            d.disa.map(|id| Orphan {
                label: format!("disa id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_disa(client: &Client, id: u64) -> anyhow::Result<()> {
    client.del_disa(&DelDISAParams { disa: Some(id) }).await?;
    Ok(())
}

async fn list_ring_group_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetRingGroupsResponse = client
        .get_ring_groups(&GetRingGroupsParams::default())
        .await?;
    Ok(resp
        .ring_groups
        .into_iter()
        .filter(|g| owned(&g.name))
        .filter_map(|g| {
            g.ring_group.map(|id| Orphan {
                label: format!("ringgroup id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_ring_group(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_ring_group(&DelRingGroupParams {
            ring_group: Some(id),
        })
        .await?;
    Ok(())
}

async fn list_time_condition_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetTimeConditionsResponse = client
        .get_time_conditions(&GetTimeConditionsParams::default())
        .await?;
    Ok(resp
        .timecondition
        .into_iter()
        .filter(|t| owned(&t.name))
        .filter_map(|t| {
            t.timecondition.map(|id| Orphan {
                label: format!("timecondition id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_time_condition(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_time_condition(&DelTimeConditionParams {
            timecondition: Some(id),
        })
        .await?;
    Ok(())
}

async fn list_recording_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetRecordingsResponse = client
        .get_recordings(&GetRecordingsParams::default())
        .await?;
    Ok(resp
        .recordings
        .into_iter()
        .filter(|r| owned(&r.description))
        .filter_map(|r| {
            r.value.map(|id| Orphan {
                label: format!("recording id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_recording(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_recording(&DelRecordingParams {
            recording: Some(id),
        })
        .await?;
    Ok(())
}

/// The marker-bearing queues the static-member fixture stands up as scaffolding.
/// Enumerated here (not left to the `queue` area) so callflow reclaims them --
/// and, by cascade, any static member left on them -- when run without `queue`.
/// Overlaps harmlessly with the `queue` area's sweep when both are selected.
async fn list_dep_queue_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetQueuesResponse = client.get_queues(&GetQueuesParams::default()).await?;
    Ok(resp
        .queues
        .into_iter()
        .filter(|q| owned(&q.queue_name))
        .filter_map(|q| {
            q.queue.map(|id| Orphan {
                label: format!("queue id={id}"),
                id,
            })
        })
        .collect())
}

async fn del_queue(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_queue(&DelQueueParams { queue: Some(id) })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A G.711 mu-law WAV of `seconds`: same audio, half the bytes of the
    /// 16-bit PCM the fixture uploads. Stands in for voip.ms re-encoding to the
    /// ordinary telephony codec, which a byte-count floor would fail.
    fn mulaw_wav(seconds: u32) -> Vec<u8> {
        const SAMPLE_RATE: u32 = 8_000;
        let data_len = seconds * SAMPLE_RATE;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&7u16.to_le_bytes()); // format: G.711 mu-law
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes()); // one byte per frame
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend(std::iter::repeat_n(0x7fu8, data_len as usize));
        wav
    }

    #[test]
    fn tone_wav_is_a_decodable_wave_of_the_expected_size() {
        let wav = tone_wav(FIXTURE_SECONDS);
        // 44-byte header plus the frames, at 8 kHz 16-bit mono.
        let frames = FIXTURE_SECONDS * 8_000;
        assert_eq!(wav.len() as u32, 44 + frames * 2);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 8_000);
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(
            u32::from_le_bytes(wav[40..44].try_into().unwrap()),
            frames * 2
        );
        assert!(
            wav[44..].iter().any(|b| *b != 0),
            "the tone must carry a signal, not silence"
        );
    }

    #[test]
    fn wav_duration_is_read_from_the_container_not_its_size() {
        // Two encodings of the same two seconds, one of them half the bytes.
        let pcm = tone_wav(2);
        let mulaw = mulaw_wav(2);
        assert!(mulaw.len() < pcm.len() * 3 / 5, "the case's premise");
        assert_eq!(wav_duration_secs(&pcm), Some(2.0));
        assert_eq!(wav_duration_secs(&mulaw), Some(2.0));
        assert_eq!(wav_duration_secs(b"not a wav at all"), None);
    }

    #[test]
    fn stored_wav_accepts_an_intact_upload_however_it_was_re_encoded() {
        assert!(matches!(
            stored_wav(&BASE64.encode(tone_wav(FIXTURE_SECONDS)), FIXTURE_SECONDS),
            Outcome::Pass
        ));

        // A re-encode to G.711 halves the byte count and loses no audio. This
        // is the case a byte-count floor failed: it lands within 22 bytes of
        // half the upload, and under it once any trim is applied.
        let mulaw = mulaw_wav(FIXTURE_SECONDS);
        assert!(
            mulaw.len() < tone_wav(FIXTURE_SECONDS).len() / 2 + 64,
            "the case's premise"
        );
        assert!(matches!(
            stored_wav(&BASE64.encode(&mulaw), FIXTURE_SECONDS),
            Outcome::Pass
        ));
    }

    #[test]
    fn stored_wav_rejects_a_truncated_upload() {
        // A header with no frames: a well-formed container carrying nothing,
        // which is what a wholly cut body looks like once voip.ms stores it.
        assert!(matches!(
            stored_wav(&BASE64.encode(tone_wav(0)), FIXTURE_SECONDS),
            Outcome::Fail(_)
        ));

        // A partial cut, the case an absolute byte floor passed: a quarter of
        // the file, still a container far larger than a request line. Its
        // `data` chunk declares more than remains, which is how a cut body
        // arrives, so the duration comes from what is actually there.
        let full = tone_wav(FIXTURE_SECONDS);
        let partial = full[..full.len() / 4].to_vec();
        assert!(
            BASE64.encode(&partial).len() > REQUEST_LINE_BYTES,
            "the case's premise"
        );
        assert!(matches!(
            stored_wav(&BASE64.encode(&partial), FIXTURE_SECONDS),
            Outcome::Fail(_)
        ));

        assert!(matches!(
            stored_wav(&BASE64.encode(b"not audio at all"), FIXTURE_SECONDS),
            Outcome::Fail(_)
        ));
        assert!(matches!(
            stored_wav("not base64 either!", FIXTURE_SECONDS),
            Outcome::Fail(_)
        ));
    }

    #[test]
    fn the_uploaded_recording_could_not_have_been_sent_on_a_request_line() {
        // The whole reason the method posts, asserted against the payload the
        // fixture actually sends: both read FIXTURE_SECONDS, so shrinking the
        // upload toward a size a GET could carry fails here.
        let encoded = BASE64.encode(tone_wav(FIXTURE_SECONDS)).len();
        assert!(
            encoded > 4 * REQUEST_LINE_BYTES,
            "{encoded} base64 chars must clear the request line by a wide margin"
        );
    }
}
