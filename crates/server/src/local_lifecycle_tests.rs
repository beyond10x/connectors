//! Lifecycle transport behavior against the same test backend as ordinary local requests.

use super::tests::{temporary_socket, SyntheticBackend};
use super::*;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn lifecycle_stop_acknowledges_then_releases_socket_and_backend() {
    let (socket, root) = temporary_socket();
    let backend = Arc::new(SyntheticBackend::default());
    let daemon = LocalOperationDaemon::bind(&socket, backend.clone())
        .await
        .unwrap()
        .with_configuration(Some(PathBuf::from("/fixture/connectors.toml")));
    let serving = tokio::spawn(daemon.serve_until(std::future::pending()));
    for request in [
        protocol::lifecycle::LifecycleRequest::Status {},
        protocol::lifecycle::LifecycleRequest::Stop {},
    ] {
        let mut stream = UnixStream::connect(&socket).await.unwrap();
        let envelope = protocol::lifecycle::RequestEnvelope {
            protocol: protocol::lifecycle::CONTRACT.into(),
            request_id: "lifecycle-fixture".into(),
            request,
        };
        let mut bytes = serde_json::to_vec(&envelope).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
        stream.shutdown().await.unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).await.unwrap();
        let response: protocol::lifecycle::ResponseEnvelope = serde_json::from_str(&line).unwrap();
        assert!(response.valid_for("lifecycle-fixture"));
        assert_eq!(
            response.response.configuration.as_deref(),
            Some("/fixture/connectors.toml")
        );
    }
    tokio::time::timeout(Duration::from_secs(2), serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(backend.shutdown.load(Ordering::Acquire));
    assert_eq!(backend.operation_calls.load(Ordering::SeqCst), 0);
    assert!(!socket.exists());
    std::fs::remove_file(root.join(".connectors.lock")).unwrap();
    std::fs::remove_dir(root).unwrap();
}
