//! The functional-area abstraction.
//!
//! The 222 API methods partition into functional areas mirroring the API's
//! resource domains. Each area is independently selectable (`--areas`) so a
//! costly-by-nature area (porting, e911, reseller, charges) runs only when
//! named -- "once in a while" -- while common areas run every time. An area
//! owns its probe list, its fixtures, and its sweep, keeping a resource's whole
//! lifecycle in one place.

use async_trait::async_trait;

use crate::config::{Config, Depth};
use crate::harness::Report;
use crate::harness::ledger::Ledger;
use crate::harness::marker::RunToken;
use voip_ms::Client;

/// Whether an area involves money or irreversible state by its nature, and so
/// is excluded from the default selection until explicitly named.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostClass {
    /// No method in the area costs money or is irreversible at any depth.
    Free,
    /// The area contains money/irreversible methods (reachable only at
    /// `Depth::Costly`); excluded from the default area set.
    CostlyByNature,
}

/// Per-run context handed to each area operation.
pub struct AreaCtx<'a> {
    pub client: &'a Client,
    pub depth: Depth,
    pub token: &'a RunToken,
    pub ledger: &'a Ledger,
    /// Runtime config, for the `--depth costly` fixture inputs
    /// (`--test-did`/`--sms-dst`/`--mms-media-url`/`--order-test-did`/the DID
    /// search fields) that only a handful of areas consult.
    pub config: &'a Config,
}

/// A functional area of the API surface.
#[async_trait(?Send)]
pub trait Area {
    /// Stable selection name (e.g. `"reference"`, `"voicemail"`).
    fn name(&self) -> &'static str;

    /// Cost classification, which governs default inclusion.
    fn cost_class(&self) -> CostClass;

    /// Every wire method name this area is responsible for (typed method
    /// coverage for the completeness assertion). PascalCase wire names, e.g.
    /// `"getCountries"`.
    fn methods(&self) -> &'static [&'static str];

    /// Read-only probes: call each owned `get*`/reference method typed, with
    /// raw-vs-typed drift diffing. Always safe; runs at every depth.
    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report);

    /// Reconcile leftover fixtures from prior runs for this area only.
    /// Enumerate marker-bearing objects, report them, delete dependency-ordered,
    /// then re-confirm. Default: nothing to reconcile (probe-only areas).
    async fn sweep(&self, _ctx: &AreaCtx<'_>, _report: &mut Report) -> SweepResult {
        SweepResult::clean()
    }

    /// Create -> read-typed -> delete fixtures (free at `Lifecycle`, costly at
    /// `Costly`). Default: none (probe-only areas).
    async fn run_fixtures(&self, _ctx: &AreaCtx<'_>, _report: &mut Report) {}
}

/// Outcome of an area's pre-flight sweep.
pub struct SweepResult {
    /// Marker-bearing orphans that remained after the delete+reconfirm pass.
    /// Non-empty means the harness cannot guarantee a clean slate.
    pub unreconciled: Vec<String>,
}

impl SweepResult {
    pub fn clean() -> Self {
        SweepResult {
            unreconciled: Vec::new(),
        }
    }

    pub fn is_clean(&self) -> bool {
        self.unreconciled.is_empty()
    }
}
