use super::*;
use std::sync::atomic::AtomicUsize;

#[derive(Default)]
struct Projection {
    epoch: AtomicUsize,
    mode: AtomicUsize,
}
impl ReadPolicy for Projection {
    fn metadata(
        &self,
        _: &Paths,
        _: &str,
        _: &Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> Result<ProjectionMetadata> {
        let epoch = self.epoch.load(Ordering::SeqCst);
        let declaration = json!({"effects":["network"],"semantic_effects":[],
            "risk": if epoch == 0 {"low"} else {"high"},
            "idempotency":{"kind":"none"},"approval":"not_required",
            "limits":{"request_bytes":65536,"result_bytes":4194304,
                "execution_ms":20000,"provider_ms":15000,"connect_ms":5000}});
        let operations = std::collections::BTreeMap::from([(
            "item.read".into(),
            connectors_core::operation_metadata::Metadata::parse(
                &serde_json::to_vec(&declaration).unwrap(),
                "resource",
                65536,
            )
            .unwrap(),
        )]);
        let with_revision = |revision| ProjectionMetadata {
            revision,
            operations,
        };
        match self.mode.load(Ordering::SeqCst) {
            1 => Err(Code::Unavailable.into()),
            2 => Ok(with_revision(String::new())),
            3 => Ok(with_revision("incorrectly-constant".into())),
            _ => Ok(with_revision(connectors_core::digest(
                &json!({"bootstrap":bootstrap,"epoch":epoch}),
            ))),
        }
    }
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        panic!("metadata must not admit a business invocation")
    }
}

fn describe(
    target: &Target,
    audits: &audit::Store,
    projection: &dyn ReadPolicy,
    after_admission: impl FnOnce(),
) -> Value {
    metadata::describe_using(
        (
            &target.paths,
            "owner-authority",
            Instant::now() + Duration::from_secs(20),
        ),
        "fixture",
        audits,
        Some(projection),
        after_admission,
    )
    .unwrap()
}

#[test]
fn revision_uses_original_cache_while_private_result_omits_hidden_operations() {
    let target = Target::new();
    let mut config = Config::load(&target.paths.config).unwrap();
    let adapter = config.adapters.get_mut("fixture").unwrap();
    adapter.permissions.operations.clear();
    let original = runtime::state::State::new(&target.paths.state)
        .cached(&adapter.instance_id, &adapter.selection())
        .unwrap()
        .unwrap();
    let policy = Projection::default();
    let expected = policy
        .metadata_revision(&target.paths, "fixture", adapter, &original)
        .unwrap();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    let audits = target.audits();
    let value = describe(&target, &audits, &policy, || {});
    assert_eq!(value["status"], "success");
    assert_eq!(value["audit_status"], "complete");
    assert_eq!(value["result"]["projection_revision"], expected);
    let filtered: runtime::Bootstrap =
        serde_json::from_value(value["result"]["bootstrap"].clone()).unwrap();
    assert!(filtered.descriptor().unwrap().operations.is_empty());
    assert!(filtered.requirements.is_empty());
    assert_eq!(value["result"]["operation_metadata"], json!({}));
    assert_ne!(
        expected,
        policy
            .metadata_revision(
                &target.paths,
                "fixture",
                &config.adapters["fixture"],
                &filtered
            )
            .unwrap()
    );
    assert_eq!(
        target.record(&audits, &value).anchor.activity,
        Some(audit::Activity::Describe)
    );
    assert!(!target._root.path().join("absent-provider").exists());
}

#[test]
fn coherent_metadata_is_emitted_and_changed_values_cannot_hide_behind_a_constant_revision() {
    let target = Target::new();
    let audits = target.audits();
    let policy = Projection::default();
    let value = describe(&target, &audits, &policy, || {});
    assert_eq!(value["status"], "success");
    assert_eq!(
        value["result"]["operation_metadata"]["item.read"]["risk"],
        "low"
    );
    policy.mode.store(3, Ordering::SeqCst);
    let changed = describe(&target, &audits, &policy, || {
        policy.epoch.store(1, Ordering::SeqCst);
    });
    assert_eq!(changed["error"]["code"], "stale_description");
    assert!(changed.get("result").is_none());
}

#[test]
fn projection_change_after_real_audit_acknowledgement_releases_no_metadata() {
    let target = Target::new();
    let audits = target.audits();
    let policy = Projection::default();
    let value = describe(&target, &audits, &policy, || {
        policy.epoch.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(value["error"]["code"], "stale_description");
    assert_eq!(value["audit_status"], "complete");
    assert!(value.get("result").is_none());
    assert_eq!(
        target
            .record(&audits, &value)
            .final_observation
            .unwrap()
            .outcome,
        audit::Outcome::Error
    );
}

#[test]
fn unavailable_invalid_and_unbound_projection_are_audited_refusals() {
    let target = Target::new();
    let audits = target.audits();
    let policy = Projection::default();
    for mode in [1, 2] {
        policy.mode.store(mode, Ordering::SeqCst);
        let value = describe(&target, &audits, &policy, || {
            panic!("unavailable metadata cannot pass admission")
        });
        assert_eq!(value["error"]["code"], "unavailable");
        assert_eq!(value["audit_status"], "complete");
        assert!(value.get("result").is_none());
        assert_eq!(
            target
                .record(&audits, &value)
                .final_observation
                .unwrap()
                .outcome,
            audit::Outcome::Refused
        );
    }
    let value = describe(&target, &audits, &UnboundReadPolicy, || {
        panic!("unbound projection cannot pass admission")
    });
    assert_eq!(value["error"]["code"], "unsupported");
    assert!(value.get("result").is_none());
}

#[test]
fn projection_loss_and_owner_policy_change_after_acknowledgement_release_no_result() {
    let target = Target::new();
    let audits = target.audits();
    let policy = Projection::default();
    let lost = describe(&target, &audits, &policy, || {
        policy.mode.store(1, Ordering::SeqCst);
    });
    assert_eq!(lost["error"]["code"], "unavailable");
    assert!(lost.get("result").is_none());
    policy.mode.store(0, Ordering::SeqCst);
    let withdrawn = describe(&target, &audits, &policy, || {
        let mut config = Config::load(&target.paths.config).unwrap();
        config
            .adapters
            .get_mut("fixture")
            .unwrap()
            .permissions
            .operations
            .clear();
        std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    });
    assert_eq!(withdrawn["error"]["code"], "stale_description");
    assert!(withdrawn.get("result").is_none());
}
