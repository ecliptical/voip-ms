//! The `voicemail` area: voicemail boxes and their setups, messages, and
//! transcriptions. The box and setup lists probe cleanly; the message,
//! message-file, and transcription reads need a mailbox (and folder/message
//! number, or a date window), so they are skipped at probe depth.
//!
//! At `Lifecycle` depth the area runs a create -> read -> delete fixture over a
//! voicemail box, marker in its `name`. The read-back is the point: a populated
//! `getVoicemails` element is where the message-date and callerid folds are
//! exercised. Its [`sweep`](Area::sweep) reclaims marker-bearing boxes from
//! prior runs. (Voicemail attachment formats and folder enumerations are
//! reference-scope lookups and live in the `reference` area.)

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans, tolerate_absent};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Voicemail;

const AREA: &str = "voicemail";

#[async_trait(?Send)]
impl Area for Voicemail {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "createVoicemail",
            "delMessages",
            "delVoicemail",
            "getVoicemailMessageFile",
            "getVoicemailMessages",
            "getVoicemailSetups",
            "getVoicemailTranscriptions",
            "getVoicemails",
            "markListenedVoicemailMessage",
            "markUrgentVoicemailMessage",
            "moveFolderVoicemailMessage",
            "sendVoicemailEmail",
            "setDIDVoicemail",
            "setVoicemail",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        skip_needs_input!(
            report,
            AREA,
            "getVoicemailMessageFile",
            "requires a mailbox, folder, and message number"
        );
        skip_needs_input!(report, AREA, "getVoicemailMessages", "requires a mailbox");
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemailSetups",
            GetVoicemailSetupsParams,
            GetVoicemailSetupsResponse,
            voicemailsetups
        );
        skip_needs_input!(
            report,
            AREA,
            "getVoicemailTranscriptions",
            "requires an account and mailbox"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getVoicemails",
            GetVoicemailsParams,
            GetVoicemailsResponse,
            voicemails
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        sweep_orphans(
            report,
            AREA,
            "voicemail",
            || list_orphans(client),
            |mailbox| del_voicemail(client, mailbox),
        )
        .await
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        voicemail_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

/// Create -> read-back -> (deferred) delete a voicemail box. The mailbox number
/// is caller-chosen (`digits`), so it is the id used for read-back and
/// teardown; the marker rides in the box `name` for the sweep to recognize.
async fn voicemail_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let mailbox = mailbox_digits(ctx.token.as_str());
    let name = ctx.token.marker(0);

    let created = client
        .create_voicemail(&CreateVoicemailParams {
            digits: Some(mailbox),
            name: Some(name.clone()),
            // A voicemail password must be exactly 4 digits (API rule behind
            // `invalid_password`); the 4-digit `mailbox` doubles as one.
            password: Some(format!("{mailbox:04}")),
            skip_password: Some(false),
            attach_message: Some(false),
            delete_message: Some(false),
            say_time: Some(false),
            timezone: Some("America/New_York".to_string()),
            say_callerid: Some(false),
            play_instructions: Some(PlayInstructions::Unread),
            language: Some("en".to_string()),
            // `createVoicemail` requires a real attachment format from
            // getVoicemailAttachmentFormats; `no` is rejected
            // (`invalid_email_attachment_format`). `wav49` is the recommended one.
            email_attachment_format: Some(EmailAttachmentFormat::Wav49),
            ..Default::default()
        })
        .await;

    if let Err(error) = created {
        report.record(
            AREA,
            "fixture:createVoicemail",
            Outcome::Fail(format!("createVoicemail: {error}")),
        );
        return;
    }

    report.record(AREA, "fixture:createVoicemail", Outcome::Pass);
    scope.defer(format!("voicemail mailbox={mailbox}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_voicemail(&DelVoicemailParams {
                        mailbox: Some(mailbox),
                    })
                    .await,
            )
        })
    });

    read_back::<_, GetVoicemailsResponse>(
        client,
        report,
        AREA,
        "fixture:getVoicemails",
        &GetVoicemailsParams {
            mailbox: Some(mailbox.to_string()),
        },
        |r| Some(r.voicemails.len()),
    )
    .await;
}

/// A deterministic-per-run mailbox number in the 4-digit space, folded from the
/// run token. Collisions across runs are reclaimed by the marker-driven sweep;
/// a same-run collision cannot happen (one fixture per run).
fn mailbox_digits(token: &str) -> u64 {
    let mut hash: u64 = 0;
    for b in token.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(*b));
    }

    // 1000..=9999: minimum 1 digit is allowed, but a 4-digit box avoids the
    // low-numbered ranges a real account is likelier to use.
    1000 + (hash % 9000)
}

async fn list_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetVoicemailsResponse = client
        .get_voicemails(&GetVoicemailsParams::default())
        .await?;
    Ok(resp
        .voicemails
        .into_iter()
        .filter(|v| owned(&v.name))
        .filter_map(|v| {
            v.mailbox.map(|mailbox| Orphan {
                label: format!("voicemail mailbox={mailbox}"),
                id: mailbox,
            })
        })
        .collect())
}

async fn del_voicemail(client: &Client, mailbox: u64) -> anyhow::Result<()> {
    client
        .del_voicemail(&DelVoicemailParams {
            mailbox: Some(mailbox),
        })
        .await?;
    Ok(())
}
