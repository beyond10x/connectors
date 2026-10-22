//! story:revalidation-retries-a-transient-failure (beyond10x/connectors#174).
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`), whose validation
//! is an identity probe through the host's own HTTP transport
//! (`connectors_host::http::ScopedHttp`, the catalog provider's) to a provider
//! this test serves on the loopback.
//!
//! The observed defect: a connect leaves a keep-alive connection in the
//! adapter child's pool; the provider closes it while the child is idle; the
//! next `connections revalidate` sends its probe on that connection, fails
//! `unavailable` at `dispatch`, and only a second, manual revalidation, on a new
//! connection, succeeds. The owner now sends the validation again within the
//! revalidation's deadline (contracts/cli/v1alpha1/semantics.md §6
//! "Revalidation retries"), never after a credential or permission refusal,
//! and a failure that stays transient names its cause class and attempts.
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{registry, runtime},
};
use connectors_sdk::AuthenticatedHttp;
use mounted::*;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const ADAPTER_FIXTURE: &str = "revalidate_transient_adapter_fixture";
/// Inside the jail: the loopback address of the test's provider.
const PROVIDER: &str = "/work/provider";

/// The adapter child: [`mounted::bootstrap`]'s description, validating with
/// one `GET /user` to the provider named in [`PROVIDER`] through one
/// `ScopedHttp` kept for the child's life, so its connections are pooled as the
/// catalog provider's are.
struct Probing;

fn transport() -> &'static ScopedHttp {
    static HTTP: OnceLock<ScopedHttp> = OnceLock::new();
    HTTP.get_or_init(|| {
        let address = std::fs::read_to_string(PROVIDER).unwrap();
        ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{}/", address.trim()),
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
    })
}

#[async_trait::async_trait]
impl runtime::Adapter for Probing {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        let response = transport()
            .get(&["user"], &[])
            .await
            .map_err(runtime::Failure::from_provider)?;
        // As the catalog provider's identity probe classifies its answer.
        match response.status {
            200 => {}
            401 => return Err(runtime::Failure::InvalidCredential),
            403 => return Err(runtime::Failure::Forbidden),
            429 | 500..=599 => return Err(runtime::Failure::Unavailable),
            _ => return Err(runtime::Failure::Protocol),
        }
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "fixture.user".into(),
                subject: "fixture-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        _: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        Err(runtime::Failure::Unsupported)
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn revalidate_transient_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Probing);
    }
}

/// A loopback HTTP/1.1 provider. Each request is answered with the next
/// scripted status (200 once the script is empty) on a kept-alive connection,
/// which the provider closes `idle_close` after its answer, as a provider's
/// keep-alive timeout does.
struct Provider {
    address: String,
    connections: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
    statuses: Arc<Mutex<VecDeque<u16>>>,
}

impl Provider {
    fn start(idle_close: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let connections = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(AtomicUsize::new(0));
        let statuses = Arc::new(Mutex::new(VecDeque::new()));
        let (counted, served, scripted) = (connections.clone(), requests.clone(), statuses.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                counted.fetch_add(1, Ordering::SeqCst);
                let (served, scripted) = (served.clone(), scripted.clone());
                std::thread::spawn(move || {
                    loop {
                        let mut head = Vec::new();
                        let mut byte = [0u8; 1];
                        while !head.ends_with(b"\r\n\r\n") {
                            // EOF, or no next request within the keep-alive
                            // timeout: the provider closes the connection.
                            if stream.read(&mut byte).unwrap_or(0) == 0 {
                                let _ = stream.shutdown(Shutdown::Both);
                                return;
                            }
                            head.push(byte[0]);
                        }
                        served.fetch_add(1, Ordering::SeqCst);
                        let status = scripted.lock().unwrap().pop_front().unwrap_or(200);
                        let answer =
                            format!("HTTP/1.1 {status} Fixture\r\ncontent-length: 2\r\n\r\n{{}}");
                        if stream.write_all(answer.as_bytes()).is_err() {
                            return;
                        }
                        stream.set_read_timeout(Some(idle_close)).unwrap();
                    }
                });
            }
        });
        Self {
            address,
            connections,
            requests,
            statuses,
        }
    }
    fn script(&self, statuses: &[u16]) {
        self.statuses.lock().unwrap().extend(statuses);
    }
    fn counts(&self) -> (usize, usize) {
        (
            self.connections.load(Ordering::SeqCst),
            self.requests.load(Ordering::SeqCst),
        )
    }
}

/// A container whose adapter probes `provider`, with one connection made:
/// its reference and revision. The connect's probe leaves its connection in
/// the child's pool; the test then leaves the child idle for 600 ms, longer
/// than a provider with a 200 ms keep-alive timeout keeps that connection.
fn connected(provider: &Provider) -> (Container, String, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    std::fs::write(container.path(PROVIDER), &provider.address).unwrap();
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    let (connection, revision) = (
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    );
    assert_eq!(provider.counts(), (1, 1), "the connect sent one probe");
    std::thread::sleep(Duration::from_millis(600));
    (container, connection, revision)
}

fn revalidate(container: &Container, connection: &str, revision: &str) -> std::process::Output {
    container.run(&[
        "connections",
        "revalidate",
        "--adapter",
        ALIAS,
        "--connection",
        connection,
        "--expected-revision",
        revision,
    ])
}

/// The reported failure: the first revalidation after the provider closed
/// the idle pooled connection succeeds, on a new connection, without a
/// second command.
#[test]
fn a_revalidation_after_the_provider_closed_its_idle_connection_succeeds() {
    let provider = Provider::start(Duration::from_millis(200));
    let (container, connection, revision) = connected(&provider);
    let output = revalidate(&container, &connection, &revision);
    assert!(
        output.status.success(),
        "the first revalidation after an idle close failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let revalidated = success(&output);
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
    assert_eq!(
        provider.counts(),
        (2, 2),
        "the retry opened one new connection and sent one probe on it"
    );
}

/// A credential (401) or permission (403) the provider refuses is never sent
/// again, and the refusal names no transient cause.
#[test]
fn a_credential_or_permission_refusal_is_not_sent_again() {
    for status in [401, 403] {
        let provider = Provider::start(Duration::from_secs(30));
        let (container, connection, revision) = connected(&provider);
        provider.script(&[status]);
        let (code, data) = refusal(&revalidate(&container, &connection, &revision));
        assert_eq!(code, Some(1), "{status}: {data}");
        assert_eq!(provider.counts().1, 2, "{status} was sent again: {data}");
        assert!(data.get("cause").is_none(), "{status}: {data}");
        assert!(data.get("attempts").is_none(), "{status}: {data}");
    }
}

/// A provider that keeps answering 503 is probed three times within the
/// deadline; the failure keeps its code, stage and next action and names the
/// cause class and the attempts.
#[test]
fn a_persistent_provider_outage_names_its_cause_and_attempts() {
    let provider = Provider::start(Duration::from_secs(30));
    let (container, connection, revision) = connected(&provider);
    provider.script(&[503, 503, 503]);
    let (code, data) = refusal(&revalidate(&container, &connection, &revision));
    assert_eq!(
        (code, &data["code"], &data["stage"], &data["next_action"]),
        (
            Some(1),
            &json!("unavailable"),
            &json!("dispatch"),
            &json!("retry_explicitly")
        ),
        "{data}"
    );
    assert_eq!(
        (&data["cause"], &data["attempts"]),
        (&json!("provider_unavailable"), &json!(3)),
        "{data}"
    );
    assert_eq!(
        provider.counts().1,
        4,
        "one connect probe, three revalidation probes"
    );
}
