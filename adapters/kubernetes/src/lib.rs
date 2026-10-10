use async_trait::async_trait;
use connectors_contracts::{EndpointObservation, Page};
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{
    Adapter, AuthenticatedHttp, Cursors, decode, encode, instance_descriptor, provenance,
    upstream_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub mod auth;
pub mod helm;

/// How much of a Helm release this composition may read. `Off` advertises no
/// release operation at all. `Metadata` advertises the two that read only
/// release-Secret metadata. `RedactedContent` additionally advertises the two
/// content reads, which disclose a projection of structure and digests. There
/// is deliberately no variant that discloses stored provider bytes: recorded
/// values routinely carry credentials, so the projection is the disclosure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmReleaseReads {
    #[default]
    Off,
    Metadata,
    RedactedContent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub namespaces: Vec<String>,
    pub resource_kinds: Vec<String>,
    #[serde(default)]
    pub discover_hosts: bool,
    #[serde(default)]
    pub helm_release_reads: HelmReleaseReads,
}
pub struct Kubernetes {
    http: Arc<dyn AuthenticatedHttp>,
    config: Config,
    descriptor: Descriptor,
    cursors: Arc<Cursors>,
    partition: Option<String>,
}

impl Kubernetes {
    pub fn new(
        instance: &str,
        config: Config,
        effective_configuration: Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        if config.namespaces.is_empty()
            || config.namespaces.iter().any(|n| {
                n.is_empty()
                    || n.len() > 63
                    || !n
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            || config
                .resource_kinds
                .iter()
                .any(|k| collection(k, "scope").is_err())
        {
            return Err(Error::invalid("invalid namespace or resource-kind scope"));
        }
        let mut descriptor = instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        connectors_sdk::verify_handlers(
            &descriptor,
            &[
                "resources.list",
                "resources.get",
                "namespaces.list",
                "deployments.history",
                "endpoints.discover",
                "hosts.discover",
                "helm_releases.history",
                "helm_releases.status",
                "helm_releases.values",
                "helm_releases.manifest",
            ],
        )?;
        if !config.discover_hosts {
            descriptor.operations.retain(|o| o.id != "hosts.discover");
        }
        // An operation whose disclosure the configuration did not admit is not
        // advertised at all, so a caller cannot describe it, and the separate
        // guard in `invoke` still refuses it.
        descriptor
            .operations
            .retain(|o| helm_admitted(config.helm_release_reads, &o.id));
        Ok(Self {
            http,
            config,
            descriptor,
            cursors: Arc::new(Cursors::default()),
            partition: None,
        })
    }
    /// A new immutable HTTP capability for one host-admitted use. Cursor state
    /// stays adapter-owned and is partitioned by an opaque authenticated binding,
    /// so a continuation issued under one connection is not readable under
    /// another. This value grants no credential lookup or dispatch authority.
    pub fn with_authenticated_http(
        &self,
        http: Arc<dyn AuthenticatedHttp>,
        partition: &str,
    ) -> Result<Self> {
        if !connectors_core::valid_id(partition) {
            return Err(Error::invalid("invalid cursor partition"));
        }
        Ok(Self {
            http,
            config: self.config.clone(),
            descriptor: self.descriptor.clone(),
            cursors: self.cursors.clone(),
            partition: Some(partition.to_owned()),
        })
    }
    /// One bounded page of a release's own Secrets, selected the way Helm's
    /// storage selects them. This never consults `resource_kinds`: a release
    /// read is its own admitted operation, not a resource kind a caller picks,
    /// and it can neither be reached by nor widen that closed enum.
    async fn releases(
        &self,
        operation: &str,
        namespace: &str,
        release: &str,
        deployed: bool,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<Page<helm::Revision>> {
        if !(1..=100).contains(&limit) {
            return Err(Error::invalid("limit must be between one and 100"));
        }
        if !helm::valid_release(release) {
            return Err(Error::invalid("release name is not a Helm release name"));
        }
        self.namespace(namespace)?;
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"namespace":namespace,"release":release,"deployed":deployed,"limit":limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        let mut query = vec![
            ("labelSelector", helm::selector(release, deployed)),
            ("limit", limit.to_string()),
        ];
        if let Some(cursor) = self.admitted_cursor(cursor)? {
            query.push(("continue", self.cursors.read(&context, cursor)?));
        }
        let response = self
            .http
            .get(&["api", "v1", "namespaces", namespace, "secrets"], &query)
            .await?;
        let value = upstream_json(&response)?;
        let items = value["items"].as_array().ok_or_else(|| {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "invalid Kubernetes list response",
            )
        })?;
        if items.len() > limit as usize {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes exceeded requested page size",
            ));
        }
        let revisions = items
            .iter()
            .map(|item| helm::revision(item, namespace, release))
            .collect::<Result<Vec<_>>>()?;
        let (next_cursor, complete, revision) = self.envelope(&context, &value)?;
        Ok(Page {
            items: revisions,
            complete,
            next_cursor,
            provenance: provenance(
                &self.descriptor.instance,
                format!(
                    "{namespace}/helm-releases/{release}{}",
                    if deployed { "/deployed" } else { "" }
                ),
                revision,
            ),
        })
    }
    /// Read one exact release Secret by the name Helm's storage gives it. The
    /// admitted target is that single object, not the namespace's Secrets.
    async fn release_body(
        &self,
        namespace: &str,
        release: &str,
        revision: u32,
    ) -> Result<(String, Option<String>, Value)> {
        if !helm::valid_release(release) {
            return Err(Error::invalid("release name is not a Helm release name"));
        }
        self.namespace(namespace)?;
        let name = helm::object_name(release, revision);
        let response = self
            .http
            .get(
                &["api", "v1", "namespaces", namespace, "secrets", &name],
                &[],
            )
            .await?;
        let secret = upstream_json(&response)?;
        // The object is checked as a release record before its payload is
        // decoded, so a foreign object under that name is never decompressed.
        let record = helm::revision(&secret, namespace, release)?;
        if record.revision != revision {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "release Secret reports a different revision",
            ));
        }
        let body = helm::body(&secret)?;
        // The provenance is built from the object's labels, so a body naming a
        // different release, revision or namespace must not be served under
        // that identity.
        helm::check_identity(&body, &record)?;
        Ok((name, record.source_revision, body))
    }
    /// Read the list response's own envelope: the continuation to issue and the
    /// collection revision to attribute the page to.
    ///
    /// A continuation the provider sent in a shape this binding has not
    /// established is not the absence of a continuation, and neither is one
    /// whose issued cursor would exceed the bound the published output schema
    /// declares for `next_cursor`. In both cases the provider said it had not
    /// finished, so the page reports `complete: false` with a null cursor:
    /// truthfully partial, and honest that it cannot hand back a way to
    /// continue. Defaulting either to the empty token would report
    /// `complete: true` over a selection the provider said it had not
    /// finished — the same unsound completeness claim the Helm contract
    /// refuses for a foreign labelled object, one layer up.
    ///
    /// The collection's own resourceVersion is different: a page has no
    /// vocabulary for "the revision is unreadable", and reporting it as absent
    /// would attribute the page to no observed revision. That refuses.
    fn envelope(
        &self,
        context: &Value,
        value: &Value,
    ) -> Result<(Option<String>, bool, Option<String>)> {
        let metadata = &value["metadata"];
        if !metadata.is_object() {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes list response has no metadata",
            ));
        }
        let revision = helm::optional_string(metadata, "resourceVersion")
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UpstreamProtocol,
                    "Kubernetes collection revision is not a string",
                )
            })?
            .map(str::to_owned);
        // `Partial` is "the provider continued and this binding cannot carry
        // the continuation", which is a page that is not complete rather than
        // a page that is refused.
        enum Carried {
            None,
            Partial,
            Cursor(String),
        }
        let carried = match helm::optional_string(metadata, "continue") {
            None => Carried::Partial,
            Some(None) => Carried::None,
            Some(Some("")) => Carried::None,
            Some(Some(token)) => {
                let cursor = self.cursors.issue(context, token.to_owned())?;
                if cursor.len() > 16384 {
                    Carried::Partial
                } else {
                    Carried::Cursor(cursor)
                }
            }
        };
        Ok(match carried {
            Carried::None => (None, true, revision),
            Carried::Partial => (None, false, revision),
            Carried::Cursor(cursor) => (Some(cursor), false, revision),
        })
    }
    /// `cursor` is declared `maxLength 16384` on every paging input. An
    /// over-long value is refused today only as a side effect of cursor
    /// verification failing; the enumeration of declared bounds asks for the
    /// bound itself to be checked by the layer that publishes it.
    fn admitted_cursor<'a>(&self, cursor: Option<&'a str>) -> Result<Option<&'a str>> {
        if cursor.is_some_and(|cursor| cursor.len() > 16384) {
            return Err(Error::invalid("cursor exceeds its declared bound"));
        }
        Ok(cursor)
    }
    fn namespace(&self, namespace: &str) -> Result<()> {
        if !self.config.namespaces.iter().any(|n| n == namespace) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "namespace is outside configured scope",
            ));
        }
        Ok(())
    }
    async fn list(
        &self,
        operation: &str,
        namespace: &str,
        kind: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<Page> {
        if !(1..=100).contains(&limit) {
            return Err(Error::invalid("limit must be between one and 100"));
        }
        if kind != "nodes" {
            self.namespace(namespace)?;
        }
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"namespace":namespace,"kind":kind,"limit":limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        let mut query = vec![("limit", limit.to_string())];
        if let Some(cursor) = self.admitted_cursor(cursor)? {
            query.push(("continue", self.cursors.read(&context, cursor)?));
        }
        let segments = if kind == "nodes" {
            vec!["api", "v1", "nodes"]
        } else {
            collection(kind, namespace)?
        };
        let response = self.http.get(&segments, &query).await?;
        let value = upstream_json(&response)?;
        let items = value["items"]
            .as_array()
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UpstreamProtocol,
                    "invalid Kubernetes list response",
                )
            })?
            .clone();
        if items.len() > limit as usize {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes exceeded requested page size",
            ));
        }
        let (next_cursor, complete, revision) = self.envelope(&context, &value)?;
        Ok(Page {
            items,
            complete,
            next_cursor,
            provenance: provenance(
                &self.descriptor.instance,
                format!("{namespace}/{kind}"),
                revision,
            ),
        })
    }
    fn kind(&self, kind: &str) -> Result<()> {
        if !self.config.resource_kinds.iter().any(|k| k == kind) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "resource kind is outside configured scope",
            ));
        }
        Ok(())
    }
    /// Read one object of an admitted kind by name. The scope checks precede
    /// the name check, and both precede the one provider request, so a name
    /// outside the configured namespaces is refused, never looked up. An absent
    /// object is the provider's 404, which answers not_found rather than an
    /// empty page.
    async fn get(&self, namespace: &str, kind: &str, name: &str) -> Result<Page> {
        self.namespace(namespace)?;
        self.kind(kind)?;
        if !valid_name(name) {
            return Err(Error::invalid("name is not a Kubernetes object name"));
        }
        let mut segments = collection(kind, namespace)?;
        segments.push(name);
        let response = self.http.get(&segments, &[]).await?;
        let object = upstream_json(&response)?;
        let revision = identity(&object, namespace, name)?;
        Ok(Page {
            items: vec![object],
            complete: true,
            next_cursor: None,
            provenance: provenance(
                &self.descriptor.instance,
                format!("{namespace}/{kind}/{name}"),
                revision,
            ),
        })
    }
    /// The configured namespaces that exist, each read by its exact name. The
    /// cluster-scoped namespace collection is never listed, so no namespace
    /// outside the configured set can be observed. A 404 is an absent
    /// namespace and is omitted; any other refusal refuses the page, because a
    /// denied namespace is not evidence that it does not exist. The
    /// continuation is a position in the configured list, bound to that list.
    async fn configured_namespaces(
        &self,
        operation: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<Page> {
        if !(1..=100).contains(&limit) {
            return Err(Error::invalid("limit must be between one and 100"));
        }
        let names = &self.config.namespaces;
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"namespaces":names,"limit":limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        let start = match self.admitted_cursor(cursor)? {
            Some(cursor) => self
                .cursors
                .read(&context, cursor)?
                .parse::<usize>()
                .ok()
                .filter(|start| *start < names.len())
                .ok_or_else(|| {
                    Error::new(
                        ErrorCode::StaleCursor,
                        "cursor expired or does not belong to this request",
                    )
                })?,
            None => 0,
        };
        let end = (start + limit as usize).min(names.len());
        let mut items = Vec::new();
        for name in &names[start..end] {
            let response = self
                .http
                .get(&["api", "v1", "namespaces", name.as_str()], &[])
                .await?;
            if response.status == 404 {
                continue;
            }
            let object = upstream_json(&response)?;
            if object["metadata"]["name"].as_str() != Some(name.as_str()) {
                return Err(Error::new(
                    ErrorCode::UpstreamProtocol,
                    "Kubernetes namespace reports a different name",
                ));
            }
            items.push(object);
        }
        let next_cursor = if end < names.len() {
            Some(self.cursors.issue(&context, end.to_string())?)
        } else {
            None
        };
        Ok(Page {
            items,
            complete: next_cursor.is_none(),
            next_cursor,
            provenance: provenance(&self.descriptor.instance, "namespaces", None),
        })
    }
    /// One bounded page of a Deployment's rollout history: the ReplicaSets its
    /// own selector matches, kept only when the Deployment is their controlling
    /// owner by uid. A label match alone is not ownership. The Deployment's uid
    /// binds the continuation, so a recreated Deployment of the same name
    /// cannot resume its predecessor's page.
    async fn history(
        &self,
        operation: &str,
        namespace: &str,
        name: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<Page> {
        if !(1..=100).contains(&limit) {
            return Err(Error::invalid("limit must be between one and 100"));
        }
        self.namespace(namespace)?;
        self.kind("deployments")?;
        self.kind("replicasets")?;
        if !valid_name(name) {
            return Err(Error::invalid("name is not a Kubernetes object name"));
        }
        let cursor = self.admitted_cursor(cursor)?;
        let response = self
            .http
            .get(
                &[
                    "apis",
                    "apps",
                    "v1",
                    "namespaces",
                    namespace,
                    "deployments",
                    name,
                ],
                &[],
            )
            .await?;
        let deployment = upstream_json(&response)?;
        identity(&deployment, namespace, name)?;
        let uid = deployment["metadata"]["uid"]
            .as_str()
            .filter(|uid| !uid.is_empty())
            .ok_or_else(|| Error::new(ErrorCode::UpstreamProtocol, "Deployment lacks identity"))?
            .to_owned();
        let selector = label_selector(&deployment["spec"]["selector"])?;
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"namespace":namespace,"deployment":name,"uid":uid,"limit":limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        let mut query = vec![("labelSelector", selector), ("limit", limit.to_string())];
        if let Some(cursor) = cursor {
            query.push(("continue", self.cursors.read(&context, cursor)?));
        }
        let response = self
            .http
            .get(&collection("replicasets", namespace)?, &query)
            .await?;
        let value = upstream_json(&response)?;
        let listed = value["items"].as_array().ok_or_else(|| {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "invalid Kubernetes list response",
            )
        })?;
        if listed.len() > limit as usize {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes exceeded requested page size",
            ));
        }
        let items = listed
            .iter()
            .filter(|replicaset| controlled_by(replicaset, &uid))
            .cloned()
            .collect();
        let (next_cursor, complete, revision) = self.envelope(&context, &value)?;
        Ok(Page {
            items,
            complete,
            next_cursor,
            provenance: provenance(
                &self.descriptor.instance,
                format!("{namespace}/deployments/{name}/history"),
                revision,
            ),
        })
    }
}

/// The one collection path each admissible kind binds. This match is the
/// closed set of kinds: configuration validation and dispatch both read it.
fn collection<'a>(kind: &str, namespace: &'a str) -> Result<Vec<&'a str>> {
    Ok(match kind {
        "pods" => vec!["api", "v1", "namespaces", namespace, "pods"],
        "services" => vec!["api", "v1", "namespaces", namespace, "services"],
        "events" => vec!["api", "v1", "namespaces", namespace, "events"],
        "deployments" => vec!["apis", "apps", "v1", "namespaces", namespace, "deployments"],
        "replicasets" => vec!["apis", "apps", "v1", "namespaces", namespace, "replicasets"],
        "endpointslices" => vec![
            "apis",
            "discovery.k8s.io",
            "v1",
            "namespaces",
            namespace,
            "endpointslices",
        ],
        _ => {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "resource kind not supported",
            ));
        }
    })
}

/// A DNS-1123 subdomain: the name rule of every kind this adapter reads.
fn valid_name(name: &str) -> bool {
    name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// A label value, and the name half of a label key.
fn label_value(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.len() <= 63
        && (bytes.is_empty()
            || (bytes[0].is_ascii_alphanumeric()
                && bytes[bytes.len() - 1].is_ascii_alphanumeric()
                && bytes
                    .iter()
                    .all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))))
}

fn label_key(key: &str) -> bool {
    let (prefix, name) = match key.split_once('/') {
        Some((prefix, name)) => (Some(prefix), name),
        None => (None, key),
    };
    prefix.is_none_or(valid_name) && !name.is_empty() && label_value(name)
}

/// Translate a Deployment's `spec.selector` to the label-selector query that
/// selects the same objects. Every key and value is checked against the label
/// grammar first, so no provider string can add a term. An empty selector
/// would select every ReplicaSet in the namespace; apps/v1 forbids one, so it
/// refuses rather than widening the read.
fn label_selector(selector: &Value) -> Result<String> {
    fn refused() -> Error {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "Deployment selector is not a label selector this binding can carry",
        )
    }
    if !selector.is_object() {
        return Err(refused());
    }
    let mut terms = Vec::new();
    match &selector["matchLabels"] {
        Value::Null => {}
        Value::Object(labels) => {
            for (key, value) in labels {
                let value = value.as_str().ok_or_else(refused)?;
                if !label_key(key) || !label_value(value) {
                    return Err(refused());
                }
                terms.push(format!("{key}={value}"));
            }
        }
        _ => return Err(refused()),
    }
    match &selector["matchExpressions"] {
        Value::Null => {}
        Value::Array(expressions) => {
            for expression in expressions {
                let key = expression["key"]
                    .as_str()
                    .filter(|key| label_key(key))
                    .ok_or_else(refused)?;
                let values = || -> Result<String> {
                    let values = expression["values"]
                        .as_array()
                        .filter(|values| !values.is_empty())
                        .ok_or_else(refused)?;
                    let values = values
                        .iter()
                        .map(|value| {
                            value
                                .as_str()
                                .filter(|value| label_value(value))
                                .ok_or_else(refused)
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(values.join(","))
                };
                terms.push(match expression["operator"].as_str() {
                    Some("In") => format!("{key} in ({})", values()?),
                    Some("NotIn") => format!("{key} notin ({})", values()?),
                    Some("Exists") => key.to_owned(),
                    Some("DoesNotExist") => format!("!{key}"),
                    _ => return Err(refused()),
                });
            }
        }
        _ => return Err(refused()),
    }
    if terms.is_empty() {
        return Err(refused());
    }
    Ok(terms.join(","))
}

/// The Deployment is a ReplicaSet's controlling owner, by uid.
fn controlled_by(object: &Value, uid: &str) -> bool {
    object["metadata"]["ownerReferences"]
        .as_array()
        .is_some_and(|owners| {
            owners.iter().any(|owner| {
                owner["controller"] == true && owner["kind"] == "Deployment" && owner["uid"] == uid
            })
        })
}

/// A single-object read must answer for the object it named: an object
/// reporting another name or namespace is a protocol violation, not that
/// object. Returns the object's own resourceVersion.
fn identity(object: &Value, namespace: &str, name: &str) -> Result<Option<String>> {
    let metadata = &object["metadata"];
    if metadata["name"].as_str() != Some(name) || metadata["namespace"].as_str() != Some(namespace)
    {
        return Err(Error::new(
            ErrorCode::UpstreamProtocol,
            "Kubernetes object reports a different identity",
        ));
    }
    Ok(helm::optional_string(metadata, "resourceVersion")
        .ok_or_else(|| {
            Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes object revision is not a string",
            )
        })?
        .map(str::to_owned))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    namespace: String,
    kind: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    namespace: String,
    kind: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Namespaces {
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    namespace: String,
    name: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoints {
    namespace: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hosts {
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Releases {
    namespace: String,
    release: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseContent {
    namespace: String,
    release: String,
    revision: u32,
    limit: u16,
}

/// The advertised set for a disclosure mode. Metadata reads never carry a
/// stored value, so they are admitted one step below the content reads.
fn helm_admitted(mode: HelmReleaseReads, operation: &str) -> bool {
    match operation {
        "helm_releases.history" | "helm_releases.status" => mode != HelmReleaseReads::Off,
        "helm_releases.values" | "helm_releases.manifest" => {
            mode == HelmReleaseReads::RedactedContent
        }
        _ => true,
    }
}

#[async_trait]
impl Adapter for Kubernetes {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        // The descriptor already omits an unadmitted release operation; this
        // repeats the decision at dispatch so a stale description grants
        // nothing, exactly as host discovery does below.
        if !helm_admitted(self.config.helm_release_reads, operation) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Helm release disclosure is not configured for this operation",
            ));
        }
        match operation {
            "resources.list" => {
                let args: Resources = decode(input)?;
                if !self.config.resource_kinds.contains(&args.kind) {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        "resource kind is outside configured scope",
                    ));
                }
                encode(
                    self.list(
                        operation,
                        &args.namespace,
                        &args.kind,
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?,
                )
            }
            "resources.get" => {
                let args: Object = decode(input)?;
                encode(self.get(&args.namespace, &args.kind, &args.name).await?)
            }
            "namespaces.list" => {
                let args: Namespaces = decode(input)?;
                encode(
                    self.configured_namespaces(operation, args.limit, args.cursor.as_deref())
                        .await?,
                )
            }
            "deployments.history" => {
                let args: History = decode(input)?;
                encode(
                    self.history(
                        operation,
                        &args.namespace,
                        &args.name,
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?,
                )
            }
            "endpoints.discover" => {
                let args: Endpoints = decode(input)?;
                let page = self
                    .list(
                        operation,
                        &args.namespace,
                        "endpointslices",
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?;
                let mut endpoints = Vec::new();
                for slice in &page.items {
                    let uid = slice["metadata"]["uid"].as_str().ok_or_else(|| {
                        Error::new(ErrorCode::UpstreamProtocol, "EndpointSlice lacks identity")
                    })?;
                    let service = slice["metadata"]["labels"]["kubernetes.io/service-name"]
                        .as_str()
                        .map(str::to_owned);
                    // A Service with no ready backends is ordinary Kubernetes,
                    // and its EndpointSlice serialises both collections as an
                    // explicit null rather than an empty array. Null therefore
                    // means "no observations from this slice", not a malformed
                    // document; any other non-array value is still a protocol
                    // violation. Observed on Kubernetes v1.31.5 with a Service
                    // whose selector matched no pod.
                    let empty = Vec::new();
                    let collection = |name: &str| -> Result<&Vec<Value>> {
                        match &slice[name] {
                            Value::Null => Ok(&empty),
                            value => value.as_array().ok_or_else(|| {
                                Error::new(
                                    ErrorCode::UpstreamProtocol,
                                    "EndpointSlice collection is not a list",
                                )
                            }),
                        }
                    };
                    let ports = collection("ports")?;
                    let entries = collection("endpoints")?;
                    for entry in entries {
                        let addresses = entry["addresses"].as_array().ok_or_else(|| {
                            Error::new(ErrorCode::UpstreamProtocol, "endpoint lacks addresses")
                        })?;
                        for address in addresses {
                            let address = address.as_str().ok_or_else(|| {
                                Error::new(ErrorCode::UpstreamProtocol, "invalid endpoint address")
                            })?;
                            for port in ports {
                                let Some(number) = port["port"].as_u64() else {
                                    continue;
                                };
                                let number = u16::try_from(number).map_err(|_| {
                                    Error::new(ErrorCode::UpstreamProtocol, "invalid endpoint port")
                                })?;
                                if number == 0 {
                                    return Err(Error::new(
                                        ErrorCode::UpstreamProtocol,
                                        "invalid endpoint port",
                                    ));
                                }
                                if endpoints.len() >= 4096 {
                                    return Err(Error::new(
                                        ErrorCode::Capacity,
                                        "endpoint expansion exceeds page budget; request fewer slices",
                                    ));
                                }
                                endpoints.push(EndpointObservation {
                                    id: format!(
                                        "{uid}/{address}/{number}/{}",
                                        port["protocol"].as_str().unwrap_or("TCP")
                                    ),
                                    namespace: args.namespace.clone(),
                                    service: service.clone(),
                                    address: address.into(),
                                    port: number,
                                    transport: port["protocol"].as_str().unwrap_or("TCP").into(),
                                    application_protocol: port["appProtocol"]
                                        .as_str()
                                        .map(str::to_owned),
                                    ready: entry["conditions"]["ready"].as_bool(),
                                    source_uid: uid.into(),
                                    reachability: "source_cluster_network".into(),
                                });
                            }
                        }
                    }
                }
                encode(Page {
                    items: endpoints,
                    next_cursor: page.next_cursor,
                    complete: page.complete,
                    provenance: page.provenance,
                })
            }
            "hosts.discover" => {
                if !self.config.discover_hosts {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        "host discovery is disabled",
                    ));
                }
                let args: Hosts = decode(input)?;
                let mut page = self
                    .list(operation, "", "nodes", args.limit, args.cursor.as_deref())
                    .await?;
                page.items=page.items.into_iter().map(|node|json!({"id":node["metadata"]["uid"],"name":node["metadata"]["name"],"addresses":node["status"]["addresses"],"conditions":node["status"]["conditions"],"source_revision":node["metadata"]["resourceVersion"]})).collect();
                encode(page)
            }
            "helm_releases.history" | "helm_releases.status" => {
                let args: Releases = decode(input)?;
                encode(
                    self.releases(
                        operation,
                        &args.namespace,
                        &args.release,
                        operation == "helm_releases.status",
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?,
                )
            }
            "helm_releases.values" | "helm_releases.manifest" => {
                let args: ReleaseContent = decode(input)?;
                // `revision` is declared `minimum 1, maximum 2147483647` —
                // Helm's own `Version int` domain. The host validates input
                // against that schema before dispatch, but a bound only an
                // outer layer enforces is not enforced by this binding.
                if !(1..=500).contains(&args.limit) || !(1..=2_147_483_647).contains(&args.revision)
                {
                    return Err(Error::invalid("invalid release revision or page limit"));
                }
                let (name, source_revision, body) = self
                    .release_body(&args.namespace, &args.release, args.revision)
                    .await?;
                let limit = args.limit as usize;
                let values = operation == "helm_releases.values";
                let (items, complete) = if values {
                    let (items, complete) = helm::recorded_values(&name, &body, limit)?;
                    (encode(items)?, complete)
                } else {
                    let (items, complete) = helm::manifest_documents(&name, &body, limit)?;
                    (encode(items)?, complete)
                };
                let items = match items {
                    Value::Array(items) => items,
                    _ => return Err(Error::internal()),
                };
                encode(Page {
                    items,
                    // The projection has no provider continuation: it is one
                    // stored object, not a provider collection. A cursor here
                    // would be manufactured, so incompleteness is reported
                    // through `complete` alone.
                    next_cursor: None,
                    complete,
                    provenance: provenance(
                        &self.descriptor.instance,
                        format!(
                            "{}/{name}/{}",
                            args.namespace,
                            if values { "values" } else { "manifest" }
                        ),
                        source_revision,
                    ),
                })
            }
            _ => Err(Error::new(
                ErrorCode::NotFound,
                "operation is not implemented",
            )),
        }
    }
}
