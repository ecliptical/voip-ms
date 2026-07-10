//! The area registry and selection resolution.

pub mod reference;

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use crate::config::AreaSelection;
use crate::harness::area::{Area, CostClass};

/// Every area the harness knows about. Adding an area here (and it declaring
/// its `methods()`) is what makes it selectable and covered.
pub fn registry() -> Vec<Box<dyn Area>> {
    vec![Box::new(reference::Reference)]
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

    /// Every method an area claims must be a real wire method, and no area may
    /// claim the same method as another (each method has one owning area).
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
}
