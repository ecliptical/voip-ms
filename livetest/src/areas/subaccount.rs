//! The `subaccount` area: sub-accounts, their SIP URIs, and the account's
//! configured locations. The sub-account, SIP-URI, and location lists probe
//! cleanly; the registration-status read needs a specific account, so it is
//! skipped at probe depth.
//!
//! At `Lifecycle` depth the area runs two independent create -> read -> delete
//! fixtures -- a sub-account (marker in its `lvt`-prefixed username and
//! `description`) and a SIP URI (marker in its `description`) -- and its
//! [`sweep`](Area::sweep) reclaims marker-bearing leftovers of both from prior
//! runs.

use async_trait::async_trait;

use crate::areas::probe_macros::{probe_list, skip_needs_input};
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{Orphan, owned, read_back, sweep_orphans, tolerate_absent};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Subaccount;

const AREA: &str = "subaccount";

#[async_trait(?Send)]
impl Area for Subaccount {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "createSubAccount",
            "delLocation",
            "delSIPURI",
            "delSubAccount",
            "getLocations",
            "getRegistrationStatus",
            "getSIPURIs",
            "getSubAccounts",
            "setLocation",
            "setSIPURI",
            "setSubAccount",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getLocations",
            GetLocationsParams,
            GetLocationsResponse,
            locations
        );
        skip_needs_input!(
            report,
            AREA,
            "getRegistrationStatus",
            "requires a specific account"
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getSIPURIs",
            GetSIPURIsParams,
            GetSIPURIsResponse,
            sip_uris
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getSubAccounts",
            GetSubAccountsParams,
            GetSubAccountsResponse,
            accounts
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;

        let subaccounts = sweep_orphans(
            report,
            AREA,
            "subaccount",
            || list_subaccount_orphans(client),
            |id| del_subaccount(client, id),
        )
        .await;

        let sipuris = sweep_orphans(
            report,
            AREA,
            "sipuri",
            || list_sipuri_orphans(client),
            |id| del_sipuri(client, id),
        )
        .await;

        let mut unreconciled = subaccounts.unreconciled;
        unreconciled.extend(sipuris.unreconciled);
        SweepResult { unreconciled }
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        subaccount_fixture(ctx, report, &mut scope).await;
        sipuri_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

/// Create -> read-back -> (deferred) delete a sub-account. Ports the old
/// `verify_subaccount_lifecycle` example onto the fixtures harness: the marker
/// rides in both the `lvt`-prefixed username and the `description`, and the
/// `id` returned by create is what identifies it for teardown.
async fn subaccount_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let username = ctx.token.username(0);
    let description = ctx.token.marker(0);
    let password = format!("Lv{}Pw1", ctx.token.as_str());

    // The remaining `(required)` fields take "Values from get*" codes that vary
    // by account; these are the conventional defaults (code 1, `ulaw`, `none`
    // music-on-hold) -- best-effort until a live run confirms them. A rejection
    // surfaces as a Fail naming the offending field's status.
    let created = client
        .create_sub_account(&CreateSubAccountParams {
            username: Some(username),
            protocol: Some(1),
            description: Some(description),
            auth_type: Some(1),
            password: Some(password),
            device_type: Some(1),
            lock_international: Some(1),
            international_route: Some(1),
            // `music_on_hold` is a class from getMusicOnHold; `default` ("No
            // Music") is the always-present baseline. `none` is rejected
            // (`invalid_musiconhold`).
            music_on_hold: Some("default".into()),
            allowed_codecs: Some("ulaw".into()),
            dtmf_mode: Some(DtmfMode::Auto),
            nat: Some(Nat::Yes),
            ..Default::default()
        })
        .await;

    // `getSubAccounts`'s `account` filter takes the full `<main>_<sub>` name
    // (e.g. `100000_lvt...`) that create returns in `account`, not the bare
    // `username` we sent; scoping the read-back by it keeps the response to the
    // one row so the element deserializers are actually exercised.
    let (id, account) = match created {
        Ok(resp) => match (resp.id, resp.account) {
            (Some(id), Some(account)) => (id, account),
            _ => {
                report.record(
                    AREA,
                    "fixture:createSubAccount",
                    Outcome::Fail(
                        "createSubAccount succeeded without an id and account".to_string(),
                    ),
                );
                return;
            }
        },
        Err(error) => {
            report.record(
                AREA,
                "fixture:createSubAccount",
                Outcome::Fail(format!("createSubAccount: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:createSubAccount", Outcome::Pass);
    scope.defer(format!("subaccount id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_sub_account(&DelSubAccountParams { id: Some(id) })
                    .await,
            )
        })
    });

    read_back::<_, GetSubAccountsResponse>(
        client,
        report,
        AREA,
        "fixture:getSubAccounts",
        &GetSubAccountsParams {
            account: Some(account),
        },
        |r| Some(r.accounts.len()),
    )
    .await;
}

/// Create -> read-back -> (deferred) delete a SIP URI, marker in `description`.
async fn sipuri_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let description = ctx.token.marker(1);

    let created = client
        .set_sip_uri(&SetSIPURIParams {
            uri: Some(format!("sip:{}@example.invalid", ctx.token.as_str())),
            description: Some(description.clone()),
            ..Default::default()
        })
        .await;

    // setSIPURI returns only `status` -- the new id is not in the response, so
    // the created object is looked up by its marker for both read-back and
    // teardown.
    if let Err(error) = created {
        report.record(
            AREA,
            "fixture:setSIPURI",
            Outcome::Fail(format!("setSIPURI: {error}")),
        );
        return;
    }

    let id = match find_sipuri_by_marker(client, &description).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            report.record(
                AREA,
                "fixture:setSIPURI",
                Outcome::Fail("created SIP URI not found by its marker".to_string()),
            );
            return;
        }
        Err(error) => {
            report.record(
                AREA,
                "fixture:setSIPURI",
                Outcome::Fail(format!("locating created SIP URI: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:setSIPURI", Outcome::Pass);
    scope.defer(format!("sipuri id={id}"), move |client| {
        Box::pin(async move {
            tolerate_absent(
                client
                    .del_sip_uri(&DelSIPURIParams { sip_uri: Some(id) })
                    .await,
            )
        })
    });

    read_back::<_, GetSIPURIsResponse>(
        client,
        report,
        AREA,
        "fixture:getSIPURIs",
        &GetSIPURIsParams::default(),
        |r| Some(r.sip_uris.len()),
    )
    .await;
}

async fn find_sipuri_by_marker(client: &Client, marker: &str) -> anyhow::Result<Option<u64>> {
    let resp: GetSIPURIsResponse = client.get_sip_uris(&GetSIPURIsParams::default()).await?;
    Ok(resp
        .sip_uris
        .into_iter()
        .find(|s| s.description.as_deref() == Some(marker))
        .and_then(|s| s.sip_uri))
}

async fn list_subaccount_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetSubAccountsResponse = client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await?;
    Ok(resp
        .accounts
        .into_iter()
        .filter(|a| {
            owned(&a.description)
                || a.username
                    .as_deref()
                    .is_some_and(crate::harness::marker::is_owned_username)
        })
        .filter_map(|a| {
            a.id.map(|id| Orphan {
                label: format!("subaccount {}", a.account.as_deref().unwrap_or("?")),
                id,
            })
        })
        .collect())
}

async fn del_subaccount(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_sub_account(&DelSubAccountParams { id: Some(id) })
        .await?;
    Ok(())
}

async fn list_sipuri_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetSIPURIsResponse = client.get_sip_uris(&GetSIPURIsParams::default()).await?;
    Ok(resp
        .sip_uris
        .into_iter()
        .filter(|s| owned(&s.description))
        .filter_map(|s| {
            s.sip_uri.map(|id| Orphan {
                label: format!("sipuri {}", s.uri.as_deref().unwrap_or("?")),
                id,
            })
        })
        .collect())
}

async fn del_sipuri(client: &Client, id: u64) -> anyhow::Result<()> {
    client
        .del_sip_uri(&DelSIPURIParams { sip_uri: Some(id) })
        .await?;
    Ok(())
}
