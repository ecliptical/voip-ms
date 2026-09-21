//! Codegen-drift oracle: every generated `*Response` struct deserializes.
//!
//! Hand-maintained on purpose. Each of the generated response structs must
//! decode from a bare envelope -- `status` is the one required field, and
//! every other is `Option`/`Vec`/`Map` with `#[serde(default)]`. A regen that
//! makes another field non-optional, drops a `default`, or renames a struct
//! breaks this suite; that failure is the intended signal. Update by hand when
//! the generated surface changes (see DEVELOPMENT.md).

use voip_ms::serde_json::{self, json};
use voip_ms::*;

#[test]
fn add_charge_response_deserializes() {
    assert!(serde_json::from_value::<AddChargeResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn add_lnpfile_response_deserializes() {
    assert!(serde_json::from_value::<AddLNPFileResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn add_lnpport_response_deserializes() {
    assert!(serde_json::from_value::<AddLNPPortResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn add_member_to_conference_response_deserializes() {
    assert!(
        serde_json::from_value::<AddMemberToConferenceResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn add_payment_response_deserializes() {
    assert!(serde_json::from_value::<AddPaymentResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn assign_didv_priresponse_deserializes() {
    assert!(
        serde_json::from_value::<AssignDIDvPRIResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn back_order_didcanresponse_deserializes() {
    assert!(
        serde_json::from_value::<BackOrderDIDCANResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn back_order_didusaresponse_deserializes() {
    assert!(
        serde_json::from_value::<BackOrderDIDUSAResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn cancel_didresponse_deserializes() {
    assert!(serde_json::from_value::<CancelDIDResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn cancel_faxnumber_response_deserializes() {
    assert!(
        serde_json::from_value::<CancelFAXNumberResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn connect_didresponse_deserializes() {
    assert!(serde_json::from_value::<ConnectDIDResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn connect_faxresponse_deserializes() {
    assert!(serde_json::from_value::<ConnectFAXResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn create_sub_account_response_deserializes() {
    assert!(
        serde_json::from_value::<CreateSubAccountResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn create_voicemail_response_deserializes() {
    assert!(
        serde_json::from_value::<CreateVoicemailResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_call_hunting_response_deserializes() {
    assert!(
        serde_json::from_value::<DelCallHuntingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_call_parking_response_deserializes() {
    assert!(
        serde_json::from_value::<DelCallParkingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_call_recording_response_deserializes() {
    assert!(
        serde_json::from_value::<DelCallRecordingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_callback_response_deserializes() {
    assert!(serde_json::from_value::<DelCallbackResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_caller_idfiltering_response_deserializes() {
    assert!(
        serde_json::from_value::<DelCallerIDFilteringResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn del_client_response_deserializes() {
    assert!(serde_json::from_value::<DelClientResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_conference_response_deserializes() {
    assert!(
        serde_json::from_value::<DelConferenceResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_conference_member_response_deserializes() {
    assert!(
        serde_json::from_value::<DelConferenceMemberResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn del_disaresponse_deserializes() {
    assert!(serde_json::from_value::<DelDISAResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_email_to_faxresponse_deserializes() {
    assert!(
        serde_json::from_value::<DelEmailToFAXResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_faxfolder_response_deserializes() {
    assert!(serde_json::from_value::<DelFAXFolderResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_forwarding_response_deserializes() {
    assert!(
        serde_json::from_value::<DelForwardingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_ivrresponse_deserializes() {
    assert!(serde_json::from_value::<DelIVRResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_location_response_deserializes() {
    assert!(serde_json::from_value::<DelLocationResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_member_from_conference_response_deserializes() {
    assert!(
        serde_json::from_value::<DelMemberFromConferenceResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn del_messages_response_deserializes() {
    assert!(serde_json::from_value::<DelMessagesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_music_on_hold_response_deserializes() {
    assert!(
        serde_json::from_value::<DelMusicOnHoldResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_phonebook_response_deserializes() {
    assert!(serde_json::from_value::<DelPhonebookResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_phonebook_group_response_deserializes() {
    assert!(
        serde_json::from_value::<DelPhonebookGroupResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_queue_response_deserializes() {
    assert!(serde_json::from_value::<DelQueueResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_recording_response_deserializes() {
    assert!(serde_json::from_value::<DelRecordingResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_ring_group_response_deserializes() {
    assert!(serde_json::from_value::<DelRingGroupResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_sipuriresponse_deserializes() {
    assert!(serde_json::from_value::<DelSIPURIResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn del_static_member_response_deserializes() {
    assert!(
        serde_json::from_value::<DelStaticMemberResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_sub_account_response_deserializes() {
    assert!(
        serde_json::from_value::<DelSubAccountResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_time_condition_response_deserializes() {
    assert!(
        serde_json::from_value::<DelTimeConditionResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn del_voicemail_response_deserializes() {
    assert!(serde_json::from_value::<DelVoicemailResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn delete_faxmessage_response_deserializes() {
    assert!(
        serde_json::from_value::<DeleteFAXMessageResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn delete_mms_response_deserializes() {
    let r: DeleteMMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<DeleteMMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn delete_sms_response_deserializes() {
    let r: DeleteSMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<DeleteSMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn e911_address_types_response_type_deserializes() {
    let r: E911AddressTypesResponseType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<E911AddressTypesResponseType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn e911_address_types_response_deserializes() {
    let r: E911AddressTypesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<E911AddressTypesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn e911_cancel_response_deserializes() {
    let r: E911CancelResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911CancelResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn e911_info_response_info_deserializes() {
    let r: E911InfoResponseInfo = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911InfoResponseInfo>(json!({ "status": "success" })).is_ok());
}

#[test]
fn e911_info_response_deserializes() {
    let r: E911InfoResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911InfoResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn e911_provision_response_deserializes() {
    let r: E911ProvisionResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<E911ProvisionResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn e911_provision_manually_response_deserializes() {
    let r: E911ProvisionManuallyResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<E911ProvisionManuallyResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn e911_update_response_deserializes() {
    let r: E911UpdateResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911UpdateResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn e911_validate_response_deserializes() {
    let r: E911ValidateResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911ValidateResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_allowed_codecs_response_allowed_codec_deserializes() {
    let r: GetAllowedCodecsResponseAllowedCodec =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetAllowedCodecsResponseAllowedCodec>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_allowed_codecs_response_deserializes() {
    let r: GetAllowedCodecsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetAllowedCodecsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_auth_types_response_auth_type_deserializes() {
    let r: GetAuthTypesResponseAuthType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetAuthTypesResponseAuthType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_auth_types_response_deserializes() {
    let r: GetAuthTypesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetAuthTypesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_back_orders_response_back_order_deserializes() {
    let r: GetBackOrdersResponseBackOrder =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBackOrdersResponseBackOrder>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_back_orders_response_deserializes() {
    let r: GetBackOrdersResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBackOrdersResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_balance_response_balance_deserializes() {
    let r: GetBalanceResponseBalance =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBalanceResponseBalance>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_balance_response_deserializes() {
    let r: GetBalanceResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBalanceResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_balance_management_response_balance_management_deserializes() {
    let r: GetBalanceManagementResponseBalanceManagement =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBalanceManagementResponseBalanceManagement>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_balance_management_response_deserializes() {
    let r: GetBalanceManagementResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBalanceManagementResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_cdr_response_cdr_deserializes() {
    let r: GetCDRResponseCDR = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCDRResponseCDR>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_cdr_response_deserializes() {
    let r: GetCDRResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCDRResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_call_accounts_response_account_deserializes() {
    let r: GetCallAccountsResponseAccount =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallAccountsResponseAccount>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_call_accounts_response_deserializes() {
    let r: GetCallAccountsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallAccountsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_billing_response_call_billing_deserializes() {
    let r: GetCallBillingResponseCallBilling =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallBillingResponseCallBilling>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_call_billing_response_deserializes() {
    let r: GetCallBillingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallBillingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_huntings_response_call_hunting_deserializes() {
    let r: GetCallHuntingsResponseCallHunting =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallHuntingsResponseCallHunting>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_call_huntings_response_deserializes() {
    let r: GetCallHuntingsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallHuntingsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_parking_response_call_hunting_deserializes() {
    let r: GetCallParkingResponseCallHunting =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallParkingResponseCallHunting>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_call_parking_response_deserializes() {
    let r: GetCallParkingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallParkingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_recording_response_deserializes() {
    let r: GetCallRecordingResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallRecordingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_recordings_response_recording_deserializes() {
    let r: GetCallRecordingsResponseRecording =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallRecordingsResponseRecording>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_call_recordings_response_deserializes() {
    let r: GetCallRecordingsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallRecordingsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_call_transcriptions_response_transcription_recognized_phrase_deserializes() {
    let r: GetCallTranscriptionsResponseTranscriptionRecognizedPhrase =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTranscriptionsResponseTranscriptionRecognizedPhrase>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_call_transcriptions_response_transcription_deserializes() {
    let r: GetCallTranscriptionsResponseTranscription =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTranscriptionsResponseTranscription>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_call_transcriptions_response_deserializes() {
    let r: GetCallTranscriptionsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTranscriptionsResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_call_types_response_call_type_deserializes() {
    let r: GetCallTypesResponseCallType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTypesResponseCallType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_call_types_response_deserializes() {
    let r: GetCallTypesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallTypesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_callbacks_response_callback_deserializes() {
    let r: GetCallbacksResponseCallback =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallbacksResponseCallback>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_callbacks_response_deserializes() {
    let r: GetCallbacksResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallbacksResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_caller_id_filtering_response_filtering_deserializes() {
    let r: GetCallerIDFilteringResponseFiltering =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallerIDFilteringResponseFiltering>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_caller_id_filtering_response_deserializes() {
    let r: GetCallerIDFilteringResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallerIDFilteringResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_carriers_response_carrier_deserializes() {
    let r: GetCarriersResponseCarrier =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCarriersResponseCarrier>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_carriers_response_deserializes() {
    let r: GetCarriersResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCarriersResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_charges_response_charge_deserializes() {
    let r: GetChargesResponseCharge =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetChargesResponseCharge>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_charges_response_deserializes() {
    let r: GetChargesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetChargesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_client_packages_response_package_deserializes() {
    let r: GetClientPackagesResponsePackage =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientPackagesResponsePackage>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_client_packages_response_deserializes() {
    let r: GetClientPackagesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientPackagesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_client_threshold_response_threshold_information_deserializes() {
    let r: GetClientThresholdResponseThresholdInformation =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientThresholdResponseThresholdInformation>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_client_threshold_response_deserializes() {
    let r: GetClientThresholdResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientThresholdResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_clients_response_client_deserializes() {
    let r: GetClientsResponseClient =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientsResponseClient>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_clients_response_deserializes() {
    let r: GetClientsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_conference_response_conference_deserializes() {
    let r: GetConferenceResponseConference =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceResponseConference>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_conference_response_deserializes() {
    let r: GetConferenceResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_conference_members_response_member_deserializes() {
    let r: GetConferenceMembersResponseMember =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceMembersResponseMember>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_conference_members_response_deserializes() {
    let r: GetConferenceMembersResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceMembersResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_conference_recording_file_response_recording_deserializes() {
    let r: GetConferenceRecordingFileResponseRecording =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceRecordingFileResponseRecording>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_conference_recording_file_response_deserializes() {
    let r: GetConferenceRecordingFileResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceRecordingFileResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_conference_recordings_response_recording_deserializes() {
    let r: GetConferenceRecordingsResponseRecording =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceRecordingsResponseRecording>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_conference_recordings_response_deserializes() {
    let r: GetConferenceRecordingsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceRecordingsResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_countries_response_country_deserializes() {
    let r: GetCountriesResponseCountry =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCountriesResponseCountry>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_countries_response_deserializes() {
    let r: GetCountriesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCountriesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_did_countries_response_country_deserializes() {
    let r: GetDIDCountriesResponseCountry =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDCountriesResponseCountry>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_did_countries_response_deserializes() {
    let r: GetDIDCountriesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDCountriesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_dids_can_response_did_deserializes() {
    let r: GetDIDsCANResponseDID = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsCANResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_dids_can_response_deserializes() {
    let r: GetDIDsCANResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsCANResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_dids_info_response_did_deserializes() {
    let r: GetDIDsInfoResponseDID = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInfoResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_dids_info_response_deserializes() {
    let r: GetDIDsInfoResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInfoResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_dids_international_geographic_response_location_deserializes() {
    let r: GetDIDsInternationalGeographicResponseLocation =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalGeographicResponseLocation>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_international_geographic_response_deserializes() {
    let r: GetDIDsInternationalGeographicResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalGeographicResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_international_national_response_location_deserializes() {
    let r: GetDIDsInternationalNationalResponseLocation =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalNationalResponseLocation>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_international_national_response_deserializes() {
    let r: GetDIDsInternationalNationalResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalNationalResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_international_toll_free_response_location_deserializes() {
    let r: GetDIDsInternationalTollFreeResponseLocation =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalTollFreeResponseLocation>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_international_toll_free_response_deserializes() {
    let r: GetDIDsInternationalTollFreeResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalTollFreeResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_dids_usa_response_did_deserializes() {
    let r: GetDIDsUSAResponseDID = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsUSAResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_dids_usa_response_deserializes() {
    let r: GetDIDsUSAResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsUSAResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_did_vpri_response_deserializes() {
    let r: GetDIDvPRIResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDvPRIResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_disas_response_disa_deserializes() {
    let r: GetDISAsResponseDISA = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDISAsResponseDISA>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_disas_response_deserializes() {
    let r: GetDISAsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDISAsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_dtmf_modes_response_dtmf_mode_deserializes() {
    let r: GetDTMFModesResponseDTMFMode =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDTMFModesResponseDTMFMode>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_dtmf_modes_response_deserializes() {
    let r: GetDTMFModesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDTMFModesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_deposits_response_deposit_deserializes() {
    let r: GetDepositsResponseDeposit =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDepositsResponseDeposit>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_deposits_response_deserializes() {
    let r: GetDepositsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDepositsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_device_types_response_device_type_deserializes() {
    let r: GetDeviceTypesResponseDeviceType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDeviceTypesResponseDeviceType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_device_types_response_deserializes() {
    let r: GetDeviceTypesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDeviceTypesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_email_to_fax_response_email_to_fax_deserializes() {
    let r: GetEmailToFAXResponseEmailToFAX =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetEmailToFAXResponseEmailToFAX>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_email_to_fax_response_deserializes() {
    let r: GetEmailToFAXResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetEmailToFAXResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_folders_response_folder_deserializes() {
    let r: GetFAXFoldersResponseFolder =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXFoldersResponseFolder>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_folders_response_deserializes() {
    let r: GetFAXFoldersResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXFoldersResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_message_pdf_response_deserializes() {
    let r: GetFAXMessagePDFResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXMessagePDFResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_messages_response_fax_deserializes() {
    let r: GetFAXMessagesResponseFAX =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXMessagesResponseFAX>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_messages_response_deserializes() {
    let r: GetFAXMessagesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXMessagesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_numbers_info_response_number_deserializes() {
    let r: GetFAXNumbersInfoResponseNumber =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXNumbersInfoResponseNumber>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_numbers_info_response_deserializes() {
    let r: GetFAXNumbersInfoResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXNumbersInfoResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_numbers_portability_response_deserializes() {
    let r: GetFAXNumbersPortabilityResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXNumbersPortabilityResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_provinces_response_province_deserializes() {
    let r: GetFAXProvincesResponseProvince =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXProvincesResponseProvince>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_provinces_response_deserializes() {
    let r: GetFAXProvincesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXProvincesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_rate_centers_can_response_ratecenter_deserializes() {
    let r: GetFAXRateCentersCANResponseRatecenter =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXRateCentersCANResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_fax_rate_centers_can_response_deserializes() {
    let r: GetFAXRateCentersCANResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXRateCentersCANResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_rate_centers_usa_response_ratecenter_deserializes() {
    let r: GetFAXRateCentersUSAResponseRatecenter =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXRateCentersUSAResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_fax_rate_centers_usa_response_deserializes() {
    let r: GetFAXRateCentersUSAResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXRateCentersUSAResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_fax_states_response_state_deserializes() {
    let r: GetFAXStatesResponseState =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetFAXStatesResponseState>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_fax_states_response_deserializes() {
    let r: GetFAXStatesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXStatesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_forwardings_response_forwarding_deserializes() {
    let r: GetForwardingsResponseForwarding =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetForwardingsResponseForwarding>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_forwardings_response_deserializes() {
    let r: GetForwardingsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetForwardingsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_ip_response_deserializes() {
    let r: GetIPResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIPResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_ivrs_response_ivr_deserializes() {
    let r: GetIVRsResponseIVR = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIVRsResponseIVR>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_ivrs_response_deserializes() {
    let r: GetIVRsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIVRsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_international_types_response_type_deserializes() {
    let r: GetInternationalTypesResponseType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetInternationalTypesResponseType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_international_types_response_deserializes() {
    let r: GetInternationalTypesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetInternationalTypesResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_join_when_empty_types_response_type_deserializes() {
    let r: GetJoinWhenEmptyTypesResponseType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetJoinWhenEmptyTypesResponseType>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_join_when_empty_types_response_deserializes() {
    let r: GetJoinWhenEmptyTypesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetJoinWhenEmptyTypesResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_lnp_attach_response_deserializes() {
    let r: GetLNPAttachResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPAttachResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_lnp_attach_list_response_list_deserializes() {
    let r: GetLNPAttachListResponseList =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPAttachListResponseList>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_lnp_attach_list_response_deserializes() {
    let r: GetLNPAttachListResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPAttachListResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_details_response_number_deserializes() {
    let r: GetLNPDetailsResponseNumber =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPDetailsResponseNumber>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_lnp_details_response_note_deserializes() {
    let r: GetLNPDetailsResponseNote =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPDetailsResponseNote>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_details_response_attachment_deserializes() {
    let r: GetLNPDetailsResponseAttachment =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPDetailsResponseAttachment>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_lnp_details_response_deserializes() {
    let r: GetLNPDetailsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPDetailsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_list_response_list_deserializes() {
    let r: GetLNPListResponseList = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPListResponseList>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_list_response_deserializes() {
    let r: GetLNPListResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPListResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_lnp_list_status_response_deserializes() {
    let r: GetLNPListStatusResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPListStatusResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_notes_response_list_deserializes() {
    let r: GetLNPNotesResponseList =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetLNPNotesResponseList>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_lnp_notes_response_deserializes() {
    let r: GetLNPNotesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPNotesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_lnp_status_response_deserializes() {
    let r: GetLNPStatusResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPStatusResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_languages_response_language_deserializes() {
    assert!(
        serde_json::from_value::<GetLanguagesResponseLanguage>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_languages_response_deserializes() {
    assert!(serde_json::from_value::<GetLanguagesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_locales_response_locale_deserializes() {
    assert!(
        serde_json::from_value::<GetLocalesResponseLocale>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_locales_response_deserializes() {
    assert!(serde_json::from_value::<GetLocalesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_locations_response_deserializes() {
    assert!(serde_json::from_value::<GetLocationsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_lock_international_response_lock_international_deserializes() {
    assert!(
        serde_json::from_value::<GetLockInternationalResponseLockInternational>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_lock_international_response_deserializes() {
    assert!(
        serde_json::from_value::<GetLockInternationalResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_mms_response_sms_deserializes() {
    assert!(serde_json::from_value::<GetMMSResponseSMS>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_mms_response_deserializes() {
    assert!(serde_json::from_value::<GetMMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_media_mms_response_deserializes() {
    assert!(serde_json::from_value::<GetMediaMMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_music_on_hold_response_music_on_hold_deserializes() {
    assert!(
        serde_json::from_value::<GetMusicOnHoldResponseMusicOnHold>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_music_on_hold_response_deserializes() {
    assert!(
        serde_json::from_value::<GetMusicOnHoldResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_nat_response_nat_deserializes() {
    assert!(serde_json::from_value::<GetNATResponseNAT>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_nat_response_deserializes() {
    assert!(serde_json::from_value::<GetNATResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_packages_response_package_deserializes() {
    assert!(
        serde_json::from_value::<GetPackagesResponsePackage>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_packages_response_deserializes() {
    assert!(serde_json::from_value::<GetPackagesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_phonebook_response_phonebook_deserializes() {
    assert!(
        serde_json::from_value::<GetPhonebookResponsePhonebook>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_phonebook_response_deserializes() {
    assert!(serde_json::from_value::<GetPhonebookResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_phonebook_groups_response_phonebook_deserializes() {
    assert!(
        serde_json::from_value::<GetPhonebookGroupsResponsePhonebook>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_phonebook_groups_response_deserializes() {
    assert!(
        serde_json::from_value::<GetPhonebookGroupsResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_play_instructions_response_play_instruction_deserializes() {
    assert!(
        serde_json::from_value::<GetPlayInstructionsResponsePlayInstruction>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_play_instructions_response_deserializes() {
    assert!(
        serde_json::from_value::<GetPlayInstructionsResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_portability_response_plan_deserializes() {
    assert!(
        serde_json::from_value::<GetPortabilityResponsePlan>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_portability_response_deserializes() {
    assert!(
        serde_json::from_value::<GetPortabilityResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_protocols_response_protocol_deserializes() {
    assert!(
        serde_json::from_value::<GetProtocolsResponseProtocol>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_protocols_response_deserializes() {
    assert!(serde_json::from_value::<GetProtocolsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_provinces_response_province_deserializes() {
    assert!(
        serde_json::from_value::<GetProvincesResponseProvince>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_provinces_response_deserializes() {
    assert!(serde_json::from_value::<GetProvincesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_queues_response_queue_deserializes() {
    assert!(
        serde_json::from_value::<GetQueuesResponseQueue>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_queues_response_deserializes() {
    assert!(serde_json::from_value::<GetQueuesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_rate_centers_can_response_ratecenter_deserializes() {
    assert!(
        serde_json::from_value::<GetRateCentersCANResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_rate_centers_can_response_deserializes() {
    assert!(
        serde_json::from_value::<GetRateCentersCANResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_rate_centers_usa_response_ratecenter_deserializes() {
    assert!(
        serde_json::from_value::<GetRateCentersUSAResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_rate_centers_usa_response_deserializes() {
    assert!(
        serde_json::from_value::<GetRateCentersUSAResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_rates_response_rate_deserializes() {
    assert!(serde_json::from_value::<GetRatesResponseRate>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_rates_response_deserializes() {
    assert!(serde_json::from_value::<GetRatesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_recording_file_response_recording_deserializes() {
    assert!(
        serde_json::from_value::<GetRecordingFileResponseRecording>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_recording_file_response_deserializes() {
    assert!(
        serde_json::from_value::<GetRecordingFileResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_recordings_response_recording_deserializes() {
    assert!(
        serde_json::from_value::<GetRecordingsResponseRecording>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_recordings_response_deserializes() {
    assert!(
        serde_json::from_value::<GetRecordingsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_registration_status_response_registration_deserializes() {
    assert!(
        serde_json::from_value::<GetRegistrationStatusResponseRegistration>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_registration_status_response_deserializes() {
    assert!(
        serde_json::from_value::<GetRegistrationStatusResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_report_estimated_hold_time_response_type_deserializes() {
    let r: GetReportEstimatedHoldTimeResponseType =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetReportEstimatedHoldTimeResponseType>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_report_estimated_hold_time_response_deserializes() {
    let r: GetReportEstimatedHoldTimeResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetReportEstimatedHoldTimeResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_reseller_balance_response_balance_deserializes() {
    let r: GetResellerBalanceResponseBalance =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerBalanceResponseBalance>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_reseller_balance_response_deserializes() {
    let r: GetResellerBalanceResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerBalanceResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_reseller_cdr_response_cdr_deserializes() {
    let r: GetResellerCDRResponseCDR =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerCDRResponseCDR>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_reseller_cdr_response_deserializes() {
    let r: GetResellerCDRResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerCDRResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_reseller_mms_response_sms_deserializes() {
    let r: GetResellerMMSResponseSMS =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerMMSResponseSMS>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_reseller_mms_response_deserializes() {
    let r: GetResellerMMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerMMSResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_reseller_sms_response_sms_deserializes() {
    let r: GetResellerSMSResponseSMS =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerSMSResponseSMS>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_reseller_sms_response_deserializes() {
    let r: GetResellerSMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetResellerSMSResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_ring_groups_response_ring_group_deserializes() {
    let r: GetRingGroupsResponseRingGroup =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetRingGroupsResponseRingGroup>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_ring_groups_response_deserializes() {
    let r: GetRingGroupsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetRingGroupsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_ring_strategies_response_strategy_deserializes() {
    let r: GetRingStrategiesResponseStrategy =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetRingStrategiesResponseStrategy>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_ring_strategies_response_deserializes() {
    let r: GetRingStrategiesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetRingStrategiesResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_routes_response_route_deserializes() {
    let r: GetRoutesResponseRoute = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetRoutesResponseRoute>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_routes_response_deserializes() {
    let r: GetRoutesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRoutesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_sip_uris_response_sip_uri_deserializes() {
    let r: GetSIPURIsResponseSIPURI =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetSIPURIsResponseSIPURI>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_sip_uris_response_deserializes() {
    let r: GetSIPURIsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSIPURIsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_sms_response_sms_deserializes() {
    let r: GetSMSResponseSMS = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSMSResponseSMS>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_sms_response_deserializes() {
    let r: GetSMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_servers_info_response_server_deserializes() {
    let r: GetServersInfoResponseServer =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetServersInfoResponseServer>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_servers_info_response_deserializes() {
    let r: GetServersInfoResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetServersInfoResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_states_response_state_deserializes() {
    let r: GetStatesResponseState = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetStatesResponseState>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_states_response_deserializes() {
    let r: GetStatesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetStatesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_static_members_response_member_deserializes() {
    let r: GetStaticMembersResponseMember =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetStaticMembersResponseMember>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_static_members_response_deserializes() {
    let r: GetStaticMembersResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetStaticMembersResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_sub_accounts_response_account_deserializes() {
    let r: GetSubAccountsResponseAccount =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetSubAccountsResponseAccount>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_sub_accounts_response_deserializes() {
    let r: GetSubAccountsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetSubAccountsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_termination_rates_response_route_deserializes() {
    let r: GetTerminationRatesResponseRoute =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTerminationRatesResponseRoute>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_termination_rates_response_rate_deserializes() {
    let r: GetTerminationRatesResponseRate =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTerminationRatesResponseRate>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_termination_rates_response_deserializes() {
    let r: GetTerminationRatesResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTerminationRatesResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_time_conditions_response_timecondition_deserializes() {
    let r: GetTimeConditionsResponseTimecondition =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTimeConditionsResponseTimecondition>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_time_conditions_response_deserializes() {
    let r: GetTimeConditionsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTimeConditionsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn get_timezones_response_timezone_deserializes() {
    let r: GetTimezonesResponseTimezone =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTimezonesResponseTimezone>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_timezones_response_deserializes() {
    let r: GetTimezonesResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTimezonesResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_transaction_history_response_transaction_deserializes() {
    let r: GetTransactionHistoryResponseTransaction =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTransactionHistoryResponseTransaction>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_transaction_history_response_deserializes() {
    let r: GetTransactionHistoryResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetTransactionHistoryResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_vpris_response_vpri_deserializes() {
    let r: GetVPRIsResponseVPRI = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetVPRIsResponseVPRI>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_vpris_response_deserializes() {
    let r: GetVPRIsResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetVPRIsResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn get_voicemail_attachment_formats_response_email_attachment_format_deserializes() {
    let r: GetVoicemailAttachmentFormatsResponseEmailAttachmentFormat =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetVoicemailAttachmentFormatsResponseEmailAttachmentFormat>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_attachment_formats_response_deserializes() {
    let r: GetVoicemailAttachmentFormatsResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetVoicemailAttachmentFormatsResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_folders_response_folder_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailFoldersResponseFolder>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemail_folders_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailFoldersResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemail_message_file_response_message_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailMessageFileResponseMessage>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_message_file_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailMessageFileResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemail_messages_response_message_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailMessagesResponseMessage>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_messages_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailMessagesResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemail_setups_response_voicemailsetup_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailSetupsResponseVoicemailsetup>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_setups_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailSetupsResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemail_transcriptions_response_message_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailTranscriptionsResponseMessage>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_transcriptions_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailTranscriptionsResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn get_voicemails_response_voicemail_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailsResponseVoicemail>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn get_voicemails_response_deserializes() {
    assert!(
        serde_json::from_value::<GetVoicemailsResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn mail_fax_message_pdf_response_deserializes() {
    assert!(
        serde_json::from_value::<MailFAXMessagePDFResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn mark_listened_voicemail_message_response_deserializes() {
    assert!(
        serde_json::from_value::<MarkListenedVoicemailMessageResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn mark_urgent_voicemail_message_response_deserializes() {
    assert!(
        serde_json::from_value::<MarkUrgentVoicemailMessageResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn move_fax_message_response_deserializes() {
    assert!(
        serde_json::from_value::<MoveFAXMessageResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn move_folder_voicemail_message_response_deserializes() {
    assert!(
        serde_json::from_value::<MoveFolderVoicemailMessageResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn order_did_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn order_did_international_geographic_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderDIDInternationalGeographicResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn order_did_international_national_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderDIDInternationalNationalResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn order_did_international_toll_free_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderDIDInternationalTollFreeResponse>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn order_did_virtual_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderDIDVirtualResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn order_fax_number_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderFAXNumberResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn order_toll_free_response_deserializes() {
    assert!(
        serde_json::from_value::<OrderTollFreeResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn order_vanity_response_deserializes() {
    assert!(serde_json::from_value::<OrderVanityResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn remove_did_vpri_response_deserializes() {
    assert!(
        serde_json::from_value::<RemoveDIDvPRIResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_dids_can_response_did_deserializes() {
    assert!(
        serde_json::from_value::<SearchDIDsCANResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_dids_can_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchDIDsCANResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_dids_usa_response_did_deserializes() {
    assert!(
        serde_json::from_value::<SearchDIDsUSAResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_dids_usa_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchDIDsUSAResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_fax_area_code_can_response_ratecenter_deserializes() {
    assert!(
        serde_json::from_value::<SearchFAXAreaCodeCANResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn search_fax_area_code_can_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchFAXAreaCodeCANResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn search_fax_area_code_usa_response_ratecenter_deserializes() {
    assert!(
        serde_json::from_value::<SearchFAXAreaCodeUSAResponseRatecenter>(
            json!({ "status": "success" })
        )
        .is_ok()
    );
}

#[test]
fn search_fax_area_code_usa_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchFAXAreaCodeUSAResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn search_toll_free_can_us_response_did_deserializes() {
    assert!(
        serde_json::from_value::<SearchTollFreeCANUSResponseDID>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn search_toll_free_can_us_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchTollFreeCANUSResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn search_toll_free_usa_response_did_deserializes() {
    assert!(
        serde_json::from_value::<SearchTollFreeUSAResponseDID>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn search_toll_free_usa_response_deserializes() {
    assert!(
        serde_json::from_value::<SearchTollFreeUSAResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_vanity_response_did_deserializes() {
    assert!(
        serde_json::from_value::<SearchVanityResponseDID>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn search_vanity_response_deserializes() {
    assert!(serde_json::from_value::<SearchVanityResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn send_call_recording_email_response_deserializes() {
    assert!(
        serde_json::from_value::<SendCallRecordingEmailResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn send_fax_message_response_deserializes() {
    let r: SendFAXMessageResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SendFAXMessageResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn send_mms_response_deserializes() {
    let r: SendMMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendMMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn send_sms_response_deserializes() {
    let r: SendSMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendSMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn send_voicemail_email_response_deserializes() {
    let r: SendVoicemailEmailResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SendVoicemailEmailResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn set_call_hunting_response_deserializes() {
    let r: SetCallHuntingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetCallHuntingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_call_parking_response_deserializes() {
    let r: SetCallParkingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetCallParkingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_callback_response_deserializes() {
    let r: SetCallbackResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetCallbackResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_caller_id_filtering_response_deserializes() {
    let r: SetCallerIDFilteringResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetCallerIDFilteringResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn set_client_response_deserializes() {
    let r: SetClientResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetClientResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_client_threshold_response_deserializes() {
    let r: SetClientThresholdResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetClientThresholdResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn set_conference_response_deserializes() {
    let r: SetConferenceResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetConferenceResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_conference_member_response_deserializes() {
    let r: SetConferenceMemberResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetConferenceMemberResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn set_did_billing_type_response_deserializes() {
    let r: SetDIDBillingTypeResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetDIDBillingTypeResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_did_info_response_deserializes() {
    let r: SetDIDInfoResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDInfoResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_did_pop_response_deserializes() {
    let r: SetDIDPOPResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDPOPResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_did_routing_response_deserializes() {
    let r: SetDIDRoutingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetDIDRoutingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_did_voicemail_response_deserializes() {
    let r: SetDIDVoicemailResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetDIDVoicemailResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_disa_response_deserializes() {
    let r: SetDISAResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDISAResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_email_to_fax_response_deserializes() {
    let r: SetEmailToFAXResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetEmailToFAXResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_fax_folder_response_deserializes() {
    let r: SetFAXFolderResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetFAXFolderResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_fax_number_email_response_deserializes() {
    let r: SetFAXNumberEmailResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetFAXNumberEmailResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_fax_number_info_response_deserializes() {
    let r: SetFAXNumberInfoResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetFAXNumberInfoResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_fax_number_url_callback_response_deserializes() {
    let r: SetFAXNumberURLCallbackResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetFAXNumberURLCallbackResponse>(json!({ "status": "success" }))
            .is_ok()
    );
}

#[test]
fn set_forwarding_response_deserializes() {
    let r: SetForwardingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetForwardingResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_ivr_response_deserializes() {
    let r: SetIVRResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetIVRResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_location_response_deserializes() {
    let r: SetLocationResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetLocationResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_music_on_hold_response_deserializes() {
    let r: SetMusicOnHoldResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetMusicOnHoldResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_phonebook_response_deserializes() {
    let r: SetPhonebookResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetPhonebookResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_phonebook_group_response_deserializes() {
    let r: SetPhonebookGroupResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetPhonebookGroupResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_queue_response_deserializes() {
    let r: SetQueueResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetQueueResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_recording_response_deserializes() {
    let r: SetRecordingResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetRecordingResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_ring_group_response_deserializes() {
    let r: SetRingGroupResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetRingGroupResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_sip_uri_response_deserializes() {
    let r: SetSIPURIResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetSIPURIResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_sms_response_deserializes() {
    let r: SetSMSResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetSMSResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn set_static_member_response_deserializes() {
    let r: SetStaticMemberResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetStaticMemberResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_sub_account_response_deserializes() {
    let r: SetSubAccountResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetSubAccountResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_time_condition_response_deserializes() {
    let r: SetTimeConditionResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<SetTimeConditionResponse>(json!({ "status": "success" })).is_ok()
    );
}

#[test]
fn set_voicemail_response_deserializes() {
    let r: SetVoicemailResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetVoicemailResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn signup_client_response_deserializes() {
    let r: SignupClientResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SignupClientResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn unconnect_did_response_deserializes() {
    let r: UnconnectDIDResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<UnconnectDIDResponse>(json!({ "status": "success" })).is_ok());
}

#[test]
fn unconnect_fax_response_deserializes() {
    let r: UnconnectFAXResponse = serde_json::from_value(json!({ "status": "success" })).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<UnconnectFAXResponse>(json!({ "status": "success" })).is_ok());
}

/// `status` is the envelope's own, typed rather than left a raw string: by
/// the time a typed call returns, the crate has already decided whether it is
/// `success` or an empty-collection code, and the caller should not have to
/// parse it again.
#[test]
fn the_envelope_status_is_typed() {
    let ok: GetSMSResponse =
        serde_json::from_value(json!({ "status": "success", "sms": [] })).unwrap();
    assert_eq!(ok.status, ApiStatus::Success);

    let empty: GetSMSResponse = serde_json::from_value(json!({ "status": "no_sms" })).unwrap();
    assert_eq!(empty.status, ApiStatus::NoSMS);
    assert!(empty.status.is_empty_collection());
    assert!(empty.sms.is_empty());
}

/// A nested `status` is a record's own -- a fax's, a port's -- and unrelated
/// to the envelope's, so it keeps the inferred string type.
#[test]
fn a_nested_status_is_not_the_envelopes() {
    let resp: GetLNPListResponse = serde_json::from_value(json!({
        "status": "success",
        "list": [{ "portid": "0000", "status": "completed" }],
    }))
    .unwrap();
    assert_eq!(resp.status, ApiStatus::Success);
    assert_eq!(resp.list[0].status.as_deref(), Some("completed"));
}

/// A whole response compares, so a test can assert one outright and a consumer
/// can dedupe or diff records without writing them out field by field.
#[test]
fn a_whole_response_compares() {
    let envelope = json!({
        "status": "success",
        "cdr": [{ "date": "2026-09-16 15:14:35-04:00", "seconds": "11" }],
    });
    let a: GetCDRResponse = serde_json::from_value(envelope.clone()).unwrap();
    let b: GetCDRResponse = serde_json::from_value(envelope).unwrap();
    assert_eq!(a, b);

    let other: GetCDRResponse = serde_json::from_value(json!({
        "status": "success",
        "cdr": [{ "date": "2026-09-16 15:14:35-04:00", "seconds": "12" }],
    }))
    .unwrap();
    assert_ne!(a, other);
}
