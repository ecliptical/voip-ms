//! WSDL parsing shared by codegen and the response extractor.

use roxmltree::{Document, Node};
use std::collections::BTreeSet;

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

pub struct Wsdl {
    pub operations: Vec<String>,
    pub types: std::collections::HashMap<String, Vec<(String, String)>>,
}

/// A fix to one method's request parameters where the WSDL disagrees with
/// what the server reads.
///
/// The WSDL lags the portal docs, so a disagreement alone is not enough to
/// correct it. An entry needs a second source that agrees with the docs: the
/// PHP client in voip.ms's `API.zip`, which posts each parameter under an
/// explicit key and never reads the WSDL, or a live observation.
struct Correction {
    op: &'static str,
    fix: Fix,
}

enum Fix {
    /// The server reads `to` where the WSDL declares `from`.
    Rename {
        from: &'static str,
        to: &'static str,
    },
    /// The server reads `name`, which the WSDL omits. It is inserted before
    /// `before`, the position the docs list it in.
    Add {
        name: &'static str,
        xsd_type: &'static str,
        before: &'static str,
    },
}

/// The corrections to the committed WSDL. Each way an entry can stop fitting
/// the WSDL is an error naming the fix, so none can linger.
const CORRECTIONS: &[Correction] = &[
    // The docs and the PHP client both send `zip_code`, which is also the name
    // `e911Info` reports the same value under.
    Correction {
        op: "e911Provision",
        fix: ZIP_CODE,
    },
    Correction {
        op: "e911ProvisionManually",
        fix: ZIP_CODE,
    },
    Correction {
        op: "e911Update",
        fix: ZIP_CODE,
    },
    Correction {
        op: "e911Validate",
        fix: ZIP_CODE,
    },
    // The docs mark `portid` required and the PHP client sends it; every other
    // LNP method declares it `xsd:integer`.
    Correction {
        op: "getLNPAttach",
        fix: Fix::Add {
            name: "portid",
            xsd_type: "xsd:integer",
            before: "attachid",
        },
    },
];

const ZIP_CODE: Fix = Fix::Rename {
    from: "zip",
    to: "zip_code",
};

/// Parse the WSDL into its operations and complex types, with
/// [`CORRECTIONS`] applied to the `*Input` types.
pub fn parse_wsdl(text: &str) -> Result<Wsdl, String> {
    let mut wsdl = parse_raw(text)?;
    apply_corrections(&mut wsdl, CORRECTIONS)?;
    Ok(wsdl)
}

fn apply_corrections(wsdl: &mut Wsdl, corrections: &[Correction]) -> Result<(), String> {
    for Correction { op, fix } in corrections {
        let fields = wsdl.types.get_mut(&format!("{op}Input")).ok_or_else(|| {
            format!("WSDL correction for {op}: the WSDL no longer has this method; drop the entry")
        })?;
        let position = |name: &str| fields.iter().position(|(f, _)| f == name);
        match fix {
            Fix::Rename { from, to } => {
                if position(to).is_some() {
                    return Err(format!(
                        "stale WSDL correction for {op}: the WSDL now declares {to}; \
                         drop the rename from {from}"
                    ));
                }

                let i = position(from).ok_or_else(|| {
                    format!(
                        "WSDL correction for {op}: the WSDL no longer declares {from}; \
                         check the server still reads {to} before dropping the rename"
                    )
                })?;
                fields[i].0 = (*to).to_string();
            }
            Fix::Add {
                name,
                xsd_type,
                before,
            } => {
                if position(name).is_some() {
                    return Err(format!(
                        "stale WSDL correction for {op}: the WSDL now declares {name}; \
                         drop the addition"
                    ));
                }

                let i = position(before).ok_or_else(|| {
                    format!(
                        "WSDL correction for {op}: no {before} to insert {name} before; \
                         anchor it on another field"
                    )
                })?;
                fields.insert(i, ((*name).to_string(), (*xsd_type).to_string()));
            }
        }
    }

    Ok(())
}

fn parse_raw(text: &str) -> Result<Wsdl, String> {
    let doc = Document::parse(text).map_err(|e| format!("parse WSDL: {e}"))?;

    let mut types = std::collections::HashMap::new();
    for ct in doc.descendants().filter(|n| is_xsd(n, "complexType")) {
        let Some(name) = ct.attribute("name") else {
            continue;
        };

        let mut fields: Vec<(String, String)> = Vec::new();
        if let Some(seq) = ct.children().find(|n| is_xsd(n, "sequence")) {
            for el in seq.children().filter(|n| is_xsd(n, "element")) {
                if let (Some(fname), Some(ftype)) = (el.attribute("name"), el.attribute("type")) {
                    fields.push((fname.to_string(), ftype.to_string()));
                }
            }
        }

        types.insert(name.to_string(), fields);
    }

    let mut seen = BTreeSet::new();
    for op in doc
        .descendants()
        .filter(|n| n.tag_name().name() == "operation")
    {
        if let Some(name) = op.attribute("name") {
            seen.insert(name.to_string());
        }
    }

    let operations: Vec<String> = seen.into_iter().collect();

    Ok(Wsdl { operations, types })
}

fn is_xsd(n: &Node, local: &str) -> bool {
    n.tag_name().name() == local && n.tag_name().namespace() == Some(XSD_NS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RENAME: Correction = Correction {
        op: "m",
        fix: ZIP_CODE,
    };

    const ADD: Correction = Correction {
        op: "m",
        fix: Fix::Add {
            name: "portid",
            xsd_type: "xsd:integer",
            before: "attachid",
        },
    };

    fn wsdl_with(op: &str, fields: &[&str]) -> Wsdl {
        let fields = fields
            .iter()
            .map(|f| (f.to_string(), "xsd:string".to_string()))
            .collect();
        Wsdl {
            operations: vec![op.to_string()],
            types: [(format!("{op}Input"), fields)].into_iter().collect(),
        }
    }

    fn names(wsdl: &Wsdl, op: &str) -> Vec<String> {
        wsdl.types[&format!("{op}Input")]
            .iter()
            .map(|(f, _)| f.clone())
            .collect()
    }

    fn error(op: &str, fields: &[&str], correction: Correction) -> String {
        apply_corrections(&mut wsdl_with(op, fields), &[correction]).unwrap_err()
    }

    #[test]
    fn rename_replaces_the_field_in_place() {
        let mut wsdl = wsdl_with("m", &["a", "zip", "b"]);
        apply_corrections(&mut wsdl, &[RENAME]).unwrap();
        assert_eq!(names(&wsdl, "m"), ["a", "zip_code", "b"]);
    }

    #[test]
    fn add_inserts_before_its_anchor_with_its_type() {
        let mut wsdl = wsdl_with("m", &["a", "attachid"]);
        apply_corrections(&mut wsdl, &[ADD]).unwrap();
        assert_eq!(names(&wsdl, "m"), ["a", "portid", "attachid"]);
        assert_eq!(wsdl.types["mInput"][1].1, "xsd:integer");
    }

    #[test]
    fn a_rename_the_wsdl_caught_up_with_is_stale() {
        // With `zip` still declared, only the `to` check stops the rename from
        // producing a second `zip_code`.
        let e = error("m", &["zip", "zip_code"], RENAME);
        assert!(e.contains("stale") && e.contains("drop the rename"), "{e}");
        let e = error("m", &["zip_code"], RENAME);
        assert!(e.contains("stale") && e.contains("drop the rename"), "{e}");
    }

    #[test]
    fn a_rename_whose_source_is_gone_is_not_called_stale() {
        let e = error("m", &["a"], RENAME);
        assert!(
            e.contains("no longer declares zip") && !e.contains("stale"),
            "{e}"
        );
    }

    #[test]
    fn an_addition_the_wsdl_caught_up_with_is_stale() {
        let e = error("m", &["portid", "attachid"], ADD);
        assert!(
            e.contains("stale") && e.contains("drop the addition"),
            "{e}"
        );
    }

    #[test]
    fn an_addition_whose_anchor_is_gone_asks_for_a_new_one() {
        let e = error("m", &["a"], ADD);
        assert!(
            e.contains("no attachid") && e.contains("anchor it on"),
            "{e}"
        );
    }

    #[test]
    fn a_correction_for_a_dropped_method_says_so() {
        let e = error("n", &["zip"], RENAME);
        assert!(e.contains("no longer has this method"), "{e}");
    }

    #[test]
    fn the_committed_wsdl_takes_every_correction() {
        let text = std::fs::read_to_string(crate::repo_root().join("tools/server.wsdl")).unwrap();
        let wsdl = parse_wsdl(&text).unwrap();
        for op in [
            "e911Provision",
            "e911ProvisionManually",
            "e911Update",
            "e911Validate",
        ] {
            let fields = names(&wsdl, op);
            assert!(fields.contains(&"zip_code".to_string()), "{op}: {fields:?}");
            assert!(!fields.contains(&"zip".to_string()), "{op}: {fields:?}");
        }

        assert!(names(&wsdl, "getLNPAttach").ends_with(&["portid".into(), "attachid".into()]));
    }
}
