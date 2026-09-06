use super::*;

#[tokio::test]
async fn rate_adversary_header_cardinality_and_unfinished_body_are_separate() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    for (headers, expected) in [
        ("Retry-After: 00030\t\r\n", Some("30")),
        ("Retry-After: 0\r\n", Some("0")),
        (
            "Retry-After: 18446744073709551615\r\n",
            Some("18446744073709551615"),
        ),
        ("Retry-After: 30\r\nretry-after: 30\r\n", None),
        ("Retry-After: 30\r\nRETRY-AFTER: invalid\r\n", None),
        ("Retry-After: 30, 30\r\n", None),
        ("Retry-After: +30\r\n", None),
        ("Retry-After: 18446744073709551616\r\n", None),
    ] {
        for status in [429, 200] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (release, wait) = tokio::sync::oneshot::channel();
            let producer = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut b = [0];
                    stream.read_exact(&mut b).await.unwrap();
                    request.push(b[0]);
                    assert!(request.len() < 4096);
                }
                stream.write_all(format!("HTTP/1.1 {status} fixture\r\nContent-Length: 999999\r\n{headers}X-Private: SENTINEL\r\n\r\nSENTINEL").as_bytes()).await.unwrap();
                let _ = wait.await;
            });
            let response = reqwest::Client::new()
                .get(format!("http://{address}/"))
                .send()
                .await
                .unwrap();
            let response = tokio::time::timeout(
                Duration::from_secs(1),
                read_http_response(response, 64, vec!["retry-after".into()]),
            )
            .await
            .expect("a known oversized body must not be awaited");
            release.send(()).unwrap();
            producer.await.unwrap();
            if status == 429 {
                let response = response.unwrap();
                assert_eq!(response.status, 429);
                assert!(response.body.is_empty());
                assert_eq!(
                    response.headers.get("retry-after").map(String::as_str),
                    expected
                );
                assert_eq!(response.headers.len(), usize::from(expected.is_some()));
            } else {
                assert!(matches!(
                    response,
                    Err(EgressTransportError::ResponseTooLarge)
                ));
            }
        }
    }
}
