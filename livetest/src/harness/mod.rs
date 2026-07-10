//! Harness core: outcome accumulation, RAII cleanup, and the raw-vs-typed
//! drift probe that is the whole point of the harness.

pub mod area;
pub mod ledger;
pub mod marker;
pub mod probe;
pub mod scope;

pub use probe::{ProbeOutcome, probe};

use std::fmt::Write as _;

/// The classification of a single check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Pass,
    /// A non-drift failure: network error, real API error, or a fixture step
    /// that failed for a reason other than response-shape drift.
    Fail(String),
    /// Not run, with a reason (e.g. missing fixture input, depth too shallow).
    Skip(String),
    /// The drift signal: the raw call succeeded but the typed deserialization
    /// failed. Carries the serde error and the raw JSON to paste into an
    /// override fix.
    Drift {
        error: String,
        raw_json: String,
    },
}

/// One recorded check: an area, a method/label, and its outcome.
#[derive(Clone, Debug)]
pub struct Record {
    pub area: String,
    pub name: String,
    pub outcome: Outcome,
}

/// Accumulates per-check outcomes so one failure never stops the rest of the
/// run, and renders a final summary. Supersedes the old example's `Checks`.
#[derive(Default)]
pub struct Report {
    records: Vec<Record>,
}

impl Report {
    pub fn record(&mut self, area: &str, name: &str, outcome: Outcome) {
        match &outcome {
            Outcome::Pass => println!("[pass] {area}/{name}"),
            Outcome::Skip(reason) => println!("[skip] {area}/{name}: {reason}"),
            Outcome::Fail(error) => eprintln!("[fail] {area}/{name}: {error}"),
            Outcome::Drift { error, raw_json } => {
                eprintln!("[DRIFT] {area}/{name}: typed deserialization failed but raw succeeded");
                eprintln!("        serde error: {error}");
                eprintln!("        raw JSON:");
                for line in raw_json.lines() {
                    eprintln!("          {line}");
                }
            }
        }

        self.records.push(Record {
            area: area.to_string(),
            name: name.to_string(),
            outcome,
        });
    }

    /// Fold a [`ProbeOutcome`] into the report under `area`/`method`.
    pub fn record_probe(&mut self, area: &str, method: &str, outcome: ProbeOutcome) {
        let outcome = match outcome {
            ProbeOutcome::Ok { element_count } => {
                if let Some(n) = element_count {
                    println!("[info] {area}/{method}: {n} element(s)");
                }

                Outcome::Pass
            }
            ProbeOutcome::Drift { error, raw_json } => Outcome::Drift { error, raw_json },
            ProbeOutcome::ApiError(status) => Outcome::Fail(format!("API error: {status}")),
            ProbeOutcome::Transport(error) => Outcome::Fail(format!("transport: {error}")),
        };

        self.record(area, method, outcome);
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for r in &self.records {
            match r.outcome {
                Outcome::Pass => c.pass += 1,
                Outcome::Fail(_) => c.fail += 1,
                Outcome::Skip(_) => c.skip += 1,
                Outcome::Drift { .. } => c.drift += 1,
            }
        }

        c
    }

    /// Non-zero exit is warranted when anything failed or drifted; skips are
    /// not failures.
    pub fn is_failure(&self) -> bool {
        let c = self.counts();
        c.fail > 0 || c.drift > 0
    }

    /// One machine-readable JSON line for a future CI wrapper to parse without
    /// scraping stdout. Contains no secrets (method names and counts only).
    pub fn summary_json(&self) -> String {
        let c = self.counts();
        let mut drifted = String::new();
        let mut failed = String::new();
        for r in &self.records {
            match &r.outcome {
                Outcome::Drift { .. } => {
                    let _ = write!(drifted, "{}\"{}/{}\"", sep(&drifted), r.area, r.name);
                }
                Outcome::Fail(_) => {
                    let _ = write!(failed, "{}\"{}/{}\"", sep(&failed), r.area, r.name);
                }
                _ => {}
            }
        }

        format!(
            "{{\"summary\":{{\"pass\":{},\"fail\":{},\"skip\":{},\"drift\":{}}},\
             \"drifted\":[{drifted}],\"failed\":[{failed}]}}",
            c.pass, c.fail, c.skip, c.drift
        )
    }
}

fn sep(buf: &str) -> &'static str {
    if buf.is_empty() { "" } else { "," }
}

/// Tally of outcomes by kind.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub pass: usize,
    pub fail: usize,
    pub skip: usize,
    pub drift: usize,
}
