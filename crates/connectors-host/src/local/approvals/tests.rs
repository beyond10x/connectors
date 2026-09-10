use super::*;
use crate::local::{metadata::Metadata, mutations as m};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_sdk::Secret;
use ring::signature::{self, KeyPair};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Barrier, Mutex, RwLock, RwLockReadGuard, atomic::Ordering},
};

const NOW: i64 = 1_789_056_000_000;
// Public RFC 8032 §7.1 test material, never a deployment signing key.
const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
#[derive(Clone)]
struct TestClock(Arc<Mutex<m::Result<m::ClockInterval>>>);
impl m::Clock for TestClock {
    fn now(&self) -> m::Result<m::ClockInterval> {
        *self.0.lock().unwrap()
    }
}
impl TestClock {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Ok(m::ClockInterval {
            lower_unix_ms: NOW,
            upper_unix_ms: NOW + 2000,
        }))))
    }
    fn set(&self, lower: i64, upper: i64) {
        *self.0.lock().unwrap() = Ok(m::ClockInterval {
            lower_unix_ms: lower,
            upper_unix_ms: upper,
        });
    }
}
struct Policy(RwLock<(Subject, ConfiguredApprovalKey)>);
struct Guard<'a>(RwLockReadGuard<'a, (Subject, ConfiguredApprovalKey)>);
impl CurrentAdmission for Guard<'_> {
    fn key(&self) -> &ConfiguredApprovalKey {
        &self.0.1
    }
}
impl ReceiverPolicy for Policy {
    type Guard<'a> = Guard<'a>;
    fn admit<'a>(&'a self, subject: &Subject, kid: &str) -> Result<Guard<'a>> {
        let guard = self.0.read().unwrap();
        if &guard.0 != subject || guard.1.kid != kid {
            return Err(Failure::Refused);
        }
        Ok(Guard(guard))
    }
}
impl IssuancePolicy for Policy {
    type Guard<'a> = Guard<'a>;
    fn authorize<'a>(&'a self, subject: &Subject, kid: &str) -> Result<Guard<'a>> {
        self.admit(subject, kid)
    }
}
fn subject() -> Subject {
    Subject {
        format: "connectors.approval-subject/v1".into(),
        target: Target {
            instance: "instance".into(),
            operation: "operation".into(),
            connection: "connection".into(),
            connection_revision: "revision".into(),
            contract: "operations/v1alpha1".into(),
            profile: "mutation".into(),
            descriptor_revision: "descriptor".into(),
            configuration_revision: "config".into(),
        },
        authority: Authority {
            scope: Scope {
                tenant: None,
                realm: None,
                caller: "caller".into(),
                executor: None,
            },
            current_authority: None,
            executor: None,
        },
        origin: Origin {
            kind: OriginKind::Direct,
            authority_ref: "instance".into(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: "a".repeat(64),
        approval_mode: "required".into(),
    }
}
fn policy(subject: &Subject) -> Policy {
    Policy(RwLock::new((
        subject.clone(),
        ConfiguredApprovalKey {
            issuer: "issuer".into(),
            audience: "approval:instance".into(),
            kid: "key-1".into(),
            public_key: URL_SAFE_NO_PAD.encode(hex::decode(PUBLIC).unwrap()),
            not_before_unix_ms: 0,
            not_after_unix_ms: NOW + 1_000_000,
            revoked: false,
        },
    )))
}
fn signer() -> Signer {
    Signer::from_seed(Secret(hex::decode(SEED).unwrap()), "key-1".into()).unwrap()
}
fn candidate(s: &Subject, e: &Evidence) -> m::Candidate {
    m::Candidate {
        namespace: m::Namespace {
            receiver_instance: s.target.instance.clone(),
            tenant: s.authority.scope.tenant.clone(),
            realm: s.authority.scope.realm.clone(),
            caller: s.authority.scope.caller.clone(),
            executor: s.authority.scope.executor.clone(),
            origin: match s.origin.kind {
                OriginKind::Direct => m::Origin::Direct,
                OriginKind::Federated => m::Origin::Federated {
                    gateway_instance: s.origin.authority_ref.clone(),
                },
            },
        },
        fingerprint: m::Fingerprint {
            operation: m::OperationRef {
                instance: s.target.instance.clone(),
                adapter: "adapter".into(),
                operation: s.target.operation.clone(),
            },
            connection_ref: s.target.connection.clone(),
            connection_revision: s.target.connection_revision.clone(),
            contract_ref: s.target.contract.clone(),
            profile: s.target.profile.clone(),
            descriptor_revision: s.target.descriptor_revision.clone(),
            configuration_revision: s.target.configuration_revision.clone(),
            canonicalization_version: s.canonicalization.clone(),
            input_digest: s.input_sha256.clone(),
            route: s.route.clone(),
        },
        caller_key: None,
        request_id: "request".into(),
        approval: m::Approval::Required {
            reference: e.reference().into(),
        },
    }
}
fn fixture() -> (
    tempfile::TempDir,
    TestClock,
    m::Store<TestClock>,
    Store<TestClock>,
) {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let metadata = Metadata::initialize(root.path()).unwrap();
    metadata.connection.execute_batch("INSERT INTO registry_instances VALUES ('instance','adapter','config',0);
      INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1','{}');
      INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms) VALUES ('connection','instance','profile','{}','scope','revision','fence','live',1,1);").unwrap();
    drop(metadata);
    let clock = TestClock::new();
    let ledger = m::Store::new(root.path(), clock.clone(), m::Limits::default()).unwrap();
    let spend = Store::new(root.path(), clock.clone(), 100).unwrap();
    (root, clock, ledger, spend)
}
fn prepare(
    ledger: &m::Store<TestClock>,
    s: &Subject,
    e: &Evidence,
    p: &Policy,
    clock: &TestClock,
) -> m::Prepared {
    let verified = verify(e, s, p, clock).unwrap();
    match ledger
        .prepare_approved(&candidate(s, e), &verified)
        .unwrap()
    {
        m::Preparation::Prepared(p) => p,
        _ => panic!("unexpected replay"),
    }
}
fn count(root: &Path) -> u32 {
    Metadata::inspect(root)
        .unwrap()
        .connection
        .query_row("SELECT count(*) FROM approval_redemptions", [], |r| {
            r.get(0)
        })
        .unwrap()
}
fn parts(e: &Evidence) -> (Value, Value) {
    let parts: Vec<_> = std::str::from_utf8(e.bytes()).unwrap().split('.').collect();
    (
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap(),
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap(),
    )
}
// Deliberately independent framing helper: signs even invalid profile bytes.
fn signed_raw(header: &[u8], claims: &[u8], reference: &str) -> Evidence {
    let pair = signature::Ed25519KeyPair::from_seed_unchecked(&hex::decode(SEED).unwrap()).unwrap();
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims)
    );
    let signature = pair.sign(input.as_bytes());
    Evidence::from_protected(
        reference.into(),
        Secret(format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref())).into_bytes()),
    )
    .unwrap()
}
fn signed(header: &Value, claims: &Value, reference: &str) -> Evidence {
    signed_raw(
        &connectors_core::canonical(header),
        &connectors_core::canonical(claims),
        reference,
    )
}

#[test]
fn rfc8032_vector_and_exact_fixed_algorithm_round_trip() {
    // https://www.rfc-editor.org/rfc/rfc8032.html#section-7.1
    let pair = signature::Ed25519KeyPair::from_seed_unchecked(&hex::decode(SEED).unwrap()).unwrap();
    assert_eq!(hex::encode(pair.public_key()), PUBLIC);
    let expected = hex::decode("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b").unwrap();
    assert_eq!(pair.sign(b"").as_ref(), expected);
    signature::UnparsedPublicKey::new(&signature::ED25519, hex::decode(PUBLIC).unwrap())
        .verify(b"", &expected)
        .unwrap();
    let s = subject();
    let p = policy(&s);
    let clock = TestClock::new();
    let first = signer().issue(&s, &p, &clock).unwrap();
    let second = signer().issue(&s, &p, &clock).unwrap();
    assert_ne!(first.reference(), second.reference());
    verify(&first, &s, &p, &clock).unwrap();
    let (header, claims) = parts(&first);
    assert_eq!(
        header,
        json!({"alg":"Ed25519","kid":"key-1","typ":"b10x.connectors-approval.v1+jws"})
    );
    assert_eq!(claims["iat"], NOW / 1000 + 1);
    assert_eq!(claims["nbf"], NOW / 1000 - 4);
    assert_eq!(claims["exp"], NOW / 1000 + 296);
    assert_eq!(
        first.bytes(),
        signed(&header, &claims, first.reference()).bytes()
    );
}

#[test]
fn canonical_framing_rejects_omission_duplicates_unknown_fields_and_alternate_spellings() {
    let s = subject();
    let p = policy(&s);
    let clock = TestClock::new();
    let evidence = signer().issue(&s, &p, &clock).unwrap();
    let (header, claims) = parts(&evidence);
    let header_bytes = connectors_core::canonical(&header);
    let raw = String::from_utf8(connectors_core::canonical(&claims)).unwrap();
    for malformed in [
        raw.replace(",\"tenant\":null", ""),
        raw.replace("\"realm\":null,", ""),
        raw.replace("\"current_authority\":null,", ""),
        raw.replace(",\"executor\":null", ""),
        raw.replace("\"route\":null,", ""),
        raw.replace(
            "\"caller\":\"caller\"",
            "\"caller\":\"caller\",\"caller\":\"caller\"",
        ),
        raw.replacen('{', "{\"unknown\":null,", 1),
        format!(" {raw}"),
        raw.replace("\"caller\"", "\"\\u0063aller\""),
        raw.replace(
            &format!("\"iat\":{}", NOW / 1000 + 1),
            &format!("\"iat\":{}.0", NOW / 1000 + 1),
        ),
    ] {
        let bad = signed_raw(&header_bytes, malformed.as_bytes(), evidence.reference());
        assert!(matches!(
            verify(&bad, &s, &p, &clock),
            Err(Failure::Refused)
        ));
    }
    for (field, value) in [
        ("alg", json!("EdDSA")),
        ("alg", json!("none")),
        ("typ", json!("JWT")),
        ("jwk", json!({})),
        ("crit", json!([])),
        ("kid", json!("other")),
    ] {
        let mut h = header.clone();
        h[field] = value;
        assert!(matches!(
            verify(&signed(&h, &claims, evidence.reference()), &s, &p, &clock),
            Err(Failure::Refused)
        ));
    }
    let text = std::str::from_utf8(evidence.bytes()).unwrap();
    let pieces: Vec<_> = text.split('.').collect();
    for raw in [
        format!("{}=.{}.{}", pieces[0], pieces[1], pieces[2]),
        format!("{text}.extra"),
        text[..text.len() - 1].to_owned(),
    ] {
        let bad = Evidence::from_protected(evidence.reference().into(), Secret(raw.into_bytes()))
            .unwrap();
        assert!(matches!(
            verify(&bad, &s, &p, &clock),
            Err(Failure::Refused)
        ));
    }
    assert!(
        Evidence::from_protected(
            evidence.reference().into(),
            Secret(vec![b'a'; 18 * 1024 + 1])
        )
        .is_err()
    );
}

#[test]
fn all_subject_coordinates_and_unicode_identity_are_bound() {
    let mut s = subject();
    s.authority.scope.tenant = Some("tenant".into());
    s.authority.scope.realm = Some("realm".into());
    s.authority.scope.executor = Some("agent".into());
    s.authority.current_authority = Some(Snapshot {
        id: "authority".into(),
        sha256: "b".repeat(64),
    });
    s.authority.executor = Some(Executor {
        agent: "agent".into(),
        revision: "agent-r1".into(),
        authority_snapshot: Snapshot {
            id: "agent-authority".into(),
            sha256: "c".repeat(64),
        },
    });
    s.origin = Origin {
        kind: OriginKind::Federated,
        authority_ref: "gateway".into(),
    };
    s.route = Some(m::Route {
        gateway_instance: "gateway".into(),
        route_id: "route".into(),
        route_revision: "r1".into(),
    });
    let p = policy(&s);
    let clock = TestClock::new();
    let evidence = signer().issue(&s, &p, &clock).unwrap();
    let (h, c) = parts(&evidence);
    let paths = [
        "/format",
        "/target/instance",
        "/target/operation",
        "/target/connection",
        "/target/connection_revision",
        "/target/contract",
        "/target/profile",
        "/target/descriptor_revision",
        "/target/configuration_revision",
        "/authority/scope/tenant",
        "/authority/scope/realm",
        "/authority/scope/caller",
        "/authority/scope/executor",
        "/authority/current_authority/id",
        "/authority/current_authority/sha256",
        "/authority/executor/agent",
        "/authority/executor/revision",
        "/authority/executor/authority_snapshot/id",
        "/authority/executor/authority_snapshot/sha256",
        "/origin/kind",
        "/origin/authority_ref",
        "/route/gateway_instance",
        "/route/route_id",
        "/route/route_revision",
        "/canonicalization",
        "/input_sha256",
        "/approval_mode",
    ];
    for path in paths {
        let mut changed = c.clone();
        *changed["subject"].pointer_mut(path).unwrap() = json!("changed");
        assert!(
            matches!(
                verify(&signed(&h, &changed, evidence.reference()), &s, &p, &clock),
                Err(Failure::Refused)
            ),
            "{path}"
        );
    }
    let mut composed = subject();
    composed.authority.scope.caller = "caf\u{e9}".into();
    let policy = policy(&composed);
    let proof = signer().issue(&composed, &policy, &clock).unwrap();
    verify(&proof, &composed, &policy, &clock).unwrap();
    composed.authority.scope.caller = "cafe\u{301}".into();
    assert!(verify(&proof, &composed, &policy, &clock).is_err());
}

#[test]
fn issuer_audience_time_key_and_clock_refusals_are_current() {
    let s = subject();
    let p = policy(&s);
    let clock = TestClock::new();
    let e = signer().issue(&s, &p, &clock).unwrap();
    let (h, c) = parts(&e);
    for (field, value) in [
        ("iss", json!("other")),
        ("aud", json!("delivery:instance")),
        ("reference", json!("b".repeat(64))),
        ("iat", json!(true)),
        ("iat", json!("1789056001")),
        ("iat", json!(i64::MAX)),
        ("nbf", json!(-1)),
        ("exp", json!(NOW / 1000 + 295)),
    ] {
        let mut changed = c.clone();
        changed[field] = value;
        assert!(matches!(
            verify(&signed(&h, &changed, e.reference()), &s, &p, &clock),
            Err(Failure::Refused)
        ));
    }
    clock.set(NOW - 4000, NOW - 4000);
    verify(&e, &s, &p, &clock).unwrap();
    clock.set(NOW - 4001, NOW - 4001);
    assert!(matches!(verify(&e, &s, &p, &clock), Err(Failure::Refused)));
    clock.set(NOW + 295999, NOW + 295999);
    verify(&e, &s, &p, &clock).unwrap();
    clock.set(NOW + 296000, NOW + 296000);
    assert!(matches!(verify(&e, &s, &p, &clock), Err(Failure::Refused)));
    for (lower, upper) in [
        (NOW, NOW + 4001),
        (NOW + 1, NOW),
        (-1, 0),
        (i64::MAX - 1, i64::MAX),
    ] {
        clock.set(lower, upper);
        assert!(matches!(
            verify(&e, &s, &p, &clock),
            Err(Failure::Unavailable)
        ));
    }
    *clock.0.lock().unwrap() = Err(m::Failure::ClockUnavailable);
    assert!(matches!(
        verify(&e, &s, &p, &clock),
        Err(Failure::Unavailable)
    ));
    clock.set(NOW, NOW + 2000);
    p.0.write().unwrap().1.revoked = true;
    assert!(matches!(verify(&e, &s, &p, &clock), Err(Failure::Refused)));
    assert!(matches!(
        signer().issue(&s, &p, &clock),
        Err(Failure::Refused)
    ));
    p.0.write().unwrap().1.revoked = false;
    p.0.write().unwrap().1.not_after_unix_ms = NOW + 2000;
    assert!(matches!(verify(&e, &s, &p, &clock), Err(Failure::Refused)));
}

#[test]
fn approved_gate_requires_original_spend_and_abort_never_refunds() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let first = prepare(&ledger, &s, &e, &p, &clock);
    assert!(matches!(
        ledger.open_dispatch(first),
        Err(m::Failure::Conflict)
    ));
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let receipt = store.spend(&prepared, &e, &s, &p).unwrap();
    let reference = prepared.reference();
    ledger
        .open_approved_dispatch(prepared, receipt)
        .unwrap()
        .consume()
        .unwrap();
    assert_eq!(
        ledger.observe(reference).unwrap().state,
        m::State::Dispatching
    );
    let another = prepare(&ledger, &s, &e, &p, &clock);
    assert!(matches!(
        store.spend(&another, &e, &s, &p),
        Err(Failure::Replayed)
    ));
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let receipt = store.spend(&prepared, &e, &s, &p).unwrap();
    ledger
        .abort(prepared.reference(), json!({"cause":"cancelled"}))
        .unwrap();
    assert!(ledger.open_approved_dispatch(prepared, receipt).is_err());
    let retry = prepare(&ledger, &s, &e, &p, &clock);
    assert!(matches!(
        store.spend(&retry, &e, &s, &p),
        Err(Failure::Replayed)
    ));
    assert_eq!(count(root.path()), 2);
}

#[test]
fn fresh_spend_rechecks_policy_key_time_and_exact_initial_proof() {
    for change in 0..5 {
        let (root, clock, ledger, store) = fixture();
        let s = subject();
        let p = policy(&s);
        let mut e = signer().issue(&s, &p, &clock).unwrap();
        let prepared = prepare(&ledger, &s, &e, &p, &clock);
        match change {
            0 => p.0.write().unwrap().1.revoked = true,
            1 => {
                p.0.write().unwrap().0.authority.current_authority = Some(Snapshot {
                    id: "changed".into(),
                    sha256: "b".repeat(64),
                })
            }
            2 => clock.set(NOW + 296000, NOW + 296000),
            3 => e = signer().issue(&s, &p, &clock).unwrap(),
            _ => {
                Metadata::update(root.path(), false)
                    .unwrap()
                    .connection
                    .execute(
                        "UPDATE registry_connections SET publication_fence='repaired'",
                        [],
                    )
                    .unwrap();
            }
        }
        assert!(matches!(
            store.spend(&prepared, &e, &s, &p),
            Err(Failure::Refused)
        ));
        assert_eq!(count(root.path()), 0);
        assert!(ledger.open_dispatch(prepared).is_err());
    }
}

#[test]
fn unkeyed_preparation_retains_authority_and_old_records_cannot_gain_it() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let verified = verify(&e, &s, &p, &clock).unwrap();
    let mut c = candidate(&s, &e);
    c.namespace.caller = "different".into();
    assert!(matches!(
        ledger.prepare_approved(&c, &verified),
        Err(m::Failure::InvalidInput)
    ));
    c = candidate(&s, &e);
    let old = match ledger.prepare(&c).unwrap() {
        m::Preparation::Prepared(p) => p,
        _ => panic!(),
    };
    assert!(store.spend(&old, &e, &s, &p).is_err());
    assert!(ledger.open_dispatch(old).is_err());
    c.approval = m::Approval::EventClaim {
        reference: "event".into(),
    };
    let event = match ledger.prepare(&c).unwrap() {
        m::Preparation::Prepared(p) => p,
        _ => panic!(),
    };
    assert!(ledger.open_dispatch(event).is_err());
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let metadata = Metadata::inspect(root.path()).unwrap();
    let captured: String = metadata
        .connection
        .query_row(
            "SELECT approval_subject FROM mutation_attempts WHERE attempt_id=?1",
            [prepared.reference().attempt_id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(captured.as_bytes(), s.canonical_bytes().unwrap());
    drop(metadata);
    store.spend(&prepared, &e, &s, &p).unwrap();
}

#[test]
fn concurrent_reference_and_attempt_spend_produce_one_receipt() {
    for same_attempt in [false, true] {
        let (root, clock, ledger, store) = fixture();
        let s = subject();
        let p = policy(&s);
        let e = signer().issue(&s, &p, &clock).unwrap();
        let handles: Vec<_> = (0..8)
            .map(|_| Arc::new(prepare(&ledger, &s, &e, &p, &clock)))
            .collect();
        let barrier = Barrier::new(8);
        let results = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|i| {
                    let prepared = &handles[if same_attempt { 0 } else { i }];
                    let barrier = &barrier;
                    let store = &store;
                    let e = &e;
                    let s = &s;
                    let p = &p;
                    scope.spawn(move || {
                        barrier.wait();
                        store.spend(prepared, e, s, p)
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|t| t.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(count(root.path()), 1);
    }
}

#[test]
fn rollback_or_lost_ack_never_reconstructs_receipt() {
    for fault in [1, 2] {
        let (root, clock, ledger, store) = fixture();
        let s = subject();
        let p = policy(&s);
        let e = signer().issue(&s, &p, &clock).unwrap();
        let prepared = prepare(&ledger, &s, &e, &p, &clock);
        store.fault.store(fault, Ordering::SeqCst);
        let error = if fault == 1 {
            Failure::MetadataUnavailable
        } else {
            Failure::OutcomeUnknown
        };
        assert!(matches!(store.spend(&prepared,&e,&s,&p),Err(v) if v==error));
        let reopened = Store::new(root.path(), clock.clone(), 100).unwrap();
        assert!(matches!(
            reopened.spend(&prepared, &e, &s, &p),
            Err(Failure::Refused)
        ));
        let reference = prepared.reference();
        assert!(ledger.open_dispatch(prepared).is_err());
        assert_eq!(ledger.recover(reference).unwrap().state, m::State::Aborted);
        assert_eq!(count(root.path()), u32::from(fault == 2));
        let replacement = prepare(&ledger, &s, &e, &p, &clock);
        if fault == 2 {
            assert!(matches!(
                reopened.spend(&replacement, &e, &s, &p),
                Err(Failure::Replayed)
            ));
        }
    }
}

#[test]
fn tombstone_capacity_and_migrations_survive_restart_without_secret_storage() {
    let (root, clock, ledger, _) = fixture();
    let s = subject();
    let p = policy(&s);
    let store = Store::new(root.path(), clock.clone(), 1).unwrap();
    let metadata = Metadata::inspect(root.path()).unwrap();
    let authority = metadata.authority().unwrap();
    assert_eq!(
        metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    drop(metadata);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    store.spend(&prepared, &e, &s, &p).unwrap();
    ledger
        .abort(prepared.reference(), json!({"cancelled":true}))
        .unwrap();
    let reopened = Store::new(root.path(), clock.clone(), 1).unwrap();
    let fresh = signer().issue(&s, &p, &clock).unwrap();
    let candidate = prepare(&ledger, &s, &fresh, &p, &clock);
    assert!(matches!(
        reopened.spend(&candidate, &fresh, &s, &p),
        Err(Failure::Capacity)
    ));
    let metadata = Metadata::update(root.path(), false).unwrap();
    assert_eq!(authority, metadata.authority().unwrap());
    assert_eq!(
        metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert!(
        metadata
            .connection
            .execute("DELETE FROM approval_redemptions", [])
            .is_err()
    );
    assert!(
        metadata
            .connection
            .execute("UPDATE approval_redemptions SET reference='changed'", [])
            .is_err()
    );
    let sql: String = metadata
        .connection
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='approval_redemptions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    for excluded in [
        "evidence",
        "private_key",
        "seed",
        "credential",
        "proof_sha256",
    ] {
        assert!(!sql.contains(excluded));
    }
    drop(metadata);
    assert_eq!(count(root.path()), 1);
}

#[test]
fn expiry_during_acknowledgement_retains_spend_without_granting_receipt() {
    struct LateClock(std::sync::atomic::AtomicUsize);
    impl m::Clock for LateClock {
        fn now(&self) -> m::Result<m::ClockInterval> {
            let time = if self.0.fetch_add(1, Ordering::SeqCst) < 2 {
                NOW
            } else {
                NOW + 296000
            };
            Ok(m::ClockInterval {
                lower_unix_ms: time,
                upper_unix_ms: time,
            })
        }
    }
    let (root, clock, ledger, _) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let store = Store::new(
        root.path(),
        LateClock(std::sync::atomic::AtomicUsize::new(0)),
        100,
    )
    .unwrap();
    assert!(matches!(
        store.spend(&prepared, &e, &s, &p),
        Err(Failure::Refused)
    ));
    assert_eq!(count(root.path()), 1);
    assert!(ledger.open_dispatch(prepared).is_err());
    let next = prepare(&ledger, &s, &e, &p, &clock);
    let store = Store::new(root.path(), clock, 100).unwrap();
    assert!(matches!(
        store.spend(&next, &e, &s, &p),
        Err(Failure::Replayed)
    ));
}

#[test]
fn concurrent_substituted_reference_cannot_rebind_one_attempt() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let other = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let good = scope.spawn(|| {
            barrier.wait();
            store.spend(&prepared, &e, &s, &p)
        });
        let substituted = scope.spawn(|| {
            barrier.wait();
            store.spend(&prepared, &other, &s, &p)
        });
        (good.join().unwrap(), substituted.join().unwrap())
    });
    assert!(matches!(results.1, Err(Failure::Refused)));
    assert_eq!(count(root.path()), u32::from(results.0.is_ok()));
    // Also exercise the independently enforced database constraint, using the
    // successful row from a new original attempt, whichever racing call won.
    let original = prepare(&ledger, &s, &other, &p, &clock);
    store.spend(&original, &other, &s, &p).unwrap();
    let metadata = Metadata::update(root.path(), false).unwrap();
    assert!(metadata.connection.execute("INSERT INTO approval_redemptions SELECT 'replacement','other-issuer',reference,instance_id,attempt_id,subject,spent_at_ms FROM approval_redemptions WHERE attempt_id=?1",[original.reference().attempt_id.to_string()]).is_err());
}

#[test]
fn unavailable_live_binding_is_storage_failure_without_receipt_or_retry() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let metadata = Metadata::update(root.path(), false).unwrap();
    // Deliberately corrupt a test-owned schema after preparation. Historical
    // migration digests cannot establish availability of current row queries.
    metadata
        .connection
        .execute_batch("PRAGMA foreign_keys=OFF; DROP TABLE registry_connections;")
        .unwrap();
    drop(metadata);
    assert!(matches!(
        store.spend(&prepared, &e, &s, &p),
        Err(Failure::MetadataUnavailable)
    ));
    assert!(matches!(
        store.spend(&prepared, &e, &s, &p),
        Err(Failure::Refused)
    ));
    assert_eq!(count(root.path()), 0);
}

#[test]
fn unavailable_redemption_evidence_cannot_open_dispatch() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let reference = prepared.reference();
    let receipt = store.spend(&prepared, &e, &s, &p).unwrap();
    Metadata::update(root.path(), false)
        .unwrap()
        .connection
        .execute_batch("DROP TABLE approval_redemptions;")
        .unwrap();
    assert!(matches!(
        ledger.open_approved_dispatch(prepared, receipt),
        Err(m::Failure::MetadataUnavailable)
    ));
    assert_eq!(ledger.observe(reference).unwrap().state, m::State::Prepared);
}

#[test]
fn policy_guard_survives_commit_and_clock_is_rechecked_after_storage() {
    struct InspectPolicy<'a> {
        policy: &'a Policy,
        root: &'a Path,
    }
    struct InspectGuard<'a> {
        inner: Guard<'a>,
        root: &'a Path,
        policy: &'a Policy,
    }
    impl CurrentAdmission for InspectGuard<'_> {
        fn key(&self) -> &ConfiguredApprovalKey {
            self.inner.key()
        }
    }
    impl Drop for InspectGuard<'_> {
        fn drop(&mut self) {
            // A concurrent authority change remains excluded while the committed
            // row is already visible to an independent SQLite connection.
            assert!(self.policy.0.try_write().is_err());
            let conn = rusqlite::Connection::open_with_flags(
                self.root.join("metadata.sqlite3"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(
                conn.query_row("SELECT count(*) FROM approval_redemptions", [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                1
            );
        }
    }
    impl ReceiverPolicy for InspectPolicy<'_> {
        type Guard<'a>
            = InspectGuard<'a>
        where
            Self: 'a;
        fn admit<'a>(&'a self, s: &Subject, kid: &str) -> Result<InspectGuard<'a>> {
            Ok(InspectGuard {
                inner: self.policy.admit(s, kid)?,
                root: self.root,
                policy: self.policy,
            })
        }
    }
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    store
        .spend(
            &prepared,
            &e,
            &s,
            &InspectPolicy {
                policy: &p,
                root: root.path(),
            },
        )
        .unwrap();
    assert!(p.0.try_write().is_ok());
    struct ExpiringClock(std::sync::atomic::AtomicUsize);
    impl m::Clock for ExpiringClock {
        fn now(&self) -> m::Result<m::ClockInterval> {
            let time = if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                NOW
            } else {
                NOW + 296000
            };
            Ok(m::ClockInterval {
                lower_unix_ms: time,
                upper_unix_ms: time,
            })
        }
    }
    let fresh = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &fresh, &p, &clock);
    let expiring = Store::new(
        root.path(),
        ExpiringClock(std::sync::atomic::AtomicUsize::new(0)),
        100,
    )
    .unwrap();
    assert!(matches!(
        expiring.spend(&prepared, &fresh, &s, &p),
        Err(Failure::Refused)
    ));
    assert_eq!(count(root.path()), 1);
}

#[test]
fn mismatched_receipt_missing_subject_and_existing_key_never_open_new_gate() {
    let (root, clock, ledger, store) = fixture();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let first = prepare(&ledger, &s, &e, &p, &clock);
    let second = prepare(&ledger, &s, &e, &p, &clock);
    let receipt = store.spend(&first, &e, &s, &p).unwrap();
    assert!(ledger.open_approved_dispatch(second, receipt).is_err());
    let fresh = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &fresh, &p, &clock);
    Metadata::update(root.path(), false)
        .unwrap()
        .connection
        .execute(
            "UPDATE mutation_attempts SET approval_subject=NULL WHERE attempt_id=?1",
            [prepared.reference().attempt_id.to_string()],
        )
        .unwrap();
    assert!(matches!(
        store.spend(&prepared, &fresh, &s, &p),
        Err(Failure::Refused)
    ));
    let verified = verify(&fresh, &s, &p, &clock).unwrap();
    let mut c = candidate(&s, &fresh);
    c.caller_key = Some("same-key".into());
    let keyed = match ledger.prepare_approved(&c, &verified).unwrap() {
        m::Preparation::Prepared(p) => p,
        _ => panic!(),
    };
    let spent = store.spend(&keyed, &fresh, &s, &p).unwrap();
    let reference = ledger
        .open_approved_dispatch(keyed, spent)
        .unwrap()
        .consume()
        .unwrap();
    ledger
        .settle(reference, &m::Outcome::Applied(json!({"safe":true})))
        .unwrap();
    clock.set(NOW + 500000, NOW + 500000);
    p.0.write().unwrap().1.revoked = true;
    assert_eq!(ledger.lookup(&c).unwrap().unwrap().reference, reference);
    assert!(matches!(
        ledger.prepare(&c).unwrap(),
        m::Preparation::Existing(_)
    ));
    assert_eq!(count(root.path()), 2);
}

#[test]
fn abrupt_process_exits_recover_without_refunding_or_resending() {
    for phase in ["spend-before", "spend-after", "gate-after", "effect-after"] {
        let (root, _, ledger, _) = fixture();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local::approvals::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("CONNECTORS_APPROVAL_CRASH_ROOT", root.path())
            .env("CONNECTORS_APPROVAL_CRASH_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let reference = m::AttemptRef {
            authority: Metadata::inspect(root.path()).unwrap().authority().unwrap(),
            attempt_id: uuid::Uuid::parse_str(
                &std::fs::read_to_string(root.path().join("attempt-id")).unwrap(),
            )
            .unwrap(),
        };
        assert_eq!(count(root.path()), u32::from(phase != "spend-before"));
        let expected = if phase.starts_with("spend") {
            m::State::Aborted
        } else {
            m::State::Indeterminate
        };
        assert_eq!(ledger.recover(reference).unwrap().state, expected);
        assert_eq!(ledger.recover(reference).unwrap().state, expected);
        assert_eq!(
            root.path().join("simulated-effect").exists(),
            phase == "effect-after"
        );
        if phase == "effect-after" {
            assert_eq!(
                std::fs::read_to_string(root.path().join("simulated-effect")).unwrap(),
                "one"
            );
        }
    }
}

#[test]
#[ignore = "subprocess entry point exercised by abrupt_process_exits"]
fn crash_child() {
    use std::io::Write;
    let root =
        std::path::PathBuf::from(std::env::var_os("CONNECTORS_APPROVAL_CRASH_ROOT").unwrap());
    let phase = std::env::var("CONNECTORS_APPROVAL_CRASH_PHASE").unwrap();
    let clock = TestClock::new();
    let ledger = m::Store::new(&root, clock.clone(), m::Limits::default()).unwrap();
    let store = Store::new(&root, clock.clone(), 100).unwrap();
    let s = subject();
    let p = policy(&s);
    let e = signer().issue(&s, &p, &clock).unwrap();
    let prepared = prepare(&ledger, &s, &e, &p, &clock);
    let file = std::fs::File::create_new(root.join("attempt-id")).unwrap();
    (&file)
        .write_all(prepared.reference().attempt_id.to_string().as_bytes())
        .unwrap();
    file.sync_all().unwrap();
    if phase.starts_with("spend") {
        store.fault.store(
            if phase == "spend-before" { 7 } else { 8 },
            Ordering::SeqCst,
        );
    }
    let receipt = store.spend(&prepared, &e, &s, &p).unwrap();
    ledger
        .open_approved_dispatch(prepared, receipt)
        .unwrap()
        .consume()
        .unwrap();
    if phase == "effect-after" {
        let file = std::fs::File::create_new(root.join("simulated-effect")).unwrap();
        (&file).write_all(b"one").unwrap();
        file.sync_all().unwrap();
    }
    std::process::exit(73);
}
