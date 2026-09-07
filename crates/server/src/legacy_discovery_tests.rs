use super::tests::{temporary_socket, SyntheticBackend};
use super::*;

#[tokio::test]
async fn old_discovery_contracts_refuse_before_backend_dispatch() {
    use protocol::connection::*;
    let backend = Arc::new(SyntheticBackend::default());
    let (socket, root) = temporary_socket();
    let daemon = LocalOperationDaemon::bind(&socket, backend.clone())
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(daemon.serve_until(async {
        let _ = stopped.await;
    }));
    for version in [CONTRACT, protocol::connection_v2::CONTRACT] {
        for request in [
            ConnectionRequest::CandidateSearch(CandidateSearchRequest {
                integration_ref: "kubernetes".into(),
                query: String::new(),
                limit: 1,
            }),
            ConnectionRequest::CandidateActivate(CandidateActivateRequest {
                candidate_ref: "candidate:fixture".into(),
                label: "fixture".into(),
            }),
            ConnectionRequest::ObservationSearch(ObservationSearchRequest {
                source_connection_ref: "connection:fixture".into(),
                query: String::new(),
                limit: 1,
            }),
            ConnectionRequest::Materialize(MaterializeRequest {
                observation_ref: "observation:fixture".into(),
            }),
        ] {
            let envelope = RequestEnvelope {
                protocol: version.into(),
                request_id: "legacy:fixture".into(),
                context: super::tests::context(),
                request,
            };
            let mut stream = UnixStream::connect(&socket).await.unwrap();
            let mut bytes = serde_json::to_vec(&envelope).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).await.unwrap();
            let mut response = String::new();
            BufReader::new(stream)
                .read_line(&mut response)
                .await
                .unwrap();
            let (_, response) =
                protocol::connection_v2::decode_response(response.as_bytes()).unwrap();
            let error = response.error.unwrap();
            assert!(!error.retriable);
            assert!(error.message.contains("legacy discovery is retired"));
        }
    }
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    assert!(!backend
        .connection_called
        .load(std::sync::atomic::Ordering::Acquire));
    std::fs::remove_dir_all(root).unwrap();
}
