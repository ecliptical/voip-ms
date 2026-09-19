//! Shared probe macros so each area's `probe()` reads as one line per method.
//!
//! The macros here:
//!   * [`probe_list!`] -- a response whose payload is a single list; the count
//!     is that list's length, matching the reference area's original macro.
//!   * [`probe_zoned_list!`] -- the same, for a list whose records carry a
//!     timestamp reported in the UTC offset the request asked for. `sms` and
//!     `mms` probe with it.
//!   * [`probe_scalar!`] -- a scalar/object response, or one with several lists
//!     where no single count is meaningful; nothing to count.
//!   * [`skip_needs_input!`] -- a method whose required input (a resource id, a
//!     date window) can't be supplied without a fixture, so probing it on an
//!     empty account would only ever record an API error. Skipped at probe
//!     depth with a reason rather than reported as a failure.
//!
//! Not every area probes through a macro: `reseller` has its own
//! `probe_reseller`, which folds `invalid_client` into a Skip, and `cdr` skips
//! at probe depth and reads back at costly depth instead.

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

/// Probe a record-listing method: as [`probe_list!`], with `$timestamps` the
/// crate's `*_TIMESTAMPS` const for the method, naming the response timestamps
/// that come back in the offset the request carried. UTC, so a reported wall
/// clock is the instant it names.
macro_rules! probe_zoned_list {
    ($ctx:expr, $report:expr, $area:expr, $wire:literal, $params:ty, $resp:ty, $field:ident, $timestamps:expr) => {{
        let outcome = $crate::harness::probe_zoned_default::<$params, $resp>(
            $ctx.client,
            $wire,
            $timestamps,
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

pub(crate) use {probe_list, probe_scalar, probe_zoned_list, skip_needs_input};
