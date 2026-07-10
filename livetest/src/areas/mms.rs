//! The `mms` area: MMS messages and their media. The message list probes
//! cleanly; fetching a message's media needs an MMS id, so it is skipped at
//! probe depth. `sendMMS`/`deleteMMS` are owned but run only at costly depth.
//!
//! At `Depth::Costly`, given `--test-did` and `--sms-dst` (both required
//! together, shared with [`crate::areas::sms`]) plus `--mms-media-url`,
//! `sendMMS` sends one message with the media attachment from the dedicated
//! `--test-did` to `--sms-dst`, then reads it back through `getMMS` and
//! `getMediaMMS` (scoped by the id `sendMMS` returns). `deleteMMS` is not
//! exercised: it would destroy the message this fixture just validated the
//! read-back of.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Mms;

const AREA: &str = "mms";

#[async_trait(?Send)]
impl Area for Mms {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["deleteMMS", "getMMS", "getMediaMMS", "sendMMS"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getMMS",
            GetMMSParams,
            GetMMSResponse,
            sms
        );
        skip_needs_input!(report, AREA, "getMediaMMS", "requires an MMS id");
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let (Some(fixture), Some(media_url)) = (&ctx.config.sms_fixture, &ctx.config.mms_media_url)
        else {
            report.record(
                AREA,
                "fixture:sendMMS",
                Outcome::Skip("--test-did/--sms-dst/--mms-media-url not set".to_string()),
            );
            return;
        };

        let sent = ctx
            .client
            .send_mms(&SendMMSParams {
                did: Some(fixture.test_did.clone()),
                dst: Some(fixture.sms_dst.clone()),
                message: Some(format!("{} mms fixture", ctx.token.marker(0))),
                media1: Some(media_url.clone()),
                ..Default::default()
            })
            .await;

        let id = match sent {
            Ok(resp) => match resp.mms {
                Some(id) => id,
                None => {
                    return fail(report, "fixture:sendMMS", "sendMMS succeeded without an id");
                }
            },
            Err(error) => return fail(report, "fixture:sendMMS", &format!("sendMMS: {error}")),
        };

        report.record(AREA, "fixture:sendMMS", Outcome::Pass);

        read_back::<_, GetMMSResponse>(
            ctx.client,
            report,
            AREA,
            "fixture:getMMS",
            &GetMMSParams {
                did: Some(fixture.test_did.clone()),
                ..Default::default()
            },
            |r| Some(r.sms.len()),
        )
        .await;

        read_back::<_, GetMediaMMSResponse>(
            ctx.client,
            report,
            AREA,
            "fixture:getMediaMMS",
            &GetMediaMMSParams {
                id: Some(id as i64),
                ..Default::default()
            },
            |_| None,
        )
        .await;
    }
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}
