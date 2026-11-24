// Adversary cases for the second attempt (contracts/service/clock.md, "Second
// attempt"; story:an-approved-write-survives-one-refused-clock-acquisition).
// Every case sets the query deadline on the thread that acquires.
use super::*;

/// A source that answers every request with `reply`, whatever was asked. It
/// has answered once before it is returned and is then counted as unasked,
/// like `Server::fixture`: the query deadline never pays for its thread's
/// start.
fn replying(reply: Vec<u8>) -> Server {
    let server = Server::spawn(Box::new(move |_, _| Some(reply.clone())));
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    socket.send_to(&[0], &server.address).unwrap();
    socket
        .recv_from(&mut [0u8; 2048])
        .expect("the source answers its warm-up request");
    server.requests.store(0, Ordering::SeqCst);
    server
}
/// A source whose signed reply is `fixture`'s.
fn signing(fixture: fixture::Fixture) -> Server {
    Server::fixture(fixture)
}

/// Every reply the contract calls `unverified` ends the acquisition after
/// one attempt, wherever its source stands in the table. The other source
/// drops one request and would answer a second attempt.
#[test]
fn every_unverified_reply_ends_the_acquisition_after_one_attempt() {
    type Bad = fn() -> Server;
    let cases: [(&str, Bad); 8] = [
        ("a signature of another key", || answering(10, T, 1)),
        ("a replayed nonce", || {
            signing(fixture::Fixture {
                seed: 9,
                nonce_override: Some([3; 32]),
                ..Default::default()
            })
        }),
        ("a response of another type", || {
            signing(fixture::Fixture {
                seed: 9,
                msg_type: 0,
                ..Default::default()
            })
        }),
        ("a midpoint outside the delegation", || {
            signing(fixture::Fixture {
                seed: 9,
                mint: T + 1,
                maxt: T + 2,
                ..Default::default()
            })
        }),
        ("a version the source is not configured for", || {
            signing(fixture::Fixture {
                seed: 9,
                version: 0x8000_000b,
                versions: vec![0x8000_000b],
                ..Default::default()
            })
        }),
        ("an empty datagram", || replying(Vec::new())),
        ("a packet of noise", || replying(vec![0xab; 1024])),
        ("an oversized packet", || replying(vec![0; 1025])),
    ];
    std::thread::scope(|scope| {
        for (name, bad) in cases {
            for bad_first in [true, false] {
                scope.spawn(move || {
                    set_test_query_timeout(QUERY);
                    let (bad, other) = (bad(), dropping(8));
                    drop_next(&[&other]);
                    let (sources, expected) = if bad_first {
                        (
                            vec![source(&bad, 9), source(&other, 8)],
                            [SourceResult::Unverified, SourceResult::Unreachable],
                        )
                    } else {
                        (
                            vec![source(&other, 8), source(&bad, 9)],
                            [SourceResult::Unreachable, SourceResult::Unverified],
                        )
                    };
                    let refused = quorum(2, sources)
                        .acquire_within(far(), RESERVE)
                        .err()
                        .unwrap();
                    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum), "{name}");
                    assert_eq!(results(&refused.sources), expected, "{name}");
                    assert_eq!(
                        refused.next_action(),
                        NextAction::CheckConfiguration,
                        "{name}"
                    );
                    assert_eq!(requests(&[&bad, &other]), [1, 1], "{name}");
                });
            }
        }
    });
}

/// One unverified source among three, with one verified and one that dropped
/// its request: below the quorum of two, and not asked again.
#[test]
fn an_unverified_source_beside_a_verified_one_is_not_repeated() {
    set_test_query_timeout(QUERY);
    let (good, dropped, bad) = (answering(7, T, 1), dropping(8), answering(10, T, 1));
    drop_next(&[&dropped]);
    let config = quorum(
        2,
        vec![source(&good, 7), source(&dropped, 8), source(&bad, 9)],
    );
    let refused = config.acquire_within(far(), RESERVE).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [
            SourceResult::Verified,
            SourceResult::Unreachable,
            SourceResult::Unverified
        ]
    );
    assert_eq!(requests(&[&good, &dropped, &bad]), [1, 1, 1]);
}

/// `too_imprecise` is answered after one attempt (the contract's list of
/// refusals that are not repeated).
#[test]
fn too_imprecise_is_not_repeated() {
    set_test_query_timeout(QUERY);
    let (a, b) = (answering(9, T, 5), answering(10, T, 5));
    let config = quorum(2, vec![source(&a, 9), source(&b, 10)]);
    let refused = config.acquire_within(far(), RESERVE).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::TooImprecise));
    assert_eq!(requests(&[&a, &b]), [1, 1]);
}

/// The one refusal that is repeated with a counted source in it: one source
/// verified and one dropped its request.
#[test]
fn a_verified_source_below_quorum_is_asked_again() {
    set_test_query_timeout(QUERY);
    let (good, dropped) = (answering(7, T, 1), dropping(8));
    drop_next(&[&dropped]);
    let config = quorum(2, vec![source(&good, 7), source(&dropped, 8)]);
    let clock = config
        .acquire_within(far(), RESERVE)
        .expect("the second attempt answers");
    assert_eq!(
        results(&clock.observe().unwrap().sources),
        [SourceResult::Agreed, SourceResult::Agreed]
    );
    assert_eq!(requests(&[&good, &dropped]), [2, 2]);
}

/// No attempt follows the second, even where a third would answer: each
/// source drops two requests and answers the third.
#[test]
fn a_third_attempt_that_would_answer_is_not_made() {
    set_test_query_timeout(QUERY);
    let (a, b) = (dropping(7), dropping(8));
    for server in [&a, &b] {
        server.drops.store(2, Ordering::SeqCst);
    }
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let refused = config.acquire_within(far(), RESERVE).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(requests(&[&a, &b]), [2, 2]);
    // The third request is answered: the sources were ready for it.
    config.acquire().expect("the sources answer now");
    assert_eq!(requests(&[&a, &b]), [3, 3]);
}

/// After two refusals the answer names the second attempt's sources: the
/// first found both unreachable, the second found one unverified.
#[test]
fn the_refusal_after_two_attempts_names_the_second_attempts_sources() {
    set_test_query_timeout(QUERY);
    // Silent on request 0; from request 1 on it signs with key 10, and is
    // configured with key 9.
    let turned = Server::spawn(Box::new(|n, request| {
        (n > 0).then(|| {
            fixture::Fixture {
                seed: 10,
                ..Default::default()
            }
            .reply(request)
        })
    }));
    let b = dropping(8);
    drop_next(&[&b]);
    let config = quorum(2, vec![source(&turned, 9), source(&b, 8)]);
    let refused = config.acquire_within(far(), RESERVE).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Unverified, SourceResult::Verified]
    );
    assert_eq!(refused.next_action(), NextAction::CheckConfiguration);
    assert_eq!(requests(&[&turned, &b]), [2, 2]);
}

/// An `until` that has passed and a reserve no deadline holds each leave one
/// attempt, and neither panics.
#[test]
fn a_passed_deadline_and_an_unbounded_reserve_leave_one_attempt() {
    let query = Duration::from_millis(200);
    set_test_query_timeout(query);
    let (a, b) = (Server::silent(), Server::silent());
    let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let now = Instant::now();
    let passed = now.checked_sub(Duration::from_secs(1)).unwrap_or(now);
    let refused = config.acquire_within(passed, RESERVE).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(requests(&[&a, &b]), [1, 1]);
    let refused = config.acquire_within(far(), Duration::MAX).err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(requests(&[&a, &b]), [2, 2]);
    let now = Instant::now();
    let hour = now + Duration::from_secs(3600);
    assert!(!second_attempt_fits(now, hour, query, Duration::MAX));
    assert!(!second_attempt_fits(now, hour, Duration::MAX, RESERVE));
    assert!(!second_attempt_fits(
        now,
        hour,
        Duration::MAX,
        Duration::MAX
    ));
}

/// The retired table refuses through the new entry point as through
/// `acquire_explained`: at once, naming the retired format and no source.
#[test]
fn the_retired_table_refuses_at_once_through_the_new_entry_point() {
    set_test_query_timeout(QUERY);
    let key = STANDARD.encode(fixture::root_key());
    let selection: ApprovalClock = toml::from_str(&format!(
        "format='roughtime-clock/1'\naddress='127.0.0.1:2002'\npublic_key='{key}'\nmax_rate_error_ppm=10000\n"
    ))
    .unwrap();
    assert!(selection.is_retired());
    let started = Instant::now();
    let refused = selection.acquire_within(far(), RESERVE).err().unwrap();
    assert!(started.elapsed() < QUERY, "{:?}", started.elapsed());
    assert_eq!(refused.failure, Failure::InvalidConfiguration);
    assert!(refused.retired);
    assert_eq!(refused.refusal, None);
    assert!(refused.sources.is_empty());
}

/// The quorum table behind `ApprovalClock`, which every caller holds, makes
/// the second attempt too.
#[test]
fn the_selected_quorum_table_makes_the_second_attempt() {
    set_test_query_timeout(QUERY);
    let (a, b) = (dropping(7), dropping(8));
    drop_next(&[&a, &b]);
    let selection = ApprovalClock::Quorum(quorum(2, vec![source(&a, 7), source(&b, 8)]));
    assert!(selection.acquire_within(far(), RESERVE).is_ok());
    assert_eq!(requests(&[&a, &b]), [2, 2]);
}

// --- Mutants of the decision, written here and not in `clock.rs`: `twin` is
// `Configuration::acquire_within` with one condition changed. Each case runs
// the scenario of the test that pins the condition and shows that the
// unchanged twin gives the pinned count and the mutant another.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mutant {
    /// `acquire_within` as it is.
    None,
    /// The `unverified` condition is dropped.
    IgnoresUnverified,
    /// Any quorum refusal is repeated, not `below_quorum` alone.
    AnyRefusal,
    /// Two attempts may follow the first.
    ThreeAttempts,
}
fn twin(
    config: &Configuration,
    until: Instant,
    reserve: Duration,
    mutant: Mutant,
) -> std::result::Result<BoundedClock, Refused> {
    let mut refused = match config.acquire_explained() {
        Ok(clock) => return Ok(clock),
        Err(refused) => refused,
    };
    let more = if mutant == Mutant::ThreeAttempts {
        2
    } else {
        1
    };
    for _ in 0..more {
        let below = if mutant == Mutant::AnyRefusal {
            refused.refusal.is_some()
        } else {
            refused.refusal == Some(Refusal::BelowQuorum)
        };
        let clean = mutant == Mutant::IgnoresUnverified
            || refused
                .sources
                .iter()
                .all(|source| source.result != SourceResult::Unverified);
        if !(below && clean && second_attempt_fits(Instant::now(), until, query_timeout(), reserve))
        {
            break;
        }
        refused = match config.acquire_explained() {
            Ok(clock) => return Ok(clock),
            Err(refused) => refused,
        };
    }
    Err(refused)
}

/// The scenario of `an_attempt_with_an_unverified_source_is_not_repeated`.
#[test]
fn the_unverified_scenario_tells_the_mutant_that_ignores_it_apart() {
    set_test_query_timeout(QUERY);
    let asked = |mutant| {
        let (bad, silent) = (answering(8, T, 1), Server::silent());
        let config = quorum(2, vec![source(&bad, 9), source(&silent, 7)]);
        assert!(twin(&config, far(), RESERVE, mutant).is_err());
        requests(&[&bad, &silent])
    };
    assert_eq!(asked(Mutant::None), [1, 1]);
    assert_eq!(asked(Mutant::IgnoresUnverified), [2, 2]);
}

/// The scenarios of `a_split_is_not_repeated` and `too_imprecise_is_not_repeated`.
#[test]
fn the_split_and_width_scenarios_tell_the_mutant_that_repeats_any_refusal_apart() {
    set_test_query_timeout(QUERY);
    let asked = |second: (u64, u32), mutant| {
        let (a, b) = (answering(7, T, second.1), answering(8, second.0, second.1));
        let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
        assert!(twin(&config, far(), RESERVE, mutant).is_err());
        requests(&[&a, &b])
    };
    for scenario in [(T + 20, 1), (T, 5)] {
        assert_eq!(asked(scenario, Mutant::None), [1, 1]);
        assert_eq!(asked(scenario, Mutant::AnyRefusal), [2, 2]);
    }
}

/// The scenario of `sources_that_never_answer_end_after_two_attempts`.
#[test]
fn the_silent_scenario_tells_the_mutant_with_a_third_attempt_apart() {
    set_test_query_timeout(QUERY);
    let asked = |mutant| {
        let (a, b) = (Server::silent(), Server::silent());
        let config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
        assert!(twin(&config, far(), RESERVE, mutant).is_err());
        requests(&[&a, &b])
    };
    assert_eq!(asked(Mutant::None), [2, 2]);
    assert_eq!(asked(Mutant::ThreeAttempts), [3, 3]);
}
