use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn forwarded_websocket_retains_logical_host_and_declared_authentication() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = ConnectionEgress::for_endpoint_route(
        "connection:ari",
        "http://asterisk.voice.svc:8088/",
        Some(listener.local_addr().unwrap()),
    )
    .unwrap();
    let url = "ws://asterisk.voice.svc:8088/ari/events?app=fixture";
    for header in ["Host", "Connection", "Upgrade", "Sec-WebSocket-Key"] {
        let refused = transport
            .connect_websocket_with_headers(
                "connection:ari",
                url.into(),
                BTreeMap::from([(header.into(), "override".into())]),
                4096,
            )
            .await;
        assert!(matches!(refused, Err(EgressTransportError::Refused)));
    }
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_hdr_async(
            stream,
            |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
                assert_eq!(request.headers()["host"], "asterisk.voice.svc:8088");
                assert_eq!(request.headers()["authorization"], "Basic fixture-only");
                assert_eq!(request.uri().path(), "/ari/events");
                Ok(response)
            },
        )
        .await
        .unwrap();
        socket
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "fixture-event".into(),
            ))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });
    let mut socket = transport
        .connect_websocket_with_headers(
            "connection:ari",
            url.into(),
            BTreeMap::from([("Authorization".into(), "Basic fixture-only".into())]),
            4096,
        )
        .await
        .unwrap();
    assert!(
        matches!(socket.receive().await.unwrap(), EgressWebSocketFrame::Text(value) if value == "fixture-event")
    );
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn endpoint_headers_cannot_replace_the_logical_host() {
    use service::EgressTransport;
    let transport = ConnectionEgress::for_endpoint_route(
        "connection:fixture",
        "http://loki.monitoring.svc:3100/",
        Some("127.0.0.1:1".parse().unwrap()),
    )
    .unwrap();
    let result = transport
        .execute(
            "connection:fixture",
            service::EgressHttpRequest {
                request: connector_resolve::Request {
                    method: "GET".into(),
                    url: "http://loki.monitoring.svc:3100/".into(),
                    headers: std::collections::BTreeMap::from([(
                        "Host".into(),
                        "another.internal".into(),
                    )]),
                    body: None,
                },
                maximum_response_bytes: 1024,
                response_headers: Vec::new(),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(service::EgressTransportError::Refused)
    ));
}

#[tokio::test]
async fn forwarded_http_retains_logical_host_and_refuses_another_authority_or_origin() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let route = listener.local_addr().unwrap();
    let transport = ConnectionEgress::for_endpoint_route(
        "connection:fixture",
        "http://loki.monitoring.svc:3100/",
        Some(route),
    )
    .unwrap();
    let url = Url::parse("http://loki.monitoring.svc:3100/api/v1/query").unwrap();
    assert!(transport
        .request(
            "connection:another",
            Method::GET,
            url.clone(),
            Duration::from_secs(2)
        )
        .await
        .is_err());
    assert!(transport
        .request(
            "connection:fixture",
            Method::GET,
            Url::parse("http://other.monitoring.svc:3100/").unwrap(),
            Duration::from_secs(2)
        )
        .await
        .is_err());
    assert!(transport
        .request(
            "connection:fixture",
            Method::GET,
            Url::parse("https://loki.monitoring.svc:3100/").unwrap(),
            Duration::from_secs(2)
        )
        .await
        .is_err());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let size = stream.read(&mut request).await.unwrap();
        let request = std::str::from_utf8(&request[..size])
            .unwrap()
            .to_ascii_lowercase();
        assert!(request.starts_with("get /api/v1/query http/1.1\r\n"));
        assert!(
            request.contains("\r\nhost: loki.monitoring.svc:3100\r\n"),
            "{request}"
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .await
            .unwrap();
    });
    let response = transport
        .request(
            "connection:fixture",
            Method::GET,
            url,
            Duration::from_secs(2),
        )
        .await
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().await.unwrap(), "ok");
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn endpoint_route_cannot_smuggle_a_destination_or_secret_into_its_capability() {
    for origin in [
        "http://user:password@service.local/",
        "http://service.local/?token=secret",
        "http://service.local/#fragment",
        "file:///etc/passwd",
    ] {
        assert!(ConnectionEgress::for_endpoint_route("connection:fixture", origin, None).is_err());
    }
    assert!(ConnectionEgress::for_endpoint_route(
        "connection:fixture",
        "http://service.local/",
        Some("10.0.0.2:8080".parse().unwrap())
    )
    .is_err());
    assert!(
        DestinationRule::exact_origin("http://service.local/", AddressScope::OperatorNetwork)
            .is_err()
    );
}
