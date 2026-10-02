//! Test-only observation of the existing recorded authority. No write API,
//! replacement authority, legacy business SQL or production inspection export.
use entity_core::{EntityDefinition, EntityInstance, Registry};
use entity_eventlog::{
    Authority, RecordedProviderFacade,
    sync::{BridgeConfig, CallWait, EventlogRecordedStoreOwner, ShutdownMode, ShutdownOutcome},
};
use entity_store::asynchronous::StoreCoverage;
use eventlog_core::CaptureLimits;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU16,
    path::Path,
    time::{Duration, Instant},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const ATTEMPT: &str = "connectors.mutations.AttemptRecord";
const KEY: &str = "connectors.idempotency.KeyReservation";
const AUDIT: &str = "connectors.execution_audit.AuditRecord";
const REDEMPTION: &str = "connectors.delegation.ApprovalRedemption";
const SCOPE: &str = "connectors.local-metadata/1";

#[derive(Deserialize)]
struct Bundle {
    format: String,
    definitions: Vec<Definition>,
}
#[derive(Deserialize)]
struct Definition {
    name: String,
    definition: EntityDefinition,
}

pub(super) struct Reader {
    facade: RecordedProviderFacade,
    authority: String,
    fields: BTreeMap<String, BTreeSet<String>>,
    states: BTreeMap<String, BTreeSet<String>>,
}
impl Reader {
    pub(super) fn open(state: &Path) -> Self {
        drop(connectors_host::local::metadata::Metadata::inspect(state).unwrap());
        let database = rusqlite::Connection::open_with_flags(
            state.join("metadata.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        let version: i64 = database
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 9);
        let (id, scope, tenant, stream): (String, String, String, String) = database.query_row(
            "SELECT authority_id,logical_scope,tenant,stream_identity FROM connectors_er_authority WHERE singleton=1",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap();
        assert_eq!(scope, SCOPE);
        assert_eq!(tenant, id);
        drop(database);
        let bundle: Bundle = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/connectors-host/src/local/metadata/entity-runtime-definitions.json"
        )))
        .unwrap();
        assert_eq!(bundle.format, "connectors.entity-runtime-definitions/1");
        let mut registry = Registry::new();
        let mut fields = BTreeMap::new();
        let mut states = BTreeMap::new();
        for entry in bundle.definitions {
            assert_eq!(entry.name, entry.definition.entity);
            states.insert(
                entry.name.clone(),
                entry.definition.lifecycle.states.iter().cloned().collect(),
            );
            fields.insert(
                entry.name,
                entry.definition.schema.fields.keys().cloned().collect(),
            );
            registry.register(entry.definition).unwrap();
        }
        registry.validate_all().unwrap();
        let facade = RecordedProviderFacade::start(
            registry,
            EventlogRecordedStoreOwner::Sqlite {
                path: state.join("metadata.sqlite3").to_str().unwrap().into(),
                prefix: "connectors_er".into(),
                authority: Authority {
                    logical_scope: scope,
                    tenant,
                    stream_identity: stream,
                },
                limits: CaptureLimits {
                    max_events: 20_000,
                    max_blobs: 20_000,
                    max_projection_rows: 100_000,
                    max_payload_bytes: 128 * 1024 * 1024,
                },
            },
            BridgeConfig {
                queue_capacity: NonZeroU16::new(1).unwrap(),
            },
        )
        .unwrap();
        Self {
            facade,
            authority: id,
            fields,
            states,
        }
    }

    pub(super) fn snapshot(&self) -> Facts {
        let snapshot = self
            .facade
            .complete_snapshot(CallWait::Until(Instant::now() + Duration::from_secs(30)))
            .unwrap();
        assert_eq!(snapshot.scope, SCOPE);
        assert_eq!(snapshot.coverage, StoreCoverage::CompleteSnapshot);
        let mut rows = BTreeMap::new();
        for subject in snapshot.histories {
            let terminal = subject.terminal;
            assert_eq!(subject.history.subject.entity, terminal.entity);
            assert_eq!(subject.history.subject.id, terminal.id);
            let allowed = self.fields.get(&terminal.entity).expect("unknown entity");
            assert!(
                self.states[&terminal.entity].contains(&terminal.lifecycle_state),
                "unknown recorded lifecycle state"
            );
            assert!(
                terminal.fields.keys().all(|field| allowed.contains(field)),
                "unknown recorded field"
            );
            assert!(
                rows.insert((terminal.entity.clone(), terminal.id.clone()), terminal)
                    .is_none(),
                "duplicate recorded subject"
            );
        }
        Facts {
            authority: self.authority.clone(),
            rows,
        }
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        let outcome = self.facade.shutdown(
            ShutdownMode::Drain,
            CallWait::Until(Instant::now() + Duration::from_secs(30)),
        );
        if std::thread::panicking() {
            eprintln!("catalog observer shutdown during unwinding: {outcome:?}");
        } else {
            assert!(
                matches!(outcome, ShutdownOutcome::Joined { provider: Ok(()) }),
                "observer did not retire: {outcome:?}"
            );
        }
    }
}

pub(super) struct Facts {
    authority: String,
    rows: BTreeMap<(String, String), EntityInstance>,
}
pub(super) type StoredAttempt = (String, String, Option<i64>, Option<i64>);
pub(super) type RecoveryState = (String, String, Option<i64>, Option<String>);
impl Facts {
    pub(super) fn assert_unchanged(&self, after: &Self) {
        assert_eq!(self.authority, after.authority);
        assert_eq!(
            serde_json::to_value(self.rows.values().collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(after.rows.values().collect::<Vec<_>>()).unwrap()
        );
    }
    pub(super) fn evidence(&self, mode: u8, phase: &str) {
        // All four acceptance entity collections are serialized; unrelated
        // metadata is not. These rows hold references, not credentials/proofs.
        // Retained nocapture output is the explicit fixture evidence sink.
        eprintln!(
            "catalog mode {mode} {phase} all typed Attempt/Key/Audit/Redemption terminal rows from validated CompleteSnapshot capture (not full capture/event-history archive): {}",
            json!({
                "format":"catalog-recorded-fixture-snapshot/1", "authority":self.authority,
                "scope":SCOPE,"capture_coverage":"CompleteSnapshot","serialized_entities":[ATTEMPT,KEY,AUDIT,REDEMPTION],"terminals":self.rows.values().filter(|row| [ATTEMPT,KEY,AUDIT,REDEMPTION].contains(&row.entity.as_str())).collect::<Vec<_>>()
            })
        );
    }
    pub(super) fn pending_acquisitions(&self) -> BTreeSet<String> {
        self.entities("connectors.auth_bindings.Acquisition")
            .filter(|row| {
                text(row, "instance_id") == "fixture-gitlab" && row.lifecycle_state == "Pending"
            })
            .map(|row| logical_id(row, "acquisition_ref").into())
            .collect()
    }
    pub(super) fn assert_seeded(&self, reference: &Value, keyed: bool) {
        assert_eq!(reference["authority"], self.authority);
        let id = reference["id"].as_str().unwrap();
        let attempt = self.attempt(id);
        assert_eq!(attempt.lifecycle_state, "Prepared");
        assert_eq!(
            text(attempt, "request_id"),
            "a6b7af40-a60f-4a73-a4b2-fd247c773c11"
        );
        assert_eq!(self.key(attempt).is_some(), keyed);
        assert_eq!(timestamp(attempt, "settled_at"), None);
    }
    fn entities<'a>(&'a self, entity: &'a str) -> impl Iterator<Item = &'a EntityInstance> {
        self.rows.values().filter(move |row| row.entity == entity)
    }
    fn attempt(&self, id: &str) -> &EntityInstance {
        let found: Vec<_> = self
            .entities(ATTEMPT)
            .filter(|row| logical_id(row, "attempt_id") == id)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "exact logical attempt missing or duplicated"
        );
        found[0]
    }
    fn key(&self, attempt: &EntityInstance) -> Option<&EntityInstance> {
        let attempt_id = logical_id(attempt, "attempt_id");
        assert!(matches!(
            attempt.lifecycle_state.as_str(),
            "Prepared" | "Dispatching" | "Aborted" | "Completed" | "Failed" | "Indeterminate"
        ));
        let keys: Vec<_> = self
            .entities(KEY)
            .filter(|row| text(row, "attempt_id") == attempt_id)
            .collect();
        assert!(keys.len() <= 1, "multiple reservations for an attempt");
        if let Some(key) = keys.first() {
            logical_id(key, "reservation_id");
            assert!(matches!(
                key.lifecycle_state.as_str(),
                "Pending" | "Replayable" | "Quarantined" | "Expired"
            ));
            assert_eq!(
                key.fields["fingerprint"],
                attempt.fields["request_fingerprint"]
            );
            assert_eq!(
                key.fields["namespace"]["receiver_instance"],
                attempt.fields["instance_id"]
            );
            assert_eq!(key.fields["caller_key"], attempt.fields["idempotency_key"]);
            assert_eq!(
                timestamp(attempt, "settled_at"),
                timestamp(key, "settled_at")
            );
        }
        keys.into_iter().next()
    }
    pub(super) fn original(&self, id: &str) -> StoredAttempt {
        let attempt = self.attempt(id);
        let key = self.key(attempt).expect("original key missing");
        (
            attempt.lifecycle_state.to_ascii_lowercase(),
            key.lifecycle_state.to_ascii_lowercase(),
            timestamp(key, "settled_at"),
            timestamp(key, "replay_expires_at"),
        )
    }
    pub(super) fn only_attempt(&self) -> StoredAttempt {
        let attempts: Vec<_> = self
            .entities(ATTEMPT)
            .filter(|row| text(row, "instance_id") == "fixture-gitlab")
            .collect();
        assert_eq!(
            attempts.len(),
            1,
            "observation requires exactly one original attempt"
        );
        self.original(logical_id(attempts[0], "attempt_id"))
    }
    pub(super) fn states(&self) -> Vec<RecoveryState> {
        let mut rows: Vec<RecoveryState> = self
            .entities(ATTEMPT)
            .filter(|row| text(row, "instance_id") == "fixture-gitlab")
            .map(|row| {
                (
                    text(row, "request_id").into(),
                    row.lifecycle_state.to_ascii_lowercase(),
                    timestamp(row, "settled_at"),
                    self.key(row)
                        .map(|key| key.lifecycle_state.to_ascii_lowercase()),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    }
    pub(super) fn audits(&self) -> Vec<Value> {
        self.entities(AUDIT)
            .filter(|row| text(row, "instance_id") == "fixture-gitlab")
            .map(|row| {
                let reference = connectors_host::local::audit::Reference {
                    instance: text(row, "instance_id").into(),
                    audit_ref: text(row, "audit_ref").into(),
                };
                assert_eq!(
                    reference.encode_private().unwrap(),
                    text(row, "audit_record_ref")
                );
                logical_id(row, "audit_record_ref");
                let mut anchor = serde_json::Map::new();
                for (source, target) in [
                    ("instance_id", "instance_id"),
                    ("anchor_kind", "kind"),
                    ("hop_role", "hop"),
                    ("stage", "stage"),
                ] {
                    anchor.insert(target.into(), row.fields[source].clone());
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
                    json!(timestamp(row, "recorded_at").unwrap()),
                );
                let final_observation = row
                    .fields
                    .get("final_observation")
                    .map(|value| {
                        let mut value = value.as_object().unwrap().clone();
                        let recorded = value.remove("recorded_at").unwrap();
                        value.insert("recorded_at_ms".into(), json!(millis(&recorded)));
                        value.entry("code").or_insert(Value::Null);
                        Value::Object(value)
                    })
                    .unwrap_or(Value::Null);
                let record: connectors_host::local::audit::Record = serde_json::from_value(json!({
                "reference": reference, "anchor": anchor, "final_observation": final_observation,
            })).unwrap();
                serde_json::to_value(record).unwrap()
            })
            .collect()
    }
    /// Independent recorded-state and owning-port agreement at a quiescent stage.
    pub(super) fn cross_check(&self, state: &Path) {
        use connectors_host::local::{audit, mutations};
        struct NoClock;
        impl mutations::Clock for NoClock {
            fn now(&self) -> mutations::Result<mutations::ClockInterval> {
                panic!("observation used clock")
            }
        }
        let store = mutations::Store::new(state, NoClock, mutations::Limits::default()).unwrap();
        for row in self
            .entities(ATTEMPT)
            .filter(|row| text(row, "instance_id") == "fixture-gitlab")
        {
            let reference = mutations::AttemptRef {
                authority: self.authority.parse().unwrap(),
                attempt_id: logical_id(row, "attempt_id").parse().unwrap(),
            };
            let actual = store.observe(reference).unwrap();
            assert_eq!(format!("{:?}", actual.state), row.lifecycle_state);
            assert_eq!(actual.request_id, text(row, "request_id"));
            assert_eq!(actual.settled_at_ms, timestamp(row, "settled_at"));
            let key = self.key(row);
            assert_eq!(
                actual.reservation_id.map(|v| v.to_string()),
                key.map(|k| logical_id(k, "reservation_id").to_owned())
            );
            assert_eq!(
                actual.replay_expires_at_ms,
                key.and_then(|k| timestamp(k, "replay_expires_at"))
            );
            assert_eq!(
                actual.result,
                row.fields
                    .get("terminal_result_json")
                    .map(|v| serde_json::from_str::<Value>(v.as_str().unwrap()).unwrap())
            );
        }
        let store = audit::Store::new(state, 100_000).unwrap();
        for value in self.audits() {
            let reference = serde_json::from_value(value["reference"].clone()).unwrap();
            assert_eq!(
                serde_json::to_value(store.observe(&reference).unwrap().unwrap()).unwrap(),
                value
            );
        }
    }
    pub(super) fn assert_dispatched_and_spent(&self) -> String {
        let attempts: Vec<_> = self
            .entities(ATTEMPT)
            .filter(|row| text(row, "instance_id") == "fixture-gitlab")
            .collect();
        assert_eq!(attempts.len(), 1);
        let attempt = attempts[0];
        assert_eq!(
            self.original(logical_id(attempt, "attempt_id")),
            ("dispatching".into(), "pending".into(), None, None)
        );
        assert_eq!(text(attempt, "operation_id"), "merge_request.merge");
        let redemptions: Vec<_> = self
            .entities(REDEMPTION)
            .filter(|row| text(row, "attempt_id") == logical_id(attempt, "attempt_id"))
            .collect();
        assert_eq!(redemptions.len(), 1);
        assert_eq!(redemptions[0].lifecycle_state, "Spent");
        assert_eq!(
            redemptions[0].fields["instance_id"],
            attempt.fields["instance_id"]
        );
        let audits = self.audits();
        assert_eq!(audits.len(), 1);
        assert!(audits[0]["final_observation"].is_null());
        assert_eq!(
            audits[0]["anchor"]["connection_ref"],
            attempt.fields["connection_ref"]
        );
        // The production anchor precedes preparation and intentionally has no
        // attempt id. Its exact request identifies the later ledger attempt.
        assert!(audits[0]["anchor"]["attempt_id"].is_null());
        assert_eq!(
            audits[0]["anchor"]["request_id"],
            attempt.fields["request_id"]
        );
        assert_eq!(
            audits[0]["anchor"]["operation_id"],
            attempt.fields["operation_id"]
        );
        logical_id(attempt, "attempt_id").into()
    }
}
// Production metadata uses a typed-string ER subject id and retains the exact
// business reference in its declared identity field (metadata/er.rs row/create).
fn logical_id<'a>(row: &'a EntityInstance, field: &str) -> &'a str {
    let id = text(row, field);
    assert_eq!(
        row.id,
        format!("s:{id}"),
        "subject/declared identity mismatch"
    );
    id
}
fn text<'a>(row: &'a EntityInstance, name: &str) -> &'a str {
    row.fields
        .get(name)
        .unwrap_or_else(|| panic!("missing {name}"))
        .as_str()
        .unwrap()
}
fn millis(value: &Value) -> i64 {
    let timestamp = OffsetDateTime::parse(value.as_str().unwrap(), &Rfc3339).unwrap();
    i64::try_from(timestamp.unix_timestamp_nanos() / 1_000_000).unwrap()
}
fn timestamp(row: &EntityInstance, name: &str) -> Option<i64> {
    row.fields.get(name).map(millis)
}
