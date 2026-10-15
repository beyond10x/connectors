//! Namespaces, single objects, events and rollout history against recorded
//! fixtures in the Kubernetes API's own response shapes. No cluster is
//! contacted; every provider request is observed by path and query.
use connectors_core::{ErrorCode, Result};
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

type Call = (String, BTreeMap<String, String>);

/// A recorded cluster: an exact path answers its recorded status and object;
/// every other path is the provider's 404.
struct Recorded {
    routes: BTreeMap<String, (u16, Value)>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Recorded {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let path = format!("/{}", segments.join("/"));
        let query = query
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        self.calls.lock().unwrap().push((path.clone(), query));
        let (status, body) = self
            .routes
            .get(&path)
            .cloned()
            .unwrap_or((404, json!({"kind":"Status","apiVersion":"v1","status":"Failure","reason":"NotFound","code":404})));
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}
impl Recorded {
    fn paths(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }
    fn query(&self, index: usize) -> BTreeMap<String, String> {
        self.calls.lock().unwrap()[index].1.clone()
    }
}

fn recorded_adapter(
    namespaces: &[&str],
    kinds: &[&str],
    routes: Vec<(&str, u16, Value)>,
) -> (Kubernetes, Arc<Recorded>) {
    let recorded = Arc::new(Recorded {
        routes: routes
            .into_iter()
            .map(|(path, status, body)| (path.to_owned(), (status, body)))
            .collect(),
        calls: Mutex::new(Vec::new()),
    });
    let config = Config {
        namespaces: namespaces.iter().map(|n| (*n).to_owned()).collect(),
        resource_kinds: kinds.iter().map(|k| (*k).to_owned()).collect(),
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec: false,
        node_reads: false,
        custom_resources: Vec::new(),
        kubeconfig_contexts: false,
    };
    let effective = json!({"service":{"instance":"recorded","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    let adapter = Kubernetes::new("recorded", config, effective, recorded.clone()).unwrap();
    (adapter, recorded)
}

fn namespace(name: &str) -> Value {
    json!({"kind":"Namespace","apiVersion":"v1","metadata":{"name":name,"uid":format!("ns-{name}"),"resourceVersion":"17","labels":{"kubernetes.io/metadata.name":name}},"spec":{"finalizers":["kubernetes"]},"status":{"phase":"Active"}})
}
fn pod(namespace: &str, name: &str) -> Value {
    json!({"kind":"Pod","apiVersion":"v1","metadata":{"name":name,"namespace":namespace,"uid":"pod-uid","resourceVersion":"301"},"spec":{"containers":[{"name":"api","image":"registry.invalid/api:1"}]},"status":{"phase":"Running"}})
}
fn deployment(namespace: &str, name: &str) -> Value {
    json!({"kind":"Deployment","apiVersion":"apps/v1","metadata":{"name":name,"namespace":namespace,"uid":"deployment-uid","resourceVersion":"402","annotations":{"deployment.kubernetes.io/revision":"3"}},
        "spec":{"replicas":2,"revisionHistoryLimit":10,"selector":{"matchLabels":{"app":"api"},"matchExpressions":[{"key":"tier","operator":"In","values":["web"]}]}},
        "status":{"replicas":2,"readyReplicas":2,"observedGeneration":3}})
}
fn replicaset(name: &str, revision: &str, owner_uid: &str, controller: bool) -> Value {
    json!({"kind":"ReplicaSet","apiVersion":"apps/v1","metadata":{"name":name,"namespace":"engineering","uid":format!("{name}-uid"),"resourceVersion":"500",
        "labels":{"app":"api","tier":"web"},
        "annotations":{"deployment.kubernetes.io/revision":revision},
        "ownerReferences":[{"apiVersion":"apps/v1","kind":"Deployment","name":"api","uid":owner_uid,"controller":controller,"blockOwnerDeletion":true}]},
        "spec":{"replicas":0},"status":{"replicas":0}})
}
fn list(kind: &str, items: Vec<Value>, continuation: &str) -> Value {
    json!({"kind":kind,"apiVersion":"v1","metadata":{"resourceVersion":"900","continue":continuation},"items":items})
}

#[tokio::test]
async fn resources_get_reads_one_pod_and_one_deployment_by_name() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering"],
        &["pods", "deployments"],
        vec![
            (
                "/api/v1/namespaces/engineering/pods/api-7d9f",
                200,
                pod("engineering", "api-7d9f"),
            ),
            (
                "/apis/apps/v1/namespaces/engineering/deployments/api",
                200,
                deployment("engineering", "api"),
            ),
        ],
    );
    let pod = adapter
        .invoke(
            "resources.get",
            json!({"namespace":"engineering","kind":"pods","name":"api-7d9f"}),
        )
        .await
        .unwrap();
    assert_eq!(pod["items"].as_array().map(Vec::len), Some(1));
    // The full provider object, not a projection.
    assert_eq!(pod["items"][0]["status"]["phase"], "Running");
    assert_eq!(pod["complete"], true);
    assert_eq!(pod["next_cursor"], Value::Null);
    assert_eq!(pod["provenance"]["resource"], "engineering/pods/api-7d9f");
    assert_eq!(pod["provenance"]["source_revision"], "301");
    let deployment = adapter
        .invoke(
            "resources.get",
            json!({"namespace":"engineering","kind":"deployments","name":"api"}),
        )
        .await
        .unwrap();
    assert_eq!(deployment["items"][0]["status"]["readyReplicas"], 2);
    assert_eq!(
        recorded.paths(),
        [
            "/api/v1/namespaces/engineering/pods/api-7d9f",
            "/apis/apps/v1/namespaces/engineering/deployments/api"
        ]
    );
}

#[tokio::test]
async fn a_read_outside_the_configured_scope_is_refused_before_any_request() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering"],
        &["pods", "deployments", "replicasets"],
        vec![],
    );
    for (operation, input) in [
        (
            "resources.get",
            json!({"namespace":"kube-system","kind":"pods","name":"api"}),
        ),
        (
            "resources.get",
            json!({"namespace":"engineering","kind":"services","name":"api"}),
        ),
        (
            "resources.get",
            json!({"namespace":"engineering","kind":"events","name":"api"}),
        ),
        (
            "deployments.history",
            json!({"namespace":"kube-system","name":"api","limit":10}),
        ),
        (
            "resources.list",
            json!({"namespace":"engineering","kind":"events","limit":10}),
        ),
    ] {
        assert_eq!(
            adapter
                .invoke(operation, input.clone())
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden,
            "{operation} {input}"
        );
    }
    // A name that is not an object name never becomes a path segment.
    for name in ["../secrets", "Api", "api/log", "", "-api", "api."] {
        assert_eq!(
            adapter
                .invoke(
                    "resources.get",
                    json!({"namespace":"engineering","kind":"pods","name":name}),
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput,
            "{name:?}"
        );
    }
    assert!(recorded.paths().is_empty());
}

#[tokio::test]
async fn an_absent_object_answers_not_found_rather_than_an_empty_page() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering"],
        &["pods", "deployments", "replicasets"],
        vec![],
    );
    let missing = adapter
        .invoke(
            "resources.get",
            json!({"namespace":"engineering","kind":"pods","name":"gone"}),
        )
        .await
        .unwrap_err();
    assert_eq!(missing.code, ErrorCode::NotFound);
    let history = adapter
        .invoke(
            "deployments.history",
            json!({"namespace":"engineering","name":"gone","limit":10}),
        )
        .await
        .unwrap_err();
    assert_eq!(history.code, ErrorCode::NotFound);
    // The absent Deployment ends the read: no ReplicaSet list is attempted.
    assert_eq!(
        recorded.paths(),
        [
            "/api/v1/namespaces/engineering/pods/gone",
            "/apis/apps/v1/namespaces/engineering/deployments/gone"
        ]
    );
}

#[tokio::test]
async fn a_single_object_reporting_another_identity_is_refused() {
    let (adapter, _) = recorded_adapter(
        &["engineering", "payments"],
        &["pods"],
        vec![(
            "/api/v1/namespaces/engineering/pods/api",
            200,
            pod("payments", "api"),
        )],
    );
    assert_eq!(
        adapter
            .invoke(
                "resources.get",
                json!({"namespace":"engineering","kind":"pods","name":"api"}),
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::UpstreamProtocol
    );
}

#[tokio::test]
async fn namespaces_list_reads_each_configured_namespace_by_name_and_omits_absent_ones() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering", "missing", "payments"],
        &["pods"],
        vec![
            (
                "/api/v1/namespaces/engineering",
                200,
                namespace("engineering"),
            ),
            ("/api/v1/namespaces/payments", 200, namespace("payments")),
            // Present in the cluster, never configured, so never read.
            (
                "/api/v1/namespaces/kube-system",
                200,
                namespace("kube-system"),
            ),
            (
                "/api/v1/namespaces",
                200,
                list("NamespaceList", vec![namespace("kube-system")], ""),
            ),
        ],
    );
    let page = adapter
        .invoke("namespaces.list", json!({"limit":10}))
        .await
        .unwrap();
    let names: Vec<&str> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["metadata"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["engineering", "payments"]);
    assert_eq!(page["items"][0]["status"]["phase"], "Active");
    assert_eq!(page["complete"], true);
    assert_eq!(page["next_cursor"], Value::Null);
    assert_eq!(page["provenance"]["resource"], "namespaces");
    assert_eq!(page["provenance"]["source_revision"], Value::Null);
    // Exactly the configured names, each by name; the cluster-scoped
    // collection is never listed.
    assert_eq!(
        recorded.paths(),
        [
            "/api/v1/namespaces/engineering",
            "/api/v1/namespaces/missing",
            "/api/v1/namespaces/payments"
        ]
    );
}

#[tokio::test]
async fn namespaces_list_pages_the_configured_set_and_a_denied_namespace_is_not_absent() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering", "payments"],
        &["pods"],
        vec![
            (
                "/api/v1/namespaces/engineering",
                200,
                namespace("engineering"),
            ),
            ("/api/v1/namespaces/payments", 200, namespace("payments")),
        ],
    );
    let first = adapter
        .invoke("namespaces.list", json!({"limit":1}))
        .await
        .unwrap();
    assert_eq!(first["items"][0]["metadata"]["name"], "engineering");
    assert_eq!(first["complete"], false);
    let cursor = first["next_cursor"].as_str().unwrap().to_owned();
    let second = adapter
        .invoke("namespaces.list", json!({"limit":1,"cursor":cursor}))
        .await
        .unwrap();
    assert_eq!(second["items"][0]["metadata"]["name"], "payments");
    assert_eq!(second["complete"], true);
    assert_eq!(recorded.paths().len(), 2);
    // A continuation issued for one page size does not resume another.
    assert_eq!(
        adapter
            .invoke("namespaces.list", json!({"limit":2,"cursor":cursor}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    assert_eq!(recorded.paths().len(), 2);

    let (denied, _) = recorded_adapter(
        &["engineering", "payments"],
        &["pods"],
        vec![
            (
                "/api/v1/namespaces/engineering",
                200,
                namespace("engineering"),
            ),
            (
                "/api/v1/namespaces/payments",
                403,
                json!({"kind":"Status","code":403,"reason":"Forbidden"}),
            ),
        ],
    );
    assert_eq!(
        denied
            .invoke("namespaces.list", json!({"limit":10}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
}

#[tokio::test]
async fn resources_list_reads_replicasets_and_events() {
    let event = json!({"kind":"Event","apiVersion":"v1","metadata":{"name":"api-7d9f.17a3","namespace":"engineering","resourceVersion":"77"},
        "involvedObject":{"kind":"Pod","name":"api-7d9f","namespace":"engineering"},"reason":"BackOff","type":"Warning","count":4});
    let (adapter, recorded) = recorded_adapter(
        &["engineering"],
        &["replicasets", "events"],
        vec![
            (
                "/apis/apps/v1/namespaces/engineering/replicasets",
                200,
                list(
                    "ReplicaSetList",
                    vec![replicaset("api-1", "1", "deployment-uid", true)],
                    "",
                ),
            ),
            (
                "/api/v1/namespaces/engineering/events",
                200,
                list("EventList", vec![event], ""),
            ),
        ],
    );
    let replicasets = adapter
        .invoke(
            "resources.list",
            json!({"namespace":"engineering","kind":"replicasets","limit":10}),
        )
        .await
        .unwrap();
    assert_eq!(replicasets["items"][0]["metadata"]["name"], "api-1");
    assert_eq!(
        replicasets["provenance"]["resource"],
        "engineering/replicasets"
    );
    let events = adapter
        .invoke(
            "resources.list",
            json!({"namespace":"engineering","kind":"events","limit":10}),
        )
        .await
        .unwrap();
    assert_eq!(events["items"][0]["reason"], "BackOff");
    assert_eq!(events["complete"], true);
    assert_eq!(events["provenance"]["source_revision"], "900");
    assert_eq!(
        recorded.paths(),
        [
            "/apis/apps/v1/namespaces/engineering/replicasets",
            "/api/v1/namespaces/engineering/events"
        ]
    );
}

#[tokio::test]
async fn deployment_history_keeps_only_the_replicasets_the_deployment_controls() {
    let (adapter, recorded) = recorded_adapter(
        &["engineering"],
        &["deployments", "replicasets"],
        vec![
            (
                "/apis/apps/v1/namespaces/engineering/deployments/api",
                200,
                deployment("engineering", "api"),
            ),
            (
                "/apis/apps/v1/namespaces/engineering/replicasets",
                200,
                list(
                    "ReplicaSetList",
                    vec![
                        replicaset("api-1", "1", "deployment-uid", true),
                        replicaset("api-2", "2", "deployment-uid", true),
                        // Matching labels, a different controlling owner: a
                        // predecessor Deployment of the same name.
                        replicaset("api-old", "7", "predecessor-uid", false),
                        replicaset("api-foreign", "1", "predecessor-uid", true),
                        // The right uid but not the controller.
                        replicaset("api-adopted", "4", "deployment-uid", false),
                    ],
                    "",
                ),
            ),
        ],
    );
    let history = adapter
        .invoke(
            "deployments.history",
            json!({"namespace":"engineering","name":"api","limit":20}),
        )
        .await
        .unwrap();
    let revisions: Vec<(&str, &str)> = history["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["metadata"]["name"].as_str().unwrap(),
                item["metadata"]["annotations"]["deployment.kubernetes.io/revision"]
                    .as_str()
                    .unwrap(),
            )
        })
        .collect();
    assert_eq!(revisions, [("api-1", "1"), ("api-2", "2")]);
    assert_eq!(history["complete"], true);
    assert_eq!(
        history["provenance"]["resource"],
        "engineering/deployments/api/history"
    );
    assert_eq!(history["provenance"]["source_revision"], "900");
    assert_eq!(
        recorded.paths(),
        [
            "/apis/apps/v1/namespaces/engineering/deployments/api",
            "/apis/apps/v1/namespaces/engineering/replicasets"
        ]
    );
    // The list is selected by the Deployment's own selector, not by name.
    let query = recorded.query(1);
    assert_eq!(query["labelSelector"], "app=api,tier in (web)");
    assert_eq!(query["limit"], "20");
}

#[tokio::test]
async fn deployment_history_needs_both_kinds_configured() {
    let (adapter, recorded) = recorded_adapter(&["engineering"], &["deployments"], vec![]);
    assert_eq!(
        adapter
            .invoke(
                "deployments.history",
                json!({"namespace":"engineering","name":"api","limit":10}),
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert!(recorded.paths().is_empty());
}

#[tokio::test]
async fn deployment_history_refuses_a_selector_it_cannot_carry_exactly() {
    let mut empty = deployment("engineering", "api");
    empty["spec"]["selector"] = json!({});
    let mut injected = deployment("engineering", "api");
    injected["spec"]["selector"] = json!({"matchLabels":{"app":"api,tier!=web"}});
    for object in [empty, injected] {
        let (adapter, recorded) = recorded_adapter(
            &["engineering"],
            &["deployments", "replicasets"],
            vec![(
                "/apis/apps/v1/namespaces/engineering/deployments/api",
                200,
                object,
            )],
        );
        assert_eq!(
            adapter
                .invoke(
                    "deployments.history",
                    json!({"namespace":"engineering","name":"api","limit":10}),
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::UpstreamProtocol
        );
        // Refused before the ReplicaSet list, which would otherwise widen.
        assert_eq!(recorded.paths().len(), 1);
    }
}

#[tokio::test]
async fn the_new_kinds_are_configurable_and_the_new_reads_are_advertised() {
    let (adapter, _) = recorded_adapter(&["engineering"], &["replicasets", "events"], vec![]);
    for operation in [
        "resources.get",
        "namespaces.list",
        "deployments.history",
        "resources.list",
    ] {
        assert!(
            adapter.descriptor().operation(operation).is_ok(),
            "{operation}"
        );
    }
    let config = Config {
        namespaces: vec!["engineering".into()],
        resource_kinds: vec!["namespaces".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::Off,
        pod_logs: false,
        pod_exec: false,
        node_reads: false,
        custom_resources: Vec::new(),
        kubeconfig_contexts: false,
    };
    let effective = json!({"adapter":config});
    assert!(
        Kubernetes::new(
            "recorded",
            config,
            effective,
            Arc::new(Recorded {
                routes: BTreeMap::new(),
                calls: Mutex::new(Vec::new()),
            }),
        )
        .is_err()
    );
}

// Statefulsets, nodes and custom resources (story
// kubernetes-eks-exec-auth-and-kinds, the reads half).

/// A recorded cluster that answers each path with its recorded pages in order,
/// so a continuation can be followed to the provider's next page.
struct Paged {
    pages: Mutex<BTreeMap<String, Vec<(u16, Value)>>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Paged {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let path = format!("/{}", segments.join("/"));
        let query = query
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        self.calls.lock().unwrap().push((path.clone(), query));
        let (status, body) = self
            .pages
            .lock()
            .unwrap()
            .get_mut(&path)
            .filter(|pages| !pages.is_empty())
            .map(|pages| pages.remove(0))
            .unwrap_or((404, json!({"kind":"Status","apiVersion":"v1","status":"Failure","reason":"NotFound","code":404})));
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}
impl Paged {
    fn paths(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }
    fn query(&self, index: usize) -> BTreeMap<String, String> {
        self.calls.lock().unwrap()[index].1.clone()
    }
}

/// The adapter built from a configuration document, the way the federated
/// service reads one, so every new field is exercised through its serialized
/// name.
fn configured(
    adapter: Value,
    pages: Vec<(&str, Vec<(u16, Value)>)>,
) -> std::result::Result<(Kubernetes, Arc<Paged>), String> {
    let paged = Arc::new(Paged {
        pages: Mutex::new(
            pages
                .into_iter()
                .map(|(path, pages)| (path.to_owned(), pages))
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    });
    let config: Config = serde_json::from_value(adapter.clone()).map_err(|e| e.to_string())?;
    let effective = json!({"service":{"instance":"recorded","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"http":{"base_url":"https://fixture.invalid/"},"adapter":adapter});
    let built = Kubernetes::new("recorded", config, effective, paged.clone())
        .map_err(|e| e.message.to_string())?;
    Ok((built, paged))
}

fn node(name: &str) -> Value {
    json!({"kind":"Node","apiVersion":"v1","metadata":{"name":name,"uid":format!("{name}-uid"),"resourceVersion":"812",
        "labels":{"kubernetes.io/hostname":name,"node.kubernetes.io/instance-type":"m6i.large","eks.amazonaws.com/nodegroup":"general"}},
        "spec":{"providerID":format!("aws:///eu-central-1a/i-{name}")},
        "status":{"capacity":{"cpu":"2","memory":"7974872Ki","pods":"29"},
            "nodeInfo":{"kubeletVersion":"v1.31.5-eks-5d632ec","containerRuntimeVersion":"containerd://1.7.25","osImage":"Amazon Linux 2023"},
            "addresses":[{"type":"InternalIP","address":"10.0.1.12"}],
            "conditions":[{"type":"Ready","status":"True"}]}})
}
fn statefulset(name: &str) -> Value {
    json!({"kind":"StatefulSet","apiVersion":"apps/v1","metadata":{"name":name,"namespace":"engineering","uid":format!("{name}-uid"),"resourceVersion":"611"},
        "spec":{"replicas":3,"serviceName":name,"selector":{"matchLabels":{"app":name}}},
        "status":{"replicas":3,"readyReplicas":3,"currentRevision":format!("{name}-6c9d"),"updateRevision":format!("{name}-6c9d")}})
}
fn service_monitor(name: &str) -> Value {
    json!({"kind":"ServiceMonitor","apiVersion":"monitoring.coreos.com/v1","metadata":{"name":name,"namespace":"engineering","uid":format!("{name}-uid"),"resourceVersion":"733"},
        "spec":{"selector":{"matchLabels":{"app":name}},"endpoints":[{"port":"metrics","interval":"30s"}]}})
}
fn page(kind: &str, items: Vec<Value>, continuation: &str) -> Value {
    json!({"kind":kind,"apiVersion":"v1","metadata":{"resourceVersion":"900","continue":continuation},"items":items})
}
const SERVICE_MONITORS: &str =
    "/apis/monitoring.coreos.com/v1/namespaces/engineering/servicemonitors";
fn monitors() -> Value {
    json!({"group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors"})
}

#[tokio::test]
async fn statefulsets_are_listed_and_read_by_name() {
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["statefulsets"]}),
        vec![
            (
                "/apis/apps/v1/namespaces/engineering/statefulsets",
                vec![(200, page("StatefulSetList", vec![statefulset("redis")], ""))],
            ),
            (
                "/apis/apps/v1/namespaces/engineering/statefulsets/redis",
                vec![(200, statefulset("redis"))],
            ),
        ],
    )
    .unwrap();
    let listed = adapter
        .invoke(
            "resources.list",
            json!({"namespace":"engineering","kind":"statefulsets","limit":10}),
        )
        .await
        .unwrap();
    assert_eq!(listed["items"][0]["status"]["readyReplicas"], 3);
    assert_eq!(listed["complete"], true);
    assert_eq!(listed["provenance"]["resource"], "engineering/statefulsets");
    assert_eq!(listed["provenance"]["source_revision"], "900");
    let one = adapter
        .invoke(
            "resources.get",
            json!({"namespace":"engineering","kind":"statefulsets","name":"redis"}),
        )
        .await
        .unwrap();
    assert_eq!(one["items"][0]["spec"]["serviceName"], "redis");
    assert_eq!(one["provenance"]["source_revision"], "611");
    assert_eq!(
        paged.paths(),
        [
            "/apis/apps/v1/namespaces/engineering/statefulsets",
            "/apis/apps/v1/namespaces/engineering/statefulsets/redis"
        ]
    );
    assert_eq!(paged.query(0)["limit"], "10");
}

#[tokio::test]
async fn nodes_list_pages_full_node_objects_when_configured() {
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"node_reads":true}),
        vec![(
            "/api/v1/nodes",
            vec![
                (
                    200,
                    page("NodeList", vec![node("ip-10-0-1-12")], "node-token-1"),
                ),
                (200, page("NodeList", vec![node("ip-10-0-2-40")], "")),
            ],
        )],
    )
    .unwrap();
    assert!(adapter.descriptor().operation("nodes.list").is_ok());
    let first = adapter
        .invoke("nodes.list", json!({"limit":1}))
        .await
        .unwrap();
    // The full Node object, not the hosts.discover projection.
    assert_eq!(
        first["items"][0]["status"]["nodeInfo"]["kubeletVersion"],
        "v1.31.5-eks-5d632ec"
    );
    assert_eq!(
        first["items"][0]["metadata"]["labels"]["eks.amazonaws.com/nodegroup"],
        "general"
    );
    assert_eq!(first["complete"], false);
    assert_eq!(first["provenance"]["resource"], "nodes");
    assert_eq!(first["provenance"]["source_revision"], "900");
    let cursor = first["next_cursor"].as_str().unwrap();
    // The provider's token never reaches the caller.
    assert_ne!(cursor, "node-token-1");
    let second = adapter
        .invoke("nodes.list", json!({"limit":1,"cursor":cursor}))
        .await
        .unwrap();
    assert_eq!(second["items"][0]["metadata"]["name"], "ip-10-0-2-40");
    assert_eq!(second["complete"], true);
    assert_eq!(second["next_cursor"], Value::Null);
    assert_eq!(paged.paths(), ["/api/v1/nodes", "/api/v1/nodes"]);
    assert_eq!(paged.query(0).get("continue"), None);
    assert_eq!(paged.query(1)["continue"], "node-token-1");
    assert_eq!(paged.query(1)["limit"], "1");
}

#[tokio::test]
async fn nodes_list_is_neither_advertised_nor_answered_without_node_reads() {
    // discover_hosts reads the same collection, as a projection; it does not
    // admit the full objects.
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"discover_hosts":true}),
        vec![(
            "/api/v1/nodes",
            vec![(
                200,
                page("NodeList", vec![node("ip-10-0-1-12")], "node-token-1"),
            )],
        )],
    )
    .unwrap();
    assert!(adapter.descriptor().operation("nodes.list").is_err());
    assert_eq!(
        adapter
            .invoke("nodes.list", json!({"limit":1}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert!(paged.paths().is_empty());
    // A continuation hosts.discover issued does not page nodes.list, even
    // where both are configured.
    let (both, _) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"discover_hosts":true,"node_reads":true}),
        vec![(
            "/api/v1/nodes",
            vec![(200, page("NodeList", vec![node("ip-10-0-1-12")], "node-token-1"))],
        )],
    )
    .unwrap();
    let hosts = both
        .invoke("hosts.discover", json!({"limit":1}))
        .await
        .unwrap();
    let cursor = hosts["next_cursor"].as_str().unwrap();
    assert_eq!(
        both.invoke("nodes.list", json!({"limit":1,"cursor":cursor}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
}

#[tokio::test]
async fn custom_resources_list_reads_a_configured_collection_page_by_page() {
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"custom_resources":[monitors()]}),
        vec![(
            SERVICE_MONITORS,
            vec![
                (200, page("ServiceMonitorList", vec![service_monitor("api")], "sm-token-1")),
                (200, page("ServiceMonitorList", vec![service_monitor("worker")], "")),
            ],
        )],
    )
    .unwrap();
    assert!(
        adapter
            .descriptor()
            .operation("custom_resources.list")
            .is_ok()
    );
    let selection = json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors","limit":1});
    let first = adapter
        .invoke("custom_resources.list", selection.clone())
        .await
        .unwrap();
    assert_eq!(first["items"][0]["spec"]["endpoints"][0]["port"], "metrics");
    assert_eq!(first["complete"], false);
    assert_eq!(
        first["provenance"]["resource"],
        "engineering/monitoring.coreos.com/v1/servicemonitors"
    );
    let cursor = first["next_cursor"].as_str().unwrap();
    let mut next = selection.clone();
    next["cursor"] = json!(cursor);
    let second = adapter.invoke("custom_resources.list", next).await.unwrap();
    assert_eq!(second["items"][0]["metadata"]["name"], "worker");
    assert_eq!(second["complete"], true);
    assert_eq!(paged.paths(), [SERVICE_MONITORS, SERVICE_MONITORS]);
    assert_eq!(paged.query(1)["continue"], "sm-token-1");
}

#[tokio::test]
async fn a_custom_resource_cursor_pages_only_its_own_collection() {
    let rules = json!({"group":"monitoring.coreos.com","version":"v1","plural":"prometheusrules"});
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"custom_resources":[monitors(),rules]}),
        vec![(
            SERVICE_MONITORS,
            vec![(200, page("ServiceMonitorList", vec![service_monitor("api")], "sm-token-1"))],
        )],
    )
    .unwrap();
    let first = adapter
        .invoke(
            "custom_resources.list",
            json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors","limit":1}),
        )
        .await
        .unwrap();
    let cursor = first["next_cursor"].as_str().unwrap();
    assert_eq!(
        adapter
            .invoke(
                "custom_resources.list",
                json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"prometheusrules","limit":1,"cursor":cursor}),
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleCursor
    );
    assert_eq!(paged.paths().len(), 1);
}

#[tokio::test]
async fn custom_resources_outside_the_configuration_are_refused_before_any_request() {
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"custom_resources":[monitors()]}),
        vec![],
    )
    .unwrap();
    for selection in [
        // Another plural, version or group of a configured collection.
        json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"podmonitors","limit":1}),
        json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1beta1","plural":"servicemonitors","limit":1}),
        json!({"namespace":"engineering","group":"example.com","version":"v1","plural":"servicemonitors","limit":1}),
        // A built-in collection is never reachable through this operation.
        json!({"namespace":"engineering","group":"","version":"v1","plural":"secrets","limit":1}),
        json!({"namespace":"engineering","group":"rbac.authorization.k8s.io","version":"v1","plural":"roles","limit":1}),
        // A namespace outside the configured set.
        json!({"namespace":"kube-system","group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors","limit":1}),
    ] {
        assert_eq!(
            adapter
                .invoke("custom_resources.list", selection.clone())
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden,
            "{selection}"
        );
    }
    assert!(paged.paths().is_empty());
    // A configuration admitting none advertises nothing and answers nothing.
    let (none, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"]}),
        vec![],
    )
    .unwrap();
    assert!(
        none.descriptor()
            .operation("custom_resources.list")
            .is_err()
    );
    assert_eq!(
        none.invoke(
            "custom_resources.list",
            json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors","limit":1}),
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::Forbidden
    );
    assert!(paged.paths().is_empty());
}

#[tokio::test]
async fn a_configured_custom_resource_the_cluster_does_not_serve_is_not_found() {
    let (adapter, paged) = configured(
        json!({"namespaces":["engineering"],"resource_kinds":["pods"],"custom_resources":[monitors()]}),
        vec![],
    )
    .unwrap();
    assert_eq!(
        adapter
            .invoke(
                "custom_resources.list",
                json!({"namespace":"engineering","group":"monitoring.coreos.com","version":"v1","plural":"servicemonitors","limit":1}),
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(paged.paths(), [SERVICE_MONITORS]);
}

#[test]
fn a_custom_resource_configuration_outside_the_custom_resource_rules_is_refused() {
    let entry = |group: &str, version: &str, plural: &str| json!({"group":group,"version":version,"plural":plural});
    for entries in [
        // Built-in groups, the core group and an undotted group.
        vec![entry("", "v1", "secrets")],
        vec![entry("apps", "v1", "statefulsets")],
        vec![entry("k8s.io", "v1", "things")],
        vec![entry("rbac.authorization.k8s.io", "v1", "clusterroles")],
        vec![entry("kubernetes.io", "v1", "things")],
        vec![entry("metrics.kubernetes.io", "v1beta1", "pods")],
        // Grammar: a path segment, upper case, an empty label.
        vec![entry("Monitoring.coreos.com", "v1", "servicemonitors")],
        vec![entry("monitoring.coreos.com", "v1/..", "servicemonitors")],
        vec![entry("monitoring.coreos.com", "v1", "service/monitors")],
        vec![entry("monitoring..coreos.com", "v1", "servicemonitors")],
        vec![entry("monitoring.coreos.com", "", "servicemonitors")],
        vec![entry("monitoring.coreos.com", "v1", "")],
        // A duplicate entry.
        vec![monitors(), monitors()],
        // More than 32 entries.
        (0..33)
            .map(|n| entry("example.com", "v1", &format!("things{n}")))
            .collect(),
    ] {
        assert!(
            configured(
                json!({"namespaces":["engineering"],"resource_kinds":["pods"],"custom_resources":entries}),
                vec![],
            )
            .is_err(),
            "{entries:?}"
        );
    }
}
