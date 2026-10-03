use super::*;
use crate::local::{config, registry};
use std::{cell::Cell, collections::BTreeSet, sync::atomic::Ordering, time::Duration};

struct Target {
    _root: tempfile::TempDir,
    paths: Paths,
    schema: String,
}
impl Target {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut config = Config::load(&paths.config).unwrap();
        let adapter = Adapter {
            instance_id: "read-instance".into(),
            adapter_id: "read-adapter".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable: config::Executable {
                path: root.path().join("absent-provider"),
                sha256: "a".repeat(64),
                args: vec![],
            },
            permissions: config::Permissions {
                profiles: BTreeSet::from(["token".into()]),
                operations: BTreeSet::from(["item.read".into(), "absent".into()]),
            },
        };
        config.adapters.insert("fixture".into(), adapter.clone());
        std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: adapter.instance_id.clone(),
            adapter: adapter.adapter_id.clone(),
            revision: "desc-1".into(),
            operations: vec![connectors_core::Operation {
                id: "item.read".into(),
                description: "Read fixture".into(),
                contract: "operations/v1alpha1".into(),
                profile: "resource".into(),
                input_schema: json!({"type":"object","required":["n"],"properties":{"n":{"type":"number"}},"additionalProperties":false}),
                output_schema: json!({"type":"object"}),
            }],
            configuration_schema: json!({"type":"object"}),
        };
        let bootstrap = runtime::Bootstrap {
            instance: adapter.instance_id.clone(),
            adapter: adapter.adapter_id.clone(),
            protocol: adapter.protocol.clone(),
            configuration_revision: adapter.configuration_revision.clone(),
            provider_authority: "https://fixture.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: "token".into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![runtime::EntryField {
                    name: "token".into(),
                    label: "Token".into(),
                    max_bytes: 1024,
                }],
                acquisition: None,
            }],
            requirements: vec![runtime::Requirement {
                operation: "item.read".into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            }],
        };
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
        // Install the real registry namespace through the owner's acquisition
        // API. There is deliberately no credential, connection or provider.
        registry::Registry::new(&paths.state)
            .begin(
                &bootstrap.binding("token").unwrap(),
                connectors_sdk::now_ms(),
            )
            .unwrap();
        let schema = schema(&bootstrap, "item.read").unwrap();
        Self {
            _root: root,
            paths,
            schema,
        }
    }
    fn request(&self) -> Request<'_> {
        Request {
            connection: "connection",
            operation: "item.read",
            schema: &self.schema,
            revision: "desc-1",
            input: r#"{"n":1844674407370955161701}"#,
        }
    }
    fn audits(&self) -> audit::Store {
        audit::Store::new(&self.paths.state, 100_000).unwrap()
    }
    fn invoke(
        &self,
        request: &Request<'_>,
        audits: &audit::Store,
        dispatch: impl FnOnce(&Adapter) -> Result<Vec<u8>>,
    ) -> Value {
        read_with_audit(
            &self.paths,
            "owner-authority",
            "fixture",
            request,
            Instant::now() + Duration::from_secs(20),
            audits,
            dispatch,
        )
        .unwrap()
    }
    fn record(&self, audits: &audit::Store, value: &Value) -> audit::Record {
        audits
            .observe(&audit::Reference {
                instance: "read-instance".into(),
                audit_ref: value["audit_ref"].as_str().unwrap().into(),
            })
            .unwrap()
            .unwrap()
    }
}

#[test]
fn policy_precedes_revision_and_existence_without_provider_work() {
    let target = Target::new();
    let mut config = Config::load(&target.paths.config).unwrap();
    config
        .adapters
        .get_mut("fixture")
        .unwrap()
        .permissions
        .operations
        .clear();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    let audits = target.audits();
    let request = Request {
        operation: "absent",
        revision: "stale",
        input: "malformed",
        ..target.request()
    };
    let value = target.invoke(&request, &audits, |_| panic!("denied lookup dispatched"));
    assert_eq!(value["error"]["code"], "not_granted");
    assert_eq!(value["audit_status"], "complete");
    let record = target.record(&audits, &value);
    assert_eq!(record.anchor.kind, audit::Kind::EarlyRefusal);
    assert!(record.anchor.operation_id.is_none());
    assert!(record.anchor.descriptor_revision.is_none());
    assert_eq!(
        record.final_observation.unwrap().outcome,
        audit::Outcome::Refused
    );
}

#[test]
fn admitted_revision_precedes_private_existence_and_input() {
    let target = Target::new();
    let audits = target.audits();
    for (operation, revision, input, expected) in [
        ("absent", "stale", "invalid", "stale_description"),
        ("absent", "desc-1", "invalid", "not_found"),
        ("item.read", "stale", "invalid", "stale_description"),
        ("item.read", "desc-1", "invalid", "invalid_input"),
        ("item.read", "desc-1", r#"{"n":1,"n":2}"#, "invalid_input"),
    ] {
        let request = Request {
            operation,
            revision,
            input,
            ..target.request()
        };
        let value = target.invoke(&request, &audits, |_| panic!("refused read dispatched"));
        assert_eq!(value["error"]["code"], expected);
    }
}

#[test]
fn no_audit_acknowledgement_means_no_dispatch_or_public_reference() {
    let target = Target::new();
    for fault in [1, 2] {
        let audits = target.audits();
        audits.fault.store(fault, Ordering::SeqCst);
        let value = target.invoke(&target.request(), &audits, |_| {
            panic!("unacknowledged read dispatched")
        });
        assert_eq!(value["error"]["code"], "unavailable");
        assert_eq!(value["audit_status"], "unavailable");
        assert!(value["audit_ref"].is_null());
    }
}

#[test]
fn acknowledged_anchor_exists_before_the_single_provider_call() {
    let target = Target::new();
    let audits = target.audits();
    let calls = Cell::new(0);
    let value = target.invoke(&target.request(), &audits, |_| {
        calls.set(calls.get()+1);
        let db = crate::local::metadata::Metadata::inspect(&target.paths.state).unwrap();
        let count:i64 = db.connection.query_row("SELECT count(*) FROM execution_audits", [], |r|r.get(0)).unwrap();
        assert_eq!(count,1);
        Ok(br#"{"n":1844674407370955161701,"huge":1e400,"$serde_json::private::Number":"literal"}"#.to_vec())
    });
    assert_eq!(calls.get(), 1);
    assert_eq!(value["status"], "success");
    assert_eq!(value["result"]["n"].to_string(), "1844674407370955161701");
    assert_eq!(value["result"]["huge"].to_string(), "1e+400");
    assert_eq!(value["result"]["$serde_json::private::Number"], "literal");
    let record = target.record(&audits, &value);
    assert_eq!(
        record.anchor.request_id.as_deref(),
        value["request_id"].as_str()
    );
    assert_eq!(
        record.final_observation.unwrap().outcome,
        audit::Outcome::Success
    );
}

#[test]
fn final_append_failure_preserves_known_result_and_real_reference() {
    let target = Target::new();
    for fault in [1, 2, 10] {
        let audits = target.audits();
        let calls = Cell::new(0);
        let value = target.invoke(&target.request(), &audits, |_| {
            calls.set(calls.get() + 1);
            audits.fault.store(fault, Ordering::SeqCst);
            Ok(br#"{"result":"known"}"#.to_vec())
        });
        assert_eq!(value["status"], "success");
        assert_eq!(value["result"]["result"], "known");
        assert_eq!(calls.get(), 1, "audit recovery cannot repeat provider work");
        assert_eq!(
            value["audit_status"],
            if fault == 10 {
                "incomplete"
            } else {
                "complete"
            }
        );
        let record = target.record(&audits, &value);
        assert_eq!(record.final_observation.is_some(), fault != 10);
    }
}

#[test]
fn provider_protocol_failure_is_audited_without_retry_or_raw_error_text() {
    let target = Target::new();
    let audits = target.audits();
    let value = target.invoke(&target.request(), &audits, |_| {
        Ok(br#"{"n":1,"n":2}"#.to_vec())
    });
    assert_eq!(value["error"]["code"], "upstream_protocol");
    assert_eq!(value["audit_status"], "complete");
    assert_eq!(
        target
            .record(&audits, &value)
            .final_observation
            .unwrap()
            .code
            .as_deref(),
        Some("upstream_protocol")
    );
}

#[test]
fn complete_envelope_budget_returns_capacity_instead_of_partial_result() {
    let target = Target::new();
    let audits = target.audits();
    let value = target.invoke(&target.request(), &audits, |_| {
        // The business document fits; the complete service envelope does not.
        Ok(format!("\"{}\"", "x".repeat(runtime::RESULT_LIMIT - 2)).into_bytes())
    });
    assert_eq!(value["error"]["code"], "capacity");
    assert!(value.get("result").is_none());
    assert_eq!(value["audit_status"], "complete");
    assert_eq!(
        target
            .record(&audits, &value)
            .final_observation
            .unwrap()
            .code
            .as_deref(),
        Some("capacity")
    );
}
