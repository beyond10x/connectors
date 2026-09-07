//! Transport-free fixtures using the installed declarations; no receiver/listener is claimed here.

use super::*;
use serde_json::json;

const SECRET: &[u8] = b"not-a-real-secret-webhook-fixture-only";
const NOW: u64 = 1_788_775_200;
const URL: &str = "https://receiver.example.test/inbound?route=fixture";
const SLACK_EVENTS: &[&str] = &["app_mention", "message.channels"];

fn plan(provider: &str, binding: &str) -> WebhookPlan {
    WebhookPlan::from_catalog(
        catalog::provider(catalog::ProviderKey::id(provider)).unwrap(),
        binding,
    )
    .unwrap()
}

fn slack_identity() -> WebhookIdentity<'static> {
    WebhookIdentity::Slack {
        team_id: "T-FIXTURE",
        api_app_id: "A-FIXTURE",
    }
}

fn twilio_identity() -> WebhookIdentity<'static> {
    WebhookIdentity::Twilio {
        account_sid: "account-fixture",
    }
}

fn slack_event() -> Value {
    json!({
        "type":"event_callback", "team_id":"T-FIXTURE", "api_app_id":"A-FIXTURE",
        "event_id":"Ev-fixture", "token":"deprecated-envelope-token-never-a-credential",
        "event":{"type":"app_mention", "user":"U-FIXTURE", "text":"hello", "channel":"C-FIXTURE", "ts":"1.2"}
    })
}

// Fixture signing states the two vendor formulas directly, independent of the template renderer.
// Published external vectors below additionally pin both real declarations and cryptography.
fn slack_signature(body: &[u8], timestamp: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET).unwrap();
    mac.update(format!("v0:{timestamp}:").as_bytes());
    mac.update(body);
    format!("v0={}", hex::encode(mac.finalize().into_bytes()))
}

fn receive_slack(body: &[u8], allowed: &[&str]) -> Result<WebhookOutcome, WebhookError> {
    let plan = plan("slack", "events-api");
    let timestamp = NOW.to_string();
    let signature = slack_signature(body, &timestamp);
    let headers = [
        ("content-type", "application/json; charset=utf-8"),
        ("x-slack-signature", signature.as_str()),
        ("x-slack-request-timestamp", timestamp.as_str()),
    ];
    plan.receive(
        &WebhookRequest {
            headers: &headers,
            body,
            public_url: URL,
            now_unix_seconds: NOW,
        },
        WebhookSecret {
            name: plan.secret_name(),
            value: SECRET,
        },
        slack_identity(),
        allowed,
    )
}

fn twilio_signature(url: &str, fields: &[(&str, &str)]) -> String {
    let sorted: BTreeMap<_, _> = fields.iter().copied().collect();
    let mut mac = Hmac::<Sha1>::new_from_slice(SECRET).unwrap();
    mac.update(url.as_bytes());
    for (key, value) in sorted {
        mac.update(key.as_bytes());
        mac.update(value.as_bytes());
    }
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

fn receive_twilio(
    binding: &str,
    body: &[u8],
    fields: &[(&str, &str)],
) -> Result<WebhookOutcome, WebhookError> {
    let plan = plan("twilio", binding);
    let signature = twilio_signature(URL, fields);
    let headers = [
        ("Content-Type", "application/x-www-form-urlencoded"),
        ("X-Twilio-Signature", signature.as_str()),
    ];
    let allowed = plan
        .events
        .iter()
        .map(|event| event.name)
        .collect::<Vec<_>>();
    plan.receive(
        &WebhookRequest {
            headers: &headers,
            body,
            public_url: URL,
            now_unix_seconds: NOW,
        },
        WebhookSecret {
            name: plan.secret_name(),
            value: SECRET,
        },
        twilio_identity(),
        &allowed,
    )
}

fn event(outcome: WebhookOutcome) -> WebhookEvent {
    let WebhookOutcome::Event(event) = outcome else {
        panic!("expected an admitted event")
    };
    event
}

#[test]
fn catalog_rules_verify_the_published_vendor_vectors() {
    // Exact vectors already pinned by connector-spec/tests/main/verification_conformance.rs.
    // Slack: https://docs.slack.dev/authentication/verifying-requests-from-slack/
    // Twilio: https://www.twilio.com/docs/usage/security#explore-the-algorithm-yourself
    let slack_body = b"token=xyzz0WbapA4vBCDEFasx0q6G&team_id=T1DC2JH3J&team_domain=testteamnow&channel_id=G8PSS9T3V&channel_name=foobar&user_id=U2CERLKJA&user_name=roadrunner&command=%2Fwebhook-collect&text=&response_url=https%3A%2F%2Fhooks.slack.com%2Fcommands%2FT1DC2JH3J%2F397700885554%2F96rGlfmibIGlgcZRskXaIFfN&trigger_id=398738663015.47445629121.803a0bc887a14d10d2c447fce8b6703c";
    let slack = plan("slack", "events-api");
    let headers = [
        (
            "X-Slack-Signature",
            "v0=a2114d57b48eac39b9ad189dd8316235a7b4a8d21a10bd27519666489c69b503",
        ),
        ("X-Slack-Request-Timestamp", "1531420618"),
    ];
    slack
        .verify(
            &WebhookRequest {
                headers: &headers,
                body: slack_body,
                public_url: URL,
                now_unix_seconds: 1_531_420_618,
            },
            WebhookSecret {
                name: slack.secret_name(),
                value: b"8f742231b10e8888abcd99yyyzzz85a5",
            },
        )
        .unwrap();

    let twilio = plan("twilio", "message-status-callback");
    let body = b"To=%2B18005551212&From=%2B14158675310&Digits=1234&Caller=%2B14158675310&CallSid=CA1234567890ABCDE";
    let headers = [
        ("content-type", "application/x-www-form-urlencoded"),
        ("X-Twilio-Signature", "L/OH5YylLD5NRKLltdqwSvS0BnU="),
    ];
    twilio
        .verify(
            &WebhookRequest {
                headers: &headers,
                body,
                public_url: "https://example.com/myapp.php?foo=1&bar=2",
                now_unix_seconds: 0,
            },
            WebhookSecret {
                name: twilio.secret_name(),
                value: b"12345",
            },
        )
        .unwrap();
}

#[test]
fn signed_slack_event_preserves_native_schema_and_optional_reply_fields() {
    let mut envelope = slack_event();
    let first =
        event(receive_slack(&serde_json::to_vec(&envelope).unwrap(), SLACK_EVENTS).unwrap());
    assert_eq!(first.event_type, "app_mention");
    assert_eq!(first.delivery_id.as_deref(), Some("Ev-fixture"));
    assert_eq!(first.payload, envelope["event"]);
    assert_eq!(first.payload["channel"], "C-FIXTURE");
    assert_eq!(first.payload["ts"], "1.2");
    assert!(first.payload.get("token").is_none());
    assert_eq!(first.reply_fields.len(), 3);
    assert!(!first.reply_fields.contains_key("thread"));
    envelope["event"]["thread_ts"] = json!("1.0");
    let threaded =
        event(receive_slack(&serde_json::to_vec(&envelope).unwrap(), SLACK_EVENTS).unwrap());
    assert_eq!(threaded.reply_fields["thread"], "1.0");
    assert_eq!(threaded.reply_fields["conversation"], "1.0");
    assert_eq!(threaded.payload["thread_ts"], "1.0");
}

#[test]
fn slack_challenge_is_authenticated_control_with_no_event_admission_needed() {
    let body = br#"{"type":"url_verification","challenge":"fixture-challenge","token":"deprecated-token"}"#;
    assert_eq!(
        receive_slack(body, &[]),
        Ok(WebhookOutcome::Challenge("fixture-challenge".into()))
    );
    let plan = plan("slack", "events-api");
    let headers = [
        ("content-type", "application/json"),
        ("x-slack-signature", "v0=00"),
        ("x-slack-request-timestamp", "1788775200"),
    ];
    assert_eq!(
        plan.receive(
            &WebhookRequest {
                headers: &headers,
                body,
                public_url: URL,
                now_unix_seconds: NOW
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET
            },
            slack_identity(),
            &[]
        ),
        Err(WebhookError::Signature)
    );
    for body in [
        br#"{"type":"url_verification","challenge":""}"#.as_slice(),
        br#"{"type":"url_verification","challenge":2}"#.as_slice(),
    ] {
        assert_eq!(receive_slack(body, &[]), Err(WebhookError::Payload));
    }
}

#[test]
fn slack_narrowing_loop_guards_and_closed_admission_precede_delivery() {
    let mut envelope = slack_event();
    envelope["event"]["type"] = json!("message");
    envelope["event"]["channel_type"] = json!("channel");
    let raw = serde_json::to_vec(&envelope).unwrap();
    assert_eq!(
        event(receive_slack(&raw, SLACK_EVENTS).unwrap()).event_type,
        "message.channels"
    );
    assert_eq!(
        receive_slack(&raw, &["app_mention"]),
        Ok(WebhookOutcome::Ignored)
    );
    for (field, value) in [
        ("channel_type", "im"),
        ("bot_id", "B-FIXTURE"),
        ("subtype", "message_changed"),
    ] {
        let mut excluded = envelope.clone();
        excluded["event"][field] = json!(value);
        assert_eq!(
            receive_slack(&serde_json::to_vec(&excluded).unwrap(), SLACK_EVENTS),
            Ok(WebhookOutcome::Ignored)
        );
    }
    assert_eq!(receive_slack(&raw, &["*"]), Err(WebhookError::Context));
    assert_eq!(
        receive_slack(&raw, &["app_mention", "app_mention"]),
        Err(WebhookError::Context)
    );
}

#[test]
fn verified_slack_event_requires_target_attribution_delivery_id_and_native_schema() {
    for field in ["team_id", "api_app_id"] {
        let mut wrong = slack_event();
        wrong[field] = json!("different-identity");
        assert_eq!(
            receive_slack(&serde_json::to_vec(&wrong).unwrap(), SLACK_EVENTS),
            Err(WebhookError::Attribution)
        );
    }
    for field in ["event_id", "team_id", "api_app_id"] {
        let mut missing = slack_event();
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(
            receive_slack(&serde_json::to_vec(&missing).unwrap(), SLACK_EVENTS),
            Err(WebhookError::Payload)
        );
    }
    let mut missing = slack_event();
    missing["event"].as_object_mut().unwrap().remove("text");
    assert_eq!(
        receive_slack(&serde_json::to_vec(&missing).unwrap(), SLACK_EVENTS),
        Err(WebhookError::Payload)
    );
    missing = slack_event();
    missing["event"]["channel"] = json!(5);
    assert_eq!(
        receive_slack(&serde_json::to_vec(&missing).unwrap(), SLACK_EVENTS),
        Err(WebhookError::Payload)
    );
}

#[test]
fn signature_checks_raw_json_before_parsing_and_does_not_accept_reserialized_bytes() {
    let plan = plan("slack", "events-api");
    let original = serde_json::to_vec(&slack_event()).unwrap();
    let pretty = serde_json::to_vec_pretty(&slack_event()).unwrap();
    let timestamp = NOW.to_string();
    let signature = slack_signature(&original, &timestamp);
    let headers = [
        ("content-type", "application/json"),
        ("x-slack-signature", signature.as_str()),
        ("x-slack-request-timestamp", timestamp.as_str()),
    ];
    for body in [pretty.as_slice(), b"not json"] {
        assert_eq!(
            plan.receive(
                &WebhookRequest {
                    headers: &headers,
                    body,
                    public_url: URL,
                    now_unix_seconds: NOW
                },
                WebhookSecret {
                    name: plan.secret_name(),
                    value: SECRET
                },
                slack_identity(),
                SLACK_EVENTS
            ),
            Err(WebhookError::Signature)
        );
    }
    assert_eq!(
        receive_slack(b"not json", SLACK_EVENTS),
        Err(WebhookError::Payload)
    );
}

#[test]
fn signature_headers_are_case_insensitive_but_duplicates_fail_closed() {
    let plan = plan("slack", "events-api");
    let body = serde_json::to_vec(&slack_event()).unwrap();
    let timestamp = NOW.to_string();
    let signature = slack_signature(&body, &timestamp);
    let base = [
        ("CONTENT-TYPE", "application/json"),
        ("X-SLACK-SIGNATURE", signature.as_str()),
        ("X-SLACK-REQUEST-TIMESTAMP", timestamp.as_str()),
    ];
    let run = |headers: &[(&str, &str)]| {
        plan.receive(
            &WebhookRequest {
                headers,
                body: &body,
                public_url: URL,
                now_unix_seconds: NOW,
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET,
            },
            slack_identity(),
            SLACK_EVENTS,
        )
    };
    assert!(matches!(run(&base), Ok(WebhookOutcome::Event(_))));
    for duplicate in [
        ("content-type", "application/json"),
        ("x-slack-signature", signature.as_str()),
        ("x-slack-request-timestamp", timestamp.as_str()),
    ] {
        let mut headers = base.to_vec();
        headers.push(duplicate);
        assert_eq!(run(&headers), Err(WebhookError::Header));
    }
    assert_eq!(run(&base[..2]), Err(WebhookError::Header));
}

#[test]
fn declared_timestamp_window_checks_both_directions_without_overflow() {
    let plan = plan("slack", "events-api");
    let body = serde_json::to_vec(&slack_event()).unwrap();
    for (now, timestamp, expected) in [
        (NOW, (NOW - 300).to_string(), None),
        (NOW, (NOW + 300).to_string(), None),
        (NOW, (NOW - 301).to_string(), Some(WebhookError::Timestamp)),
        (NOW, (NOW + 301).to_string(), Some(WebhookError::Timestamp)),
        (NOW, u64::MAX.to_string(), Some(WebhookError::Timestamp)),
        (u64::MAX, "0".into(), Some(WebhookError::Timestamp)),
        (NOW, "-1".into(), Some(WebhookError::Timestamp)),
        (
            NOW,
            "18446744073709551616".into(),
            Some(WebhookError::Timestamp),
        ),
    ] {
        let signature = slack_signature(&body, &timestamp);
        let headers = [
            ("content-type", "application/json"),
            ("x-slack-signature", signature.as_str()),
            ("x-slack-request-timestamp", timestamp.as_str()),
        ];
        let result = plan.receive(
            &WebhookRequest {
                headers: &headers,
                body: &body,
                public_url: URL,
                now_unix_seconds: now,
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET,
            },
            slack_identity(),
            SLACK_EVENTS,
        );
        assert_eq!(result.err(), expected);
    }
}

#[test]
fn both_twilio_status_declarations_preserve_native_and_optional_fields_without_inventing_dedup() {
    for (binding, fields, body, event_type, optional) in [
        ("message-status-callback", vec![("AccountSid", "account-fixture"), ("MessageSid", "message-fixture"), ("MessageStatus", "sent"), ("To", "+1 fixture"), ("Extra", "future-field")],
            "MessageStatus=sent&To=%2B1+fixture&MessageSid=message-fixture&AccountSid=account-fixture&Extra=future-field", "message.status_callback", "error_code"),
        ("call-status-callback", vec![("AccountSid", "account-fixture"), ("CallSid", "call-fixture"), ("CallStatus", "ringing")],
            "CallStatus=ringing&AccountSid=account-fixture&CallSid=call-fixture", "call.status_callback", "duration"),
    ] {
        let received = event(receive_twilio(binding, body.as_bytes(), &fields).unwrap());
        assert_eq!(received.event_type, event_type);
        assert_eq!(received.delivery_id, None);
        assert_eq!(received.payload["AccountSid"], "account-fixture");
        assert!(!received.reply_fields.contains_key(optional));
        if event_type == "message.status_callback" {
            assert_eq!(received.payload["To"], "+1 fixture");
            assert_eq!(received.payload["Extra"], "future-field");
            assert_eq!(received.reply_fields["status"], "sent");
        }
    }
}

#[test]
fn twilio_status_changes_are_distinct_and_optional_failure_fields_are_retained() {
    for status in ["queued", "sent", "delivered", "failed"] {
        let fields = [
            ("AccountSid", "account-fixture"),
            ("MessageSid", "same-message"),
            ("MessageStatus", status),
            ("ErrorCode", "fixture-error"),
        ];
        let body = format!("AccountSid=account-fixture&MessageSid=same-message&MessageStatus={status}&ErrorCode=fixture-error");
        let received =
            event(receive_twilio("message-status-callback", body.as_bytes(), &fields).unwrap());
        assert_eq!(received.delivery_id, None);
        assert_eq!(received.reply_fields["status"], status);
        assert_eq!(received.reply_fields["error_code"], "fixture-error");
    }
}

#[test]
fn twilio_requires_the_exact_configured_url_and_every_form_field_for_verification() {
    let plan = plan("twilio", "call-status-callback");
    let fields = [
        ("AccountSid", "account-fixture"),
        ("CallSid", "call-fixture"),
        ("CallStatus", "completed"),
        ("CallDuration", "31"),
    ];
    let body =
        b"AccountSid=account-fixture&CallSid=call-fixture&CallStatus=completed&CallDuration=31";
    let signature = twilio_signature(URL, &fields);
    let headers = [
        ("content-type", "application/x-www-form-urlencoded"),
        ("x-twilio-signature", signature.as_str()),
    ];
    for url in [
        "http://internal/inbound?route=fixture",
        "https://receiver.example.test/inbound",
        "https://receiver.example.test/inbound?route=other",
    ] {
        assert_eq!(
            plan.receive(
                &WebhookRequest {
                    headers: &headers,
                    body,
                    public_url: url,
                    now_unix_seconds: NOW
                },
                WebhookSecret {
                    name: plan.secret_name(),
                    value: SECRET
                },
                twilio_identity(),
                &["call.status_callback"]
            ),
            Err(WebhookError::Signature)
        );
    }
    let altered = [body.as_slice(), b"&NewField=not-in-signature"].concat();
    assert_eq!(
        plan.receive(
            &WebhookRequest {
                headers: &headers,
                body: &altered,
                public_url: URL,
                now_unix_seconds: NOW
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET
            },
            twilio_identity(),
            &["call.status_callback"]
        ),
        Err(WebhookError::Signature)
    );
    let received = event(
        plan.receive(
            &WebhookRequest {
                headers: &headers,
                body,
                public_url: URL,
                now_unix_seconds: u64::MAX,
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET,
            },
            twilio_identity(),
            &["call.status_callback"],
        )
        .unwrap(),
    );
    assert_eq!(received.reply_fields["duration"], "31"); // No invented Twilio timestamp window.
}

#[test]
fn twilio_checks_account_attribution_and_native_schema_after_authentication() {
    for (account, status, error) in [
        ("other-account", "sent", WebhookError::Attribution),
        ("account-fixture", "invented-status", WebhookError::Payload),
    ] {
        let fields = [
            ("AccountSid", account),
            ("MessageSid", "message-fixture"),
            ("MessageStatus", status),
        ];
        let body =
            format!("AccountSid={account}&MessageSid=message-fixture&MessageStatus={status}");
        assert_eq!(
            receive_twilio("message-status-callback", body.as_bytes(), &fields),
            Err(error)
        );
    }
}

#[test]
fn malformed_or_ambiguous_forms_never_become_authenticated_events() {
    for body in [
        b"a=1&a=2".as_slice(),
        b"a=1&%61=2",
        b"a=%",
        b"a=%GG",
        b"a=%ff",
        b"a=1&",
        b"=value",
        b"missing-equals",
    ] {
        assert_eq!(parse_form(body), Err(WebhookError::Payload));
    }
    assert_eq!(
        parse_form(b"a=&b=%2b+%26%3D").unwrap(),
        BTreeMap::from([("a".into(), "".into()), ("b".into(), "+ &=".into())])
    );
}

#[test]
fn bounds_and_named_credentials_are_enforced() {
    let plan = plan("slack", "events-api");
    let body = vec![b'a'; MAX_WEBHOOK_BODY_BYTES + 1];
    let request = WebhookRequest {
        headers: &[],
        body: &body,
        public_url: URL,
        now_unix_seconds: NOW,
    };
    assert_eq!(
        plan.receive(
            &request,
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET
            },
            slack_identity(),
            SLACK_EVENTS
        ),
        Err(WebhookError::Bounds)
    );
    assert_eq!(
        plan.receive(
            &request,
            WebhookSecret {
                name: "slack.bot_token",
                value: SECRET
            },
            slack_identity(),
            SLACK_EVENTS
        ),
        Err(WebhookError::Context)
    );
    assert_eq!(
        plan.receive(
            &request,
            WebhookSecret {
                name: plan.secret_name(),
                value: b""
            },
            slack_identity(),
            SLACK_EVENTS
        ),
        Err(WebhookError::Context)
    );
    assert_eq!(
        plan.receive(
            &request,
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET
            },
            twilio_identity(),
            SLACK_EVENTS
        ),
        Err(WebhookError::Context)
    );
    let headers = vec![("a", "b"); MAX_HEADERS + 1];
    assert_eq!(
        plan.receive(
            &WebhookRequest {
                headers: &headers,
                body: b"",
                public_url: URL,
                now_unix_seconds: NOW
            },
            WebhookSecret {
                name: plan.secret_name(),
                value: SECRET
            },
            slack_identity(),
            SLACK_EVENTS
        ),
        Err(WebhookError::Bounds)
    );
    let many_fields = (0..=MAX_FORM_FIELDS)
        .map(|index| format!("a{index}=b"))
        .collect::<Vec<_>>()
        .join("&");
    assert_eq!(
        parse_form(many_fields.as_bytes()),
        Err(WebhookError::Bounds)
    );
}

#[test]
fn twilio_rejects_ambiguous_form_fields_and_unsupported_media_at_the_public_boundary() {
    let plan = plan("twilio", "message-status-callback");
    let fields = [
        ("AccountSid", "account-fixture"),
        ("MessageSid", "message-fixture"),
        ("MessageStatus", "sent"),
    ];
    let signature = twilio_signature(URL, &fields);
    let original = b"AccountSid=account-fixture&MessageSid=message-fixture&MessageStatus=sent";
    for (suffix, content_type, expected) in [
        (
            b"&MessageStatus=failed".as_slice(),
            "application/x-www-form-urlencoded",
            WebhookError::Payload,
        ),
        (
            b"&%4dessageStatus=sent",
            "application/x-www-form-urlencoded",
            WebhookError::Payload,
        ),
        (
            b"&NewField=%GG",
            "application/x-www-form-urlencoded",
            WebhookError::Payload,
        ),
        (b"", "application/json", WebhookError::ContentType),
    ] {
        let body = [original.as_slice(), suffix].concat();
        let headers = [
            ("content-type", content_type),
            ("x-twilio-signature", signature.as_str()),
        ];
        assert_eq!(
            plan.receive(
                &WebhookRequest {
                    headers: &headers,
                    body: &body,
                    public_url: URL,
                    now_unix_seconds: NOW
                },
                WebhookSecret {
                    name: plan.secret_name(),
                    value: SECRET
                },
                twilio_identity(),
                &["message.status_callback"]
            ),
            Err(expected)
        );
    }
}

#[test]
fn unsupported_or_incoherent_declarations_never_gain_a_verifier() {
    let provider = catalog::provider(catalog::ProviderKey::id("slack")).unwrap();
    assert!(matches!(
        WebhookPlan::from_catalog(provider, "socket"),
        Err(WebhookError::Declaration)
    ));
    assert!(matches!(
        WebhookPlan::from_catalog(provider, "missing"),
        Err(WebhookError::Declaration)
    ));
    for template in [
        "{timestamp}",
        "{url}",
        "{body",
        "{unknown}{body}",
        "{body}}",
        "literal",
    ] {
        assert!(matches!(
            compile_template(template),
            Err(WebhookError::Declaration)
        ));
    }
    for tolerance in ["0s", "7d", "3601s", "18446744073709551615h", "5", "1m30s"] {
        assert_eq!(parse_tolerance(tolerance), Err(WebhookError::Declaration));
    }
    assert_eq!(parse_tolerance("5m"), Ok(300));
}
