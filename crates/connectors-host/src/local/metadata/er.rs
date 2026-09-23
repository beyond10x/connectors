use super::{
    APPROVAL_KEYS_MIGRATION, APPROVAL_MIGRATION, APPROVAL_POLICY_MIGRATION, AUDIT_MIGRATION,
    Failure, MIGRATION as BASE_MIGRATION, MUTATION_MIGRATION, REGISTRY_MIGRATION,
    RUNTIME_MIGRATION, Result, migration_digest as base_migration_digest, unavailable,
};
use entity_core::{EntityDefinition, EntityInstance, OperationFieldAction, Registry};
use entity_eventlog::{
    Authority, EventlogOperationContext, RecordedProviderFacade,
    sync::{
        BridgeConfig, CallWait, EventlogRecordedStoreOwner, EventlogRecordedStoreProvisioner,
        ProvisionAuthority,
    },
};
use entity_executor::{BatchAction, CreateRequest, ExecuteRequest, ExecutionError};
use entity_store::{
    LegacyStoreSnapshot, Recording,
    asynchronous::{
        AsyncStoreError, BatchKey, CompleteStoreSnapshot, HistoryOrigin, LegacyAnchor,
        LegacyCompleteness, LegacyOrderDeclaration, RecordedEntry, Subject, SubjectHistory,
        WriteFailure,
    },
};
use eventlog_core::CaptureLimits;
use rusqlite::{Connection, OptionalExtension, params_from_iter, types::Value as SqlValue};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    num::NonZeroU16,
    path::Path,
    time::{Duration, Instant},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Visible projection rows keyed by entity and identity: lifecycle state and fields.
type ProjectionRows = BTreeMap<(String, String), (String, Map<String, Value>)>;

pub(super) const LEVEL: i64 = 9;
pub(super) const PREFIX: &str = "connectors_er";
pub(super) const LOGICAL_SCOPE: &str = "connectors.local-metadata/1";
pub(super) const MARKER: &str = "connectors_er_authority";
pub(super) const MIGRATION: &str = "CREATE TABLE connectors_er_authority (singleton INTEGER PRIMARY KEY CHECK(singleton=1), authority_id TEXT NOT NULL UNIQUE, logical_scope TEXT NOT NULL, tenant TEXT NOT NULL, stream_identity TEXT NOT NULL, source_level INTEGER NOT NULL CHECK(source_level BETWEEN 1 AND 8), projection_level INTEGER NOT NULL CHECK(projection_level BETWEEN source_level AND 8), source_digest TEXT NOT NULL);";

const LIMITS: CaptureLimits = CaptureLimits {
    // The predecessor has owner-specific row limits but no shared lifetime or
    // aggregate limit. A smaller capture cap would eventually make every read
    // refuse even though the owning port still admits its state. Preserve that
    // contract while retaining Eventlog's checked, explicitly selected bound.
    max_events: u64::MAX,
    max_blobs: u64::MAX,
    max_projection_rows: u64::MAX,
    max_payload_bytes: u64::MAX,
};

#[derive(Clone, Copy)]
enum Kind {
    Text,
    Integer,
    Boolean,
}

#[derive(Clone, Copy)]
struct Column {
    name: &'static str,
    kind: Kind,
    nullable: bool,
    introduced: i64,
}

#[derive(Clone, Copy)]
struct Table {
    level: i64,
    table: &'static str,
    entity: &'static str,
    primary_key: &'static str,
    columns: &'static [Column],
}

macro_rules! text {
    ($name:literal) => {
        Column {
            name: $name,
            kind: Kind::Text,
            nullable: false,
            introduced: 1,
        }
    };
    ($name:literal, nullable) => {
        Column {
            name: $name,
            kind: Kind::Text,
            nullable: true,
            introduced: 1,
        }
    };
}
macro_rules! integer {
    ($name:literal) => {
        Column {
            name: $name,
            kind: Kind::Integer,
            nullable: false,
            introduced: 1,
        }
    };
    ($name:literal, nullable) => {
        Column {
            name: $name,
            kind: Kind::Integer,
            nullable: true,
            introduced: 1,
        }
    };
}
macro_rules! boolean {
    ($name:literal) => {
        Column {
            name: $name,
            kind: Kind::Boolean,
            nullable: false,
            introduced: 1,
        }
    };
}

const TABLES: &[Table] = &[
    Table {
        level: 2,
        table: "registry_clock",
        entity: "connectors.clock.LocalClockFloor",
        primary_key: "singleton",
        columns: &[integer!("singleton"), integer!("last_seen_ms")],
    },
    Table {
        level: 2,
        table: "registry_instances",
        entity: "connectors.declarations.ServiceConfiguration",
        primary_key: "instance_id",
        columns: &[
            text!("instance_id"),
            text!("adapter_id"),
            text!("configuration_revision"),
            integer!("epoch"),
        ],
    },
    Table {
        level: 2,
        table: "registry_profiles",
        entity: "connectors.auth_bindings.AuthProfile",
        primary_key: "profile_key",
        columns: &[
            text!("profile_key"),
            text!("adapter_id"),
            text!("profile_ref"),
            text!("revision"),
            text!("declaration"),
        ],
    },
    Table {
        level: 2,
        table: "registry_connections",
        entity: "connectors.auth_bindings.Connection",
        primary_key: "connection_ref",
        columns: &[
            text!("connection_ref"),
            text!("instance_id"),
            text!("profile_key"),
            text!("binding"),
            text!("scope_id"),
            text!("semantic_revision"),
            text!("publication_fence"),
            text!("state"),
            boolean!("public"),
            text!("identity", nullable),
            text!("active_generation", nullable),
            text!("active_material", nullable),
            text!("baseline", nullable),
            integer!("created_at_ms"),
            integer!("revoked_at_ms", nullable),
        ],
    },
    Table {
        level: 2,
        table: "registry_acquisitions",
        entity: "connectors.auth_bindings.Acquisition",
        primary_key: "acquisition_ref",
        columns: &[
            text!("acquisition_ref"),
            text!("connection_ref"),
            text!("owner_token"),
            text!("expected_fence"),
            text!("state"),
            text!("failure", nullable),
            integer!("created_at_ms"),
            integer!("expires_at_ms"),
            integer!("consumed_at_ms", nullable),
            text!("generation_id"),
            text!("capture_id"),
            text!("candidate_id", nullable),
        ],
    },
    Table {
        level: 2,
        table: "registry_generations",
        entity: "connectors.credentials.CredentialGeneration",
        primary_key: "generation_id",
        columns: &[
            text!("generation_id"),
            text!("connection_ref"),
            text!("capture_id"),
            text!("expected_identity"),
        ],
    },
    Table {
        level: 2,
        table: "registry_materials",
        entity: "connectors.auth_bindings.CustodyVersion",
        primary_key: "version_id",
        columns: &[
            text!("version_id"),
            text!("connection_ref"),
            text!("acquisition_ref"),
            text!("generation_id"),
            boolean!("acknowledged"),
            integer!("acknowledged_at_ms", nullable),
            boolean!("deleted"),
            text!("invalid_reason", nullable),
            integer!("byte_size", nullable),
            text!("retirement_fence", nullable),
            integer!("retired_at_ms", nullable),
            integer!("delete_not_before_ms", nullable),
        ],
    },
    Table {
        level: 2,
        table: "registry_uses",
        entity: "connectors.credential_evidence.ReadUse",
        primary_key: "use_id",
        columns: &[
            text!("use_id"),
            text!("connection_ref"),
            text!("generation_id"),
            text!("version_id"),
            text!("publication_fence"),
            integer!("expires_at_ms"),
            boolean!("dispatched"),
            boolean!("released"),
        ],
    },
    Table {
        level: 2,
        table: "registry_cursors",
        entity: "connectors.cli.ConnectionListCursor",
        primary_key: "cursor_id",
        columns: &[
            text!("cursor_id"),
            text!("instance_id"),
            text!("adapter_id"),
            text!("configuration_revision"),
            integer!("epoch"),
            text!("last_connection"),
            integer!("page_limit"),
            integer!("expires_at_ms"),
        ],
    },
    Table {
        level: 3,
        table: "local_runtime_instances",
        entity: "connectors.cli.LocalRuntimeRecord",
        primary_key: "instance_id",
        columns: &[
            text!("instance_id"),
            boolean!("suppressed"),
            text!("selection", nullable),
            text!("bootstrap", nullable),
            integer!("observed_at_ms", nullable),
        ],
    },
    Table {
        level: 4,
        table: "mutation_clock",
        entity: "connectors.clock.LocalClockFloor",
        primary_key: "singleton",
        columns: &[integer!("singleton"), integer!("last_lower_ms")],
    },
    Table {
        level: 4,
        table: "mutation_attempts",
        entity: "connectors.mutations.AttemptRecord",
        primary_key: "attempt_id",
        columns: &[
            text!("attempt_id"),
            text!("instance_id"),
            text!("connection_ref"),
            text!("request_id"),
            text!("fingerprint"),
            text!("approval_mode"),
            text!("approval_ref", nullable),
            text!("owner_nonce"),
            text!("publication_fence"),
            text!("state"),
            integer!("settled_at_ms", nullable),
            text!("result_json", nullable),
            Column {
                name: "approval_subject",
                kind: Kind::Text,
                nullable: true,
                introduced: 6,
            },
        ],
    },
    Table {
        level: 4,
        table: "mutation_keys",
        entity: "connectors.idempotency.KeyReservation",
        primary_key: "reservation_id",
        columns: &[
            text!("reservation_id"),
            text!("namespace_key"),
            text!("attempt_id"),
            text!("fingerprint"),
            text!("state"),
            integer!("settled_at_ms", nullable),
            integer!("replay_expires_at_ms", nullable),
        ],
    },
    Table {
        level: 5,
        table: "execution_audits",
        entity: "connectors.execution_audit.AuditRecord",
        primary_key: "audit_record_ref",
        columns: &[
            text!("audit_record_ref"),
            text!("instance_id"),
            text!("audit_ref"),
            text!("connection_ref", nullable),
            text!("attempt_id", nullable),
            text!("record_json"),
        ],
    },
    Table {
        level: 6,
        table: "approval_redemptions",
        entity: "connectors.delegation.ApprovalRedemption",
        primary_key: "receipt_id",
        columns: &[
            text!("receipt_id"),
            text!("issuer"),
            text!("reference"),
            text!("instance_id"),
            text!("attempt_id"),
            text!("subject"),
            integer!("spent_at_ms"),
        ],
    },
    Table {
        level: 7,
        table: "local_approval_issuers",
        entity: "connectors.approval_issuers.ApprovalIssuer",
        primary_key: "issuer_id",
        columns: &[
            text!("issuer_id"),
            text!("instance_id"),
            text!("revision"),
            text!("custody_scope"),
        ],
    },
    Table {
        level: 7,
        table: "local_approval_keys",
        entity: "connectors.approval_issuers.ApprovalSigningKey",
        primary_key: "key_id",
        columns: &[
            text!("key_id"),
            text!("issuer_id"),
            text!("public_key"),
            text!("material_version"),
            text!("state"),
        ],
    },
    Table {
        level: 8,
        table: "local_approval_policies",
        entity: "connectors.local_approval_policy.LocalApprovalPolicy",
        primary_key: "policy_id",
        columns: &[
            text!("policy_id"),
            text!("instance_id"),
            text!("issuer_id"),
            integer!("revision"),
            integer!("owner_uid"),
            text!("selection"),
            text!("operations"),
        ],
    },
];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RowImage {
    entity: String,
    id: String,
    revision: u64,
    lifecycle_state: String,
    fields: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
struct RawRow {
    table: &'static str,
    fields: Map<String, Value>,
}

#[derive(serde::Deserialize)]
struct DefinitionBundle {
    format: String,
    definitions: Vec<DefinitionEntry>,
    commands: Vec<CommandEntry>,
}

#[derive(serde::Deserialize)]
struct DefinitionEntry {
    name: String,
    definition: EntityDefinition,
}

#[derive(Clone, serde::Deserialize)]
struct CommandEntry {
    name: String,
    entity: String,
    operation: Option<String>,
    slots: Vec<SlotEntry>,
}

#[derive(Clone, serde::Deserialize)]
struct SlotEntry {
    slot: String,
    target: String,
    outcome: Option<String>,
    field: Option<String>,
    source: String,
    required: bool,
}

pub(super) struct ErAuthority {
    facade: RecordedProviderFacade,
    source_level: i64,
    projection_level: i64,
    durable_path: std::path::PathBuf,
    baseline: BTreeMap<(String, String), RowImage>,
}

#[cfg(test)]
pub(super) fn snapshot_facts(
    er: &ErAuthority,
) -> Vec<(String, String, String, Map<String, Value>)> {
    er.baseline
        .values()
        .map(|row| {
            (
                row.entity.clone(),
                row.id.clone(),
                row.lifecycle_state.clone(),
                row.fields.clone(),
            )
        })
        .collect()
}

impl std::fmt::Debug for ErAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ErAuthority")
            .field("authority", self.facade.authority())
            .field("subjects", &self.baseline.len())
            .field("projection_level", &self.projection_level)
            .finish()
    }
}

pub(super) fn registry() -> Result<Registry> {
    let bundle = definition_bundle()?;
    if bundle.format != "connectors.entity-runtime-definitions/1" {
        return Err(Failure::MetadataUnavailable);
    }
    let mut registry = Registry::new();
    for entry in bundle.definitions {
        if entry.name != entry.definition.entity {
            return Err(Failure::MetadataUnavailable);
        }
        registry
            .register(entry.definition)
            .map_err(|_| Failure::MetadataUnavailable)?;
    }
    registry
        .validate_all()
        .map_err(|_| Failure::MetadataUnavailable)?;
    Ok(registry)
}

fn definition_bundle() -> Result<DefinitionBundle> {
    serde_json::from_slice(include_bytes!("entity-runtime-definitions.json"))
        .map_err(|_| Failure::MetadataUnavailable)
}

fn presence(name: &str) -> String {
    format!("{name}_present")
}

pub(super) fn capture(connection: &Connection, level: i64) -> Result<Vec<RowImage>> {
    let mut images = Vec::new();
    for table in TABLES.iter().filter(|table| table.level <= level) {
        let available = table
            .columns
            .iter()
            .filter(|column| column.introduced <= level)
            .collect::<Vec<_>>();
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {}",
            available
                .iter()
                .map(|column| column.name)
                .collect::<Vec<_>>()
                .join(","),
            table.table,
            table.primary_key
        );
        let mut statement = connection.prepare(&sql).map_err(unavailable)?;
        let rows = statement
            .query_map([], |row| {
                let mut fields = Map::new();
                let mut index = 0;
                for column in table.columns {
                    let (present, value) = if column.introduced <= level {
                        let value = row.get_ref(index)?;
                        index += 1;
                        sqlite_json(value, column.kind)?
                    } else {
                        default_json(column.kind)
                    };
                    fields.insert(column.name.into(), value);
                    if column.nullable {
                        fields.insert(presence(column.name), json!(present));
                    }
                }
                Ok(RawRow {
                    table: table.table,
                    fields,
                })
            })
            .map_err(unavailable)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(unavailable)?;
        for row in rows {
            images.push(domain_row(connection, table, row)?);
        }
    }
    Ok(images)
}

fn sqlite_json(
    value: rusqlite::types::ValueRef<'_>,
    kind: Kind,
) -> rusqlite::Result<(bool, Value)> {
    use rusqlite::types::ValueRef;
    match (value, kind) {
        (ValueRef::Null, Kind::Text) => Ok((false, json!(""))),
        (ValueRef::Null, Kind::Integer) => Ok((false, json!(0))),
        (ValueRef::Null, Kind::Boolean) => Ok((false, json!(false))),
        (ValueRef::Text(value), Kind::Text) => Ok((true, json!(String::from_utf8_lossy(value)))),
        (ValueRef::Integer(value), Kind::Integer) => Ok((true, json!(value))),
        (ValueRef::Integer(value), Kind::Boolean) if value == 0 || value == 1 => {
            Ok((true, json!(value == 1)))
        }
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn default_json(kind: Kind) -> (bool, Value) {
    match kind {
        Kind::Text => (false, json!("")),
        Kind::Integer => (false, json!(0)),
        Kind::Boolean => (false, json!(false)),
    }
}

fn domain_row(connection: &Connection, table: &Table, raw: RawRow) -> Result<RowImage> {
    let mut fields = Map::new();
    let (logical_id, state) = match raw.table {
        "registry_clock" => {
            fields.insert("owner".into(), json!("registry"));
            fields.insert("lower_unix_ms".into(), integer_field(&raw, "last_seen_ms")?);
            ("registry".to_owned(), "Recorded")
        }
        "mutation_clock" => {
            fields.insert("owner".into(), json!("mutations"));
            fields.insert(
                "lower_unix_ms".into(),
                integer_field(&raw, "last_lower_ms")?,
            );
            ("mutations".to_owned(), "Recorded")
        }
        "registry_instances" => {
            let id = text_field(&raw, "instance_id")?;
            fields.insert("instance_id".into(), json!(id));
            fields.insert("adapter_id".into(), json!(text_field(&raw, "adapter_id")?));
            fields.insert(
                "revision".into(),
                json!(text_field(&raw, "configuration_revision")?),
            );
            fields.insert("registry_epoch".into(), integer_field(&raw, "epoch")?);
            (id.to_owned(), "Declared")
        }
        "registry_profiles" => {
            let id = text_field(&raw, "profile_key")?;
            let declaration = parse_json(text_field(&raw, "declaration")?)?;
            fields.insert("profile_record_ref".into(), json!(id));
            fields.insert("adapter_id".into(), json!(text_field(&raw, "adapter_id")?));
            fields.insert("profile_id".into(), json!(text_field(&raw, "profile_ref")?));
            fields.insert(
                "declaration_revision".into(),
                json!(text_field(&raw, "revision")?),
            );
            fields.insert(
                "local_static_profile".into(),
                local_static_profile(declaration)?,
            );
            (id.to_owned(), "Declared")
        }
        "registry_connections" => connection_row(connection, &raw, &mut fields)?,
        "registry_acquisitions" => acquisition_row(connection, &raw, &mut fields)?,
        "registry_generations" => generation_row(connection, &raw, &mut fields)?,
        "registry_materials" => custody_row(connection, &raw, &mut fields)?,
        "registry_uses" => read_use_row(&raw, &mut fields)?,
        "registry_cursors" => {
            let id = text_field(&raw, "cursor_id")?;
            fields.insert("cursor_id".into(), json!(id));
            copy_text(&raw, &mut fields, "instance_id", "instance_id")?;
            copy_text(&raw, &mut fields, "adapter_id", "adapter_id")?;
            copy_text(
                &raw,
                &mut fields,
                "configuration_revision",
                "configuration_revision",
            )?;
            fields.insert("registry_epoch".into(), integer_field(&raw, "epoch")?);
            copy_text(&raw, &mut fields, "last_connection", "last_connection_ref")?;
            fields.insert("page_limit".into(), integer_field(&raw, "page_limit")?);
            fields.insert(
                "expires_at".into(),
                timestamp_value(integer(&raw, "expires_at_ms")?)?,
            );
            (id.to_owned(), "Active")
        }
        "local_runtime_instances" => {
            let id = text_field(&raw, "instance_id")?;
            fields.insert("instance_id".into(), json!(id));
            fields.insert("suppressed".into(), boolean_field(&raw, "suppressed")?);
            copy_optional_text(&raw, &mut fields, "selection", "selection", false)?;
            if optional_present(&raw, "bootstrap")? {
                let encoded = text_field(&raw, "bootstrap")?;
                let bootstrap: crate::local::runtime::Bootstrap =
                    serde_json::from_str(encoded).map_err(|_| Failure::MetadataUnavailable)?;
                bootstrap
                    .validate()
                    .map_err(|_| Failure::MetadataUnavailable)?;
                fields.insert(
                    "bootstrap".into(),
                    serde_json::to_value(bootstrap).map_err(|_| Failure::MetadataUnavailable)?,
                );
            }
            copy_optional_timestamp(&raw, &mut fields, "observed_at_ms", "observed_at")?;
            (id.to_owned(), "Retained")
        }
        "mutation_attempts" => attempt_row(connection, &raw, &mut fields)?,
        "mutation_keys" => reservation_row(&raw, &mut fields)?,
        "execution_audits" => audit_row(&raw, &mut fields)?,
        "approval_redemptions" => {
            let id = text_field(&raw, "receipt_id")?;
            fields.insert("receipt_id".into(), json!(id));
            copy_text(&raw, &mut fields, "instance_id", "instance_id")?;
            copy_text(&raw, &mut fields, "issuer", "issuer")?;
            copy_text(&raw, &mut fields, "reference", "reference")?;
            fields.insert(
                "subject".into(),
                approval_subject_for_er(text_field(&raw, "subject")?)?,
            );
            copy_text(&raw, &mut fields, "attempt_id", "attempt_id")?;
            fields.insert(
                "spent_at".into(),
                timestamp_value(integer(&raw, "spent_at_ms")?)?,
            );
            (id.to_owned(), "Spent")
        }
        "local_approval_issuers" => {
            let id = text_field(&raw, "issuer_id")?;
            fields.insert("issuer_id".into(), json!(id));
            copy_text(&raw, &mut fields, "instance_id", "instance_id")?;
            fields.insert(
                "issuer".into(),
                json!(format!("connectors.local-issuer/{id}")),
            );
            fields.insert(
                "audience".into(),
                json!(format!("connectors.approval/{id}")),
            );
            copy_text(&raw, &mut fields, "revision", "revision")?;
            copy_text(&raw, &mut fields, "custody_scope", "custody_scope_ref")?;
            (id.to_owned(), "Bound")
        }
        "local_approval_keys" => {
            let id = text_field(&raw, "key_id")?;
            fields.insert("key_id".into(), json!(id));
            copy_text(&raw, &mut fields, "issuer_id", "issuer_id")?;
            copy_text(&raw, &mut fields, "public_key", "public_key")?;
            fields.insert("not_before_unix_ms".into(), json!(0));
            fields.insert("not_after_unix_ms".into(), json!(9_007_199_254_740_991_i64));
            copy_text(&raw, &mut fields, "material_version", "material_version")?;
            (
                id.to_owned(),
                signing_key_state(text_field(&raw, "state")?)?,
            )
        }
        "local_approval_policies" => {
            let id = text_field(&raw, "policy_id")?;
            fields.insert("policy_id".into(), json!(id));
            copy_text(&raw, &mut fields, "instance_id", "instance_id")?;
            copy_text(&raw, &mut fields, "issuer_id", "issuer_id")?;
            fields.insert("revision".into(), integer_field(&raw, "revision")?);
            fields.insert("owner_uid".into(), integer_field(&raw, "owner_uid")?);
            fields.insert(
                "selection".into(),
                parse_json(text_field(&raw, "selection")?)?,
            );
            fields.insert(
                "operations".into(),
                parse_json(text_field(&raw, "operations")?)?,
            );
            (id.to_owned(), "Configured")
        }
        _ => return Err(Failure::MetadataUnavailable),
    };
    Ok(RowImage {
        entity: table.entity.into(),
        id: format!("s:{logical_id}"),
        revision: 1,
        lifecycle_state: state.into(),
        fields,
    })
}

fn text_field<'a>(row: &'a RawRow, name: &str) -> Result<&'a str> {
    row.fields
        .get(name)
        .and_then(Value::as_str)
        .ok_or(Failure::MetadataUnavailable)
}

fn integer(row: &RawRow, name: &str) -> Result<i64> {
    row.fields
        .get(name)
        .and_then(Value::as_i64)
        .ok_or(Failure::MetadataUnavailable)
}

fn integer_field(row: &RawRow, name: &str) -> Result<Value> {
    Ok(json!(integer(row, name)?))
}

fn boolean_field(row: &RawRow, name: &str) -> Result<Value> {
    row.fields
        .get(name)
        .and_then(Value::as_bool)
        .map(Value::Bool)
        .ok_or(Failure::MetadataUnavailable)
}

fn optional_present(row: &RawRow, name: &str) -> Result<bool> {
    row.fields
        .get(&presence(name))
        .and_then(Value::as_bool)
        .ok_or(Failure::MetadataUnavailable)
}

fn copy_text(
    row: &RawRow,
    fields: &mut Map<String, Value>,
    source: &str,
    target: &str,
) -> Result<()> {
    fields.insert(target.into(), json!(text_field(row, source)?));
    Ok(())
}

fn copy_optional_text(
    row: &RawRow,
    fields: &mut Map<String, Value>,
    source: &str,
    target: &str,
    parse: bool,
) -> Result<()> {
    if optional_present(row, source)? {
        let value = text_field(row, source)?;
        fields.insert(
            target.into(),
            if parse {
                parse_json(value)?
            } else {
                json!(value)
            },
        );
    }
    Ok(())
}

fn copy_optional_json(
    row: &RawRow,
    fields: &mut Map<String, Value>,
    source: &str,
    target: &str,
) -> Result<()> {
    copy_optional_text(row, fields, source, target, true)
}

fn copy_optional_timestamp(
    row: &RawRow,
    fields: &mut Map<String, Value>,
    source: &str,
    target: &str,
) -> Result<()> {
    if optional_present(row, source)? {
        fields.insert(target.into(), timestamp_value(integer(row, source)?)?);
    }
    Ok(())
}

fn timestamp_value(milliseconds: i64) -> Result<Value> {
    let value = OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
        .map_err(|_| Failure::MetadataUnavailable)?;
    Ok(json!(
        value
            .format(&Rfc3339)
            .map_err(|_| Failure::MetadataUnavailable)?
    ))
}

fn parse_json(value: &str) -> Result<Value> {
    serde_json::from_str(value).map_err(|_| Failure::MetadataUnavailable)
}

// Canonical approval proof bytes require explicit nulls for these fixed
// coordinates. ER's current Optional newtype schema admits absence instead;
// restore the exact proof representation at the SQL projection boundary.
fn approval_subject_for_er(encoded: &str) -> Result<Value> {
    let mut value = parse_json(encoded)?;
    let subject: crate::local::approvals::Subject =
        serde_json::from_value(value.clone()).map_err(|_| Failure::MetadataUnavailable)?;
    if subject
        .canonical_bytes()
        .map_err(|_| Failure::MetadataUnavailable)?
        != encoded.as_bytes()
    {
        return Err(Failure::MetadataUnavailable);
    }
    approval_subject_nulls(&mut value, false)?;
    Ok(value)
}

fn approval_subject_from_er(value: &Value) -> Result<Value> {
    let mut value = value.clone();
    approval_subject_nulls(&mut value, true)?;
    let subject: crate::local::approvals::Subject =
        serde_json::from_value(value.clone()).map_err(|_| Failure::MetadataUnavailable)?;
    subject
        .canonical_bytes()
        .map_err(|_| Failure::MetadataUnavailable)?;
    Ok(value)
}

fn approval_subject_nulls(value: &mut Value, restore: bool) -> Result<()> {
    let subject = value.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
    nullable_proof_coordinate(subject, "route", restore)?;
    let authority = subject
        .get_mut("authority")
        .and_then(Value::as_object_mut)
        .ok_or(Failure::MetadataUnavailable)?;
    nullable_proof_coordinate(authority, "current_authority", restore)?;
    nullable_proof_coordinate(authority, "executor", restore)?;
    let scope = authority
        .get_mut("scope")
        .and_then(Value::as_object_mut)
        .ok_or(Failure::MetadataUnavailable)?;
    for name in ["tenant", "realm", "executor"] {
        nullable_proof_coordinate(scope, name, restore)?;
    }
    Ok(())
}

fn nullable_proof_coordinate(
    object: &mut Map<String, Value>,
    name: &str,
    restore: bool,
) -> Result<()> {
    if restore {
        object.entry(name.to_owned()).or_insert(Value::Null);
    } else if object
        .get(name)
        .ok_or(Failure::MetadataUnavailable)?
        .is_null()
    {
        object.remove(name);
    }
    Ok(())
}

fn local_static_profile(mut value: Value) -> Result<Value> {
    let object = value.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
    let id = object.remove("id").ok_or(Failure::MetadataUnavailable)?;
    object.insert("profile_id".into(), id);
    Ok(value)
}

fn signing_key_state(value: &str) -> Result<&'static str> {
    match value {
        "candidate" => Ok("Candidate"),
        "active" => Ok("Active"),
        "retired" => Ok("Retired"),
        "retiring" => Ok("Retiring"),
        "deleted" => Ok("Deleted"),
        _ => Err(Failure::MetadataUnavailable),
    }
}

fn attempt_row(
    connection: &Connection,
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "attempt_id")?;
    let fingerprint = fingerprint_value(text_field(row, "fingerprint")?)?;
    let operation_ref = fingerprint
        .get("operation_ref")
        .and_then(Value::as_str)
        .ok_or(Failure::MetadataUnavailable)?;
    let operation = parse_json(operation_ref)?;
    let operation_id = operation
        .as_array()
        .and_then(|parts| parts.get(3))
        .and_then(Value::as_str)
        .ok_or(Failure::MetadataUnavailable)?;
    let input_digest = fingerprint
        .get("input_digest")
        .and_then(Value::as_str)
        .ok_or(Failure::MetadataUnavailable)?;
    fields.insert("attempt_id".into(), json!(id));
    copy_text(row, fields, "instance_id", "instance_id")?;
    copy_text(row, fields, "request_id", "request_id")?;
    fields.insert("operation_id".into(), json!(operation_id));
    copy_text(row, fields, "connection_ref", "connection_ref")?;
    fields.insert("input_digest".into(), json!(input_digest));
    fields.insert("request_fingerprint".into(), fingerprint.clone());
    copy_text(row, fields, "approval_mode", "approval_mode")?;
    copy_optional_text(row, fields, "approval_ref", "approval_ref", false)?;
    if optional_present(row, "approval_subject")? {
        fields.insert(
            "approval_subject".into(),
            approval_subject_for_er(text_field(row, "approval_subject")?)?,
        );
    }
    if let Some(namespace_key) = connection
        .query_row(
            "SELECT namespace_key FROM mutation_keys WHERE attempt_id=?1",
            [id],
            |record| record.get::<_, String>(0),
        )
        .optional()
        .map_err(unavailable)?
    {
        let namespace = parse_namespace_key(&namespace_key)?;
        fields.insert(
            "idempotency_key".into(),
            namespace
                .get("caller_key")
                .cloned()
                .ok_or(Failure::MetadataUnavailable)?,
        );
    }
    copy_text(row, fields, "owner_nonce", "owner_nonce")?;
    copy_text(row, fields, "publication_fence", "publication_fence")?;
    copy_optional_timestamp(row, fields, "settled_at_ms", "settled_at")?;
    copy_optional_text(row, fields, "result_json", "terminal_result_json", false)?;
    let state = match text_field(row, "state")? {
        "prepared" => "Prepared",
        "dispatching" => "Dispatching",
        "aborted" => "Aborted",
        "completed" => "Completed",
        "failed" => "Failed",
        "indeterminate" => "Indeterminate",
        _ => return Err(Failure::MetadataUnavailable),
    };
    Ok((id.to_owned(), state))
}

fn reservation_row(
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "reservation_id")?;
    let namespace = parse_namespace_key(text_field(row, "namespace_key")?)?;
    fields.insert("reservation_id".into(), json!(id));
    fields.insert(
        "namespace".into(),
        namespace
            .get("namespace")
            .cloned()
            .ok_or(Failure::MetadataUnavailable)?,
    );
    fields.insert(
        "caller_key".into(),
        namespace
            .get("caller_key")
            .cloned()
            .ok_or(Failure::MetadataUnavailable)?,
    );
    fields.insert(
        "fingerprint".into(),
        fingerprint_value(text_field(row, "fingerprint")?)?,
    );
    copy_text(row, fields, "attempt_id", "attempt_id")?;
    copy_optional_timestamp(row, fields, "settled_at_ms", "settled_at")?;
    copy_optional_timestamp(row, fields, "replay_expires_at_ms", "replay_expires_at")?;
    let state = match text_field(row, "state")? {
        "pending" => "Pending",
        "replayable" => "Replayable",
        "quarantined" => "Quarantined",
        "expired" => "Expired",
        _ => return Err(Failure::MetadataUnavailable),
    };
    Ok((id.to_owned(), state))
}

fn fingerprint_value(encoded: &str) -> Result<Value> {
    let value = parse_json(encoded)?;
    let items = value.as_array().ok_or(Failure::MetadataUnavailable)?;
    if items.len() != 11 || items[0] != "mutation-request/v2" {
        return Err(Failure::MetadataUnavailable);
    }
    let operation = items[1].as_array().ok_or(Failure::MetadataUnavailable)?;
    if operation.len() != 4 || operation[0] != "connectors.operation/v1" {
        return Err(Failure::MetadataUnavailable);
    }
    operation[3].as_str().ok_or(Failure::MetadataUnavailable)?;
    let operation_ref =
        serde_json::to_string(&items[1]).map_err(|_| Failure::MetadataUnavailable)?;
    let mut fingerprint = json!({
        "operation_ref": operation_ref,
        "connection_ref": items[2],
        "connection_revision": items[3],
        "contract_ref": items[4],
        "profile": items[5],
        "descriptor_revision": items[6],
        "configuration_revision": items[7],
        "canonicalization_version": items[8],
        "input_digest": items[9],
    });
    if !items[10].is_null() {
        fingerprint["route"] = items[10].clone();
    }
    Ok(fingerprint)
}

fn parse_namespace_key(encoded: &str) -> Result<Value> {
    let value = parse_json(encoded)?;
    let items = value.as_array().ok_or(Failure::MetadataUnavailable)?;
    if items.len() != 9 || items[0] != "mutation-key/v1" {
        return Err(Failure::MetadataUnavailable);
    }
    let receiver = items[1].clone();
    let origin_kind = items[6].as_str().ok_or(Failure::MetadataUnavailable)?;
    if !matches!(origin_kind, "direct" | "federated") {
        return Err(Failure::MetadataUnavailable);
    }
    let mut authority = Map::new();
    authority.insert("caller".into(), items[4].clone());
    for (name, index) in [("tenant", 2), ("realm", 3), ("executor", 5)] {
        if !items[index].is_null() {
            authority.insert(name.into(), items[index].clone());
        }
    }
    Ok(json!({
        "namespace": {
            "receiver_instance": receiver,
            "authority": authority,
            "origin": {"kind": origin_kind, "authority_ref": items[7]},
        },
        "caller_key": items[8],
    }))
}

fn audit_row(row: &RawRow, fields: &mut Map<String, Value>) -> Result<(String, &'static str)> {
    let id = text_field(row, "audit_record_ref")?;
    let encoded = text_field(row, "record_json")?;
    let record: crate::local::audit::Record =
        serde_json::from_str(encoded).map_err(|_| Failure::MetadataUnavailable)?;
    if record.encode().map_err(|_| Failure::MetadataUnavailable)? != encoded {
        return Err(Failure::MetadataUnavailable);
    }
    let record = serde_json::to_value(record).map_err(|_| Failure::MetadataUnavailable)?;
    let object = record.as_object().ok_or(Failure::MetadataUnavailable)?;
    let anchor = object
        .get("anchor")
        .and_then(Value::as_object)
        .ok_or(Failure::MetadataUnavailable)?;
    fields.insert("audit_record_ref".into(), json!(id));
    copy_json_field(anchor, fields, "instance_id", "instance_id")?;
    copy_text(row, fields, "audit_ref", "audit_ref")?;
    copy_json_field(anchor, fields, "kind", "anchor_kind")?;
    copy_optional_json_field(anchor, fields, "activity", "activity")?;
    copy_json_field(anchor, fields, "hop", "hop_role")?;
    copy_json_field(anchor, fields, "stage", "stage")?;
    for name in [
        "request_id",
        "principal_ref",
        "operation_id",
        "connection_ref",
        "descriptor_revision",
        "attempt_id",
    ] {
        copy_optional_json_field(anchor, fields, name, name)?;
    }
    let recorded = anchor
        .get("recorded_at_ms")
        .and_then(Value::as_i64)
        .ok_or(Failure::MetadataUnavailable)?;
    fields.insert("recorded_at".into(), timestamp_value(recorded)?);
    let final_observation = object
        .get("final_observation")
        .filter(|value| !value.is_null());
    if let Some(final_observation) = final_observation {
        let mut final_value = final_observation.clone();
        let final_object = final_value
            .as_object_mut()
            .ok_or(Failure::MetadataUnavailable)?;
        let at = final_object
            .remove("recorded_at_ms")
            .and_then(|value| value.as_i64())
            .ok_or(Failure::MetadataUnavailable)?;
        final_object.insert("recorded_at".into(), timestamp_value(at)?);
        if final_object.get("code").is_some_and(Value::is_null) {
            final_object.remove("code");
        }
        fields.insert("final_observation".into(), final_value);
    }
    Ok((
        id.to_owned(),
        if final_observation.is_some() {
            "FinalObserved"
        } else {
            "Anchored"
        },
    ))
}

fn copy_json_field(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
    source_name: &str,
    target_name: &str,
) -> Result<()> {
    target.insert(
        target_name.into(),
        source
            .get(source_name)
            .filter(|value| !value.is_null())
            .cloned()
            .ok_or(Failure::MetadataUnavailable)?,
    );
    Ok(())
}

fn copy_optional_json_field(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
    source_name: &str,
    target_name: &str,
) -> Result<()> {
    if let Some(value) = source.get(source_name).filter(|value| !value.is_null()) {
        target.insert(target_name.into(), value.clone());
    }
    Ok(())
}

fn connection_row(
    connection: &Connection,
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "connection_ref")?;
    let binding = parse_json(text_field(row, "binding")?)?;
    let binding = binding.as_object().ok_or(Failure::MetadataUnavailable)?;
    let profile = binding
        .get("profile")
        .and_then(Value::as_object)
        .ok_or(Failure::MetadataUnavailable)?;
    fields.insert("connection_ref".into(), json!(id));
    copy_text(row, fields, "instance_id", "instance_id")?;
    copy_text(row, fields, "profile_key", "profile_record_ref")?;
    copy_json_field(profile, fields, "id", "profile_ref")?;
    copy_text(row, fields, "scope_id", "owner_scope")?;
    fields.insert("actor".into(), json!("connectors-host"));
    copy_json_field(binding, fields, "provider_authority", "provider_authority")?;
    fields.insert("access_mode".into(), json!("credential"));
    copy_text(row, fields, "semantic_revision", "semantic_revision")?;
    copy_text(row, fields, "publication_fence", "publication_fence")?;
    fields.insert("enabled".into(), json!(true));
    fields.insert("published".into(), boolean_field(row, "public")?);
    copy_optional_json(row, fields, "identity", "external_identity")?;
    copy_optional_text(
        row,
        fields,
        "active_generation",
        "active_generation_id",
        false,
    )?;
    if optional_present(row, "baseline")? {
        fields.insert(
            "baseline_evidence".into(),
            evidence_value(parse_json(text_field(row, "baseline")?)?)?,
        );
    }
    copy_optional_text(
        row,
        fields,
        "active_material",
        "active_custody_version_ref",
        false,
    )?;
    let active_material = if optional_present(row, "active_material")? {
        Some(text_field(row, "active_material")?)
    } else {
        None
    };
    let mut statement = connection
        .prepare(
            "SELECT version_id FROM registry_materials \
             WHERE connection_ref=?1 AND (?2 IS NULL OR version_id<>?2) ORDER BY version_id",
        )
        .map_err(unavailable)?;
    let superseded = statement
        .query_map(rusqlite::params![id, active_material], |record| {
            record.get::<_, String>(0)
        })
        .map_err(unavailable)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(unavailable)?;
    fields.insert("superseded_custody_version_refs".into(), json!(superseded));
    fields.insert(
        "created_at".into(),
        timestamp_value(integer(row, "created_at_ms")?)?,
    );
    copy_optional_timestamp(row, fields, "revoked_at_ms", "revoked_at")?;
    Ok((
        id.to_owned(),
        match text_field(row, "state")? {
            "live" => "Live",
            "revoked" => "Revoked",
            _ => return Err(Failure::MetadataUnavailable),
        },
    ))
}

fn acquisition_row(
    connection: &Connection,
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "acquisition_ref")?;
    let connection_ref = text_field(row, "connection_ref")?;
    let (instance_id, profile_record_ref, scope_ref): (String, String, String) = connection
        .query_row(
            "SELECT instance_id,profile_key,scope_id FROM registry_connections WHERE connection_ref=?1",
            [connection_ref],
            |record| Ok((record.get(0)?, record.get(1)?, record.get(2)?)),
        )
        .map_err(unavailable)?;
    fields.insert("acquisition_ref".into(), json!(id));
    fields.insert("instance_id".into(), json!(instance_id));
    fields.insert("profile_record_ref".into(), json!(profile_record_ref));
    fields.insert("owner_scope".into(), json!(scope_ref));
    fields.insert("admitted_origin_ref".into(), json!("local-static-entry"));
    fields.insert(
        "protected_correlation_ref".into(),
        json!(text_field(row, "capture_id")?),
    );
    fields.insert(
        "expires_at".into(),
        timestamp_value(integer(row, "expires_at_ms")?)?,
    );
    copy_text(row, fields, "expected_fence", "expected_publication_fence")?;
    fields.insert("target_connection_ref".into(), json!(connection_ref));
    if text_field(row, "state")? == "completed" {
        fields.insert("result_connection_ref".into(), json!(connection_ref));
    }
    copy_optional_text(row, fields, "failure", "terminal_reason", false)?;
    copy_text(row, fields, "owner_token", "owner_token")?;
    fields.insert(
        "created_at".into(),
        timestamp_value(integer(row, "created_at_ms")?)?,
    );
    copy_optional_timestamp(row, fields, "consumed_at_ms", "consumed_at")?;
    copy_text(row, fields, "generation_id", "generation_id")?;
    copy_text(row, fields, "capture_id", "captured_snapshot_ref")?;
    copy_optional_text(
        row,
        fields,
        "candidate_id",
        "candidate_custody_version_ref",
        false,
    )?;
    let state = match text_field(row, "state")? {
        "pending" => "Pending",
        "completing" => "Completing",
        "completed" => "Completed",
        "failed" => "Failed",
        "expired" => "Expired",
        _ => return Err(Failure::MetadataUnavailable),
    };
    Ok((id.to_owned(), state))
}

fn generation_row(
    connection: &Connection,
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "generation_id")?;
    let connection_ref = text_field(row, "connection_ref")?;
    let (instance_id, binding): (String, String) = connection
        .query_row(
            "SELECT instance_id,binding FROM registry_connections WHERE connection_ref=?1",
            [connection_ref],
            |record| Ok((record.get(0)?, record.get(1)?)),
        )
        .map_err(unavailable)?;
    let binding = parse_json(&binding)?;
    let binding = binding.as_object().ok_or(Failure::MetadataUnavailable)?;
    let profile = binding
        .get("profile")
        .and_then(Value::as_object)
        .ok_or(Failure::MetadataUnavailable)?;
    fields.insert("generation_id".into(), json!(id));
    fields.insert("instance_id".into(), json!(instance_id));
    fields.insert("connection_ref".into(), json!(connection_ref));
    copy_json_field(profile, fields, "id", "profile_ref")?;
    copy_json_field(binding, fields, "provider_authority", "provider_authority")?;
    copy_text(row, fields, "capture_id", "captured_snapshot_ref")?;
    fields.insert(
        "expected_external_identity".into(),
        parse_json(text_field(row, "expected_identity")?)?,
    );
    Ok((id.to_owned(), "Captured"))
}

fn custody_row(
    connection: &Connection,
    row: &RawRow,
    fields: &mut Map<String, Value>,
) -> Result<(String, &'static str)> {
    let id = text_field(row, "version_id")?;
    let connection_ref = text_field(row, "connection_ref")?;
    let (instance_id, scope_ref): (String, String) = connection
        .query_row(
            "SELECT instance_id,scope_id FROM registry_connections WHERE connection_ref=?1",
            [connection_ref],
            |record| Ok((record.get(0)?, record.get(1)?)),
        )
        .map_err(unavailable)?;
    fields.insert("version_ref".into(), json!(id));
    fields.insert("instance_id".into(), json!(instance_id));
    fields.insert("scope_ref".into(), json!(scope_ref));
    fields.insert("store_version".into(), json!(id));
    if optional_present(row, "byte_size")? {
        fields.insert(
            "credential_set".into(),
            json!({"kind":"local-static-entry","material_handle":id,"size_bytes":integer(row, "byte_size")?}),
        );
        fields.insert("byte_size".into(), integer_field(row, "byte_size")?);
    }
    copy_optional_text(row, fields, "retirement_fence", "retirement_fence", false)?;
    copy_optional_timestamp(row, fields, "delete_not_before_ms", "retire_not_before")?;
    fields.insert("connection_ref".into(), json!(connection_ref));
    copy_text(row, fields, "acquisition_ref", "acquisition_ref")?;
    copy_text(row, fields, "generation_id", "generation_id")?;
    copy_optional_timestamp(row, fields, "acknowledged_at_ms", "acknowledged_at")?;
    copy_optional_text(row, fields, "invalid_reason", "invalid_reason", false)?;
    copy_optional_timestamp(row, fields, "retired_at_ms", "retired_at")?;
    let state = if row.fields.get("deleted").and_then(Value::as_bool) == Some(true) {
        "Deleted"
    } else if optional_present(row, "retirement_fence")? {
        "Retiring"
    } else if optional_present(row, "invalid_reason")? {
        "Invalid"
    } else if row.fields.get("acknowledged").and_then(Value::as_bool) == Some(true) {
        "Stored"
    } else {
        "Candidate"
    };
    Ok((id.to_owned(), state))
}

fn read_use_row(row: &RawRow, fields: &mut Map<String, Value>) -> Result<(String, &'static str)> {
    let id = text_field(row, "use_id")?;
    fields.insert("use_id".into(), json!(id));
    copy_text(row, fields, "connection_ref", "connection_ref")?;
    copy_text(row, fields, "generation_id", "generation_id")?;
    copy_text(row, fields, "version_id", "custody_version_ref")?;
    copy_text(row, fields, "publication_fence", "publication_fence")?;
    fields.insert(
        "expires_at".into(),
        timestamp_value(integer(row, "expires_at_ms")?)?,
    );
    fields.insert("dispatch_opened".into(), boolean_field(row, "dispatched")?);
    fields.insert("released".into(), boolean_field(row, "released")?);
    let state = if row.fields.get("released").and_then(Value::as_bool) == Some(true) {
        "Released"
    } else if row.fields.get("dispatched").and_then(Value::as_bool) == Some(true) {
        "Dispatched"
    } else {
        "Captured"
    };
    Ok((id.to_owned(), state))
}

fn evidence_value(mut value: Value) -> Result<Value> {
    let object = value.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
    // Rust serializes absent Option fields as null; the authored Optional
    // fields are represented by omission in Entity Runtime.
    for name in ["granted_scopes", "credential_expires_at"] {
        if object.get(name).is_some_and(Value::is_null) {
            object.remove(name);
        }
    }
    if let Some(expires) = object.remove("credential_expires_at") {
        object.insert(
            "credential_expires_at".into(),
            timestamp_value(expires.as_i64().ok_or(Failure::MetadataUnavailable)?)?,
        );
    }
    let checks = object
        .get_mut("checks")
        .and_then(Value::as_array_mut)
        .ok_or(Failure::MetadataUnavailable)?;
    for check in checks {
        let check = check.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
        for name in ["collected_at", "valid_until"] {
            let milliseconds = check
                .remove(name)
                .and_then(|value| value.as_i64())
                .ok_or(Failure::MetadataUnavailable)?;
            check.insert(name.into(), timestamp_value(milliseconds)?);
        }
    }
    Ok(value)
}

pub(super) fn provision(
    path: &Path,
    authority_id: uuid::Uuid,
    level: i64,
    rows: Vec<RowImage>,
) -> Result<(ErAuthority, Authority, String)> {
    let registry = registry()?;
    let source_digest = rows_digest(&rows)?;
    let source_id = format!("connectors-legacy-sqlite/{authority_id}/{source_digest}");
    let expected = rows
        .iter()
        .map(|row| {
            (
                (row.entity.clone(), row.id.clone()),
                (row.lifecycle_state.clone(), row.fields.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let histories = legacy_histories(&registry, rows)?;
    let snapshot =
        LegacyStoreSnapshot::new(source_id, histories).map_err(|_| Failure::MetadataUnavailable)?;
    let (facade, authority) = {
        let facade = RecordedProviderFacade::provision(
            registry,
            EventlogRecordedStoreProvisioner::Sqlite {
                path: utf8_path(path)?,
                prefix: PREFIX.into(),
                authority: ProvisionAuthority {
                    logical_scope: LOGICAL_SCOPE.into(),
                    tenant: authority_id.to_string(),
                    expected_stream_identity: None,
                },
                limits: LIMITS,
            },
            context("provision"),
            bridge_config(),
        )
        .map_err(|_| Failure::MetadataUnavailable)?;
        let authority = facade.authority().clone();
        (facade, authority)
    };
    facade
        .import_legacy(snapshot, context("legacy-import"), call_wait())
        .map_err(|_| Failure::MetadataUnavailable)?;
    let baseline = snapshot_rows(&facade)?;
    if visible_rows(&baseline) != expected {
        return Err(Failure::MetadataUnavailable);
    }
    Ok((
        ErAuthority {
            facade,
            source_level: level,
            projection_level: level,
            durable_path: path.to_owned(),
            baseline,
        },
        authority,
        source_digest,
    ))
}

pub(super) fn open(
    path: &Path,
    authority: Authority,
    source_level: i64,
    projection_level: i64,
) -> Result<(ErAuthority, String)> {
    let definitions = registry()?;
    let facade = RecordedProviderFacade::start(
        definitions,
        EventlogRecordedStoreOwner::Sqlite {
            path: utf8_path(path)?,
            prefix: PREFIX.into(),
            authority,
            limits: LIMITS,
        },
        bridge_config(),
    )
    .map_err(|_| Failure::MetadataUnavailable)?;
    let snapshot = complete_snapshot(&facade)?;
    let source_digest = imported_digest(&snapshot)?;
    let baseline = terminal_rows(snapshot)?;
    Ok((
        ErAuthority {
            facade,
            source_level,
            projection_level,
            durable_path: path.to_owned(),
            baseline,
        },
        source_digest,
    ))
}

pub(super) fn advance_projection_level(er: &mut ErAuthority, level: i64) -> Result<()> {
    if level < er.projection_level || !(er.source_level..=8).contains(&level) {
        return Err(Failure::MetadataUnavailable);
    }
    if level == er.projection_level {
        return Ok(());
    }
    let connection = Connection::open_with_flags(
        &er.durable_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(unavailable)?;
    connection
        .busy_timeout(Duration::from_secs(30))
        .map_err(unavailable)?;
    let changed = connection
        .execute(
            "UPDATE connectors_er_authority SET projection_level=?1 \
             WHERE singleton=1 AND projection_level=?2",
            rusqlite::params![level, er.projection_level],
        )
        .map_err(unavailable)?;
    if changed != 1 {
        return Err(Failure::MetadataUnavailable);
    }
    er.projection_level = level;
    Ok(())
}

fn utf8_path(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(Failure::MetadataUnavailable)
}

fn bridge_config() -> BridgeConfig {
    BridgeConfig {
        queue_capacity: NonZeroU16::new(4).expect("nonzero bridge capacity"),
    }
}

fn legacy_histories(registry: &Registry, rows: Vec<RowImage>) -> Result<Vec<SubjectHistory>> {
    rows.into_iter()
        .map(|row| {
            let definition = registry
                .get(&row.entity, 1)
                .ok_or(Failure::MetadataUnavailable)?;
            if !definition.lifecycle.states.contains(&row.lifecycle_state) {
                return Err(Failure::MetadataUnavailable);
            }
            Ok(SubjectHistory {
                subject: Subject::new(&row.entity, &row.id)
                    .map_err(|_| Failure::MetadataUnavailable)?,
                origin: HistoryOrigin::Imported(LegacyAnchor {
                    instance: EntityInstance {
                        entity: row.entity,
                        version: 1,
                        id: row.id,
                        lifecycle_state: row.lifecycle_state,
                        revision: row.revision,
                        fields: row.fields,
                    },
                    completeness: LegacyCompleteness::AvailableEvidenceOnly,
                    order: LegacyOrderDeclaration::PerKindOnly,
                    evidence: Vec::new(),
                }),
                records: Vec::new(),
            })
        })
        .collect()
}

fn rows_digest(rows: &[RowImage]) -> Result<String> {
    let mut ordered = rows.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| (&left.entity, &left.id).cmp(&(&right.entity, &right.id)));
    let value = ordered
        .into_iter()
        .map(|row| json!([row.entity, row.id, row.lifecycle_state, row.fields]))
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&value).map_err(|_| Failure::MetadataUnavailable)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub(super) fn capture_digest(connection: &Connection, level: i64) -> Result<String> {
    rows_digest(&capture(connection, level)?)
}

fn imported_digest(snapshot: &CompleteStoreSnapshot) -> Result<String> {
    let rows = snapshot
        .histories
        .iter()
        .filter_map(|subject| match &subject.history.origin {
            HistoryOrigin::Imported(anchor) => Some(RowImage {
                entity: anchor.instance.entity.clone(),
                id: anchor.instance.id.clone(),
                revision: anchor.instance.revision,
                lifecycle_state: anchor.instance.lifecycle_state.clone(),
                fields: anchor.instance.fields.clone(),
            }),
            HistoryOrigin::Genesis => None,
        })
        .collect::<Vec<_>>();
    rows_digest(&rows)
}

fn snapshot_rows(facade: &RecordedProviderFacade) -> Result<BTreeMap<(String, String), RowImage>> {
    terminal_rows(complete_snapshot(facade)?)
}

fn complete_snapshot(facade: &RecordedProviderFacade) -> Result<CompleteStoreSnapshot> {
    facade
        .complete_snapshot(call_wait())
        .map_err(|_| Failure::MetadataUnavailable)
}

fn terminal_rows(snapshot: CompleteStoreSnapshot) -> Result<BTreeMap<(String, String), RowImage>> {
    if snapshot.scope != LOGICAL_SCOPE {
        return Err(Failure::MetadataUnavailable);
    }
    snapshot
        .histories
        .into_iter()
        .map(|subject| {
            let EntityInstance {
                entity,
                id,
                revision,
                lifecycle_state,
                fields,
                ..
            } = subject.terminal;
            let key = (entity.clone(), id.clone());
            Ok((
                key,
                RowImage {
                    entity,
                    id,
                    revision,
                    lifecycle_state,
                    fields,
                },
            ))
        })
        .collect()
}

pub(super) fn projection(authority_id: uuid::Uuid, er: &ErAuthority) -> Result<Connection> {
    let level = er.projection_level;
    let connection = Connection::open_in_memory().map_err(unavailable)?;
    connection
        .execute_batch(BASE_MIGRATION)
        .map_err(unavailable)?;
    connection
        .execute(
            "INSERT INTO local_authority VALUES (1,?1,?2)",
            rusqlite::params![authority_id.to_string(), super::fs::uid()],
        )
        .map_err(unavailable)?;
    connection
        .execute(
            "INSERT INTO schema_migrations VALUES (1,?1)",
            [base_migration_digest()],
        )
        .map_err(unavailable)?;
    for (next, source) in migrations() {
        if next > level {
            break;
        }
        connection.execute_batch(source).map_err(unavailable)?;
        connection
            .execute(
                "INSERT INTO schema_migrations VALUES (?1,?2)",
                rusqlite::params![next, hex::encode(Sha256::digest(source.as_bytes()))],
            )
            .map_err(unavailable)?;
    }
    // The old installation migrations seed both clocks at zero. In a rebuilt
    // projection the recorded ER subjects, including their advanced floors,
    // own those rows; the installation defaults must not collide with them.
    if level >= 2 {
        connection
            .execute("DELETE FROM registry_clock", [])
            .map_err(unavailable)?;
    }
    if level >= 4 {
        connection
            .execute("DELETE FROM mutation_clock", [])
            .map_err(unavailable)?;
    }
    connection
        .pragma_update(None, "foreign_keys", false)
        .map_err(unavailable)?;
    for table in TABLES.iter().filter(|table| table.level <= level) {
        for row in er
            .baseline
            .values()
            .filter(|row| row.entity == table.entity && projects(table, row))
        {
            insert_row(&connection, table, row, &er.baseline, level)?;
        }
    }
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(unavailable)?;
    let foreign_key_errors: i64 = connection
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(unavailable)?;
    if foreign_key_errors != 0 {
        return Err(Failure::MetadataUnavailable);
    }
    connection
        .pragma_update(None, "application_id", super::APPLICATION_ID)
        .map_err(unavailable)?;
    connection
        .pragma_update(None, "user_version", level)
        .map_err(unavailable)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(30))
        .map_err(unavailable)?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(unavailable)?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(unavailable)?;
    connection
        .pragma_update(None, "temp_store", "MEMORY")
        .map_err(unavailable)?;
    Ok(connection)
}

fn insert_row(
    connection: &Connection,
    table: &Table,
    row: &RowImage,
    all: &BTreeMap<(String, String), RowImage>,
    level: i64,
) -> Result<()> {
    let columns = table
        .columns
        .iter()
        .filter(|column| column.introduced <= level)
        .collect::<Vec<_>>();
    let names = columns.iter().map(|column| column.name).collect::<Vec<_>>();
    let placeholders = (1..=names.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>();
    let raw = projection_fields(table, row, all)?;
    let values = columns
        .iter()
        .map(|column| field_sql(&raw, column))
        .collect::<Result<Vec<_>>>()?;
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table.table,
        names.join(","),
        placeholders.join(",")
    );
    connection
        .execute(&sql, params_from_iter(values))
        .map_err(unavailable)?;
    Ok(())
}

fn field_sql(fields: &Map<String, Value>, column: &Column) -> Result<SqlValue> {
    if column.nullable
        && !fields
            .get(&presence(column.name))
            .and_then(Value::as_bool)
            .ok_or(Failure::MetadataUnavailable)?
    {
        return Ok(SqlValue::Null);
    }
    let value = fields
        .get(column.name)
        .ok_or(Failure::MetadataUnavailable)?;
    match column.kind {
        Kind::Text => value
            .as_str()
            .map(|value| SqlValue::Text(value.into()))
            .ok_or(Failure::MetadataUnavailable),
        Kind::Integer => value
            .as_i64()
            .map(SqlValue::Integer)
            .ok_or(Failure::MetadataUnavailable),
        Kind::Boolean => value
            .as_bool()
            .map(|value| SqlValue::Integer(i64::from(value)))
            .ok_or(Failure::MetadataUnavailable),
    }
}

fn projects(table: &Table, row: &RowImage) -> bool {
    match table.table {
        "registry_clock" => row.fields.get("owner") == Some(&json!("registry")),
        "mutation_clock" => row.fields.get("owner") == Some(&json!("mutations")),
        // Expiry deletes these transient SQL rows while their ER terminal
        // history remains available for audit and stale-token refusal.
        "registry_cursors" | "registry_uses" => row.lifecycle_state != "Expired",
        _ => true,
    }
}

fn projection_fields(
    table: &Table,
    row: &RowImage,
    all: &BTreeMap<(String, String), RowImage>,
) -> Result<Map<String, Value>> {
    let mut raw = Map::new();
    match table.table {
        "registry_clock" => put(&mut raw, "singleton", json!(1)),
        "mutation_clock" => put(&mut raw, "singleton", json!(1)),
        "registry_instances" => {
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "adapter_id", "adapter_id")?;
            copy_domain(row, &mut raw, "revision", "configuration_revision")?;
            copy_domain(row, &mut raw, "registry_epoch", "epoch")?;
        }
        "registry_profiles" => {
            copy_domain(row, &mut raw, "profile_record_ref", "profile_key")?;
            copy_domain(row, &mut raw, "adapter_id", "adapter_id")?;
            copy_domain(row, &mut raw, "profile_id", "profile_ref")?;
            copy_domain(row, &mut raw, "declaration_revision", "revision")?;
            let mut declaration = domain(row, "local_static_profile")?.clone();
            let object = declaration
                .as_object_mut()
                .ok_or(Failure::MetadataUnavailable)?;
            let profile_id = object
                .remove("profile_id")
                .ok_or(Failure::MetadataUnavailable)?;
            object.insert("id".into(), profile_id);
            let declaration: crate::local::registry::StaticProfile =
                serde_json::from_value(declaration).map_err(|_| Failure::MetadataUnavailable)?;
            put(
                &mut raw,
                "declaration",
                json!(
                    serde_json::to_string(&declaration)
                        .map_err(|_| Failure::MetadataUnavailable)?
                ),
            );
        }
        "registry_connections" => connection_projection(row, all, &mut raw)?,
        "registry_acquisitions" => acquisition_projection(row, &mut raw)?,
        "registry_generations" => generation_projection(row, &mut raw)?,
        "registry_materials" => custody_projection(row, &mut raw)?,
        "registry_uses" => read_use_projection(row, &mut raw)?,
        "registry_cursors" => {
            copy_domain(row, &mut raw, "cursor_id", "cursor_id")?;
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "adapter_id", "adapter_id")?;
            copy_domain(
                row,
                &mut raw,
                "configuration_revision",
                "configuration_revision",
            )?;
            copy_domain(row, &mut raw, "registry_epoch", "epoch")?;
            copy_domain(row, &mut raw, "last_connection_ref", "last_connection")?;
            copy_domain(row, &mut raw, "page_limit", "page_limit")?;
            put(
                &mut raw,
                "expires_at_ms",
                json!(domain_timestamp_ms(row, "expires_at")?),
            );
        }
        "local_runtime_instances" => {
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "suppressed", "suppressed")?;
            copy_optional_domain(row, &mut raw, "selection", "selection", false)?;
            let bootstrap = row
                .fields
                .get("bootstrap")
                .map(|value| {
                    let bootstrap: crate::local::runtime::Bootstrap =
                        serde_json::from_value(value.clone())
                            .map_err(|_| Failure::MetadataUnavailable)?;
                    bootstrap
                        .validate()
                        .map_err(|_| Failure::MetadataUnavailable)?;
                    serde_json::to_string(&bootstrap)
                        .map(Value::String)
                        .map_err(|_| Failure::MetadataUnavailable)
                })
                .transpose()?;
            put_optional(&mut raw, "bootstrap", bootstrap);
            copy_optional_domain_timestamp(row, &mut raw, "observed_at", "observed_at_ms")?;
        }
        "mutation_attempts" => attempt_projection(row, &mut raw)?,
        "mutation_keys" => reservation_projection(row, &mut raw)?,
        "execution_audits" => audit_projection(row, &mut raw)?,
        "approval_redemptions" => {
            copy_domain(row, &mut raw, "receipt_id", "receipt_id")?;
            copy_domain(row, &mut raw, "issuer", "issuer")?;
            copy_domain(row, &mut raw, "reference", "reference")?;
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "attempt_id", "attempt_id")?;
            put_json_text(
                &mut raw,
                "subject",
                approval_subject_from_er(domain(row, "subject")?)?,
            )?;
            put(
                &mut raw,
                "spent_at_ms",
                json!(domain_timestamp_ms(row, "spent_at")?),
            );
        }
        "local_approval_issuers" => {
            copy_domain(row, &mut raw, "issuer_id", "issuer_id")?;
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "revision", "revision")?;
            copy_domain(row, &mut raw, "custody_scope_ref", "custody_scope")?;
        }
        "local_approval_keys" => {
            copy_domain(row, &mut raw, "key_id", "key_id")?;
            copy_domain(row, &mut raw, "issuer_id", "issuer_id")?;
            copy_domain(row, &mut raw, "public_key", "public_key")?;
            copy_domain(row, &mut raw, "material_version", "material_version")?;
            put(
                &mut raw,
                "state",
                json!(row.lifecycle_state.to_ascii_lowercase()),
            );
        }
        "local_approval_policies" => {
            copy_domain(row, &mut raw, "policy_id", "policy_id")?;
            copy_domain(row, &mut raw, "instance_id", "instance_id")?;
            copy_domain(row, &mut raw, "issuer_id", "issuer_id")?;
            copy_domain(row, &mut raw, "revision", "revision")?;
            copy_domain(row, &mut raw, "owner_uid", "owner_uid")?;
            put_json_text(&mut raw, "selection", domain(row, "selection")?.clone())?;
            put_json_text(&mut raw, "operations", domain(row, "operations")?.clone())?;
        }
        _ => return Err(Failure::MetadataUnavailable),
    }
    if matches!(table.table, "registry_clock" | "mutation_clock") {
        let target = if table.table == "registry_clock" {
            "last_seen_ms"
        } else {
            "last_lower_ms"
        };
        copy_domain(row, &mut raw, "lower_unix_ms", target)?;
    }
    Ok(raw)
}

fn put(raw: &mut Map<String, Value>, name: &str, value: Value) {
    raw.insert(name.into(), value);
}

fn put_optional(raw: &mut Map<String, Value>, name: &str, value: Option<Value>) {
    raw.insert(presence(name), json!(value.is_some()));
    raw.insert(name.into(), value.unwrap_or(Value::Null));
}

fn put_json_text(raw: &mut Map<String, Value>, name: &str, value: Value) -> Result<()> {
    put(
        raw,
        name,
        json!(serde_json::to_string(&value).map_err(|_| Failure::MetadataUnavailable)?),
    );
    Ok(())
}

fn domain<'a>(row: &'a RowImage, name: &str) -> Result<&'a Value> {
    row.fields.get(name).ok_or(Failure::MetadataUnavailable)
}

fn copy_domain(
    row: &RowImage,
    raw: &mut Map<String, Value>,
    source: &str,
    target: &str,
) -> Result<()> {
    put(raw, target, domain(row, source)?.clone());
    Ok(())
}

fn copy_optional_domain(
    row: &RowImage,
    raw: &mut Map<String, Value>,
    source: &str,
    target: &str,
    encode_json: bool,
) -> Result<()> {
    let value = row.fields.get(source).cloned();
    let value = if encode_json {
        value
            .map(|value| serde_json::to_string(&value).map(Value::String))
            .transpose()
            .map_err(|_| Failure::MetadataUnavailable)?
    } else {
        value
    };
    put_optional(raw, target, value);
    Ok(())
}

fn domain_timestamp_ms(row: &RowImage, name: &str) -> Result<i64> {
    let timestamp = domain(row, name)?
        .as_str()
        .ok_or(Failure::MetadataUnavailable)?;
    let parsed =
        OffsetDateTime::parse(timestamp, &Rfc3339).map_err(|_| Failure::MetadataUnavailable)?;
    i64::try_from(parsed.unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| Failure::MetadataUnavailable)
}

fn copy_optional_domain_timestamp(
    row: &RowImage,
    raw: &mut Map<String, Value>,
    source: &str,
    target: &str,
) -> Result<()> {
    let value = match row.fields.get(source) {
        Some(_) => Some(json!(domain_timestamp_ms(row, source)?)),
        None => None,
    };
    put_optional(raw, target, value);
    Ok(())
}

fn find_entity<'a>(
    all: &'a BTreeMap<(String, String), RowImage>,
    entity: &str,
    identity: &str,
    logical_id: &Value,
) -> Result<&'a RowImage> {
    all.values()
        .find(|candidate| {
            candidate.entity == entity && candidate.fields.get(identity) == Some(logical_id)
        })
        .ok_or(Failure::MetadataUnavailable)
}

fn connection_projection(
    row: &RowImage,
    all: &BTreeMap<(String, String), RowImage>,
    raw: &mut Map<String, Value>,
) -> Result<()> {
    copy_domain(row, raw, "connection_ref", "connection_ref")?;
    copy_domain(row, raw, "instance_id", "instance_id")?;
    copy_domain(row, raw, "profile_record_ref", "profile_key")?;
    let instance = find_entity(
        all,
        "connectors.declarations.ServiceConfiguration",
        "instance_id",
        domain(row, "instance_id")?,
    )?;
    let profile = find_entity(
        all,
        "connectors.auth_bindings.AuthProfile",
        "profile_record_ref",
        domain(row, "profile_record_ref")?,
    )?;
    let mut static_profile = domain(profile, "local_static_profile")?.clone();
    let object = static_profile
        .as_object_mut()
        .ok_or(Failure::MetadataUnavailable)?;
    let profile_id = object
        .remove("profile_id")
        .ok_or(Failure::MetadataUnavailable)?;
    object.insert("id".into(), profile_id);
    let binding = json!({
        "instance_id": domain(row, "instance_id")?,
        "adapter_id": domain(instance, "adapter_id")?,
        "configuration_revision": domain(instance, "revision")?,
        "provider_authority": domain(row, "provider_authority")?,
        "profile": static_profile,
    });
    put_json_text(raw, "binding", binding)?;
    copy_domain(row, raw, "owner_scope", "scope_id")?;
    copy_domain(row, raw, "semantic_revision", "semantic_revision")?;
    copy_domain(row, raw, "publication_fence", "publication_fence")?;
    put(
        raw,
        "state",
        json!(row.lifecycle_state.to_ascii_lowercase()),
    );
    copy_domain(row, raw, "published", "public")?;
    copy_optional_domain(row, raw, "external_identity", "identity", true)?;
    copy_optional_domain(row, raw, "active_generation_id", "active_generation", false)?;
    copy_optional_domain(
        row,
        raw,
        "active_custody_version_ref",
        "active_material",
        false,
    )?;
    let baseline = match row.fields.get("baseline_evidence") {
        Some(value) => Some(Value::String(
            super::super::registry::encode_evidence_value(evidence_sql(value.clone())?)
                .map_err(|_| Failure::MetadataUnavailable)?,
        )),
        None => None,
    };
    put_optional(raw, "baseline", baseline);
    put(
        raw,
        "created_at_ms",
        json!(domain_timestamp_ms(row, "created_at")?),
    );
    copy_optional_domain_timestamp(row, raw, "revoked_at", "revoked_at_ms")?;
    Ok(())
}

fn evidence_sql(mut value: Value) -> Result<Value> {
    let object = value.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
    if object.contains_key("credential_expires_at") {
        let expires = object
            .remove("credential_expires_at")
            .unwrap_or(Value::Null);
        let expires = if expires.is_null() {
            Value::Null
        } else {
            json!(timestamp_ms_value(&expires)?)
        };
        object.insert("credential_expires_at".into(), expires);
    }
    for check in object
        .get_mut("checks")
        .and_then(Value::as_array_mut)
        .ok_or(Failure::MetadataUnavailable)?
    {
        let check = check.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
        for name in ["collected_at", "valid_until"] {
            let value = check.remove(name).ok_or(Failure::MetadataUnavailable)?;
            check.insert(name.into(), json!(timestamp_ms_value(&value)?));
        }
    }
    Ok(value)
}

fn timestamp_ms_value(value: &Value) -> Result<i64> {
    let timestamp = value.as_str().ok_or(Failure::MetadataUnavailable)?;
    let parsed =
        OffsetDateTime::parse(timestamp, &Rfc3339).map_err(|_| Failure::MetadataUnavailable)?;
    i64::try_from(parsed.unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| Failure::MetadataUnavailable)
}

fn acquisition_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "acquisition_ref", "acquisition_ref")?;
    copy_domain(row, raw, "target_connection_ref", "connection_ref")?;
    copy_domain(row, raw, "owner_token", "owner_token")?;
    copy_domain(row, raw, "expected_publication_fence", "expected_fence")?;
    put(
        raw,
        "state",
        json!(row.lifecycle_state.to_ascii_lowercase()),
    );
    copy_optional_domain(row, raw, "terminal_reason", "failure", false)?;
    put(
        raw,
        "created_at_ms",
        json!(domain_timestamp_ms(row, "created_at")?),
    );
    put(
        raw,
        "expires_at_ms",
        json!(domain_timestamp_ms(row, "expires_at")?),
    );
    copy_optional_domain_timestamp(row, raw, "consumed_at", "consumed_at_ms")?;
    copy_domain(row, raw, "generation_id", "generation_id")?;
    copy_domain(row, raw, "captured_snapshot_ref", "capture_id")?;
    copy_optional_domain(
        row,
        raw,
        "candidate_custody_version_ref",
        "candidate_id",
        false,
    )?;
    Ok(())
}

fn generation_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "generation_id", "generation_id")?;
    copy_domain(row, raw, "connection_ref", "connection_ref")?;
    copy_domain(row, raw, "captured_snapshot_ref", "capture_id")?;
    put_json_text(
        raw,
        "expected_identity",
        domain(row, "expected_external_identity")?.clone(),
    )
}

fn custody_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "version_ref", "version_id")?;
    copy_domain(row, raw, "connection_ref", "connection_ref")?;
    copy_domain(row, raw, "acquisition_ref", "acquisition_ref")?;
    copy_domain(row, raw, "generation_id", "generation_id")?;
    put(
        raw,
        "acknowledged",
        json!(row.fields.contains_key("acknowledged_at")),
    );
    copy_optional_domain_timestamp(row, raw, "acknowledged_at", "acknowledged_at_ms")?;
    put(raw, "deleted", json!(row.lifecycle_state == "Deleted"));
    copy_optional_domain(row, raw, "invalid_reason", "invalid_reason", false)?;
    copy_optional_domain(row, raw, "byte_size", "byte_size", false)?;
    copy_optional_domain(row, raw, "retirement_fence", "retirement_fence", false)?;
    copy_optional_domain_timestamp(row, raw, "retired_at", "retired_at_ms")?;
    copy_optional_domain_timestamp(row, raw, "retire_not_before", "delete_not_before_ms")?;
    Ok(())
}

fn read_use_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    let dispatched = domain(row, "dispatch_opened")?
        .as_bool()
        .ok_or(Failure::MetadataUnavailable)?;
    let released = domain(row, "released")?
        .as_bool()
        .ok_or(Failure::MetadataUnavailable)?;
    if !matches!(
        (row.lifecycle_state.as_str(), dispatched, released),
        ("Captured", false, false) | ("Dispatched", true, false) | ("Released", false | true, true)
    ) {
        return Err(Failure::MetadataUnavailable);
    }
    copy_domain(row, raw, "use_id", "use_id")?;
    copy_domain(row, raw, "connection_ref", "connection_ref")?;
    copy_domain(row, raw, "generation_id", "generation_id")?;
    copy_domain(row, raw, "custody_version_ref", "version_id")?;
    copy_domain(row, raw, "publication_fence", "publication_fence")?;
    put(
        raw,
        "expires_at_ms",
        json!(domain_timestamp_ms(row, "expires_at")?),
    );
    copy_domain(row, raw, "dispatch_opened", "dispatched")?;
    copy_domain(row, raw, "released", "released")?;
    Ok(())
}

fn attempt_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "attempt_id", "attempt_id")?;
    copy_domain(row, raw, "instance_id", "instance_id")?;
    copy_domain(row, raw, "connection_ref", "connection_ref")?;
    copy_domain(row, raw, "request_id", "request_id")?;
    put_json_text(
        raw,
        "fingerprint",
        fingerprint_sql(domain(row, "input_digest")?, row)?,
    )?;
    copy_domain(row, raw, "approval_mode", "approval_mode")?;
    copy_optional_domain(row, raw, "approval_ref", "approval_ref", false)?;
    copy_domain(row, raw, "owner_nonce", "owner_nonce")?;
    copy_domain(row, raw, "publication_fence", "publication_fence")?;
    put(
        raw,
        "state",
        json!(row.lifecycle_state.to_ascii_lowercase()),
    );
    copy_optional_domain_timestamp(row, raw, "settled_at", "settled_at_ms")?;
    copy_optional_domain(row, raw, "terminal_result_json", "result_json", false)?;
    let subject = row
        .fields
        .get("approval_subject")
        .map(approval_subject_from_er)
        .transpose()?;
    let subject = subject
        .map(|value| serde_json::to_string(&value).map(Value::String))
        .transpose()
        .map_err(|_| Failure::MetadataUnavailable)?;
    put_optional(raw, "approval_subject", subject);
    Ok(())
}

fn fingerprint_sql(input_digest: &Value, row: &RowImage) -> Result<Value> {
    let retained = domain(row, "request_fingerprint")?;
    let retained_digest = retained
        .get("input_digest")
        .ok_or(Failure::MetadataUnavailable)?;
    if retained_digest != input_digest {
        return Err(Failure::MetadataUnavailable);
    }
    fingerprint_from_value(retained)
}

fn reservation_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "reservation_id", "reservation_id")?;
    put_json_text(raw, "namespace_key", namespace_sql(row)?)?;
    copy_domain(row, raw, "attempt_id", "attempt_id")?;
    put_json_text(
        raw,
        "fingerprint",
        fingerprint_from_value(domain(row, "fingerprint")?)?,
    )?;
    put(
        raw,
        "state",
        json!(row.lifecycle_state.to_ascii_lowercase()),
    );
    copy_optional_domain_timestamp(row, raw, "settled_at", "settled_at_ms")?;
    copy_optional_domain_timestamp(row, raw, "replay_expires_at", "replay_expires_at_ms")?;
    Ok(())
}

fn namespace_sql(row: &RowImage) -> Result<Value> {
    let namespace = domain(row, "namespace")?
        .as_object()
        .ok_or(Failure::MetadataUnavailable)?;
    let authority = namespace
        .get("authority")
        .and_then(Value::as_object)
        .ok_or(Failure::MetadataUnavailable)?;
    let origin = namespace
        .get("origin")
        .and_then(Value::as_object)
        .ok_or(Failure::MetadataUnavailable)?;
    let absent = Value::Null;
    Ok(json!([
        "mutation-key/v1",
        namespace
            .get("receiver_instance")
            .ok_or(Failure::MetadataUnavailable)?,
        authority.get("tenant").unwrap_or(&absent),
        authority.get("realm").unwrap_or(&absent),
        authority
            .get("caller")
            .ok_or(Failure::MetadataUnavailable)?,
        authority.get("executor").unwrap_or(&absent),
        origin.get("kind").ok_or(Failure::MetadataUnavailable)?,
        origin
            .get("authority_ref")
            .ok_or(Failure::MetadataUnavailable)?,
        domain(row, "caller_key")?,
    ]))
}

fn fingerprint_from_value(value: &Value) -> Result<Value> {
    let value = value.as_object().ok_or(Failure::MetadataUnavailable)?;
    let operation: Value = value
        .get("operation_ref")
        .and_then(Value::as_str)
        .ok_or(Failure::MetadataUnavailable)
        .and_then(|value| serde_json::from_str(value).map_err(|_| Failure::MetadataUnavailable))?;
    let absent = Value::Null;
    Ok(json!([
        "mutation-request/v2",
        operation,
        value
            .get("connection_ref")
            .ok_or(Failure::MetadataUnavailable)?,
        value
            .get("connection_revision")
            .ok_or(Failure::MetadataUnavailable)?,
        value
            .get("contract_ref")
            .ok_or(Failure::MetadataUnavailable)?,
        value.get("profile").ok_or(Failure::MetadataUnavailable)?,
        value
            .get("descriptor_revision")
            .ok_or(Failure::MetadataUnavailable)?,
        value
            .get("configuration_revision")
            .ok_or(Failure::MetadataUnavailable)?,
        value
            .get("canonicalization_version")
            .ok_or(Failure::MetadataUnavailable)?,
        value
            .get("input_digest")
            .ok_or(Failure::MetadataUnavailable)?,
        value.get("route").unwrap_or(&absent),
    ]))
}

fn audit_projection(row: &RowImage, raw: &mut Map<String, Value>) -> Result<()> {
    copy_domain(row, raw, "audit_record_ref", "audit_record_ref")?;
    copy_domain(row, raw, "instance_id", "instance_id")?;
    copy_domain(row, raw, "audit_ref", "audit_ref")?;
    copy_optional_domain(row, raw, "connection_ref", "connection_ref", false)?;
    copy_optional_domain(row, raw, "attempt_id", "attempt_id", false)?;
    let mut anchor = Map::new();
    for (field, target) in [
        ("instance_id", "instance_id"),
        ("anchor_kind", "kind"),
        ("hop_role", "hop"),
        ("stage", "stage"),
    ] {
        anchor.insert(target.into(), domain(row, field)?.clone());
    }
    for name in [
        "activity",
        "request_id",
        "principal_ref",
        "operation_id",
        "connection_ref",
        "descriptor_revision",
        "attempt_id",
    ] {
        anchor.insert(
            name.into(),
            row.fields.get(name).cloned().unwrap_or(Value::Null),
        );
    }
    anchor.insert(
        "recorded_at_ms".into(),
        json!(domain_timestamp_ms(row, "recorded_at")?),
    );
    let final_observation = match row.fields.get("final_observation") {
        Some(value) => {
            let mut value = value.clone();
            let object = value.as_object_mut().ok_or(Failure::MetadataUnavailable)?;
            let recorded = object
                .remove("recorded_at")
                .ok_or(Failure::MetadataUnavailable)?;
            object.insert(
                "recorded_at_ms".into(),
                json!(timestamp_ms_value(&recorded)?),
            );
            object.entry("code").or_insert(Value::Null);
            value
        }
        None => Value::Null,
    };
    let record: crate::local::audit::Record = serde_json::from_value(json!({
        "reference": {
            "instance": domain(row, "instance_id")?,
            "audit_ref": domain(row, "audit_ref")?,
        },
        "anchor": anchor,
        "final_observation": final_observation,
    }))
    .map_err(|_| Failure::MetadataUnavailable)?;
    put(
        raw,
        "record_json",
        json!(record.encode().map_err(|_| Failure::MetadataUnavailable)?),
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PersistMode {
    Standard {
        admitted_registry_use: bool,
        concurrent_observation: bool,
    },
    PreparedObservation,
    RuntimeStateOnly,
}

pub(super) fn persist(
    er: &mut ErAuthority,
    connection: &Connection,
    mode: PersistMode,
) -> Result<()> {
    let (admitted_registry_use, concurrent_observation, runtime_state_only, prepared_observation) =
        match mode {
            PersistMode::Standard {
                admitted_registry_use,
                concurrent_observation,
            } => (admitted_registry_use, concurrent_observation, false, false),
            PersistMode::PreparedObservation => (false, false, false, true),
            PersistMode::RuntimeStateOnly => (false, false, true, false),
        };
    let level: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(unavailable)?;
    let desired = capture(connection, level)?
        .into_iter()
        .map(|row| ((row.entity.clone(), row.id.clone()), row))
        .collect::<BTreeMap<_, _>>();
    if runtime_state_only {
        // Suppression and cached bootstrap writes read only the runtime
        // table. Reject any attempt to use this path for a registry clock
        // or another owner's row before preparing an authored batch.
        let changed_non_runtime = er.baseline.keys().chain(desired.keys()).any(|key| {
            let changed = match (er.baseline.get(key), desired.get(key)) {
                (Some(current), Some(next)) => {
                    current.fields != next.fields || current.lifecycle_state != next.lifecycle_state
                }
                (None, None) => false,
                _ => true,
            };
            changed && !runtime_record_key(key)
        });
        if changed_non_runtime {
            return Err(Failure::MetadataUnavailable);
        }
    }
    if prepared_observation {
        // The two admitted observation callers may only persist their
        // registry clock. This check also covers additions and removals.
        let changed_non_clock = er.baseline.keys().chain(desired.keys()).any(|key| {
            let changed = match (er.baseline.get(key), desired.get(key)) {
                (Some(current), Some(next)) => {
                    current.fields != next.fields || current.lifecycle_state != next.lifecycle_state
                }
                (None, None) => false,
                _ => true,
            };
            changed && !registry_clock_key(key)
        });
        if changed_non_clock {
            return Err(Failure::MetadataUnavailable);
        }
    }
    let bundle = definition_bundle()?;
    let commands = bundle
        .commands
        .into_iter()
        .map(|command| (command.name.clone(), command))
        .collect::<BTreeMap<_, _>>();
    let runtime_registry = registry()?;
    let mut actions = Vec::new();
    for (key, current) in &er.baseline {
        match desired.get(key) {
            Some(next)
                if next.fields == current.fields
                    && next.lifecycle_state == current.lifecycle_state => {}
            Some(next) => {
                let command = update_command(current, next)?;
                actions.push(command_action(
                    &runtime_registry,
                    &commands,
                    &desired,
                    current,
                    next,
                    command,
                )?)
            }
            None => {
                if let Some((command, next)) = removal_command(current)? {
                    actions.push(command_action(
                        &runtime_registry,
                        &commands,
                        &desired,
                        current,
                        &next,
                        command,
                    )?);
                } else if !matches!(
                    (current.entity.as_str(), current.lifecycle_state.as_str()),
                    (
                        "connectors.cli.ConnectionListCursor"
                            | "connectors.credential_evidence.ReadUse",
                        "Expired"
                    )
                ) {
                    // Only these two transient SQL rows disappear after their
                    // recorded expiry. Every other deletion would discard an
                    // authoritative subject without an authored command.
                    return Err(Failure::MetadataUnavailable);
                }
            }
        }
    }
    for (key, next) in &desired {
        if !er.baseline.contains_key(key) {
            let mut stages = creation_stages(next, admitted_registry_use)?;
            let initial = stages.remove(0);
            actions.push(create_action(
                &runtime_registry,
                &commands,
                &initial,
                create_command(next)?,
            )?);
            let mut current = initial;
            for mut stage in stages {
                stage.revision = current.revision + 1;
                let command = update_command(&current, &stage)?;
                actions.push(command_action(
                    &runtime_registry,
                    &commands,
                    &desired,
                    &current,
                    &stage,
                    command,
                )?);
                current = stage;
            }
        }
    }
    if actions.is_empty() && !prepared_observation {
        return Ok(());
    }
    if !concurrent_observation && !runtime_state_only {
        // An already-open registry observation may advance its clock after
        // releasing the physical lifecycle lock. Fence that floor even
        // when this batch sampled the same millisecond as its baseline,
        // so the clock subject participates in the atomic revision guard.
        // Mutation-clock writers retain the lifecycle lock and need no
        // extra action. The authored transition records the same lower
        // value; it invents no later time.
        let mut found_registry_clock = false;
        for (key, current) in &er.baseline {
            if current.entity != "connectors.clock.LocalClockFloor"
                || current.fields.get("owner") != Some(&json!("registry"))
            {
                continue;
            }
            found_registry_clock = true;
            if desired.get(key).is_some_and(|next| {
                next.fields == current.fields && next.lifecycle_state == current.lifecycle_state
            }) {
                actions.push(command_action(
                    &runtime_registry,
                    &commands,
                    &desired,
                    current,
                    current,
                    "connectors.clock.AdvanceLocalClockFloor",
                )?);
            }
        }
        if prepared_observation && !found_registry_clock {
            return Err(Failure::MetadataUnavailable);
        }
    }
    let batch = format!("connectors-metadata-batch-{}", uuid::Uuid::new_v4());
    let outcome = er
        .facade
        .execute_batch(
            context("metadata-mutation"),
            BatchKey::Named(batch),
            actions,
            call_wait(),
        )
        .map_err(map_execution_failure)?;
    let receipt = outcome.receipt().ok_or(Failure::MetadataUnavailable)?;
    let snapshot = complete_snapshot(&er.facade)?;
    if !receipt.members().iter().all(|member| {
        snapshot.histories.iter().any(|subject| {
            subject
                .history
                .records
                .iter()
                .any(|record| record.receipt == *member)
        })
    }) {
        return Err(Failure::MetadataUnavailable);
    }
    let last_own_position = receipt
        .members()
        .iter()
        .map(|member| member.position.store)
        .max()
        .ok_or(Failure::MetadataUnavailable)?;
    // A read-only observation can advance only the registry clock while a
    // serialized business transaction is finishing. Verify our exact clock
    // state at the acknowledged batch boundary before admitting its later
    // clock state; a later record must not mask a bad own projection.
    let clock_at_own = if concurrent_observation {
        None
    } else {
        Some(
            snapshot
                .histories
                .iter()
                .filter(|subject| {
                    subject.history.subject.entity == "connectors.clock.LocalClockFloor"
                })
                .map(|subject| {
                    let instance = subject
                        .history
                        .records
                        .iter()
                        .take_while(|record| record.position.store <= last_own_position)
                        .filter_map(|record| match &record.entry {
                            RecordedEntry::Decision(commit) => Some(&commit.instance),
                            RecordedEntry::Observation(_) => None,
                        })
                        .last()
                        .or(match &subject.history.origin {
                            HistoryOrigin::Imported(anchor) => Some(&anchor.instance),
                            HistoryOrigin::Genesis => None,
                        })
                        .ok_or(Failure::MetadataUnavailable)?;
                    Ok((
                        (
                            subject.history.subject.entity.clone(),
                            subject.history.subject.id.clone(),
                        ),
                        (instance.lifecycle_state.clone(), instance.fields.clone()),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        )
    };
    let latest_positions = snapshot
        .histories
        .iter()
        .filter_map(|subject| {
            subject.history.records.last().map(|record| {
                (
                    (
                        subject.history.subject.entity.clone(),
                        subject.history.subject.id.clone(),
                    ),
                    record.position.store,
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    er.baseline = terminal_rows(snapshot)?;
    let rebuilt = projection_from_connection(connection)?;
    let current = visible_rows(&er.baseline);
    if !postcommit_projection_matches(
        &rebuilt,
        &current,
        clock_at_own.as_ref(),
        &latest_positions,
        last_own_position,
        runtime_state_only,
        prepared_observation,
    ) {
        return Err(Failure::MetadataUnavailable);
    }
    Ok(())
}

fn postcommit_projection_matches(
    desired: &ProjectionRows,
    current: &ProjectionRows,
    clock_at_own: Option<&ProjectionRows>,
    latest_positions: &BTreeMap<(String, String), u64>,
    last_own_position: u64,
    runtime_state_only: bool,
    prepared_observation: bool,
) -> bool {
    if let Some(clock_at_own) = clock_at_own {
        let desired_clock = desired
            .iter()
            .filter(|(key, _)| {
                key.0 == "connectors.clock.LocalClockFloor"
                    && !(runtime_state_only && registry_clock_key(key))
            })
            .map(|(key, row)| (key.clone(), row.clone()))
            .collect::<BTreeMap<_, _>>();
        let recorded_clock = clock_at_own
            .iter()
            .filter(|(key, _)| !(runtime_state_only && registry_clock_key(key)))
            .map(|(key, row)| (key.clone(), row.clone()))
            .collect::<BTreeMap<_, _>>();
        if desired_clock != recorded_clock {
            return false;
        }
    }
    desired.keys().chain(current.keys()).all(|key| {
        if prepared_observation && runtime_record_key(key) {
            // A runtime-record write holds the lifecycle lock but omits the
            // registry-clock guard, so it can land between this observation's
            // unlocked replay and its relock without failing the clock CAS.
            // The observation's own batch was refused above unless it changed
            // nothing but the clock, and no registry observation reads these
            // rows, so their recorded state says nothing about our projection.
            return true;
        }
        if runtime_state_only && registry_clock_key(key) {
            // The verified authored history may advance this unread clock
            // before or after our runtime-only batch. It may not change the
            // clock's owner, lifecycle or any field besides its lower bound.
            return desired.get(key).is_some_and(|desired| {
                current.get(key).is_some_and(|current| {
                    desired.0 == current.0
                        && desired.1.get("owner") == Some(&json!("registry"))
                        && desired.1.len() == current.1.len()
                        && desired.1.iter().all(|(name, value)| {
                            name == "lower_unix_ms" || current.1.get(name) == Some(value)
                        })
                        && desired
                            .1
                            .get("lower_unix_ms")
                            .and_then(Value::as_i64)
                            .zip(current.1.get("lower_unix_ms").and_then(Value::as_i64))
                            .is_some_and(|(before, after)| after >= before)
                })
            });
        }
        desired.get(key) == current.get(key)
            || (latest_positions.get(key).copied().unwrap_or(0) > last_own_position
                && (clock_at_own.is_none() || key.0 == "connectors.clock.LocalClockFloor"))
    })
}

fn registry_clock_key(key: &(String, String)) -> bool {
    key.0 == "connectors.clock.LocalClockFloor" && key.1 == "s:registry"
}

/// The only rows the runtime-state writer may change. That writer omits the
/// registry-clock guard, so these are also the only rows a prepared
/// observation's clock revision cannot fence; it never reads them either.
fn runtime_record_key(key: &(String, String)) -> bool {
    key.0 == "connectors.cli.LocalRuntimeRecord"
}

fn create_action(
    registry: &Registry,
    commands: &BTreeMap<String, CommandEntry>,
    next: &RowImage,
    command: &str,
) -> Result<BatchAction> {
    let binding = commands.get(command).ok_or(Failure::MetadataUnavailable)?;
    if binding.entity != next.entity || binding.operation.is_some() {
        return Err(Failure::MetadataUnavailable);
    }
    let definition = registry
        .get(&next.entity, 1)
        .ok_or(Failure::MetadataUnavailable)?;
    Ok(BatchAction::Create(CreateRequest {
        subject: Subject::new(&next.entity, &next.id).map_err(|_| Failure::MetadataUnavailable)?,
        definition_version: 1,
        fields: command_arguments(&definition.create.arguments, binding, next, next, None)?,
        recording: recording(command)?,
    }))
}

fn command_action(
    registry: &Registry,
    commands: &BTreeMap<String, CommandEntry>,
    all: &BTreeMap<(String, String), RowImage>,
    current: &RowImage,
    next: &RowImage,
    command: &str,
) -> Result<BatchAction> {
    let binding = commands.get(command).ok_or(Failure::MetadataUnavailable)?;
    if binding.entity != current.entity || current.entity != next.entity {
        return Err(Failure::MetadataUnavailable);
    }
    let operation = binding
        .operation
        .as_deref()
        .ok_or(Failure::MetadataUnavailable)?;
    let definition = registry
        .get(&current.entity, 1)
        .ok_or(Failure::MetadataUnavailable)?;
    let operation_definition = definition
        .operations
        .get(operation)
        .ok_or(Failure::MetadataUnavailable)?;
    let outcome = operation_definition
        .outcomes
        .iter()
        .find(|outcome| !outcome.wrong_state && outcome.refuses.is_none())
        .ok_or(Failure::MetadataUnavailable)?;
    let fulfillments = outcome
        .fulfills
        .keys()
        .map(|field| {
            let action = match next.fields.get(field) {
                Some(value) => OperationFieldAction::Set {
                    value: value.clone(),
                },
                None => OperationFieldAction::Remove,
            };
            (field.clone(), action)
        })
        .collect();
    let arguments = command_arguments(
        &operation_definition.arguments,
        binding,
        current,
        next,
        Some(all),
    )?;
    Ok(BatchAction::Execute(ExecuteRequest {
        subject: Subject::new(&current.entity, &current.id)
            .map_err(|_| Failure::MetadataUnavailable)?,
        expected_revision: current.revision,
        operation: operation.into(),
        arguments,
        fulfillments,
        recording: recording(command)?,
    }))
}

fn create_command(row: &RowImage) -> Result<&'static str> {
    match row.entity.as_str() {
        "connectors.clock.LocalClockFloor" => Ok("connectors.clock.RecordLocalClockFloor"),
        "connectors.declarations.ServiceConfiguration" => {
            Ok("connectors.declarations.RecordServiceConfiguration")
        }
        "connectors.auth_bindings.AuthProfile" => Ok("connectors.auth_bindings.RecordProfile"),
        "connectors.auth_bindings.Connection" => Ok("connectors.auth_bindings.AllocateConnection"),
        "connectors.auth_bindings.Acquisition" => Ok("connectors.auth_bindings.BeginAcquisition"),
        "connectors.auth_bindings.CustodyVersion" => {
            Ok("connectors.auth_bindings.RecordCustodyVersion")
        }
        "connectors.credentials.CredentialGeneration" => {
            Ok("connectors.credentials.RecordCredentialGeneration")
        }
        "connectors.credential_evidence.ReadUse" => {
            Ok("connectors.credential_evidence.CaptureReadUse")
        }
        "connectors.cli.ConnectionListCursor" => Ok("connectors.cli.RecordConnectionListCursor"),
        "connectors.cli.LocalRuntimeRecord" => Ok("connectors.cli.RecordLocalRuntime"),
        "connectors.mutations.AttemptRecord" => Ok("connectors.mutations.PrepareAttempt"),
        "connectors.idempotency.KeyReservation" => Ok("connectors.idempotency.ReserveKey"),
        "connectors.execution_audit.AuditRecord" => {
            Ok("connectors.execution_audit.AcknowledgeAnchor")
        }
        "connectors.delegation.ApprovalRedemption" => {
            Ok("connectors.delegation.RecordApprovalRedemption")
        }
        "connectors.approval_issuers.ApprovalIssuer" => {
            Ok("connectors.approval_issuers.BindIssuer")
        }
        "connectors.approval_issuers.ApprovalSigningKey" => {
            Ok("connectors.approval_issuers.StageKey")
        }
        "connectors.local_approval_policy.LocalApprovalPolicy" => {
            Ok("connectors.local_approval_policy.ConfigureLocalApprovalPolicy")
        }
        _ => Err(Failure::MetadataUnavailable),
    }
}

fn update_command(current: &RowImage, next: &RowImage) -> Result<&'static str> {
    let state = (
        current.lifecycle_state.as_str(),
        next.lifecycle_state.as_str(),
    );
    match (current.entity.as_str(), state) {
        ("connectors.clock.LocalClockFloor", ("Recorded", "Recorded")) => {
            Ok("connectors.clock.AdvanceLocalClockFloor")
        }
        ("connectors.declarations.ServiceConfiguration", ("Declared", "Declared")) => {
            Ok("connectors.declarations.AdvanceRegistryEpoch")
        }
        ("connectors.auth_bindings.Connection", ("Live", "Live")) => {
            Ok("connectors.auth_bindings.PublishBinding")
        }
        ("connectors.auth_bindings.Connection", ("Live", "Revoked")) => {
            Ok("connectors.auth_bindings.RevokeConnection")
        }
        ("connectors.auth_bindings.Acquisition", ("Pending", "Completing")) => {
            Ok("connectors.auth_bindings.ConsumeAcquisition")
        }
        ("connectors.auth_bindings.Acquisition", ("Completing", "Completing"))
            if current
                .fields
                .get("candidate_custody_version_ref")
                .is_none()
                && next.fields.get("candidate_custody_version_ref").is_some()
                && next.fields.len() == current.fields.len() + 1
                && current.fields.iter().all(|(name, value)| {
                    name == "candidate_custody_version_ref" || next.fields.get(name) == Some(value)
                }) =>
        {
            Ok("connectors.auth_bindings.RecordAcquisitionCandidate")
        }
        ("connectors.auth_bindings.Acquisition", ("Completing", "Completed")) => {
            Ok("connectors.auth_bindings.CompleteAcquisition")
        }
        ("connectors.auth_bindings.Acquisition", ("Pending" | "Completing", "Failed")) => {
            Ok("connectors.auth_bindings.FailAcquisition")
        }
        ("connectors.auth_bindings.Acquisition", ("Pending" | "Completing", "Expired")) => {
            Ok("connectors.auth_bindings.ExpireAcquisition")
        }
        ("connectors.auth_bindings.CustodyVersion", ("Candidate", "Stored")) => {
            Ok("connectors.auth_bindings.AcknowledgeCustodyWrite")
        }
        ("connectors.auth_bindings.CustodyVersion", ("Candidate" | "Stored", "Invalid")) => {
            Ok("connectors.auth_bindings.InvalidateCustodyVersion")
        }
        (
            "connectors.auth_bindings.CustodyVersion",
            ("Candidate" | "Stored" | "Invalid", "Retiring"),
        ) => Ok("connectors.auth_bindings.RetireCustodyVersion"),
        ("connectors.auth_bindings.CustodyVersion", ("Retiring", "Deleted")) => {
            Ok("connectors.auth_bindings.AcknowledgeCustodyDeletion")
        }
        ("connectors.credential_evidence.ReadUse", ("Captured", "Dispatched"))
            if read_use_transition(current, next, "dispatch_opened") =>
        {
            Ok("connectors.credential_evidence.DispatchReadUse")
        }
        ("connectors.credential_evidence.ReadUse", ("Captured" | "Dispatched", "Released"))
            if read_use_transition(current, next, "released") =>
        {
            Ok("connectors.credential_evidence.ReleaseReadUse")
        }
        ("connectors.cli.ConnectionListCursor", ("Active", "Expired")) => {
            Ok("connectors.cli.ExpireConnectionListCursor")
        }
        ("connectors.cli.LocalRuntimeRecord", ("Retained", "Retained")) => {
            if current.fields.get("suppressed") != next.fields.get("suppressed")
                && current.fields.get("selection") == next.fields.get("selection")
                && current.fields.get("bootstrap") == next.fields.get("bootstrap")
                && current.fields.get("observed_at") == next.fields.get("observed_at")
            {
                Ok("connectors.cli.SetLocalRuntimeSuppression")
            } else {
                Ok("connectors.cli.RememberLocalRuntimeBootstrap")
            }
        }
        ("connectors.mutations.AttemptRecord", ("Prepared", "Dispatching")) => {
            Ok("connectors.mutations.OpenDispatch")
        }
        ("connectors.mutations.AttemptRecord", ("Prepared", "Aborted")) => {
            Ok("connectors.mutations.AbortPrepared")
        }
        ("connectors.mutations.AttemptRecord", ("Dispatching", "Completed")) => {
            Ok("connectors.mutations.RecordCompletion")
        }
        ("connectors.mutations.AttemptRecord", ("Dispatching", "Failed")) => {
            Ok("connectors.mutations.RecordRefusal")
        }
        ("connectors.mutations.AttemptRecord", ("Dispatching", "Indeterminate")) => {
            Ok("connectors.mutations.RecordUncertainty")
        }
        ("connectors.idempotency.KeyReservation", ("Pending", "Replayable")) => {
            Ok("connectors.idempotency.RetainResult")
        }
        ("connectors.idempotency.KeyReservation", ("Pending", "Quarantined")) => {
            Ok("connectors.idempotency.QuarantineKey")
        }
        ("connectors.idempotency.KeyReservation", ("Replayable", "Expired")) => {
            Ok("connectors.idempotency.ExpireResult")
        }
        ("connectors.execution_audit.AuditRecord", ("Anchored", "FinalObserved")) => {
            Ok("connectors.execution_audit.AppendFinalObservation")
        }
        ("connectors.approval_issuers.ApprovalIssuer", ("Bound", "Bound")) => {
            Ok("connectors.approval_issuers.AdvanceIssuerRevision")
        }
        ("connectors.approval_issuers.ApprovalSigningKey", ("Candidate", "Active")) => {
            Ok("connectors.approval_issuers.PublishKey")
        }
        ("connectors.approval_issuers.ApprovalSigningKey", ("Candidate" | "Active", "Retired")) => {
            Ok("connectors.approval_issuers.RevokeKey")
        }
        (
            "connectors.approval_issuers.ApprovalSigningKey",
            ("Candidate" | "Retired", "Retiring"),
        ) => Ok("connectors.approval_issuers.RetireKey"),
        ("connectors.approval_issuers.ApprovalSigningKey", ("Retiring", "Deleted")) => {
            Ok("connectors.approval_issuers.ConfirmKeyDeletion")
        }
        ("connectors.local_approval_policy.LocalApprovalPolicy", ("Configured", "Configured")) => {
            Ok("connectors.local_approval_policy.ReviseLocalApprovalPolicy")
        }
        _ => Err(Failure::MetadataUnavailable),
    }
}

fn read_use_transition(current: &RowImage, next: &RowImage, changed: &str) -> bool {
    current.fields.get(changed) == Some(&json!(false))
        && next.fields.get(changed) == Some(&json!(true))
        && current.fields.len() == next.fields.len()
        && current
            .fields
            .iter()
            .all(|(name, value)| name == changed || next.fields.get(name) == Some(value))
}

fn removal_command(current: &RowImage) -> Result<Option<(&'static str, RowImage)>> {
    let command = match (current.entity.as_str(), current.lifecycle_state.as_str()) {
        ("connectors.cli.ConnectionListCursor", "Active") => {
            "connectors.cli.ExpireConnectionListCursor"
        }
        ("connectors.credential_evidence.ReadUse", "Captured" | "Dispatched" | "Released") => {
            "connectors.credential_evidence.ExpireReadUse"
        }
        _ => return Ok(None),
    };
    let mut next = current.clone();
    next.lifecycle_state = "Expired".into();
    Ok(Some((command, next)))
}

fn creation_stages(desired: &RowImage, admitted_registry_use: bool) -> Result<Vec<RowImage>> {
    if desired.entity == "connectors.credential_evidence.ReadUse"
        && (!admitted_registry_use
            || desired.lifecycle_state != "Captured"
            || desired.fields.get("dispatch_opened") != Some(&json!(false))
            || desired.fields.get("released") != Some(&json!(false)))
    {
        return Err(Failure::MetadataUnavailable);
    }
    let initial_state = match desired.entity.as_str() {
        "connectors.clock.LocalClockFloor" => "Recorded",
        "connectors.declarations.ServiceConfiguration" => "Declared",
        "connectors.auth_bindings.AuthProfile" => "Declared",
        "connectors.auth_bindings.Connection" => "Live",
        "connectors.auth_bindings.Acquisition" => "Pending",
        "connectors.auth_bindings.CustodyVersion" => "Candidate",
        "connectors.credentials.CredentialGeneration" => "Captured",
        "connectors.credential_evidence.ReadUse" => "Captured",
        "connectors.cli.ConnectionListCursor" => "Active",
        "connectors.cli.LocalRuntimeRecord" => "Retained",
        "connectors.mutations.AttemptRecord" => "Prepared",
        "connectors.idempotency.KeyReservation" => "Pending",
        "connectors.execution_audit.AuditRecord" => "Anchored",
        "connectors.delegation.ApprovalRedemption" => "Spent",
        "connectors.approval_issuers.ApprovalIssuer" => "Bound",
        "connectors.approval_issuers.ApprovalSigningKey" => "Candidate",
        "connectors.local_approval_policy.LocalApprovalPolicy" => "Configured",
        _ => return Err(Failure::MetadataUnavailable),
    };
    if desired.lifecycle_state == initial_state {
        let mut initial = desired.clone();
        initial.revision = 1;
        return Ok(vec![initial]);
    }

    Err(Failure::MetadataUnavailable)
}

fn command_arguments(
    schema: &entity_core::ObjectSchema,
    binding: &CommandEntry,
    current: &RowImage,
    next: &RowImage,
    all: Option<&BTreeMap<(String, String), RowImage>>,
) -> Result<Value> {
    let input_schema = schema
        .fields
        .get("input")
        .ok_or(Failure::MetadataUnavailable)?;
    let mut candidates = next.fields.clone();
    if input_schema.properties.contains_key("decision") {
        // These commands are reached only after the corresponding trusted host
        // owner has completed its admission/custody/audit predicate and committed
        // the exact SQL change being recorded. Refused and unavailable host
        // decisions commit no such business row and cannot reach this binding.
        let records_successful_owner_decision = matches!(
            binding.name.as_str(),
            "connectors.auth_bindings.AllocateConnection"
                | "connectors.auth_bindings.RevokeConnection"
                | "connectors.auth_bindings.PublishBinding"
                | "connectors.auth_bindings.BeginAcquisition"
                | "connectors.auth_bindings.ConsumeAcquisition"
                | "connectors.auth_bindings.CompleteAcquisition"
                | "connectors.auth_bindings.FailAcquisition"
                | "connectors.auth_bindings.ExpireAcquisition"
                | "connectors.auth_bindings.RecordCustodyVersion"
                | "connectors.auth_bindings.AcknowledgeCustodyWrite"
                | "connectors.auth_bindings.AcknowledgeCustodyDeletion"
                | "connectors.execution_audit.AcknowledgeAnchor"
                | "connectors.execution_audit.AppendFinalObservation"
        );
        if !records_successful_owner_decision {
            return Err(Failure::MetadataUnavailable);
        }
        candidates.insert("decision".into(), json!("allow"));
    }
    if input_schema.properties.contains_key("reason") {
        let reason = match binding.name.as_str() {
            "connectors.auth_bindings.FailAcquisition" => next.fields.get("terminal_reason"),
            "connectors.auth_bindings.InvalidateCustodyVersion" => {
                next.fields.get("invalid_reason")
            }
            _ => None,
        }
        .cloned()
        .ok_or(Failure::MetadataUnavailable)?;
        candidates.insert("reason".into(), reason);
    }
    let logical_id = next
        .id
        .strip_prefix("s:")
        .ok_or(Failure::MetadataUnavailable)?;
    if let Some(identity) = identity_field(&next.entity) {
        candidates.insert(identity.into(), json!(logical_id));
    }
    if input_schema.properties.contains_key("publication") {
        candidates.insert("publication".into(), publication_value(current, next, all)?);
    }
    let mut input = Map::new();
    for (name, field) in &input_schema.properties {
        match candidates.get(name) {
            Some(value) => {
                input.insert(name.clone(), value.clone());
            }
            None if field.required => return Err(Failure::MetadataUnavailable),
            None => {}
        }
    }
    let mut bound = Map::new();
    for slot in &binding.slots {
        let value = slot_value(slot, &input, current, next, logical_id)?;
        match value {
            Some(value) => {
                bound.insert(slot.slot.clone(), value);
            }
            None if slot.required => return Err(Failure::MetadataUnavailable),
            None => {}
        }
    }
    Ok(json!({"input": input, "bound": bound}))
}

fn identity_field(entity: &str) -> Option<&'static str> {
    match entity {
        "connectors.clock.LocalClockFloor" => Some("owner"),
        "connectors.declarations.ServiceConfiguration" => Some("instance_id"),
        "connectors.auth_bindings.AuthProfile" => Some("profile_record_ref"),
        "connectors.auth_bindings.Connection" => Some("connection_ref"),
        "connectors.auth_bindings.Acquisition" => Some("acquisition_ref"),
        "connectors.auth_bindings.CustodyVersion" => Some("version_ref"),
        "connectors.credentials.CredentialGeneration" => Some("generation_id"),
        "connectors.credential_evidence.ReadUse" => Some("use_id"),
        "connectors.cli.ConnectionListCursor" => Some("cursor_id"),
        "connectors.cli.LocalRuntimeRecord" => Some("instance_id"),
        "connectors.mutations.AttemptRecord" => Some("attempt_id"),
        "connectors.idempotency.KeyReservation" => Some("reservation_id"),
        "connectors.execution_audit.AuditRecord" => Some("audit_record_ref"),
        "connectors.delegation.ApprovalRedemption" => Some("receipt_id"),
        "connectors.approval_issuers.ApprovalIssuer" => Some("issuer_id"),
        "connectors.approval_issuers.ApprovalSigningKey" => Some("key_id"),
        "connectors.local_approval_policy.LocalApprovalPolicy" => Some("policy_id"),
        _ => None,
    }
}

fn publication_value(
    current: &RowImage,
    next: &RowImage,
    all: Option<&BTreeMap<(String, String), RowImage>>,
) -> Result<Value> {
    let (connection, connection_ref, expected_fence, profile_ref) =
        if next.entity == "connectors.auth_bindings.Acquisition" {
            let connection_ref = domain(next, "result_connection_ref")?
                .as_str()
                .ok_or(Failure::MetadataUnavailable)?;
            let connection = all
                .and_then(|rows| {
                    rows.get(&(
                        "connectors.auth_bindings.Connection".into(),
                        format!("s:{connection_ref}"),
                    ))
                })
                .ok_or(Failure::MetadataUnavailable)?;
            (
                connection,
                domain(next, "result_connection_ref")?,
                domain(current, "expected_publication_fence")?,
                domain(next, "profile_record_ref")?,
            )
        } else {
            (
                next,
                domain(next, "connection_ref")?,
                domain(current, "publication_fence")?,
                domain(next, "profile_record_ref")?,
            )
        };
    if domain(connection, "connection_ref")? != connection_ref
        || domain(connection, "profile_record_ref")? != profile_ref
    {
        return Err(Failure::MetadataUnavailable);
    }
    let mut publication = Map::new();
    publication.insert("connection_ref".into(), connection_ref.clone());
    publication.insert("expected_publication_fence".into(), expected_fence.clone());
    publication.insert("profile_record_ref".into(), profile_ref.clone());
    for (source, target) in [
        ("active_generation_id", "generation_id"),
        ("active_custody_version_ref", "custody_version_ref"),
        ("external_identity", "external_identity"),
    ] {
        if let Some(value) = connection.fields.get(source) {
            publication.insert(target.into(), value.clone());
        }
    }
    Ok(Value::Object(publication))
}

fn slot_value(
    slot: &SlotEntry,
    input: &Map<String, Value>,
    current: &RowImage,
    next: &RowImage,
    logical_id: &str,
) -> Result<Option<Value>> {
    let _outcome = slot.outcome.as_deref();
    let field_value = slot.field.as_deref().and_then(|field| {
        input
            .get(field)
            .or_else(|| next.fields.get(field))
            .or_else(|| current.fields.get(field))
            .cloned()
    });
    let field_value = field_value.or_else(|| {
        if current.entity == "connectors.execution_audit.AuditRecord"
            && slot.target == "event_field"
            && slot.field.as_deref() == Some("observation_id")
        {
            return input
                .get("final_observation")
                .and_then(Value::as_object)
                .and_then(|observation| observation.get("observation_id"))
                .cloned();
        }
        None
    });
    let value = match slot.target.as_str() {
        "logical_identity" => Some(json!(logical_id)),
        "entity_field" | "event_field" | "response_field" => field_value,
        "external_evidence" => None,
        _ => return Err(Failure::MetadataUnavailable),
    };
    if value.is_none() && slot.source == "generated" {
        return Ok(Some(json!(uuid::Uuid::new_v4().to_string())));
    }
    Ok(value)
}

fn map_execution_failure(error: entity_eventlog::sync::SyncExecutionError) -> Failure {
    match error {
        entity_eventlog::sync::SyncExecutionError::Execution(ExecutionError::Store(
            AsyncStoreError::RevisionConflict { .. },
        ))
        | entity_eventlog::sync::SyncExecutionError::Execution(ExecutionError::Write(
            WriteFailure::NotCommitted(AsyncStoreError::RevisionConflict { .. }),
        )) => Failure::ConcurrentRevision,
        entity_eventlog::sync::SyncExecutionError::Execution(ExecutionError::Write(
            WriteFailure::Uncertain { .. },
        )) => Failure::OutcomeUnknown,
        entity_eventlog::sync::SyncExecutionError::Rejected(_)
        | entity_eventlog::sync::SyncExecutionError::Execution(_) => Failure::MetadataUnavailable,
    }
}

fn projection_from_connection(connection: &Connection) -> Result<ProjectionRows> {
    let level = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(unavailable)?;
    Ok(capture(connection, level)?
        .into_iter()
        .map(|row| ((row.entity, row.id), (row.lifecycle_state, row.fields)))
        .collect())
}

fn visible_rows(rows: &BTreeMap<(String, String), RowImage>) -> ProjectionRows {
    rows.iter()
        .filter(|(_, row)| {
            TABLES
                .iter()
                .any(|table| table.entity == row.entity && projects(table, row))
        })
        .map(|(key, row)| {
            (
                key.clone(),
                (row.lifecycle_state.clone(), row.fields.clone()),
            )
        })
        .collect()
}

fn recording(label: &str) -> Result<Recording> {
    Ok(Recording {
        record_id: format!("connectors-{label}-{}", uuid::Uuid::new_v4()),
        recorded_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| Failure::MetadataUnavailable)?,
        correlation: Some("connectors-local-metadata".into()),
        causation: None,
        actor: Some("connectors-host".into()),
    })
}

fn context(label: &str) -> EventlogOperationContext {
    let id = uuid::Uuid::new_v4();
    EventlogOperationContext {
        subject: "connectors-local-user".into(),
        actor: "connectors-host".into(),
        request_id: format!("connectors-{label}-{id}"),
        trace_id: format!("connectors-metadata-{id}"),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
    }
}

fn call_wait() -> CallWait {
    CallWait::Until(Instant::now() + Duration::from_secs(30))
}

pub(super) fn migrations() -> impl Iterator<Item = (i64, &'static str)> {
    [
        (2, REGISTRY_MIGRATION),
        (3, RUNTIME_MIGRATION),
        (4, MUTATION_MIGRATION),
        (5, AUDIT_MIGRATION),
        (6, APPROVAL_MIGRATION),
        (7, APPROVAL_KEYS_MIGRATION),
        (8, APPROVAL_POLICY_MIGRATION),
    ]
    .into_iter()
}

pub(super) fn migration_digest() -> String {
    hex::encode(Sha256::digest(MIGRATION.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity_store::asynchronous::RecordedEntry;

    #[test]
    fn postcommit_business_projection_requires_its_own_clock_state() {
        let clock = (
            "connectors.clock.LocalClockFloor".to_owned(),
            "i:1".to_owned(),
        );
        let business = (
            "connectors.declarations.ServiceConfiguration".to_owned(),
            "s:fixture".to_owned(),
        );
        let clock_row = |time| {
            (
                "Recorded".to_owned(),
                json!({"owner":"registry","last_seen_ms":time})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
        };
        let business_row = (
            "Declared".to_owned(),
            json!({"instance_id":"fixture"})
                .as_object()
                .unwrap()
                .clone(),
        );
        let desired = BTreeMap::from([
            (clock.clone(), clock_row(10)),
            (business.clone(), business_row.clone()),
        ]);
        let mut current = desired.clone();
        current.insert(clock.clone(), clock_row(20));
        let own_clock = BTreeMap::from([(clock.clone(), clock_row(10))]);
        let later = BTreeMap::from([(clock.clone(), 11), (business.clone(), 10)]);
        assert!(postcommit_projection_matches(
            &desired,
            &current,
            Some(&own_clock),
            &later,
            10,
            false,
            false,
        ));

        let wrong_own_clock = BTreeMap::from([(clock.clone(), clock_row(9))]);
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&wrong_own_clock),
            &later,
            10,
            false,
            false,
        ));
        let stale_clock = BTreeMap::from([(clock.clone(), 10)]);
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&own_clock),
            &stale_clock,
            10,
            false,
            false,
        ));
        current.insert(business.clone(), ("Changed".to_owned(), business_row.1));
        let later_business = BTreeMap::from([(clock, 11), (business, 12)]);
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&own_clock),
            &later_business,
            10,
            false,
            false,
        ));
    }

    #[test]
    fn postcommit_runtime_state_accepts_only_an_authored_registry_clock_advance() {
        let registry = (
            "connectors.clock.LocalClockFloor".to_owned(),
            "s:registry".to_owned(),
        );
        let mutations = (
            "connectors.clock.LocalClockFloor".to_owned(),
            "s:mutations".to_owned(),
        );
        let runtime = (
            "connectors.cli.LocalRuntimeRecord".to_owned(),
            "fixture".to_owned(),
        );
        let clock = |owner, lower| {
            (
                "Recorded".to_owned(),
                json!({"owner":owner,"lower_unix_ms":lower})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
        };
        let runtime_row = (
            "Retained".to_owned(),
            json!({"instance_id":"fixture","suppressed":false})
                .as_object()
                .unwrap()
                .clone(),
        );
        let desired = BTreeMap::from([
            (registry.clone(), clock("registry", 10)),
            (mutations.clone(), clock("mutations", 5)),
            (runtime.clone(), runtime_row.clone()),
        ]);
        let mut current = desired.clone();
        current.insert(registry.clone(), clock("registry", 20));
        let at_own = BTreeMap::from([
            (registry.clone(), clock("registry", 20)),
            (mutations.clone(), clock("mutations", 5)),
        ]);
        let positions = BTreeMap::from([(registry.clone(), 9), (mutations.clone(), 5)]);
        assert!(postcommit_projection_matches(
            &desired,
            &current,
            Some(&at_own),
            &positions,
            10,
            true,
            false,
        ));

        // A later-position marker cannot mask a regressed or otherwise changed
        // registry clock on this narrow path.
        current.insert(registry.clone(), clock("registry", 9));
        let later = BTreeMap::from([(registry.clone(), 11)]);
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&at_own),
            &later,
            10,
            true,
            false,
        ));
        current.insert(registry, clock("registry", 20));
        let wrong_mutation = BTreeMap::from([(mutations, clock("mutations", 4))]);
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&wrong_mutation),
            &positions,
            10,
            true,
            false,
        ));
    }

    #[test]
    fn postcommit_prepared_observation_ignores_only_unread_runtime_records() {
        let registry = (
            "connectors.clock.LocalClockFloor".to_owned(),
            "s:registry".to_owned(),
        );
        let business = (
            "connectors.declarations.ServiceConfiguration".to_owned(),
            "s:fixture".to_owned(),
        );
        let runtime = (
            "connectors.cli.LocalRuntimeRecord".to_owned(),
            "fixture".to_owned(),
        );
        let clock = |lower| {
            (
                "Recorded".to_owned(),
                json!({"owner":"registry","lower_unix_ms":lower})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
        };
        let row = |state: &str, suppressed| {
            (
                state.to_owned(),
                json!({"instance_id":"fixture","suppressed":suppressed})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
        };
        // The observation replayed before a runtime-record write landed at
        // position 8, then committed only its clock revision at position 10.
        let desired = BTreeMap::from([
            (registry.clone(), clock(10)),
            (business.clone(), row("Declared", false)),
        ]);
        let mut current = desired.clone();
        current.insert(runtime.clone(), row("Retained", true));
        let at_own = BTreeMap::from([(registry.clone(), clock(10))]);
        let positions = BTreeMap::from([
            (registry.clone(), 10),
            (business.clone(), 3),
            (runtime.clone(), 8),
        ]);
        assert!(postcommit_projection_matches(
            &desired,
            &current,
            Some(&at_own),
            &positions,
            10,
            false,
            true,
        ));
        // A changed retained record is the same unread difference.
        let mut desired_with_runtime = desired.clone();
        desired_with_runtime.insert(runtime.clone(), row("Retained", false));
        assert!(postcommit_projection_matches(
            &desired_with_runtime,
            &current,
            Some(&at_own),
            &positions,
            10,
            false,
            true,
        ));
        // A business write holds the lock across its replay, so the same
        // difference there is a bad own projection.
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&at_own),
            &positions,
            10,
            false,
            false,
        ));
        // Any registry row the observation could have read still refuses.
        current.insert(business.clone(), row("Changed", false));
        assert!(!postcommit_projection_matches(
            &desired,
            &current,
            Some(&at_own),
            &positions,
            10,
            false,
            true,
        ));
    }

    fn prepared_attempt() -> RowImage {
        let attempt_id = "5cff4e3d-a599-4ca5-9715-a92c22ced4fe";
        let owner_nonce = "cf76c83b-e102-42f0-9759-bdad22102a09";
        let request_fingerprint = json!({
            "operation_ref": "[\"connectors.operation/v1\",\"instance\",\"adapter\",\"write\"]",
            "connection_ref": "connection",
            "connection_revision": "revision",
            "contract_ref": "contract",
            "profile": "profile",
            "descriptor_revision": "descriptor",
            "configuration_revision": "configuration",
            "canonicalization_version": "1",
            "input_digest": "digest",
        });
        RowImage {
            entity: "connectors.mutations.AttemptRecord".into(),
            id: format!("s:{attempt_id}"),
            revision: 1,
            lifecycle_state: "Prepared".into(),
            fields: json!({
                "attempt_id": attempt_id,
                "instance_id": "instance",
                "request_id": "request",
                "operation_id": "write",
                "connection_ref": "connection",
                "input_digest": "digest",
                "request_fingerprint": request_fingerprint,
                "approval_mode": "not_required",
                "owner_nonce": owner_nonce,
                "publication_fence": "fence",
            })
            .as_object()
            .unwrap()
            .clone(),
        }
    }

    fn transition(
        authority: &ErAuthority,
        current: &RowImage,
        next: &RowImage,
        command: &str,
    ) -> std::result::Result<(), entity_eventlog::sync::SyncExecutionError> {
        let bundle = definition_bundle().unwrap();
        let commands = bundle
            .commands
            .into_iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let desired = BTreeMap::from([((next.entity.clone(), next.id.clone()), next.clone())]);
        let action = command_action(
            &registry().unwrap(),
            &commands,
            &desired,
            current,
            next,
            command,
        )
        .unwrap();
        authority
            .facade
            .execute_batch(
                context("attempt-lifecycle-test"),
                BatchKey::Named(format!("attempt-lifecycle-{}", uuid::Uuid::new_v4())),
                vec![action],
                call_wait(),
            )
            .map(|_| ())
    }

    #[test]
    fn final_projection_row_cannot_invent_missing_lifecycle_history() {
        let mut completed = prepared_attempt();
        completed.lifecycle_state = "Completed".into();
        completed
            .fields
            .insert("settled_at".into(), json!("2026-09-17T00:00:00Z"));
        completed
            .fields
            .insert("terminal_result_json".into(), json!("{}"));
        assert!(creation_stages(&completed, false).is_err());
    }

    #[test]
    fn read_use_creation_requires_the_admitted_registry_operation() {
        let mut use_row = prepared_attempt();
        use_row.entity = "connectors.credential_evidence.ReadUse".into();
        use_row.id = "s:13a79444-7f32-4dc8-b753-978483e202e1".into();
        use_row.lifecycle_state = "Captured".into();
        use_row.fields = json!({"dispatch_opened": false, "released": false})
            .as_object()
            .unwrap()
            .clone();
        assert!(creation_stages(&use_row, false).is_err());
        let stages = creation_stages(&use_row, true).unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].lifecycle_state, "Captured");
        use_row.fields.insert("dispatch_opened".into(), json!(true));
        assert!(creation_stages(&use_row, true).is_err());
        use_row
            .fields
            .insert("dispatch_opened".into(), json!(false));
        use_row.fields.insert("released".into(), json!(true));
        assert!(creation_stages(&use_row, true).is_err());
    }

    #[test]
    fn attempt_transitions_are_decided_recorded_replayed_and_rebuilt_by_er() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("metadata.sqlite3");
        let prepared = prepared_attempt();
        let (mut authority, coordinates, _) =
            provision(&path, uuid::Uuid::new_v4(), 4, vec![prepared.clone()]).unwrap();

        let mut completed = prepared.clone();
        completed.lifecycle_state = "Completed".into();
        completed
            .fields
            .insert("settled_at".into(), json!("2026-09-17T00:00:00Z"));
        completed
            .fields
            .insert("terminal_result_json".into(), json!("{}"));
        assert!(
            transition(
                &authority,
                &prepared,
                &completed,
                "connectors.mutations.RecordCompletion"
            )
            .is_err()
        );

        let mut dispatching = prepared.clone();
        dispatching.lifecycle_state = "Dispatching".into();
        dispatching.revision = 2;
        transition(
            &authority,
            &prepared,
            &dispatching,
            "connectors.mutations.OpenDispatch",
        )
        .unwrap();
        completed.revision = 3;
        transition(
            &authority,
            &dispatching,
            &completed,
            "connectors.mutations.RecordCompletion",
        )
        .unwrap();
        assert!(
            transition(
                &authority,
                &completed,
                &dispatching,
                "connectors.mutations.OpenDispatch"
            )
            .is_err()
        );

        authority.baseline = snapshot_rows(&authority.facade).unwrap();
        let snapshot = authority.facade.complete_snapshot(call_wait()).unwrap();
        let history = snapshot
            .histories
            .iter()
            .find(|subject| subject.terminal.id == prepared.id)
            .unwrap();
        let decisions = history
            .history
            .records
            .iter()
            .filter_map(|record| match &record.entry {
                RecordedEntry::Decision(commit) => Some(&commit.envelope.record),
                RecordedEntry::Observation(_) => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].to_state, "Dispatching");
        assert_eq!(
            decisions[0].events[0].event_type,
            "connectors.mutations.DispatchOpened"
        );
        assert_eq!(decisions[1].to_state, "Completed");
        assert_eq!(
            decisions[1].events[0].event_type,
            "connectors.mutations.AttemptCompleted"
        );

        // This isolated ER subject has no imported Connection foreign-key
        // graph. Rebuild its SQL row here; the full metadata migration tests
        // exercise the complete relational projection.
        let terminal = authority
            .baseline
            .get(&(prepared.entity.clone(), prepared.id.clone()))
            .unwrap();
        let mut projected = Map::new();
        attempt_projection(terminal, &mut projected).unwrap();
        assert_eq!(projected["state"], "completed");
        assert_eq!(projected["attempt_id"], prepared.fields["attempt_id"]);
        drop(authority);

        let (reopened, _) = open(&path, coordinates, 4, 4).unwrap();
        let replayed = reopened.facade.complete_snapshot(call_wait()).unwrap();
        let terminal = replayed
            .histories
            .iter()
            .find(|subject| subject.terminal.id == prepared.id)
            .unwrap();
        assert_eq!(terminal.terminal.lifecycle_state, "Completed");
        assert_eq!(terminal.history.records.len(), 2);
    }
}
