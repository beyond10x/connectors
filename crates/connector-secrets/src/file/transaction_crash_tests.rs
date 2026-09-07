use super::*;
use crate::{
    CredentialScope, PreparedSecretError, PreparedSecretStore, SecretProposalDigest,
    SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const OLD: &str = "SENTINEL-NOT-A-REAL-SECRET-crash-old";
const NEW: &str = "SENTINEL-NOT-A-REAL-SECRET-crash-new";

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "connector-secrets-transaction-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    fn store(&self) -> PathBuf {
        self.0.join("store").join("credentials")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn generation() -> SecretTransactionGeneration {
    SecretTransactionGeneration::from_protocol_bytes([0, 0, 0, 0, 0, 0, 0, 1]).expect("non-zero")
}

fn id() -> SecretTransactionId {
    SecretTransactionId::new(generation(), [0x51; 24])
}

fn digest() -> SecretProposalDigest {
    SecretProposalDigest::from_protocol_bytes([0x52; 32])
}

fn reference() -> CredentialRef {
    CredentialRef::new("tenant-a", "com.acme.api", "default", "token").expect("valid")
}

fn batch() -> SecretBatch {
    let reference = reference();
    let mut batch = SecretBatch::new(
        CredentialScope::new(reference.tenant(), reference.authority()).expect("scope"),
    );
    batch.put(reference, Secret::new(NEW)).expect("mutation");
    batch
}

#[tokio::test]
async fn failed_acknowledgement_requires_readback_before_another_write() {
    let scratch = Scratch::new("uncertain-acknowledgement");
    let store = FileStore::open(scratch.store()).unwrap();
    store.prepare(id(), digest(), &batch()).await.unwrap();
    store.commit(id()).await.unwrap();
    store.fail_next_write.store(true, Ordering::SeqCst);
    assert_eq!(
        store.acknowledge(id()).await,
        Err(PreparedSecretError::Backend)
    );
    assert_eq!(
        store.retirement_watermark().await,
        Err(PreparedSecretError::Backend)
    );
    let next = SecretTransactionId::new(generation().checked_next().unwrap(), [3; 24]);
    assert_eq!(
        store.prepare(next, digest(), &batch()).await,
        Err(PreparedSecretError::Backend)
    );
    assert!(store.put(&reference(), &Secret::new(OLD)).await.is_err());
    assert!(store.delete(&reference()).await.is_err());
    assert!(store.apply(&batch()).await.is_err());
    assert_eq!(
        store.state(id()).await,
        Ok(SecretTransactionState::Committed)
    );
    store.acknowledge(id()).await.unwrap();
    assert_eq!(store.state(id()).await, Err(PreparedSecretError::Retired));
}

fn released_v0_19_1_accepts(contents: &str) -> bool {
    // This is the released v0.19.1 parser's version gate: any version line other than v1 is a
    // hard refusal. Keep the fixture small so it proves the upgrade boundary, not a second
    // implementation of the legacy store.
    !contents.lines().any(|line| {
        line.strip_prefix(VERSION_PREFIX)
            .is_some_and(|version| version != VERSION)
    })
}

fn run(operation: &str, store: &Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let store = FileStore::open(store).expect("child open");
        match operation {
            "prepare" => {
                let _ = store.prepare(id(), digest(), &batch()).await;
            }
            "commit" => {
                let _ = store.commit(id()).await;
            }
            "abort" => {
                let _ = store.abort(id()).await;
            }
            "reclaim" => {
                let _ = store.reclaim(generation()).await;
            }
            "acknowledge" => {
                let _ = store.acknowledge(id()).await;
            }
            _ => panic!("unknown child operation"),
        }
    });
    panic!("the configured crash boundary was not reached");
}

#[test]
fn prepared_crash_child() {
    let Ok(operation) = std::env::var("CONNECTOR_SECRETS_CRASH_CHILD") else {
        return;
    };
    let store = PathBuf::from(std::env::var_os("CONNECTOR_SECRETS_CRASH_STORE").expect("path"));
    run(&operation, &store);
}

#[test]
fn lease_child() {
    let Ok(mode) = std::env::var("CONNECTOR_SECRETS_LEASE_CHILD") else {
        return;
    };
    let store = PathBuf::from(std::env::var_os("CONNECTOR_SECRETS_LEASE_STORE").expect("path"));
    match mode.as_str() {
        "hold" => {
            let _store = FileStore::open(&store).expect("holder open");
            let ready = std::env::var_os("CONNECTOR_SECRETS_LEASE_READY").expect("ready path");
            std::fs::write(ready, b"ready").expect("signal ready");
            std::thread::sleep(Duration::from_secs(30));
        }
        "probe" => match FileStore::open(&store) {
            Ok(_) => std::process::exit(0),
            Err(StoreError::Conflict { .. }) => std::process::exit(42),
            Err(_) => std::process::exit(43),
        },
        _ => panic!("unknown lease child mode"),
    }
}

#[test]
fn legacy_writer_child() {
    if std::env::var_os("CONNECTOR_SECRETS_LEGACY_CHILD").is_none() {
        return;
    }
    let store = PathBuf::from(
        std::env::var_os("CONNECTOR_SECRETS_LEGACY_STORE").expect("legacy store path"),
    );
    let ready = PathBuf::from(
        std::env::var_os("CONNECTOR_SECRETS_LEGACY_READY").expect("legacy ready path"),
    );
    let release = PathBuf::from(
        std::env::var_os("CONNECTOR_SECRETS_LEGACY_RELEASE").expect("legacy release path"),
    );
    let opened = std::fs::read_to_string(&store).expect("legacy open");
    assert!(released_v0_19_1_accepts(&opened), "fixture must open v1");
    std::fs::write(&ready, b"ready").expect("signal legacy open");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !release.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(release.exists(), "legacy writer was not released");
    // Released 0.19.1 held this parsed v1 image in memory and rewrote it wholesale for every
    // point mutation. Reinstalling the bytes it opened models the unsafe stale-writer edge
    // without teaching this fixture a second credential-address encoder.
    std::fs::write(&store, opened).expect("legacy v1 rewrite");
}

#[test]
fn every_durable_transaction_boundary_recovers_one_complete_state() {
    let cases: &[(&str, &[&str])] = &[
        (
            "prepare",
            &[
                "stage:create",
                "stage:write",
                "stage:flush",
                "stage:replace",
                "stage:directory-sync",
                "live:create",
                "live:write",
                "live:flush",
                "live:replace",
                "live:directory-sync",
            ],
        ),
        (
            "commit",
            &[
                "live:create",
                "live:write",
                "live:flush",
                "live:replace",
                "live:directory-sync",
                "stage:cleanup",
            ],
        ),
        (
            "abort",
            &[
                "live:create",
                "live:write",
                "live:flush",
                "live:replace",
                "live:directory-sync",
                "stage:cleanup",
            ],
        ),
        (
            "reclaim",
            &[
                "live:create",
                "live:write",
                "live:flush",
                "live:replace",
                "live:directory-sync",
            ],
        ),
        (
            "acknowledge",
            &[
                "live:create",
                "live:write",
                "live:flush",
                "live:replace",
                "live:directory-sync",
            ],
        ),
    ];

    for (operation, boundaries) in cases {
        for boundary in *boundaries {
            let scratch = Scratch::new(&format!(
                "crash-{}-{}",
                operation,
                boundary.replace(':', "-")
            ));
            let path = scratch.store();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("runtime");
            runtime.block_on(async {
                let store = FileStore::open(&path).expect("setup open");
                store
                    .put(&reference(), &Secret::new(OLD))
                    .await
                    .expect("seed");
                if matches!(*operation, "commit" | "abort" | "acknowledge") {
                    store
                        .prepare(id(), digest(), &batch())
                        .await
                        .expect("prepare");
                    if *operation == "acknowledge" {
                        store.commit(id()).await.expect("committed receipt");
                    }
                } else if *operation == "reclaim" {
                    store.abort(id()).await.expect("terminal tombstone");
                }
            });

            let status = Command::new(std::env::current_exe().expect("test executable"))
                .arg("--exact")
                .arg("file::transaction_crash_tests::prepared_crash_child")
                .arg("--nocapture")
                .env("CONNECTOR_SECRETS_CRASH_CHILD", operation)
                .env("CONNECTOR_SECRETS_CRASH_STORE", &path)
                .env("CONNECTOR_SECRETS_CRASH_AT", boundary)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("spawn crash child");
            assert!(!status.success(), "{operation} did not crash at {boundary}");

            runtime.block_on(async {
                let recovered = FileStore::open(&path).unwrap_or_else(|error| {
                    panic!("{operation}/{boundary} did not recover: {error}")
                });
                let transaction = recovered.state(id()).await;
                let value = recovered
                    .get(&reference())
                    .await
                    .expect("one complete credential image")
                    .expose_secret()
                    .to_owned();
                match *operation {
                    "prepare" => match transaction {
                        Ok(SecretTransactionState::Absent) => assert_eq!(value, OLD),
                        Ok(SecretTransactionState::Prepared) => {
                            assert_eq!(value, OLD);
                            recovered
                                .commit(id())
                                .await
                                .expect("candidate remains committable");
                            assert_eq!(
                                recovered
                                    .get(&reference())
                                    .await
                                    .expect("new")
                                    .expose_secret(),
                                NEW
                            );
                        }
                        other => panic!("unexpected prepare recovery at {boundary}: {other:?}"),
                    },
                    "commit" => match transaction {
                        Ok(SecretTransactionState::Prepared) => assert_eq!(value, OLD),
                        Ok(SecretTransactionState::Committed) => assert_eq!(value, NEW),
                        other => panic!("unexpected commit recovery at {boundary}: {other:?}"),
                    },
                    "abort" => match transaction {
                        Ok(SecretTransactionState::Prepared)
                        | Ok(SecretTransactionState::Absent) => assert_eq!(value, OLD),
                        other => panic!("unexpected abort recovery at {boundary}: {other:?}"),
                    },
                    "reclaim" => match transaction {
                        Ok(SecretTransactionState::Absent) | Err(PreparedSecretError::Retired) => {
                            assert_eq!(value, OLD)
                        }
                        other => panic!("unexpected reclaim recovery at {boundary}: {other:?}"),
                    },
                    "acknowledge" => match transaction {
                        Ok(SecretTransactionState::Committed)
                        | Err(PreparedSecretError::Retired) => {
                            assert_eq!(value, NEW);
                            recovered
                                .acknowledge(id())
                                .await
                                .expect("idempotent recovery acknowledgement");
                        }
                        other => {
                            panic!("unexpected acknowledgement recovery at {boundary}: {other:?}")
                        }
                    },
                    _ => unreachable!(),
                }
            });
        }
    }
}

#[test]
fn two_children_prove_lease_refusal_and_abrupt_release() {
    let scratch = Scratch::new("lease-processes");
    let path = scratch.store();
    drop(FileStore::open(&path).expect("initialize"));
    let ready = scratch.0.join("holder.ready");
    let executable = std::env::current_exe().expect("test executable");
    let mut holder = Command::new(&executable)
        .arg("--exact")
        .arg("file::transaction_crash_tests::lease_child")
        .arg("--nocapture")
        .env("CONNECTOR_SECRETS_LEASE_CHILD", "hold")
        .env("CONNECTOR_SECRETS_LEASE_STORE", &path)
        .env("CONNECTOR_SECRETS_LEASE_READY", &ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn holder");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "holder did not acquire the lease");

    let probe = || {
        Command::new(&executable)
            .arg("--exact")
            .arg("file::transaction_crash_tests::lease_child")
            .arg("--nocapture")
            .env("CONNECTOR_SECRETS_LEASE_CHILD", "probe")
            .env("CONNECTOR_SECRETS_LEASE_STORE", &path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawn probe")
    };
    assert_eq!(probe().code(), Some(42), "contending child must refuse");
    holder.kill().expect("abruptly terminate holder");
    holder.wait().expect("reap holder");
    assert!(
        probe().success(),
        "kernel lease must release after process exit"
    );
}

#[test]
fn native_upgrade_fixture_proves_legacy_quiescence_and_v2_refusal() {
    let scratch = Scratch::new("legacy-upgrade");
    let path = scratch.store();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let store = FileStore::open(&path).expect("initialize v1");
        store
            .put(&reference(), &Secret::new(OLD))
            .await
            .expect("seed legacy value");
    });

    let ready = scratch.0.join("legacy.ready");
    let release = scratch.0.join("legacy.release");
    let mut legacy = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("file::transaction_crash_tests::legacy_writer_child")
        .arg("--nocapture")
        .env("CONNECTOR_SECRETS_LEGACY_CHILD", "1")
        .env("CONNECTOR_SECRETS_LEGACY_STORE", &path)
        .env("CONNECTOR_SECRETS_LEGACY_READY", &ready)
        .env("CONNECTOR_SECRETS_LEGACY_RELEASE", &release)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn already-open legacy writer");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "legacy writer did not open v1");

    runtime.block_on(async {
        let current = FileStore::open(&path).expect("0.20 opener ignores no active legacy lock");
        assert_eq!(
            current.abort(id()).await,
            Ok(SecretTransactionState::Absent)
        );
        let v2 = std::fs::read_to_string(&path).expect("read migrated v2");
        assert!(
            !released_v0_19_1_accepts(&v2),
            "a fresh released v0.19.1 parser must refuse v2"
        );

        std::fs::write(&release, b"rewrite").expect("release legacy writer");
        assert!(legacy.wait().expect("wait for legacy writer").success());
        assert_eq!(
            current.state(id()).await,
            Ok(SecretTransactionState::Absent),
            "an already-open legacy writer can erase the acknowledged tombstone"
        );
    });
    assert!(released_v0_19_1_accepts(
        &std::fs::read_to_string(&path).expect("legacy v1 survived")
    ));
}
