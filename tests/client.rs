use rust_decimal::Decimal;
use serde_json::{Value, json};
use voip_ms::{
    ApiStatus, Client, Error, GetBalanceParams, GetCDRParams, GetConferenceParams,
    GetSubAccountsParams, GetSubAccountsResponse, MaxMembers,
};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a `Client` pointed at a mock server's REST endpoint.
async fn fixture() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let base_url = format!("{}/api/v1/rest.php", server.uri()).parse().unwrap();
    let client = Client::builder("user@example.com", "secret")
        .base_url(base_url)
        .build()
        .unwrap();
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
    assert!(ApiStatus::NoSMS.is_empty());
    assert!(ApiStatus::NoCDR.is_empty());
    assert!(ApiStatus::NoMessages.is_empty());
    assert!(!ApiStatus::NoProvision.is_empty());
    assert!(!ApiStatus::NoBase64file.is_empty());
    assert!(!ApiStatus::NoCallstatus.is_empty());
    assert!(!ApiStatus::InvalidCredentials.is_empty());
    assert!(!ApiStatus::Unknown("whatever".to_string()).is_empty());
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
            r#type: Some(SearchType::Starts),
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
            r#type: Some(MessageType::Received),
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
    assert_eq!(msgs[0].r#type, Some(MessageType::Received));
    assert_eq!(msgs[1].r#type, Some(MessageType::Sent));
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
    assert_eq!(envelope.status.as_deref(), Some("no_sms"));
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
        messages[0].date,
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
