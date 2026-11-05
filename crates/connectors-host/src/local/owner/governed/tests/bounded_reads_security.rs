//! Conformance of the owner's bounded-read port to its effects rule: an
//! operation that declares a semantic effect, or an executable effect other
//! than `network`, is never run as a bounded read, on any owner-side admission
//! of that port, and the refusal claims no attempt.
#![cfg(unix)]
use super::*;
use crate::local::owner::supervisor::{Pool, Task};
use connectors_core::operation_metadata::Metadata;
use std::{collections::BTreeMap, sync::Arc};

fn admitted() -> Value {
    json!({"effects":["network"],"semantic_effects":[],"risk":"low",
        "idempotency":{"kind":"none"},"approval":"not_required",
        "realization":"implemented",
        "limits":{"execution_ms":20000,"provider_ms":15000,"connect_ms":5000,
            "request_bytes":65536,"result_bytes":4194304}})
}

/// Every declaration of the closed vocabularies that a `resource` operation
/// can carry and that the read rule refuses.
fn refused() -> Vec<(&'static str, Value)> {
    let mut cases = Vec::new();
    for (name, field, effects) in [
        (
            "human_visible",
            "semantic_effects",
            json!(["human_visible"]),
        ),
        ("billable", "semantic_effects", json!(["billable"])),
        ("irreversible", "semantic_effects", json!(["irreversible"])),
        (
            "all_semantic",
            "semantic_effects",
            json!(["human_visible", "billable", "irreversible"]),
        ),
        ("process_only", "effects", json!(["process"])),
        (
            "local_system",
            "effects",
            json!(["network", "local_system"]),
        ),
        ("process_first", "effects", json!(["process", "network"])),
    ] {
        let mut value = admitted();
        value[field] = effects;
        cases.push((name, value));
    }
    let mut both = admitted();
    both["semantic_effects"] = json!(["billable"]);
    both["effects"] = json!(["network", "process"]);
    cases.push(("semantic_and_process", both));
    cases
}

fn prepare_v3(target: &mut Target) {
    let mut config = Config::load(&target.paths.config).unwrap();
    let old = config.adapters["fixture"].clone();
    let mut bootstrap = runtime::state::State::new(&target.paths.state)
        .cached(&old.instance_id, &old.selection())
        .unwrap()
        .unwrap();
    config.adapters.get_mut("fixture").unwrap().private_protocol =
        Some(runtime::PrivateProtocol::V3);
    let mut descriptor = bootstrap.descriptor().unwrap();
    descriptor.operations[0].input_schema = json!({"type":"object"});
    bootstrap.descriptor = serde_json::to_string(&descriptor).unwrap();
    target.schema = crate::local::owner::schema(&bootstrap, "item.read").unwrap();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    runtime::state::State::new(&target.paths.state)
        .remember(&config.adapters["fixture"].selection(), &bootstrap)
        .unwrap();
}

/// A projection that admits every request and declares `declaration` for
/// `item.read` (or no operation at all) under `revision`.
struct Projection {
    declaration: Option<Value>,
    revision: &'static str,
}
impl Projection {
    fn of(declaration: Value) -> Self {
        Self {
            declaration: Some(declaration),
            revision: "projection",
        }
    }
}
impl ReadPolicy for Projection {
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        Ok(())
    }
    fn metadata(
        &self,
        _: &Paths,
        _: &str,
        _: &config::Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> Result<ProjectionMetadata> {
        let descriptor = bootstrap.descriptor()?;
        let profile = &descriptor.operation("item.read").unwrap().profile;
        let mut operations = BTreeMap::new();
        if let Some(declaration) = &self.declaration {
            operations.insert(
                "item.read".to_owned(),
                Metadata::parse(&serde_json::to_vec(declaration).unwrap(), profile, 65536).unwrap(),
            );
        }
        Ok(ProjectionMetadata {
            revision: self.revision.into(),
            operations,
        })
    }
}

fn budget() -> runtime::ReadBudget {
    runtime::ReadBudget::start(20000, 15000, 65536, 4194304).unwrap()
}

fn code(result: Result<ReadPlan>) -> Option<Code> {
    result.err().map(|error| error.code)
}

fn resolve(target: &Target, policy: &Projection) -> Option<Code> {
    code(bounded::resolve(
        &target.paths,
        "fixture",
        &target.request(),
        Some((policy, "projection")),
        Some(budget()),
    ))
}

/// Every refused declaration of the vocabularies is answered `unsupported`
/// with no provider dispatch, and its audit is an early refusal that names no
/// operation, access or descriptor revision: nothing claims an attempt.
#[test]
fn refused_effects_record_only_an_early_refusal_and_never_dispatch() {
    let mut target = Target::new();
    prepare_v3(&mut target);
    assert_eq!(resolve(&target, &Projection::of(admitted())), None);
    for (name, declaration) in refused() {
        let policy = Projection::of(declaration);
        assert_eq!(resolve(&target, &policy), Some(Code::Unsupported), "{name}");
        let value = bounded::read(
            &target.paths,
            "owner-authority",
            "fixture",
            &target.request(),
            (&policy, "projection"),
            budget(),
            |_| panic!("{name}: a refused bounded read reached provider dispatch"),
        )
        .unwrap();
        assert_eq!(value["error"]["code"], "unsupported", "{name}");
        assert_eq!(value["audit_status"], "complete", "{name}");
        let record = target.record(&target.audits(), &value);
        assert_eq!(record.anchor.kind, audit::Kind::EarlyRefusal, "{name}");
        assert!(record.anchor.operation_id.is_none(), "{name}");
        assert!(record.anchor.access.is_none(), "{name}");
        assert!(record.anchor.descriptor_revision.is_none(), "{name}");
        assert_eq!(
            record.final_observation.unwrap().outcome,
            audit::Outcome::Refused,
            "{name}"
        );
    }
}

/// The revision and lookup guards keep their place ahead of the effects rule:
/// a stale projection is `stale_description` and an operation absent from the
/// projection is `not_found`, whatever effects the declaration carries.
#[test]
fn revision_and_lookup_still_precede_the_effects_refusal() {
    let mut target = Target::new();
    prepare_v3(&mut target);
    for (name, declaration) in refused() {
        let stale = Projection {
            declaration: Some(declaration),
            revision: "other-projection",
        };
        assert_eq!(
            resolve(&target, &stale),
            Some(Code::StaleDescription),
            "{name}"
        );
    }
    let absent = Projection {
        declaration: None,
        revision: "projection",
    };
    assert_eq!(resolve(&target, &absent), Some(Code::NotFound));
}

/// The serialized worker admits a bounded read again before it starts any
/// provider process. Driven directly, past the owner's audited coordinator,
/// a refused declaration is `unsupported` there, while the admitted one
/// passes that admission and fails later, at provider startup.
#[test]
fn worker_admission_refuses_effects_before_provider_startup() {
    let mut target = Target::new();
    prepare_v3(&mut target);
    let adapter = Config::load(&target.paths.config).unwrap().adapters["fixture"].clone();
    let run = |declaration: Value| {
        let paths = Arc::new(Paths {
            config: target.paths.config.clone(),
            state: target.paths.state.clone(),
        });
        let pool = Pool::shared_with_read_policy(
            paths,
            "owner-authority".into(),
            Arc::new(Projection::of(declaration)),
        );
        let request = target.request();
        let result = pool.run(
            "fixture",
            &adapter,
            Task::Invoke {
                connection: request.connection.into(),
                operation: request.operation.into(),
                schema: request.schema.into(),
                revision: request.revision.into(),
                document: request.input.as_bytes().to_vec(),
                governed: true,
                projection_revision: Some("projection".into()),
                budget: Some(budget()),
            },
            connectors_sdk::now_ms() + 5000,
        );
        pool.shutdown().unwrap();
        result.err().map(|error| error.code)
    };
    let control = run(admitted());
    assert!(
        control.is_some_and(|code| code != Code::Unsupported),
        "the admitted declaration must pass worker admission: {control:?}"
    );
    for (name, declaration) in refused() {
        assert_eq!(run(declaration), Some(Code::Unsupported), "{name}");
    }
    assert!(!adapter.executable.path.exists());
}
