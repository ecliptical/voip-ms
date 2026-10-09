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
/// official PHP client in voip.ms's `API.zip`, which posts each parameter
/// under an explicit key and never reads the WSDL, or a live observation.
enum Correction {
    /// The server reads `to` where the WSDL declares `from`.
    Rename {
        op: &'static str,
        from: &'static str,
        to: &'static str,
    },
    /// The server reads `name`, which the WSDL omits. It is inserted before
    /// `before`, the position the docs list it in.
    Add {
        op: &'static str,
        name: &'static str,
        xsd_type: &'static str,
        before: &'static str,
    },
}

/// The corrections to the committed WSDL. An entry the WSDL has caught up with
/// is an error, so it cannot linger.
const CORRECTIONS: &[Correction] = &[
    // The docs and the PHP client both send `zip_code`, which is also the name
    // `e911Info` reports the same value under.
    Correction::Rename {
        op: "e911Provision",
        from: "zip",
        to: "zip_code",
    },
    Correction::Rename {
        op: "e911ProvisionManually",
        from: "zip",
        to: "zip_code",
    },
    Correction::Rename {
        op: "e911Update",
        from: "zip",
        to: "zip_code",
    },
    Correction::Rename {
        op: "e911Validate",
        from: "zip",
        to: "zip_code",
    },
    // The docs mark `portid` required and the PHP client sends it; every other
    // LNP method declares it `xsd:integer`.
    Correction::Add {
        op: "getLNPAttach",
        name: "portid",
        xsd_type: "xsd:integer",
        before: "attachid",
    },
];

/// Parse the WSDL into its operations and complex types, with
/// [`CORRECTIONS`] applied to the `*Input` types.
pub fn parse_wsdl(text: &str) -> Result<Wsdl, String> {
    let mut wsdl = parse_raw(text)?;
    apply_corrections(&mut wsdl, CORRECTIONS)?;
    Ok(wsdl)
}

fn apply_corrections(wsdl: &mut Wsdl, corrections: &[Correction]) -> Result<(), String> {
    for correction in corrections {
        let op = match correction {
            Correction::Rename { op, .. } | Correction::Add { op, .. } => *op,
        };
        let fields = wsdl
            .types
            .get_mut(&format!("{op}Input"))
            .ok_or_else(|| format!("WSDL correction names unknown method {op}"))?;
        let position = |name: &str| fields.iter().position(|(f, _)| f == name);
        match correction {
            Correction::Rename { from, to, .. } => {
                if position(to).is_some() {
                    return Err(format!(
                        "stale WSDL correction: {op} already declares {to}; drop the rename from {from}"
                    ));
                }

                let i = position(from).ok_or_else(|| {
                    format!("stale WSDL correction: {op} no longer declares {from}")
                })?;
                fields[i].0 = (*to).to_string();
            }
            Correction::Add {
                name,
                xsd_type,
                before,
                ..
            } => {
                if position(name).is_some() {
                    return Err(format!(
                        "stale WSDL correction: {op} already declares {name}; drop the addition"
                    ));
                }

                let i = position(before).ok_or_else(|| {
                    format!("WSDL correction for {op}: no {before} to insert {name} before")
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

    #[test]
    fn rename_replaces_the_field_in_place() {
        let mut wsdl = wsdl_with("m", &["a", "zip", "b"]);
        let fix = [Correction::Rename {
            op: "m",
            from: "zip",
            to: "zip_code",
        }];
        apply_corrections(&mut wsdl, &fix).unwrap();
        assert_eq!(names(&wsdl, "m"), ["a", "zip_code", "b"]);
    }

    #[test]
    fn add_inserts_before_its_anchor_with_its_type() {
        let mut wsdl = wsdl_with("m", &["a", "attachid"]);
        let fix = [Correction::Add {
            op: "m",
            name: "portid",
            xsd_type: "xsd:integer",
            before: "attachid",
        }];
        apply_corrections(&mut wsdl, &fix).unwrap();
        assert_eq!(names(&wsdl, "m"), ["a", "portid", "attachid"]);
        assert_eq!(wsdl.types["mInput"][1].1, "xsd:integer");
    }

    #[test]
    fn a_correction_the_wsdl_caught_up_with_fails() {
        let rename = [Correction::Rename {
            op: "m",
            from: "zip",
            to: "zip_code",
        }];
        let add = [Correction::Add {
            op: "m",
            name: "portid",
            xsd_type: "xsd:integer",
            before: "attachid",
        }];
        assert!(apply_corrections(&mut wsdl_with("m", &["zip_code"]), &rename).is_err());
        assert!(apply_corrections(&mut wsdl_with("m", &["attachid"]), &rename).is_err());
        assert!(apply_corrections(&mut wsdl_with("m", &["portid", "attachid"]), &add).is_err());
        assert!(apply_corrections(&mut wsdl_with("n", &["zip"]), &rename).is_err());
    }

    #[test]
    fn the_committed_wsdl_takes_every_correction() {
        let text = std::fs::read_to_string(crate::repo_root().join("tools/server.wsdl")).unwrap();
        let wsdl = parse_wsdl(&text).unwrap();
        assert!(names(&wsdl, "e911Validate").contains(&"zip_code".to_string()));
        assert!(names(&wsdl, "getLNPAttach").ends_with(&["portid".into(), "attachid".into()]));
    }
}
