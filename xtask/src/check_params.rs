//! `cargo xtask check-params`: audit the corrected WSDL inputs against the
//! parameters the portal docs list (`param_docs` in `tools/api-responses.json`).
//!
//! Two directions:
//!
//! * a documented parameter the WSDL does not declare: `gen` drops its
//!   description, and no caller can set it through the typed surface;
//! * a WSDL input the docs do not list.
//!
//! Each finding gets a `CORRECTIONS` entry in `xtask/src/wsdl.rs` once a
//! second source agrees with the docs, or goes in [`DELIBERATE`] with the
//! reason. A `DELIBERATE` entry the docs and the WSDL agree on is reported as
//! stale.
//!
//! Reports and exits successfully by default; `--deny` makes any finding a
//! non-zero exit, which is how CI holds it at zero. Run it after each
//! `extract-responses` refresh and each WSDL update.

use std::collections::BTreeSet;
use std::fs;

use crate::{CLIENT_FIELDS, ParamDocs, repo_root, wsdl};

/// Mismatches between the docs and the corrected WSDL that stay, as
/// `wireMethod.param` and the reason.
const DELIBERATE: &[(&str, &str)] = &[
    (
        "getResellerMMS.mms",
        "the MMS id; the docs name it `sms`, pasted from getResellerSMS",
    ),
    (
        "getResellerMMS.sms",
        "the docs' name for the MMS id, pasted from getResellerSMS; the WSDL's `mms` is kept",
    ),
    (
        "setDIDInfo.sms_sipaccount",
        "live: setDIDInfo stores it but ignores sms_sipaccount_enabled=0, so the \
         routing could be turned on here and not off; setSMS takes both",
    ),
    (
        "setDIDInfo.sms_sipaccount_enabled",
        "live: setDIDInfo ignores it set to 0; setSMS takes it and turns the routing off",
    ),
    (
        "setFaxNumberInfo.max_attempts",
        "a CORRECTIONS addition the docs leave out; live: the server stores it, and \
         resets it to 10 when a call omits it",
    ),
    (
        "setSMS.smpp_enabled",
        "live: the test account answers sms_apply_regulations, so no value can be read back",
    ),
    (
        "setSMS.smpp_pass",
        "live: the test account answers sms_apply_regulations, so no value can be read back",
    ),
    (
        "setSMS.smpp_url",
        "live: the test account answers sms_apply_regulations, so no value can be read back",
    ),
    (
        "setSMS.smpp_user",
        "live: the test account answers sms_apply_regulations, so no value can be read back",
    ),
];

/// The parameters the docs and the corrected WSDL disagree on, as
/// `wireMethod.param`, less the [`DELIBERATE`] ones.
#[derive(Debug, Default, PartialEq, Eq)]
struct Report {
    /// Listed by the docs, not declared by the WSDL.
    undeclared: Vec<String>,
    /// Declared by the WSDL, not listed by the docs.
    undocumented: Vec<String>,
    /// Deliberate entries that are no longer a mismatch.
    stale: Vec<String>,
}

pub fn cmd_check_params(args: &[String]) -> Result<(), String> {
    let deny = args.iter().any(|a| a == "--deny");
    let root = repo_root();
    let wsdl_path = root.join("tools").join("server.wsdl");
    let responses_path = root.join("tools").join("api-responses.json");

    let wsdl_text =
        fs::read_to_string(&wsdl_path).map_err(|e| format!("read {}: {e}", wsdl_path.display()))?;
    let wsdl = wsdl::parse_wsdl(&wsdl_text)?;
    let param_docs = crate::load_param_docs(&responses_path)?;
    let deliberate: Vec<&str> = DELIBERATE.iter().map(|(path, _)| *path).collect();
    let report = audit(&wsdl, &param_docs, &deliberate);

    if report == Report::default() {
        println!("ok: the corrected WSDL declares every documented parameter and no other");
        return Ok(());
    }

    print_section(
        "documented param(s) the WSDL does not declare (no caller can set them)",
        &report.undeclared,
    );
    print_section("WSDL input(s) the docs do not list", &report.undocumented);
    print_section(
        "DELIBERATE entr(ies) the docs and the WSDL agree on (stale)",
        &report.stale,
    );

    let summary = format!(
        "{} undeclared, {} undocumented, {} stale; add a CORRECTIONS entry in \
         xtask/src/wsdl.rs once a second source agrees with the docs, or record \
         why the difference stays in this command's DELIBERATE list",
        report.undeclared.len(),
        report.undocumented.len(),
        report.stale.len(),
    );
    if deny {
        return Err(summary);
    }

    println!("\n{summary}");
    Ok(())
}

fn print_section(title: &str, paths: &[String]) {
    if paths.is_empty() {
        return;
    }

    println!("{} {title}:", paths.len());
    for path in paths {
        println!("  {path}");
    }
}

/// Compare each method's documented parameters with its WSDL inputs. A method
/// the docs list no parameters for is skipped: the extractor found no table,
/// which says nothing about any one parameter.
fn audit(wsdl: &wsdl::Wsdl, param_docs: &ParamDocs, deliberate: &[&str]) -> Report {
    let mut report = Report::default();
    let mut mismatched = BTreeSet::new();
    let mut operations: Vec<&String> = wsdl.operations.iter().collect();
    operations.sort();
    for op in operations {
        let Some(documented) = param_docs.get(op) else {
            continue;
        };

        let declared: BTreeSet<&str> = wsdl
            .types
            .get(&format!("{op}Input"))
            .into_iter()
            .flatten()
            .map(|(name, _)| name.as_str())
            .filter(|name| !CLIENT_FIELDS.contains(name))
            .collect();
        let documented: BTreeSet<&str> = documented.keys().map(String::as_str).collect();
        for (names, others, out) in [
            (&documented, &declared, &mut report.undeclared),
            (&declared, &documented, &mut report.undocumented),
        ] {
            for name in names.difference(others) {
                let path = format!("{op}.{name}");
                mismatched.insert(path.clone());
                if !deliberate.contains(&path.as_str()) {
                    out.push(path);
                }
            }
        }
    }

    report.stale = deliberate
        .iter()
        .filter(|path| !mismatched.contains(**path))
        .map(|path| path.to_string())
        .collect();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wsdl_with(ops: &[(&str, &[&str])]) -> wsdl::Wsdl {
        wsdl::Wsdl {
            operations: ops.iter().map(|(op, _)| op.to_string()).collect(),
            types: ops
                .iter()
                .map(|(op, fields)| {
                    let fields = fields
                        .iter()
                        .map(|f| (f.to_string(), "xsd:string".to_string()))
                        .collect();
                    (format!("{op}Input"), fields)
                })
                .collect(),
        }
    }

    fn docs_with(ops: &[(&str, &[&str])]) -> ParamDocs {
        ops.iter()
            .map(|(op, fields)| {
                let fields = fields
                    .iter()
                    .map(|f| (f.to_string(), "doc".to_string()))
                    .collect();
                (op.to_string(), fields)
            })
            .collect()
    }

    fn strings(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn reports_each_direction_and_ignores_the_credentials() {
        let wsdl = wsdl_with(&[("m", &["api_username", "api_password", "a", "b"])]);
        let docs = docs_with(&[("m", &["a", "c"])]);
        assert_eq!(
            audit(&wsdl, &docs, &[]),
            Report {
                undeclared: strings(&["m.c"]),
                undocumented: strings(&["m.b"]),
                stale: vec![],
            }
        );
    }

    #[test]
    fn skips_a_method_the_docs_list_no_parameters_for() {
        let wsdl = wsdl_with(&[("m", &["a"]), ("n", &["b"])]);
        let docs = docs_with(&[("n", &["b"])]);
        assert_eq!(audit(&wsdl, &docs, &[]), Report::default());
    }

    #[test]
    fn a_deliberate_entry_hides_its_finding_and_goes_stale_once_fixed() {
        let wsdl = wsdl_with(&[("m", &["a"])]);
        let docs = docs_with(&[("m", &["a", "c"])]);
        assert_eq!(audit(&wsdl, &docs, &["m.c"]), Report::default());

        let docs = docs_with(&[("m", &["a"])]);
        assert_eq!(
            audit(&wsdl, &docs, &["m.c"]),
            Report {
                stale: strings(&["m.c"]),
                ..Report::default()
            }
        );
    }
}
