//! Codegen-drift oracle: every generated wire enum's `tag <-> variant` map.
//!
//! Hand-maintained on purpose. For each enum, the known wire strings and their
//! `from_wire`/`as_wire`/`Display`/serde round-trips are asserted independently
//! of `generated.rs`, plus that an unrecognized value is preserved via the
//! `Unknown` variant. A regen that changes an enum's wire mapping breaks this
//! suite -- the intended signal. Update by hand when a wire enum changes (see
//! DEVELOPMENT.md).

use voip_ms::serde_json;
use voip_ms::*;

#[test]
fn call_pickup_behavior_wire_roundtrips() {
    for w in ["1", "2", "3", "4"] {
        assert_eq!(CallPickupBehavior::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(CallPickupBehavior::from_wire(w)).unwrap();
        let back: CallPickupBehavior = serde_json::from_value(v).unwrap();
        assert_eq!(back, CallPickupBehavior::from_wire(w));
    }

    assert_eq!(CallPickupBehavior::from_wire("1").to_string(), "1");
}

#[test]
fn call_pickup_behavior_unknown_preserved() {
    let u = CallPickupBehavior::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: CallPickupBehavior = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn dialing_mode_wire_roundtrips() {
    for w in ["0", "1", "2"] {
        assert_eq!(DialingMode::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(DialingMode::from_wire(w)).unwrap();
        let back: DialingMode = serde_json::from_value(v).unwrap();
        assert_eq!(back, DialingMode::from_wire(w));
    }

    assert_eq!(DialingMode::from_wire("0").to_string(), "0");
}

#[test]
fn dialing_mode_unknown_preserved() {
    let u = DialingMode::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: DialingMode = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn did_billing_type_wire_roundtrips() {
    for w in ["1", "2"] {
        assert_eq!(DidBillingType::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(DidBillingType::from_wire(w)).unwrap();
        let back: DidBillingType = serde_json::from_value(v).unwrap();
        assert_eq!(back, DidBillingType::from_wire(w));
    }

    assert_eq!(DidBillingType::from_wire("1").to_string(), "1");
}

#[test]
fn did_billing_type_unknown_preserved() {
    let u = DidBillingType::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: DidBillingType = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn dtmf_mode_wire_roundtrips() {
    for w in ["auto", "rfc2833", "inband", "info"] {
        assert_eq!(DtmfMode::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(DtmfMode::from_wire(w)).unwrap();
        let back: DtmfMode = serde_json::from_value(v).unwrap();
        assert_eq!(back, DtmfMode::from_wire(w));
    }

    assert_eq!(DtmfMode::from_wire("auto").to_string(), "auto");
}

#[test]
fn dtmf_mode_unknown_preserved() {
    let u = DtmfMode::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: DtmfMode = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn email_attachment_format_wire_roundtrips() {
    for w in ["wav49", "wav", "wavmp3", "no"] {
        assert_eq!(EmailAttachmentFormat::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(EmailAttachmentFormat::from_wire(w)).unwrap();
        let back: EmailAttachmentFormat = serde_json::from_value(v).unwrap();
        assert_eq!(back, EmailAttachmentFormat::from_wire(w));
    }

    // The MP3 option's wire value is `wavmp3`, not `mp3`.
    assert_eq!(
        EmailAttachmentFormat::from_wire("wavmp3"),
        EmailAttachmentFormat::Mp3
    );

    assert_eq!(
        EmailAttachmentFormat::from_wire("wav49").to_string(),
        "wav49"
    );
}

#[test]
fn email_attachment_format_unknown_preserved() {
    let u = EmailAttachmentFormat::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: EmailAttachmentFormat = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn estimated_hold_time_announce_wire_roundtrips() {
    for w in ["yes", "no", "once"] {
        assert_eq!(EstimatedHoldTimeAnnounce::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(EstimatedHoldTimeAnnounce::from_wire(w)).unwrap();
        let back: EstimatedHoldTimeAnnounce = serde_json::from_value(v).unwrap();
        assert_eq!(back, EstimatedHoldTimeAnnounce::from_wire(w));
    }

    assert_eq!(
        EstimatedHoldTimeAnnounce::from_wire("yes").to_string(),
        "yes"
    );
}

#[test]
fn estimated_hold_time_announce_unknown_preserved() {
    let u = EstimatedHoldTimeAnnounce::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: EstimatedHoldTimeAnnounce = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn location_type_wire_roundtrips() {
    for w in ["0", "1"] {
        assert_eq!(LocationType::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(LocationType::from_wire(w)).unwrap();
        let back: LocationType = serde_json::from_value(v).unwrap();
        assert_eq!(back, LocationType::from_wire(w));
    }

    assert_eq!(LocationType::from_wire("0").to_string(), "0");
}

#[test]
fn location_type_unknown_preserved() {
    let u = LocationType::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: LocationType = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn message_type_wire_roundtrips() {
    for w in ["1", "0"] {
        assert_eq!(MessageType::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(MessageType::from_wire(w)).unwrap();
        let back: MessageType = serde_json::from_value(v).unwrap();
        assert_eq!(back, MessageType::from_wire(w));
    }

    assert_eq!(MessageType::from_wire("1").to_string(), "1");
}

#[test]
fn message_type_unknown_preserved() {
    let u = MessageType::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: MessageType = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn nat_wire_roundtrips() {
    for w in ["yes", "no", "route", "never"] {
        assert_eq!(Nat::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(Nat::from_wire(w)).unwrap();
        let back: Nat = serde_json::from_value(v).unwrap();
        assert_eq!(back, Nat::from_wire(w));
    }

    assert_eq!(Nat::from_wire("yes").to_string(), "yes");
}

#[test]
fn nat_unknown_preserved() {
    let u = Nat::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: Nat = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn play_instructions_wire_roundtrips() {
    for w in ["su", "u"] {
        assert_eq!(PlayInstructions::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(PlayInstructions::from_wire(w)).unwrap();
        let back: PlayInstructions = serde_json::from_value(v).unwrap();
        assert_eq!(back, PlayInstructions::from_wire(w));
    }

    assert_eq!(PlayInstructions::from_wire("su").to_string(), "su");
}

#[test]
fn play_instructions_unknown_preserved() {
    let u = PlayInstructions::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: PlayInstructions = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn queue_empty_behavior_wire_roundtrips() {
    for w in ["yes", "no", "strict"] {
        assert_eq!(QueueEmptyBehavior::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(QueueEmptyBehavior::from_wire(w)).unwrap();
        let back: QueueEmptyBehavior = serde_json::from_value(v).unwrap();
        assert_eq!(back, QueueEmptyBehavior::from_wire(w));
    }

    assert_eq!(QueueEmptyBehavior::from_wire("yes").to_string(), "yes");
}

#[test]
fn queue_empty_behavior_unknown_preserved() {
    let u = QueueEmptyBehavior::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: QueueEmptyBehavior = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn recording_sort_wire_roundtrips() {
    for w in ["alpha", "random"] {
        assert_eq!(RecordingSort::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(RecordingSort::from_wire(w)).unwrap();
        let back: RecordingSort = serde_json::from_value(v).unwrap();
        assert_eq!(back, RecordingSort::from_wire(w));
    }

    assert_eq!(RecordingSort::from_wire("alpha").to_string(), "alpha");
}

#[test]
fn recording_sort_unknown_preserved() {
    let u = RecordingSort::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: RecordingSort = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn ring_group_order_wire_roundtrips() {
    for w in ["follow", "random"] {
        assert_eq!(RingGroupOrder::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(RingGroupOrder::from_wire(w)).unwrap();
        let back: RingGroupOrder = serde_json::from_value(v).unwrap();
        assert_eq!(back, RingGroupOrder::from_wire(w));
    }

    assert_eq!(RingGroupOrder::from_wire("follow").to_string(), "follow");
}

#[test]
fn ring_group_order_unknown_preserved() {
    let u = RingGroupOrder::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: RingGroupOrder = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn ring_strategy_wire_roundtrips() {
    for w in [
        "ringall",
        "leastrecent",
        "fewestcalls",
        "random",
        "rrmemory",
    ] {
        assert_eq!(RingStrategy::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(RingStrategy::from_wire(w)).unwrap();
        let back: RingStrategy = serde_json::from_value(v).unwrap();
        assert_eq!(back, RingStrategy::from_wire(w));
    }

    assert_eq!(RingStrategy::from_wire("ringall").to_string(), "ringall");
}

#[test]
fn ring_strategy_unknown_preserved() {
    let u = RingStrategy::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: RingStrategy = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn search_type_wire_roundtrips() {
    for w in ["starts", "contains", "ends"] {
        assert_eq!(SearchType::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(SearchType::from_wire(w)).unwrap();
        let back: SearchType = serde_json::from_value(v).unwrap();
        assert_eq!(back, SearchType::from_wire(w));
    }

    assert_eq!(SearchType::from_wire("starts").to_string(), "starts");
}

#[test]
fn search_type_unknown_preserved() {
    let u = SearchType::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: SearchType = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn toll_free_carrier_wire_roundtrips() {
    for w in ["-1", "0", "1", "2"] {
        assert_eq!(TollFreeCarrier::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(TollFreeCarrier::from_wire(w)).unwrap();
        let back: TollFreeCarrier = serde_json::from_value(v).unwrap();
        assert_eq!(back, TollFreeCarrier::from_wire(w));
    }

    assert_eq!(TollFreeCarrier::from_wire("-1").to_string(), "-1");
}

#[test]
fn toll_free_carrier_unknown_preserved() {
    let u = TollFreeCarrier::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: TollFreeCarrier = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn transcription_format_wire_roundtrips() {
    for w in ["text", "html"] {
        assert_eq!(TranscriptionFormat::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(TranscriptionFormat::from_wire(w)).unwrap();
        let back: TranscriptionFormat = serde_json::from_value(v).unwrap();
        assert_eq!(back, TranscriptionFormat::from_wire(w));
    }

    assert_eq!(TranscriptionFormat::from_wire("text").to_string(), "text");
}

#[test]
fn transcription_format_unknown_preserved() {
    let u = TranscriptionFormat::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: TranscriptionFormat = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn vanity_type_wire_roundtrips() {
    for w in ["8**", "800", "833", "844", "855", "866", "877", "888"] {
        assert_eq!(VanityType::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(VanityType::from_wire(w)).unwrap();
        let back: VanityType = serde_json::from_value(v).unwrap();
        assert_eq!(back, VanityType::from_wire(w));
    }

    assert_eq!(VanityType::from_wire("8**").to_string(), "8**");
}

#[test]
fn vanity_type_unknown_preserved() {
    let u = VanityType::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: VanityType = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}

#[test]
fn voicemail_folder_wire_roundtrips() {
    for w in ["INBOX", "Old", "Urgent", "Family", "Friends", "Work"] {
        assert_eq!(VoicemailFolder::from_wire(w).as_wire(), w);
        let v = serde_json::to_value(VoicemailFolder::from_wire(w)).unwrap();
        let back: VoicemailFolder = serde_json::from_value(v).unwrap();
        assert_eq!(back, VoicemailFolder::from_wire(w));
    }

    assert_eq!(VoicemailFolder::from_wire("INBOX").to_string(), "INBOX");
}

#[test]
fn voicemail_folder_unknown_preserved() {
    let u = VoicemailFolder::from_wire("zzz_unknown");
    assert_eq!(u.as_wire(), "zzz_unknown");
    let v = serde_json::to_value(&u).unwrap();
    let back: VoicemailFolder = serde_json::from_value(v).unwrap();
    assert_eq!(back, u);
}
