//! `kubernetes.workloads` on the personal-local placement, read through the operator's kubeconfig.
//!
//! The projection, the schemas, the description lease and the cursors all come from
//! `crate::workloads`, so a workload read here and a workload read in the cluster are the same
//! record with the same digest. What differs — and all that differs — is the credential: the
//! hosted receiver carries a mounted ServiceAccount token, this one carries whichever context the
//! operator activated, including an EKS exec plugin.

use async_trait::async_trait;
use kube::Client;
use protocol::datasource::{
    BindingSearchRequest, DatasourceError, DatasourceErrorCode, DatasourceRequest,
    DatasourceResult, DescribeRequest as DatasourceDescribeRequest,
};
use protocol::operation::{OperationError, OperationErrorCode};
use serde::de::DeserializeOwned;
use sha2::{Digest as _, Sha256};

use service::PrincipalContext;

use crate::workloads::{
    datasource_description, datasource_not_found, datasource_summary, datasource_unavailable,
    namespace_binding, namespace_binding_ref, project_compact, project_pod, read_workloads,
    safe_event_reason, unavailable, valid_dns_label, CursorStore, DeploymentReader,
    DeploymentStatus, KubernetesDeployment, KubernetesEvent, KubernetesList, KubernetesPod,
    now_unix_ms, outcome_unknown, project, stale, RestartAccepted, WarningSummary, WorkloadDetail,
    WorkloadList, DATASOURCE, MAX_KUBERNETES_RESPONSE_BYTES, MAX_RELATED_RECORDS,
};

/// Reads the Kubernetes API through a `kube::Client` bound to one kubeconfig context.
///
/// Requests go out as raw paths rather than through typed `Api<Deployment>` handles so the wire
/// types and the projection are literally the ones the hosted receiver uses — a typed client would
/// have meant a second mapping, and a second mapping is how two placements start disagreeing about
/// what a workload is. The client owns auth, TLS and the API server address, which is exactly the
/// part that is allowed to differ.
pub(crate) struct KubeconfigReader {
    client: Client,
}

impl KubeconfigReader {
    pub(crate) const fn new(client: Client) -> Self {
        Self { client }
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, DatasourceError> {
        let request = http::Request::get(path)
            .body(Vec::new())
            .map_err(|_| datasource_unavailable("Kubernetes API endpoint is invalid"))?;
        let body = self
            .client
            .request_text(request)
            .await
            .map_err(datasource_error)?;
        if body.len() > MAX_KUBERNETES_RESPONSE_BYTES {
            return Err(DatasourceError::new(
                DatasourceErrorCode::ResultTooLarge,
                "Kubernetes response exceeds the result bound",
                false,
            ));
        }
        serde_json::from_str(&body)
            .map_err(|_| datasource_unavailable("Kubernetes response is malformed"))
    }
}

/// A refusal a person can act on, from whatever the cluster or the credential helper said.
///
/// The API status code is the only part worth keeping: `403` from an EKS context usually means the
/// operator's IAM identity is not bound to a Role in this namespace, and reporting that as
/// "unavailable" sent people to look at the cluster's health instead of at their own grant.
fn datasource_error(error: kube::Error) -> DatasourceError {
    match error {
        kube::Error::Api(status) => match status.code {
            404 => datasource_not_found("Kubernetes workload was not found"),
            401 | 403 => DatasourceError::new(
                DatasourceErrorCode::NotGranted,
                format!(
                    "the active kubeconfig identity may not read this: {}",
                    status.message
                ),
                false,
            ),
            _ => datasource_unavailable("Kubernetes API returned a non-success response"),
        },
        _ => datasource_unavailable("Kubernetes API request failed"),
    }
}

/// The same fault, said as an operation rather than as a read.
fn operation_from_datasource(error: DatasourceError) -> OperationError {
    let code = match error.code {
        DatasourceErrorCode::NotFound => OperationErrorCode::NotFound,
        DatasourceErrorCode::NotGranted => OperationErrorCode::NotGranted,
        DatasourceErrorCode::InvalidInput => OperationErrorCode::InvalidInput,
        _ => OperationErrorCode::Unavailable,
    };
    OperationError::new(code, error.message, error.retriable)
}

/// A refused restart, classified by what the cluster said.
///
/// A conflict means the Deployment moved between the read and the patch, which is a stale-authority
/// refusal a caller fixes by reading again — not an unknown outcome. Anything the transport could
/// not classify stays `OutcomeUnknown`, because a patch that may have been applied must never be
/// reported as one that was not.
fn restart_error(error: kube::Error) -> OperationError {
    match error {
        kube::Error::Api(status) => match status.code {
            404 => OperationError::new(
                OperationErrorCode::NotFound,
                "Kubernetes Deployment was not found",
                false,
            ),
            401 | 403 => OperationError::new(
                OperationErrorCode::NotGranted,
                "the active kubeconfig identity may not restart this Deployment",
                false,
            ),
            409 | 422 => stale("Kubernetes Deployment changed before the restart patch"),
            _ => outcome_unknown("Kubernetes rollout restart outcome is unknown after dispatch"),
        },
        _ => outcome_unknown("Kubernetes rollout restart outcome is unknown after dispatch"),
    }
}

fn query(pairs: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

/// Names reaching a request path are always checked, never trusted.
///
/// A namespace arrives from this placement's configuration and a workload name from a datasource
/// key; neither is a credential, but both are interpolated into a URL path, and a name carrying a
/// `/` or a `..` would address a different resource than the one admitted.
fn path_segment(value: &str) -> Result<&str, DatasourceError> {
    if valid_dns_label(value, 253) {
        Ok(value)
    } else {
        Err(DatasourceError::new(
            DatasourceErrorCode::InvalidInput,
            "Kubernetes name is invalid",
            false,
        ))
    }
}

#[async_trait]
impl DeploymentReader for KubeconfigReader {
    async fn read(&self, namespace: &str, name: &str) -> Result<DeploymentStatus, OperationError> {
        let namespace = path_segment(namespace).map_err(operation_from_datasource)?;
        let name = path_segment(name).map_err(operation_from_datasource)?;
        let deployment: KubernetesDeployment = self
            .get_json(&format!(
                "/apis/apps/v1/namespaces/{namespace}/deployments/{name}"
            ))
            .await
            .map_err(operation_from_datasource)?;
        if deployment.metadata.namespace != namespace || deployment.metadata.name != name {
            return Err(unavailable("Kubernetes returned a different Deployment"));
        }
        Ok(project(deployment))
    }

    async fn list_workloads(
        &self,
        namespace: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<WorkloadList, DatasourceError> {
        let namespace = path_segment(namespace)?;
        let mut pairs = vec![("limit", limit.to_string())];
        if let Some(cursor) = cursor {
            pairs.push(("continue", cursor.to_owned()));
        }
        let borrowed = pairs
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect::<Vec<_>>();
        let path = format!(
            "/apis/apps/v1/namespaces/{namespace}/deployments?{}",
            query(&borrowed)
        );
        let list: KubernetesList<KubernetesDeployment> = self.get_json(&path).await?;
        let workloads = list.items.into_iter().map(project_compact).collect();
        Ok(WorkloadList {
            workloads,
            next_cursor: (!list.metadata.continue_token.is_empty())
                .then_some(list.metadata.continue_token),
        })
    }

    async fn workload_detail(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<WorkloadDetail, DatasourceError> {
        let namespace = path_segment(namespace)?;
        let name = path_segment(name)?;
        let deployment: KubernetesDeployment = self
            .get_json(&format!(
                "/apis/apps/v1/namespaces/{namespace}/deployments/{name}"
            ))
            .await?;
        if deployment.metadata.namespace != namespace || deployment.metadata.name != name {
            return Err(datasource_unavailable(
                "Kubernetes returned a different Deployment",
            ));
        }

        let selector = deployment
            .spec
            .selector
            .match_labels
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut related_complete = !selector.is_empty();
        let related_limit = (MAX_RELATED_RECORDS + 1).to_string();
        let pods = if selector.is_empty() {
            Vec::new()
        } else {
            let path = format!(
                "/api/v1/namespaces/{namespace}/pods?{}",
                query(&[("labelSelector", &selector), ("limit", &related_limit)])
            );
            let list: KubernetesList<KubernetesPod> = self.get_json(&path).await?;
            related_complete &=
                list.metadata.continue_token.is_empty() && list.items.len() <= MAX_RELATED_RECORDS;
            list.items
                .into_iter()
                .take(MAX_RELATED_RECORDS)
                .map(project_pod)
                .collect()
        };

        let field_selector = format!(
            "type=Warning,involvedObject.kind=Deployment,involvedObject.name={name}"
        );
        let path = format!(
            "/api/v1/namespaces/{namespace}/events?{}",
            query(&[
                ("fieldSelector", field_selector.as_str()),
                ("limit", &related_limit),
            ])
        );
        let events: KubernetesList<KubernetesEvent> = self.get_json(&path).await?;
        related_complete &=
            events.metadata.continue_token.is_empty() && events.items.len() <= MAX_RELATED_RECORDS;
        let warnings = events
            .items
            .into_iter()
            .take(MAX_RELATED_RECORDS)
            .filter(|event| safe_event_reason(&event.reason))
            .map(|event| WarningSummary {
                involved_kind: event.involved_object.kind,
                involved_name: event.involved_object.name,
                reason: event.reason,
                count: event.count,
                first_observed_at: event.first_timestamp,
                last_observed_at: event.last_timestamp,
            })
            .collect();

        Ok(WorkloadDetail {
            workload: project_compact(deployment),
            pods,
            warnings,
            related_complete,
        })
    }

    /// Restarts one rollout, and only the exact object the caller named.
    ///
    /// The uid and resourceVersion travel in the patch, so the API server refuses it outright if
    /// the Deployment changed since the caller read it. That is what makes a restart safe to
    /// approve against a cluster somebody else is also changing: the thing approved and the thing
    /// patched are provably the same object.
    async fn restart(
        &self,
        namespace: &str,
        name: &str,
        uid: &str,
        resource_version: &str,
    ) -> Result<RestartAccepted, OperationError> {
        let namespace = path_segment(namespace).map_err(operation_from_datasource)?;
        let name = path_segment(name).map_err(operation_from_datasource)?;
        let body = serde_json::to_vec(&serde_json::json!({
            "metadata": {"uid": uid, "resourceVersion": resource_version},
            "spec": {"template": {"metadata": {"annotations": {
                "kubectl.kubernetes.io/restartedAt": now_unix_ms().to_string()
            }}}}
        }))
        .map_err(|_| outcome_unknown("Kubernetes restart patch could not be encoded"))?;
        let request = http::Request::patch(format!(
            "/apis/apps/v1/namespaces/{namespace}/deployments/{name}"
        ))
        .header("content-type", "application/strategic-merge-patch+json")
        .body(body)
        .map_err(|_| unavailable("Kubernetes API endpoint is invalid"))?;
        let response = self
            .client
            .request_text(request)
            .await
            .map_err(restart_error)?;
        let deployment: KubernetesDeployment = serde_json::from_str(&response).map_err(|_| {
            outcome_unknown("Kubernetes restart response is malformed after dispatch")
        })?;
        if deployment.metadata.namespace != namespace
            || deployment.metadata.name != name
            || deployment.metadata.uid != uid
            || deployment.metadata.resource_version.is_empty()
        {
            return Err(OperationError::new(
                OperationErrorCode::OutcomeUnknown,
                "Kubernetes accepted a restart but returned unexpected authority",
                false,
            ));
        }
        Ok(RestartAccepted {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            uid: uid.to_owned(),
            resource_version: deployment.metadata.resource_version,
            patch_accepted: true,
        })
    }
}

/// The datasource half of the personal-local Kubernetes placement.
///
/// Admission here is the whole of personal-local admission: the daemon listens on a Unix socket
/// only its owner can open, and `check_context` has already established that the caller is that
/// owner. There is no group model to consult, which is why the namespace grant refusals the hosted
/// receiver builds have no counterpart — a namespace this placement lists is a namespace its
/// configuration named, and the cluster's own RBAC is the second gate.
#[derive(Default)]
pub(crate) struct WorkloadSurface {
    cursors: CursorStore,
}

impl WorkloadSurface {
    pub(crate) fn owns(request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Search(_) => true,
            DatasourceRequest::Describe(DatasourceDescribeRequest { datasource_ref })
            | DatasourceRequest::Bindings(BindingSearchRequest { datasource_ref, .. }) => {
                datasource_ref == DATASOURCE
            }
            DatasourceRequest::Read(read) => read.datasource_ref == DATASOURCE,
        }
    }

    /// Resolves a binding ref back to the namespace it names, refusing anything this placement
    /// did not offer.
    fn binding_namespace<'a>(
        namespaces: &'a [String],
        binding_ref: &str,
    ) -> Result<&'a str, DatasourceError> {
        namespaces
            .iter()
            .find(|namespace| namespace_binding_ref(namespace) == binding_ref)
            .map(String::as_str)
            .ok_or_else(|| {
                DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    format!(
                        "`{binding_ref}` is not a binding of `{DATASOURCE}`; list its bindings and read through one of those"
                    ),
                    false,
                )
            })
    }

    pub(crate) async fn handle(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
        connection: Option<(String, Client)>,
        namespaces: &[String],
    ) -> Result<DatasourceResult, DatasourceError> {
        let Some((connection_ref, client)) = connection else {
            // Not a defect and not a missing grant: nobody has activated a kubeconfig context in
            // this daemon generation yet, so there is no cluster to read. Retriable, because
            // activating one is a thing the person reading this can go and do.
            return Err(DatasourceError::new(
                DatasourceErrorCode::Unavailable,
                "no Kubernetes cluster is attached yet; activate a kubeconfig context first",
                true,
            ));
        };
        match request {
            DatasourceRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let definitions = if query.is_empty() || DATASOURCE.contains(&query) {
                    vec![datasource_summary()]
                } else {
                    Vec::new()
                };
                Ok(DatasourceResult::Search { definitions })
            }
            DatasourceRequest::Describe(DatasourceDescribeRequest { datasource_ref })
                if datasource_ref == DATASOURCE =>
            {
                Ok(DatasourceResult::Describe(datasource_description(context)))
            }
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref,
                query,
                limit,
            }) if datasource_ref == DATASOURCE => {
                let query = query.to_ascii_lowercase();
                let bindings = namespaces
                    .iter()
                    .filter(|namespace| query.is_empty() || namespace.contains(&query))
                    .take(usize::from(limit))
                    .map(|namespace| namespace_binding(&connection_ref, namespace))
                    .collect();
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Read(read) => {
                let namespace = Self::binding_namespace(namespaces, &read.binding_ref)?.to_owned();
                let reader = KubeconfigReader::new(client);
                let connector_audit_ref = audit_ref(context, &connection_ref, &namespace);
                read_workloads(
                    &reader,
                    &self.cursors,
                    context,
                    &namespace,
                    read,
                    connector_audit_ref,
                )
                .await
            }
            _ => Err(datasource_not_found("Kubernetes datasource was not found")),
        }
    }
}

fn audit_ref(context: &PrincipalContext, connection_ref: &str, namespace: &str) -> String {
    let digest = Sha256::digest(format!(
        "{}\0{DATASOURCE}\0{connection_ref}\0{namespace}",
        context.authority_snapshot_sha256()
    ));
    format!("audit:kubernetes-local:{}", hex::encode(&digest[..16]))
}
#[cfg(test)]
include!("local_workloads_tests.rs");
