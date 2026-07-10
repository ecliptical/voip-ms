//! The `phonebook` area: phonebook entries and their groups. Both list reads
//! probe cleanly; at `Lifecycle` depth the area runs create -> read -> delete
//! fixtures over a phonebook group and a phonebook entry (each marker in its
//! `name`) and its [`sweep`](Area::sweep) reclaims marker-bearing leftovers of
//! both.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Phonebook;

const AREA: &str = "phonebook";

#[async_trait(?Send)]
impl Area for Phonebook {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delPhonebook",
            "delPhonebookGroup",
            "getPhonebook",
            "getPhonebookGroups",
            "setPhonebook",
            "setPhonebookGroup",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getPhonebook",
            GetPhonebookParams,
            GetPhonebookResponse,
            phonebooks
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getPhonebookGroups",
            GetPhonebookGroupsParams,
            GetPhonebookGroupsResponse,
            phonebooks
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        let mut unreconciled = Vec::new();

        // Entries can reference a group, so reclaim entries before groups.
        for result in [
            sweep_orphans(
                report,
                AREA,
                "phonebook",
                || list_entry_orphans(client),
                |id| del_entry(client, id),
            )
            .await,
            sweep_orphans(
                report,
                AREA,
                "phonebookgroup",
                || list_group_orphans(client),
                |id| del_group(client, id),
            )
            .await,
        ] {
            unreconciled.extend(result.unreconciled);
        }

        SweepResult { unreconciled }
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        group_fixture(ctx, report, &mut scope).await;
        entry_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn group_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(0);

    let created = client
        .set_phonebook_group(&SetPhonebookGroupParams {
            name: Some(name),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.group {
            Some(id) => id as i64,
            None => return fail(report, "fixture:setPhonebookGroup", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setPhonebookGroup",
                &format!("setPhonebookGroup: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setPhonebookGroup", Outcome::Pass);
    scope.defer(format!("phonebookgroup id={id}"), move |client| {
        Box::pin(async move {
            client
                .del_phonebook_group(&DelPhonebookGroupParams {
                    group: Some(id.to_string()),
                })
                .await?;
            Ok(())
        })
    });

    read_back::<_, GetPhonebookGroupsResponse>(
        client,
        report,
        AREA,
        "fixture:getPhonebookGroups",
        &GetPhonebookGroupsParams::default(),
        |r| Some(r.phonebooks.len()),
    )
    .await;
}

async fn entry_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(1);

    let created = client
        .set_phonebook(&SetPhonebookParams {
            name: Some(name),
            number: Some(15555550102),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.phonebook {
            Some(id) => id as i64,
            None => return fail(report, "fixture:setPhonebook", "no id returned"),
        },
        Err(error) => {
            return fail(
                report,
                "fixture:setPhonebook",
                &format!("setPhonebook: {error}"),
            );
        }
    };

    report.record(AREA, "fixture:setPhonebook", Outcome::Pass);
    scope.defer(format!("phonebook id={id}"), move |client| {
        Box::pin(async move {
            client
                .del_phonebook(&DelPhonebookParams {
                    phonebook: Some(id),
                })
                .await?;
            Ok(())
        })
    });

    read_back::<_, GetPhonebookResponse>(
        client,
        report,
        AREA,
        "fixture:getPhonebook",
        &GetPhonebookParams::default(),
        |r| Some(r.phonebooks.len()),
    )
    .await;
}

fn fail(report: &mut Report, label: &str, error: &str) {
    report.record(AREA, label, Outcome::Fail(error.to_string()));
}

async fn list_entry_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetPhonebookResponse = client.get_phonebook(&GetPhonebookParams::default()).await?;
    Ok(resp
        .phonebooks
        .into_iter()
        .filter(|p| owned(&p.name))
        .filter_map(|p| {
            p.phonebook.map(|id| Orphan {
                label: format!("phonebook id={id}"),
                id: id as i64,
            })
        })
        .collect())
}

async fn del_entry(client: &Client, id: i64) -> anyhow::Result<()> {
    client
        .del_phonebook(&DelPhonebookParams {
            phonebook: Some(id),
        })
        .await?;
    Ok(())
}

async fn list_group_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetPhonebookGroupsResponse = client
        .get_phonebook_groups(&GetPhonebookGroupsParams::default())
        .await?;
    Ok(resp
        .phonebooks
        .into_iter()
        .filter(|g| owned(&g.name))
        .filter_map(|g| {
            g.phonebook_group.map(|id| Orphan {
                label: format!("phonebookgroup id={id}"),
                id: id as i64,
            })
        })
        .collect())
}

async fn del_group(client: &Client, id: i64) -> anyhow::Result<()> {
    client
        .del_phonebook_group(&DelPhonebookGroupParams {
            group: Some(id.to_string()),
        })
        .await?;
    Ok(())
}
