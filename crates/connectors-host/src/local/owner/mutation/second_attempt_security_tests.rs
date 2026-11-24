//! Conformance checks of the clock's second attempt through `execute`
//! (contracts/service/clock.md, "Second attempt";
//! story:an-approved-write-survives-one-refused-clock-acquisition): a grant
//! that expired by the reading of the second attempt admits nothing, a write
//! is sent at most once whichever of its two readings takes two attempts,
//! and a reading refused after two attempts leaves what one refusal leaves.
//!
//! The site is the one of `tests::recheck` (a live child of this test
//! binary, a saved connection, a published policy, an active key and one
//! grant), with clock sources that answer by a script: per request a drop, a
//! signed time or a reply that does not verify.
use super::execution::seam;
use super::*;
use crate::local::{
    approvals, clock, clock::fixture as clock_fixture, config, keyring::custody, metadata,
    owner::lifecycle, registry, runtime, standing,
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use connectors_sdk::Secret;
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{Ed25519KeyPair, KeyPair},
};
use std::{
    collections::{BTreeSet, VecDeque},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Every wait of these tests: a bound only a hang reaches, inside the 60 s a
/// test may run.
const WAIT: Duration = Duration::from_secs(50);
/// The query deadline of the writes under test, on the test's thread.
const QUERY: Duration = Duration::from_millis(500);
/// The fixtures' signed midpoint, in Unix seconds: the time the grant is
/// signed at. It expires one hour after it.
const T: u64 = 1_789_076_475;
/// Two hours later on the same UTC day: past the grant's `not_after`.
const EXPIRED: u64 = T + 7_200;

/// What a source does with one request.
#[derive(Clone, Copy)]
enum Reply {
    /// No answer.
    Drop,
    /// The signed time, in Unix seconds.
    At(u64),
    /// A signed reply to another nonce: `unverified`.
    Forged,
}

/// One clock source and the replies to its next requests; with none left it
/// answers `At(T)`.
struct Scripted {
    server: clock_fixture::Server,
    script: Arc<Mutex<VecDeque<Reply>>>,
}
fn scripted(seed: u8) -> Scripted {
    let script = Arc::new(Mutex::new(VecDeque::new()));
    let steps = script.clone();
    let server = clock_fixture::Server::spawn(Box::new(move |_, request| {
        let signing = |midpoint| clock_fixture::Fixture {
            seed,
            midpoint,
            ..Default::default()
        };
        let step = steps.lock().unwrap().pop_front().unwrap_or(Reply::At(T));
        match step {
            Reply::Drop => None,
            Reply::At(midpoint) => Some(signing(midpoint).reply(request)),
            Reply::Forged => Some(
                clock_fixture::Fixture {
                    nonce_override: Some([1; 32]),
                    ..signing(T)
                }
                .reply(request),
            ),
        }
    }));
    Scripted { server, script }
}

struct Site {
    child: runtime::Child,
    clocks: [Scripted; 2],
    paths: Paths,
    /// Where the child records `prepare`, `send` and `destroy`.
    events: PathBuf,
    connection: String,
    schema: String,
    grant_id: String,
    _root: tempfile::TempDir,
}

/// `tests::recheck::site`, with scripted clock sources. The grant of `write`
/// for `/value` true is signed at the reading of `T` and expires an hour
/// after it.
fn site() -> Site {
    metadata::set_lifecycle_lock_wait(WAIT);
    clock::set_test_query_timeout(WAIT);
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let events = root.path().join("events");
    std::fs::create_dir(&events).unwrap();
    let grants = root.path().join("fixture.grants");
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    config::Config::initialize(&paths).unwrap();
    let clocks = [7, 8].map(scripted);
    let mut config = config::Config::load(&paths.config).unwrap();
    config.format = "connectors-local/2".into();
    config.secret_service_socket = Some(root.path().join("absent-bus"));
    config.approval_clock = Some(clock::ApprovalClock::Quorum(clock::Configuration {
        format: "roughtime-quorum/1".into(),
        quorum: 2,
        max_rate_error_ppm: 100,
        sources: clocks
            .iter()
            .zip([7, 8])
            .map(|(source, seed)| clock::Source {
                address: source.server.address.clone(),
                public_key: STANDARD.encode(clock_fixture::key(seed)),
                version: clock::RoughtimeVersion::Draft0x8000000c,
            })
            .collect(),
    }));
    let (path, sha256) = crate::local::test_executable::path_and_sha256();
    let adapter = config::Adapter {
        standing_grants: Some(grants.clone()),
        credential_revision: None,
        instance_id: "fixture".into(),
        adapter_id: "fixture".into(),
        configuration_revision: "fixture-config".into(),
        protocol: "v1alpha1".into(),
        private_protocol: Some(runtime::PrivateProtocol::V2),
        startup: Default::default(),
        restart: Default::default(),
        executable: config::Executable {
            path,
            sha256,
            args: vec![
                "--exact".into(),
                "local::runtime::process::write_tests::fixture".into(),
                "--".into(),
                "write-mode=owned".into(),
                format!("write-root={}", events.display()),
            ],
        },
        permissions: config::Permissions {
            profiles: BTreeSet::from(["fixture".into()]),
            operations: BTreeSet::from(["write".into(), "read".into()]),
            custody: Default::default(),
        },
    };
    config.adapters.insert("fixture".into(), adapter.clone());
    std::fs::write(&paths.config, toml::to_string(&config).unwrap()).unwrap();

    let child = runtime::Child::spawn_within(&adapter, WAIT).unwrap();
    let bootstrap = child.bootstrap().clone();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let schema = crate::local::owner::schema(&bootstrap, "write").unwrap();

    let now = connectors_sdk::now_ms();
    let registry = registry::Registry::new(&paths.state);
    let acquisition = registry
        .begin(&bootstrap.binding("fixture").unwrap(), now)
        .unwrap();
    let claim = registry.consume(acquisition, now).unwrap();
    let candidate = registry
        .prepare(
            &claim,
            registry::ValidatedBaseline {
                binding: None,
                identity: registry::ExternalIdentity {
                    kind: "fixture".into(),
                    subject: "owner".into(),
                },
                granted_scopes: Some(BTreeSet::new()),
                credential_expires_at_ms: None,
                collected_at_ms: now,
                valid_until_ms: now + 60_000,
            },
            12,
            now,
        )
        .unwrap();
    let receipt = custody::WrittenVersion::fixture(candidate.version());
    let stored = registry.acknowledge(candidate, receipt, now).unwrap();
    let connection = registry.publish(stored, now).unwrap();

    let mut seed = [0u8; 32];
    SystemRandom::new().fill(&mut seed).unwrap();
    let pair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let (issuer, kid) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let mut store = metadata::Metadata::update_approval_keys(&paths.state).unwrap();
    store
        .connection
        .execute(
            "INSERT INTO local_approval_issuers VALUES (?1,'fixture',?2,?3)",
            rusqlite::params![
                issuer,
                Uuid::new_v4().to_string(),
                Uuid::new_v4().to_string()
            ],
        )
        .unwrap();
    store
        .connection
        .execute(
            "INSERT INTO local_approval_keys VALUES (?1,?2,?3,?4,'candidate')",
            rusqlite::params![
                kid,
                issuer,
                URL_SAFE_NO_PAD.encode(pair.public_key().as_ref()),
                Uuid::new_v4().to_string()
            ],
        )
        .unwrap();
    store.persist().unwrap();
    drop(store);
    let mut store = metadata::Metadata::update_approval_keys(&paths.state).unwrap();
    store
        .connection
        .execute(
            "UPDATE local_approval_keys SET state='active' WHERE key_id=?1 AND state='candidate'",
            [&kid],
        )
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE local_approval_issuers SET revision=?1 WHERE issuer_id=?2",
            rusqlite::params![Uuid::new_v4().to_string(), issuer],
        )
        .unwrap();
    store.persist().unwrap();
    drop(store);
    drop(
        crate::local::approval_keys::Lease::acquire(&paths.state, "approval-issuer.lock", true)
            .unwrap(),
    );
    issuance::policy_set(
        &paths,
        "fixture",
        br#"{"operations":["write"]}"#,
        None,
        Instant::now() + WAIT,
    )
    .unwrap();

    let reading =
        standing::reading(&config.approval_clock.as_ref().unwrap().acquire().unwrap()).unwrap();
    let request = serde_json::to_vec(&serde_json::json!({
        "operation": "write",
        "target": [{"pointer": "/value", "value": true}],
        "not_after_unix_ms": reading.upper_unix_ms + 3_600_000,
        "max_per_hour": 1,
    }))
    .unwrap();
    let grant_id = {
        let lock = standing::lock(&paths.state).unwrap();
        let keys = issuance::key_store(&paths, &config, &adapter)
            .unwrap()
            .acquire_key()
            .unwrap();
        standing::grant(
            &lock,
            &grants,
            &approvals::Signer::from_seed(Secret(seed.to_vec()), kid).unwrap(),
            keys.key().unwrap(),
            "fixture",
            reading,
            &request,
        )
        .unwrap()
    };
    Site {
        child,
        clocks,
        paths,
        events,
        connection,
        schema,
        grant_id,
        _root: root,
    }
}

impl Site {
    /// Both sources answer their next requests as `replies` say, in order.
    fn script(&self, replies: &[Reply]) {
        for source in &self.clocks {
            *source.script.lock().unwrap() = replies.iter().copied().collect();
        }
    }
    /// How often each source was asked so far.
    fn asked(&self) -> [usize; 2] {
        [0, 1].map(|i| self.clocks[i].server.requests.load(Ordering::SeqCst))
    }
    fn asked_since(&self, before: [usize; 2]) -> [usize; 2] {
        let now = self.asked();
        [0, 1].map(|i| now[i] - before[i])
    }
    fn request(&self) -> issuance::Request<'_> {
        issuance::Request {
            connection: &self.connection,
            operation: "write",
            schema: &self.schema,
            revision: "fixture-private-descriptor",
            input: r#"{"value":true}"#,
        }
    }
    /// The keyed write `{"value":true}` through `execute`, as
    /// `tests::recheck::write_within` runs it, with the grant never revoked.
    fn write(&mut self) -> Delivery {
        self.write_until(Instant::now() + WAIT)
    }
    /// [`Self::write`] with the deadline `until`.
    fn write_until(&mut self, until: Instant) -> Delivery {
        seam::set(Some(seam::Seam {
            credential: b"fictional-only".to_vec(),
            at: Box::new(|_| {}),
        }));
        let owned = Mutex::new(lifecycle::Control::default());
        let (stopped, pool) = (AtomicBool::new(false), std::sync::Weak::new());
        let delivery = execute(
            &self.paths,
            "fixture",
            Invocation {
                request: issuance::Request {
                    connection: &self.connection,
                    operation: "write",
                    schema: &self.schema,
                    revision: "fixture-private-descriptor",
                    input: r#"{"value":true}"#,
                },
                key: Some("key-1"),
                proof: None,
                until,
            },
            &mut self.child,
            Control {
                lifecycle: &owned,
                epoch: 0,
                stopped: &stopped,
                pool: &pool,
            },
        );
        seam::set(None);
        let delivery = delivery.unwrap();
        delivery.validate().unwrap();
        delivery
    }
    /// How often the child recorded `event`.
    fn count(&self, event: &str) -> usize {
        std::fs::read_to_string(self.events.join(event))
            .unwrap_or_default()
            .lines()
            .count()
    }
    /// The rows of the use store: grant, state and attempt of each use.
    fn uses(&self) -> Vec<(String, String, Option<String>)> {
        let connection = rusqlite::Connection::open_with_flags(
            self.paths.state.join("standing-grants.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = connection
            .prepare("SELECT grant_id, state, attempt_id FROM grant_uses")
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
    /// Whether the ledger holds an attempt of the keyed write, looked up as
    /// recovery looks it up.
    fn recorded(&self) -> bool {
        let first = issuance::resolve(
            &self.paths,
            "fixture",
            &self.request(),
            Instant::now() + WAIT,
        )
        .unwrap();
        let candidate = candidate(&first, Some("key-1"), &Uuid::new_v4().to_string()).unwrap();
        store(&self.paths, NoClock)
            .unwrap()
            .lookup(&candidate)
            .unwrap()
            .is_some()
    }
    /// What a write left behind: its answer, what the child did, the states
    /// of the uses and whether the ledger holds an attempt.
    fn left(&self, delivery: &Delivery) -> Left {
        Left {
            code: delivery.error.as_ref().map(|error| error.code),
            classification: delivery.mutation.classification,
            attempt: delivery.mutation.attempt.is_some(),
            standing: delivery.mutation.standing.is_some(),
            prepared: self.count("prepare"),
            sent: self.count("send"),
            uses: self
                .uses()
                .into_iter()
                .map(|(_, state, attempt)| (state, attempt.is_some()))
                .collect(),
            recorded: self.recorded(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Left {
    code: Option<FailureCode>,
    classification: Classification,
    attempt: bool,
    standing: bool,
    prepared: usize,
    sent: usize,
    uses: Vec<(String, bool)>,
    recorded: bool,
}

/// I3. Every source drops the first request of the write, and the second
/// attempt reads a time past the grant's `not_after`. The write is refused
/// as a write no grant covers, with that reading: nothing is prepared or
/// sent, no use is taken and no attempt exists. With the same drop and a
/// reading inside the grant's life the write is sent
/// (`tests::recheck::a_granted_write_is_dispatched_once_...`).
#[test]
fn a_grant_expired_at_the_second_attempts_reading_admits_nothing() {
    let mut site = site();
    clock::set_test_query_timeout(QUERY);
    let before = site.asked();
    site.script(&[Reply::Drop, Reply::At(EXPIRED)]);
    let delivery = site.write();
    assert_eq!(
        site.left(&delivery),
        Left {
            code: Some(FailureCode::Approval(ApprovalCode::ApprovalRequired)),
            classification: Classification::NotAttempted,
            attempt: false,
            standing: false,
            prepared: 0,
            sent: 0,
            uses: vec![],
            recorded: false,
        }
    );
    assert_eq!(site.asked_since(before), [2, 2]);
}

/// I4 (the story's statements 13 and 14 for the second reading). The first
/// reading answers at once; every source drops the first request of the
/// second reading. The write is admitted and sent once.
#[test]
fn a_write_whose_second_reading_takes_two_attempts_is_sent_once() {
    let mut site = site();
    clock::set_test_query_timeout(QUERY);
    let before = site.asked();
    site.script(&[Reply::At(T), Reply::Drop, Reply::At(T)]);
    let delivery = site.write();
    assert_eq!(
        delivery
            .mutation
            .standing
            .as_ref()
            .map(|standing| standing.grant_id.as_str()),
        Some(site.grant_id.as_str())
    );
    assert_eq!(
        site.left(&delivery),
        Left {
            code: None,
            classification: Classification::Applied,
            attempt: true,
            standing: true,
            prepared: 1,
            sent: 1,
            uses: vec![("bound".to_owned(), true)],
            recorded: true,
        }
    );
    // One attempt of the first reading, two of the second.
    assert_eq!(site.asked_since(before), [3, 3]);
}

/// I3. The grant expires during the wait for the second reading: the write
/// was admitted at `T`, the first attempt of the second reading is dropped
/// and its second attempt reads a time past `not_after`. The write is not
/// sent, no attempt is recorded and the use is given back.
#[test]
fn a_grant_that_expires_during_the_wait_of_the_second_reading_sends_nothing() {
    let mut site = site();
    clock::set_test_query_timeout(QUERY);
    let before = site.asked();
    site.script(&[Reply::At(T), Reply::Drop, Reply::At(EXPIRED)]);
    let delivery = site.write();
    assert_eq!(
        site.left(&delivery),
        Left {
            code: Some(FailureCode::Approval(ApprovalCode::ApprovalRequired)),
            classification: Classification::NotAttempted,
            attempt: false,
            standing: false,
            prepared: 1,
            sent: 0,
            uses: vec![("released".to_owned(), false)],
            recorded: false,
        }
    );
    assert_eq!(site.asked_since(before), [3, 3]);
}

/// I4. The first reading's reserve holds one attempt of the second reading.
/// With 2.4 s to the deadline less than 1.9 s are left after the first
/// attempt, and a second needs 2 s: the query deadline and the reserve before
/// the second reading (1.5 s here, 3 s in production). One second, the
/// reserve of the other call sites, would admit it. No second attempt is
/// made and the refusal is the clock's, inside the deadline.
#[test]
fn the_first_reading_makes_no_second_attempt_inside_its_reserve() {
    let mut site = site();
    clock::set_test_query_timeout(QUERY);
    assert_eq!(
        QUERY + clock::reserve_before_second_reading(),
        Duration::from_secs(2)
    );
    let before = site.asked();
    site.script(&[Reply::Drop, Reply::Drop]);
    let started = Instant::now();
    let until = started + Duration::from_millis(2_400);
    let delivery = site.write_until(until);
    let spent = started.elapsed();
    assert_eq!(site.asked_since(before), [1, 1], "{spent:?}");
    assert_eq!(
        site.left(&delivery),
        Left {
            code: Some(FailureCode::Owner(Code::Unavailable)),
            classification: Classification::NotAttempted,
            attempt: false,
            standing: false,
            prepared: 0,
            sent: 0,
            uses: vec![],
            recorded: false,
        },
        "{spent:?}"
    );
    assert!(spent < Duration::from_millis(2_400), "{spent:?}");
}

/// I4. The second reading's reserve is one second. With 2 s to the deadline
/// less than 1.5 s are left after the first attempt of the second reading
/// (the first reading and the preflight came before it), and a second
/// attempt needs 1.5 s. A reserve of zero would admit it. No second attempt
/// is made, nothing is sent and the refusal is the clock's.
#[test]
fn the_second_reading_makes_no_second_attempt_inside_its_reserve() {
    let mut site = site();
    clock::set_test_query_timeout(QUERY);
    assert_eq!(
        QUERY + clock::RESERVE_AFTER_CLOCK,
        Duration::from_millis(1_500)
    );
    let before = site.asked();
    site.script(&[Reply::At(T), Reply::Drop, Reply::Drop]);
    let started = Instant::now();
    let delivery = site.write_until(started + Duration::from_secs(2));
    let spent = started.elapsed();
    // One attempt of each reading.
    assert_eq!(site.asked_since(before), [2, 2], "{spent:?}");
    assert_eq!(
        site.left(&delivery),
        Left {
            code: Some(FailureCode::Owner(Code::Unavailable)),
            classification: Classification::NotAttempted,
            attempt: false,
            standing: false,
            prepared: 1,
            sent: 0,
            uses: vec![("released".to_owned(), false)],
            recorded: false,
        },
        "{spent:?}"
    );
}

/// I4. A second reading refused after two attempts leaves what a second
/// reading refused after one leaves (a reply that does not verify is not
/// asked again): the same answer, nothing sent, no attempt in the ledger and
/// the use given back.
#[test]
fn a_reading_refused_after_two_attempts_leaves_what_one_refusal_leaves() {
    let refused_after = |second_reading: &[Reply]| {
        let mut site = site();
        clock::set_test_query_timeout(QUERY);
        let before = site.asked();
        let mut replies = vec![Reply::At(T)];
        replies.extend_from_slice(second_reading);
        site.script(&replies);
        let delivery = site.write();
        (site.left(&delivery), site.asked_since(before))
    };
    let (once, asked_once) = refused_after(&[Reply::Forged]);
    let (twice, asked_twice) = refused_after(&[Reply::Drop, Reply::Drop]);
    assert_eq!((asked_once, asked_twice), ([2, 2], [3, 3]));
    assert_eq!(
        once,
        Left {
            code: Some(FailureCode::Owner(Code::Unavailable)),
            classification: Classification::NotAttempted,
            attempt: false,
            standing: false,
            prepared: 1,
            sent: 0,
            uses: vec![("released".to_owned(), false)],
            recorded: false,
        }
    );
    assert_eq!(twice, once);
}
