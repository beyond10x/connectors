//! The bounded, redacted upstream reason a provider refusal may carry: what it
//! is read from, what withholds it, and how it is bounded.
use connectors_core::reason;

#[test]
fn a_top_level_message_string_is_the_reason() {
    assert_eq!(
        reason::from_body(br#"{"code":401,"message":"Unauthorized; scope does not match"}"#)
            .as_deref(),
        Some("Unauthorized; scope does not match")
    );
    // `message`, then `error_description`, then `error`, each only as a string.
    assert_eq!(
        reason::from_body(br#"{"error":"invalid_grant","error_description":"Grant expired"}"#)
            .as_deref(),
        Some("Grant expired")
    );
    assert_eq!(
        reason::from_body(br#"{"error":"fixture refusal"}"#).as_deref(),
        Some("fixture refusal")
    );
    assert_eq!(
        reason::from_body(br#"{"message":7,"error":"Missing scope read:confluence-content.all"}"#)
            .as_deref(),
        Some("Missing scope read:confluence-content.all")
    );
    // Whitespace runs, line breaks included, read as one space.
    assert_eq!(
        reason::from_body(br#"{"message":"  Scope\n\tdoes not   match "}"#).as_deref(),
        Some("Scope does not match")
    );
}

#[test]
fn anything_but_a_top_level_string_field_of_a_json_object_is_no_reason() {
    for body in [
        b"Unauthorized; scope does not match".as_slice(),
        b"",
        br#"["message"]"#,
        br#""message""#,
        br#"{"error":{"message":"nested"}}"#,
        br#"{"errors":[{"message":"listed"}]}"#,
        br#"{"detail":"another field"}"#,
        br#"{"message":""}"#,
        br#"{"message":"   "}"#,
        br#"{"message":"a","message":"b"}"#,
    ] {
        assert_eq!(
            reason::from_body(body),
            None,
            "{}",
            String::from_utf8_lossy(body)
        );
    }
}

#[test]
fn a_secret_shaped_value_withholds_the_whole_reason() {
    for text in [
        // Token-like runs: 20+ bytes mixing letters and digits, or any 32+.
        "Token fixtok-AbCdEfGhIjKl0123456789 was revoked",
        "Bad credential ATATT3xFfGF0abcdefghijklmnop",
        "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJl rejected",
        "request 3f2c8a1e-9b7d-4c6a-8e2f-0a1b2c3d4e5f failed",
        "key abcdefghijklmnopqrstuvwxyzabcdefgh refused",
        // Header- and credential-like content, in any case.
        "Authorization header malformed",
        "expected Bearer scheme",
        "BASIC realm required",
        "password expired",
        "client secret mismatch",
        "invalid api_key",
        "x-api-key missing",
        "token=abc rejected",
        "refresh_token unknown",
        "Cookie rejected",
        "-----BEGIN PRIVATE KEY----- found",
        // Control and invisible formatting characters.
        "scope \u{1b}[31mred",
        "scope \u{202e}desrever",
        "zero\u{200b}width",
    ] {
        assert_eq!(reason::admit(text), None, "{text:?}");
        let body = serde_json::to_vec(&serde_json::json!({ "message": text })).unwrap();
        assert_eq!(reason::from_body(&body), None, "{text:?}");
    }
    // Ordinary provider prose and scope names pass.
    for text in [
        "Unauthorized; scope does not match",
        "Missing scope read:confluence-content.all",
        "Request had insufficient authentication scopes.",
        "Project Not Found",
        "Invalid token",
        "https://www.googleapis.com/auth/drive",
    ] {
        assert_eq!(reason::admit(text).as_deref(), Some(text), "{text:?}");
    }
}

#[test]
fn a_reason_longer_than_256_bytes_is_cut_on_a_character_and_word_boundary() {
    assert_eq!(reason::LIMIT, 256);
    // 13 + 7 * 40 bytes; byte 256 falls inside the two-byte `é` of a word.
    let long = format!("Unauthorized {}", "scopé ".repeat(40));
    assert!(!long.is_char_boundary(256));
    let body = serde_json::to_vec(&serde_json::json!({ "message": long })).unwrap();
    let cut = reason::from_body(&body).unwrap();
    assert!(cut.len() <= reason::LIMIT, "{} bytes", cut.len());
    assert_eq!(
        cut,
        format!("Unauthorized {}", "scopé ".repeat(34).trim_end())
    );
    assert!(long.starts_with(&cut));
    // A text at the limit is kept whole; one byte over loses its last word.
    let exact = format!("{}abcd", "abc ".repeat(63));
    assert_eq!(reason::admit(&exact).as_deref(), Some(exact.as_str()));
    let over = format!("{}abcde", "abc ".repeat(63));
    assert_eq!(
        reason::admit(&over).as_deref(),
        Some("abc ".repeat(63).trim_end())
    );
    // A secret beyond the cut still withholds the reason it was cut from.
    let hidden = format!("{} fixtok-AbCdEfGhIjKl0123456789", "word ".repeat(60));
    assert_eq!(reason::admit(&hidden), None);
}

#[test]
fn a_reason_carrying_the_requests_own_credential_is_withheld() {
    let token = br#"{"token":"fixture-pat-one"}"#;
    for text in [
        "Unauthorized for fixture-pat-one",
        "Unauthorized for FIXTURE-PAT-ONE",
        // Any eight-byte piece of a credential value is enough.
        "no such key fixture-pat",
        "id x-pat-one unknown",
    ] {
        assert!(reason::carries_credential(text, token), "{text:?}");
    }
    assert!(!reason::carries_credential(
        "Unauthorized; scope does not match",
        token
    ));
    // Every string value of the document counts, the account included.
    let basic = br#"{"account":"someone@example.test","token":"short"}"#;
    assert!(reason::carries_credential(
        "account someone@example.test lacks access",
        basic
    ));
    // A value shorter than eight bytes counts whole.
    assert!(reason::carries_credential("bad short credential", basic));
    // A document that is not JSON counts as one value.
    assert!(reason::carries_credential(
        "header Zm9vYmFy invalid",
        b"Basic Zm9vYmFyYmF6"
    ));
    assert!(!reason::carries_credential("Scope does not match", b""));
}
