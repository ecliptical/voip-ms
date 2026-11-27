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
    /// The server reads `name`, which the WSDL omits. It is inserted `at` the
    /// position the docs list it in.
    Add {
        name: &'static str,
        xsd_type: &'static str,
        at: At,
    },
}

/// Where [`Fix::Add`] inserts a parameter.
enum At {
    /// Before this input.
    Before(&'static str),
    /// After the last input.
    End,
}

const fn add(op: &'static str, name: &'static str, xsd_type: &'static str, at: At) -> Correction {
    Correction {
        op,
        fix: Fix::Add { name, xsd_type, at },
    }
}

/// The corrections to the committed WSDL. Each way an entry can stop fitting
/// the WSDL is an error naming the fix, so none can linger.
const CORRECTIONS: &[Correction] = &[
    // The docs and the PHP client both send `zip_code`, which is also the name
    // `e911Info` reports the same value under. Confirmed live on e911Validate:
    // a postal code sent as `zip` answers `missing_zip`, the same as none.
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
            at: At::Before("attachid"),
        },
    },
    // The docs list `allow225` after `lock_international` on both methods.
    // Confirmed live: `createSubAccount` with `allow225=1` and `setSubAccount`
    // flipping it to `1` and back each read back on `getSubAccounts`.
    Correction {
        op: "createSubAccount",
        fix: ALLOW_225,
    },
    Correction {
        op: "setSubAccount",
        fix: ALLOW_225,
    },
    // The docs list these on `setDIDInfo` and `setSMS`. Confirmed live on a
    // DID: each value sent read back on `getDIDsInfo`, `inbound_dialing_mode=2`
    // answered `invalid_inbound_dialing_mode` and `dialmode=3`
    // `invalid_dialmode`. The transcription switches are stored only while
    // `transcribe` is on. The docs also list `sms_sipaccount` and
    // `sms_sipaccount_enabled` on `setDIDInfo`, which sets them but ignores
    // `sms_sipaccount_enabled=0`; `setSMS` declares both and turns the routing
    // off. Neither clears `sms_sipaccount` with an empty value. The docs' SMPP
    // parameters on `setSMS` are left out: the test account answers
    // `sms_apply_regulations`, so no value could be read back.
    Correction {
        op: "setDIDInfo",
        fix: Fix::Add {
            name: "inbound_dialing_mode",
            xsd_type: "xsd:integer",
            at: At::Before("note"),
        },
    },
    Correction {
        op: "setDIDInfo",
        fix: Fix::Add {
            name: "transcription_redaction",
            xsd_type: "xsd:string",
            at: At::Before("transcription_email"),
        },
    },
    Correction {
        op: "setDIDInfo",
        fix: Fix::Add {
            name: "transcription_summary",
            xsd_type: "xsd:string",
            at: At::Before("transcription_email"),
        },
    },
    Correction {
        op: "setDIDInfo",
        fix: Fix::Add {
            name: "transcription_sentiment",
            xsd_type: "xsd:string",
            at: At::Before("transcription_email"),
        },
    },
    Correction {
        op: "setSMS",
        fix: Fix::Add {
            name: "webhook_enable",
            xsd_type: "xsd:integer",
            at: At::Before("sms_sipaccount"),
        },
    },
    Correction {
        op: "setSMS",
        fix: Fix::Add {
            name: "webhook",
            xsd_type: "xsd:string",
            at: At::Before("sms_sipaccount"),
        },
    },
    Correction {
        op: "setSMS",
        fix: Fix::Add {
            name: "dialmode",
            xsd_type: "xsd:integer",
            at: At::Before("sms_sipaccount"),
        },
    },
    // Each entry from here to `max_attempts` was confirmed live, as its comment
    // says.
    //
    // A reseller client filter: `getCallAccounts` with a client lists only
    // that client's sub-accounts, `getVoicemails` answers `no_mailbox` for a
    // client with none, and an unknown id answers `invalid_client` on both.
    // The PHP client also sends it on `getCallAccounts`.
    add("getCallAccounts", "client", "xsd:integer", At::End),
    add("getVoicemails", "client", "xsd:integer", At::End),
    // `outgoing` drops the account's one incoming recording, and `bogus`
    // answers `invalid_call_type`.
    add("getCallRecordings", "call_type", "xsd:string", At::End),
    // An unknown mailbox answers `invalid_mailbox`.
    add("getVoicemailFolders", "mailbox", "xsd:integer", At::End),
    // `wav` returns a RIFF file where `mp3` and the default return MP3 frames.
    add("getVoicemailMessageFile", "format", "xsd:string", At::End),
    // 3 and 7 read back on `getCallParking`; 11 answers `invalid_blf_lamps`.
    add("setCallParking", "blf_lamps", "xsd:integer", At::End),
    // 1 and 0 are accepted and 7 answers `invalid_attestation_a_enabled`.
    // `getForwardings` does not report the value.
    add(
        "setForwarding",
        "attestation_a_enabled",
        "xsd:integer",
        At::End,
    ),
    // Each read back on `getFaxNumbersInfo`. Neither an empty `note` nor a
    // space clears one already set.
    add(
        "setFaxNumberInfo",
        "fax_to_sip_enabled",
        "xsd:integer",
        At::Before("test"),
    ),
    add(
        "setFaxNumberInfo",
        "fax_to_sip_enabled_account",
        "xsd:string",
        At::Before("test"),
    ),
    add("setFaxNumberInfo", "note", "xsd:string", At::Before("test")),
    // A mailbox created for client 637122 with a 15-second delay, then set to
    // client 637123 and 30 seconds, read back each on `getVoicemails`. A delay
    // of 7 answers `invalid_transcription_delay`, an unknown client
    // `invalid_client`.
    add(
        "createVoicemail",
        "client",
        "xsd:integer",
        At::Before("email_attachment_format"),
    ),
    add(
        "createVoicemail",
        "transcription_delay",
        "xsd:integer",
        At::End,
    ),
    add(
        "setVoicemail",
        "client",
        "xsd:integer",
        At::Before("email_attachment_format"),
    ),
    add(
        "setVoicemail",
        "transcription_delay",
        "xsd:integer",
        At::End,
    ),
    // Each read back on `getSubAccounts` after `createSubAccount` or
    // `setSubAccount`. `createSubAccount` answers `invalid_default_911` to a
    // DID with no e911 registration as `default_e911`, which `setSubAccount`
    // already declares.
    add(
        "createSubAccount",
        "parking_lot",
        "xsd:integer",
        At::Before("transcription_start_delay"),
    ),
    add(
        "createSubAccount",
        "default_e911",
        "xsd:string",
        At::Before("dialing_mode"),
    ),
    add(
        "createSubAccount",
        "call_pickup_behavior",
        "xsd:integer",
        At::Before("dialing_mode"),
    ),
    add(
        "setSubAccount",
        "parking_lot",
        "xsd:integer",
        At::Before("transcription_start_delay"),
    ),
    add(
        "setSubAccount",
        "enable_internal_cnam",
        "xsd:integer",
        At::Before("dialing_mode"),
    ),
    add(
        "setSubAccount",
        "internal_cnam",
        "xsd:string",
        At::Before("dialing_mode"),
    ),
    add(
        "setSubAccount",
        "call_pickup_behavior",
        "xsd:integer",
        At::Before("dialing_mode"),
    ),
    // Without it both answer `missing_email`, and a malformed one
    // `invalid_email`, before checking that the DID is on the account.
    add(
        "e911Provision",
        "email",
        "xsd:string",
        At::Before("other_info"),
    ),
    add(
        "e911Validate",
        "email",
        "xsd:string",
        At::Before("other_info"),
    ),
    // Neither the docs nor the PHP client name it, but `getFaxNumbersInfo`
    // reports it and the server reads it. Confirmed live on a fax number: 1, 3,
    // 11 and 99 each read back as sent, 0 and `abc` left the value as it was,
    // and a call without it set it to 10 whatever it was before.
    Correction {
        op: "setFaxNumberInfo",
        fix: Fix::Add {
            name: "max_attempts",
            xsd_type: "xsd:integer",
            at: At::Before("test"),
        },
    },
];

const ALLOW_225: Fix = Fix::Add {
    name: "allow225",
    xsd_type: "xsd:integer",
    at: At::Before("international_route"),
};

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
            Fix::Add { name, xsd_type, at } => {
                if position(name).is_some() {
                    return Err(format!(
                        "stale WSDL correction for {op}: the WSDL now declares {name}; \
                         drop the addition"
                    ));
                }

                let i = match at {
                    At::Before(before) => position(before).ok_or_else(|| {
                        format!(
                            "WSDL correction for {op}: no {before} to insert {name} before; \
                             anchor it on another field"
                        )
                    })?,
                    At::End => fields.len(),
                };
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
            at: At::Before("attachid"),
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
    fn add_at_the_end_appends_after_the_last_input() {
        let mut wsdl = wsdl_with("m", &["a", "b"]);
        let append = add("m", "c", "xsd:integer", At::End);
        apply_corrections(&mut wsdl, &[append]).unwrap();
        assert_eq!(names(&wsdl, "m"), ["a", "b", "c"]);
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
