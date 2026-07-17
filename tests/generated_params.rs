//! Codegen-drift oracle: every generated `*Params` struct serializes.
//!
//! Hand-maintained on purpose. `src/generated.rs` is produced by
//! `cargo xtask gen` from the WSDL; this suite is the fixed, independent
//! check that each of the 222 request structs still derives `Default` and
//! `Serialize` and emits a JSON object. A regen that drops, renames, or
//! mistypes a struct breaks compilation here -- that failure is the gate
//! working, not a test to paper over. When the generated surface changes,
//! update these cases by hand (see DEVELOPMENT.md).

#![allow(clippy::needless_update)]

use voip_ms::serde_json;
use voip_ms::*;

#[test]
fn add_charge_params_roundtrips() {
    let p = AddChargeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AddChargeParams {
        client: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn add_lnp_file_params_roundtrips() {
    let p = AddLNPFileParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AddLNPFileParams {
        portid: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn add_lnp_port_params_roundtrips() {
    let p = AddLNPPortParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AddLNPPortParams {
        port_type: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn add_member_to_conference_params_roundtrips() {
    let p = AddMemberToConferenceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AddMemberToConferenceParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn add_payment_params_roundtrips() {
    let p = AddPaymentParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AddPaymentParams {
        client: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn assign_did_vpri_params_roundtrips() {
    let p = AssignDIDvPRIParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = AssignDIDvPRIParams {
        vpri: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn back_order_did_can_params_roundtrips() {
    let p = BackOrderDIDCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = BackOrderDIDCANParams {
        quantity: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn back_order_did_usa_params_roundtrips() {
    let p = BackOrderDIDUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = BackOrderDIDUSAParams {
        quantity: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn cancel_did_params_roundtrips() {
    let p = CancelDIDParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = CancelDIDParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn cancel_fax_number_params_roundtrips() {
    let p = CancelFAXNumberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = CancelFAXNumberParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn connect_did_params_roundtrips() {
    let p = ConnectDIDParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = ConnectDIDParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn connect_fax_params_roundtrips() {
    let p = ConnectFAXParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = ConnectFAXParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn create_sub_account_params_roundtrips() {
    let p = CreateSubAccountParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = CreateSubAccountParams {
        username: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn create_voicemail_params_roundtrips() {
    let p = CreateVoicemailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = CreateVoicemailParams {
        digits: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_call_hunting_params_roundtrips() {
    let p = DelCallHuntingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelCallHuntingParams {
        callhunting: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_call_parking_params_roundtrips() {
    let p = DelCallParkingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelCallParkingParams {
        callparking: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_call_recording_params_roundtrips() {
    let p = DelCallRecordingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelCallRecordingParams {
        callrecording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_callback_params_roundtrips() {
    let p = DelCallbackParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelCallbackParams {
        callback: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_caller_id_filtering_params_roundtrips() {
    let p = DelCallerIDFilteringParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelCallerIDFilteringParams {
        filtering: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_client_params_roundtrips() {
    let p = DelClientParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelClientParams {
        client: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_conference_params_roundtrips() {
    let p = DelConferenceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelConferenceParams {
        conference: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_conference_member_params_roundtrips() {
    let p = DelConferenceMemberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelConferenceMemberParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_disa_params_roundtrips() {
    let p = DelDISAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelDISAParams {
        disa: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_email_to_fax_params_roundtrips() {
    let p = DelEmailToFAXParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelEmailToFAXParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_fax_folder_params_roundtrips() {
    let p = DelFAXFolderParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelFAXFolderParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_forwarding_params_roundtrips() {
    let p = DelForwardingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelForwardingParams {
        forwarding: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_ivr_params_roundtrips() {
    let p = DelIVRParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelIVRParams {
        ivr: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_location_params_roundtrips() {
    let p = DelLocationParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelLocationParams {
        name: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_member_from_conference_params_roundtrips() {
    let p = DelMemberFromConferenceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelMemberFromConferenceParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_messages_params_roundtrips() {
    let p = DelMessagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelMessagesParams {
        mailbox: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_music_on_hold_params_roundtrips() {
    let p = DelMusicOnHoldParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelMusicOnHoldParams {
        music_on_hold: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_phonebook_params_roundtrips() {
    let p = DelPhonebookParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelPhonebookParams {
        phonebook: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_phonebook_group_params_roundtrips() {
    let p = DelPhonebookGroupParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelPhonebookGroupParams {
        group: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_queue_params_roundtrips() {
    let p = DelQueueParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelQueueParams {
        queue: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_recording_params_roundtrips() {
    let p = DelRecordingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelRecordingParams {
        recording: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_ring_group_params_roundtrips() {
    let p = DelRingGroupParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelRingGroupParams {
        ring_group: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
    // The field is `ring_group` (consistent with get/set) but still travels on
    // the wire as the upstream `ringgroup`.
    assert_eq!(v2.get("ringgroup"), Some(&serde_json::json!(1)));
    assert!(v2.get("ring_group").is_none());
}

#[test]
fn del_sip_uri_params_roundtrips() {
    let p = DelSIPURIParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelSIPURIParams {
        sip_uri: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_static_member_params_roundtrips() {
    let p = DelStaticMemberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelStaticMemberParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_sub_account_params_roundtrips() {
    let p = DelSubAccountParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelSubAccountParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_time_condition_params_roundtrips() {
    let p = DelTimeConditionParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelTimeConditionParams {
        timecondition: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn del_voicemail_params_roundtrips() {
    let p = DelVoicemailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DelVoicemailParams {
        mailbox: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn delete_fax_message_params_roundtrips() {
    let p = DeleteFAXMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DeleteFAXMessageParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn delete_mms_params_roundtrips() {
    let p = DeleteMMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DeleteMMSParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn delete_sms_params_roundtrips() {
    let p = DeleteSMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = DeleteSMSParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_address_types_params_roundtrips() {
    let p = E911AddressTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911AddressTypesParams {
        r#type: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_cancel_params_roundtrips() {
    let p = E911CancelParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911CancelParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_info_params_roundtrips() {
    let p = E911InfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911InfoParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_provision_params_roundtrips() {
    let p = E911ProvisionParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911ProvisionParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_provision_manually_params_roundtrips() {
    let p = E911ProvisionManuallyParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911ProvisionManuallyParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_update_params_roundtrips() {
    let p = E911UpdateParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911UpdateParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn e911_validate_params_roundtrips() {
    let p = E911ValidateParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = E911ValidateParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_allowed_codecs_params_roundtrips() {
    let p = GetAllowedCodecsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetAllowedCodecsParams {
        codec: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_auth_types_params_roundtrips() {
    let p = GetAuthTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetAuthTypesParams {
        r#type: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_back_orders_params_roundtrips() {
    let p = GetBackOrdersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetBackOrdersParams {
        id: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_balance_params_roundtrips() {
    let p = GetBalanceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetBalanceParams {
        advanced: Some(true),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_balance_management_params_roundtrips() {
    let p = GetBalanceManagementParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetBalanceManagementParams {
        balance_management: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_cdr_params_roundtrips() {
    let p = GetCDRParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCDRParams {
        date_from: Some(voip_ms::chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_accounts_params_roundtrips() {
    let p = GetCallAccountsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_call_billing_params_roundtrips() {
    let p = GetCallBillingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_call_huntings_params_roundtrips() {
    let p = GetCallHuntingsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallHuntingsParams {
        callhunting: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_parking_params_roundtrips() {
    let p = GetCallParkingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallParkingParams {
        callparking: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_recording_params_roundtrips() {
    let p = GetCallRecordingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallRecordingParams {
        callrecording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_recordings_params_roundtrips() {
    let p = GetCallRecordingsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallRecordingsParams {
        account: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_transcriptions_params_roundtrips() {
    let p = GetCallTranscriptionsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallTranscriptionsParams {
        account: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_call_types_params_roundtrips() {
    let p = GetCallTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallTypesParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_callbacks_params_roundtrips() {
    let p = GetCallbacksParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallbacksParams {
        callback: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_caller_id_filtering_params_roundtrips() {
    let p = GetCallerIDFilteringParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCallerIDFilteringParams {
        filtering: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_carriers_params_roundtrips() {
    let p = GetCarriersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCarriersParams {
        carrier: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_charges_params_roundtrips() {
    let p = GetChargesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetChargesParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_client_packages_params_roundtrips() {
    let p = GetClientPackagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetClientPackagesParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_client_threshold_params_roundtrips() {
    let p = GetClientThresholdParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetClientThresholdParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_clients_params_roundtrips() {
    let p = GetClientsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetClientsParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_conference_params_roundtrips() {
    let p = GetConferenceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetConferenceParams {
        conference: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_conference_members_params_roundtrips() {
    let p = GetConferenceMembersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetConferenceMembersParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_conference_recording_file_params_roundtrips() {
    let p = GetConferenceRecordingFileParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetConferenceRecordingFileParams {
        conference: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_conference_recordings_params_roundtrips() {
    let p = GetConferenceRecordingsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetConferenceRecordingsParams {
        conference: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_countries_params_roundtrips() {
    let p = GetCountriesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetCountriesParams {
        country: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_did_countries_params_roundtrips() {
    let p = GetDIDCountriesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDCountriesParams {
        country_id: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_can_params_roundtrips() {
    let p = GetDIDsCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsCANParams {
        province: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_info_params_roundtrips() {
    let p = GetDIDsInfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsInfoParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_international_geographic_params_roundtrips() {
    let p = GetDIDsInternationalGeographicParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsInternationalGeographicParams {
        country_id: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_international_national_params_roundtrips() {
    let p = GetDIDsInternationalNationalParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsInternationalNationalParams {
        country_id: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_international_toll_free_params_roundtrips() {
    let p = GetDIDsInternationalTollFreeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsInternationalTollFreeParams {
        country_id: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dids_usa_params_roundtrips() {
    let p = GetDIDsUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDsUSAParams {
        state: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_did_v_pri_params_roundtrips() {
    let p = GetDIDvPRIParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDIDvPRIParams {
        vpri: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_disas_params_roundtrips() {
    let p = GetDISAsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDISAsParams {
        disa: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_dtmf_modes_params_roundtrips() {
    let p = GetDTMFModesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDTMFModesParams {
        dtmf_mode: Some(DtmfMode::from_wire("1")),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_deposits_params_roundtrips() {
    let p = GetDepositsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDepositsParams {
        client: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_device_types_params_roundtrips() {
    let p = GetDeviceTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetDeviceTypesParams {
        device_type: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_email_to_fax_params_roundtrips() {
    let p = GetEmailToFAXParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetEmailToFAXParams { id: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_folders_params_roundtrips() {
    let p = GetFAXFoldersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXFoldersParams { id: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_message_pdf_params_roundtrips() {
    let p = GetFAXMessagePDFParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXMessagePDFParams { id: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_messages_params_roundtrips() {
    let p = GetFAXMessagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXMessagesParams {
        from: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_numbers_info_params_roundtrips() {
    let p = GetFAXNumbersInfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXNumbersInfoParams {
        did: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_numbers_portability_params_roundtrips() {
    let p = GetFAXNumbersPortabilityParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXNumbersPortabilityParams {
        did: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_provinces_params_roundtrips() {
    let p = GetFAXProvincesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXProvincesParams {
        province: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_rate_centers_can_params_roundtrips() {
    let p = GetFAXRateCentersCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXRateCentersCANParams {
        province: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_rate_centers_usa_params_roundtrips() {
    let p = GetFAXRateCentersUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXRateCentersUSAParams {
        state: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_fax_states_params_roundtrips() {
    let p = GetFAXStatesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetFAXStatesParams {
        state: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_forwardings_params_roundtrips() {
    let p = GetForwardingsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetForwardingsParams {
        forwarding: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_ip_params_roundtrips() {
    let p = GetIPParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_ivrs_params_roundtrips() {
    let p = GetIVRsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetIVRsParams {
        ivr: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_international_types_params_roundtrips() {
    let p = GetInternationalTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetInternationalTypesParams {
        r#type: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_join_when_empty_types_params_roundtrips() {
    let p = GetJoinWhenEmptyTypesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetJoinWhenEmptyTypesParams {
        r#type: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_attach_params_roundtrips() {
    let p = GetLNPAttachParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPAttachParams { attachid: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_attach_list_params_roundtrips() {
    let p = GetLNPAttachListParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPAttachListParams { portid: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_details_params_roundtrips() {
    let p = GetLNPDetailsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPDetailsParams { portid: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_list_params_roundtrips() {
    let p = GetLNPListParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPListParams {
        portid: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_list_status_params_roundtrips() {
    let p = GetLNPListStatusParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_lnp_notes_params_roundtrips() {
    let p = GetLNPNotesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPNotesParams { portid: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_lnp_status_params_roundtrips() {
    let p = GetLNPStatusParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLNPStatusParams { portid: Some(1) };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_languages_params_roundtrips() {
    let p = GetLanguagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLanguagesParams {
        language: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_locales_params_roundtrips() {
    let p = GetLocalesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLocalesParams {
        locale: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_locations_params_roundtrips() {
    let p = GetLocationsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_lock_international_params_roundtrips() {
    let p = GetLockInternationalParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetLockInternationalParams {
        lock_international: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_mms_params_roundtrips() {
    let p = GetMMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetMMSParams {
        mms: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_media_mms_params_roundtrips() {
    let p = GetMediaMMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetMediaMMSParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_music_on_hold_params_roundtrips() {
    let p = GetMusicOnHoldParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetMusicOnHoldParams {
        music_on_hold: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_nat_params_roundtrips() {
    let p = GetNATParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetNATParams {
        nat: Some(Nat::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_packages_params_roundtrips() {
    let p = GetPackagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetPackagesParams {
        package: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_phonebook_params_roundtrips() {
    let p = GetPhonebookParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetPhonebookParams {
        phonebook: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_phonebook_groups_params_roundtrips() {
    let p = GetPhonebookGroupsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetPhonebookGroupsParams {
        name: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_play_instructions_params_roundtrips() {
    let p = GetPlayInstructionsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetPlayInstructionsParams {
        play_instructions: Some(PlayInstructions::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_portability_params_roundtrips() {
    let p = GetPortabilityParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetPortabilityParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_protocols_params_roundtrips() {
    let p = GetProtocolsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetProtocolsParams {
        protocol: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_provinces_params_roundtrips() {
    let p = GetProvincesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_queues_params_roundtrips() {
    let p = GetQueuesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetQueuesParams {
        queue: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_rate_centers_can_params_roundtrips() {
    let p = GetRateCentersCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRateCentersCANParams {
        province: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_rate_centers_usa_params_roundtrips() {
    let p = GetRateCentersUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRateCentersUSAParams {
        state: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_rates_params_roundtrips() {
    let p = GetRatesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRatesParams {
        package: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_recording_file_params_roundtrips() {
    let p = GetRecordingFileParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRecordingFileParams {
        recording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_recordings_params_roundtrips() {
    let p = GetRecordingsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRecordingsParams {
        recording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_registration_status_params_roundtrips() {
    let p = GetRegistrationStatusParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRegistrationStatusParams {
        account: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_report_estimated_hold_time_params_roundtrips() {
    let p = GetReportEstimatedHoldTimeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetReportEstimatedHoldTimeParams {
        r#type: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_reseller_balance_params_roundtrips() {
    let p = GetResellerBalanceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetResellerBalanceParams {
        client: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_reseller_cdr_params_roundtrips() {
    let p = GetResellerCDRParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetResellerCDRParams {
        date_from: Some(voip_ms::chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_reseller_mms_params_roundtrips() {
    let p = GetResellerMMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetResellerMMSParams {
        mms: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_reseller_sms_params_roundtrips() {
    let p = GetResellerSMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetResellerSMSParams {
        sms: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_ring_groups_params_roundtrips() {
    let p = GetRingGroupsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRingGroupsParams {
        ring_group: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_ring_strategies_params_roundtrips() {
    let p = GetRingStrategiesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRingStrategiesParams {
        strategy: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_routes_params_roundtrips() {
    let p = GetRoutesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetRoutesParams {
        route: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_sip_uris_params_roundtrips() {
    let p = GetSIPURIsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetSIPURIsParams {
        sip_uri: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_sms_params_roundtrips() {
    let p = GetSMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetSMSParams {
        sms: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_servers_info_params_roundtrips() {
    let p = GetServersInfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetServersInfoParams {
        server_pop: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_states_params_roundtrips() {
    let p = GetStatesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_static_members_params_roundtrips() {
    let p = GetStaticMembersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetStaticMembersParams {
        queue: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_sub_accounts_params_roundtrips() {
    let p = GetSubAccountsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetSubAccountsParams {
        account: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_termination_rates_params_roundtrips() {
    let p = GetTerminationRatesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetTerminationRatesParams {
        query: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_time_conditions_params_roundtrips() {
    let p = GetTimeConditionsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetTimeConditionsParams {
        timecondition: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_timezones_params_roundtrips() {
    let p = GetTimezonesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetTimezonesParams {
        timezone: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_transaction_history_params_roundtrips() {
    let p = GetTransactionHistoryParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetTransactionHistoryParams {
        date_from: Some(voip_ms::chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_vpris_params_roundtrips() {
    let p = GetVPRIsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());
}

#[test]
fn get_voicemail_attachment_formats_params_roundtrips() {
    let p = GetVoicemailAttachmentFormatsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailAttachmentFormatsParams {
        email_attachment_format: Some(EmailAttachmentFormat::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemail_folders_params_roundtrips() {
    let p = GetVoicemailFoldersParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailFoldersParams {
        folder: Some(VoicemailFolder::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemail_message_file_params_roundtrips() {
    let p = GetVoicemailMessageFileParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailMessageFileParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemail_messages_params_roundtrips() {
    let p = GetVoicemailMessagesParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailMessagesParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemail_setups_params_roundtrips() {
    let p = GetVoicemailSetupsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailSetupsParams {
        voicemailsetup: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemail_transcriptions_params_roundtrips() {
    let p = GetVoicemailTranscriptionsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailTranscriptionsParams {
        account: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn get_voicemails_params_roundtrips() {
    let p = GetVoicemailsParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = GetVoicemailsParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn mail_fax_message_pdf_params_roundtrips() {
    let p = MailFAXMessagePDFParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = MailFAXMessagePDFParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn mark_listened_voicemail_message_params_roundtrips() {
    let p = MarkListenedVoicemailMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = MarkListenedVoicemailMessageParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn mark_urgent_voicemail_message_params_roundtrips() {
    let p = MarkUrgentVoicemailMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = MarkUrgentVoicemailMessageParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn move_fax_message_params_roundtrips() {
    let p = MoveFAXMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = MoveFAXMessageParams {
        fax_id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn move_folder_voicemail_message_params_roundtrips() {
    let p = MoveFolderVoicemailMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = MoveFolderVoicemailMessageParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_did_params_roundtrips() {
    let p = OrderDIDParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderDIDParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_did_international_geographic_params_roundtrips() {
    let p = OrderDIDInternationalGeographicParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderDIDInternationalGeographicParams {
        location_id: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_did_international_national_params_roundtrips() {
    let p = OrderDIDInternationalNationalParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderDIDInternationalNationalParams {
        location_id: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_did_international_toll_free_params_roundtrips() {
    let p = OrderDIDInternationalTollFreeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderDIDInternationalTollFreeParams {
        location_id: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_did_virtual_params_roundtrips() {
    let p = OrderDIDVirtualParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderDIDVirtualParams {
        digits: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_fax_number_params_roundtrips() {
    let p = OrderFAXNumberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderFAXNumberParams {
        location: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_toll_free_params_roundtrips() {
    let p = OrderTollFreeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderTollFreeParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn order_vanity_params_roundtrips() {
    let p = OrderVanityParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = OrderVanityParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn remove_did_vpri_params_roundtrips() {
    let p = RemoveDIDvPRIParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = RemoveDIDvPRIParams {
        vpri: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_dids_can_params_roundtrips() {
    let p = SearchDIDsCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchDIDsCANParams {
        province: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_dids_usa_params_roundtrips() {
    let p = SearchDIDsUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchDIDsUSAParams {
        state: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_fax_area_code_can_params_roundtrips() {
    let p = SearchFAXAreaCodeCANParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchFAXAreaCodeCANParams {
        area_code: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_fax_area_code_usa_params_roundtrips() {
    let p = SearchFAXAreaCodeUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchFAXAreaCodeUSAParams {
        area_code: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_toll_free_can_us_params_roundtrips() {
    let p = SearchTollFreeCANUSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchTollFreeCANUSParams {
        r#type: Some(SearchType::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_toll_free_usa_params_roundtrips() {
    let p = SearchTollFreeUSAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchTollFreeUSAParams {
        r#type: Some(SearchType::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn search_vanity_params_roundtrips() {
    let p = SearchVanityParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SearchVanityParams {
        r#type: Some(VanityType::from_wire("1")),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn send_call_recording_email_params_roundtrips() {
    let p = SendCallRecordingEmailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SendCallRecordingEmailParams {
        callrecording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn send_fax_message_params_roundtrips() {
    let p = SendFAXMessageParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SendFAXMessageParams {
        to_number: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn send_mms_params_roundtrips() {
    let p = SendMMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SendMMSParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn send_sms_params_roundtrips() {
    let p = SendSMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SendSMSParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn send_voicemail_email_params_roundtrips() {
    let p = SendVoicemailEmailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SendVoicemailEmailParams {
        mailbox: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_call_hunting_params_roundtrips() {
    let p = SetCallHuntingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetCallHuntingParams {
        callhunting: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_call_parking_params_roundtrips() {
    let p = SetCallParkingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetCallParkingParams {
        callparking: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_callback_params_roundtrips() {
    let p = SetCallbackParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetCallbackParams {
        callback: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_caller_id_filtering_params_roundtrips() {
    let p = SetCallerIDFilteringParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetCallerIDFilteringParams {
        filter: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_client_params_roundtrips() {
    let p = SetClientParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetClientParams {
        client: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_client_threshold_params_roundtrips() {
    let p = SetClientThresholdParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetClientThresholdParams {
        client: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_conference_params_roundtrips() {
    let p = SetConferenceParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetConferenceParams {
        conference: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_conference_member_params_roundtrips() {
    let p = SetConferenceMemberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetConferenceMemberParams {
        member: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_did_billing_type_params_roundtrips() {
    let p = SetDIDBillingTypeParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDIDBillingTypeParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_did_info_params_roundtrips() {
    let p = SetDIDInfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDIDInfoParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_did_pop_params_roundtrips() {
    let p = SetDIDPOPParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDIDPOPParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_did_routing_params_roundtrips() {
    let p = SetDIDRoutingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDIDRoutingParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_did_voicemail_params_roundtrips() {
    let p = SetDIDVoicemailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDIDVoicemailParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_disa_params_roundtrips() {
    let p = SetDISAParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetDISAParams {
        disa: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_email_to_fax_params_roundtrips() {
    let p = SetEmailToFAXParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetEmailToFAXParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_fax_folder_params_roundtrips() {
    let p = SetFAXFolderParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetFAXFolderParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_fax_number_email_params_roundtrips() {
    let p = SetFAXNumberEmailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetFAXNumberEmailParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_fax_number_info_params_roundtrips() {
    let p = SetFAXNumberInfoParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetFAXNumberInfoParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_fax_number_url_callback_params_roundtrips() {
    let p = SetFAXNumberURLCallbackParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetFAXNumberURLCallbackParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_forwarding_params_roundtrips() {
    let p = SetForwardingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetForwardingParams {
        forwarding: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_ivr_params_roundtrips() {
    let p = SetIVRParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetIVRParams {
        ivr: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_location_params_roundtrips() {
    let p = SetLocationParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetLocationParams {
        name: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_music_on_hold_params_roundtrips() {
    let p = SetMusicOnHoldParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetMusicOnHoldParams {
        name: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_phonebook_params_roundtrips() {
    let p = SetPhonebookParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetPhonebookParams {
        phonebook: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_phonebook_group_params_roundtrips() {
    let p = SetPhonebookGroupParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetPhonebookGroupParams {
        phonebook: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_queue_params_roundtrips() {
    let p = SetQueueParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetQueueParams {
        queue: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_recording_params_roundtrips() {
    let p = SetRecordingParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetRecordingParams {
        recording: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_ring_group_params_roundtrips() {
    let p = SetRingGroupParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetRingGroupParams {
        ring_group: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_sip_uri_params_roundtrips() {
    let p = SetSIPURIParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetSIPURIParams {
        sip_uri: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_sms_params_roundtrips() {
    let p = SetSMSParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetSMSParams {
        did: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_static_member_params_roundtrips() {
    let p = SetStaticMemberParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetStaticMemberParams {
        member: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_sub_account_params_roundtrips() {
    let p = SetSubAccountParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetSubAccountParams {
        id: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_time_condition_params_roundtrips() {
    let p = SetTimeConditionParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetTimeConditionParams {
        timecondition: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn set_voicemail_params_roundtrips() {
    let p = SetVoicemailParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SetVoicemailParams {
        mailbox: Some(1),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn signup_client_params_roundtrips() {
    let p = SignupClientParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = SignupClientParams {
        firstname: Some("x".into()),
        ..Default::default()
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn unconnect_did_params_roundtrips() {
    let p = UnconnectDIDParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = UnconnectDIDParams {
        did: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}

#[test]
fn unconnect_fax_params_roundtrips() {
    let p = UnconnectFAXParams::default();
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.is_object());

    let p2 = UnconnectFAXParams {
        did: Some("x".into()),
    };
    let v2 = serde_json::to_value(&p2).unwrap();
    assert!(v2.is_object());
}
