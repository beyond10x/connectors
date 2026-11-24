//! Conformance checks of the second attempt against its contract
//! (contracts/service/clock.md, "Second attempt";
//! docs/design/approval-clock-quorum.md, the network-attacker row;
//! story:an-approved-write-survives-one-refused-clock-acquisition).
//!
//! Each test names the invariant it holds. Every test sets the query deadline
//! to 500 ms on its own thread and acquires there; every source is a fixture
//! on 127.0.0.1.
use super::*;
use fixture::{Fixture, Server};
use std::sync::{Arc, Mutex};

/// The fixtures' signed midpoint, in Unix seconds.
const T: u64 = 1_789_076_475;
const QUERY: Duration = Duration::from_millis(500);
const RESERVE: Duration = Duration::from_millis(300);
const WIRE: u32 = 0x8000_000c;

fn signing(seed: u8) -> Fixture {
    Fixture {
        seed,
        ..Default::default()
    }
}
/// A source that answers its `n`th request (from 0) as `answer` says. Like
/// `Server::fixture` it has answered once before it is returned, and is then
/// counted as unasked, so no deadline pays for the serving thread's start.
fn serving(answer: impl Fn(usize, &[u8]) -> Option<Vec<u8>> + Send + 'static) -> Server {
    const WARM: &[u8] = b"warm";
    let server = Server::spawn(Box::new(move |n, request| {
        if request == WARM {
            Some(WARM.to_vec())
        } else {
            answer(n, request)
        }
    }));
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    socket.send_to(WARM, &server.address).unwrap();
    socket
        .recv_from(&mut [0u8; 16])
        .expect("the source answers its warm-up request");
    server.requests.store(0, Ordering::SeqCst);
    server
}
/// A source signing `T` with key `seed` that drops its next request.
fn dropping_once(seed: u8) -> Server {
    let server = Server::dropping(signing(seed));
    server.drops.store(1, Ordering::SeqCst);
    server
}
fn source(server: &Server, seed: u8) -> Source {
    Source {
        address: server.address.clone(),
        public_key: STANDARD.encode(fixture::key(seed)),
        version: RoughtimeVersion::Draft0x8000000c,
    }
}
fn quorum(quorum: u32, sources: Vec<Source>) -> Configuration {
    Configuration {
        format: FORMAT.into(),
        quorum,
        max_rate_error_ppm: 10_000,
        sources,
    }
}
fn results(reports: &[SourceReport]) -> Vec<SourceResult> {
    reports.iter().map(|report| report.result).collect()
}
fn asked(servers: &[&Server]) -> Vec<usize> {
    servers
        .iter()
        .map(|server| server.requests.load(Ordering::SeqCst))
        .collect()
}
/// An `until` no test reaches.
fn far() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn nonce_of(request: &[u8]) -> Vec<u8> {
    fixture::fields(&request[12..])[b"NONC"].clone()
}

/// I1. Each attempt has one verified source and they are two different
/// sources: added up they would reach the quorum of two. The acquisition
/// refuses, with the second attempt's lines alone.
#[test]
fn answers_of_the_two_attempts_are_never_added_up() {
    set_test_query_timeout(QUERY);
    let a = serving(|n, request| (n == 0).then(|| signing(7).reply(request)));
    let b = serving(|n, request| (n == 1).then(|| signing(8).reply(request)));
    let c = Server::silent();
    let config = quorum(2, vec![source(&a, 7), source(&b, 8), source(&c, 10)]);
    let refused = config
        .acquire_within(far(), RESERVE)
        .err()
        .expect("one verified source per attempt is below the quorum of two");
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [
            SourceResult::Unreachable,
            SourceResult::Verified,
            SourceResult::Unreachable
        ]
    );
    assert_eq!(refused.sources[0].radius_ms, None);
    assert_eq!(asked(&[&a, &b, &c]), [2, 2, 2]);
}

/// I1. Source A answers its first request only after the second was sent,
/// and the answer reaches the second attempt's socket: the signed reply to
/// request 0, sent to the peer of request 1. The reply is the source's own
/// and verifies against request 0; the second attempt refuses it, so it is
/// matched by the nonce and the request and not by the source alone.
#[test]
fn a_signed_reply_to_the_first_request_is_refused_by_the_second_attempt() {
    set_test_query_timeout(QUERY);
    let first_request = Arc::new(Mutex::new(None::<Vec<u8>>));
    let late_reply = Arc::new(Mutex::new(None::<Vec<u8>>));
    let (kept, sent) = (first_request.clone(), late_reply.clone());
    let a = serving(move |n, request| {
        if n == 0 {
            *kept.lock().unwrap() = Some(request.to_vec());
            return None;
        }
        let reply = signing(7).reply(kept.lock().unwrap().as_ref().unwrap());
        *sent.lock().unwrap() = Some(reply.clone());
        Some(reply)
    });
    let b = dropping_once(8);
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let refused = config
        .acquire_within(far(), RESERVE)
        .err()
        .expect("a reply to the first attempt's request does not serve the second");
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Unverified, SourceResult::Verified]
    );
    assert_eq!(asked(&[&a, &b]), [2, 2]);
    // The late reply is a valid answer of this source to request 0: only the
    // attempt it answers makes it unverified.
    let request = first_request.lock().unwrap().clone().unwrap();
    let reply = late_reply.lock().unwrap().clone().unwrap();
    let nonce: [u8; 32] = nonce_of(&request).try_into().unwrap();
    assert!(protocol::verify(&reply, &request, &nonce, &fixture::key(7), WIRE).is_ok());
}

/// I1, I3. The first attempt holds one verified answer at `T`; the second
/// holds two at `T + 100 s`. The clock is the second attempt's: its interval
/// lies around `T + 100 s`, and the source of the first attempt is
/// `unreachable` in it.
#[test]
fn the_reading_is_the_second_attempts_alone() {
    set_test_query_timeout(QUERY);
    const LATER: u64 = T + 100;
    let a = serving(|n, request| (n == 0).then(|| signing(7).reply(request)));
    let later = |seed: u8| {
        serving(move |n, request| {
            (n > 0).then(|| {
                Fixture {
                    midpoint: LATER,
                    ..signing(seed)
                }
                .reply(request)
            })
        })
    };
    let (b, c) = (later(8), later(10));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8), source(&c, 10)]);
    let clock = config
        .acquire_within(far(), RESERVE)
        .expect("the second attempt answers");
    let view = clock.observe().unwrap();
    assert_eq!(
        results(&view.sources),
        [
            SourceResult::Unreachable,
            SourceResult::Agreed,
            SourceResult::Agreed
        ]
    );
    assert_eq!(view.sources[0].radius_ms, None);
    let later_ms = i64::try_from(LATER * 1000).unwrap();
    // Radius 1 s and the 3 ms allowance below; above, the same plus the time
    // since the exchange started.
    assert!(view.lower_unix_ms >= later_ms - 1_003, "{view:?}");
    assert!(view.upper_unix_ms <= later_ms + 1_003 + 5_000, "{view:?}");
    assert_eq!(asked(&[&a, &b, &c]), [2, 2, 2]);
}

/// I2. Whatever the first attempt reports as `unverified` ends the
/// acquisition. The forging source answers its second request honestly and
/// the other source answers every request, so a second attempt would form a
/// clock: none is made, and no time is spent waiting.
#[test]
fn a_reply_that_does_not_verify_ends_the_acquisition() {
    set_test_query_timeout(QUERY);
    type Forgery = fn(&[u8]) -> Vec<u8>;
    let forgeries: [(&str, Forgery); 12] = [
        ("another nonce", |request| {
            Fixture {
                nonce_override: Some([1; 32]),
                ..signing(7)
            }
            .reply(request)
        }),
        ("another root key", |request| signing(11).reply(request)),
        ("a time after the delegation", |request| {
            Fixture {
                midpoint: 1_790_000_001,
                ..signing(7)
            }
            .reply(request)
        }),
        ("a time before the delegation", |request| {
            Fixture {
                midpoint: 1_788_999_999,
                ..signing(7)
            }
            .reply(request)
        }),
        ("another version", |request| {
            Fixture {
                version: 0x8000_000b,
                versions: vec![0x8000_000b],
                ..signing(7)
            }
            .reply(request)
        }),
        ("a request type", |request| {
            Fixture {
                msg_type: 0,
                ..signing(7)
            }
            .reply(request)
        }),
        ("a zero radius", |request| {
            Fixture {
                radius: 0,
                ..signing(7)
            }
            .reply(request)
        }),
        ("a cut reply", |request| {
            let mut reply = signing(7).reply(request);
            reply.truncate(reply.len() - 4);
            reply
        }),
        ("a changed byte", |request| {
            let mut reply = signing(7).reply(request);
            let middle = reply.len() / 2;
            reply[middle] ^= 1;
            reply
        }),
        ("no Roughtime message", |_| {
            b"ROUGHTIM\x08\0\0\0\x01\0\0\0junk".to_vec()
        }),
        ("an overlong datagram", |_| {
            vec![0u8; protocol::PACKET_SIZE + 1]
        }),
        ("an empty datagram", |_| Vec::new()),
    ];
    for (name, forge) in forgeries {
        let a = serving(move |n, request| {
            Some(if n == 0 {
                forge(request)
            } else {
                signing(7).reply(request)
            })
        });
        let b = serving(|_, request| Some(signing(8).reply(request)));
        let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
        let started = Instant::now();
        let refused = config
            .acquire_within(far(), RESERVE)
            .err()
            .unwrap_or_else(|| panic!("{name}: a second attempt answered"));
        let spent = started.elapsed();
        assert_eq!(refused.refusal, Some(Refusal::BelowQuorum), "{name}");
        assert_eq!(
            results(&refused.sources),
            [SourceResult::Unverified, SourceResult::Verified],
            "{name}"
        );
        assert_eq!(
            refused.next_action(),
            NextAction::CheckConfiguration,
            "{name}"
        );
        assert_eq!(asked(&[&a, &b]), [1, 1], "{name}");
        assert!(spent < QUERY, "{name}: {spent:?}");
    }
}

/// I2. A network that drops gets one more round and no third: both sources
/// would answer their third request, and the acquisition refuses after two
/// query deadlines.
#[test]
fn no_third_attempt_is_made_where_it_would_answer() {
    set_test_query_timeout(QUERY);
    let from_the_third =
        |seed: u8| serving(move |n, request| (n >= 2).then(|| signing(seed).reply(request)));
    let (a, b) = (from_the_third(7), from_the_third(8));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let started = Instant::now();
    let refused = config
        .acquire_within(far(), RESERVE)
        .err()
        .expect("two dropped rounds refuse");
    let spent = started.elapsed();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Unreachable, SourceResult::Unreachable]
    );
    assert_eq!(asked(&[&a, &b]), [2, 2]);
    assert!(spent >= 2 * QUERY, "{spent:?}");
    assert!(spent < 2 * QUERY + Duration::from_secs(1), "{spent:?}");
}

/// I2 and the contract's last list: `too_imprecise`, the retired shape and
/// an invalid configuration are answered after one attempt, also where a
/// second would answer.
#[test]
fn the_other_refusals_are_answered_after_one_attempt() {
    set_test_query_timeout(QUERY);
    // Radius 10 s on the first request (an answer 20 s wide), 1 s after it.
    let wide_once = |seed: u8| {
        serving(move |n, request| {
            Some(
                Fixture {
                    radius: if n == 0 { 10 } else { 1 },
                    ..signing(seed)
                }
                .reply(request),
            )
        })
    };
    let (a, b) = (wide_once(7), wide_once(8));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let refused = config
        .acquire_within(far(), RESERVE)
        .err()
        .expect("an answer 20 s wide is too imprecise");
    assert_eq!(refused.refusal, Some(Refusal::TooImprecise));
    assert_eq!(asked(&[&a, &b]), [1, 1]);

    let (c, d) = (serving(|_, _| None), serving(|_, _| None));
    let invalid = quorum(3, vec![source(&c, 7), source(&d, 8)]);
    let refused = invalid
        .acquire_within(far(), RESERVE)
        .err()
        .expect("a quorum over the number of sources is invalid");
    assert_eq!(
        (refused.failure, refused.refusal),
        (Failure::InvalidConfiguration, None)
    );
    let retired = ApprovalClock::Retired(Retired {
        format: RETIRED_FORMAT.into(),
        address: c.address.clone(),
        public_key: STANDARD.encode(fixture::key(7)),
        max_rate_error_ppm: 100,
    });
    let refused = retired
        .acquire_within(far(), RESERVE)
        .err()
        .expect("the retired shape is no clock");
    assert_eq!(
        (refused.failure, refused.refusal, refused.retired),
        (Failure::InvalidConfiguration, None, true)
    );
    assert_eq!(asked(&[&c, &d]), [0, 0]);
}

/// I4. The decision admits a second attempt exactly when one query deadline
/// and the reserve end at or before `until`, over 10,000 constructed cases
/// from a fixed seed, and at the ends of the range without a panic.
#[test]
fn the_decision_admits_only_what_ends_before_until() {
    let ms = Duration::from_millis;
    let base = Instant::now();
    let mut state: u64 = 0x5eed_2026_1124;
    let mut next = |bound: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % bound
    };
    let (mut admitted, mut refused) = (0, 0);
    for _ in 0..10_000 {
        let (now_ms, until_ms) = (next(10_000), next(10_000));
        let (query, reserve) = (ms(next(4_000)), ms(next(4_000)));
        let (now, until) = (base + ms(now_ms), base + ms(until_ms));
        let needed = u64::try_from((query + reserve).as_millis()).unwrap();
        let expected = until_ms >= now_ms && until_ms - now_ms >= needed;
        let fits = second_attempt_fits(now, until, query, reserve);
        assert_eq!(
            fits, expected,
            "now {now_ms} until {until_ms} {query:?} {reserve:?}"
        );
        if fits {
            assert!(now + query + reserve <= until);
            admitted += 1;
        } else {
            refused += 1;
        }
    }
    // Both sides of the decision were drawn.
    assert!(admitted > 500 && refused > 500, "{admitted} {refused}");
    let hour = base + Duration::from_secs(3600);
    assert!(!second_attempt_fits(
        base,
        hour,
        Duration::MAX,
        Duration::MAX
    ));
    assert!(!second_attempt_fits(
        base,
        hour,
        Duration::from_secs(2),
        Duration::MAX
    ));
    assert!(!second_attempt_fits(
        base,
        hour,
        Duration::MAX,
        Duration::ZERO
    ));
    assert!(!second_attempt_fits(hour, base, ms(1), Duration::ZERO));
}

/// I4. Where a second attempt was made the call returns with the reserve
/// still left before `until`.
#[test]
fn a_refused_second_attempt_returns_with_the_reserve_left() {
    set_test_query_timeout(QUERY);
    let (a, b) = (serving(|_, _| None), serving(|_, _| None));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    // Two deadlines, the reserve, and 500 ms for what an attempt does besides
    // waiting.
    let until = Instant::now() + 2 * QUERY + RESERVE + Duration::from_millis(500);
    let refused = config
        .acquire_within(until, RESERVE)
        .err()
        .expect("silent sources refuse");
    let returned = Instant::now();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(asked(&[&a, &b]), [2, 2]);
    let left = until.checked_duration_since(returned);
    assert!(left.is_some_and(|left| left >= RESERVE), "{left:?}");
}

/// I4. A reserve the deadline cannot hold gets the first attempt and no
/// second, also where the second would answer.
#[test]
fn a_reserve_the_deadline_cannot_hold_gets_one_attempt() {
    set_test_query_timeout(QUERY);
    for (name, reserve) in [
        ("the largest duration", Duration::MAX),
        ("the whole deadline", Duration::from_secs(30)),
    ] {
        let (a, b) = (dropping_once(7), dropping_once(8));
        let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
        let refused = config
            .acquire_within(far(), reserve)
            .err()
            .unwrap_or_else(|| panic!("{name}: a second attempt answered"));
        assert_eq!(refused.refusal, Some(Refusal::BelowQuorum), "{name}");
        assert_eq!(asked(&[&a, &b]), [1, 1], "{name}");
    }
}

/// I4. `until` bounds the second attempt alone. A call entered at its
/// deadline asks no source twice and ends within one query deadline of
/// `until`: what a single attempt takes, as before the second attempt
/// existed.
#[test]
fn a_call_entered_at_its_deadline_ends_within_one_query_deadline() {
    set_test_query_timeout(QUERY);
    let (a, b) = (dropping_once(7), dropping_once(8));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let until = Instant::now();
    let refused = config
        .acquire_within(until, RESERVE)
        .err()
        .expect("no second attempt answers after the deadline");
    let late = until.elapsed();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert!(asked(&[&a, &b]).iter().all(|asked| *asked <= 1));
    assert!(late < QUERY + Duration::from_millis(250), "{late:?}");
}

/// I5. `approvals clock-check` acquires through
/// `ApprovalClock::acquire_explained` and the maintenance scan through
/// `ApprovalClock::acquire`. Each asks every source once where the dropout
/// is one request long, and the scan's file names no second attempt
/// (statement 19 of the story).
#[test]
fn the_entry_points_of_clock_check_and_the_scan_make_one_attempt() {
    set_test_query_timeout(QUERY);
    for explained in [true, false] {
        let (a, b) = (dropping_once(7), dropping_once(8));
        let selected = ApprovalClock::Quorum(quorum(2, vec![source(&a, 7), source(&b, 8)]));
        if explained {
            let refused = selected
                .acquire_explained()
                .err()
                .expect("clock-check shows the dropout");
            assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
            assert_eq!(
                results(&refused.sources),
                [SourceResult::Unreachable, SourceResult::Unreachable]
            );
        } else {
            assert!(matches!(selected.acquire(), Err(Failure::Unavailable)));
        }
        assert_eq!(asked(&[&a, &b]), [1, 1]);
        // The dropout was one request long.
        assert!(selected.acquire().is_ok());
        assert_eq!(asked(&[&a, &b]), [2, 2]);
    }
    let scan = include_str!("../owner/maintenance.rs");
    assert!(scan.contains("source.acquire()"));
    assert!(!scan.contains("acquire_within"));
}

/// I6. The refusal of a second attempt holds, per source, its index, its
/// address, its result and a verified radius: no key, no nonce of either
/// attempt and no signature, in its debug text or as JSON.
#[test]
fn a_refusal_of_the_second_attempt_names_no_key_nonce_or_signature() {
    set_test_query_timeout(QUERY);
    let secrets = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    // Silent on request 0. On request 1 the first source answers under
    // another root key and the second honestly; every nonce and every reply
    // signature is kept.
    let recording = |signer: u8| {
        let kept = secrets.clone();
        serving(move |n, request| {
            let mut kept = kept.lock().unwrap();
            kept.push(nonce_of(request));
            (n > 0).then(|| {
                let reply = signing(signer).reply(request);
                let fields = fixture::fields(&reply[12..]);
                kept.push(fields[b"SIG\0"].clone());
                kept.push(fixture::fields(&fields[b"CERT"])[b"SIG\0"].clone());
                reply
            })
        })
    };
    let (a, b) = (recording(11), recording(8));
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let refused = config
        .acquire_within(far(), RESERVE)
        .err()
        .expect("another root key does not verify");
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Unverified, SourceResult::Verified]
    );
    assert_eq!(asked(&[&a, &b]), [2, 2]);

    let json = serde_json::to_value(&refused.sources).unwrap();
    for line in json.as_array().unwrap() {
        for member in line.as_object().unwrap().keys() {
            assert!(
                ["index", "address", "result", "radius_ms"].contains(&member.as_str()),
                "{member}"
            );
        }
    }
    let text = format!("{refused:?} {json}");
    let mut secrets = secrets.lock().unwrap().clone();
    // Four nonces (two attempts of two sources) and two signatures per reply.
    assert_eq!(secrets.len(), 8);
    secrets.extend([7, 8, 11].map(|seed| fixture::key(seed).to_vec()));
    for secret in secrets {
        let hex: String = secret.iter().map(|byte| format!("{byte:02x}")).collect();
        let list = format!("{secret:?}");
        for form in [
            STANDARD.encode(&secret),
            hex,
            list.trim_matches(['[', ']']).to_owned(),
        ] {
            assert!(!text.contains(&form), "{form} in {text}");
        }
    }
}
