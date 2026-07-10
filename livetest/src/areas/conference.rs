//! The `conference` area: conference bridges and their members. The bridge and
//! member lists probe cleanly; the recording reads require a conference id (and
//! `getConferenceRecordingFile` a recording id too), so they are skipped at
//! probe depth.
//!
//! At `Lifecycle` depth the area runs create -> read -> delete fixtures over a
//! conference bridge and a standalone conference-member profile (each marker in
//! its `description`) and its [`sweep`](Area::sweep) reclaims marker-bearing
//! leftovers of both.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Conference;

const AREA: &str = "conference";

#[async_trait(?Send)]
impl Area for Conference {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "addMemberToConference",
            "delConference",
            "delConferenceMember",
            "delMemberFromConference",
            "getConference",
            "getConferenceMembers",
            "getConferenceRecordingFile",
            "getConferenceRecordings",
            "setConference",
            "setConferenceMember",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getConference",
            GetConferenceParams,
            GetConferenceResponse,
            conference
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getConferenceMembers",
            GetConferenceMembersParams,
            GetConferenceMembersResponse,
            members
        );
        skip_needs_input!(
            report,
            AREA,
            "getConferenceRecordingFile",
            "requires a conference and recording id"
        );
        skip_needs_input!(
            report,
            AREA,
            "getConferenceRecordings",
            "requires a conference id"
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        let mut unreconciled = Vec::new();

        for result in [
            sweep_orphans(
                report,
                AREA,
                "conference",
                || list_conference_orphans(client),
                |id| del_conference(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "conferencemember",
                || list_member_orphans(client),
                |id| del_member(client, id),
            )
            .await,
        ] {
            unreconciled.extend(result.unreconciled);
        }

        SweepResult { unreconciled }
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        conference_fixture(ctx, report, &mut scope).await;
        member_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn conference_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.username(0);
    let description = ctx.token.marker(0);

    let created = client
        .set_conference(&SetConferenceParams {
            name: Some(name),
            description: Some(description),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.conference {
            Some(id) => id as i64,
            None => return fail(report, "fixture:setConference", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setConference",
                &format!("setConference: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setConference", Outcome::Pass);
    scope.defer(format!("conference id={id}"), move |client| {
        Box::pin(async move {
            client
                .del_conference(&DelConferenceParams {
                    conference: Some(id),
                })
                .await?;
            Ok(())
        })
    });

    read_back::<_, GetConferenceResponse>(
        client,
        report,
        AREA,
        "fixture:getConference",
        &GetConferenceParams::default(),
        |r| Some(r.conference.len()),
    )
    .await;
}

async fn member_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.username(1);
    let description = ctx.token.marker(1);

    let created = client
        .set_conference_member(&SetConferenceMemberParams {
            name: Some(name),
            description: Some(description),
            pin: Some(4321),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.member {
            Some(id) => id as i64,
            None => return fail(report, "fixture:setConferenceMember", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setConferenceMember",
                &format!("setConferenceMember: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setConferenceMember", Outcome::Pass);
    scope.defer(format!("conferencemember id={id}"), move |client| {
        Box::pin(async move {
            client
                .del_conference_member(&DelConferenceMemberParams { member: Some(id) })
                .await?;
            Ok(())
        })
    });

    read_back::<_, GetConferenceMembersResponse>(
        client,
        report,
        AREA,
        "fixture:getConferenceMembers",
        &GetConferenceMembersParams::default(),
        |r| Some(r.members.len()),
    )
    .await;
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}

async fn list_conference_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetConferenceResponse = client
        .get_conference(&GetConferenceParams::default())
        .await?;
    Ok(resp
        .conference
        .into_iter()
        .filter(|c| owned(&c.description))
        .filter_map(|c| {
            c.conference.map(|id| Orphan {
                label: format!("conference id={id}"),
                id: id as i64,
            })
        })
        .collect())
}

async fn del_conference(client: &Client, id: i64) -> anyhow::Result<()> {
    client
        .del_conference(&DelConferenceParams {
            conference: Some(id),
        })
        .await?;
    Ok(())
}

async fn list_member_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetConferenceMembersResponse = client
        .get_conference_members(&GetConferenceMembersParams::default())
        .await?;
    Ok(resp
        .members
        .into_iter()
        .filter(|m| owned(&m.description))
        .filter_map(|m| {
            m.member.map(|id| Orphan {
                label: format!("conferencemember id={id}"),
                id: id as i64,
            })
        })
        .collect())
}

async fn del_member(client: &Client, id: i64) -> anyhow::Result<()> {
    client
        .del_conference_member(&DelConferenceMemberParams { member: Some(id) })
        .await?;
    Ok(())
}
