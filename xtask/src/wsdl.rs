//! WSDL parsing shared by codegen and the response extractor.

use roxmltree::{Document, Node};
use std::collections::BTreeSet;

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug)]
pub struct Wsdl {
    pub operations: Vec<String>,
    pub types: std::collections::HashMap<String, Vec<(String, String)>>,
}

pub fn parse_wsdl(text: &str) -> Result<Wsdl, String> {
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
