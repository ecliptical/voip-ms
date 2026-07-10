//! Shared probe macros so each area's `probe()` reads as one line per method.
//!
//! Three shapes cover every read-only method:
//!   * [`probe_list!`] -- a response whose payload is a single list; the count
//!     is that list's length, matching the reference area's original macro.
//!   * [`probe_scalar!`] -- a scalar/object response, or one with several lists
//!     where no single count is meaningful; nothing to count.
//!   * [`skip_needs_input!`] -- a method whose required input (a resource id, a
//!     date window) can't be supplied without a fixture, so probing it on an
//!     empty account would only ever record an API error. Skipped at probe
//!     depth with a reason rather than reported as a failure.

/// Probe a list method: call typed-over-raw with default params and count the
/// single list field.
macro_rules! probe_list {
    ($ctx:expr, $report:expr, $area:expr, $wire:literal, $params:ty, $resp:ty, $field:ident) => {{
        let outcome = $crate::harness::probe::<$params, $resp>(
            $ctx.client,
            $wire,
            &<$params>::default(),
            |r| Some(r.$field.len()),
        )
        .await;
        $report.record_probe($area, $wire, outcome);
    }};
}

/// Probe a scalar/object (or multi-list) method: call typed-over-raw with
/// default params; there is no single count to report.
macro_rules! probe_scalar {
    ($ctx:expr, $report:expr, $area:expr, $wire:literal, $params:ty, $resp:ty) => {{
        let outcome = $crate::harness::probe::<$params, $resp>(
            $ctx.client,
            $wire,
            &<$params>::default(),
            |_| None,
        )
        .await;
        $report.record_probe($area, $wire, outcome);
    }};
}

/// Record a method as skipped at probe depth because it needs an input the
/// harness can't supply without a fixture.
macro_rules! skip_needs_input {
    ($report:expr, $area:expr, $wire:literal, $reason:literal) => {{
        $report.record(
            $area,
            $wire,
            $crate::harness::Outcome::Skip($reason.to_string()),
        );
    }};
}

pub(crate) use {probe_list, probe_scalar, skip_needs_input};
