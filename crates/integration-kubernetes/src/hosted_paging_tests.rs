#[cfg(test)]
mod paging_tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    /// How many deployments the fake namespace holds — the dev cluster's `latest` namespace
    /// shape that produced the live `result_too_large` refusal (S-063).
    const NAMESPACE_DEPLOYMENTS: usize = 120;
    /// Raw bytes of managed-fields-style bulk per fake Deployment object. Realistic objects
    /// carry 10–60 KiB of managedFields, annotations and pod template; 12 KiB × 25 objects is
    /// ~300 KiB, comfortably past `MAX_KUBERNETES_RESPONSE_BYTES`.
    const RAW_OBJECT_PADDING: usize = 12 * 1024;

    /// One fake apiserver serving `GET /apis/apps/v1/namespaces/latest/deployments` with the
    /// real API's `limit`/`continue` paging contract, over `NAMESPACE_DEPLOYMENTS` bloated
    /// objects. Plain std networking on a thread: no new dependency, one connection per
    /// request.
    fn fake_apiserver() -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake apiserver");
        let port = listener.local_addr().expect("local addr").port();
        let handle = std::thread::spawn(move || {
            let padding = "x".repeat(RAW_OBJECT_PADDING);
            loop {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut request = Vec::new();
                let mut byte = [0_u8; 1];
                while !request.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => request.push(byte[0]),
                        _ => break,
                    }
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                let target = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                if target == "/stop" {
                    let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\n\r\n");
                    return;
                }
                assert!(
                    target.starts_with("/apis/apps/v1/namespaces/latest/deployments"),
                    "unexpected request target: {target}"
                );
                let query = target.split_once('?').map(|(_, query)| query).unwrap_or("");
                let mut limit = NAMESPACE_DEPLOYMENTS;
                let mut start = 0_usize;
                for pair in query.split('&') {
                    if let Some(value) = pair.strip_prefix("limit=") {
                        limit = value.parse().expect("limit is a number");
                    }
                    if let Some(value) = pair.strip_prefix("continue=") {
                        start = value
                            .strip_prefix("off-")
                            .expect("continue token shape")
                            .parse()
                            .expect("continue offset");
                    }
                }
                let end = NAMESPACE_DEPLOYMENTS.min(start + limit);
                let items = (start..end)
                    .map(|index| {
                        serde_json::json!({
                            "metadata": {
                                "name": format!("dep-{index:03}"),
                                "namespace": "latest",
                                "uid": format!("uid-{index:03}"),
                                "resourceVersion": "42",
                                "generation": 7,
                                "managedFields": [{"raw": padding}]
                            },
                            "spec": {"replicas": 3, "selector": {"matchLabels": {"app": "a"}}},
                            "status": {
                                "observedGeneration": 7,
                                "readyReplicas": 3,
                                "availableReplicas": 3,
                                "updatedReplicas": 3,
                                "conditions": [{"type": "Available", "status": "True"}]
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                let continue_token = if end < NAMESPACE_DEPLOYMENTS {
                    format!("off-{end}")
                } else {
                    String::new()
                };
                let body = serde_json::json!({
                    "metadata": {"continue": continue_token},
                    "items": items,
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (port, handle)
    }

    fn reader_against(port: u16) -> (InClusterReader, tempfile::NamedTempFile) {
        let token_file = tempfile::NamedTempFile::new().expect("token file");
        fs::write(token_file.path(), "test-token").expect("token bytes");
        let reader = InClusterReader {
            client: reqwest::Client::new(),
            base: Url::parse(&format!("http://127.0.0.1:{port}/")).expect("fake base url"),
            token_file: token_file.path().to_path_buf(),
        };
        (reader, token_file)
    }

    fn stop_apiserver(port: u16, handle: std::thread::JoinHandle<()>) {
        let _ = std::net::TcpStream::connect(("127.0.0.1", port)).map(|mut stream| {
            let _ = stream.write_all(b"GET /stop HTTP/1.1\r\nhost: fake\r\n\r\n");
        });
        let _ = handle.join();
    }

    /// The S-063 live failure, reproduced headlessly: a namespace whose raw Deployment objects
    /// are far larger than their projections must list completely — the upstream response
    /// bound may never surface as `result_too_large` for an ordinary page, because the raw
    /// bytes are walked in bounded upstream pages and only the minimal projection is kept.
    #[tokio::test]
    async fn a_busy_namespace_lists_in_full_despite_the_upstream_response_bound() {
        let (port, handle) = fake_apiserver();
        let (reader, _token_file) = reader_against(port);

        let mut names = Vec::new();
        let mut cursor: Option<String> = None;
        let mut first_page = true;
        loop {
            let page = reader
                .list_workloads("latest", 25, cursor.as_deref())
                .await
                .expect("a busy namespace must list instead of refusing result_too_large");
            if first_page {
                assert_eq!(page.workloads.len(), 25, "the page bound is honored");
                first_page = false;
            }
            for workload in &page.workloads {
                let record = serde_json::to_value(workload).expect("record serializes");
                let mut keys = record
                    .as_object()
                    .expect("record is an object")
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                keys.sort();
                assert_eq!(
                    keys,
                    ["desired_replicas", "name", "ready_replicas", "rollout_state"],
                    "a list record carries the minimal projection and nothing else"
                );
                names.push(record["name"].as_str().expect("name").to_owned());
            }
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        assert_eq!(names.len(), NAMESPACE_DEPLOYMENTS);
        assert_eq!(names.first().map(String::as_str), Some("dep-000"));
        assert_eq!(names.last().map(String::as_str), Some("dep-119"));

        stop_apiserver(port, handle);
    }
}
