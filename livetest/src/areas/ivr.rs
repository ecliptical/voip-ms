//! The `ivr` area: interactive voice response menus. The list read probes
//! cleanly; at `Lifecycle` depth the area runs a create -> read -> delete
//! fixture over an IVR (marker in its `name`) and its [`sweep`](Area::sweep)
//! reclaims marker-bearing menus from prior runs.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::area::{Area, AreaCtx, CostClass, SweepResult};
use crate::harness::fixtures::{
    Orphan, owned, owned_orphans, read_back, sweep_orphans, tolerate_absent,
};
use crate::harness::scope::Scope;
use crate::harness::{Outcome, Report};
use voip_ms::*;

pub struct Ivr;

const AREA: &str = "ivr";

#[async_trait(?Send)]
impl Area for Ivr {
    fn name(&self) -> &'static str {
        AREA
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &["delIVR", "getIVRs", "setIVR"]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        probe_list!(
            ctx,
            report,
            AREA,
            "getIVRs",
            GetIVRsParams,
            GetIVRsResponse,
            ivrs
        );
    }

    async fn sweep(&self, ctx: &AreaCtx<'_>, report: &mut Report) -> SweepResult {
        let client = ctx.client;
        sweep_orphans(
            report,
            AREA,
            "ivr",
            || list_orphans(client),
            |id| del_ivr(client, id),
        )
        .await
    }

    async fn run_fixtures(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        let mut scope = Scope::new();
        ivr_fixture(ctx, report, &mut scope).await;

        for label in scope.cleanup(ctx.client).await {
            report.record(
                AREA,
                "cleanup",
                Outcome::Fail(format!("teardown failed for {label}")),
            );
        }
    }
}

async fn ivr_fixture(ctx: &AreaCtx<'_>, report: &mut Report, scope: &mut Scope) {
    let client = ctx.client;
    let name = ctx.token.marker(0);

    // `recording` is `(required)` and must be a real recording id ("Values from
    // getRecordings"); a nonexistent id is rejected (`invalid_recording`), so
    // discover one and skip if the account has none. `voicemailsetup` code 1 and
    // a single hangup choice are the conventional defaults.
    //
    // Marker-bearing recordings are passed over: `callflow` creates one, and
    // an IVR referencing it makes `delRecording` refuse, which fails that
    // area's sweep and aborts the run before `ivr`'s sweep can free it.
    let recording = match client.get_recordings(&GetRecordingsParams::default()).await {
        Ok(resp) => borrowable_recording(resp.recordings),
        // `no_recording` deserializes as an empty list on some paths; treat any
        // read failure as "none discoverable" rather than a hard error here.
        Err(_) => None,
    };
    let Some(recording) = recording else {
        report.record(
            AREA,
            "fixture:setIVR",
            Outcome::Skip("requires an existing recording".to_string()),
        );
        return;
    };

    let created = client
        .set_ivr(&SetIVRParams {
            name: Some(name),
            timeout: Some(10),
            language: Some("en".to_string()),
            recording: Some(recording),
            voicemailsetup: Some(1),
            choices: Some("1=sys:hangup".to_string()),
            ..Default::default()
        })
        .await;

    let id = match created {
        Ok(resp) => match resp.ivr {
            Some(id) => id,
            None => {
                report.record(
                    AREA,
                    "fixture:setIVR",
                    Outcome::Fail("setIVR succeeded without an id".to_string()),
                );
                return;
            }
        },
        Err(error) => {
            report.record(
                AREA,
                "fixture:setIVR",
                Outcome::Fail(format!("setIVR: {error}")),
            );
            return;
        }
    };

    report.record(AREA, "fixture:setIVR", Outcome::Pass);
    scope.defer(format!("ivr id={id}"), move |client| {
        Box::pin(
            async move { tolerate_absent(client.del_ivr(&DelIVRParams { ivr: Some(id) }).await) },
        )
    });

    read_back::<_, GetIVRsResponse>(
        client,
        report,
        AREA,
        "fixture:getIVRs",
        &GetIVRsParams::default(),
        |r| Some(r.ivrs.len()),
    )
    .await;
}

/// The id of a recording the fixture may point an IVR at: one the account
/// already had, never one this harness created.
///
/// Filtering and id-taking are separate passes on purpose. A single `find`
/// stops at the first record it looks at, so an unowned recording reporting no
/// id would end the search rather than be passed over -- which is how this
/// regressed once already.
fn borrowable_recording(
    listed: impl IntoIterator<Item = GetRecordingsResponseRecording>,
) -> Option<u64> {
    listed
        .into_iter()
        .filter(|r| !owned(&r.description))
        .find_map(|r| r.value)
}

async fn list_orphans(client: &Client) -> anyhow::Result<Vec<Orphan>> {
    let resp: GetIVRsResponse = client.get_ivrs(&GetIVRsParams::default()).await?;
    Ok(owned_orphans(resp.ivrs, "ivr", |i| &i.name, |i| i.ivr))
}

async fn del_ivr(client: &Client, id: u64) -> anyhow::Result<()> {
    client.del_ivr(&DelIVRParams { ivr: Some(id) }).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::harness::marker::RunToken;

    /// One listed recording. Built through the wire shape because a `*Response`
    /// record has no `Default` -- a received value is never manufactured.
    fn recording(description: Option<&str>, value: Option<u64>) -> GetRecordingsResponseRecording {
        serde_json::from_value(serde_json::json!({
            "status": "success",
            "description": description,
            "value": value,
        }))
        .expect("a recording record")
    }

    #[test]
    fn an_unowned_recording_with_no_id_does_not_end_the_search() {
        // The regression this guards: the first record is one the fixture may
        // borrow but cannot name, and the usable recording is behind it.
        let listed = vec![
            recording(Some("customer greeting"), None),
            recording(Some("main menu"), Some(42)),
        ];

        assert_eq!(borrowable_recording(listed), Some(42));
    }

    #[test]
    fn a_marker_bearing_recording_is_never_borrowed() {
        // Pointing an IVR at callflow's own recording makes `delRecording`
        // refuse, which fails that area's sweep and aborts the run.
        let marker = RunToken::new().short_marker(0);
        let listed = vec![
            recording(Some(&marker), Some(1)),
            recording(Some("main menu"), Some(2)),
        ];

        assert_eq!(borrowable_recording(listed), Some(2));
    }

    #[test]
    fn an_account_with_nothing_to_borrow_yields_none() {
        assert_eq!(borrowable_recording(Vec::new()), None);
        assert_eq!(
            borrowable_recording(vec![recording(Some("no id"), None)]),
            None
        );
    }
}
