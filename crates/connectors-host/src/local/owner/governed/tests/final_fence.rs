//! Real custody and owned child evidence at the serialized dispatch boundary.
use super::*;
use crate::local::{
    keyring::custody,
    owner::supervisor::{Output, Pool, Task},
};
use connectors_sdk::Secret;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize},
    },
};

struct Provider {
    bootstrap: runtime::Bootstrap,
    marker: PathBuf,
}
#[async_trait::async_trait]
impl runtime::Adapter for Provider {
    fn bootstrap(&self) -> runtime::Bootstrap {
        self.bootstrap.clone()
    }
    fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(self.bootstrap.clone())
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(self.bootstrap.clone())
    }
    async fn invoke_bounded(
        &self,
        operation: &str,
        partition: &str,
        material: Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        self.invoke(operation, partition, material, input).await
    }
    async fn validate(&self, _: &str, _: Secret) -> runtime::Result<runtime::Baseline> {
        Err(runtime::Failure::Unsupported)
    }
    async fn invoke(
        &self,
        operation: &str,
        _: &str,
        material: Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        assert_eq!(operation, "item.read");
        assert!(material.0 == b"fictional-fence-only");
        // Exclusive creation detects any repeated business invocation too.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.marker)
            .unwrap();
        file.sync_all().unwrap();
        Ok(json!({"observed":true}))
    }
}

#[test]
fn provider_fixture() {
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--connectors-private-fd") {
        return;
    }
    let root = PathBuf::from(
        args.iter()
            .find_map(|arg| arg.strip_prefix("fence-root="))
            .unwrap(),
    );
    let bootstrap =
        serde_json::from_slice(&std::fs::read(root.join("bootstrap.json")).unwrap()).unwrap();
    runtime::serve(
        3,
        Provider {
            bootstrap,
            marker: root.join("provider-invoked"),
        },
    )
    .unwrap();
}

struct Policy {
    calls: AtomicUsize,
    allow: AtomicBool,
    admitted: usize,
}
impl ReadPolicy for Policy {
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call < self.admitted || self.allow.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(Code::Forbidden.into())
        }
    }
    fn metadata(
        &self,
        _: &Paths,
        _: &str,
        _: &Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> Result<ProjectionMetadata> {
        let descriptor = bootstrap.descriptor()?;
        let metadata = connectors_core::operation_metadata::Metadata::parse(
            &serde_json::to_vec(&super::bounded_reads::declaration(false)).unwrap(),
            &descriptor.operation("item.read").unwrap().profile,
            65536,
        )
        .unwrap();
        Ok(ProjectionMetadata {
            revision: "projection".into(),
            operations: std::collections::BTreeMap::from([("item.read".into(), metadata)]),
        })
    }
}

#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn withdrawal_at_final_dispatch_prevents_real_child_invocation() {
    for bounded in [false, true] {
        exercise(bounded);
    }
}

fn exercise(bounded: bool) {
    let custody_fixture = custody::tests::Fixture::new();
    let mut target = Target::new();
    if bounded {
        super::bounded_reads::prepare(&mut target, false);
    }
    let bootstrap = crate::local::owner::cached(&target.paths, "fixture").unwrap();
    let mut config = Config::load(&target.paths.config).unwrap();
    config.secret_service_socket = Some(custody_fixture.socket.clone());
    let adapter = config.adapters.get_mut("fixture").unwrap();
    adapter.executable.path = std::env::current_exe().unwrap();
    adapter.executable.sha256 = hex::encode(Sha256::digest(
        std::fs::read(&adapter.executable.path).unwrap(),
    ));
    adapter.executable.args = vec![
        "--exact".into(),
        "local::owner::governed::tests::final_fence::provider_fixture".into(),
        "--".into(),
        format!("fence-root={}", target._root.path().display()),
    ];
    let adapter = adapter.clone();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    std::fs::write(
        target._root.path().join("bootstrap.json"),
        serde_json::to_vec(&bootstrap).unwrap(),
    )
    .unwrap();
    runtime::state::State::new(&target.paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();

    let now = connectors_sdk::now_ms();
    let registry = registry::Registry::new(&target.paths.state);
    let acquired = registry
        .begin(&bootstrap.binding("token").unwrap(), now)
        .unwrap();
    let claim = registry.consume(acquired, now).unwrap();
    let material = Secret(b"fictional-fence-only".to_vec());
    let candidate = registry
        .prepare(
            &claim,
            registry::ValidatedBaseline {
                identity: registry::ExternalIdentity {
                    kind: "fixture".into(),
                    subject: "owner".into(),
                },
                granted_scopes: Some(BTreeSet::new()),
                credential_expires_at_ms: None,
                collected_at_ms: now,
                valid_until_ms: now + 60_000,
            },
            material.0.len(),
            now,
        )
        .unwrap();
    let version = candidate.version();
    let store = custody::Store::open_at(version.scope(), Some(&custody_fixture.socket)).unwrap();
    let written = store
        .write_new_guarded(version, &material, || Ok(()))
        .unwrap();
    let stored = registry.acknowledge(candidate, written, now).unwrap();
    let connection = registry.publish(stored, now).unwrap();

    let policy = Arc::new(Policy {
        calls: AtomicUsize::new(0),
        allow: AtomicBool::new(false),
        admitted: if bounded { 3 } else { 1 },
    });
    let pool = Pool::with_read_policy(
        Arc::new(Paths {
            config: target.paths.config.clone(),
            state: target.paths.state.clone(),
        }),
        "owner-authority".into(),
        policy.clone(),
    );
    let invoke = || {
        let request = target.request();
        let budget =
            bounded.then(|| runtime::ReadBudget::start(20000, 15000, 65536, 4194304).unwrap());
        let dispatch = || {
            pool.run(
                "fixture",
                &adapter,
                Task::Invoke {
                    connection: connection.clone(),
                    operation: request.operation.into(),
                    schema: request.schema.into(),
                    revision: request.revision.into(),
                    document: request.input.as_bytes().to_vec(),
                    governed: true,
                    projection_revision: Some("projection".into()),
                    budget,
                },
                connectors_sdk::now_ms() + 30_000,
            )
        };
        if let Some(budget) = budget {
            super::super::bounded::read(
                &target.paths,
                "owner-authority",
                "fixture",
                &request,
                (policy.as_ref(), "projection"),
                budget,
                |_| match dispatch()? {
                    Output::Document(bytes) => Ok(bytes),
                    _ => Err(Code::Unavailable.into()),
                },
            )
            .map(Output::Value)
        } else {
            dispatch()
        }
    };
    let refused = invoke();
    assert!(
        !target._root.path().join("provider-invoked").exists(),
        "withdrawn projection reached the real provider"
    );
    if bounded {
        let Output::Value(value) = refused.unwrap() else {
            panic!("bounded service envelope required");
        };
        assert_eq!(value["error"]["code"], "forbidden");
        assert_eq!(value["audit_status"], "complete");
    } else {
        assert!(
            matches!(
                refused,
                Err(Error {
                    code: Code::Forbidden,
                    ..
                })
            ),
            "final projection refusal expected"
        );
    }
    assert_eq!(
        policy.calls.load(Ordering::SeqCst),
        if bounded { 4 } else { 2 },
        "must reach final admission after real custody read"
    );
    let uses = || {
        let metadata = crate::local::metadata::Metadata::inspect(&target.paths.state).unwrap();
        metadata
            .connection
            .query_row(
                "SELECT count(*),sum(dispatched),sum(released) FROM registry_uses",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .unwrap()
    };
    assert_eq!(
        uses(),
        (1, 0, 1),
        "refusal must cancel the captured credential use"
    );

    policy.allow.store(true, Ordering::SeqCst);
    let accepted = invoke();
    pool.shutdown().unwrap();
    match accepted.unwrap() {
        Output::Document(bytes) if !bounded => assert_eq!(
            connectors_core::json::decode(&bytes, 64).unwrap(),
            json!({"observed":true})
        ),
        Output::Value(value) if bounded => {
            assert_eq!(value["result"], json!({"observed":true}));
            assert_eq!(value["audit_status"], "complete");
        }
        _ => panic!("governed read did not return selected carrier"),
    }
    assert!(target._root.path().join("provider-invoked").is_file());
    assert_eq!(
        policy.calls.load(Ordering::SeqCst),
        if bounded { 8 } else { 4 }
    );
    assert_eq!(
        uses(),
        (2, 1, 2),
        "exactly one use dispatched and both were released"
    );
}
