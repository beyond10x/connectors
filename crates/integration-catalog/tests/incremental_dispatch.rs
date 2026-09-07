//! Incremental contracts must hold through the personal runtime's catalog backend.
use async_trait::async_trait;
use connector_secrets::{CredentialRef, MemoryStore, Secret, SecretStore, StoreError};
use connectors_config::{CatalogIntegrationConfig, InitiationConfig, NetworkScopeConfig};
use integration_catalog::{credential_address, CatalogBackend};
use protocol::operation::{
    DescribeRequest, InvokeRequest, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OwnerContext,
};
use serde_json::{json, Value};
use service::{
    ConnectorBackend, EgressHttpRequest, EgressHttpResponse, EgressTransport, EgressTransportError,
    EgressWebSocket, PrincipalContext,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

struct CountedSecrets {
    inner: MemoryStore,
    reads: Arc<std::sync::atomic::AtomicUsize>,
}
#[async_trait]
impl SecretStore for CountedSecrets {
    async fn ready(&self) -> Result<(), StoreError> {
        self.inner.ready().await
    }
    async fn get(&self, key: &CredentialRef) -> Result<Secret, StoreError> {
        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.get(key).await
    }
    async fn put(&self, key: &CredentialRef, value: &Secret) -> Result<(), StoreError> {
        self.inner.put(key, value).await
    }
    async fn delete(&self, key: &CredentialRef) -> Result<(), StoreError> {
        self.inner.delete(key).await
    }
}
struct Fixture {
    credential_reads: Arc<std::sync::atomic::AtomicUsize>,
    status: u16,
    value: Value,
    seen: Mutex<Vec<EgressHttpRequest>>,
}
#[async_trait]
impl EgressTransport for Fixture {
    async fn execute(
        &self,
        _: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        self.seen.lock().unwrap().push(request);
        Ok(EgressHttpResponse {
            status: self.status,
            headers: BTreeMap::from([
                ("x-next-page".into(), "2".into()),
                ("retry-after".into(), "7".into()),
            ]),
            body: serde_json::to_vec(&self.value).unwrap(),
        })
    }
    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}
async fn invoke(
    provider: &str,
    operation: &str,
    input: Value,
    status: u16,
    value: Value,
) -> (Result<Value, OperationError>, Arc<Fixture>) {
    let principal = PrincipalContext::local(&OwnerContext {
        tenant_id: "local".into(),
        agent_id: "agent".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot:test".into(),
        authority_snapshot_sha256: "0".repeat(64),
    })
    .unwrap();
    let leaf = if provider == "gitlab" {
        "token"
    } else {
        "service_api_token"
    };
    let entry = CatalogIntegrationConfig {
        provider: provider.into(),
        instance: None,
        label: None,
        grant_ref: format!("grant:{provider}:test"),
        initiation: InitiationConfig::Platform,
        allow_writes: false,
        endpoints: BTreeMap::from([
            (
                "cloud_id".into(),
                "11111111-2222-3333-4444-555555555555".into(),
            ),
            ("origin".into(), "https://gitlab.example.test".into()),
        ]),
        usernames: BTreeMap::new(),
        operator_approved: true,
        network: NetworkScopeConfig::Public,
        credential: Some(format!("{provider}.{leaf}")),
        credential_file: None,
        oauth: None,
    };
    let declared = catalog::provider(catalog::ProviderKey::id(provider)).unwrap();
    let credential_reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let secrets = Arc::new(CountedSecrets {
        inner: MemoryStore::new(),
        reads: credential_reads.clone(),
    });
    let address = credential_address("local", declared.authority.unwrap(), &entry, leaf).unwrap();
    secrets
        .put(&address, &Secret::new("SENTINEL-NOT-A-REAL-SECRET"))
        .await
        .unwrap();
    let transport = Arc::new(Fixture {
        credential_reads,
        status,
        value,
        seen: Mutex::new(Vec::new()),
    });
    let backend =
        CatalogBackend::bind_stored(principal.clone(), &[entry], secrets, transport.clone())
            .unwrap();
    let OperationResult::Describe(description) = backend
        .handle(
            &principal,
            OperationRequest::Describe(DescribeRequest {
                operation_ref: operation.into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description")
    };
    transport
        .credential_reads
        .store(0, std::sync::atomic::Ordering::Relaxed);
    let result = backend
        .handle(
            &principal,
            OperationRequest::Invoke(InvokeRequest {
                operation_ref: operation.into(),
                connection_ref: description.connections[0].connection_ref.clone(),
                description_ref: description.description_ref,
                input,
                approval_evidence_ref: None,
            }),
        )
        .await
        .map(|result| {
            let OperationResult::Invoke(result) = result else {
                panic!("invocation")
            };
            result.output
        });
    (result, transport)
}
#[tokio::test]
async fn deployment_ordering_after_only() {
    assert_deployment_ordering(true, false).await;
}

#[tokio::test]
async fn deployment_ordering_before_only() {
    assert_deployment_ordering(false, true).await;
}

#[tokio::test]
async fn deployment_ordering_both_bounds() {
    assert_deployment_ordering(true, true).await;
}

#[tokio::test]
async fn deployment_ordering_neither_bound() {
    assert_deployment_ordering(false, false).await;
}

async fn assert_deployment_ordering(after: bool, before: bool) {
    let mut input = json!({"project_id":7,"per_page":2,"page":1});
    if after {
        input["updated_after"] = json!("2026-09-05T10:00:00+02:00");
    }
    if before {
        input["updated_before"] = json!("2026-09-06T10:00:00Z");
    }
    let (result, transport) = invoke(
        "gitlab",
        "gitlab-deployment-list",
        input.clone(),
        200,
        json!([{"id":1,"updated_at":"2026-09-05T10:00:00Z"}]),
    )
    .await;
    result.unwrap();
    {
        let seen = transport.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let target = url::Url::parse(&seen[0].request.url).unwrap();
        assert_eq!(target.path(), "/api/v4/projects/7/deployments");
        let pairs: Vec<_> = target.query_pairs().into_owned().collect();
        let query: BTreeMap<_, _> = pairs.iter().cloned().collect();
        assert_eq!(
            pairs.len(),
            query.len(),
            "query keys must not be duplicated"
        );
        let mut expected =
            BTreeMap::from([("per_page".into(), "2".into()), ("page".into(), "1".into())]);
        for key in ["updated_after", "updated_before"] {
            if let Some(value) = input[key].as_str() {
                expected.insert(key.into(), value.into());
            }
        }
        if after || before {
            expected.insert("order_by".into(), "updated_at".into());
        }
        assert_eq!(query, expected);
    }
    input["order_by"] = json!("updated_at");
    let (result, transport) =
        invoke("gitlab", "gitlab-deployment-list", input, 200, json!([])).await;
    assert_eq!(result.unwrap_err().code, OperationErrorCode::InvalidInput);
    assert!(
        transport.seen.lock().unwrap().is_empty(),
        "ordering remains derived, never caller input"
    );
    assert_eq!(
        transport
            .credential_reads
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn jira_incremental_personal_dispatch_translates_declared_input() {
    let (_result, transport) = invoke(
        "jira",
        "jira-issue-search",
        json!({"project_key":"PROJ","updated_since_ms":1700000000000_u64,"limit":2}),
        200,
        json!({"issues":[],"isLast":true}),
    )
    .await;
    let seen = transport.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    let target = url::Url::parse(&seen[0].request.url).unwrap();
    let query: BTreeMap<_, _> = target.query_pairs().into_owned().collect();
    assert_eq!(
        query.get("jql").map(String::as_str),
        Some("project = \"PROJ\" AND updated >= 1700000000000 ORDER BY updated ASC, key ASC")
    );
    assert_eq!(query.get("maxResults").map(String::as_str), Some("2"));
    assert!(!query.contains_key("project_key"));
}
#[tokio::test]
async fn gitlab_incremental_personal_dispatch_preserves_pagination_and_projection() {
    let (result, transport) = invoke("gitlab", "gitlab-pipeline-list", json!({"project_id":7,"per_page":2}), 200, json!([{"id":1,"project_id":7,"updated_at":"2026-09-05T10:00:00Z","user":{"email":"private@example.test"}}])).await;
    let output = result.unwrap();
    assert!(
        !output.to_string().contains("private@example.test"),
        "the personal read returned an unprojected person object: {output}"
    );
    assert_eq!(output["next_page"], 2);
    assert_eq!(output["items"][0]["id"], 1);
    assert!(transport.seen.lock().unwrap()[0]
        .response_headers
        .iter()
        .any(|header| header == "x-next-page"));
}
#[tokio::test]
async fn confluence_rate_limit_is_retriable() {
    let (result, transport) = invoke(
        "confluence",
        "confluence-page-search",
        json!({"cql":"space=DOCS AND type=page","limit":2,"expand":"version,body.storage,space"}),
        429,
        json!({"message":"private provider text"}),
    )
    .await;
    let error = result.unwrap_err();
    assert_eq!(error.code, OperationErrorCode::RateLimited);
    assert_eq!(error.retry_after_seconds, Some(7));
    assert!(transport.seen.lock().unwrap()[0]
        .response_headers
        .iter()
        .any(|header| header == "retry-after"));
    assert!(
        error.retriable,
        "rate limiting must permit retry: {error:?}"
    );
    assert!(!error.message.contains("private provider text"));
}
#[tokio::test]
async fn confluence_expired_permission_is_not_granted() {
    let (result, _) = invoke(
        "confluence",
        "confluence-page-search",
        json!({"cql":"space=DOCS AND type=page","limit":2,"expand":"version,body.storage,space"}),
        403,
        json!({}),
    )
    .await;
    assert_eq!(result.unwrap_err().code, OperationErrorCode::NotGranted);
}

fn examples() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        (
            "jira",
            "jira-issue-comments-read",
            json!({"issue_key":"PROJ-1","limit":2,"start_at":1}),
            json!({"comments":[{"id":"3","body":"Comment","created":"2026-09-05T10:00:00.000+0000","updated":"2026-09-05T10:00:00.000+0000","visibility":{"type":"role","value":"Team","identifier":"4","unknown":"PRIVATE-SENTINEL"},"jsdPublic":false,"author":{"displayName":"PRIVATE-SENTINEL"}}],"startAt":1,"maxResults":2,"total":4}),
        ),
        (
            "gitlab",
            "gitlab-issue-search",
            json!({"project_id":7,"per_page":2,"updated_after":"2026-09-05T10:00:00Z","state":"all"}),
            json!([{"id":1,"iid":2,"project_id":7,"title":"Issue","description":"Content","confidential":false,"updated_at":"2026-09-05T10:00:00Z","author":{"email":"PRIVATE-SENTINEL"}}]),
        ),
        (
            "gitlab",
            "gitlab-merge-request-search",
            json!({"project_id":7,"per_page":2,"updated_before":"2026-09-06T10:00:00Z","state":"merged"}),
            json!([{"id":1,"iid":2,"project_id":7,"title":"MR","draft":false,"updated_at":"2026-09-05T10:00:00Z","author":{"email":"PRIVATE-SENTINEL"}}]),
        ),
        (
            "jira",
            "jira-issue-search",
            json!({"project_key":"PROJ","updated_since_ms":1700000000000_u64,"limit":2,"next_page_token":"one"}),
            json!({"issues":[{"id":"1","key":"PROJ-1","fields":{"summary":"Issue","status":{"name":"Open"},"issuetype":{"name":"Task"},"updated":"2026-09-05T10:00:00Z","description":"Content","customfield_100":"PRIVATE-SENTINEL"}}],"isLast":false,"nextPageToken":"two"}),
        ),
        (
            "jira",
            "jira-project-list",
            json!({"limit":2,"start_at":1}),
            json!({"values":[{"id":"7","key":"PROJ","name":"Project","lead":{"email":"PRIVATE-SENTINEL"}}],"isLast":false,"startAt":1,"maxResults":2,"total":4}),
        ),
        (
            "gitlab",
            "gitlab-project-activity-list",
            json!({"per_page":2,"page":1,"last_activity_after":"2026-09-05T10:00:00Z","last_activity_before":"2026-09-06T10:00:00Z"}),
            json!([{"id":7,"name":"Project","last_activity_at":"2026-09-05T10:00:00Z","default_branch":"main","owner":{"email":"PRIVATE-SENTINEL"}}]),
        ),
        (
            "gitlab",
            "gitlab-pipeline-list",
            json!({"project_id":7,"per_page":2,"page":1}),
            json!([{"id":1,"project_id":7,"updated_at":"2026-09-05T10:00:00Z","user":{"email":"PRIVATE-SENTINEL"}}]),
        ),
        (
            "gitlab",
            "gitlab-deployment-list",
            json!({"project_id":7,"per_page":2,"page":1}),
            json!([{"id":1,"updated_at":"2026-09-05T10:00:00Z","deployable":{"commit":{"id":"abc123","author_email":"PRIVATE-SENTINEL"}},"environment":{"name":"production"}}]),
        ),
        (
            "gitlab",
            "gitlab-repository-commit-list",
            json!({"project_id":7,"per_page":2,"page":1,"ref_name":"main"}),
            json!([{"id":"abc123","committed_date":"2026-09-05T10:00:00Z","author_email":"PRIVATE-SENTINEL"}]),
        ),
        (
            "confluence",
            "confluence-page-search",
            json!({"cql":"space=DOCS AND type=page","limit":2,"expand":"version,body.storage,space"}),
            json!({"results":[{"id":"1","title":"Page","type":"page","version":{"number":1,"when":"2026-09-05T10:00:00Z","by":{"email":"PRIVATE-SENTINEL"}},"body":{"storage":{"value":"Content","representation":"storage"}},"space":{"key":"DOCS"},"_links":{"webui":"/pages/1"}}],"size":1,"limit":2,"_links":{"next":"/rest/api/content/search?cursor=two"}}),
        ),
    ]
}

#[tokio::test]
async fn every_personal_incremental_read_projects_its_declared_envelope() {
    for (provider, id, input, payload) in examples() {
        let (result, transport) = invoke(provider, id, input.clone(), 200, payload).await;
        {
            let seen = transport.seen.lock().unwrap();
            let target = url::Url::parse(&seen[0].request.url).unwrap();
            let query: BTreeMap<_, _> = target.query_pairs().into_owned().collect();
            if provider == "gitlab" {
                for key in [
                    "per_page",
                    "page",
                    "updated_after",
                    "updated_before",
                    "last_activity_after",
                    "last_activity_before",
                    "ref_name",
                    "state",
                ] {
                    if let Some(value) = input.get(key) {
                        let expected = value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string());
                        assert_eq!(query.get(key), Some(&expected), "{id}: {key}");
                    }
                }
                if let Some(project) = input["project_id"].as_u64() {
                    assert!(target.path().contains(&format!("/projects/{project}/")));
                }
            }
            if id == "jira-issue-comments-read" {
                assert!(target.path().ends_with("/issue/PROJ-1/comment"));
            }
        }
        let output = result.unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert!(
            !output.to_string().contains("PRIVATE-SENTINEL"),
            "{id} must scrub identity and unknown objects"
        );
        if provider == "gitlab" {
            assert_eq!(output["next_page"], 2, "{id}");
            assert!(output["items"].is_array());
        }
        if id == "jira-issue-search" {
            assert_eq!(output["next_page_token"], "two");
            assert_eq!(output["issues"][0]["id"], "1");
        }
        if matches!(id, "jira-project-list" | "jira-issue-comments-read") {
            assert_eq!(
                output["next_start_at"],
                if id == "jira-project-list" { 3 } else { 2 }
            );
            let seen = transport.seen.lock().unwrap();
            let url = url::Url::parse(&seen[0].request.url).unwrap();
            let query: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(query["startAt"], "1");
            assert_eq!(query["maxResults"], "2");
        }
    }
}

#[tokio::test]
async fn every_personal_incremental_read_rejects_unbounded_or_unknown_input_before_dispatch() {
    for (provider, id, input, payload) in examples() {
        for mutation in ["oversized", "fractional", "unknown"] {
            let mut invalid = input.clone();
            if mutation == "unknown" {
                invalid["undeclared"] = json!("value");
            } else {
                invalid[if provider == "gitlab" {
                    "per_page"
                } else {
                    "limit"
                }] = if mutation == "oversized" {
                    json!(101)
                } else {
                    json!(0.5)
                };
            }
            let (result, transport) = invoke(provider, id, invalid, 200, payload.clone()).await;
            assert_eq!(
                result.unwrap_err().code,
                OperationErrorCode::InvalidInput,
                "{id}: {mutation}"
            );
            assert_eq!(
                transport
                    .credential_reads
                    .load(std::sync::atomic::Ordering::Relaxed),
                0,
                "{id}: invalid input read credentials"
            );
            assert!(
                transport.seen.lock().unwrap().is_empty(),
                "{id}: invalid input reached provider"
            );
        }
    }
}

#[tokio::test]
async fn personal_incremental_reads_reject_foreign_projects_and_broken_pages() {
    for (provider, id, input, mut payload) in examples() {
        if id == "jira-issue-search" {
            payload["issues"][0]["key"] = json!("OTHER-1");
        } else if matches!(id, "jira-project-list" | "jira-issue-comments-read") {
            payload["startAt"] = json!(2);
        } else if matches!(
            id,
            "gitlab-pipeline-list"
                | "gitlab-deployment-list"
                | "gitlab-issue-search"
                | "gitlab-merge-request-search"
        ) {
            payload[0]["project_id"] = json!(8);
        } else {
            continue;
        }
        let (result, _) = invoke(provider, id, input, 200, payload).await;
        assert_eq!(
            result.unwrap_err().code,
            OperationErrorCode::Protocol,
            "{id}"
        );
    }
}

#[tokio::test]
async fn every_personal_incremental_read_classifies_expired_permission_and_rate_limiting() {
    for (provider, id, input, _) in examples() {
        for status in [401, 403, 404, 429] {
            let (result, _) = invoke(
                provider,
                id,
                input.clone(),
                status,
                json!({"message":"PRIVATE-SENTINEL"}),
            )
            .await;
            let error = result.unwrap_err();
            assert_eq!(
                error.code,
                if status == 429 {
                    OperationErrorCode::RateLimited
                } else {
                    OperationErrorCode::NotGranted
                },
                "{id}: {status}"
            );
            assert_eq!(error.retriable, status == 429, "{id}: {status}");
            assert!(!error.message.contains("PRIVATE-SENTINEL"));
        }
    }
}

#[tokio::test]
async fn adversary_empty_optional_jira_description_is_a_valid_issue() {
    let payload = json!({"issues":[{"id":"1","key":"PROJ-1","fields":{
        "summary":"Issue without prose","status":{"name":"Open"},"issuetype":{"name":"Task"},
        "updated":"2026-09-05T10:00:00Z","description":""}}],"isLast":true});
    let (result, _) = invoke(
        "jira",
        "jira-issue-search",
        json!({"project_key":"PROJ","updated_since_ms":0,"limit":1}),
        200,
        payload,
    )
    .await;
    let output =
        result.expect("an empty optional description must not block the complete issue page");
    assert_eq!(output["issues"][0]["description"], "");
}

#[tokio::test]
async fn adversary_comment_pages_preserve_missing_and_group_restriction_evidence() {
    let comment = json!({"id":"3","body":"","created":"2026-09-05T10:00:00.000+0000",
        "updated":"2026-09-05T10:00:00.000+0000","visibility":{"type":"group","identifier":"group-7","value":"Readers","members":[{"email":"PRIVATE-SENTINEL"}]}});
    let (first, _) = invoke(
        "jira",
        "jira-issue-comments-read",
        json!({"issue_key":"PROJ-1","limit":2}),
        200,
        json!({"comments":[comment.clone()],"startAt":0,"maxResults":2,"total":2}),
    )
    .await;
    let first = first.unwrap();
    assert_eq!(first["next_start_at"], 1);
    assert!(first["comments"][0]["jsd_public"].is_null());
    assert_eq!(first["comments"][0]["visibility"]["identifier"], "group-7");
    assert!(!first.to_string().contains("PRIVATE-SENTINEL"));
    let (last, _) = invoke(
        "jira",
        "jira-issue-comments-read",
        json!({"issue_key":"PROJ-1","limit":2,"start_at":1}),
        200,
        json!({"comments":[comment],"startAt":1,"maxResults":2,"total":2}),
    )
    .await;
    assert!(last.unwrap()["next_start_at"].is_null());
    for payload in [
        json!({"comments":[],"startAt":0,"maxResults":2,"total":1}),
        json!({"comments":[],"startAt":0,"maxResults":0,"total":0}),
    ] {
        let (result, _) = invoke(
            "jira",
            "jira-issue-comments-read",
            json!({"issue_key":"PROJ-1","limit":2}),
            200,
            payload,
        )
        .await;
        assert_eq!(result.unwrap_err().code, OperationErrorCode::Protocol);
    }
}

#[tokio::test]
async fn adversary_gitlab_update_searches_retain_confidential_and_draft_state() {
    for (id, key) in [
        ("gitlab-issue-search", "confidential"),
        ("gitlab-merge-request-search", "draft"),
    ] {
        let mut item = json!({"id":2,"project_id":7,"description":"","updated_at":"2026-09-05T10:00:00Z","author":{"email":"PRIVATE-SENTINEL"}});
        item[key] = json!(true);
        let input = json!({"project_id":7,"per_page":1,"state":"all","updated_after":"2026-09-05T10:00:00+02:00","updated_before":"2026-09-06T10:00:00Z"});
        let (result, transport) = invoke("gitlab", id, input, 200, json!([item])).await;
        let output = result.unwrap();
        assert_eq!(output["items"][0][key], true);
        assert_eq!(output["items"][0]["description"], "");
        assert!(!output.to_string().contains("PRIVATE-SENTINEL"));
        let seen = transport.seen.lock().unwrap();
        let target = url::Url::parse(&seen[0].request.url).unwrap();
        let query: BTreeMap<_, _> = target.query_pairs().into_owned().collect();
        assert_eq!(query["state"], "all");
        assert_eq!(query["updated_after"], "2026-09-05T10:00:00+02:00");
        assert_eq!(query["updated_before"], "2026-09-06T10:00:00Z");
    }
}
