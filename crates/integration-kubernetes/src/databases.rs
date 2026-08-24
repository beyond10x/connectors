//! The Kubernetes database-endpoint discovery projection (S-059, design 15).
//!
//! The admitted namespace carries the deployment's database inventory as Crossplane provider-sql
//! managed resources — `databases.mysql.sql.crossplane.io` and
//! `databases.postgresql.sql.crossplane.io` — and this module is the host-mode-neutral half of
//! reading them: the wire types, the projection into `DatabaseEndpoint` descriptors, the JSON
//! Schemas, the projection digest, the description lease, the binding refs and the merged
//! two-engine listing. Like `crate::workloads`, admission stays with the placement:
//! `DeploymentReader` is the one seam, and the hosted receiver decides who may read which
//! namespace before `read_database_endpoints` runs.
//!
//! Discovery publishes requirements and references, never secret bytes (design 15). A descriptor
//! names the engine (derived from the API group, never read from the resource body), the
//! resource, whatever endpoint facts the resource itself declares (`spec.forProvider`) or
//! observes (`status.atProvider`), the `writeConnectionSecretToRef` NAME, and readiness. Where
//! the resource carries no host, port or database name — provider-sql's resources carry none;
//! those facts live in the connection Secret — the descriptor says `null` honestly rather than
//! guessing, and nothing in this module ever touches a Secret endpoint.

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
    DeploymentReader, KubernetesCondition, KubernetesMetadata,
};

pub(crate) const DATABASES_DATASOURCE: &str = "kubernetes.databases";

/// The served version of the Crossplane provider-sql Database CRDs
/// (`databases.mysql.sql.crossplane.io`, `databases.postgresql.sql.crossplane.io`).
/// crossplane-contrib/provider-sql serves `v1alpha1` for both groups, and that is what the
/// target cluster runs today; if the cluster moves to a newer served version, this constant
/// follows it in one place.
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

/// One Crossplane Database managed resource, as the cluster serves it. Only the fields the
/// projection may publish are read; annotations, labels, provider plumbing and anything
/// credential-shaped never deserialize at all.
#[derive(Deserialize)]
pub(crate) struct CrossplaneDatabase {
    pub(crate) metadata: KubernetesMetadata,
    #[serde(default)]
    pub(crate) spec: CrossplaneDatabaseSpec,
    #[serde(default)]
    pub(crate) status: CrossplaneDatabaseStatus,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrossplaneDatabaseSpec {
    #[serde(default)]
    pub(crate) for_provider: CrossplaneEndpointFields,
    #[serde(default)]
    pub(crate) write_connection_secret_to_ref: Option<CrossplaneSecretRef>,
}

/// The optional endpoint facts a managed resource may declare (`spec.forProvider`) or observe
/// (`status.atProvider`). provider-sql's Database resources carry none of them — the connection
/// facts live in the connection Secret — and the descriptor then says `null` honestly.
#[derive(Default, Deserialize)]
pub(crate) struct CrossplaneEndpointFields {
    #[serde(default)]
    pub(crate) host: Option<String>,
    #[serde(default)]
    pub(crate) port: Option<u16>,
    #[serde(default)]
    pub(crate) database: Option<String>,
}

/// A connection-secret reference: names only, never bytes. The platform's custody (S-058)
/// resolves it; discovery must not, and no code path in this crate reads a Secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CrossplaneSecretRef {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) namespace: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrossplaneDatabaseStatus {
    #[serde(default)]
    pub(crate) at_provider: CrossplaneEndpointFields,
    #[serde(default)]
    pub(crate) conditions: Vec<KubernetesCondition>,
}

/// One engine's page of Database resources, as the reader fetched it.
pub(crate) struct DatabaseList {
    pub(crate) items: Vec<CrossplaneDatabase>,
    /// The provider's own `continue` token for this engine's list; `None` when the engine is
    /// exhausted — or when its CRD is absent, which discovery treats as an empty inventory.
    pub(crate) next_cursor: Option<String>,
}

/// The published endpoint descriptor: what S-058's connections consume. Optional fields
/// serialize as `null` rather than disappearing, so an absent fact is stated, not implied.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DatabaseEndpoint {
    pub(crate) engine: String,
    pub(crate) name: String,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) database: Option<String>,
    pub(crate) secret_ref: Option<CrossplaneSecretRef>,
    pub(crate) ready: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DatabaseEndpointKey {
    pub(crate) engine: String,
    pub(crate) name: String,
}

/// Projects one managed resource into its endpoint descriptor. Observed state
/// (`status.atProvider`) wins over declared intent (`spec.forProvider`) where both carry a
/// fact; `ready` is the Crossplane `Ready` condition and nothing subtler.
pub(crate) fn project_database_endpoint(
    engine: DatabaseEngine,
    database: CrossplaneDatabase,
) -> DatabaseEndpoint {
    let ready = database
        .status
        .conditions
        .iter()
        .any(|condition| condition.kind == "Ready" && condition.status == "True");
    let observed = database.status.at_provider;
    let declared = database.spec.for_provider;
    DatabaseEndpoint {
        engine: engine.as_str().to_owned(),
        name: database.metadata.name,
        host: observed.host.or(declared.host),
        port: observed.port.or(declared.port),
        database: observed.database.or(declared.database),
        secret_ref: database.spec.write_connection_secret_to_ref,
        ready,
    }
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
        "required": ["engine", "name", "host", "port", "database", "secret_ref", "ready"],
        "properties": {
            "engine": {"enum": ["mysql", "postgresql"]},
            "name": {"type": "string"},
            "host": {"type": ["string", "null"]},
            "port": {"type": ["integer", "null"], "minimum": 1, "maximum": 65535},
            "database": {"type": ["string", "null"]},
            "secret_ref": {
                "type": ["object", "null"],
                "additionalProperties": false,
                "required": ["name", "namespace"],
                "properties": {
                    "name": {"type": "string"},
                    "namespace": {"type": ["string", "null"]}
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
        "version": 1,
        "key_schema": database_key_schema(),
        "compact_schema": database_endpoint_schema(),
        "detail_schema": database_endpoint_schema(),
        "excluded": ["secret_values", "credentials", "annotations", "labels", "provider_config", "raw_objects"]
    });
    hex::encode(Sha256::digest(
        serde_json::to_vec(&declaration).expect("static projection declaration"),
    ))
}

pub(crate) fn databases_description(context: &PrincipalContext) -> DatasourceDescription {
    DatasourceDescription {
        summary: databases_summary(),
        description: "Live, namespace-scoped database endpoint descriptors discovered from the Crossplane databases.{mysql,postgresql}.sql.crossplane.io managed resources: engine, name, whatever endpoint facts the resource declares or observes, the connection-secret reference by name, and readiness. Secret values are never read and never returned.".to_owned(),
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
                let list = reader
                    .list_databases(namespace, engine, page_limit, engine_cursor.as_deref())
                    .await?;
                endpoints.extend(
                    list.items
                        .into_iter()
                        .map(|database| project_database_endpoint(engine, database)),
                );
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
            let database = reader.database_detail(namespace, engine, &key.name).await?;
            let endpoint = project_database_endpoint(engine, database);
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
