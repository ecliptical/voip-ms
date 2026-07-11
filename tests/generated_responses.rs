//! Codegen-drift oracle: every generated `*Response` struct deserializes.
//!
//! Hand-maintained on purpose. Each of the generated response structs must
//! decode from an empty JSON object -- every field is `Option`/`Vec`/`Map`
//! with `#[serde(default)]`, so a bare `{}` is always valid. A regen that
//! makes a field non-optional, drops a `default`, or renames a struct breaks
//! this suite; that failure is the intended signal. Update by hand when the
//! generated surface changes (see DEVELOPMENT.md).

use voip_ms::serde_json::{self, json};
use voip_ms::*;

#[test]
fn add_charge_response_deserializes() {
    assert!(serde_json::from_value::<AddChargeResponse>(json!({})).is_ok());
}

#[test]
fn add_lnpfile_response_deserializes() {
    assert!(serde_json::from_value::<AddLNPFileResponse>(json!({})).is_ok());
}

#[test]
fn add_lnpport_response_deserializes() {
    assert!(serde_json::from_value::<AddLNPPortResponse>(json!({})).is_ok());
}

#[test]
fn add_member_to_conference_response_deserializes() {
    assert!(serde_json::from_value::<AddMemberToConferenceResponse>(json!({})).is_ok());
}

#[test]
fn add_payment_response_deserializes() {
    assert!(serde_json::from_value::<AddPaymentResponse>(json!({})).is_ok());
}

#[test]
fn assign_didv_priresponse_deserializes() {
    assert!(serde_json::from_value::<AssignDIDvPRIResponse>(json!({})).is_ok());
}

#[test]
fn back_order_didcanresponse_deserializes() {
    assert!(serde_json::from_value::<BackOrderDIDCANResponse>(json!({})).is_ok());
}

#[test]
fn back_order_didusaresponse_deserializes() {
    assert!(serde_json::from_value::<BackOrderDIDUSAResponse>(json!({})).is_ok());
}

#[test]
fn cancel_didresponse_deserializes() {
    assert!(serde_json::from_value::<CancelDIDResponse>(json!({})).is_ok());
}

#[test]
fn cancel_faxnumber_response_deserializes() {
    assert!(serde_json::from_value::<CancelFAXNumberResponse>(json!({})).is_ok());
}

#[test]
fn connect_didresponse_deserializes() {
    assert!(serde_json::from_value::<ConnectDIDResponse>(json!({})).is_ok());
}

#[test]
fn connect_faxresponse_deserializes() {
    assert!(serde_json::from_value::<ConnectFAXResponse>(json!({})).is_ok());
}

#[test]
fn create_sub_account_response_deserializes() {
    assert!(serde_json::from_value::<CreateSubAccountResponse>(json!({})).is_ok());
}

#[test]
fn create_voicemail_response_deserializes() {
    assert!(serde_json::from_value::<CreateVoicemailResponse>(json!({})).is_ok());
}

#[test]
fn del_call_hunting_response_deserializes() {
    assert!(serde_json::from_value::<DelCallHuntingResponse>(json!({})).is_ok());
}

#[test]
fn del_call_parking_response_deserializes() {
    assert!(serde_json::from_value::<DelCallParkingResponse>(json!({})).is_ok());
}

#[test]
fn del_call_recording_response_deserializes() {
    assert!(serde_json::from_value::<DelCallRecordingResponse>(json!({})).is_ok());
}

#[test]
fn del_callback_response_deserializes() {
    assert!(serde_json::from_value::<DelCallbackResponse>(json!({})).is_ok());
}

#[test]
fn del_caller_idfiltering_response_deserializes() {
    assert!(serde_json::from_value::<DelCallerIDFilteringResponse>(json!({})).is_ok());
}

#[test]
fn del_client_response_deserializes() {
    assert!(serde_json::from_value::<DelClientResponse>(json!({})).is_ok());
}

#[test]
fn del_conference_response_deserializes() {
    assert!(serde_json::from_value::<DelConferenceResponse>(json!({})).is_ok());
}

#[test]
fn del_conference_member_response_deserializes() {
    assert!(serde_json::from_value::<DelConferenceMemberResponse>(json!({})).is_ok());
}

#[test]
fn del_disaresponse_deserializes() {
    assert!(serde_json::from_value::<DelDISAResponse>(json!({})).is_ok());
}

#[test]
fn del_email_to_faxresponse_deserializes() {
    assert!(serde_json::from_value::<DelEmailToFAXResponse>(json!({})).is_ok());
}

#[test]
fn del_faxfolder_response_deserializes() {
    assert!(serde_json::from_value::<DelFAXFolderResponse>(json!({})).is_ok());
}

#[test]
fn del_forwarding_response_deserializes() {
    assert!(serde_json::from_value::<DelForwardingResponse>(json!({})).is_ok());
}

#[test]
fn del_ivrresponse_deserializes() {
    assert!(serde_json::from_value::<DelIVRResponse>(json!({})).is_ok());
}

#[test]
fn del_location_response_deserializes() {
    assert!(serde_json::from_value::<DelLocationResponse>(json!({})).is_ok());
}

#[test]
fn del_member_from_conference_response_deserializes() {
    assert!(serde_json::from_value::<DelMemberFromConferenceResponse>(json!({})).is_ok());
}

#[test]
fn del_messages_response_deserializes() {
    assert!(serde_json::from_value::<DelMessagesResponse>(json!({})).is_ok());
}

#[test]
fn del_music_on_hold_response_deserializes() {
    assert!(serde_json::from_value::<DelMusicOnHoldResponse>(json!({})).is_ok());
}

#[test]
fn del_phonebook_response_deserializes() {
    assert!(serde_json::from_value::<DelPhonebookResponse>(json!({})).is_ok());
}

#[test]
fn del_phonebook_group_response_deserializes() {
    assert!(serde_json::from_value::<DelPhonebookGroupResponse>(json!({})).is_ok());
}

#[test]
fn del_queue_response_deserializes() {
    assert!(serde_json::from_value::<DelQueueResponse>(json!({})).is_ok());
}

#[test]
fn del_recording_response_deserializes() {
    assert!(serde_json::from_value::<DelRecordingResponse>(json!({})).is_ok());
}

#[test]
fn del_ring_group_response_deserializes() {
    assert!(serde_json::from_value::<DelRingGroupResponse>(json!({})).is_ok());
}

#[test]
fn del_sipuriresponse_deserializes() {
    assert!(serde_json::from_value::<DelSIPURIResponse>(json!({})).is_ok());
}

#[test]
fn del_static_member_response_deserializes() {
    assert!(serde_json::from_value::<DelStaticMemberResponse>(json!({})).is_ok());
}

#[test]
fn del_sub_account_response_deserializes() {
    assert!(serde_json::from_value::<DelSubAccountResponse>(json!({})).is_ok());
}

#[test]
fn del_time_condition_response_deserializes() {
    assert!(serde_json::from_value::<DelTimeConditionResponse>(json!({})).is_ok());
}

#[test]
fn del_voicemail_response_deserializes() {
    assert!(serde_json::from_value::<DelVoicemailResponse>(json!({})).is_ok());
}

#[test]
fn delete_faxmessage_response_deserializes() {
    assert!(serde_json::from_value::<DeleteFAXMessageResponse>(json!({})).is_ok());
}

#[test]
fn delete_mms_response_deserializes() {
    let r: DeleteMMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<DeleteMMSResponse>(json!({})).is_ok());
}

#[test]
fn delete_sms_response_deserializes() {
    let r: DeleteSMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<DeleteSMSResponse>(json!({})).is_ok());
}

#[test]
fn e911_address_types_response_type_deserializes() {
    let r: E911AddressTypesResponseType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911AddressTypesResponseType>(json!({})).is_ok());
}

#[test]
fn e911_address_types_response_deserializes() {
    let r: E911AddressTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911AddressTypesResponse>(json!({})).is_ok());
}

#[test]
fn e911_cancel_response_deserializes() {
    let r: E911CancelResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911CancelResponse>(json!({})).is_ok());
}

#[test]
fn e911_info_response_info_deserializes() {
    let r: E911InfoResponseInfo = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911InfoResponseInfo>(json!({})).is_ok());
}

#[test]
fn e911_info_response_deserializes() {
    let r: E911InfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911InfoResponse>(json!({})).is_ok());
}

#[test]
fn e911_provision_response_deserializes() {
    let r: E911ProvisionResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911ProvisionResponse>(json!({})).is_ok());
}

#[test]
fn e911_provision_manually_response_deserializes() {
    let r: E911ProvisionManuallyResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911ProvisionManuallyResponse>(json!({})).is_ok());
}

#[test]
fn e911_update_response_deserializes() {
    let r: E911UpdateResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911UpdateResponse>(json!({})).is_ok());
}

#[test]
fn e911_validate_response_deserializes() {
    let r: E911ValidateResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<E911ValidateResponse>(json!({})).is_ok());
}

#[test]
fn get_allowed_codecs_response_allowed_codec_deserializes() {
    let r: GetAllowedCodecsResponseAllowedCodec = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetAllowedCodecsResponseAllowedCodec>(json!({})).is_ok());
}

#[test]
fn get_allowed_codecs_response_deserializes() {
    let r: GetAllowedCodecsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetAllowedCodecsResponse>(json!({})).is_ok());
}

#[test]
fn get_auth_types_response_auth_type_deserializes() {
    let r: GetAuthTypesResponseAuthType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetAuthTypesResponseAuthType>(json!({})).is_ok());
}

#[test]
fn get_auth_types_response_deserializes() {
    let r: GetAuthTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetAuthTypesResponse>(json!({})).is_ok());
}

#[test]
fn get_back_orders_response_back_order_deserializes() {
    let r: GetBackOrdersResponseBackOrder = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBackOrdersResponseBackOrder>(json!({})).is_ok());
}

#[test]
fn get_back_orders_response_deserializes() {
    let r: GetBackOrdersResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBackOrdersResponse>(json!({})).is_ok());
}

#[test]
fn get_balance_response_balance_deserializes() {
    let r: GetBalanceResponseBalance = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBalanceResponseBalance>(json!({})).is_ok());
}

#[test]
fn get_balance_response_deserializes() {
    let r: GetBalanceResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBalanceResponse>(json!({})).is_ok());
}

#[test]
fn get_balance_management_response_balance_management_deserializes() {
    let r: GetBalanceManagementResponseBalanceManagement =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetBalanceManagementResponseBalanceManagement>(json!({})).is_ok()
    );
}

#[test]
fn get_balance_management_response_deserializes() {
    let r: GetBalanceManagementResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetBalanceManagementResponse>(json!({})).is_ok());
}

#[test]
fn get_cdr_response_cdr_deserializes() {
    let r: GetCDRResponseCDR = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCDRResponseCDR>(json!({})).is_ok());
}

#[test]
fn get_cdr_response_deserializes() {
    let r: GetCDRResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCDRResponse>(json!({})).is_ok());
}

#[test]
fn get_call_accounts_response_account_deserializes() {
    let r: GetCallAccountsResponseAccount = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallAccountsResponseAccount>(json!({})).is_ok());
}

#[test]
fn get_call_accounts_response_deserializes() {
    let r: GetCallAccountsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallAccountsResponse>(json!({})).is_ok());
}

#[test]
fn get_call_billing_response_call_billing_deserializes() {
    let r: GetCallBillingResponseCallBilling = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallBillingResponseCallBilling>(json!({})).is_ok());
}

#[test]
fn get_call_billing_response_deserializes() {
    let r: GetCallBillingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallBillingResponse>(json!({})).is_ok());
}

#[test]
fn get_call_huntings_response_call_hunting_deserializes() {
    let r: GetCallHuntingsResponseCallHunting = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallHuntingsResponseCallHunting>(json!({})).is_ok());
}

#[test]
fn get_call_huntings_response_deserializes() {
    let r: GetCallHuntingsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallHuntingsResponse>(json!({})).is_ok());
}

#[test]
fn get_call_parking_response_call_hunting_deserializes() {
    let r: GetCallParkingResponseCallHunting = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallParkingResponseCallHunting>(json!({})).is_ok());
}

#[test]
fn get_call_parking_response_deserializes() {
    let r: GetCallParkingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallParkingResponse>(json!({})).is_ok());
}

#[test]
fn get_call_recording_response_deserializes() {
    let r: GetCallRecordingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallRecordingResponse>(json!({})).is_ok());
}

#[test]
fn get_call_recordings_response_recording_deserializes() {
    let r: GetCallRecordingsResponseRecording = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallRecordingsResponseRecording>(json!({})).is_ok());
}

#[test]
fn get_call_recordings_response_deserializes() {
    let r: GetCallRecordingsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallRecordingsResponse>(json!({})).is_ok());
}

#[test]
fn get_call_transcriptions_response_transcription_recognized_phrase_deserializes() {
    let r: GetCallTranscriptionsResponseTranscriptionRecognizedPhrase =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTranscriptionsResponseTranscriptionRecognizedPhrase>(
            json!({})
        )
        .is_ok()
    );
}

#[test]
fn get_call_transcriptions_response_transcription_deserializes() {
    let r: GetCallTranscriptionsResponseTranscription = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetCallTranscriptionsResponseTranscription>(json!({})).is_ok()
    );
}

#[test]
fn get_call_transcriptions_response_deserializes() {
    let r: GetCallTranscriptionsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallTranscriptionsResponse>(json!({})).is_ok());
}

#[test]
fn get_call_types_response_call_type_deserializes() {
    let r: GetCallTypesResponseCallType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallTypesResponseCallType>(json!({})).is_ok());
}

#[test]
fn get_call_types_response_deserializes() {
    let r: GetCallTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallTypesResponse>(json!({})).is_ok());
}

#[test]
fn get_callbacks_response_callback_deserializes() {
    let r: GetCallbacksResponseCallback = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallbacksResponseCallback>(json!({})).is_ok());
}

#[test]
fn get_callbacks_response_deserializes() {
    let r: GetCallbacksResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallbacksResponse>(json!({})).is_ok());
}

#[test]
fn get_caller_id_filtering_response_filtering_deserializes() {
    let r: GetCallerIDFilteringResponseFiltering = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallerIDFilteringResponseFiltering>(json!({})).is_ok());
}

#[test]
fn get_caller_id_filtering_response_deserializes() {
    let r: GetCallerIDFilteringResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCallerIDFilteringResponse>(json!({})).is_ok());
}

#[test]
fn get_carriers_response_carrier_deserializes() {
    let r: GetCarriersResponseCarrier = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCarriersResponseCarrier>(json!({})).is_ok());
}

#[test]
fn get_carriers_response_deserializes() {
    let r: GetCarriersResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCarriersResponse>(json!({})).is_ok());
}

#[test]
fn get_charges_response_charge_deserializes() {
    let r: GetChargesResponseCharge = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetChargesResponseCharge>(json!({})).is_ok());
}

#[test]
fn get_charges_response_deserializes() {
    let r: GetChargesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetChargesResponse>(json!({})).is_ok());
}

#[test]
fn get_client_packages_response_package_deserializes() {
    let r: GetClientPackagesResponsePackage = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientPackagesResponsePackage>(json!({})).is_ok());
}

#[test]
fn get_client_packages_response_deserializes() {
    let r: GetClientPackagesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientPackagesResponse>(json!({})).is_ok());
}

#[test]
fn get_client_threshold_response_threshold_information_deserializes() {
    let r: GetClientThresholdResponseThresholdInformation =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetClientThresholdResponseThresholdInformation>(json!({})).is_ok()
    );
}

#[test]
fn get_client_threshold_response_deserializes() {
    let r: GetClientThresholdResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientThresholdResponse>(json!({})).is_ok());
}

#[test]
fn get_clients_response_client_deserializes() {
    let r: GetClientsResponseClient = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientsResponseClient>(json!({})).is_ok());
}

#[test]
fn get_clients_response_deserializes() {
    let r: GetClientsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetClientsResponse>(json!({})).is_ok());
}

#[test]
fn get_conference_response_conference_deserializes() {
    let r: GetConferenceResponseConference = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceResponseConference>(json!({})).is_ok());
}

#[test]
fn get_conference_response_deserializes() {
    let r: GetConferenceResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceResponse>(json!({})).is_ok());
}

#[test]
fn get_conference_members_response_member_deserializes() {
    let r: GetConferenceMembersResponseMember = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceMembersResponseMember>(json!({})).is_ok());
}

#[test]
fn get_conference_members_response_deserializes() {
    let r: GetConferenceMembersResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceMembersResponse>(json!({})).is_ok());
}

#[test]
fn get_conference_recording_file_response_recording_deserializes() {
    let r: GetConferenceRecordingFileResponseRecording = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetConferenceRecordingFileResponseRecording>(json!({})).is_ok()
    );
}

#[test]
fn get_conference_recording_file_response_deserializes() {
    let r: GetConferenceRecordingFileResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceRecordingFileResponse>(json!({})).is_ok());
}

#[test]
fn get_conference_recordings_response_recording_deserializes() {
    let r: GetConferenceRecordingsResponseRecording = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceRecordingsResponseRecording>(json!({})).is_ok());
}

#[test]
fn get_conference_recordings_response_deserializes() {
    let r: GetConferenceRecordingsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetConferenceRecordingsResponse>(json!({})).is_ok());
}

#[test]
fn get_countries_response_country_deserializes() {
    let r: GetCountriesResponseCountry = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCountriesResponseCountry>(json!({})).is_ok());
}

#[test]
fn get_countries_response_deserializes() {
    let r: GetCountriesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetCountriesResponse>(json!({})).is_ok());
}

#[test]
fn get_did_countries_response_country_deserializes() {
    let r: GetDIDCountriesResponseCountry = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDCountriesResponseCountry>(json!({})).is_ok());
}

#[test]
fn get_did_countries_response_deserializes() {
    let r: GetDIDCountriesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDCountriesResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_can_response_did_deserializes() {
    let r: GetDIDsCANResponseDID = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsCANResponseDID>(json!({})).is_ok());
}

#[test]
fn get_dids_can_response_deserializes() {
    let r: GetDIDsCANResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsCANResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_info_response_did_deserializes() {
    let r: GetDIDsInfoResponseDID = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInfoResponseDID>(json!({})).is_ok());
}

#[test]
fn get_dids_info_response_deserializes() {
    let r: GetDIDsInfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInfoResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_international_geographic_response_location_deserializes() {
    let r: GetDIDsInternationalGeographicResponseLocation =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalGeographicResponseLocation>(json!({})).is_ok()
    );
}

#[test]
fn get_dids_international_geographic_response_deserializes() {
    let r: GetDIDsInternationalGeographicResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInternationalGeographicResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_international_national_response_location_deserializes() {
    let r: GetDIDsInternationalNationalResponseLocation =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalNationalResponseLocation>(json!({})).is_ok()
    );
}

#[test]
fn get_dids_international_national_response_deserializes() {
    let r: GetDIDsInternationalNationalResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInternationalNationalResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_international_toll_free_response_location_deserializes() {
    let r: GetDIDsInternationalTollFreeResponseLocation =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetDIDsInternationalTollFreeResponseLocation>(json!({})).is_ok()
    );
}

#[test]
fn get_dids_international_toll_free_response_deserializes() {
    let r: GetDIDsInternationalTollFreeResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsInternationalTollFreeResponse>(json!({})).is_ok());
}

#[test]
fn get_dids_usa_response_did_deserializes() {
    let r: GetDIDsUSAResponseDID = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsUSAResponseDID>(json!({})).is_ok());
}

#[test]
fn get_dids_usa_response_deserializes() {
    let r: GetDIDsUSAResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDsUSAResponse>(json!({})).is_ok());
}

#[test]
fn get_did_vpri_response_deserializes() {
    let r: GetDIDvPRIResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDIDvPRIResponse>(json!({})).is_ok());
}

#[test]
fn get_disas_response_disa_deserializes() {
    let r: GetDISAsResponseDISA = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDISAsResponseDISA>(json!({})).is_ok());
}

#[test]
fn get_disas_response_deserializes() {
    let r: GetDISAsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDISAsResponse>(json!({})).is_ok());
}

#[test]
fn get_dtmf_modes_response_dtmf_mode_deserializes() {
    let r: GetDTMFModesResponseDTMFMode = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDTMFModesResponseDTMFMode>(json!({})).is_ok());
}

#[test]
fn get_dtmf_modes_response_deserializes() {
    let r: GetDTMFModesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDTMFModesResponse>(json!({})).is_ok());
}

#[test]
fn get_deposits_response_deposit_deserializes() {
    let r: GetDepositsResponseDeposit = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDepositsResponseDeposit>(json!({})).is_ok());
}

#[test]
fn get_deposits_response_deserializes() {
    let r: GetDepositsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDepositsResponse>(json!({})).is_ok());
}

#[test]
fn get_device_types_response_device_type_deserializes() {
    let r: GetDeviceTypesResponseDeviceType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDeviceTypesResponseDeviceType>(json!({})).is_ok());
}

#[test]
fn get_device_types_response_deserializes() {
    let r: GetDeviceTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetDeviceTypesResponse>(json!({})).is_ok());
}

#[test]
fn get_email_to_fax_response_email_to_fax_deserializes() {
    let r: GetEmailToFAXResponseEmailToFAX = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetEmailToFAXResponseEmailToFAX>(json!({})).is_ok());
}

#[test]
fn get_email_to_fax_response_deserializes() {
    let r: GetEmailToFAXResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetEmailToFAXResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_folders_response_folder_deserializes() {
    let r: GetFAXFoldersResponseFolder = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXFoldersResponseFolder>(json!({})).is_ok());
}

#[test]
fn get_fax_folders_response_deserializes() {
    let r: GetFAXFoldersResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXFoldersResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_message_pdf_response_deserializes() {
    let r: GetFAXMessagePDFResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXMessagePDFResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_messages_response_fax_deserializes() {
    let r: GetFAXMessagesResponseFAX = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXMessagesResponseFAX>(json!({})).is_ok());
}

#[test]
fn get_fax_messages_response_deserializes() {
    let r: GetFAXMessagesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXMessagesResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_numbers_info_response_number_deserializes() {
    let r: GetFAXNumbersInfoResponseNumber = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXNumbersInfoResponseNumber>(json!({})).is_ok());
}

#[test]
fn get_fax_numbers_info_response_deserializes() {
    let r: GetFAXNumbersInfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXNumbersInfoResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_numbers_portability_response_deserializes() {
    let r: GetFAXNumbersPortabilityResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXNumbersPortabilityResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_provinces_response_province_deserializes() {
    let r: GetFAXProvincesResponseProvince = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXProvincesResponseProvince>(json!({})).is_ok());
}

#[test]
fn get_fax_provinces_response_deserializes() {
    let r: GetFAXProvincesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXProvincesResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_rate_centers_can_response_ratecenter_deserializes() {
    let r: GetFAXRateCentersCANResponseRatecenter = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXRateCentersCANResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn get_fax_rate_centers_can_response_deserializes() {
    let r: GetFAXRateCentersCANResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXRateCentersCANResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_rate_centers_usa_response_ratecenter_deserializes() {
    let r: GetFAXRateCentersUSAResponseRatecenter = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXRateCentersUSAResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn get_fax_rate_centers_usa_response_deserializes() {
    let r: GetFAXRateCentersUSAResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXRateCentersUSAResponse>(json!({})).is_ok());
}

#[test]
fn get_fax_states_response_state_deserializes() {
    let r: GetFAXStatesResponseState = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXStatesResponseState>(json!({})).is_ok());
}

#[test]
fn get_fax_states_response_deserializes() {
    let r: GetFAXStatesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetFAXStatesResponse>(json!({})).is_ok());
}

#[test]
fn get_forwardings_response_forwarding_deserializes() {
    let r: GetForwardingsResponseForwarding = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetForwardingsResponseForwarding>(json!({})).is_ok());
}

#[test]
fn get_forwardings_response_deserializes() {
    let r: GetForwardingsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetForwardingsResponse>(json!({})).is_ok());
}

#[test]
fn get_ip_response_deserializes() {
    let r: GetIPResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIPResponse>(json!({})).is_ok());
}

#[test]
fn get_ivrs_response_ivr_deserializes() {
    let r: GetIVRsResponseIVR = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIVRsResponseIVR>(json!({})).is_ok());
}

#[test]
fn get_ivrs_response_deserializes() {
    let r: GetIVRsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetIVRsResponse>(json!({})).is_ok());
}

#[test]
fn get_international_types_response_type_deserializes() {
    let r: GetInternationalTypesResponseType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetInternationalTypesResponseType>(json!({})).is_ok());
}

#[test]
fn get_international_types_response_deserializes() {
    let r: GetInternationalTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetInternationalTypesResponse>(json!({})).is_ok());
}

#[test]
fn get_join_when_empty_types_response_type_deserializes() {
    let r: GetJoinWhenEmptyTypesResponseType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetJoinWhenEmptyTypesResponseType>(json!({})).is_ok());
}

#[test]
fn get_join_when_empty_types_response_deserializes() {
    let r: GetJoinWhenEmptyTypesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetJoinWhenEmptyTypesResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_attach_response_deserializes() {
    let r: GetLNPAttachResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPAttachResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_attach_list_response_list_deserializes() {
    let r: GetLNPAttachListResponseList = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPAttachListResponseList>(json!({})).is_ok());
}

#[test]
fn get_lnp_attach_list_response_deserializes() {
    let r: GetLNPAttachListResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPAttachListResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_details_response_number_deserializes() {
    let r: GetLNPDetailsResponseNumber = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPDetailsResponseNumber>(json!({})).is_ok());
}

#[test]
fn get_lnp_details_response_note_deserializes() {
    let r: GetLNPDetailsResponseNote = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPDetailsResponseNote>(json!({})).is_ok());
}

#[test]
fn get_lnp_details_response_attachment_deserializes() {
    let r: GetLNPDetailsResponseAttachment = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPDetailsResponseAttachment>(json!({})).is_ok());
}

#[test]
fn get_lnp_details_response_deserializes() {
    let r: GetLNPDetailsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPDetailsResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_list_response_list_deserializes() {
    let r: GetLNPListResponseList = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPListResponseList>(json!({})).is_ok());
}

#[test]
fn get_lnp_list_response_deserializes() {
    let r: GetLNPListResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPListResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_list_status_response_deserializes() {
    let r: GetLNPListStatusResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPListStatusResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_notes_response_list_deserializes() {
    let r: GetLNPNotesResponseList = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPNotesResponseList>(json!({})).is_ok());
}

#[test]
fn get_lnp_notes_response_deserializes() {
    let r: GetLNPNotesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPNotesResponse>(json!({})).is_ok());
}

#[test]
fn get_lnp_status_response_deserializes() {
    let r: GetLNPStatusResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetLNPStatusResponse>(json!({})).is_ok());
}

#[test]
fn get_languages_response_language_deserializes() {
    assert!(serde_json::from_value::<GetLanguagesResponseLanguage>(json!({})).is_ok());
}

#[test]
fn get_languages_response_deserializes() {
    assert!(serde_json::from_value::<GetLanguagesResponse>(json!({})).is_ok());
}

#[test]
fn get_locales_response_locale_deserializes() {
    assert!(serde_json::from_value::<GetLocalesResponseLocale>(json!({})).is_ok());
}

#[test]
fn get_locales_response_deserializes() {
    assert!(serde_json::from_value::<GetLocalesResponse>(json!({})).is_ok());
}

#[test]
fn get_locations_response_deserializes() {
    assert!(serde_json::from_value::<GetLocationsResponse>(json!({})).is_ok());
}

#[test]
fn get_lock_international_response_lock_international_deserializes() {
    assert!(
        serde_json::from_value::<GetLockInternationalResponseLockInternational>(json!({})).is_ok()
    );
}

#[test]
fn get_lock_international_response_deserializes() {
    assert!(serde_json::from_value::<GetLockInternationalResponse>(json!({})).is_ok());
}

#[test]
fn get_mms_response_sms_deserializes() {
    assert!(serde_json::from_value::<GetMMSResponseSMS>(json!({})).is_ok());
}

#[test]
fn get_mms_response_deserializes() {
    assert!(serde_json::from_value::<GetMMSResponse>(json!({})).is_ok());
}

#[test]
fn get_media_mms_response_deserializes() {
    assert!(serde_json::from_value::<GetMediaMMSResponse>(json!({})).is_ok());
}

#[test]
fn get_music_on_hold_response_music_on_hold_deserializes() {
    assert!(serde_json::from_value::<GetMusicOnHoldResponseMusicOnHold>(json!({})).is_ok());
}

#[test]
fn get_music_on_hold_response_deserializes() {
    assert!(serde_json::from_value::<GetMusicOnHoldResponse>(json!({})).is_ok());
}

#[test]
fn get_nat_response_nat_deserializes() {
    assert!(serde_json::from_value::<GetNATResponseNAT>(json!({})).is_ok());
}

#[test]
fn get_nat_response_deserializes() {
    assert!(serde_json::from_value::<GetNATResponse>(json!({})).is_ok());
}

#[test]
fn get_packages_response_package_deserializes() {
    assert!(serde_json::from_value::<GetPackagesResponsePackage>(json!({})).is_ok());
}

#[test]
fn get_packages_response_deserializes() {
    assert!(serde_json::from_value::<GetPackagesResponse>(json!({})).is_ok());
}

#[test]
fn get_phonebook_response_phonebook_deserializes() {
    assert!(serde_json::from_value::<GetPhonebookResponsePhonebook>(json!({})).is_ok());
}

#[test]
fn get_phonebook_response_deserializes() {
    assert!(serde_json::from_value::<GetPhonebookResponse>(json!({})).is_ok());
}

#[test]
fn get_phonebook_groups_response_phonebook_deserializes() {
    assert!(serde_json::from_value::<GetPhonebookGroupsResponsePhonebook>(json!({})).is_ok());
}

#[test]
fn get_phonebook_groups_response_deserializes() {
    assert!(serde_json::from_value::<GetPhonebookGroupsResponse>(json!({})).is_ok());
}

#[test]
fn get_play_instructions_response_play_instruction_deserializes() {
    assert!(
        serde_json::from_value::<GetPlayInstructionsResponsePlayInstruction>(json!({})).is_ok()
    );
}

#[test]
fn get_play_instructions_response_deserializes() {
    assert!(serde_json::from_value::<GetPlayInstructionsResponse>(json!({})).is_ok());
}

#[test]
fn get_portability_response_plan_deserializes() {
    assert!(serde_json::from_value::<GetPortabilityResponsePlan>(json!({})).is_ok());
}

#[test]
fn get_portability_response_deserializes() {
    assert!(serde_json::from_value::<GetPortabilityResponse>(json!({})).is_ok());
}

#[test]
fn get_protocols_response_protocol_deserializes() {
    assert!(serde_json::from_value::<GetProtocolsResponseProtocol>(json!({})).is_ok());
}

#[test]
fn get_protocols_response_deserializes() {
    assert!(serde_json::from_value::<GetProtocolsResponse>(json!({})).is_ok());
}

#[test]
fn get_provinces_response_province_deserializes() {
    assert!(serde_json::from_value::<GetProvincesResponseProvince>(json!({})).is_ok());
}

#[test]
fn get_provinces_response_deserializes() {
    assert!(serde_json::from_value::<GetProvincesResponse>(json!({})).is_ok());
}

#[test]
fn get_queues_response_queue_deserializes() {
    assert!(serde_json::from_value::<GetQueuesResponseQueue>(json!({})).is_ok());
}

#[test]
fn get_queues_response_deserializes() {
    assert!(serde_json::from_value::<GetQueuesResponse>(json!({})).is_ok());
}

#[test]
fn get_rate_centers_can_response_ratecenter_deserializes() {
    assert!(serde_json::from_value::<GetRateCentersCANResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn get_rate_centers_can_response_deserializes() {
    assert!(serde_json::from_value::<GetRateCentersCANResponse>(json!({})).is_ok());
}

#[test]
fn get_rate_centers_usa_response_ratecenter_deserializes() {
    assert!(serde_json::from_value::<GetRateCentersUSAResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn get_rate_centers_usa_response_deserializes() {
    assert!(serde_json::from_value::<GetRateCentersUSAResponse>(json!({})).is_ok());
}

#[test]
fn get_rates_response_rate_deserializes() {
    assert!(serde_json::from_value::<GetRatesResponseRate>(json!({})).is_ok());
}

#[test]
fn get_rates_response_deserializes() {
    assert!(serde_json::from_value::<GetRatesResponse>(json!({})).is_ok());
}

#[test]
fn get_recording_file_response_recording_deserializes() {
    assert!(serde_json::from_value::<GetRecordingFileResponseRecording>(json!({})).is_ok());
}

#[test]
fn get_recording_file_response_deserializes() {
    assert!(serde_json::from_value::<GetRecordingFileResponse>(json!({})).is_ok());
}

#[test]
fn get_recordings_response_recording_deserializes() {
    assert!(serde_json::from_value::<GetRecordingsResponseRecording>(json!({})).is_ok());
}

#[test]
fn get_recordings_response_deserializes() {
    assert!(serde_json::from_value::<GetRecordingsResponse>(json!({})).is_ok());
}

#[test]
fn get_registration_status_response_registration_deserializes() {
    assert!(serde_json::from_value::<GetRegistrationStatusResponseRegistration>(json!({})).is_ok());
}

#[test]
fn get_registration_status_response_deserializes() {
    assert!(serde_json::from_value::<GetRegistrationStatusResponse>(json!({})).is_ok());
}

#[test]
fn get_report_estimated_hold_time_response_type_deserializes() {
    let r: GetReportEstimatedHoldTimeResponseType = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetReportEstimatedHoldTimeResponseType>(json!({})).is_ok());
}

#[test]
fn get_report_estimated_hold_time_response_deserializes() {
    let r: GetReportEstimatedHoldTimeResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetReportEstimatedHoldTimeResponse>(json!({})).is_ok());
}

#[test]
fn get_reseller_balance_response_balance_deserializes() {
    let r: GetResellerBalanceResponseBalance = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerBalanceResponseBalance>(json!({})).is_ok());
}

#[test]
fn get_reseller_balance_response_deserializes() {
    let r: GetResellerBalanceResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerBalanceResponse>(json!({})).is_ok());
}

#[test]
fn get_reseller_cdr_response_cdr_deserializes() {
    let r: GetResellerCDRResponseCDR = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerCDRResponseCDR>(json!({})).is_ok());
}

#[test]
fn get_reseller_cdr_response_deserializes() {
    let r: GetResellerCDRResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerCDRResponse>(json!({})).is_ok());
}

#[test]
fn get_reseller_mms_response_sms_deserializes() {
    let r: GetResellerMMSResponseSMS = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerMMSResponseSMS>(json!({})).is_ok());
}

#[test]
fn get_reseller_mms_response_deserializes() {
    let r: GetResellerMMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerMMSResponse>(json!({})).is_ok());
}

#[test]
fn get_reseller_sms_response_sms_deserializes() {
    let r: GetResellerSMSResponseSMS = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerSMSResponseSMS>(json!({})).is_ok());
}

#[test]
fn get_reseller_sms_response_deserializes() {
    let r: GetResellerSMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetResellerSMSResponse>(json!({})).is_ok());
}

#[test]
fn get_ring_groups_response_ring_group_deserializes() {
    let r: GetRingGroupsResponseRingGroup = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRingGroupsResponseRingGroup>(json!({})).is_ok());
}

#[test]
fn get_ring_groups_response_deserializes() {
    let r: GetRingGroupsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRingGroupsResponse>(json!({})).is_ok());
}

#[test]
fn get_ring_strategies_response_strategy_deserializes() {
    let r: GetRingStrategiesResponseStrategy = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRingStrategiesResponseStrategy>(json!({})).is_ok());
}

#[test]
fn get_ring_strategies_response_deserializes() {
    let r: GetRingStrategiesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRingStrategiesResponse>(json!({})).is_ok());
}

#[test]
fn get_routes_response_route_deserializes() {
    let r: GetRoutesResponseRoute = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRoutesResponseRoute>(json!({})).is_ok());
}

#[test]
fn get_routes_response_deserializes() {
    let r: GetRoutesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetRoutesResponse>(json!({})).is_ok());
}

#[test]
fn get_sip_uris_response_sip_uri_deserializes() {
    let r: GetSIPURIsResponseSIPURI = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSIPURIsResponseSIPURI>(json!({})).is_ok());
}

#[test]
fn get_sip_uris_response_deserializes() {
    let r: GetSIPURIsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSIPURIsResponse>(json!({})).is_ok());
}

#[test]
fn get_sms_response_sms_deserializes() {
    let r: GetSMSResponseSMS = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSMSResponseSMS>(json!({})).is_ok());
}

#[test]
fn get_sms_response_deserializes() {
    let r: GetSMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSMSResponse>(json!({})).is_ok());
}

#[test]
fn get_servers_info_response_server_deserializes() {
    let r: GetServersInfoResponseServer = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetServersInfoResponseServer>(json!({})).is_ok());
}

#[test]
fn get_servers_info_response_deserializes() {
    let r: GetServersInfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetServersInfoResponse>(json!({})).is_ok());
}

#[test]
fn get_states_response_state_deserializes() {
    let r: GetStatesResponseState = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetStatesResponseState>(json!({})).is_ok());
}

#[test]
fn get_states_response_deserializes() {
    let r: GetStatesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetStatesResponse>(json!({})).is_ok());
}

#[test]
fn get_static_members_response_member_deserializes() {
    let r: GetStaticMembersResponseMember = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetStaticMembersResponseMember>(json!({})).is_ok());
}

#[test]
fn get_static_members_response_deserializes() {
    let r: GetStaticMembersResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetStaticMembersResponse>(json!({})).is_ok());
}

#[test]
fn get_sub_accounts_response_account_deserializes() {
    let r: GetSubAccountsResponseAccount = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSubAccountsResponseAccount>(json!({})).is_ok());
}

#[test]
fn get_sub_accounts_response_deserializes() {
    let r: GetSubAccountsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetSubAccountsResponse>(json!({})).is_ok());
}

#[test]
fn get_termination_rates_response_route_deserializes() {
    let r: GetTerminationRatesResponseRoute = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTerminationRatesResponseRoute>(json!({})).is_ok());
}

#[test]
fn get_termination_rates_response_rate_deserializes() {
    let r: GetTerminationRatesResponseRate = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTerminationRatesResponseRate>(json!({})).is_ok());
}

#[test]
fn get_termination_rates_response_deserializes() {
    let r: GetTerminationRatesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTerminationRatesResponse>(json!({})).is_ok());
}

#[test]
fn get_time_conditions_response_timecondition_deserializes() {
    let r: GetTimeConditionsResponseTimecondition = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTimeConditionsResponseTimecondition>(json!({})).is_ok());
}

#[test]
fn get_time_conditions_response_deserializes() {
    let r: GetTimeConditionsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTimeConditionsResponse>(json!({})).is_ok());
}

#[test]
fn get_timezones_response_timezone_deserializes() {
    let r: GetTimezonesResponseTimezone = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTimezonesResponseTimezone>(json!({})).is_ok());
}

#[test]
fn get_timezones_response_deserializes() {
    let r: GetTimezonesResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTimezonesResponse>(json!({})).is_ok());
}

#[test]
fn get_transaction_history_response_transaction_deserializes() {
    let r: GetTransactionHistoryResponseTransaction = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTransactionHistoryResponseTransaction>(json!({})).is_ok());
}

#[test]
fn get_transaction_history_response_deserializes() {
    let r: GetTransactionHistoryResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetTransactionHistoryResponse>(json!({})).is_ok());
}

#[test]
fn get_vpris_response_vpri_deserializes() {
    let r: GetVPRIsResponseVPRI = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetVPRIsResponseVPRI>(json!({})).is_ok());
}

#[test]
fn get_vpris_response_deserializes() {
    let r: GetVPRIsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetVPRIsResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_attachment_formats_response_email_attachment_format_deserializes() {
    let r: GetVoicemailAttachmentFormatsResponseEmailAttachmentFormat =
        serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(
        serde_json::from_value::<GetVoicemailAttachmentFormatsResponseEmailAttachmentFormat>(
            json!({})
        )
        .is_ok()
    );
}

#[test]
fn get_voicemail_attachment_formats_response_deserializes() {
    let r: GetVoicemailAttachmentFormatsResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<GetVoicemailAttachmentFormatsResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_folders_response_folder_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailFoldersResponseFolder>(json!({})).is_ok());
}

#[test]
fn get_voicemail_folders_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailFoldersResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_message_file_response_message_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailMessageFileResponseMessage>(json!({})).is_ok());
}

#[test]
fn get_voicemail_message_file_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailMessageFileResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_messages_response_message_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailMessagesResponseMessage>(json!({})).is_ok());
}

#[test]
fn get_voicemail_messages_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailMessagesResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_setups_response_voicemailsetup_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailSetupsResponseVoicemailsetup>(json!({})).is_ok());
}

#[test]
fn get_voicemail_setups_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailSetupsResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemail_transcriptions_response_message_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailTranscriptionsResponseMessage>(json!({})).is_ok());
}

#[test]
fn get_voicemail_transcriptions_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailTranscriptionsResponse>(json!({})).is_ok());
}

#[test]
fn get_voicemails_response_voicemail_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailsResponseVoicemail>(json!({})).is_ok());
}

#[test]
fn get_voicemails_response_deserializes() {
    assert!(serde_json::from_value::<GetVoicemailsResponse>(json!({})).is_ok());
}

#[test]
fn mail_fax_message_pdf_response_deserializes() {
    assert!(serde_json::from_value::<MailFAXMessagePDFResponse>(json!({})).is_ok());
}

#[test]
fn mark_listened_voicemail_message_response_deserializes() {
    assert!(serde_json::from_value::<MarkListenedVoicemailMessageResponse>(json!({})).is_ok());
}

#[test]
fn mark_urgent_voicemail_message_response_deserializes() {
    assert!(serde_json::from_value::<MarkUrgentVoicemailMessageResponse>(json!({})).is_ok());
}

#[test]
fn move_fax_message_response_deserializes() {
    assert!(serde_json::from_value::<MoveFAXMessageResponse>(json!({})).is_ok());
}

#[test]
fn move_folder_voicemail_message_response_deserializes() {
    assert!(serde_json::from_value::<MoveFolderVoicemailMessageResponse>(json!({})).is_ok());
}

#[test]
fn order_did_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDResponse>(json!({})).is_ok());
}

#[test]
fn order_did_international_geographic_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDInternationalGeographicResponse>(json!({})).is_ok());
}

#[test]
fn order_did_international_national_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDInternationalNationalResponse>(json!({})).is_ok());
}

#[test]
fn order_did_international_toll_free_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDInternationalTollFreeResponse>(json!({})).is_ok());
}

#[test]
fn order_did_virtual_response_deserializes() {
    assert!(serde_json::from_value::<OrderDIDVirtualResponse>(json!({})).is_ok());
}

#[test]
fn order_fax_number_response_deserializes() {
    assert!(serde_json::from_value::<OrderFAXNumberResponse>(json!({})).is_ok());
}

#[test]
fn order_toll_free_response_deserializes() {
    assert!(serde_json::from_value::<OrderTollFreeResponse>(json!({})).is_ok());
}

#[test]
fn order_vanity_response_deserializes() {
    assert!(serde_json::from_value::<OrderVanityResponse>(json!({})).is_ok());
}

#[test]
fn remove_did_vpri_response_deserializes() {
    assert!(serde_json::from_value::<RemoveDIDvPRIResponse>(json!({})).is_ok());
}

#[test]
fn search_dids_can_response_did_deserializes() {
    assert!(serde_json::from_value::<SearchDIDsCANResponseDID>(json!({})).is_ok());
}

#[test]
fn search_dids_can_response_deserializes() {
    assert!(serde_json::from_value::<SearchDIDsCANResponse>(json!({})).is_ok());
}

#[test]
fn search_dids_usa_response_did_deserializes() {
    assert!(serde_json::from_value::<SearchDIDsUSAResponseDID>(json!({})).is_ok());
}

#[test]
fn search_dids_usa_response_deserializes() {
    assert!(serde_json::from_value::<SearchDIDsUSAResponse>(json!({})).is_ok());
}

#[test]
fn search_fax_area_code_can_response_ratecenter_deserializes() {
    assert!(serde_json::from_value::<SearchFAXAreaCodeCANResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn search_fax_area_code_can_response_deserializes() {
    assert!(serde_json::from_value::<SearchFAXAreaCodeCANResponse>(json!({})).is_ok());
}

#[test]
fn search_fax_area_code_usa_response_ratecenter_deserializes() {
    assert!(serde_json::from_value::<SearchFAXAreaCodeUSAResponseRatecenter>(json!({})).is_ok());
}

#[test]
fn search_fax_area_code_usa_response_deserializes() {
    assert!(serde_json::from_value::<SearchFAXAreaCodeUSAResponse>(json!({})).is_ok());
}

#[test]
fn search_toll_free_can_us_response_did_deserializes() {
    assert!(serde_json::from_value::<SearchTollFreeCANUSResponseDID>(json!({})).is_ok());
}

#[test]
fn search_toll_free_can_us_response_deserializes() {
    assert!(serde_json::from_value::<SearchTollFreeCANUSResponse>(json!({})).is_ok());
}

#[test]
fn search_toll_free_usa_response_did_deserializes() {
    assert!(serde_json::from_value::<SearchTollFreeUSAResponseDID>(json!({})).is_ok());
}

#[test]
fn search_toll_free_usa_response_deserializes() {
    assert!(serde_json::from_value::<SearchTollFreeUSAResponse>(json!({})).is_ok());
}

#[test]
fn search_vanity_response_did_deserializes() {
    assert!(serde_json::from_value::<SearchVanityResponseDID>(json!({})).is_ok());
}

#[test]
fn search_vanity_response_deserializes() {
    assert!(serde_json::from_value::<SearchVanityResponse>(json!({})).is_ok());
}

#[test]
fn send_call_recording_email_response_deserializes() {
    assert!(serde_json::from_value::<SendCallRecordingEmailResponse>(json!({})).is_ok());
}

#[test]
fn send_fax_message_response_deserializes() {
    let r: SendFAXMessageResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendFAXMessageResponse>(json!({})).is_ok());
}

#[test]
fn send_mms_response_deserializes() {
    let r: SendMMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendMMSResponse>(json!({})).is_ok());
}

#[test]
fn send_sms_response_deserializes() {
    let r: SendSMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendSMSResponse>(json!({})).is_ok());
}

#[test]
fn send_voicemail_email_response_deserializes() {
    let r: SendVoicemailEmailResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SendVoicemailEmailResponse>(json!({})).is_ok());
}

#[test]
fn set_call_hunting_response_deserializes() {
    let r: SetCallHuntingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetCallHuntingResponse>(json!({})).is_ok());
}

#[test]
fn set_call_parking_response_deserializes() {
    let r: SetCallParkingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetCallParkingResponse>(json!({})).is_ok());
}

#[test]
fn set_callback_response_deserializes() {
    let r: SetCallbackResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetCallbackResponse>(json!({})).is_ok());
}

#[test]
fn set_caller_id_filtering_response_deserializes() {
    let r: SetCallerIDFilteringResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetCallerIDFilteringResponse>(json!({})).is_ok());
}

#[test]
fn set_client_response_deserializes() {
    let r: SetClientResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetClientResponse>(json!({})).is_ok());
}

#[test]
fn set_client_threshold_response_deserializes() {
    let r: SetClientThresholdResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetClientThresholdResponse>(json!({})).is_ok());
}

#[test]
fn set_conference_response_deserializes() {
    let r: SetConferenceResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetConferenceResponse>(json!({})).is_ok());
}

#[test]
fn set_conference_member_response_deserializes() {
    let r: SetConferenceMemberResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetConferenceMemberResponse>(json!({})).is_ok());
}

#[test]
fn set_did_billing_type_response_deserializes() {
    let r: SetDIDBillingTypeResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDBillingTypeResponse>(json!({})).is_ok());
}

#[test]
fn set_did_info_response_deserializes() {
    let r: SetDIDInfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDInfoResponse>(json!({})).is_ok());
}

#[test]
fn set_did_pop_response_deserializes() {
    let r: SetDIDPOPResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDPOPResponse>(json!({})).is_ok());
}

#[test]
fn set_did_routing_response_deserializes() {
    let r: SetDIDRoutingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDRoutingResponse>(json!({})).is_ok());
}

#[test]
fn set_did_voicemail_response_deserializes() {
    let r: SetDIDVoicemailResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDIDVoicemailResponse>(json!({})).is_ok());
}

#[test]
fn set_disa_response_deserializes() {
    let r: SetDISAResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetDISAResponse>(json!({})).is_ok());
}

#[test]
fn set_email_to_fax_response_deserializes() {
    let r: SetEmailToFAXResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetEmailToFAXResponse>(json!({})).is_ok());
}

#[test]
fn set_fax_folder_response_deserializes() {
    let r: SetFAXFolderResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetFAXFolderResponse>(json!({})).is_ok());
}

#[test]
fn set_fax_number_email_response_deserializes() {
    let r: SetFAXNumberEmailResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetFAXNumberEmailResponse>(json!({})).is_ok());
}

#[test]
fn set_fax_number_info_response_deserializes() {
    let r: SetFAXNumberInfoResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetFAXNumberInfoResponse>(json!({})).is_ok());
}

#[test]
fn set_fax_number_url_callback_response_deserializes() {
    let r: SetFAXNumberURLCallbackResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetFAXNumberURLCallbackResponse>(json!({})).is_ok());
}

#[test]
fn set_forwarding_response_deserializes() {
    let r: SetForwardingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetForwardingResponse>(json!({})).is_ok());
}

#[test]
fn set_ivr_response_deserializes() {
    let r: SetIVRResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetIVRResponse>(json!({})).is_ok());
}

#[test]
fn set_location_response_deserializes() {
    let r: SetLocationResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetLocationResponse>(json!({})).is_ok());
}

#[test]
fn set_music_on_hold_response_deserializes() {
    let r: SetMusicOnHoldResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetMusicOnHoldResponse>(json!({})).is_ok());
}

#[test]
fn set_phonebook_response_deserializes() {
    let r: SetPhonebookResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetPhonebookResponse>(json!({})).is_ok());
}

#[test]
fn set_phonebook_group_response_deserializes() {
    let r: SetPhonebookGroupResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetPhonebookGroupResponse>(json!({})).is_ok());
}

#[test]
fn set_queue_response_deserializes() {
    let r: SetQueueResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetQueueResponse>(json!({})).is_ok());
}

#[test]
fn set_recording_response_deserializes() {
    let r: SetRecordingResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetRecordingResponse>(json!({})).is_ok());
}

#[test]
fn set_ring_group_response_deserializes() {
    let r: SetRingGroupResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetRingGroupResponse>(json!({})).is_ok());
}

#[test]
fn set_sip_uri_response_deserializes() {
    let r: SetSIPURIResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetSIPURIResponse>(json!({})).is_ok());
}

#[test]
fn set_sms_response_deserializes() {
    let r: SetSMSResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetSMSResponse>(json!({})).is_ok());
}

#[test]
fn set_static_member_response_deserializes() {
    let r: SetStaticMemberResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetStaticMemberResponse>(json!({})).is_ok());
}

#[test]
fn set_sub_account_response_deserializes() {
    let r: SetSubAccountResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetSubAccountResponse>(json!({})).is_ok());
}

#[test]
fn set_time_condition_response_deserializes() {
    let r: SetTimeConditionResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetTimeConditionResponse>(json!({})).is_ok());
}

#[test]
fn set_voicemail_response_deserializes() {
    let r: SetVoicemailResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SetVoicemailResponse>(json!({})).is_ok());
}

#[test]
fn signup_client_response_deserializes() {
    let r: SignupClientResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<SignupClientResponse>(json!({})).is_ok());
}

#[test]
fn unconnect_did_response_deserializes() {
    let r: UnconnectDIDResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<UnconnectDIDResponse>(json!({})).is_ok());
}

#[test]
fn unconnect_fax_response_deserializes() {
    let r: UnconnectFAXResponse = serde_json::from_value(json!({})).unwrap();
    _ = &r;
    assert!(serde_json::from_value::<UnconnectFAXResponse>(json!({})).is_ok());
}
