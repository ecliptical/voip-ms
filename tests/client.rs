use rust_decimal::Decimal;
use serde_json::{Value, json};
use voip_ms::{
    ApiStatus, Client, Error, GetBalanceParams, GetSubAccountsParams, GetSubAccountsResponse,
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
        .and(query_param("advanced", "true"))
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
        .as_ref()
        .and_then(|accounts| accounts.first())
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
        .as_ref()
        .and_then(|accounts| accounts.first())
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
    // voip.ms rejects for these parameters.
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
    let account = envelope.accounts.unwrap().into_iter().next().unwrap();
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
    let queue = envelope.queues.unwrap().into_iter().next().unwrap();
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
    // voip.ms returns the SMS `type` as a bare JSON number (1 = received,
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
    let msgs = envelope.sms.unwrap();
    assert_eq!(msgs[0].r#type, Some(MessageType::Received));
    assert_eq!(msgs[1].r#type, Some(MessageType::Sent));
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
    let did = envelope.dids.unwrap().into_iter().next().unwrap();
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
    let queue = envelope.queues.unwrap().into_iter().next().unwrap();
    assert_eq!(queue.retry_timer, Some(Seconds::Value(30)));
    assert_eq!(queue.maximum_wait_time, Some(WaitTime::Unlimited));
}
