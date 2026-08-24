//! The Kubernetes database-endpoint discovery projection (S-059, S-062, design 15).
//!
//! The deployment's database inventory is Crossplane provider-sql managed resources —
//! `databases.mysql.sql.crossplane.io` and `databases.postgresql.sql.crossplane.io` — and both
//! CRDs are CLUSTER-scoped: a Database carries no namespace of its own, and its spec declares no
//! endpoint facts at all, only the deletion policy and a `providerConfigRef`. The connection
//! facts live behind that referenced (equally cluster-scoped) ProviderConfig, whose
//! `credentials.connectionSecretRef` names the server connection Secret and the namespace it
//! lives in. This module is the host-mode-neutral half of reading that surface: the wire types,
//! the join from a Database to its ProviderConfig's secret reference, the projection into
//! `DatabaseEndpoint` descriptors, the JSON Schemas, the projection digest, the description
//! lease, the binding refs and the merged two-engine listing. Like `crate::workloads`, admission
//! stays with the placement: `DeploymentReader` is the one seam, and the hosted receiver decides
//! who may read which namespace before `read_database_endpoints` runs.
//!
//! The namespace gate over a cluster-scoped inventory is the connection secret's namespace: a
//! binding is one admitted namespace, and it lists exactly the Databases whose ProviderConfig
//! keeps its connection Secret there. A Database whose secret lives elsewhere — or whose
//! `providerConfigRef` resolves to nothing — associates with no binding and is excluded, not
//! erred (see the S-062 story notes for the decision).
//!
//! Discovery publishes requirements and references, never secret bytes (design 15). A descriptor
//! names the engine (derived from the API group, never read from the resource body), the
//! resource, the ProviderConfig it binds, the connection Secret by NAME and namespace, and
//! readiness. No code path in this crate ever reads a Secret value — endpoint resolution stays
//! with the SQL driver's credential custody at connect time.

use std::collections::BTreeMap;

use protocol::datasource::{
    AccessMode, Completeness, DatasourceBinding, DatasourceDescription, DatasourceError,
    DatasourceErrorCode, DatasourcePage, DatasourceProvenance, DatasourceRead, DatasourceRecord,
    DatasourceResult, DatasourceSummary, ReadRequest, ReadVerb, RecordView,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest as _, Sha256};

use service::PrincipalContext;

use crate::workloads::{
    datasource_not_found, datasource_unavailable, now_unix_ms, valid_dns_label, CursorStore,
    DeploymentReader, KubernetesCondition,
};

pub(crate) const DATABASES_DATASOURCE: &str = "kubernetes.databases";

/// The served version of the Crossplane provider-sql CRDs — the Database collections
/// (`databases.mysql.sql.crossplane.io`, `databases.postgresql.sql.crossplane.io`) and the
/// ProviderConfigs living in the same groups. crossplane-contrib/provider-sql serves `v1alpha1`
/// for both groups, and that is what the target cluster runs today; if the cluster moves to a
/// newer served version, this constant follows it in one place.
pub(crate) const DATABASE_CRD_VERSION: &str = "v1alpha1";

/// The two discovered engines, in listing order: pages walk MySQL first, then PostgreSQL, so a
/// merged listing is deterministic without buffering both inventories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseEngine {
    Mysql,
    Postgresql,
}

impl DatabaseEngine {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Mysql => "mysql",
            Self::Postgresql => "postgresql",
        }
    }

    /// The API group the engine's Database custom resources live under. The engine IS the
    /// group's first label, which is why discovery derives it from the endpoint asked and never
    /// from the resource body.
    pub(crate) fn group(self) -> &'static str {
        match self {
            Self::Mysql => "mysql.sql.crossplane.io",
            Self::Postgresql => "postgresql.sql.crossplane.io",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "mysql" => Some(Self::Mysql),
            "postgresql" => Some(Self::Postgresql),
            _ => None,
        }
    }

    /// The engine listed after this one, or `None` when the merged listing is over.
    fn following(self) -> Option<Self> {
        match self {
            Self::Mysql => Some(Self::Postgresql),
            Self::Postgresql => None,
        }
    }
}

/// One Crossplane Database managed resource, as the cluster serves it: cluster-scoped, so its
/// metadata carries no namespace. Only the fields the projection may publish are read;
/// annotations, labels, provider plumbing and anything credential-shaped never deserialize at
/// all.
#[derive(Deserialize)]
pub(crate) struct CrossplaneDatabase {
    pub(crate) metadata: KubernetesClusterMetadata,
    #[serde(default)]
    pub(crate) spec: CrossplaneDatabaseSpec,
    #[serde(default)]
    pub(crate) status: CrossplaneDatabaseStatus,
}

/// Cluster-scoped resource metadata: the name is the identity, and there is no namespace to
/// read. `crate::workloads::KubernetesMetadata` requires one and would refuse the real
/// resources for lacking it — the exact wrong-shape failure S-062 fixes.
#[derive(Deserialize)]
pub(crate) struct KubernetesClusterMetadata {
    pub(crate) name: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrossplaneDatabaseSpec {
    /// The ProviderConfig this resource binds. The live resources always carry it (the CRD
    /// schema defaults it); a resource without one binds no ProviderConfig, resolves no secret
    /// reference, and therefore associates with no namespace.
    #[serde(default)]
    pub(crate) provider_config_ref: Option<CrossplaneProviderConfigRef>,
}

#[derive(Deserialize)]
pub(crate) struct CrossplaneProviderConfigRef {
    pub(crate) name: String,
}

/// A connection-secret reference as the cluster declares it: names only, never bytes. The
/// platform's custody (S-058) resolves it; discovery must not, and no code path in this crate
/// reads a Secret.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CrossplaneSecretRef {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) namespace: Option<String>,
}

#[derive(Default, Deserialize)]
pub(crate) struct CrossplaneDatabaseStatus {
    #[serde(default)]
    pub(crate) conditions: Vec<KubernetesCondition>,
}

/// One Crossplane ProviderConfig, as the cluster serves it: cluster-scoped, and the only fact
/// discovery may read is which Secret its credentials come from — the reference, never the
/// acquisition mechanics (`source`) and never the bytes.
#[derive(Deserialize)]
pub(crate) struct CrossplaneProviderConfig {
    pub(crate) metadata: KubernetesClusterMetadata,
    #[serde(default)]
    pub(crate) spec: CrossplaneProviderConfigSpec,
}

#[derive(Default, Deserialize)]
pub(crate) struct CrossplaneProviderConfigSpec {
    #[serde(default)]
    pub(crate) credentials: CrossplaneProviderCredentials,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrossplaneProviderCredentials {
    #[serde(default)]
    pub(crate) connection_secret_ref: Option<CrossplaneSecretRef>,
}

/// One engine's page of Database resources, as the reader fetched it.
pub(crate) struct DatabaseList {
    pub(crate) items: Vec<CrossplaneDatabase>,
    /// The provider's own `continue` token for this engine's list; `None` when the engine is
    /// exhausted — or when its API group is absent from the cluster, which discovery treats as
    /// an empty inventory.
    pub(crate) next_cursor: Option<String>,
}

/// One API group-version discovery document (`/apis/{group}/{version}`), reduced to the fact
/// discovery needs: which collections the group serves.
#[derive(Default, Deserialize)]
pub(crate) struct KubernetesApiResourceList {
    #[serde(default)]
    pub(crate) resources: Vec<KubernetesApiResource>,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesApiResource {
    pub(crate) name: String,
}

/// Decides what a 404 on one of the group's collection lists means, from the group's own
/// discovery document (`None` when the group itself is not served): `Ok` says the inventory is
/// genuinely empty, `Err` refuses to pretend.
///
/// A cluster without the Crossplane provider serves no such group at all — an empty inventory,
/// not a failure. But when the group IS served and its discovery document lists the collection,
/// a 404 can only mean the read path is wrong: the namespaced-path bug S-062 fixes shipped
/// silently as "0 records" precisely because every 404 was assumed benign.
pub(crate) fn absent_collection_is_empty(
    group: &str,
    collection: &str,
    discovery: Option<&KubernetesApiResourceList>,
) -> Result<(), DatasourceError> {
    let served = discovery.is_some_and(|document| {
        document
            .resources
            .iter()
            .any(|resource| resource.name == collection)
    });
    if served {
        return Err(DatasourceError::new(
            DatasourceErrorCode::Unavailable,
            format!(
                "Kubernetes serves the {group} API group and lists `{collection}`, yet answered its cluster-scoped list with 404; this is a wrong read path, not an empty inventory"
            ),
            true,
        ));
    }
    Ok(())
}

/// The published endpoint descriptor: what S-058's connections consume. Every field is present:
/// a descriptor exists only where the resource associates with the read namespace, and a
/// resource whose facts cannot be resolved is excluded rather than published with guesses.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DatabaseEndpoint {
    pub(crate) engine: String,
    pub(crate) name: String,
    /// The cluster-scoped ProviderConfig the resource binds — its NAME, never its contents.
    pub(crate) provider_config: String,
    pub(crate) secret_ref: DatabaseSecretRef,
    pub(crate) ready: bool,
}

/// The published connection-secret reference: name and namespace, both required — the
/// namespace IS the binding association, so a descriptor without one cannot exist.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DatabaseSecretRef {
    pub(crate) name: String,
    pub(crate) namespace: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DatabaseEndpointKey {
    pub(crate) engine: String,
    pub(crate) name: String,
}

/// The ProviderConfig join map for one engine group: config name → its connection-secret
/// reference. A config without a secret reference resolves nothing and is dropped here, so a
/// Database binding it stays unassociated.
pub(crate) fn provider_config_secrets(
    configs: Vec<CrossplaneProviderConfig>,
) -> BTreeMap<String, CrossplaneSecretRef> {
    configs
        .into_iter()
        .filter_map(|config| {
            let secret = config.spec.credentials.connection_secret_ref?;
            Some((config.metadata.name, secret))
        })
        .collect()
}

/// Projects one managed resource into its endpoint descriptor, or `None` when it does not
/// associate with the namespace being read. The association is the referenced ProviderConfig's
/// connection-secret namespace: a resource whose reference dangles, or whose secret lives in
/// another namespace, belongs to no binding rather than to a guessed one. `ready` is the
/// Crossplane `Ready` condition and nothing subtler.
pub(crate) fn project_database_endpoint(
    engine: DatabaseEngine,
    database: CrossplaneDatabase,
    provider_configs: &BTreeMap<String, CrossplaneSecretRef>,
    namespace: &str,
) -> Option<DatabaseEndpoint> {
    let provider_config = database.spec.provider_config_ref?.name;
    let secret = provider_configs.get(&provider_config)?;
    let secret_namespace = secret.namespace.as_deref()?;
    if secret_namespace != namespace {
        return None;
    }
    let ready = database
        .status
        .conditions
        .iter()
        .any(|condition| condition.kind == "Ready" && condition.status == "True");
    Some(DatabaseEndpoint {
        engine: engine.as_str().to_owned(),
        name: database.metadata.name,
        provider_config,
        secret_ref: DatabaseSecretRef {
            name: secret.name.clone(),
            namespace: secret_namespace.to_owned(),
        },
        ready,
    })
}

pub(crate) fn databases_summary() -> DatasourceSummary {
    DatasourceSummary {
        datasource_ref: DATABASES_DATASOURCE.to_owned(),
        title: "Kubernetes database endpoints".to_owned(),
        access_mode: AccessMode::Live,
        verbs: vec![ReadVerb::List, ReadVerb::Get],
    }
}

pub(crate) fn database_key_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["engine", "name"],
        "properties": {
            "engine": {"enum": ["mysql", "postgresql"]},
            "name": {"type": "string", "minLength": 1, "maxLength": 253}
        }
    })
}

/// The one descriptor schema: list and get publish the same shape, so there is no separate
/// detail projection to drift from the compact one.
pub(crate) fn database_endpoint_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["engine", "name", "provider_config", "secret_ref", "ready"],
        "properties": {
            "engine": {"enum": ["mysql", "postgresql"]},
            "name": {"type": "string"},
            "provider_config": {"type": "string", "minLength": 1},
            "secret_ref": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name", "namespace"],
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "namespace": {"type": "string", "minLength": 1}
                }
            },
            "ready": {"type": "boolean"}
        }
    })
}

pub(crate) fn databases_projection_sha256() -> String {
    let declaration = json!({
        "protocol": "b10x.value-projection.v1",
        "datasource_ref": DATABASES_DATASOURCE,
        // Version 2 (S-062): the descriptor follows the real cluster-scoped resources — the
        // ProviderConfig name and its connection-secret reference replace the host/port/database
        // facts the resources never declared.
        "version": 2,
        "key_schema": database_key_schema(),
        "compact_schema": database_endpoint_schema(),
        "detail_schema": database_endpoint_schema(),
        "excluded": ["secret_values", "credentials", "annotations", "labels", "deletion_policy", "raw_objects"]
    });
    hex::encode(Sha256::digest(
        serde_json::to_vec(&declaration).expect("static projection declaration"),
    ))
}

pub(crate) fn databases_description(context: &PrincipalContext) -> DatasourceDescription {
    DatasourceDescription {
        summary: databases_summary(),
        description: "Live database endpoint descriptors discovered from the cluster-scoped Crossplane databases.{mysql,postgresql}.sql.crossplane.io managed resources: engine, name, the ProviderConfig each resource binds, that config's connection-secret reference (name and namespace), and readiness. A binding lists the resources whose connection secret lives in its admitted namespace. Secret values are never read and never returned.".to_owned(),
        key_schema: database_key_schema(),
        compact_schema: database_endpoint_schema(),
        detail_schema: database_endpoint_schema(),
        projection_protocol: "b10x.value-projection.v1".to_owned(),
        projection_sha256: databases_projection_sha256(),
        description_ref: databases_description_ref(context),
    }
}

/// The description lease for the database-endpoint datasource. Seeded from the stable
/// authority for the same reason `datasource_description_ref` is (see the rationale there):
/// a describe and the read that follows may straddle an access-token refresh.
pub(crate) fn databases_description_ref(context: &PrincipalContext) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/kubernetes-database-datasource-description/v1\0");
    digest.update(context.stable_authority_seed());
    digest.update(b"\0");
    digest.update(databases_projection_sha256().as_bytes());
    format!(
        "description:kubernetes:datasource:{}",
        hex::encode(&digest.finalize()[..16])
    )
}

/// One namespace, offered as a binding of the database-endpoint datasource. Its own seam: the
/// digest is domain-separated from the workload binding, so a binding ref names exactly one
/// (datasource, namespace) pair.
pub(crate) fn database_namespace_binding(
    connection_ref: &str,
    namespace: &str,
) -> DatasourceBinding {
    let digest = Sha256::digest(format!("{DATABASES_DATASOURCE}\0{namespace}\0v1"));
    let mut generation_bytes = [0_u8; 8];
    generation_bytes.copy_from_slice(&digest[..8]);
    DatasourceBinding {
        datasource_ref: DATABASES_DATASOURCE.to_owned(),
        binding_ref: database_namespace_binding_ref(namespace),
        connection_ref: connection_ref.to_owned(),
        label: namespace.to_owned(),
        purpose: None,
        generation: u64::from_be_bytes(generation_bytes),
    }
}

pub(crate) fn database_namespace_binding_ref(namespace: &str) -> String {
    let digest = Sha256::digest(format!("{DATABASES_DATASOURCE}\0{namespace}\0binding-v1"));
    format!("binding:kubernetes:{}", hex::encode(&digest[..16]))
}

/// One merged-page cursor: the engine the next fetch resumes and, after the separator, that
/// engine's own `continue` token (empty = the engine's first page). This string never leaves
/// the process — `CursorStore` hands the caller an authority-bound digest for it.
fn encode_database_cursor(engine: DatabaseEngine, token: &str) -> String {
    format!("{}\0{token}", engine.as_str())
}

fn decode_database_cursor(
    cursor: Option<&str>,
) -> Result<(DatabaseEngine, Option<String>), DatasourceError> {
    let Some(cursor) = cursor else {
        return Ok((DatabaseEngine::Mysql, None));
    };
    let invalid = || datasource_unavailable("Kubernetes database cursor state is invalid");
    let (engine, token) = cursor.split_once('\0').ok_or_else(invalid)?;
    let engine = DatabaseEngine::parse(engine).ok_or_else(invalid)?;
    Ok((engine, (!token.is_empty()).then(|| token.to_owned())))
}

/// Serves one read of `kubernetes.databases`, once the placement has decided the caller may
/// read this namespace — the same contract as `read_workloads`, for the same reason: what a
/// discovered endpoint IS must not depend on which placement answered.
pub(crate) async fn read_database_endpoints(
    reader: &dyn DeploymentReader,
    cursors: &CursorStore,
    context: &PrincipalContext,
    namespace: &str,
    request: ReadRequest,
    connector_audit_ref: String,
) -> Result<DatasourceResult, DatasourceError> {
    if request.datasource_ref != DATABASES_DATASOURCE {
        return Err(datasource_not_found("Kubernetes datasource was not found"));
    }
    if request.description_ref != databases_description_ref(context) {
        return Err(DatasourceError::new(
            DatasourceErrorCode::StaleAuthority,
            "the Kubernetes database datasource description lease has moved on; describe it again and retry the read",
            true,
        ));
    }
    let (records, next_cursor) = match request.read {
        DatasourceRead::List { limit, cursor } => {
            let provider_cursor = cursors.resolve(context, namespace, cursor.as_deref())?;
            let (mut engine, mut engine_cursor) =
                decode_database_cursor(provider_cursor.as_deref())?;
            let mut endpoints = Vec::new();
            let mut merged_cursor = None;
            loop {
                let remaining = usize::from(limit).saturating_sub(endpoints.len());
                let page_limit = u16::try_from(remaining).expect("remaining is bounded by limit");
                // The join surface first: one bounded ProviderConfig read per engine touched,
                // so every database on the page resolves against the same snapshot. The list
                // itself is cluster-scoped; the namespace gate is the projection's association
                // filter, not a list path segment.
                let provider_configs =
                    provider_config_secrets(reader.provider_configs(engine).await?);
                let list = reader
                    .list_databases(engine, page_limit, engine_cursor.as_deref())
                    .await?;
                endpoints.extend(list.items.into_iter().filter_map(|database| {
                    project_database_endpoint(engine, database, &provider_configs, namespace)
                }));
                if let Some(token) = list.next_cursor {
                    merged_cursor = Some(encode_database_cursor(engine, &token));
                    break;
                }
                let Some(following) = engine.following() else {
                    break;
                };
                engine = following;
                engine_cursor = None;
                if endpoints.len() >= usize::from(limit) {
                    // The page is full and an engine is still unread: the next page starts it.
                    merged_cursor = Some(encode_database_cursor(engine, ""));
                    break;
                }
            }
            let next_cursor = cursors.store(context, namespace, merged_cursor)?;
            let records = endpoints
                .into_iter()
                .map(|endpoint| {
                    let key = json!({"engine": endpoint.engine, "name": endpoint.name});
                    let value = serde_json::to_value(endpoint).map_err(|_| {
                        datasource_unavailable("Kubernetes database endpoint could not be encoded")
                    })?;
                    Ok(DatasourceRecord {
                        key,
                        view: RecordView::Compact,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, DatasourceError>>()?;
            (records, next_cursor)
        }
        DatasourceRead::Get { key } => {
            let key: DatabaseEndpointKey = serde_json::from_value(key).map_err(|_| {
                DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    "Kubernetes database endpoint key is invalid",
                    false,
                )
            })?;
            let Some(engine) = DatabaseEngine::parse(&key.engine) else {
                return Err(DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    "Kubernetes database engine is invalid",
                    false,
                ));
            };
            if !valid_dns_label(&key.name, 253) {
                return Err(DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    "Kubernetes database name is invalid",
                    false,
                ));
            }
            let database = reader.database_detail(engine, &key.name).await?;
            let provider_configs = provider_config_secrets(reader.provider_configs(engine).await?);
            let Some(endpoint) =
                project_database_endpoint(engine, database, &provider_configs, namespace)
            else {
                // The resource exists on the cluster, but not for this binding: its connection
                // secret lives in another namespace (or nowhere resolvable), and this
                // namespace's read must neither publish nor confirm it.
                return Err(datasource_not_found(
                    "Kubernetes database endpoint was not found",
                ));
            };
            let key = json!({"engine": endpoint.engine, "name": endpoint.name});
            let value = serde_json::to_value(endpoint).map_err(|_| {
                datasource_unavailable("Kubernetes database endpoint could not be encoded")
            })?;
            (
                vec![DatasourceRecord {
                    key,
                    view: RecordView::Detail,
                    value,
                }],
                None,
            )
        }
    };
    Ok(DatasourceResult::Read(DatasourcePage {
        datasource_ref: DATABASES_DATASOURCE.to_owned(),
        records,
        next_cursor,
        completeness: Completeness::Complete,
        observed_at_unix_ms: now_unix_ms(),
        provenance: DatasourceProvenance {
            binding_ref: request.binding_ref,
            projection_sha256: databases_projection_sha256(),
            connector_audit_ref,
        },
    }))
}
