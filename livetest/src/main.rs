//! Live VoIP.ms API integration harness (operator-local, never published).
//!
//! Exercises the real API to catch response-shape drift the mock tests can't.
//! Selection is two-dimensional -- functional AREAS and DEPTH -- and a
//! pre-flight sweep guarantees a clean slate before any fixtures run. See
//! `--help` and the design notes in the plan.

// The harness framework (Scope, ownership markers, ledger, sweep hooks) is
// built ahead of the fixtures and costly areas that consume it. Until those
// land, the framework surface reads as dead code; the allowance keeps the
// scaffolding intact rather than deleting pieces later phases depend on.
#![allow(dead_code)]

mod areas;
mod config;
mod harness;
mod wire_methods;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use config::{Cli, Config, Depth};
use harness::area::AreaCtx;
use harness::ledger::{self, Ledger};
use harness::marker::RunToken;
use harness::{ProbeOutcome, Report, probe};
use voip_ms::{GetIPParams, GetIPResponse};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            // anyhow's chain, without leaking secrets (Config redacts; other
            // errors here are structural).
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Returns `Ok(true)` when every check passed, `Ok(false)` when some failed or
/// drifted (a normal non-zero exit), `Err` for a setup problem that stops the
/// run before it starts.
async fn run() -> Result<bool> {
    let cli = Cli::parse();

    if cli.list_areas {
        println!("Available areas:\n{}", areas::describe());
        return Ok(true);
    }

    let config = Config::from_cli(cli)?;
    let server = if config.api_url.is_some() {
        "custom"
    } else {
        "voip.ms (default)"
    };
    let auth = if config.basic_auth.is_some() {
        " +basic-auth"
    } else {
        ""
    };
    println!(
        "livetest: depth={:?}, server={server}{auth}, ledger={}",
        config.depth,
        ledger::describe_path(&config.ledger_path),
    );

    let selected = areas::resolve(&config.area_selection)?;
    if selected.is_empty() {
        println!("no areas selected; nothing to do (see --list-areas)");
        return Ok(true);
    }

    println!(
        "selected areas: {}",
        selected
            .iter()
            .map(|a| a.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let client = config.build_client()?;
    confirm_connectivity(&client).await?;

    let token = RunToken::new();
    let ledger = Ledger::open(config.ledger_path.clone(), &config.username);

    let mut report = Report::default();

    // Pre-flight sweep, area-scoped: reconcile leftover fixtures from prior runs
    // before creating any new ones. Abort if any area can't reach a clean slate.
    for area in &selected {
        let ctx = AreaCtx {
            client: &client,
            depth: config.depth,
            token: &token,
            ledger: &ledger,
            config: &config,
        };

        let result = area.sweep(&ctx, &mut report).await;
        if !result.is_clean() {
            anyhow::bail!(
                "pre-flight sweep for area `{}` left {} unreconciled orphan(s): {} -- \
                 refusing to create new fixtures on an un-clean account",
                area.name(),
                result.unreconciled.len(),
                result.unreconciled.join(", ")
            );
        }
    }

    // Probe every selected area (read-only, all depths).
    for area in &selected {
        let ctx = AreaCtx {
            client: &client,
            depth: config.depth,
            token: &token,
            ledger: &ledger,
            config: &config,
        };

        area.probe(&ctx, &mut report).await;

        // Fixtures at lifecycle+ depth.
        if config.depth.at_least(Depth::Lifecycle) {
            area.run_fixtures(&ctx, &mut report).await;
        }
    }

    print_summary(&report);
    Ok(!report.is_failure())
}

/// Confirm the client can reach the API and (via the proxy, if any) presents an
/// allow-listed source IP. `getIP` is the one method that works without an
/// allow-listed IP, so a failure here is a proxy/credential problem, surfaced
/// before any real work.
async fn confirm_connectivity(client: &voip_ms::Client) -> Result<()> {
    let outcome =
        probe::<GetIPParams, GetIPResponse>(client, "getIP", &GetIPParams::default(), |_| None)
            .await;

    match outcome {
        ProbeOutcome::Ok { .. } => {
            println!("[ok] connectivity confirmed via getIP");
            Ok(())
        }
        ProbeOutcome::Drift { error, .. } => {
            // getIP drifting is itself a finding, but connectivity is proven.
            eprintln!("[warn] getIP connectivity ok but response drifted: {error}");
            Ok(())
        }
        ProbeOutcome::ApiError(status) => Err(anyhow::anyhow!(
            "getIP returned API error `{status}` -- check credentials and the \
             account's API allow-list (the proxy egress IP must be allow-listed)"
        )),
        ProbeOutcome::Transport(error) => {
            Err(anyhow::anyhow!(error)).context("getIP transport failure -- check the proxy")
        }
    }
}

fn print_summary(report: &Report) {
    let c = report.counts();
    println!(
        "\n== summary == pass={} fail={} skip={} drift={}",
        c.pass, c.fail, c.skip, c.drift
    );
    println!("{}", report.summary_json());

    if c.drift > 0 {
        eprintln!(
            "\n{} response-shape drift(s) detected -- fix via tools/api-response-overrides.json \
             and regenerate (see DEVELOPMENT.md).",
            c.drift
        );
    }
}
