use protocol::{connection, connection_v2};
use serde_json::{json, Value};

fn session() -> Value {
    json!({
        "connect_session_ref":"connect-session:fixture",
        "integration_ref":"fixture",
        "state":"pending",
        "expires_at_unix_ms":1234
    })
}

fn envelope(identity: &str, method: &str, value: Value) -> Value {
    json!({"protocol":identity,"request_id":"request-1","status":"ok",
        "response":{"result":method,"value":value}})
}

fn valid(frame: &Value) -> bool {
    let bytes = serde_json::to_vec(frame).unwrap();
    if frame["protocol"] == connection::CONTRACT {
        serde_json::from_slice::<connection::ResponseEnvelope>(&bytes)
            .is_ok_and(|response| response.validate().is_ok())
    } else {
        connection_v2::decode_response(&bytes).is_ok()
    }
}

fn schema() -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&protocol::connection_v2_schema::connection_v2_schema())
        .unwrap()
}

#[test]
fn pending_status_round_trips_without_reissuing_a_completion_capability() {
    let pending = connection::ConnectSessionStatus {
        connect_session_ref: "connect-session:fixture".into(),
        integration_ref: "fixture".into(),
        state: connection::ConnectSessionState::Pending,
        expires_at_unix_ms: 1234,
        completion_endpoint: None,
        browser_completion_url: None,
        connection_ref: None,
    };
    let response = connection::ResponseEnvelope::success(
        "request-1",
        connection::ConnectionResult::ConnectSessionStatus(pending.clone()),
    );
    let value = serde_json::to_value(response).unwrap();
    assert!(value["response"]["value"]
        .get("completion_endpoint")
        .is_none());
    assert!(value["response"]["value"]
        .get("browser_completion_url")
        .is_none());
    assert!(valid(&value));
    assert_eq!(value["response"]["value"], session());

    let validator = schema();
    for identity in [connection::CONTRACT, connection_v2::CONTRACT] {
        for (method, expected) in [
            ("connect_session_status", true),
            ("connect_session_create", false),
        ] {
            let frame = envelope(identity, method, serde_json::to_value(&pending).unwrap());
            assert_eq!(valid(&frame), expected, "{identity}/{method}");
            if identity == connection_v2::CONTRACT {
                assert_eq!(validator.is_valid(&frame), expected, "schema/{method}");
            }
        }
    }
}

#[test]
fn creation_preserves_socket_browser_and_combined_completion_routes() {
    let validator = schema();
    for routes in [
        json!({"completion_endpoint":"unix:/state/connect.sock"}),
        json!({"browser_completion_url":"https://connector.example/connect-sessions/fixture#token=fixture-capability"}),
        json!({"completion_endpoint":"unix:/state/connect.sock","browser_completion_url":"http://127.0.0.1:43123/#token=fixture-capability"}),
    ] {
        let mut pending = session();
        pending
            .as_object_mut()
            .unwrap()
            .extend(routes.as_object().unwrap().clone());
        for identity in [connection::CONTRACT, connection_v2::CONTRACT] {
            for method in ["connect_session_create", "connect_session_status"] {
                let frame = envelope(identity, method, pending.clone());
                assert!(valid(&frame), "{identity}/{method}");
                if identity == connection_v2::CONTRACT {
                    assert!(validator.is_valid(&frame));
                }
            }
        }
    }
}

#[test]
fn optional_status_locators_still_reject_malformed_or_terminal_capabilities() {
    for identity in [connection::CONTRACT, connection_v2::CONTRACT] {
        for malformed in [
            json!({"completion_endpoint":""}),
            json!({"completion_endpoint":"unix:/state/bad\npath"}),
            json!({"browser_completion_url":"https://connector.example/connect-sessions/fixture#token="}),
            json!({"browser_completion_url":"https://user@connector.example/connect-sessions/fixture#token=fixture"}),
            json!({"connection_ref":"connection:premature"}),
        ] {
            let mut pending = session();
            pending
                .as_object_mut()
                .unwrap()
                .extend(malformed.as_object().unwrap().clone());
            assert!(!valid(&envelope(
                identity,
                "connect_session_status",
                pending
            )));
        }
        for state in ["completed", "expired", "failed"] {
            let mut terminal = session();
            terminal["state"] = json!(state);
            if state == "completed" {
                terminal["connection_ref"] = json!("connection:fixture");
            }
            assert!(valid(&envelope(
                identity,
                "connect_session_status",
                terminal.clone()
            )));
            for field in ["completion_endpoint", "browser_completion_url"] {
                let mut invalid = terminal.clone();
                invalid[field] = json!(if field == "completion_endpoint" {
                    "unix:/state/connect.sock"
                } else {
                    "http://127.0.0.1:43123/#token=fixture"
                });
                assert!(!valid(&envelope(
                    identity,
                    "connect_session_status",
                    invalid
                )));
            }
        }
    }
}

#[test]
fn pending_bound_status_may_omit_a_capability_but_start_must_supply_one() {
    let validator = schema();
    let mut bound = json!({
        "connect_session_ref":"connect-session:fixture","operation_ref":"fixture.read",
        "connection_ref":"connection:fixture","integration_ref":"fixture","auth_profile":"fixture.user",
        "need":"authorize_configured","session_state":"pending","expires_at_unix_ms":1234,
        "resume_state":"pending","session":session()
    });
    for (method, expected) in [("remediation_status", true), ("remediation_start", false)] {
        let frame = envelope(connection_v2::CONTRACT, method, bound.clone());
        assert_eq!(valid(&frame), expected, "{method}");
        assert_eq!(validator.is_valid(&frame), expected, "schema/{method}");
    }
    bound["session"]["browser_completion_url"] =
        json!("http://127.0.0.1:43123/#token=fixture-capability");
    let started = envelope(connection_v2::CONTRACT, "remediation_start", bound);
    assert!(valid(&started));
    assert!(validator.is_valid(&started));
}
