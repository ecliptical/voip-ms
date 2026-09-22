use rust_decimal::Decimal;
use serde_json::{Value, json};
use voip_ms::{
    ApiStatus, Client, Error, GetBalanceParams, GetCDRParams, GetConferenceParams,
    GetSubAccountsParams, GetSubAccountsResponse, MaxMembers, ParamsError,
};
use wiremock::matchers::{
    body_string_contains, header_regex, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a `Client` pointed at a mock server's REST endpoint.
async fn fixture() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let base_url = format!("{}/api/v1/rest.php", server.uri()).parse().unwrap();
    let client = Client::builder("user@example.com", "secret")
        .base_url(base_url)
        .build();
    (server, client)
}

#[tokio::test]
async fn call_success_returns_full_envelope() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("api_username", "user@example.com"))
        .and(query_param("api_password", "secret"))
        .and(query_param("method", "getBalance"))
        .and(query_param("advanced", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "balance": {
                "current_balance": "1.234",
                "spent_total": "0.000"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client
        .get_balance_raw(&GetBalanceParams {
            advanced: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(resp["status"], "success");
    assert_eq!(resp["balance"]["current_balance"], "1.234");
}

#[tokio::test]
async fn api_status_other_than_success_is_an_error() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"status": "invalid_credentials"})),
        )
        .mount(&server)
        .await;

    let err = client
        .get_balance_raw(&GetBalanceParams::default())
        .await
        .unwrap_err();

    match err {
        Error::Api(s) => {
            assert_eq!(s, ApiStatus::InvalidCredentials);
            assert_eq!(s.as_str(), "invalid_credentials");
            assert_eq!(s.description(), Some("Username or Password is incorrect"));
            assert!(s.is_documented());
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[test]
fn api_status_variants_and_descriptions() {
    // A documented code round-trips to its typed variant and description.
    let status = ApiStatus::from_wire("api_not_enabled");
    assert_eq!(status, ApiStatus::APINotEnabled);
    assert_eq!(status.as_str(), "api_not_enabled");
    assert_eq!(
        status.description(),
        Some("API has not been enabled or has been disabled")
    );
    assert!(status.is_documented());
    assert_eq!(status.to_string(), "api_not_enabled");

    // Acronym-aware variant naming.
    assert_eq!(ApiStatus::from_wire("no_did"), ApiStatus::NoDID);

    // Codes the docs ship capitalized keep their wire casing on the wire
    // side while normalizing to the lowercase-sibling variant form.
    assert_eq!(
        ApiStatus::InvalidThreshold.as_str(),
        "Invalid_threshold",
        "verbatim wire casing is preserved"
    );
    assert_eq!(
        ApiStatus::from_wire("Invalid_threshold"),
        ApiStatus::InvalidThreshold
    );

    // An undocumented code is preserved verbatim with no description.
    let unknown = ApiStatus::from_wire("brand_new_code");
    assert_eq!(unknown, ApiStatus::Unknown("brand_new_code".to_string()));
    assert_eq!(unknown.as_str(), "brand_new_code");
    assert_eq!(unknown.description(), None);
    assert!(!unknown.is_documented());

    // `From<String>`/`From<&str>` keep working against the enum.
    assert_eq!(
        ApiStatus::from("invalid_credentials"),
        ApiStatus::InvalidCredentials
    );

    // Empty-collection statuses are flagged; `no_*` codes that signal a
    // real failure are not.
    assert!(ApiStatus::NoSMS.is_empty_collection());
    assert!(ApiStatus::NoCDR.is_empty_collection());
    assert!(ApiStatus::NoMessages.is_empty_collection());
    assert!(!ApiStatus::NoProvision.is_empty_collection());
    assert!(!ApiStatus::NoBase64file.is_empty_collection());
    assert!(!ApiStatus::NoCallstatus.is_empty_collection());
    assert!(!ApiStatus::InvalidCredentials.is_empty_collection());
    assert!(!ApiStatus::Unknown("whatever".to_string()).is_empty_collection());
}

#[tokio::test]
async fn http_error_status_is_surfaced() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let err = client
        .get_balance_raw(&GetBalanceParams::default())
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Http(_)), "got {err:?}");
}

#[tokio::test]
async fn response_without_status_field_is_invalid() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"unexpected": "shape"})))
        .mount(&server)
        .await;

    let err = client
        .get_balance_raw(&GetBalanceParams::default())
        .await
        .unwrap_err();

    assert!(matches!(err, Error::InvalidResponse(_)), "got {err:?}");
}

#[tokio::test]
async fn empty_body_is_treated_as_success() {
    // delConference and similar answer a successful call with an empty body;
    // that must read as success, not a JSON parse error.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    let body = client
        .call_raw("delConference", &GetBalanceParams::default())
        .await
        .expect("empty body must classify as success");
    assert_eq!(body["status"], "success");
}

#[tokio::test]
async fn omitted_optional_params_are_not_sent() {
    let (server, client) = fixture().await;

    // Match a request that has the credentials + method but NO `advanced` param.
    let mock = Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "success"})))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    client
        .get_balance_raw(&GetBalanceParams { advanced: None })
        .await
        .unwrap();

    // Drop the scoped mock to verify the expectation.
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    let qs = received[0].url.query().unwrap_or_default();
    assert!(
        !qs.contains("advanced="),
        "advanced param should be omitted, query was: {qs}"
    );
    drop(mock);
}

#[tokio::test]
async fn typed_response_via_call_raw_helper() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "balance": {"current_balance": "5.00"}
        })))
        .mount(&server)
        .await;

    #[derive(serde::Deserialize)]
    struct Balance {
        current_balance: String,
    }

    let body: Value = client
        .call_raw("getBalance", &GetBalanceParams::default())
        .await
        .unwrap();

    let balance: Balance = serde_json::from_value(body["balance"].clone()).unwrap();
    assert_eq!(balance.current_balance, "5.00");
}

#[tokio::test]
async fn typed_response_via_call_helper() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "balance": {"current_balance": "7.50"}
        })))
        .mount(&server)
        .await;

    #[derive(serde::Deserialize)]
    struct Envelope {
        balance: Balance,
        status: String,
    }

    #[derive(serde::Deserialize)]
    struct Balance {
        current_balance: String,
    }

    let envelope: Envelope = client
        .call("getBalance", &GetBalanceParams::default())
        .await
        .unwrap();

    assert_eq!(envelope.status, "success");
    assert_eq!(envelope.balance.current_balance, "7.50");
}

#[tokio::test]
async fn typed_response_via_call_at_helper() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "balance": {"current_balance": "9.99"}
        })))
        .mount(&server)
        .await;

    #[derive(serde::Deserialize)]
    struct Balance {
        current_balance: String,
    }

    let balance: Balance = client
        .call_at("getBalance", &GetBalanceParams::default(), "/balance")
        .await
        .unwrap();

    assert_eq!(balance.current_balance, "9.99");
}

#[tokio::test]
async fn typed_response_via_generated_default_method() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "balance": {"current_balance": "12.00"}
        })))
        .mount(&server)
        .await;

    let envelope = client
        .get_balance(&GetBalanceParams::default())
        .await
        .unwrap();

    let current_balance = envelope
        .balance
        .as_ref()
        .and_then(|balance| balance.current_balance);
    assert_eq!(current_balance, Some(Decimal::new(1200, 2)));
}

#[tokio::test]
async fn typed_get_sub_accounts_tolerates_minus_one_sentinel_values() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSubAccounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "accounts": [
                {
                    "id": "123",
                    "account": "100000_fixture",
                    "username": "fixture-sub",
                    "callerid_number": "-1",
                    "canada_routing": -1
                }
            ]
        })))
        .mount(&server)
        .await;

    let envelope: GetSubAccountsResponse = client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await
        .unwrap();

    let account = envelope
        .accounts
        .first()
        .expect("expected at least one sub-account");
    assert_eq!(account.id, Some(123));
    assert_eq!(account.callerid_number, None);
    assert_eq!(account.canada_routing, None);
}

#[tokio::test]
async fn typed_get_sub_accounts_decodes_enum_and_routing_fields() {
    use voip_ms::{DtmfMode, Nat};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSubAccounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "accounts": [
                {
                    "id": "1",
                    "account": "100000_fixture",
                    "dtmf_mode": "rfc2833",
                    "nat": "route"
                }
            ]
        })))
        .mount(&server)
        .await;

    let envelope: GetSubAccountsResponse = client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await
        .unwrap();

    let account = envelope
        .accounts
        .first()
        .expect("expected at least one sub-account");

    assert_eq!(account.dtmf_mode, Some(DtmfMode::Rfc2833));
    assert_eq!(account.nat, Some(Nat::Route));
}

#[tokio::test]
async fn routing_param_serializes_as_tagged_string() {
    use voip_ms::{Routing, SetDIDInfoParams};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setDIDInfo"))
        .and(query_param("routing", "account:100001_VoIP"))
        .and(query_param("failover_unreachable", "none:"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = SetDIDInfoParams {
        did: Some("5551234567".into()),
        routing: Some(Routing::Account("100001_VoIP".into())),
        failover_unreachable: Some(Routing::None),
        ..Default::default()
    };
    client.set_did_info_raw(&params).await.unwrap();
}

#[tokio::test]
async fn flag_params_serialize_to_wire_form() {
    // A `1`/`0` flag param is a plain `Option<bool>`; its `serialize_with`
    // emits `1`/`0`, not the `true`/`false` a bare `bool` would produce, which
    // VoIP.ms rejects for these parameters.
    use voip_ms::SetSMSParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setSMS"))
        .and(query_param("enable", "1"))
        .and(query_param("email_enabled", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = SetSMSParams {
        did: Some("5551234567".into()),
        enable: Some(true),
        email_enabled: Some(false),
        ..Default::default()
    };
    client.set_sms_raw(&params).await.unwrap();
}

#[tokio::test]
async fn yes_no_flag_param_serializes_to_word() {
    use voip_ms::SetConferenceMemberParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setConferenceMember"))
        .and(query_param("admin", "yes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = SetConferenceMemberParams {
        admin: Some(true),
        ..Default::default()
    };
    client.set_conference_member_raw(&params).await.unwrap();
}

#[tokio::test]
async fn money_and_id_params_serialize_exactly() {
    // Money params are `Decimal`, whose `Serialize` emits the exact decimal
    // string -- no float artifacts on the two methods that move money. Ids are
    // `u64`, matching the response side.
    use voip_ms::AddChargeParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "addCharge"))
        .and(query_param("client", "250071"))
        .and(query_param("charge", "1.25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = AddChargeParams {
        client: Some(250071),
        charge: Some(Decimal::new(125, 2)),
        test: true,
        ..Default::default()
    };
    client.add_charge_raw(&params).await.unwrap();
}

#[tokio::test]
async fn date_params_serialize_as_iso_dates() {
    // Date-range params are `NaiveDate`; its `Serialize` emits the
    // `YYYY-MM-DD` wire form the docs specify.
    use voip_ms::GetCDRParams;
    use voip_ms::chrono::NaiveDate;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .and(query_param("date_from", "2026-01-01"))
        .and(query_param("date_to", "2026-01-31"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetCDRParams {
        date_from: NaiveDate::from_ymd_opt(2026, 1, 1),
        date_to: NaiveDate::from_ymd_opt(2026, 1, 31),
        ..Default::default()
    };
    client.get_cdr_raw(&params).await.unwrap();
}

#[tokio::test]
async fn snake_cased_param_keeps_camel_case_wire_name() {
    // A field whose wire name is camelCase gets a snake_case Rust ident with
    // a serde `rename` back to the wire form; `is_mobile` is also a `1`/`0`
    // flag, so both attributes compose on the same field.
    use voip_ms::AddLNPPortParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "addLNPPort"))
        .and(query_param("isMobile", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = AddLNPPortParams {
        is_mobile: Some(true),
        ..Default::default()
    };
    client.add_lnp_port_raw(&params).await.unwrap();
}

#[tokio::test]
async fn true_only_flag_present_when_true() {
    // `test` is a plain `bool`: `true` serializes `1`, and `false` (the
    // default) is left off the wire entirely.
    use voip_ms::OrderDIDParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "orderDID"))
        .and(query_param("test", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = OrderDIDParams {
        test: true,
        ..Default::default()
    };
    client.order_did_raw(&params).await.unwrap();
}

#[tokio::test]
async fn true_only_flag_absent_when_false() {
    use voip_ms::OrderDIDParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "orderDID"))
        .and(query_param_is_missing("test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = OrderDIDParams::default();
    client.order_did_raw(&params).await.unwrap();
}

#[tokio::test]
async fn unknown_enum_value_is_preserved_verbatim() {
    use voip_ms::DtmfMode;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSubAccounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "accounts": [
                { "id": "1", "dtmf_mode": "future_mode" }
            ]
        })))
        .mount(&server)
        .await;

    let envelope: GetSubAccountsResponse = client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await
        .unwrap();
    let account = envelope.accounts.into_iter().next().unwrap();
    assert_eq!(
        account.dtmf_mode,
        Some(DtmfMode::Unknown("future_mode".into())),
    );
}

#[tokio::test]
async fn queue_empty_behavior_param_serializes_to_wire() {
    // The third value (`strict`) is why this is an enum, not a bool: a bool
    // would lose it. The param serializes to its bare wire string.
    use voip_ms::{QueueEmptyBehavior, SetQueueParams};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setQueue"))
        .and(query_param("leave_when_empty", "strict"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = SetQueueParams {
        leave_when_empty: Some(QueueEmptyBehavior::Strict),
        ..Default::default()
    };
    client.set_queue_raw(&params).await.unwrap();
}

#[tokio::test]
async fn queue_empty_behavior_response_deserializes_third_value() {
    use voip_ms::QueueEmptyBehavior;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getQueues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "queues": [
                { "queue_name": "support", "leave_when_empty": "strict" }
            ]
        })))
        .mount(&server)
        .await;

    let envelope = client
        .get_queues(&voip_ms::GetQueuesParams::default())
        .await
        .unwrap();
    let queue = envelope.queues.into_iter().next().unwrap();
    assert_eq!(queue.leave_when_empty, Some(QueueEmptyBehavior::Strict));
}

#[tokio::test]
async fn per_struct_type_enum_serializes() {
    // The same field name (`type`) is a search mode here and a message
    // direction elsewhere; the per-struct override picks the right enum.
    use voip_ms::{GetSMSParams, MessageType, SearchDIDsUSAParams, SearchType};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "searchDIDsUSA"))
        .and(query_param("type", "starts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;
    client
        .search_dids_usa_raw(&SearchDIDsUSAParams {
            search_type: Some(SearchType::Starts),
            ..Default::default()
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSMS"))
        .and(query_param("type", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;
    client
        .get_sms_raw(&GetSMSParams {
            direction: Some(MessageType::Received),
            ..Default::default()
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn message_type_response_deserializes_numeric_wire() {
    // VoIP.ms returns the SMS `type` as a bare JSON number (1 = received,
    // 0 = sent), not a string -- the enum deserializer must tolerate that.
    use voip_ms::MessageType;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSMS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "sms": [
                { "id": 1, "type": 1 },
                { "id": 2, "type": 0 }
            ]
        })))
        .mount(&server)
        .await;

    let envelope = client
        .get_sms(&voip_ms::GetSMSParams::default())
        .await
        .unwrap();
    let msgs = envelope.sms;
    assert_eq!(msgs[0].direction, Some(MessageType::Received));
    assert_eq!(msgs[1].direction, Some(MessageType::Sent));
}

#[tokio::test]
async fn empty_collection_status_yields_empty_response() {
    // VoIP.ms answers an empty SMS list with `{"status": "no_sms"}` and no
    // `sms` field. For the typed call that is an empty list, not an error:
    // it succeeds with an empty `sms`. The `*_raw` escape hatch keeps the
    // verbatim contract and still surfaces it as `Error::Api`.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSMS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "no_sms" })))
        .mount(&server)
        .await;

    let envelope = client
        .get_sms(&voip_ms::GetSMSParams::default())
        .await
        .unwrap();
    assert_eq!(envelope.status, ApiStatus::NoSMS);
    assert!(envelope.sms.is_empty());

    let err = client
        .get_sms_raw(&voip_ms::GetSMSParams::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Api(ApiStatus::NoSMS)), "got {err:?}");
}

#[tokio::test]
async fn real_error_no_status_still_errors() {
    // `no_*` codes that signal a genuine failure (rather than an empty list)
    // are still surfaced as `Error::Api`.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "no_provision" })))
        .mount(&server)
        .await;

    let err = client
        .e911_provision_raw(&voip_ms::E911ProvisionParams::default())
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Api(ApiStatus::NoProvision)),
        "got {err:?}"
    );
}

#[tokio::test]
async fn integer_coded_enum_round_trips() {
    // billing_type is a numeric coded enum on the wire (1 = per minute,
    // 2 = flat); it serializes to the digit and parses back from a number.
    use voip_ms::{DidBillingType, GetDIDsInfoParams, SetDIDBillingTypeParams};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setDIDBillingType"))
        .and(query_param("billing_type", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;
    client
        .set_did_billing_type_raw(&SetDIDBillingTypeParams {
            billing_type: Some(DidBillingType::Flat),
            ..Default::default()
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getDIDsInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "dids": [ { "did": "5551234567", "billing_type": 1 } ]
        })))
        .mount(&server)
        .await;
    let envelope = client
        .get_dids_info(&GetDIDsInfoParams::default())
        .await
        .unwrap();
    let did = envelope.dids.into_iter().next().unwrap();
    assert_eq!(did.billing_type, Some(DidBillingType::PerMinute));
}

#[tokio::test]
async fn seconds_param_serializes_value_and_sentinel() {
    // A queue duration is a count of seconds OR a no-limit sentinel that
    // serializes to its documented word (`none` / `unlimited`).
    use voip_ms::{Seconds, SetQueueParams, WaitTime};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setQueue"))
        .and(query_param("retry_timer", "30"))
        .and(query_param("wrapup_time", "none"))
        .and(query_param("maximum_wait_time", "unlimited"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .set_queue_raw(&SetQueueParams {
            retry_timer: Some(Seconds::Value(30)),
            wrapup_time: Some(Seconds::Unlimited),
            maximum_wait_time: Some(WaitTime::Unlimited),
            ..Default::default()
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn seconds_response_deserializes_number_and_sentinel() {
    use voip_ms::{Seconds, WaitTime};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getQueues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "queues": [
                { "queue_name": "q", "retry_timer": 30, "maximum_wait_time": "unlimited" }
            ]
        })))
        .mount(&server)
        .await;

    let envelope = client
        .get_queues(&voip_ms::GetQueuesParams::default())
        .await
        .unwrap();
    let queue = envelope.queues.into_iter().next().unwrap();
    assert_eq!(queue.retry_timer, Some(Seconds::Value(30)));
    assert_eq!(queue.maximum_wait_time, Some(WaitTime::Unlimited));
}

#[tokio::test]
async fn fax_message_folder_is_free_text_not_voicemail_enum() {
    // `folder` is a `VoicemailFolder` enum on voicemail methods, but a fax
    // folder is a free-text name (`SENT`/`ALL`/user-created) outside that
    // variant set; `getFaxMessages.folder` must stay a `String` so it
    // serializes verbatim. (Regression: the global `folder` override once
    // mistyped this as `VoicemailFolder`.)
    use voip_ms::GetFAXMessagesParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getFaxMessages"))
        .and(query_param("folder", "SENT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .get_fax_messages_raw(&GetFAXMessagesParams {
            folder: Some("SENT".into()),
            ..Default::default()
        })
        .await
        .unwrap();
}

#[test]
fn single_object_list_field_coerces_to_one_element_vec() {
    use voip_ms::GetVoicemailMessageFileResponse;

    // A fetch-one method returns its list field as a bare object, not a
    // one-element array; the tolerant deserializer wraps it into a `Vec`.
    let single: GetVoicemailMessageFileResponse = serde_json::from_value(json!({
        "status": "success",
        "message": { "mailbox": "1001", "folder": "INBOX", "message_num": "1", "data": "Zm9v" }
    }))
    .unwrap();
    let msgs = single.message;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].data.as_deref(), Some("Zm9v"));

    // A genuine array is preserved as-is (no regression).
    let many: GetVoicemailMessageFileResponse = serde_json::from_value(json!({
        "status": "success",
        "message": [
            { "message_num": "1", "data": "Zm9v" },
            { "message_num": "2", "data": "YmFy" }
        ]
    }))
    .unwrap();
    assert_eq!(many.message.len(), 2);

    // A null / absent list field stays empty.
    let empty: GetVoicemailMessageFileResponse =
        serde_json::from_value(json!({ "status": "success", "message": null })).unwrap();
    assert!(empty.message.is_empty());
}

#[test]
fn callerid_accepts_named_display_form() {
    use voip_ms::{GetFAXMessagesResponse, GetVoicemailMessagesResponse};

    // VoIP.ms fills `callerid` on an inbound message with the caller's
    // display form -- a name and number in angle brackets -- not a bare
    // number. Typed as `String`, the whole envelope must still deserialize.
    let vm: GetVoicemailMessagesResponse = serde_json::from_value(json!({
        "status": "success",
        "messages": [{
            "mailbox": "1001",
            "folder": "INBOX",
            "message_num": "1",
            "callerid": "PARKWOODSDENTAL <4164442828>",
            "duration": "00:00:06"
        }]
    }))
    .unwrap();
    let messages = vm.messages;
    assert_eq!(
        messages[0].callerid.as_deref(),
        Some("PARKWOODSDENTAL <4164442828>")
    );

    // A purely numeric caller ID still round-trips as its string form.
    let fax: GetFAXMessagesResponse = serde_json::from_value(json!({
        "status": "success",
        "faxes": [{ "id": "42", "callerid": "5552341234" }]
    }))
    .unwrap();
    let faxes = fax.faxes;
    assert_eq!(faxes[0].callerid.as_deref(), Some("5552341234"));
}

#[test]
fn voicemail_message_date_accepts_full_timestamp() {
    use voip_ms::GetVoicemailMessagesResponse;

    // VoIP.ms fills `date` on a voicemail message with a full timestamp, not
    // a bare date. Typed as `NaiveDate`, this failed with "trailing input".
    let vm: GetVoicemailMessagesResponse = serde_json::from_value(json!({
        "status": "success",
        "messages": [{
            "mailbox": "1001",
            "folder": "INBOX",
            "message_num": "1",
            "date": "2023-06-26 15:37:05",
            "duration": "00:00:06"
        }]
    }))
    .unwrap();
    let messages = vm.messages;
    assert_eq!(
        messages[0].date.as_ref().and_then(voip_ms::Reported::get),
        Some(
            chrono::NaiveDate::from_ymd_opt(2023, 6, 26)
                .unwrap()
                .and_hms_opt(15, 37, 5)
                .unwrap()
        )
    );
}

#[test]
fn termination_rates_route_is_a_list() {
    use voip_ms::GetTerminationRatesResponse;

    // VoIP.ms returns `route` as a one-element list of `{value, description}`,
    // not a bare object. Typed as a single object, deserializing the array
    // mapped its first element positionally onto the `value` field and failed
    // with "expected string or number, got {...}".
    let resp: GetTerminationRatesResponse = serde_json::from_value(json!({
        "status": "success",
        "route": [{ "value": "1", "description": "Value" }],
        "rates": [{
            "destination": "Canada - 204 Manitoba",
            "prefix": "1204",
            "increment": "6",
            "rate": 0.0052
        }]
    }))
    .unwrap();

    let route = resp.route.first().expect("one route entry");
    assert_eq!(route.value, Some(1));
    assert_eq!(route.description.as_deref(), Some("Value"));
    assert_eq!(resp.rates[0].prefix, Some(1204));
    assert_eq!(resp.rates[0].rate, Some(Decimal::new(52, 4)));
}

#[test]
fn e911_address_types_is_a_list() {
    use voip_ms::E911AddressTypesResponse;

    // The address types come back as a list of `{value, description}` catalog
    // entries (like every other reference catalog), not flattened scalars.
    let resp: E911AddressTypesResponse = serde_json::from_value(json!({
        "status": "success",
        "types": [
            { "value": "Apartment", "description": "Apartment" },
            { "value": "Basement", "description": "Basement" }
        ]
    }))
    .unwrap();

    assert_eq!(resp.types.len(), 2);
    assert_eq!(resp.types[0].value.as_deref(), Some("Apartment"));
    assert_eq!(resp.types[1].description.as_deref(), Some("Basement"));
}

#[test]
fn search_fax_area_code_can_is_a_ratecenter_list() {
    use voip_ms::SearchFAXAreaCodeCANResponse;

    // Matching area codes return `ratecenters` as a list of
    // `{area_code, available, ratecenter}` objects; typed as a scalar it
    // failed with "expected string, number, or bool, got [{...}]".
    let resp: SearchFAXAreaCodeCANResponse = serde_json::from_value(json!({
        "status": "success",
        "ratecenters": [
            { "area_code": "514", "available": "yes", "ratecenter": "ILE-PERROT" },
            { "area_code": "514", "available": "yes", "ratecenter": "LACHINE" },
            { "area_code": "514", "available": "yes", "ratecenter": "POINTE-CLAIRE" },
            { "area_code": "514", "available": "yes", "ratecenter": "STE-GENEVIEVE" },
            { "area_code": "514", "available": "yes", "ratecenter": "ROXBORO" }
        ]
    }))
    .unwrap();

    assert_eq!(resp.ratecenters.len(), 5);
    assert_eq!(resp.ratecenters[0].area_code, Some(514));
    assert_eq!(resp.ratecenters[0].available, Some(true));
    assert_eq!(
        resp.ratecenters[0].ratecenter.as_deref(),
        Some("ILE-PERROT")
    );
    assert_eq!(resp.ratecenters[4].ratecenter.as_deref(), Some("ROXBORO"));
}

#[test]
fn search_fax_area_code_usa_is_a_ratecenter_list() {
    use voip_ms::SearchFAXAreaCodeUSAResponse;

    // Same shape as the Canadian variant: a list of
    // `{area_code, available, ratecenter}` objects.
    let resp: SearchFAXAreaCodeUSAResponse = serde_json::from_value(json!({
        "status": "success",
        "ratecenters": [
            { "area_code": "415", "available": "yes", "ratecenter": "BELVEDERE (MARIN)" }
        ]
    }))
    .unwrap();

    assert_eq!(resp.ratecenters.len(), 1);
    assert_eq!(resp.ratecenters[0].area_code, Some(415));
    assert_eq!(resp.ratecenters[0].available, Some(true));
    assert_eq!(
        resp.ratecenters[0].ratecenter.as_deref(),
        Some("BELVEDERE (MARIN)")
    );
}

#[test]
fn search_fax_area_code_empty_is_success() {
    use voip_ms::{SearchFAXAreaCodeCANResponse, SearchFAXAreaCodeUSAResponse};

    // Area codes with zero matches return `{"status":"success"}` with no
    // `ratecenters` field at all; the list must default to empty, not fail.
    let can: SearchFAXAreaCodeCANResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    assert!(can.ratecenters.is_empty());

    let usa: SearchFAXAreaCodeUSAResponse =
        serde_json::from_value(json!({ "status": "success" })).unwrap();
    assert!(usa.ratecenters.is_empty());
}

#[test]
fn fax_numbers_info_did_accepts_dotted_string() {
    use voip_ms::GetFAXNumbersInfoResponse;

    // A fax number's `did` arrives dotted (`647.948.4755`), not as a bare
    // integer. Typed `u64`, it failed with "invalid digit found in string".
    let resp: GetFAXNumbersInfoResponse = serde_json::from_value(json!({
        "status": "success",
        "numbers": [{ "id": "0000", "did": "647.948.4755" }]
    }))
    .unwrap();

    assert_eq!(resp.numbers[0].did.as_deref(), Some("647.948.4755"));
}

#[test]
fn lnp_list_status_is_a_string_keyed_map() {
    use voip_ms::GetLNPListStatusResponse;

    // `list_status` is a `code => description` map whose keys are data
    // (including an empty-string key). Typed as a scalar, it failed with
    // "expected string, number, or bool, got {...}".
    let resp: GetLNPListStatusResponse = serde_json::from_value(json!({
        "status": "success",
        "list_status": {
            "awaiting_documentation": "Awaiting Documentation",
            "cancelled": "Cancelled",
            "completed": "Completed",
            "": "Port cancel upon customer request"
        }
    }))
    .unwrap();

    assert_eq!(
        resp.list_status
            .get("awaiting_documentation")
            .map(String::as_str),
        Some("Awaiting Documentation")
    );
    assert_eq!(
        resp.list_status.get("").map(String::as_str),
        Some("Port cancel upon customer request")
    );
    assert_eq!(resp.list_status.len(), 4);
}

#[test]
fn report_estimated_hold_time_values_are_strings() {
    use voip_ms::GetReportEstimatedHoldTimeResponse;

    // The types catalog carries free-text values -- `value: "once"` with
    // `description: "Yes, only once"` -- not booleans. Typed `bool`, the
    // "Yes, only once" description failed the yes/no coercion.
    let resp: GetReportEstimatedHoldTimeResponse = serde_json::from_value(json!({
        "status": "success",
        "types": [
            { "value": "yes", "description": "Yes" },
            { "value": "no", "description": "No" },
            { "value": "once", "description": "Yes, only once" }
        ]
    }))
    .unwrap();

    assert_eq!(resp.types.len(), 3);
    assert_eq!(resp.types[2].value.as_deref(), Some("once"));
    assert_eq!(resp.types[2].description.as_deref(), Some("Yes, only once"));
}

#[test]
fn e911_info_is_a_nested_object() {
    use voip_ms::E911InfoResponse;

    // A provisioned DID's e911 record comes back as a nested `info` object, not
    // flattened top-level scalars. Modeled as a scalar it failed with
    // "expected string, number, or bool, got {...}".
    let resp: E911InfoResponse = serde_json::from_value(json!({
        "status": "success",
        "info": {
            "did": "7472127447",
            "status": "2",
            "full_name": "test",
            "street_number": "23",
            "street_name": "W BROAD ST",
            "address_type": "Hanger",
            "city": "RICHMOND",
            "state": "VA",
            "zip_code": "12345",
            "country": "US"
        }
    }))
    .unwrap();

    let info = resp.info.expect("info object");
    assert_eq!(info.did.as_deref(), Some("7472127447"));
    assert_eq!(info.city.as_deref(), Some("RICHMOND"));
}

#[test]
fn lnp_list_is_a_list_of_orders() {
    use voip_ms::GetLNPListResponse;

    // `list` is a list of port orders, not flattened sibling scalars.
    let resp: GetLNPListResponse = serde_json::from_value(json!({
        "status": "success",
        "list": [
            { "portid": "0000", "numbers": "5551234567", "foc_date": "2019-01-08", "status": "processing" },
            { "portid": "1111", "numbers": "5551234568", "foc_date": false, "status": "completed" }
        ]
    }))
    .unwrap();

    assert_eq!(resp.list.len(), 2);
    assert_eq!(resp.list[0].portid.as_deref(), Some("0000"));
    assert_eq!(resp.list[1].status.as_deref(), Some("completed"));
}

#[test]
fn lnp_notes_and_attach_list_are_lists() {
    use voip_ms::{GetLNPAttachListResponse, GetLNPNotesResponse};

    let notes: GetLNPNotesResponse = serde_json::from_value(json!({
        "status": "success",
        "list": [{ "note": "Order submitted.", "date": "2019-02-25", "time": "15:05:11" }]
    }))
    .unwrap();
    assert_eq!(notes.list.len(), 1);
    assert_eq!(notes.list[0].note.as_deref(), Some("Order submitted."));

    let attach: GetLNPAttachListResponse = serde_json::from_value(json!({
        "status": "success",
        "list": [{ "attachid": "000", "type": "pdf", "size": "151600" }]
    }))
    .unwrap();
    assert_eq!(attach.list.len(), 1);
    assert_eq!(attach.list[0].attachid.as_deref(), Some("000"));
}

#[test]
fn phone_number_identifier_fields_accept_formatted_strings() {
    use voip_ms::{GetForwardingsResponse, GetSMSResponse};

    // DID / peer-contact / caller-id fields are identifiers, not integers: they
    // can carry formatting, `+`, short codes, or non-NANP forms. Typed as `u64`
    // any of those failed integer parsing.
    let sms: GetSMSResponse = serde_json::from_value(json!({
        "status": "success",
        "sms": [{ "id": "1", "did": "+1 647-478-1287", "contact": "911" }]
    }))
    .unwrap();
    assert_eq!(sms.sms[0].did.as_deref(), Some("+1 647-478-1287"));
    assert_eq!(sms.sms[0].contact.as_deref(), Some("911"));

    let fwd: GetForwardingsResponse = serde_json::from_value(json!({
        "status": "success",
        "forwardings": [{ "forwarding": "1", "phone_number": "011 44 20 7946 0000" }]
    }))
    .unwrap();
    assert_eq!(
        fwd.forwardings[0].phone_number.as_deref(),
        Some("011 44 20 7946 0000")
    );
}

#[test]
fn callerid_override_fields_are_strings_with_minus_one_as_none() {
    use voip_ms::{GetForwardingsResponse, GetSubAccountsResponse};

    // Caller-ID override fields are phone-number strings, but voip.ms signals
    // "not set" with a `-1` sentinel (a value a real caller ID never takes),
    // which folds to `None`; a real (possibly formatted) value survives.
    let unset: GetSubAccountsResponse = serde_json::from_value(json!({
        "status": "success",
        "accounts": [{ "id": "1", "account": "a", "callerid_number": "-1", "default_e911": "" }]
    }))
    .unwrap();
    assert_eq!(unset.accounts[0].callerid_number, None);
    assert_eq!(unset.accounts[0].default_e911, None);

    let set: GetSubAccountsResponse = serde_json::from_value(json!({
        "status": "success",
        "accounts": [{ "id": "1", "account": "a", "callerid_number": "+1 (647) 478-1287" }]
    }))
    .unwrap();
    assert_eq!(
        set.accounts[0].callerid_number.as_deref(),
        Some("+1 (647) 478-1287")
    );

    let fwd: GetForwardingsResponse = serde_json::from_value(json!({
        "status": "success",
        "forwardings": [{ "forwarding": "1", "callerid_override": "-1" }]
    }))
    .unwrap();
    assert_eq!(fwd.forwardings[0].callerid_override, None);
}

#[tokio::test]
async fn typed_get_cdr_decodes_alphanumeric_uniqueid() {
    // A CDR `uniqueid` can be alphanumeric (e.g. `12964421x41098i8c`), so the
    // field must be `String`: the earlier `u64` typing failed to deserialize a
    // real value outright.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [{ "destination": "5551234567", "uniqueid": "12964421x41098i8c" }]
        })))
        .mount(&server)
        .await;

    let envelope = client.get_cdr(&GetCDRParams::default()).await.unwrap();
    let cdr = envelope.cdr.first().expect("expected at least one CDR row");
    assert_eq!(cdr.uniqueid.as_deref(), Some("12964421x41098i8c"));
    assert_eq!(cdr.destination.as_deref(), Some("5551234567"));
}

#[tokio::test]
async fn typed_get_cdr_decodes_ip_and_useragent() {
    // `ip` and `useragent` are on the wire but absent from the docs' Output
    // block, so they reach the generated struct through an `additions` entry in
    // the overrides rather than the extractor. Most rows send them as `""`,
    // which folds to `None` like any other unset scalar. The populated values
    // here are the shape a live outbound call from a registered softphone
    // returned, truncated User-Agent included -- neither is guaranteed well
    // formed, which is why both stay `String` rather than becoming `IpAddr` or
    // a parsed agent.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [
                {
                    "uniqueid": "128238059",
                    "ip": "203.0.113.7",
                    "useragent": "VoIP_ms Softphone/0.0.6 (build 2335157"
                },
                { "uniqueid": "128238060", "ip": "", "useragent": "" }
            ]
        })))
        .mount(&server)
        .await;

    let envelope = client.get_cdr(&GetCDRParams::default()).await.unwrap();
    assert_eq!(envelope.cdr[0].ip.as_deref(), Some("203.0.113.7"));
    assert_eq!(
        envelope.cdr[0].useragent.as_deref(),
        Some("VoIP_ms Softphone/0.0.6 (build 2335157")
    );
    assert_eq!(envelope.cdr[1].ip, None);
    assert_eq!(envelope.cdr[1].useragent, None);
}

#[tokio::test]
async fn typed_get_conference_decodes_unlimited_max_members() {
    // getConference reports an uncapped conference's max_members as the word
    // `Unlimited`; the earlier `u64` typing failed to deserialize it.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getConference"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "conference": [
                { "conference": "7397", "name": "lvt", "max_members": "Unlimited" },
                { "conference": "7398", "name": "capped", "max_members": "40" }
            ]
        })))
        .mount(&server)
        .await;

    let envelope = client
        .get_conference(&GetConferenceParams::default())
        .await
        .unwrap();
    assert_eq!(
        envelope.conference[0].max_members,
        Some(MaxMembers::Unlimited)
    );
    assert_eq!(
        envelope.conference[1].max_members,
        Some(MaxMembers::Value(40))
    );
}

#[tokio::test]
async fn record_listing_timezone_resolves_zone_to_offset_at_start_date() {
    // The record-listing `timezone` is a `Tz` on the public params; the wire
    // gets the zone's numeric UTC offset resolved at the query start date --
    // DST-aware, so New York is -4 in July and -5 in January.
    use voip_ms::GetSMSParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSMS"))
        .and(query_param("from", "2026-07-15"))
        .and(query_param("timezone", "-4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetSMSParams {
        from: Some("2026-07-15".into()),
        timezone: Some(voip_ms::chrono_tz::America::New_York),
        ..Default::default()
    };
    client.get_sms_raw(&params).await.unwrap();
}

#[tokio::test]
async fn cdr_timezone_resolves_offset_from_date_from() {
    // The CDR variant anchors the resolution on `date_from` (a typed date);
    // January in New York is EST, -5.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .and(query_param("timezone", "-5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetCDRParams {
        date_from: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 1, 15),
        date_to: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 1, 16),
        timezone: Some(voip_ms::chrono_tz::America::New_York),
        ..Default::default()
    };
    client.get_cdr_raw(&params).await.unwrap();
}

#[tokio::test]
async fn record_listing_without_a_zone_asks_for_utc() {
    // No zone means UTC rather than the account's own: a request that named no
    // offset would come back in a zone nothing on the response reports.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .and(query_param("timezone", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [{ "date": "2026-09-16 19:14:35", "seconds": "11" }],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = client.get_cdr(&GetCDRParams::default()).await.unwrap();
    assert_eq!(
        envelope.cdr[0].date,
        Some(voip_ms::Reported::Parsed(
            voip_ms::chrono::DateTime::parse_from_rfc3339("2026-09-16T19:14:35+00:00").unwrap()
        ))
    );
}

#[tokio::test]
async fn record_listing_timestamps_carry_the_offset_that_was_sent() {
    // voip.ms shifts the timestamps by the offset the request carried and then
    // reports the shifted wall clock without it, so the crate puts it back --
    // the same record read at -4 and at 0 is the same instant.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .and(query_param("timezone", "-4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [{ "date": "2026-09-16 15:14:35" }],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetCDRParams {
        date_from: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 9, 14),
        date_to: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 9, 17),
        timezone: Some(voip_ms::chrono_tz::America::New_York),
        ..Default::default()
    };
    let date = client.get_cdr(&params).await.unwrap().cdr[0]
        .date
        .as_ref()
        .and_then(voip_ms::Reported::get)
        .unwrap();
    assert_eq!(
        date,
        voip_ms::chrono::DateTime::parse_from_rfc3339("2026-09-16T15:14:35-04:00").unwrap()
    );
    assert_eq!(date.to_utc().to_string(), "2026-09-16 19:14:35 UTC");
}

#[tokio::test]
async fn record_listing_timestamps_qualify_a_single_bare_record() {
    // VoIP.ms returns a one-row list as a bare object; the timestamp in it is
    // reached and qualified the same way a list element's is.
    use voip_ms::GetSMSParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getSMS"))
        .and(query_param("timezone", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "sms": { "id": "111120", "date": "2026-03-30 10:24:16" },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = client.get_sms(&GetSMSParams::default()).await.unwrap();
    assert_eq!(
        envelope.sms[0].date,
        Some(voip_ms::Reported::Parsed(
            voip_ms::chrono::DateTime::parse_from_rfc3339("2026-03-30T10:24:16+00:00").unwrap()
        ))
    );
}

#[tokio::test]
async fn record_listing_blank_timestamp_does_not_lose_the_response() {
    // A blank `date` is one record's missing value, not a broken envelope: it
    // folds to `None` and every other record still deserializes.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [
                { "date": "", "uniqueid": "1" },
                { "date": "0000-00-00 00:00:00", "uniqueid": "2" },
                { "date": "2026-09-16 19:14:35", "uniqueid": "3" },
            ],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = client.get_cdr(&GetCDRParams::default()).await.unwrap();
    assert_eq!(envelope.cdr.len(), 3);
    assert_eq!(envelope.cdr[0].date, None);
    assert_eq!(envelope.cdr[1].date, None);
    assert_eq!(
        envelope.cdr[2].date,
        Some(voip_ms::Reported::Parsed(
            voip_ms::chrono::DateTime::parse_from_rfc3339("2026-09-16T19:14:35+00:00").unwrap()
        ))
    );
}

#[tokio::test]
async fn record_listing_half_hour_zone_stamps_the_offset_it_sent() {
    // A half-hour zone must survive the round trip intact: the wire carries the
    // fraction and the reported timestamp is qualified with the same one, so
    // the instant is the one the caller asked about.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getCDR"))
        .and(query_param("timezone", "5.50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "cdr": [{ "date": "2026-09-17 00:44:35" }],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetCDRParams {
        date_from: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 9, 16),
        date_to: voip_ms::chrono::NaiveDate::from_ymd_opt(2026, 9, 17),
        timezone: Some(voip_ms::chrono_tz::Asia::Kolkata),
        ..Default::default()
    };
    let date = client.get_cdr(&params).await.unwrap().cdr[0]
        .date
        .as_ref()
        .and_then(voip_ms::Reported::get)
        .unwrap();
    assert_eq!(
        date,
        voip_ms::chrono::DateTime::parse_from_rfc3339("2026-09-17T00:44:35+05:30").unwrap()
    );
    assert_eq!(date.to_utc().to_string(), "2026-09-16 19:14:35 UTC");
}

#[tokio::test]
async fn record_listing_timezone_without_start_date_errors() {
    // A zone with no start date has no instant to resolve DST at; the call
    // fails before any request is sent.
    use voip_ms::{GetSMSParams, TimezoneOffsetError};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(0)
        .mount(&server)
        .await;

    let params = GetSMSParams {
        timezone: Some(voip_ms::chrono_tz::America::New_York),
        ..Default::default()
    };
    let err = client.get_sms_raw(&params).await.unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidParams(ParamsError::Timezone(TimezoneOffsetError::MissingStartDate))
    ));
}

#[tokio::test]
async fn named_zone_timezone_serializes_as_iana_name() {
    // The voicemail / getTimezones `timezone` is a named zone, not an offset:
    // the wire carries the IANA name verbatim. The response side is tolerant --
    // voip.ms's catalog still lists legacy names (`Asia/Beijing`) the IANA
    // database has dropped, which must survive verbatim instead of failing
    // the whole response.
    use voip_ms::{GetTimezonesParams, TimezoneName};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getTimezones"))
        .and(query_param("timezone", "America/New_York"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "timezones": [
                { "value": "America/New_York", "description": "America/New York" },
                { "value": "Asia/Beijing", "description": "Asia/Beijing" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let params = GetTimezonesParams {
        timezone: Some(voip_ms::chrono_tz::America::New_York),
    };
    let envelope = client.get_timezones(&params).await.unwrap();
    assert_eq!(
        envelope.timezones[0].value,
        Some(TimezoneName::Known(voip_ms::chrono_tz::America::New_York))
    );
    assert_eq!(
        envelope.timezones[1].value,
        Some(TimezoneName::Unrecognized("Asia/Beijing".into()))
    );
}

#[tokio::test]
async fn a_base64_file_parameter_travels_as_a_multipart_post() {
    // 60 kB of base64 is more than seven times the 8190-byte request line
    // voip.ms accepts, so the call cannot be a GET. Every field moves into the
    // body -- including the credentials, which no longer ride in the URL.
    use voip_ms::SetRecordingParams;

    let (server, client) = fixture().await;
    let payload = "QUJD".repeat(15_000);

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .and(header_regex(
            "content-type",
            "^multipart/form-data; boundary=",
        ))
        .and(query_param_is_missing("api_password"))
        .and(query_param_is_missing("method"))
        // Every field matched as a whole part. A bare substring would hold for
        // a body that put the value under a different name, which is the
        // regression these are here to catch.
        .and(body_string_contains(
            "name=\"method\"\r\n\r\nsetRecording\r\n",
        ))
        .and(body_string_contains(
            "name=\"api_username\"\r\n\r\nuser@example.com\r\n",
        ))
        .and(body_string_contains(
            "name=\"api_password\"\r\n\r\nsecret\r\n",
        ))
        .and(body_string_contains(format!(
            "name=\"file\"\r\n\r\n{payload}\r\n"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "status": "success", "recording": 295001 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let resp = client
        .set_recording(&SetRecordingParams {
            name: Some("greeting".into()),
            file: Some(payload),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(resp.recording, Some(295001));
}

#[tokio::test]
async fn every_file_carrying_method_posts_and_the_rest_do_not() {
    // The four methods with a base64 file parameter, and one without to pin the
    // contrast: transport is decided per method, not per payload size.
    use voip_ms::{AddLNPFileParams, SendFAXMessageParams, SendMMSParams, SetRecordingParams};

    let (server, client) = fixture().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(4)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .set_recording(&SetRecordingParams {
            file: Some("QUJD".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .send_fax_message(&SendFAXMessageParams {
            file: Some("QUJD".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .send_mms(&SendMMSParams {
            media2: Some("QUJD".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .add_lnp_file(&AddLNPFileParams {
            portid: Some(1),
            file: Some("QUJD".into()),
        })
        .await
        .unwrap();
    client
        .get_balance(&GetBalanceParams::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn multipart_fields_carry_the_same_wire_forms_as_the_query_string() {
    // The transport moves where a value rides, not how it is encoded: a `1`/`0`
    // flag is still `1`, a `None` is still absent, and a `+` inside a base64
    // payload reaches the part unescaped, where a query string would percent-
    // encode it.
    use voip_ms::SendFAXMessageParams;

    let (server, client) = fixture().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .and(body_string_contains("name=\"send_email_enabled\""))
        .and(body_string_contains("name=\"file\"\r\n\r\nQQ+/word==\r\n"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client
        .send_fax_message_raw(&SendFAXMessageParams {
            to_number: Some("5551234567".into()),
            send_email_enabled: Some(true),
            file: Some("QQ+/word==".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(resp["status"], "success");
    let body = &server.received_requests().await.unwrap()[0].body;
    let body = String::from_utf8_lossy(body);
    assert!(
        body.contains("name=\"send_email_enabled\"\r\n\r\n1\r\n"),
        "a 1/0 flag keeps its wire form as a form field: {body}"
    );
    assert!(
        !body.contains("from_name"),
        "a None field is omitted, as it is from the query string: {body}"
    );
}

#[tokio::test]
async fn a_base64_response_payload_survives_its_escaped_slashes() {
    // voip.ms escapes `/` as `\/` inside a JSON string, which base64 is full
    // of. Nothing in this crate unescapes it by hand -- serde does.
    use voip_ms::GetRecordingFileParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getRecordingFile"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"status":"success","recordings":[{"value":295001,"data":"UklGRi\/\/AABXQVZF"}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let resp = client
        .get_recording_file(&GetRecordingFileParams {
            recording: Some(295001),
        })
        .await
        .unwrap();

    assert_eq!(resp.recordings[0].data.as_deref(), Some("UklGRi//AABXQVZF"));
}

#[cfg(feature = "unchecked-raw")]
#[tokio::test]
async fn the_unchecked_diagnostic_hatch_has_both_transports() {
    // The pair exists so diagnosing a file method reaches it over the transport
    // that method needs. Both surface a non-success envelope verbatim rather
    // than as `Error::Api`, and each must use its own transport to do it.
    let (server, client) = fixture().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .and(body_string_contains("name=\"file\"\r\n\r\nQUJD\r\n"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"status": "invalid_credentials"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"status": "invalid_credentials"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let posted = client
        .call_multipart_raw_unchecked("setRecording", &json!({ "file": "QUJD" }))
        .await
        .expect("an error status is returned in the body, not as Err");
    assert_eq!(posted["status"], "invalid_credentials");

    let got = client
        .call_raw_unchecked("getBalance", &GetBalanceParams::default())
        .await
        .unwrap();
    assert_eq!(got["status"], "invalid_credentials");
}

#[test]
fn requires_multipart_does_not_claim_a_name_it_has_never_seen() {
    // A method this crate has not been regenerated for gets the default
    // transport, which a caller reaching for a brand-new wire name has to know:
    // `call_raw_by_name` can only answer for the 222 names in the table, so an
    // ungenerated upload method needs `call_multipart_raw` chosen by hand.
    //
    // Which names the predicate *does* match is asserted as a set over the whole
    // generated surface, in livetest's `completeness` suite -- the only place the
    // list of all 222 methods exists. Repeating four of them here would be a
    // second thing to update and no case that check would miss.
    assert!(!voip_ms::requires_multipart("someBrandNewMethod"));
}

#[tokio::test]
async fn a_call_by_name_takes_the_transport_the_method_requires() {
    // The choice a caller dispatching by wire name would otherwise re-derive.
    // Both arms are exercised here because no generated method reaches this
    // path, so nothing else would tell a one-armed dispatcher from a correct
    // one.
    let (server, client) = fixture().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .and(header_regex(
            "content-type",
            "^multipart/form-data; boundary=",
        ))
        .and(body_string_contains(
            "name=\"method\"\r\n\r\nsetRecording\r\n",
        ))
        .and(body_string_contains("name=\"file\"\r\n\r\nQUJD\r\n"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "status": "success", "recording": 295001 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "status": "success", "balance": {} })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let posted = client
        .call_raw_by_name("setRecording", &json!({ "file": "QUJD" }))
        .await
        .unwrap();
    assert_eq!(posted["recording"], 295001);

    let got = client
        .call_raw_by_name("getBalance", &GetBalanceParams::default())
        .await
        .unwrap();
    assert_eq!(got["status"], "success");
}

#[cfg(feature = "unchecked-raw")]
#[tokio::test]
async fn an_unchecked_call_by_name_takes_the_same_transport() {
    // A diagnostic dump has to go out the way the call did, or it describes a
    // request that was never made.
    let (server, client) = fixture().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/rest.php"))
        .and(body_string_contains("name=\"file\"\r\n\r\nQUJD\r\n"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "status": "invalid_credentials" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getBalance"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "status": "invalid_credentials" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let posted = client
        .call_raw_unchecked_by_name("setRecording", &json!({ "file": "QUJD" }))
        .await
        .expect("an error status is returned in the body, not as Err");
    assert_eq!(posted["status"], "invalid_credentials");

    let got = client
        .call_raw_unchecked_by_name("getBalance", &GetBalanceParams::default())
        .await
        .expect("an error status is returned in the body, not as Err");
    assert_eq!(got["status"], "invalid_credentials");
}

#[tokio::test]
async fn parameters_with_no_field_rendering_are_refused_before_anything_is_sent() {
    // A nested value has no form field and no query parameter, so the call
    // fails as invalid parameters rather than as a transport error -- and on
    // both transports, which render their fields the same way.
    let (server, client) = fixture().await;

    Mock::given(path("/api/v1/rest.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(0)
        .mount(&server)
        .await;

    let nested = json!({ "routing": { "kind": "sys", "value": "hangup" } });
    for error in [
        client.call_raw("setDISA", &nested).await.unwrap_err(),
        client
            .call_multipart_raw("setRecording", &nested)
            .await
            .unwrap_err(),
    ] {
        let voip_ms::Error::InvalidParams(voip_ms::ParamsError::Unencodable(message)) = error
        else {
            panic!("expected InvalidParams, got {error}");
        };

        assert!(message.contains("`routing`"), "{message}");
    }
}

/// A method the WSDL declares no parameters for takes no argument, so a call
/// site does not name an empty struct to say nothing.
#[tokio::test]
async fn a_parameterless_method_takes_no_argument() {
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getIP"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "status": "success", "ip": "203.0.113.7" })),
        )
        .expect(2)
        .mount(&server)
        .await;

    let typed = client.get_ip().await.unwrap();
    assert_eq!(typed.ip.as_deref(), Some("203.0.113.7"));

    let raw = client.get_ip_raw().await.unwrap();
    assert_eq!(raw["ip"], "203.0.113.7");
}

/// A consumer holding several clients (a reseller plus its sub-accounts)
/// labels a log line from the client itself rather than carrying the username
/// beside it.
#[test]
fn the_client_names_the_account_it_speaks_for() {
    let client = Client::new("user@example.com", "secret");
    assert_eq!(client.api_username(), "user@example.com");
}

/// The read and write sides of a record now share a type, so a caller who
/// listed one and then updated it passes the value straight through. Each of
/// these param fields was `Option<String>` while its response counterpart was
/// already a number, a decimal, or an enum.
#[tokio::test]
async fn realigned_params_serialize_to_the_wire_form_the_response_reports() {
    use voip_ms::{EstimatedHoldTimeAnnounce, SetForwardingParams, SetQueueParams, WaitTime};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setQueue"))
        // A record id the response reports as a number.
        .and(query_param("queue", "32208"))
        // A count, or the documented word for no cap.
        .and(query_param("maximum_callers", "unlimited"))
        .and(query_param("priority_weight", "1"))
        // Three-valued (`yes`/`no`/`once`), not the boolean its `yes` sample
        // made it look like.
        .and(query_param("report_hold_time_agent", "once"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .set_queue_raw(&SetQueueParams {
            queue: Some(32208),
            maximum_callers: Some(WaitTime::Unlimited),
            priority_weight: Some(1),
            report_hold_time_agent: Some(EstimatedHoldTimeAnnounce::Once),
            ..Default::default()
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "setForwarding"))
        .and(query_param("forwarding", "19183"))
        // Documented "0 to 10 in increments of 0.5", so a half second has to
        // survive the round trip.
        .and(query_param("pause", "1.5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .set_forwarding_raw(&SetForwardingParams {
            forwarding: Some(19183),
            pause: Some(Decimal::from_str_exact("1.5").unwrap()),
            ..Default::default()
        })
        .await
        .unwrap();
}

/// An identifier an all-digit doc sample made the extractor read as a number.
/// A US ZIP with a leading zero and a voicemail PIN with one both survive now;
/// as `u64` each lost its padding.
#[tokio::test]
async fn identifier_fields_keep_their_leading_zeros() {
    use voip_ms::{GetClientsParams, GetVoicemailsParams};

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getClients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "clients": [{ "client": "561115", "zip": "02134" }],
        })))
        .mount(&server)
        .await;

    let clients = client
        .get_clients(&GetClientsParams::default())
        .await
        .unwrap();
    assert_eq!(clients.clients[0].client, Some(561115));
    assert_eq!(clients.clients[0].zip.as_deref(), Some("02134"));

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getVoicemails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "voicemails": [{ "mailbox": "1001", "password": "0123" }],
        })))
        .mount(&server)
        .await;

    let boxes = client
        .get_voicemails(&GetVoicemailsParams::default())
        .await
        .unwrap();
    assert_eq!(boxes.voicemails[0].mailbox, Some(1001));
    assert_eq!(boxes.voicemails[0].password.as_deref(), Some("0123"));
}

/// `getClients` and `getDIDsInfo` document their `client` parameter as an id
/// *or* an e-mail address / sub-account name, so those two keep a `String`
/// where every other `client` is the numeric id the responses report.
#[tokio::test]
async fn the_polymorphic_client_filters_still_take_a_string() {
    use voip_ms::GetClientsParams;

    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getClients"))
        .and(query_param("client", "john@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "status": "success" })))
        .expect(1)
        .mount(&server)
        .await;

    client
        .get_clients_raw(&GetClientsParams {
            client: Some("john@example.com".into()),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn an_unreadable_date_costs_its_own_field_and_nothing_else() {
    use voip_ms::{GetDIDsInfoParams, Reported, chrono::NaiveDate};

    // A response is one value built from one envelope, so a date VoIP.ms
    // spells in a form this crate does not model reads as `Unreadable` with
    // the text intact: the records beside it survive, and the value is still
    // there to salvage.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getDIDsInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "dids": [
                { "did": "5551234567", "next_billing": "2026-10-08" },
                { "did": "5557654321", "next_billing": "08/10/2026" },
                { "did": "5559999999", "next_billing": "0000-00-00" },
            ],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = client
        .get_dids_info(&GetDIDsInfoParams::default())
        .await
        .unwrap();

    assert_eq!(envelope.dids.len(), 3);
    assert_eq!(
        envelope.dids[0].next_billing,
        Some(Reported::Parsed(
            NaiveDate::from_ymd_opt(2026, 10, 8).unwrap()
        ))
    );
    assert_eq!(
        envelope.dids[1].next_billing,
        Some(Reported::Unreadable("08/10/2026".to_string()))
    );
    // The placeholder is still absence, not an unreadable value.
    assert_eq!(envelope.dids[2].next_billing, None);
    // And the row that could not be read still carries the rest of itself.
    assert_eq!(envelope.dids[1].did.as_deref(), Some("5557654321"));
}

#[tokio::test]
async fn a_transaction_history_span_does_not_lose_the_response() {
    use voip_ms::{GetTransactionHistoryParams, TransactionDate, chrono::NaiveDate};

    // The row that totals a usage-metered charge over the requested window
    // reports that window in place of a timestamp: it reads as a `Period`, and
    // every row beside it still deserializes.
    let (server, client) = fixture().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/rest.php"))
        .and(query_param("method", "getTransactionHistory"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "transactions": [
                {
                    "date": "2026-09-08 01:19:49",
                    "uniqueid": "61971626x8ae57cb9",
                    "type": "DID6474785907",
                    "description": "DID Monthly Fee: 6474785907",
                    "ammount": "-0.85",
                },
                {
                    "date": "2026-08-01 to 2026-08-31",
                    "uniqueid": "n/a",
                    "type": "CNAM Queries",
                    "description": "CNAM Queries",
                    "ammount": "-0.1760",
                },
                { "date": "", "uniqueid": "3" },
                { "date": "2026-08-14", "uniqueid": "4" },
            ],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let envelope = client
        .get_transaction_history(&GetTransactionHistoryParams {
            date_from: NaiveDate::from_ymd_opt(2026, 8, 1),
            date_to: NaiveDate::from_ymd_opt(2026, 9, 21),
        })
        .await
        .unwrap();

    assert_eq!(envelope.transactions.len(), 4);
    assert_eq!(
        envelope.transactions[0].date,
        Some(TransactionDate::At(
            NaiveDate::from_ymd_opt(2026, 9, 8)
                .unwrap()
                .and_hms_opt(1, 19, 49)
                .unwrap()
        ))
    );
    // The migration path the changelog gives: the previous wall clock, and
    // `None` where the row names a window rather than a point in time.
    assert_eq!(
        envelope.transactions[0]
            .date
            .as_ref()
            .and_then(TransactionDate::at),
        Some(
            NaiveDate::from_ymd_opt(2026, 9, 8)
                .unwrap()
                .and_hms_opt(1, 19, 49)
                .unwrap()
        )
    );
    assert_eq!(
        envelope.transactions[1]
            .date
            .as_ref()
            .and_then(TransactionDate::at),
        None
    );
    assert_eq!(
        envelope.transactions[1].date,
        Some(TransactionDate::Period {
            from: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            to: NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
        })
    );
    assert_eq!(envelope.transactions[2].date, None);
    // A date with no time of day stays a date rather than gaining a midnight.
    assert_eq!(
        envelope.transactions[3].date,
        Some(TransactionDate::On(
            NaiveDate::from_ymd_opt(2026, 8, 14).unwrap()
        ))
    );
    // The synthesized row names no transaction and is metered to four decimal
    // places; both survive beside the range.
    assert_eq!(envelope.transactions[1].uniqueid.as_deref(), Some("n/a"));
    assert_eq!(
        envelope.transactions[1].ammount,
        Some(Decimal::from_str_exact("-0.1760").unwrap())
    );
}
