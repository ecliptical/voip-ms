//! The area registry and selection resolution.

pub mod account;
pub mod callflow;
pub mod cdr;
pub mod conference;
pub mod dids;
pub mod e911;
pub mod fax;
pub mod forwarding;
pub mod ivr;
pub mod mms;
pub mod phonebook;
pub mod porting;
pub mod probe_macros;
pub mod queue;
pub mod reference;
pub mod reseller;
pub mod sms;
pub mod subaccount;
pub mod voicemail;

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use crate::config::AreaSelection;
use crate::harness::area::{Area, CostClass};

/// Every area the harness knows about. Adding an area here (and it declaring
/// its `methods()`) is what makes it selectable and covered.
pub fn registry() -> Vec<Box<dyn Area>> {
    vec![
        Box::new(account::Account),
        Box::new(callflow::Callflow),
        Box::new(cdr::Cdr),
        Box::new(conference::Conference),
        Box::new(dids::Dids),
        Box::new(e911::E911),
        Box::new(fax::Fax),
        Box::new(forwarding::Forwarding),
        Box::new(ivr::Ivr),
        Box::new(mms::Mms),
        Box::new(phonebook::Phonebook),
        Box::new(porting::Porting),
        Box::new(queue::Queue),
        Box::new(reference::Reference),
        Box::new(reseller::Reseller),
        Box::new(sms::Sms),
        Box::new(subaccount::Subaccount),
        Box::new(voicemail::Voicemail),
    ]
}

/// Resolve a [`AreaSelection`] against the registry into the concrete set of
/// areas to run, validating that every named area exists.
pub fn resolve(selection: &AreaSelection) -> Result<Vec<Box<dyn Area>>> {
    let all = registry();
    let known: BTreeSet<&'static str> = all.iter().map(|a| a.name()).collect();

    let (include, exclude): (Option<Vec<String>>, &[String]) = match selection {
        AreaSelection::DefaultFree { exclude } => (None, exclude),
        AreaSelection::All { exclude } => {
            (Some(known.iter().map(|s| s.to_string()).collect()), exclude)
        }
        AreaSelection::Explicit { include, exclude } => (Some(include.clone()), exclude),
    };

    for name in include.iter().flatten().chain(exclude.iter()) {
        if !known.contains(name.as_str()) {
            bail!(
                "unknown area `{name}`; known areas: {}",
                known.iter().copied().collect::<Vec<_>>().join(", ")
            );
        }
    }

    let exclude: BTreeSet<&str> = exclude.iter().map(String::as_str).collect();

    let selected = all
        .into_iter()
        .filter(|area| {
            let name = area.name();
            if exclude.contains(name) {
                return false;
            }

            match &include {
                // Explicit / all: keep exactly what was asked (minus excludes).
                Some(list) => list.iter().any(|n| n == name),
                // Default: every FREE area (costly-by-nature excluded until named).
                None => area.cost_class() == CostClass::Free,
            }
        })
        .collect();

    Ok(selected)
}

/// A one-line listing of areas for `--list-areas`.
pub fn describe() -> String {
    registry()
        .iter()
        .map(|a| {
            let tag = match a.cost_class() {
                CostClass::Free => "free",
                CostClass::CostlyByNature => "costly-by-nature (named-only)",
            };

            format!(
                "  {:<16} {} method(s), {}",
                a.name(),
                a.methods().len(),
                tag
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod completeness {
    use super::*;

    use crate::wire_methods::WIRE_METHODS;

    /// No area may claim the same method as another: each method has exactly
    /// one owning area.
    #[test]
    fn method_names_are_unique_across_areas() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for area in registry() {
            for m in area.methods() {
                assert!(
                    seen.insert(m),
                    "method `{m}` is claimed by more than one area"
                );
            }
        }
    }

    /// Area names are unique and non-empty.
    #[test]
    fn area_names_are_unique() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for area in registry() {
            assert!(!area.name().is_empty(), "empty area name");
            assert!(
                seen.insert(area.name()),
                "duplicate area name `{}`",
                area.name()
            );
        }
    }

    /// The registry partitions the whole API surface exactly: every generated
    /// wire method is owned by exactly one area, and no area claims a name that
    /// isn't a generated wire method. A newly generated method with no owning
    /// area (or a method removed from the crate but still claimed) fails here,
    /// so coverage gaps and stale claims break the build rather than slipping
    /// through. Regenerate `wire_methods.rs` with `cargo xtask dump-methods`
    /// after changing the API surface.
    #[test]
    fn registry_partitions_every_wire_method_exactly_once() {
        let wire: BTreeSet<&str> = WIRE_METHODS.iter().copied().collect();
        assert_eq!(
            wire.len(),
            WIRE_METHODS.len(),
            "wire_methods.rs contains duplicate entries"
        );

        let mut owned: BTreeSet<&str> = BTreeSet::new();
        for area in registry() {
            for m in area.methods() {
                owned.insert(m);
            }
        }

        let unowned: Vec<&&str> = wire.iter().filter(|m| !owned.contains(**m)).collect();
        let unknown: Vec<&&str> = owned.iter().filter(|m| !wire.contains(**m)).collect();

        assert!(
            unowned.is_empty() && unknown.is_empty(),
            "area registry does not partition the wire surface exactly:\n  \
             {} wire method(s) with no owning area: {unowned:?}\n  \
             {} claimed name(s) that are not wire methods: {unknown:?}",
            unowned.len(),
            unknown.len(),
        );

        // With both differences empty and no cross-area duplicates (checked
        // above), the owned set equals the wire set.
        assert_eq!(owned.len(), wire.len());
    }
}
