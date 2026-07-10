//! The `sms` area: SMS messages. The message list probes cleanly;
//! `sendSMS`/`setSMS`/`deleteSMS` are owned but run only at costly depth.
//!
//! At `Depth::Costly`, given `--test-did` and `--sms-dst` (both required
//! together), `sendSMS` sends one message from the dedicated, already
//! SMS-enabled `--test-did` to `--sms-dst`, then reads it back through
//! `getSMS`. `--test-did` is deliberately not the number [`crate::areas::dids`]
//! orders and cancels: this fixture never mutates it, so the sms/mms and dids
//! costly paths can be run independently. `setSMS`/`deleteSMS` are not
//! exercised here: `setSMS` is dids' concern (enabling it on an ordered
//! number), and `deleteSMS` would destroy the message this fixture just
//! validated the read-back of.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::config::Depth;
use crate::harness::area::{Area, AreaCtx, CostClass};
use crate::harness::fixtures::read_back;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Sms;

const AREA: &str = "sms";

#[async_trait(?Send)]
impl Area for Sms {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["deleteSMS", "getSMS", "sendSMS", "setSMS"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getSMS",
            GetSMSParams,
            GetSMSResponse,
            sms
        );
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        if ctx.depth != Depth::Costly {
            return;
        }

        let Some(fixture) = &ctx.config.sms_fixture else {
            report.record(
                AREA,
                "fixture:sendSMS",
                Outcome::Skip("--test-did/--sms-dst not set".to_string()),
            );
            return;
        };

        let sent = ctx
            .client
            .send_sms(&SendSMSParams {
                did: Some(fixture.test_did.clone()),
                dst: Some(fixture.sms_dst.clone()),
                message: Some(format!("{} sms fixture", ctx.token.marker(0))),
            })
            .await;

        match sent {
            Ok(_) => report.record(AREA, "fixture:sendSMS", Outcome::Pass),
            Err(error) => {
                report.record(
                    AREA,
                    "fixture:sendSMS",
                    Outcome::Fail(format!("sendSMS: {error}")),
                );
                return;
            }
        }

        read_back::<_, GetSMSResponse>(
            ctx.client,
            report,
            AREA,
            "fixture:getSMS",
            &GetSMSParams {
                did: Some(fixture.test_did.clone()),
                ..Default::default()
            },
            |r| Some(r.sms.len()),
        )
        .await;
    }
}
