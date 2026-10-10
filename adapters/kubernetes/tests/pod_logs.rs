//! `pods.logs` (profile `kubernetes-pod-logs`) against recorded log bodies in
//! the shape the pod log subresource returns with `timestamps=true`. No
//! cluster is contacted; every request is observed by path, query and the
//! byte limit the adapter asked the port to enforce.
use connectors_core::{ErrorCode, Result};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse, HttpResponsePrefix};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

struct Call {
    path: String,
    query: Vec<(String, String)>,
    limit: Option<usize>,
}

/// A recorded cluster: an exact path answers its recorded status and bytes,
/// every other path the provider's 404. A prefix read keeps at most `limit`
/// bytes and reports a clean end only when the body fit.
struct Recorded {
    routes: BTreeMap<String, (u16, Vec<u8>)>,
    calls: Mutex<Vec<Call>>,
}
impl Recorded {
    fn answer(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
        limit: Option<usize>,
    ) -> (u16, Vec<u8>) {
        let path = format!("/{}", segments.join("/"));
        self.calls.lock().unwrap().push(Call {
            path: path.clone(),
            query: query
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            limit,
        });
        self.routes
            .get(&path)
            .cloned()
            .unwrap_or((404, br#"{"kind":"Status","code":404}"#.to_vec()))
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Recorded {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let (status, body) = self.answer(segments, query, None);
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body,
        })
    }
    async fn get_prefix(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
        limit: usize,
    ) -> Result<HttpResponsePrefix> {
        let (status, mut body) = self.answer(segments, query, Some(limit));
        let complete = body.len() <= limit;
        body.truncate(limit);
        Ok(HttpResponsePrefix {
            status,
            headers: BTreeMap::new(),
            body,
            complete,
        })
    }
}

const LOG: &str = "/api/v1/namespaces/engineering/pods/api-0/log";

fn adapter(pod_logs: bool, routes: Vec<(&str, u16, &[u8])>) -> (Kubernetes, Arc<Recorded>) {
    let recorded = Arc::new(Recorded {
        routes: routes
            .into_iter()
            .map(|(path, status, body)| (path.to_owned(), (status, body.to_vec())))
            .collect(),
        calls: Mutex::new(Vec::new()),
    });
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs,
        pod_exec: false,
        kubeconfig_contexts: false,
    };
    let effective = json!({"service":{"instance":"recorded","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    let adapter = Kubernetes::new("recorded", config, effective, recorded.clone()).unwrap();
    (adapter, recorded)
}

fn query(recorded: &Recorded) -> BTreeMap<String, String> {
    recorded.calls.lock().unwrap()[0]
        .query
        .iter()
        .cloned()
        .collect()
}

#[tokio::test]
async fn pod_logs_are_bounded_and_container_selected() {
    let body = b"2026-10-10T12:00:00.123456789Z started api\n2026-10-10T12:00:01Z ready\n";
    let (adapter, recorded) = adapter(true, vec![(LOG, 200, body)]);
    assert!(
        adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "pods.logs")
    );
    let logs = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0","container":"api","since_seconds":600,"tail_lines":2,"max_bytes":4096}),
        )
        .await
        .unwrap();
    // One request, to the pod's log subresource, with the selection carried
    // exactly and the byte ceiling enforced by the port, not sent upstream.
    assert_eq!(recorded.count(), 1);
    {
        let calls = recorded.calls.lock().unwrap();
        assert_eq!(calls[0].path, LOG);
        assert_eq!(calls[0].limit, Some(4096));
    }
    let sent = query(&recorded);
    assert_eq!(sent.get("container").map(String::as_str), Some("api"));
    assert_eq!(sent.get("sinceSeconds").map(String::as_str), Some("600"));
    assert_eq!(sent.get("tailLines").map(String::as_str), Some("2"));
    assert_eq!(sent.get("timestamps").map(String::as_str), Some("true"));
    assert_eq!(sent.get("follow").map(String::as_str), Some("false"));
    assert_eq!(sent.get("previous").map(String::as_str), Some("false"));
    assert!(!sent.contains_key("limitBytes"));
    assert!(!sent.contains_key("insecureSkipTLSVerifyBackend"));

    let lines = logs["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["timestamp_unix_ns"], "1791633600123456789");
    assert_eq!(lines[0]["line"], "started api");
    assert_eq!(lines[1]["timestamp_unix_ns"], "1791633601000000000");
    assert_eq!(lines[1]["line"], "ready");
    assert_eq!(
        lines[0]["stream"],
        json!({"namespace":"engineering","pod":"api-0","container":"api"})
    );
    assert_eq!(lines[0]["source"], Value::Null);
    assert_eq!(lines[0]["redacted"], false);
    assert_eq!(
        logs["selection"],
        json!({"kind":"kubernetes-relative-tail","since_seconds":600,"tail_lines":2})
    );
    assert_eq!(logs["order"], "provider");
    // tail_lines occurrences were observed, so older ones may exist unseen.
    assert_eq!(logs["complete"], false);
    assert_eq!(logs["truncation"]["causes"], json!(["provider_limit"]));
    assert_eq!(logs["truncation"]["occurrences_dropped"], Value::Null);
    assert_eq!(logs["truncation"]["stream_groups_dropped"], 0);
    assert_eq!(logs["next_cursor"], Value::Null);
    assert_eq!(logs["provenance"]["resource"], "engineering/pods/api-0/log");
    assert_eq!(logs["provenance"]["source_revision"], Value::Null);
}

#[tokio::test]
async fn pod_logs_below_both_caps_are_complete_and_keep_order_empty_and_untimestamped_lines() {
    let body =
        b"2026-10-10T12:00:00.000000001Z first\nplain line\n\n2026-10-10T12:00:02Z unterminated";
    let (adapter, recorded) = adapter(true, vec![(LOG, 200, body)]);
    let logs = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0"}),
        )
        .await
        .unwrap();
    // Defaults: one day, 200 lines, 128 KiB; no container term when omitted.
    let sent = query(&recorded);
    assert!(!sent.contains_key("container"));
    assert_eq!(sent.get("sinceSeconds").map(String::as_str), Some("86400"));
    assert_eq!(sent.get("tailLines").map(String::as_str), Some("200"));
    assert_eq!(recorded.calls.lock().unwrap()[0].limit, Some(131_072));

    let lines: Vec<(Value, Value)> = logs["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| (line["timestamp_unix_ns"].clone(), line["line"].clone()))
        .collect();
    assert_eq!(
        lines,
        vec![
            (json!("1791633600000000001"), json!("first")),
            (Value::Null, json!("plain line")),
            (Value::Null, json!("")),
            (json!("1791633602000000000"), json!("unterminated")),
        ]
    );
    assert_eq!(logs["lines"][0]["stream"]["container"], Value::Null);
    assert_eq!(logs["complete"], true);
    assert_eq!(logs["truncation"]["causes"], json!([]));
    assert_eq!(logs["truncation"]["occurrences_dropped"], 0);
}

#[tokio::test]
async fn pod_logs_byte_cutoff_withholds_a_split_scalar_and_is_partial() {
    // "é" is two bytes; a ceiling of 25 cuts it in half.
    let body = "2026-10-10T12:00:00Z caf\u{e9} au lait\n".as_bytes();
    assert_eq!(&body[21..24], "caf".as_bytes());
    let (adapter, _) = adapter(true, vec![(LOG, 200, body)]);
    let logs = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0","max_bytes":25}),
        )
        .await
        .unwrap();
    let lines = logs["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["line"], "caf");
    assert_eq!(lines[0]["line_truncated"], true);
    assert_eq!(logs["complete"], false);
    assert_eq!(
        logs["truncation"]["causes"],
        json!(["source_bytes", "line_bytes"])
    );
    assert_eq!(logs["truncation"]["occurrences_dropped"], Value::Null);
}

#[tokio::test]
async fn pod_logs_clip_a_long_line_at_8_kib_and_stay_complete() {
    let long = format!("2026-10-10T12:00:00Z {}\n", "x".repeat(20 * 1024));
    let (adapter, _) = adapter(true, vec![(LOG, 200, long.as_bytes())]);
    let logs = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0"}),
        )
        .await
        .unwrap();
    let line = &logs["lines"][0];
    assert_eq!(line["line"].as_str().map(str::len), Some(8192));
    assert_eq!(line["line_truncated"], true);
    // Clipping is not omission: the read can still be complete.
    assert_eq!(logs["complete"], true);
    assert_eq!(logs["truncation"]["causes"], json!(["line_bytes"]));
    assert_eq!(logs["truncation"]["occurrences_dropped"], 0);
}

#[tokio::test]
async fn pod_logs_outside_the_configured_namespaces_are_refused_with_no_request() {
    let (adapter, recorded) = adapter(true, vec![]);
    let error = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"kube-system","pod":"api-0"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(recorded.count(), 0);
}

#[tokio::test]
async fn pod_logs_refuse_names_that_could_add_a_path_segment_or_query_term() {
    let (adapter, recorded) = adapter(true, vec![]);
    for input in [
        json!({"namespace":"engineering","pod":"api-0/../secrets"}),
        json!({"namespace":"engineering","pod":"api-0","container":"api&follow=true"}),
        json!({"namespace":"engineering","pod":"api-0","tail_lines":1001}),
        json!({"namespace":"engineering","pod":"api-0","previous":true}),
    ] {
        let error = adapter.invoke("pods.logs", input).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
    assert_eq!(recorded.count(), 0);
}

#[tokio::test]
async fn pod_logs_unconfigured_are_neither_advertised_nor_served() {
    let (adapter, recorded) = adapter(false, vec![(LOG, 200, b"line\n")]);
    assert!(
        !adapter
            .descriptor()
            .operations
            .iter()
            .any(|o| o.id == "pods.logs")
    );
    let error = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(recorded.count(), 0);
}

#[tokio::test]
async fn pod_logs_provider_denial_and_absence_are_not_empty_logs() {
    let (adapter, _) = adapter(true, vec![(LOG, 403, br#"{"kind":"Status","code":403}"#)]);
    let error = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    let error = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"missing"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn pod_logs_with_malformed_text_before_the_end_refuse() {
    let (adapter, _) = adapter(true, vec![(LOG, 200, b"ok\n\xff\xfe broken\n")]);
    let error = adapter
        .invoke(
            "pods.logs",
            json!({"namespace":"engineering","pod":"api-0"}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}
