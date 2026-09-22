//! Harness core: outcome accumulation, RAII cleanup, and the raw-vs-typed
//! drift probe that is the whole point of the harness.

pub mod area;
pub mod fixtures;
pub mod keydiff;
pub mod ledger;
pub mod marker;
pub mod probe;
pub mod scope;

pub use probe::{ProbeOutcome, probe, probe_zoned_default};

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
    /// The fidelity signal: the call succeeded and deserialized, but the
    /// response carried keys no `*Response` field claims, so the typed surface
    /// dropped them. Carries the paths in the override file's grammar.
    Unmodeled {
        paths: Vec<String>,
    },
    /// The tolerance signal: the call deserialized, but a value landed in a
    /// catch-all variant, so the crate read the envelope without understanding
    /// one of its values. Carries the rendered variants.
    ///
    /// This is the signal [`Outcome::Drift`] used to carry for these values.
    /// Once a type tolerates what it cannot parse, the typed read stops failing
    /// -- which is the point, and which would leave the harness passing on the
    /// exact input that made it necessary.
    Degraded {
        values: Vec<String>,
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
            Outcome::Degraded { values } => {
                eprintln!(
                    "[DEGRADED] {area}/{name}: {} value(s) the typed shape could not read",
                    values.len()
                );
                eprintln!(
                    "        the envelope survived, so this is what tolerance caught \
                     rather than what it hid:"
                );
                for value in values {
                    eprintln!("          {value}");
                }
            }
            Outcome::Unmodeled { paths } => {
                eprintln!(
                    "[UNMODELED] {area}/{name}: {} response key(s) the typed shape drops",
                    paths.len()
                );
                eprintln!("        declare them in tools/api-response-overrides.json:");
                // Keyed by the wire method, not the report label: an overrides
                // entry named `fixture:getX` is skipped as an unknown method, so
                // pasting it would look applied and change nothing.
                eprintln!("          \"{}\": {{ \"additions\": [", wire_method(name));
                let entries: Vec<String> = paths
                    .iter()
                    .map(|path| {
                        format!(
                            "            {{ \"path\": {}, \"type\": \"string\" }}",
                            quote(path)
                        )
                    })
                    .collect();
                // Separated, not terminated: serde_json rejects a trailing comma,
                // and this block exists to be pasted verbatim.
                eprintln!("{}", entries.join(",\n"));
                eprintln!("          ] }}");
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
            ProbeOutcome::Ok {
                element_count,
                unmodeled,
                degraded,
            } => {
                if let Some(n) = element_count {
                    println!("[info] {area}/{method}: {n} element(s)");
                }

                // Degraded outranks unmodeled: a value the crate could not read
                // is a live wire change, where a dropped key may be one the
                // docs never listed.
                if !degraded.is_empty() {
                    Outcome::Degraded { values: degraded }
                } else if unmodeled.is_empty() {
                    Outcome::Pass
                } else {
                    Outcome::Unmodeled { paths: unmodeled }
                }
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
                Outcome::Unmodeled { .. } => c.unmodeled += 1,
                Outcome::Degraded { .. } => c.degraded += 1,
            }
        }

        c
    }

    /// Non-zero exit is warranted when anything failed, drifted, came back with
    /// keys the typed surface drops, or carried a value it could not read;
    /// skips are not failures.
    pub fn is_failure(&self) -> bool {
        let c = self.counts();
        c.fail > 0 || c.drift > 0 || c.unmodeled > 0 || c.degraded > 0
    }

    /// One machine-readable JSON line for a future CI wrapper to parse without
    /// scraping stdout. Contains no secrets (method names and counts only).
    pub fn summary_json(&self) -> String {
        let c = self.counts();
        let mut drifted = String::new();
        let mut failed = String::new();
        let mut unmodeled = String::new();
        let mut degraded = String::new();
        for r in &self.records {
            match &r.outcome {
                Outcome::Drift { .. } => {
                    let _ = write!(drifted, "{}\"{}/{}\"", sep(&drifted), r.area, r.name);
                }
                Outcome::Fail(_) => {
                    let _ = write!(failed, "{}\"{}/{}\"", sep(&failed), r.area, r.name);
                }
                Outcome::Degraded { .. } => {
                    let _ = write!(degraded, "{}\"{}/{}\"", sep(&degraded), r.area, r.name);
                }
                Outcome::Unmodeled { paths } => {
                    for path in paths {
                        // Quoted through serde: a path segment is a live
                        // response key, so it can carry a `"` or a `\` that
                        // would otherwise break the line a CI wrapper parses.
                        let _ = write!(
                            unmodeled,
                            "{}{}",
                            sep(&unmodeled),
                            quote(&format!("{}.{path}", wire_method(&r.name))),
                        );
                    }
                }
                _ => {}
            }
        }

        format!(
            "{{\"summary\":{{\"pass\":{},\"fail\":{},\"skip\":{},\"drift\":{},\
             \"unmodeled\":{},\"degraded\":{}}},\"drifted\":[{drifted}],\
             \"failed\":[{failed}],\"unmodeled\":[{unmodeled}],\
             \"degraded\":[{degraded}]}}",
            c.pass, c.fail, c.skip, c.drift, c.unmodeled, c.degraded
        )
    }
}

fn sep(buf: &str) -> &'static str {
    if buf.is_empty() { "" } else { "," }
}

/// The wire method behind a report key. A fixture read-back is reported under
/// `fixture:getX` but calls `getX`, and only the latter names anything the
/// overrides file or a CI wrapper can act on.
fn wire_method(name: &str) -> &str {
    name.strip_prefix("fixture:").unwrap_or(name)
}

/// A JSON string literal for `value`, escaping whatever a live response key
/// happens to contain.
fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

/// Tally of outcomes by kind.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub pass: usize,
    pub fail: usize,
    pub skip: usize,
    pub drift: usize,
    pub unmodeled: usize,
    pub degraded: usize,
}
