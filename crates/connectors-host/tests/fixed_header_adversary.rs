//! story:grafana-irm-history-reads, security review: conformance of the composition-fixed
//! header (`ScopedHttp::with_fixed_header`) and of the GET-only trailing-slash allowance
//! against `contracts/auth/capability/v1alpha1/http-headers.md` and the host's own doc
//! comments. Every request goes to a local fixture listener; no provider is contacted.
use connectors_core::ErrorCode;
use connectors_host::http::{FIXED_HEADER_VALUE_LIMIT, HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, WriteMethod};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn port(base: &str) -> ScopedHttp {
    ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap()
}

fn failed<T>(result: connectors_core::Result<T>) -> connectors_core::Error {
    match result {
        Ok(_) => panic!("the request was admitted"),
        Err(error) => error,
    }
}

fn refused_name(http: &ScopedHttp, name: &str) -> bool {
    match http.with_fixed_header(name, "fixture-value") {
        Ok(_) => false,
        Err(error) => {
            assert_eq!(error.code, ErrorCode::InvalidInput, "{name}");
            true
        }
    }
}

/// The contract names "content" headers as a class ("`content-type` ... and their kin"),
/// and the review brief's invariant names `Content-*`. Each member of that family,
/// in any case, is refused at composition.
#[test]
fn a_fixed_header_cannot_take_any_content_header() {
    let http = port("https://fixture.invalid/");
    let accepted: Vec<&str> = [
        "content-length",
        "content-type",
        "content-encoding",
        "content-range",
        "content-location",
        "content-language",
        "content-disposition",
        "content-md5",
        "Content-Range",
        "CONTENT-LOCATION",
    ]
    .into_iter()
    .filter(|name| !refused_name(&http, name))
    .collect();
    assert!(
        accepted.is_empty(),
        "content headers accepted as a fixed header: {accepted:?}"
    );
}

/// The contract refuses "routing" headers "and their kin". A forwarding header names
/// the origin or authority an intermediary routes to, and a method override retargets a
/// read port's GET as another verb at a provider that honours it.
#[test]
fn a_fixed_header_cannot_take_a_forwarding_or_method_override_header() {
    let http = port("https://fixture.invalid/");
    let accepted: Vec<&str> = [
        "forwarded",
        "x-forwarded-host",
        "x-forwarded-proto",
        "x-forwarded-for",
        "X-HTTP-Method-Override",
        "x-http-method",
        "x-method-override",
    ]
    .into_iter()
    .filter(|name| !refused_name(&http, name))
    .collect();
    assert!(
        accepted.is_empty(),
        "routing headers accepted as a fixed header: {accepted:?}"
    );
}

/// Every name the implementation lists is refused in every ASCII case variant, not only
/// the lower-case spelling the in-crate test exercises.
#[test]
fn every_refused_name_is_refused_in_any_case() {
    let http = port("https://fixture.invalid/");
    for name in [
        "authorization",
        "proxy-authorization",
        "cookie",
        "host",
        "content-length",
        "content-type",
        "content-encoding",
        "accept",
        "accept-encoding",
        "transfer-encoding",
        "connection",
        "proxy-connection",
        "keep-alive",
        "te",
        "trailer",
        "upgrade",
        "expect",
        "sec-websocket-protocol",
    ] {
        let upper = name.to_ascii_uppercase();
        let mixed: String = name
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect();
        for variant in [name, upper.as_str(), mixed.as_str()] {
            assert!(refused_name(&http, variant), "{variant} was accepted");
        }
    }
}

/// Values: exactly the visible ASCII bytes and the space are admitted; every control,
/// DEL and every non-ASCII character is refused; the limit is in bytes.
#[test]
fn a_fixed_header_value_admits_only_visible_ascii_and_space() {
    let http = port("https://fixture.invalid/");
    for byte in 0_u8..=0x7f {
        let value = format!("a{}b", byte as char);
        let admitted = http.with_fixed_header("x-fixture", &value).is_ok();
        let expected = byte == b' ' || byte.is_ascii_graphic();
        assert_eq!(admitted, expected, "byte {byte:#04x}");
    }
    for c in ['\u{80}', '\u{a0}', '\u{ff}', '\u{2028}', '\u{feff}'] {
        assert!(
            http.with_fixed_header("x-fixture", &format!("a{c}b"))
                .is_err(),
            "{c:?}"
        );
    }
    let at = "v".repeat(FIXED_HEADER_VALUE_LIMIT);
    assert!(http.with_fixed_header("x-fixture", &at).is_ok());
    let over = format!("{at}v");
    assert!(http.with_fixed_header("x-fixture", &over).is_err());
}

/// A fixture that accepts connections, records each request head, and answers `200 {}`.
async fn listener() -> (String, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let heads = Arc::new(std::sync::Mutex::new(Vec::new()));
    let (count, record) = (accepted.clone(), heads.clone());
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            count.fetch_add(1, Ordering::SeqCst);
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                match socket.read_u8().await {
                    Ok(byte) => head.push(byte),
                    Err(_) => break,
                }
            }
            let text = String::from_utf8_lossy(&head).into_owned();
            let length: usize = text
                .lines()
                .filter_map(|l| l.split_once(':'))
                .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse().unwrap_or(0))
                .unwrap_or(0);
            let mut body = vec![0; length];
            let _ = socket.read_exact(&mut body).await;
            record.lock().unwrap().push(text);
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await;
        }
    });
    (format!("http://{address}/base/"), accepted, heads)
}

/// The trailing-slash allowance is one empty last segment after named segments that are
/// themselves admissible: a dot segment before it is still refused, a slash inside a
/// named segment is still encoded, and nothing reaches the fixture for a refusal.
#[tokio::test]
async fn the_trailing_slash_never_admits_a_dot_segment_or_an_unencoded_slash() {
    let (base, accepted, heads) = listener().await;
    let http = port(&base);
    for segments in [
        &["..", ""][..],
        &[".", ""][..],
        &["a", "..", ""][..],
        &["", ""][..],
    ] {
        let error = failed(http.get(segments, &[]).await);
        assert_eq!(error.code, ErrorCode::InvalidInput, "{segments:?}");
        let error = failed(http.get_prefix(segments, &[], 16).await);
        assert_eq!(error.code, ErrorCode::InvalidInput, "{segments:?}");
    }
    assert_eq!(accepted.load(Ordering::SeqCst), 0);
    for (segments, line) in [
        (&["a/b", ""][..], "GET /base/a%2Fb/?"),
        (&["%2e%2e", ""][..], "GET /base/%252e%252e/?"),
        (&["a?b", ""][..], "GET /base/a%3Fb/?"),
    ] {
        assert_eq!(http.get(segments, &[]).await.unwrap().status, 200);
        let head = heads.lock().unwrap().last().unwrap().clone();
        assert!(head.starts_with(line), "{segments:?}: {head}");
    }
}

/// Writes keep refusing an empty last segment, with no request sent.
#[tokio::test]
async fn a_write_keeps_refusing_a_trailing_empty_segment() {
    let (base, accepted, _) = listener().await;
    for method in [
        WriteMethod::Post,
        WriteMethod::Put,
        WriteMethod::Patch,
        WriteMethod::Delete,
    ] {
        let write = port(&base).into_write();
        let error = failed(
            write
                .send_json(method, &["items", ""], &[], &serde_json::json!({}))
                .await,
        );
        assert_eq!(error.code, ErrorCode::InvalidInput, "{method:?}");
    }
    assert_eq!(accepted.load(Ordering::SeqCst), 0);
}

/// The contract: the credential-free form exchange carries neither the credential nor
/// the fixed header.
#[tokio::test]
async fn a_form_exchange_never_carries_the_fixed_header() {
    let (base, _, heads) = listener().await;
    let http = port(&base)
        .with_fixed_header("x-grafana-url", "https://stack.example")
        .unwrap();
    assert_eq!(
        http.post_form(&["token"], &[("v", b"x".as_slice())])
            .await
            .unwrap()
            .status,
        200
    );
    let head = heads.lock().unwrap()[0].to_ascii_lowercase();
    assert!(!head.contains("x-grafana-url"), "{head}");
}
