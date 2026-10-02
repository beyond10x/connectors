//! Metamorphic relations over the local metadata authority (ESS hardening
//! technique 5). Each relation comes from a design document, not from the
//! ESS sources, and compares what two runs leave behind: the SQL
//! compatibility projection and the Entity Runtime subjects recorded for the
//! ESS entities lowered in `connectors-build/src/metadata_entities.rs`.
//!
//! A "view" below is every compatibility table of a freshly opened authority
//! plus every recorded Entity Runtime subject (entity, id, lifecycle state and
//! fields). Views are always read through a new `Metadata::inspect`, so they
//! are what a restarted owner would observe.
use super::{Metadata, er};
use crate::local::{
    approval_keys, approval_policy, approvals, audit, filesystem as fs, keyring::custody,
    mutations, registry, runtime,
};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const NOW: u64 = 1_789_056_000_000;
const INSTANCE: &str = "fixture-instance";
const ADAPTER: &str = "adapter";
const CONFIG: &str = "config";

type View = BTreeMap<String, Vec<Value>>;

// ---------------------------------------------------------------- views

fn sql_value(value: rusqlite::types::ValueRef<'_>) -> Value {
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => json!(v),
        ValueRef::Real(v) => json!(v),
        ValueRef::Text(v) => json!(String::from_utf8_lossy(v)),
        ValueRef::Blob(v) => json!(hex::encode(v)),
    }
}

fn view_of(metadata: &Metadata) -> View {
    let mut out = View::new();
    let tables = metadata
        .connection
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    for table in tables {
        let mut statement = metadata
            .connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let names = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut rows = statement
            .query_map([], |row| {
                let mut object = Map::new();
                for (index, name) in names.iter().enumerate() {
                    object.insert(name.clone(), sql_value(row.get_ref(index)?));
                }
                Ok(Value::Object(object))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        rows.sort_by_key(|row| row.to_string());
        out.insert(format!("sql:{table}"), rows);
    }
    if let Some(authority) = &metadata.er {
        for (entity, id, state, fields) in er::snapshot_facts(authority) {
            out.entry(format!("er:{entity}"))
                .or_default()
                .push(json!({"id": id, "state": state, "fields": fields}));
        }
        for rows in out.values_mut() {
            rows.sort_by_key(|row| row.to_string());
        }
    }
    out
}

/// What a restarted owner observes: a fresh passive open of the authority.
fn views(path: &Path) -> View {
    view_of(&Metadata::inspect(path).unwrap())
}

fn is_uuid(bytes: &[u8]) -> bool {
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

/// Generated identities and digests over them differ between two roots by
/// construction; the relation is about everything else.
fn scrub(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if index + 36 <= bytes.len() && is_uuid(&bytes[index..index + 36]) {
            out.push_str("<uuid>");
            index += 36;
            continue;
        }
        let run = bytes[index..]
            .iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        if run >= 32 {
            out.push_str("<digest>");
            index += run;
            continue;
        }
        let step = if run > 0 { run } else { 1 };
        out.push_str(&text[index..index + step]);
        index += step;
    }
    out
}

fn scrub_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(scrub(text)),
        Value::Array(items) => Value::Array(items.iter().map(scrub_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (scrub(k), scrub_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn normalized(view: &View) -> View {
    view.iter()
        .map(|(table, rows)| {
            let mut rows = rows.iter().map(scrub_value).collect::<Vec<_>>();
            rows.sort_by_key(|row| row.to_string());
            (table.clone(), rows)
        })
        .collect()
}

/// Rows present in one view and not the other, per table.
fn diff(before: &View, after: &View) -> Vec<(String, &'static str, Value)> {
    let mut out = Vec::new();
    for table in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let empty = Vec::new();
        let a = before.get(table).unwrap_or(&empty);
        let b = after.get(table).unwrap_or(&empty);
        for row in a.iter().filter(|row| !b.contains(row)) {
            out.push((table.clone(), "before", row.clone()));
        }
        for row in b.iter().filter(|row| !a.contains(row)) {
            out.push((table.clone(), "after", row.clone()));
        }
    }
    out
}

fn assert_unchanged(relation: &str, before: &View, after: &View, allowed: &[&str]) {
    let changed = diff(before, after)
        .into_iter()
        .filter(|(table, _, _)| !allowed.iter().any(|name| table.contains(name)))
        .collect::<Vec<_>>();
    assert!(
        changed.is_empty(),
        "{relation} violated: views changed outside {allowed:?}:\n{}",
        changed
            .iter()
            .map(|(t, side, row)| format!("  {t} [{side}] {row}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------- world

#[derive(Clone)]
struct Clock(Arc<Mutex<i64>>);
impl mutations::Clock for Clock {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        let lower = *self.0.lock().unwrap();
        Ok(mutations::ClockInterval {
            lower_unix_ms: lower,
            upper_unix_ms: lower + 2000,
        })
    }
}

struct World {
    _root: tempfile::TempDir,
    path: PathBuf,
    registry: registry::Registry,
    mutations: mutations::Store<Clock>,
    audit: audit::Store,
    clock: Clock,
    connections: Vec<(String, String)>,
}

fn binding() -> registry::Binding {
    registry::fixture_binding(INSTANCE)
}

fn baseline(subject: &str, now: u64) -> registry::ValidatedBaseline {
    registry::ValidatedBaseline {
        identity: registry::ExternalIdentity {
            kind: "fixture-account".into(),
            subject: subject.into(),
        },
        granted_scopes: Some(BTreeSet::from(["read".into()])),
        credential_expires_at_ms: Some(now + 3_600_000),
        collected_at_ms: now,
        valid_until_ms: now + 60_000,
    }
}

fn root() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("state");
    fs::directory(&path, true, true).unwrap();
    drop(Metadata::initialize(&path).unwrap());
    (root, path)
}

impl World {
    fn new() -> Self {
        let (root, path) = root();
        let registry = registry::Registry::new(&path);
        let clock = Clock(Arc::new(Mutex::new(NOW as i64)));
        let mutations =
            mutations::Store::new(&path, clock.clone(), mutations::Limits::default()).unwrap();
        let audit = audit::Store::new(&path, 100_000).unwrap();
        Self {
            _root: root,
            path,
            registry,
            mutations,
            audit,
            clock,
            connections: Vec::new(),
        }
    }

    /// Allocate, store and publish one connection through the host registry.
    fn connect(&mut self, subject: &str, now: u64) -> (String, String) {
        let acquisition = self.registry.begin(&binding(), now).unwrap();
        let claim = self.registry.consume(acquisition, now).unwrap();
        let prepared = self
            .registry
            .prepare(&claim, baseline(subject, now), 12, now)
            .unwrap();
        let receipt = custody::WrittenVersion::fixture(prepared.version());
        let stored = self.registry.acknowledge(prepared, receipt, now).unwrap();
        let reference = self.registry.publish(stored, now).unwrap();
        let revision = self.describe(&reference, now).revision;
        self.connections.push((reference.clone(), revision.clone()));
        (reference, revision)
    }

    fn describe(&self, reference: &str, now: u64) -> registry::ObservedConnection {
        self.registry
            .describe(INSTANCE, ADAPTER, CONFIG, reference, now, true)
            .unwrap()
    }

    fn candidate(
        &self,
        connection: usize,
        caller: &str,
        key: Option<&str>,
    ) -> mutations::Candidate {
        let (reference, revision) = &self.connections[connection];
        mutations::Candidate {
            namespace: mutations::Namespace {
                receiver_instance: INSTANCE.into(),
                tenant: None,
                realm: None,
                caller: caller.into(),
                executor: None,
                origin: mutations::Origin::Direct,
            },
            fingerprint: mutations::Fingerprint {
                operation: mutations::OperationRef {
                    instance: INSTANCE.into(),
                    adapter: ADAPTER.into(),
                    operation: "item.write".into(),
                },
                connection_ref: reference.clone(),
                connection_revision: revision.clone(),
                contract_ref: "operations/v1alpha1".into(),
                profile: "mutation".into(),
                descriptor_revision: "descriptor".into(),
                configuration_revision: CONFIG.into(),
                canonicalization_version: "adapter-v1-canonical-json".into(),
                input_digest: "a".repeat(64),
                route: None,
            },
            caller_key: key.map(str::to_owned),
            request_id: format!("request-{caller}"),
            approval: mutations::Approval::NotRequired,
        }
    }

    fn prepare(&self, candidate: &mutations::Candidate) -> mutations::Prepared {
        match self.mutations.prepare(candidate).unwrap() {
            mutations::Preparation::Prepared(prepared) => prepared,
            mutations::Preparation::Existing(_) => panic!("unexpected existing attempt"),
        }
    }

    fn applied(&self, candidate: &mutations::Candidate, result: Value) -> mutations::AttemptRef {
        let prepared = self.prepare(candidate);
        let gate = self.mutations.open_dispatch(prepared).unwrap();
        let reference = gate.reference();
        self.mutations
            .settle(reference, &mutations::Outcome::Applied(result))
            .unwrap();
        reference
    }

    fn anchor(&self, request: &str) -> audit::Reference {
        let anchor = audit::Anchor {
            instance_id: INSTANCE.into(),
            kind: audit::Kind::AdmittedExecution,
            activity: Some(audit::Activity::Invoke),
            hop: audit::Hop::Execution,
            stage: audit::Stage::Admission,
            request_id: Some(request.into()),
            principal_ref: Some("caller".into()),
            operation_id: Some("item.write".into()),
            connection_ref: Some(self.connections[0].0.clone()),
            descriptor_revision: Some("descriptor".into()),
            recorded_at_ms: NOW as i64,
            attempt_id: None,
        };
        match self.audit.anchor(&anchor).unwrap() {
            audit::Acknowledgement::Execution(admission) => {
                self.audit.confirm(admission, &anchor).unwrap()
            }
            audit::Acknowledgement::Refusal(reference) => reference,
        }
    }

    fn keys(&self) -> approval_keys::Store {
        approval_keys::Store::new(
            &self.path,
            Some(&self.path.parent().unwrap().join("missing-bus")),
            INSTANCE,
            ADAPTER,
            CONFIG,
        )
        .unwrap()
    }

    fn policy(&self) -> approval_policy::Store {
        approval_policy::Store::new(&self.path, INSTANCE, ADAPTER).unwrap()
    }
}

fn selection() -> approval_policy::Selection {
    approval_policy::Selection {
        configuration_revision: CONFIG.into(),
        descriptor_revision: "desc-1".into(),
        executable_selection: "a".repeat(64),
        clock_configuration_sha256: "b".repeat(64),
    }
}

fn final_observation(id: u128) -> audit::FinalObservation {
    audit::FinalObservation {
        observation_id: uuid::Uuid::from_u128(id),
        outcome: audit::Outcome::Success,
        code: None,
        recorded_at_ms: NOW as i64 + 1,
    }
}

fn approval_subject(world: &World) -> approvals::Subject {
    let (connection, revision) = &world.connections[0];
    approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: INSTANCE.into(),
            operation: "item.write".into(),
            connection: connection.clone(),
            connection_revision: revision.clone(),
            contract: "operations/v1alpha1".into(),
            profile: "mutation".into(),
            descriptor_revision: "descriptor".into(),
            configuration_revision: CONFIG.into(),
        },
        authority: approvals::Authority {
            scope: approvals::Scope {
                tenant: None,
                realm: None,
                caller: "caller".into(),
                executor: None,
            },
            current_authority: None,
            executor: None,
        },
        origin: approvals::Origin {
            kind: approvals::OriginKind::Direct,
            authority_ref: INSTANCE.into(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: "a".repeat(64),
        approval_mode: "required".into(),
    }
}

/// A store with at least one subject of every entity the host records: the
/// 12 ESS domains with a compatibility table (every lowered domain except
/// `artifact_provenance`, which has none).
fn populated() -> World {
    let mut world = World::new();
    world.connect("one", NOW);
    world.connect("two", NOW);
    // ReadUse (credential_evidence), cursor (cli), clock floors (clock).
    let (first, _) = world.connections[0].clone();
    world
        .registry
        .capture_read(&binding(), &first, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let page = world
        .registry
        .list(
            INSTANCE,
            ADAPTER,
            CONFIG,
            registry::PageOptions {
                limit: 1,
                cursor: None,
            },
            NOW,
            true,
        )
        .unwrap();
    assert!(page.next_cursor.is_some());
    // LocalRuntimeRecord (cli).
    runtime::state::State::new(&world.path)
        .suppress(INSTANCE, true)
        .unwrap();
    // AttemptRecord (mutations) and KeyReservation (idempotency).
    world.applied(&world.candidate(0, "caller", Some("key")), json!({"ok": 1}));
    let aborted = world.prepare(&world.candidate(0, "unkeyed", None));
    world
        .mutations
        .abort(aborted.reference(), json!({"cause": "fixture"}))
        .unwrap();
    // AuditRecord (execution_audit), one anchored and one final.
    world.anchor("anchored");
    let finished = world.anchor("finished");
    world
        .audit
        .append(&finished, &final_observation(7))
        .unwrap();
    // ApprovalIssuer and ApprovalSigningKey (approval_issuers): with no
    // Secret Service the owner stages the issuer and a candidate key.
    assert_eq!(
        world.keys().init(None).unwrap_err(),
        approval_keys::Failure::CustodyUnavailable
    );
    // LocalApprovalPolicy (local_approval_policy).
    world
        .policy()
        .set_admitted(&selection(), vec!["item.write".into()], None)
        .unwrap();
    // ApprovalRedemption (delegation). Spend needs a signed proof; the
    // retained row is written through the owner's SQL port and persisted.
    let mut metadata = Metadata::update_approvals(&world.path).unwrap();
    let subject = String::from_utf8(approval_subject(&world).canonical_bytes().unwrap()).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO approval_redemptions VALUES (?1,'issuer',?2,?3,?4,?5,?6)",
            rusqlite::params![
                uuid::Uuid::from_u128(11).to_string(),
                "r".repeat(64),
                INSTANCE,
                aborted.reference().attempt_id.to_string(),
                subject,
                NOW as i64
            ],
        )
        .unwrap();
    metadata.persist().unwrap();
    drop(metadata);
    world
}

fn entities(view: &View) -> BTreeSet<String> {
    view.iter()
        .filter(|(table, rows)| table.starts_with("er:") && !rows.is_empty())
        .map(|(table, _)| table.trim_start_matches("er:").to_owned())
        .collect()
}

// ---------------------------------------------------------------- relations

/// MR1 — restart yields identical views (all 12 recorded domains).
/// docs/local-er-metadata.md:36-37 "Reopening always rebuilds the projection
/// from recorded subjects"; :176-177 "A restart rebuilds every already
/// installed compatibility table at that projection level."
#[test]
fn mr1_restart_and_every_open_path_yield_identical_views() {
    let world = populated();
    // A later page after the five-minute cursor deadline: the first cursor
    // leaves SQL through its authored expiry, a new one is recorded.
    let page = world
        .registry
        .list(
            INSTANCE,
            ADAPTER,
            CONFIG,
            registry::PageOptions {
                limit: 1,
                cursor: None,
            },
            NOW + 400_000,
            true,
        )
        .unwrap();
    assert!(page.next_cursor.is_some());
    let first = views(&world.path);
    assert!(
        first["er:connectors.cli.ConnectionListCursor"]
            .iter()
            .any(|row| row["state"] == "Expired"),
        "fixture must hold an expired cursor subject"
    );
    assert_eq!(
        entities(&first).len(),
        17,
        "fixture must record every table-backed entity: {:?}",
        entities(&first)
    );
    // Owner open paths, each dropped without a change.
    drop(Metadata::initialize(&world.path).unwrap());
    assert_unchanged("MR1 (initialize)", &first, &views(&world.path), &[]);
    drop(Metadata::update(&world.path, true).unwrap());
    assert_unchanged("MR1 (update)", &first, &views(&world.path), &[]);
    drop(Metadata::update_approval_policy(&world.path).unwrap());
    assert_unchanged("MR1 (update policy)", &first, &views(&world.path), &[]);
    // The writer's own handle after a real persisted write equals a reopen.
    let mut writer = Metadata::update(&world.path, false).unwrap();
    writer
        .connection
        .execute(
            "INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES ('second',1)",
            [],
        )
        .unwrap();
    writer.persist().unwrap();
    let live = view_of(&writer);
    drop(writer);
    assert_unchanged("MR1 (writer vs reopen)", &live, &views(&world.path), &[]);
}

/// MR2 — a fresh installation and a migrated one holding the same rows
/// record the same subjects (all 12 recorded domains).
/// docs/local-er-metadata.md:167-168 "fresh and migrated installations enter
/// the identical authority path."
#[test]
fn mr2_fresh_and_migrated_installations_record_the_same_subjects() {
    let world = populated();
    let fresh = views(&world.path);
    let (_legacy_root, legacy) = {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("state");
        (root, path)
    };
    super::legacy_fixture(&legacy, 8);
    {
        let target = rusqlite::Connection::open(legacy.join(super::NAME)).unwrap();
        target.pragma_update(None, "foreign_keys", false).unwrap();
        let source = Metadata::inspect(&world.path).unwrap();
        for table in [
            "registry_clock",
            "registry_instances",
            "registry_profiles",
            "registry_connections",
            "registry_acquisitions",
            "registry_generations",
            "registry_materials",
            "registry_uses",
            "registry_cursors",
            "local_runtime_instances",
            "mutation_clock",
            "mutation_attempts",
            "mutation_keys",
            "execution_audits",
            "approval_redemptions",
            "local_approval_issuers",
            "local_approval_keys",
            "local_approval_policies",
        ] {
            let mut statement = source
                .connection
                .prepare(&format!("SELECT * FROM {table}"))
                .unwrap();
            let width = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..width)
                        .map(|i| row.get::<_, rusqlite::types::Value>(i))
                        .collect::<std::result::Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            let marks = vec!["?"; width].join(",");
            for row in rows {
                target
                    .execute(
                        &format!("INSERT OR REPLACE INTO {table} VALUES ({marks})"),
                        rusqlite::params_from_iter(row),
                    )
                    .unwrap();
            }
        }
    }
    drop(Metadata::update(&legacy, true).unwrap());
    let migrated = views(&legacy);
    let strip = |view: &View| -> View {
        normalized(view)
            .into_iter()
            // Provider bookkeeping and the authority envelope differ by design.
            .filter(|(table, _)| {
                !matches!(
                    table.as_str(),
                    "sql:local_authority" | "sql:schema_migrations" | "sql:connectors_er_authority"
                )
            })
            .collect()
    };
    assert_unchanged("MR2", &strip(&fresh), &strip(&migrated), &[]);
}

/// MR3 — replaying a keyed mutation with the same key yields the same
/// result and one attempt (mutations, idempotency).
/// docs/local-mutation-ledger.md:59-60 "An exact key hit preserves the
/// original request and attempt."; contracts/operations/v1alpha1/
/// semantics.md:185 "Replays do not slide the deadline."
#[test]
fn mr3_replayed_keyed_mutation_has_one_attempt_and_the_original_result() {
    let mut world = World::new();
    world.connect("one", NOW);
    let original = world.candidate(0, "caller", Some("key"));
    let prepared = world.prepare(&original);
    let pending = views(&world.path);
    let mut replay = original.clone();
    replay.request_id = "replayed-request".into();
    match world.mutations.prepare(&replay).unwrap() {
        mutations::Preparation::Existing(observed) => {
            assert_eq!(observed.request_id, original.request_id);
            assert_eq!(observed.reference, prepared.reference());
        }
        mutations::Preparation::Prepared(_) => {
            panic!("MR3 violated: replay prepared a second attempt")
        }
    }
    assert_unchanged("MR3 (pending replay)", &pending, &views(&world.path), &[]);

    let reference = prepared.reference();
    let gate = world.mutations.open_dispatch(prepared).unwrap();
    assert_eq!(gate.reference(), reference);
    world
        .mutations
        .settle(reference, &mutations::Outcome::Applied(json!({"ok": 1})))
        .unwrap();
    let settled = views(&world.path);
    // A later replay, one hour on: same attempt, same result, same expiry.
    *world.clock.0.lock().unwrap() += 3_600_000;
    match world.mutations.prepare(&replay).unwrap() {
        mutations::Preparation::Existing(observed) => {
            assert_eq!(observed.reference, reference);
            assert_eq!(observed.result, Some(json!({"ok": 1})));
        }
        mutations::Preparation::Prepared(_) => {
            panic!("MR3 violated: replay prepared a second attempt")
        }
    }
    world.mutations.lookup(&replay).unwrap().unwrap();
    assert_unchanged(
        "MR3 (settled replay)",
        &settled,
        &views(&world.path),
        // The mutation clock floor is the one fact a later sample may move.
        &["mutation_clock", "LocalClockFloor"],
    );
}

/// MR4 — the first terminal observation is immutable (mutations,
/// idempotency): a different settlement, a repeated one and recovery all
/// leave the views of the first settlement.
/// docs/local-mutation-ledger.md:64-65 "Terminal settlement atomically fixes
/// the attempt outcome ... The first terminal observation is immutable."
#[test]
fn mr4_a_settled_attempt_ignores_every_later_settlement() {
    let mut world = World::new();
    world.connect("one", NOW);
    let reference = world.applied(&world.candidate(0, "caller", Some("key")), json!({"ok": 1}));
    let settled = views(&world.path);
    assert_eq!(
        world
            .mutations
            .settle(reference, &mutations::Outcome::Refused(json!({"no": 1})))
            .err(),
        Some(mutations::Failure::Conflict)
    );
    assert_eq!(
        world
            .mutations
            .settle(reference, &mutations::Outcome::Unknown(json!({"ok": 1})))
            .err(),
        Some(mutations::Failure::Conflict)
    );
    world
        .mutations
        .settle(reference, &mutations::Outcome::Applied(json!({"ok": 1})))
        .unwrap();
    world.mutations.recover(reference).unwrap();
    assert_unchanged("MR4", &settled, &views(&world.path), &[]);
}

/// MR5 — revoking a connection changes nothing except its own records, the
/// instance epoch that invalidates continuations, and the registry clock
/// (auth_bindings, credentials, credential_evidence, declarations, cli, clock).
/// docs/local-connection-registry.md:78-80 "Revoke ... changes the public
/// semantic revision, clears active authority, closes pending acquisitions
/// and fences retirement."; :107-108 "Publication, revoke and known
/// invalidity invalidate continuations."
#[test]
fn mr5_revoke_changes_only_the_revoked_connection() {
    let mut world = World::new();
    let (first, revision) = world.connect("one", NOW);
    let (second, _) = world.connect("two", NOW);
    for reference in [&first, &second] {
        world
            .registry
            .capture_read(&binding(), reference, &BTreeSet::new(), NOW, NOW + 1000)
            .unwrap();
    }
    // A pending repair on each connection.
    for (reference, revision) in world.connections.clone() {
        world
            .registry
            .begin_repair(&binding(), &reference, &revision, NOW)
            .unwrap();
    }
    let before = views(&world.path);
    world
        .registry
        .revoke(INSTANCE, ADAPTER, &first, &revision, NOW + 1)
        .unwrap();
    let after = views(&world.path);
    // Every record of the revoked connection names it; nothing else may move.
    let foreign = diff(&before, &after)
        .into_iter()
        .filter(|(table, _, row)| {
            let text = row.to_string();
            !(text.contains(&first)
                || [
                    "registry_instances",
                    "ServiceConfiguration",
                    "registry_clock",
                    "LocalClockFloor",
                ]
                .iter()
                .any(|name| table.contains(name)))
        })
        .collect::<Vec<_>>();
    assert!(
        foreign.is_empty(),
        "MR5 violated: revoke changed records of other connections:\n{foreign:#?}"
    );
    // Everything naming the other connection is byte-identical.
    for (table, rows) in &before {
        for row in rows.iter().filter(|r| r.to_string().contains(&second)) {
            assert!(
                after.get(table).is_some_and(|rows| rows.contains(row)),
                "MR5 violated: {table} row of the untouched connection changed: {row}"
            );
        }
    }
}

/// MR6 — repeating a revoke returns the same terminal result and changes
/// nothing but the registry clock floor (auth_bindings).
/// docs/local-connection-registry.md:80-81 "Repeated revocation returns the
/// same terminal result."
#[test]
fn mr6_repeated_revoke_is_a_pure_replay() {
    let mut world = World::new();
    let (first, revision) = world.connect("one", NOW);
    let revoked = world
        .registry
        .revoke(INSTANCE, ADAPTER, &first, &revision, NOW + 1)
        .unwrap();
    let once = views(&world.path);
    for expected in [&revision, &revoked] {
        assert_eq!(
            world
                .registry
                .revoke(INSTANCE, ADAPTER, &first, expected, NOW + 2)
                .unwrap(),
            revoked
        );
    }
    assert_unchanged(
        "MR6",
        &once,
        &views(&world.path),
        &["registry_clock", "LocalClockFloor"],
    );
}

/// MR7 — a refused registry action changes nothing but the clock floor it
/// sampled (auth_bindings, credentials, credential_evidence, clock).
/// docs/local-connection-registry.md:47-48 "It records time even when a
/// semantic action refuses, using a savepoint to roll back that action."
#[test]
fn mr7_refused_registry_actions_change_only_the_clock() {
    let mut world = World::new();
    let (first, revision) = world.connect("one", NOW);
    // A retained use whose deadline passes before the refused capture below;
    // that capture sweeps expired uses before it refuses.
    world
        .registry
        .capture_read(&binding(), &first, &BTreeSet::new(), NOW, NOW + 1000)
        .unwrap();
    let before = views(&world.path);
    assert_eq!(
        world
            .registry
            .revoke(INSTANCE, ADAPTER, &first, "stale-revision", NOW + 1)
            .err(),
        Some(registry::Failure::Conflict)
    );
    assert!(
        world
            .registry
            .begin_repair(&binding(), &first, "stale-revision", NOW + 2)
            .is_err()
    );
    assert!(
        world
            .registry
            .capture_read(
                &binding(),
                "missing-connection",
                &BTreeSet::new(),
                NOW + 2000,
                NOW + 3000
            )
            .is_err()
    );
    assert_unchanged(
        "MR7",
        &before,
        &views(&world.path),
        &["registry_clock", "LocalClockFloor"],
    );
    assert_eq!(world.describe(&first, NOW + 2001).revision, revision);
}

/// MR8 — a refused approval-key or approval-policy command leaves every view
/// unchanged (approval_issuers, local_approval_policy).
/// docs/local-er-metadata.md:203-204 "A rejected or queued call has no
/// durable effect."; docs/local-approval-keys.md:16 "The first
/// initialization needs no expected revision."
#[test]
fn mr8_refused_key_and_policy_commands_leave_every_view_unchanged() {
    let mut world = World::new();
    world.connect("one", NOW);
    let keys = world.keys();
    assert_eq!(
        keys.init(None).unwrap_err(),
        approval_keys::Failure::CustodyUnavailable
    );
    let policy = world.policy();
    let first = policy
        .set_admitted(&selection(), vec!["item.write".into()], None)
        .unwrap();
    let staged = keys.status().unwrap().unwrap();
    let before = views(&world.path);
    // Key management with a stale or absent expected revision.
    let init = keys.init(None).err();
    let rotate = keys
        .rotate(uuid::Uuid::from_u128(1), staged.keys[0].key_id)
        .err();
    let revoke = keys
        .revoke(uuid::Uuid::from_u128(1), staged.keys[0].key_id)
        .err();
    // Policy publication that does not name the current revision.
    let published = [None, Some(first.revision + 1), Some(first.revision + 7)].map(|expected| {
        policy
            .set_admitted(&selection(), vec!["other.write".into()], expected)
            .err()
    });
    assert_unchanged("MR8", &before, &views(&world.path), &[]);
    assert_eq!(init, Some(approval_keys::Failure::Conflict));
    assert!(rotate.is_some() && revoke.is_some());
    assert_eq!(published, [Some(approval_policy::Failure::Conflict); 3]);
}

/// MR9 — keyed candidates in separate namespaces commute: the order in which
/// two callers prepare and settle, interleaved with a runtime-record write
/// and a registry observation, leaves the same state
/// (mutations, idempotency, cli, clock).
/// contracts/operations/v1alpha1/semantics.md:174 "Separate namespace;
/// evaluate as a new independently admitted candidate. Never return or
/// suppress work using the other namespace's record"; docs/local-er-metadata.md:
/// 191-193 a runtime-record write "can land between an observation's unlocked
/// replay and its relock".
#[test]
fn mr9_independent_namespaces_and_runtime_records_commute() {
    fn run(order: &[usize]) -> View {
        let mut world = World::new();
        world.connect("one", NOW);
        for step in order {
            match step {
                0 | 1 => {
                    let (caller, who) = if *step == 0 {
                        ("alice", "a")
                    } else {
                        ("bob", "b")
                    };
                    // A collision with the other namespace is the relation's
                    // failure, reported by the comparison below.
                    if let mutations::Preparation::Prepared(prepared) = world
                        .mutations
                        .prepare(&world.candidate(0, caller, Some("key")))
                        .unwrap()
                    {
                        let gate = world.mutations.open_dispatch(prepared).unwrap();
                        world
                            .mutations
                            .settle(
                                gate.reference(),
                                &mutations::Outcome::Applied(json!({"who": who})),
                            )
                            .unwrap();
                    }
                }
                2 => {
                    runtime::state::State::new(&world.path)
                        .suppress(INSTANCE, true)
                        .unwrap();
                }
                3 => {
                    let (reference, _) = world.connections[0].clone();
                    world.describe(&reference, NOW);
                }
                _ => unreachable!(),
            }
        }
        normalized(&views(&world.path))
    }
    let forward = run(&[0, 1, 2, 3]);
    let backward = run(&[3, 2, 1, 0]);
    assert_unchanged("MR9", &forward, &backward, &[]);
    assert_eq!(
        forward["sql:mutation_attempts"].len(),
        2,
        "MR9 violated: namespaces collided"
    );
    assert_eq!(
        forward["sql:mutation_keys"].len(),
        2,
        "MR9 violated: namespaces collided"
    );
}

/// MR10 — repeating the exact final audit observation is acknowledged
/// without change, and a different one refuses without change
/// (execution_audit).
/// docs/local-execution-audit.md:66-68 "Repeating the exact UUID and semantic
/// fields returns the retained acknowledgement; changing any field or the
/// UUID conflicts. The first final fact is immutable."
#[test]
fn mr10_repeated_final_audit_observation_is_a_pure_replay() {
    let mut world = World::new();
    world.connect("one", NOW);
    let reference = world.anchor("request");
    world
        .audit
        .append(&reference, &final_observation(7))
        .unwrap();
    let once = views(&world.path);
    world
        .audit
        .append(&reference, &final_observation(7))
        .unwrap();
    assert_eq!(
        world.audit.append(&reference, &final_observation(8)).err(),
        Some(audit::Failure::Conflict)
    );
    let mut changed = final_observation(7);
    changed.outcome = audit::Outcome::Refused;
    assert_eq!(
        world.audit.append(&reference, &changed).err(),
        Some(audit::Failure::Conflict)
    );
    assert_unchanged("MR10", &once, &views(&world.path), &[]);
}

/// MR11 — a passive observation in the same millisecond as the last write
/// re-records the same clock floor and leaves every view unchanged; a lower
/// sample is refused and also leaves every view unchanged (clock).
/// docs/local-er-metadata.md:189-190 "Even an unchanged millisecond gets a
/// clock revision guard"; docs/local-connection-registry.md:44 "A durable
/// clock floor rejects wall-clock regression."
#[test]
fn mr11_same_millisecond_and_regressed_observations_leave_views_unchanged() {
    let mut world = World::new();
    let (first, _) = world.connect("one", NOW + 10);
    let before = views(&world.path);
    world.describe(&first, NOW + 10);
    assert_unchanged("MR11 (same millisecond)", &before, &views(&world.path), &[]);
    assert!(
        world
            .registry
            .describe(INSTANCE, ADAPTER, CONFIG, &first, NOW, true)
            .is_err()
    );
    assert_unchanged("MR11 (regressed)", &before, &views(&world.path), &[]);
}

/// MR12 — at the Entity Runtime boundary, a projection change the recorded
/// guard refuses leaves every recorded view unchanged (clock,
/// local_approval_policy; the ess/15 `when_subject` guards).
/// docs/local-er-metadata.md:35-37 "A projection or append failure refuses
/// the caller. Reopening always rebuilds the projection from recorded
/// subjects, so an unacknowledged projection never authorizes ..."
#[test]
fn mr12_guard_refused_projection_changes_leave_recorded_views_unchanged() {
    let world = populated();
    let before = views(&world.path);
    // A regressed mutation clock floor: ClockFloorRegressed.
    let mut writer = Metadata::update(&world.path, false).unwrap();
    writer
        .connection
        .execute(
            "UPDATE mutation_clock SET last_lower_ms=last_lower_ms-1",
            [],
        )
        .unwrap();
    assert!(
        writer.persist().is_err(),
        "MR12 violated: a lower clock floor was recorded"
    );
    drop(writer);
    assert_unchanged("MR12 (clock)", &before, &views(&world.path), &[]);
    // A policy revision that does not advance: RevisionNotAdvanced. The SQL
    // successor trigger is dropped from this private projection so the
    // proposal reaches the recorded guard.
    let mut writer = Metadata::update_approval_policy(&world.path).unwrap();
    writer
        .connection
        .execute_batch(
            "DROP TRIGGER local_approval_policy_revision;
             UPDATE local_approval_policies SET operations='[\"other.write\"]';",
        )
        .unwrap();
    assert!(
        writer.persist().is_err(),
        "MR12 violated: a policy revision that does not advance was recorded"
    );
    drop(writer);
    assert_unchanged("MR12 (policy)", &before, &views(&world.path), &[]);
}

/// MR13 — a refused mutation command leaves every view unchanged
/// (mutations, idempotency, clock).
/// docs/local-mutation-ledger.md:82-83 "Clock failure during known settlement
/// leaves the earlier durable state intact"; :99 "Capacity refuses new
/// attempts/reservations instead of evicting unresolved ones."; :40-41 "The
/// local connection revision, configuration and publication fence must still
/// match at the gate."
#[test]
fn mr13_refused_mutation_commands_leave_every_view_unchanged() {
    let mut world = World::new();
    world.connect("one", NOW);
    // The first known settlement records the mutation clock floor.
    *world.clock.0.lock().unwrap() += 10_000;
    world.applied(&world.candidate(0, "first", Some("key")), json!({"ok": 0}));
    let prepared = world.prepare(&world.candidate(0, "caller", Some("key")));
    let gate = world.mutations.open_dispatch(prepared).unwrap();
    let before = views(&world.path);
    // Capacity: a third attempt in the instance is refused.
    let bounded = mutations::Store::new(
        &world.path,
        world.clock.clone(),
        mutations::Limits {
            attempts_per_instance: 2,
            ..mutations::Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        bounded
            .prepare(&world.candidate(0, "other", Some("key")))
            .err(),
        Some(mutations::Failure::Capacity)
    );
    // Stale connection revision at preparation.
    let mut stale = world.candidate(0, "stale", None);
    stale.fingerprint.connection_revision = "stale-revision".into();
    assert!(world.mutations.prepare(&stale).is_err());
    // A clock below the recorded floor during known settlement.
    *world.clock.0.lock().unwrap() -= 10_000;
    assert_eq!(
        world
            .mutations
            .settle(
                gate.reference(),
                &mutations::Outcome::Applied(json!({"ok": 1}))
            )
            .err(),
        Some(mutations::Failure::ClockUnavailable)
    );
    assert_unchanged("MR13", &before, &views(&world.path), &[]);
}

// Metadata timeout safety binds the CLI's conservative storage acknowledgement
// to the mutation, idempotency and clock domains. Holding SQLite's writer lock
// keeps the real ER worker inside provider dispatch, without changing its code.
const BATCH_DEADLINE: std::time::Duration = std::time::Duration::from_millis(250);

fn hold_worker(path: &Path) -> rusqlite::Connection {
    let blocker = rusqlite::Connection::open(path.join(super::NAME)).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    blocker
}

fn lifecycle_owned(path: &Path) -> bool {
    use std::os::fd::AsRawFd;
    let lock = std::fs::File::open(path.join(super::LOCK)).unwrap();
    // SAFETY: this fresh descriptor is owned until the end of this function.
    let result = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        false
    } else {
        assert_eq!(
            std::io::Error::last_os_error().kind(),
            std::io::ErrorKind::WouldBlock
        );
        true
    }
}

fn wait_for_runtime_row(path: &Path, id: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Ok(metadata) = Metadata::inspect(path) {
            let present: bool = metadata
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_runtime_instances WHERE instance_id=?1)",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            if present {
                return;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "worker never committed"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn metadata_timeout_safety_batch_timeout_retains_ownership() {
    const CHILD_PATH: &str = "CONNECTORS_TIMEOUT_SAFETY_CHILD_PATH";
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        // The original process performs no metadata calls while this process
        // waits: recovery therefore requires autonomous retirement.
        wait_for_runtime_row(Path::new(&path), "timed-out");
        return;
    }
    let (_root, path) = root();
    let mut metadata = Metadata::update(&path, true).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances(instance_id,suppressed) VALUES ('timed-out',1)",
            [],
        )
        .unwrap();
    let blocker = hold_worker(&path);
    let result = er::with_batch_wait(BATCH_DEADLINE, || metadata.persist_runtime_state());
    assert_eq!(
        result,
        Err(super::Failure::OutcomeUnknown),
        "the real worker must have dispatched"
    );
    let started = std::time::Instant::now();
    drop(metadata);
    let elapsed = started.elapsed();
    let retained = lifecycle_owned(&path);
    blocker.execute_batch("ROLLBACK").unwrap();
    let recovered = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "local::metadata::metamorphic_tests::metadata_timeout_safety_batch_timeout_retains_ownership", "--nocapture"])
        .env(CHILD_PATH, &path)
        .output()
        .unwrap();
    assert!(
        recovered.status.success(),
        "cross-process recovery failed: {}",
        String::from_utf8_lossy(&recovered.stdout)
    );
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "Drop must return without waiting for a stuck worker"
    );
    assert!(
        retained,
        "a dispatched batch still able to commit released lifecycle ownership"
    );
}

#[test]
fn metadata_timeout_safety_queued_batch_cancelled_before_release() {
    let (_root, path) = root();
    let mut metadata = Metadata::update(&path, true).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances(instance_id,suppressed) VALUES ('dispatched',1)",
            [],
        )
        .unwrap();
    let blocker = hold_worker(&path);
    assert_eq!(
        er::with_batch_wait(BATCH_DEADLINE, || metadata.persist_runtime_state()),
        Err(super::Failure::OutcomeUnknown)
    );
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances(instance_id,suppressed) VALUES ('queued',1)",
            [],
        )
        .unwrap();
    assert_eq!(
        er::with_batch_wait(BATCH_DEADLINE, || metadata.persist_runtime_state()),
        Err(super::Failure::MetadataUnavailable)
    );
    drop(metadata);
    let retained = lifecycle_owned(&path);
    blocker.execute_batch("ROLLBACK").unwrap();
    wait_for_runtime_row(&path, "dispatched");
    let metadata = Metadata::inspect(&path).unwrap();
    let count: i64 = metadata
        .connection
        .query_row(
            "SELECT count(*) FROM local_runtime_instances WHERE instance_id='queued'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "a queued deadline must never become a later commit"
    );
    assert!(
        retained,
        "cancelling queued work did not finish the dispatched batch"
    );
}

#[test]
fn metadata_timeout_safety_next_invoke_after_unknown_outcome() {
    let mut world = World::new();
    world.connect("timeout", NOW);
    let candidate = world.candidate(0, "timeout-caller", Some("timeout-key"));
    let prepared = world.prepare(&candidate);
    let gate = world.mutations.open_dispatch(prepared).unwrap();
    let reference = gate.reference();
    // A counted provider effect is permitted only by the original gate. A
    // subsequent keyed invoke must observe the recorded attempt, never send.
    let mut provider_requests = 1;
    let held = Arc::new(Mutex::new(None));
    let worker_lock = held.clone();
    let path = world.path.clone();
    er::before_batch(move || *worker_lock.lock().unwrap() = Some(hold_worker(&path)));
    let result = er::with_batch_wait(BATCH_DEADLINE, || {
        world.mutations.settle(
            reference,
            &mutations::Outcome::Applied(json!({"effect": 1})),
        )
    });
    assert_eq!(result.err(), Some(mutations::Failure::OutcomeUnknown));
    let retained = lifecycle_owned(&world.path);
    held.lock()
        .unwrap()
        .take()
        .unwrap()
        .execute_batch("ROLLBACK")
        .unwrap();
    match world.mutations.prepare(&candidate).unwrap() {
        mutations::Preparation::Existing(observed) => {
            assert_eq!(observed.reference, reference);
            assert_eq!(observed.result, Some(json!({"effect": 1})));
        }
        mutations::Preparation::Prepared(_) => provider_requests += 1,
    }
    assert_eq!(
        provider_requests, 1,
        "recovery issued a duplicate provider effect"
    );
    assert!(
        retained,
        "unknown settlement released its ownership before observation was possible"
    );
}

#[test]
fn metadata_timeout_safety_legacy_import_retains_ownership() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("legacy");
    let authority = super::legacy_fixture(&path, 3);
    let held = Arc::new(Mutex::new(None));
    let worker_lock = held.clone();
    let blocked_path = path.clone();
    er::before_batch(move || {
        *worker_lock.lock().unwrap() = Some(hold_worker(&blocked_path));
    });
    let result = er::with_batch_wait(BATCH_DEADLINE, || Metadata::update(&path, true));
    assert!(matches!(result, Err(super::Failure::MetadataUnavailable)));
    let retained = lifecycle_owned(&path);
    held.lock()
        .unwrap()
        .take()
        .unwrap()
        .execute_batch("ROLLBACK")
        .unwrap();
    // Recovery must resume equal imported anchors without inventing a fresh
    // authority or treating the timed-out import as a successful cutover.
    let reopened = Metadata::update(&path, true).unwrap();
    assert_eq!(reopened.authority().unwrap(), authority);
    assert!(
        retained,
        "a timed-out legacy import released a still-running worker"
    );
}

#[test]
fn adversary_timeout_unwind_keeps_ownership_until_dispatched_work_finishes() {
    let (_root, path) = root();
    let mut metadata = Metadata::update(&path, true).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances(instance_id,suppressed) VALUES ('unwind-dispatched',1)",
            [],
        )
        .unwrap();
    let blocker = hold_worker(&path);
    let started = std::time::Instant::now();
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        assert_eq!(
            er::with_batch_wait(BATCH_DEADLINE, || metadata.persist_runtime_state()),
            Err(super::Failure::OutcomeUnknown)
        );
        panic!("caller unwinds after uncertain acknowledgement");
    }));
    let elapsed = started.elapsed();
    let retained = lifecycle_owned(&path);
    blocker.execute_batch("ROLLBACK").unwrap();
    wait_for_runtime_row(&path, "unwind-dispatched");
    assert_eq!(
        unwind.unwrap_err().downcast_ref::<&str>(),
        Some(&"caller unwinds after uncertain acknowledgement"),
        "the injected panic must follow an actual OutcomeUnknown"
    );
    assert!(elapsed < std::time::Duration::from_secs(2));
    assert!(retained, "unwinding released a still-dispatched writer");
}

#[test]
fn adversary_timeout_before_dispatch_releases_ownership_without_a_commit() {
    let (_root, path) = root();
    let mut metadata = Metadata::update(&path, true).unwrap();
    metadata
        .connection
        .execute(
            "INSERT INTO local_runtime_instances(instance_id,suppressed) VALUES ('never-dispatched',1)",
            [],
        )
        .unwrap();
    assert_eq!(
        er::with_batch_wait(std::time::Duration::ZERO, || metadata
            .persist_runtime_state()),
        Err(super::Failure::MetadataUnavailable)
    );
    drop(metadata);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while lifecycle_owned(&path) {
        assert!(
            std::time::Instant::now() < deadline,
            "idle retirement leaked the lifecycle lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let reopened = Metadata::inspect(&path).unwrap();
    let count: i64 = reopened
        .connection
        .query_row(
            "SELECT count(*) FROM local_runtime_instances WHERE instance_id='never-dispatched'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "deadline-before-acceptance became a durable effect"
    );
}
