use super::*;
use fixture::Server;

/// The fixtures' signed midpoint, in Unix seconds.
const T: u64 = 1_789_076_475;
const DRAFT: RoughtimeVersion = RoughtimeVersion::Draft0x8000000c;

fn source(server: &Server, seed: u8) -> Source {
    Source {
        address: server.address.clone(),
        public_key: STANDARD.encode(fixture::key(seed)),
        version: DRAFT,
    }
}
fn quorum(quorum: u32, sources: Vec<Source>) -> Configuration {
    Configuration {
        format: "roughtime-quorum/1".into(),
        quorum,
        max_rate_error_ppm: 10_000,
        sources,
    }
}
/// A source signing `midpoint` and `radius` (seconds) with key `seed`.
fn answering(seed: u8, midpoint: u64, radius: u32) -> Server {
    Server::fixture(fixture::Fixture {
        seed,
        midpoint,
        radius,
        ..Default::default()
    })
}
fn results(reports: &[SourceReport]) -> Vec<SourceResult> {
    reports.iter().map(|r| r.result).collect()
}
fn stamp(monotonic: u64) -> Stamp {
    Stamp {
        monotonic,
        offset_lower: -1000,
        offset_upper: 1000,
    }
}
fn counted(index: usize, midpoint: u64, radius: u32, started: u64, received: u64) -> Counted {
    Counted {
        index,
        sample: protocol::Sample { midpoint, radius },
        started: stamp(started),
        received: stamp(received),
    }
}
fn report(index: usize) -> SourceReport {
    SourceReport {
        index,
        address: format!("127.0.0.1:{}", 2002 + index),
        result: SourceResult::Verified,
        radius_ms: Some(1000),
    }
}
/// Two identical counted sources at quorum two: the answer is the one
/// source's own interval, so the single-source arithmetic still holds exactly.
fn clock() -> BoundedClock {
    BoundedClock {
        counted: vec![
            counted(0, T, 1, 1_000_000_000, 1_100_000_000),
            counted(1, T, 1, 1_000_000_000, 1_100_000_000),
        ],
        quorum: 2,
        reports: vec![report(0), report(1)],
        rate_error_ppm: 10_000,
        configuration_sha256: "a".repeat(64),
        process: std::process::id(),
        last_read: AtomicU64::new(1_100_000_000),
    }
}
fn set_midpoint(c: &mut BoundedClock, midpoint: u64) {
    for source in &mut c.counted {
        source.sample.midpoint = midpoint;
    }
}
fn received(c: &BoundedClock) -> Stamp {
    c.counted[0].received
}

#[test]
fn authenticated_acquisition_and_wrong_key_refusal() {
    let (a, b) = (answering(7, T, 1), answering(8, T, 1));
    let mut config = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let clock = config.acquire().unwrap();
    let view = clock.observe().unwrap();
    assert_eq!(view.configuration_sha256, config.sha256());
    assert!((2000..=4000).contains(&(view.upper_unix_ms - view.lower_unix_ms)));
    assert_eq!(
        results(&view.sources),
        [SourceResult::Agreed, SourceResult::Agreed]
    );
    assert!(clock.now().is_ok());
    config.sources[1].public_key = STANDARD.encode([17; 32]);
    assert!(matches!(config.acquire(), Err(Failure::Unavailable)));
}

#[test]
fn configuration_never_discovers_or_defaults_trust() {
    let good = quorum(
        2,
        vec![
            Source {
                address: "127.0.0.1:2002".into(),
                public_key: STANDARD.encode(fixture::key(7)),
                version: DRAFT,
            },
            Source {
                address: "127.0.0.1:2003".into(),
                public_key: STANDARD.encode(fixture::key(8)),
                version: DRAFT,
            },
        ],
    );
    good.validate().unwrap();
    for address in [
        "localhost:2002",
        "0.0.0.0:2002",
        "127.0.0.1:0",
        "224.0.0.1:2002",
        "255.255.255.255:2002",
        "[ff02::1]:2002",
        "[fe80::1%1]:2002",
        "127.0.0.1:02002",
    ] {
        let mut c = good.clone();
        c.sources[0].address = address.into();
        assert_eq!(c.validate(), Err(Failure::InvalidConfiguration));
    }
    for p in [0, 10_001, u32::MAX] {
        let mut c = good.clone();
        c.max_rate_error_ppm = p;
        assert!(c.validate().is_err());
    }
    for key in [
        String::new(),
        STANDARD.encode([0; 32]),
        STANDARD.encode([1; 31]),
        STANDARD.encode([1; 33]),
        good.sources[0].public_key.trim_end_matches('=').into(),
    ] {
        let mut c = good.clone();
        c.sources[0].public_key = key;
        assert!(c.validate().is_err());
    }
    let mut c = good;
    c.format = "future".into();
    assert!(c.validate().is_err());
}

#[test]
fn elapsed_bounds_cover_both_rate_extremes_with_outward_rounding() {
    for ppm in [1, 500, 1000, 10_000] {
        for real_ms in [0u128, 1, 2, 100, 1999, 10_000, 29_000] {
            let real = real_ms * NS_PER_MS;
            for rate in [1_000_000 - u128::from(ppm), 1_000_000 + u128::from(ppm)] {
                let measured = (real * rate / 1_000_000) as u64;
                assert!(elapsed(measured, ppm, false).unwrap() <= real);
                assert!(elapsed(measured, ppm, true).unwrap() >= real);
            }
        }
    }
    assert_eq!(elapsed(0, 10_000, false).unwrap(), 0);
    assert_eq!(elapsed(0, 10_000, true).unwrap(), 2_020_203);
}

#[test]
fn interval_includes_full_network_delay_and_ages_without_sliding() {
    let c = clock();
    let start = c.interval_at(stamp(1_100_000_000), c.process).unwrap();
    assert_eq!(start.lower_unix_ms, 1_789_076_473_997);
    assert_eq!(start.upper_unix_ms, 1_789_076_476_107);
    let later = c.interval_at(stamp(2_100_000_000), c.process).unwrap();
    assert_eq!(later.lower_unix_ms, 1_789_076_474_985);
    assert_eq!(later.upper_unix_ms, 1_789_076_477_117);
    assert!(c.interval_at(stamp(30_698_000_000), c.process).is_ok());
    assert!(c.interval_at(stamp(30_698_000_001), c.process).is_err());
    assert!(c.interval_at(stamp(1_100_000_000), c.process).is_err());
}

#[test]
fn no_estimate_extrapolates_through_a_possible_utc_leap() {
    let midnight = 86_400 * 20_708;
    for midpoint in [midnight - 1, midnight, midnight + 1, midnight + 2] {
        let mut c = clock();
        set_midpoint(&mut c, midpoint);
        assert!(c.interval_at(received(&c), c.process).is_err());
    }
    let mut before = clock();
    set_midpoint(&mut before, midnight - 3);
    assert!(
        before
            .interval_at(received(&before), before.process)
            .is_ok()
    );
    assert!(
        before
            .interval_at(
                stamp(received(&before).monotonic + 1_000_000_000),
                before.process
            )
            .is_err()
    );
    let mut after = clock();
    set_midpoint(&mut after, midnight + 3);
    assert!(after.interval_at(received(&after), after.process).is_ok());
}

#[test]
fn process_suspend_backward_time_and_unsafe_bounds_refuse() {
    let c = clock();
    assert!(
        c.interval_at(stamp(received(&c).monotonic), c.process.wrapping_add(1))
            .is_err()
    );
    let mut suspended = stamp(received(&c).monotonic + 1);
    suspended.offset_lower += 2_000_000;
    suspended.offset_upper += 2_000_000;
    assert!(c.interval_at(suspended, c.process).is_err());
    let mut changed = stamp(received(&c).monotonic + 2);
    changed.offset_lower -= 2_000_000;
    changed.offset_upper -= 2_000_000;
    assert!(c.interval_at(changed, c.process).is_err());
    assert!(
        c.interval_at(stamp(received(&c).monotonic), c.process)
            .is_err()
    );
    for midpoint in [0, 1, u64::MAX] {
        let mut c = clock();
        set_midpoint(&mut c, midpoint);
        assert!(c.interval_at(received(&c), c.process).is_err());
    }
    // A 4 s radius alone is an interval over the 8,000 ms cap.
    for radius in [4, u32::MAX] {
        let mut c = clock();
        for source in &mut c.counted {
            source.sample.radius = radius;
        }
        assert!(c.interval_at(received(&c), c.process).is_err());
    }
    let mut c = clock();
    for source in &mut c.counted {
        source.received = stamp(source.started.monotonic + 6_000_000_000);
    }
    assert!(c.interval_at(received(&c), c.process).is_err());
}

#[test]
fn oversized_and_unavailable_peers_refuse() {
    let oversized = || Server::spawn(Box::new(|_, _| Some(vec![0; 2048])));
    let (a, b) = (oversized(), oversized());
    let c = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    let refused = c.acquire_explained().err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Unverified, SourceResult::Unverified]
    );
    let (a, b) = (Server::silent(), Server::silent());
    let c = quorum(2, vec![source(&a, 7), source(&b, 8)]);
    assert!(matches!(c.acquire(), Err(Failure::Unavailable)));
}

#[test]
fn timeout_and_excessive_signed_radius_supply_no_capability() {
    for (delay, radius) in [(Duration::from_millis(2200), 1), (Duration::ZERO, 4)] {
        let server = |seed: u8| {
            Server::spawn(Box::new(move |_, request| {
                std::thread::sleep(delay);
                Some(
                    fixture::Fixture {
                        seed,
                        radius,
                        ..Default::default()
                    }
                    .reply(request),
                )
            }))
        };
        let (a, b) = (server(7), server(8));
        let c = quorum(2, vec![source(&a, 7), source(&b, 8)]);
        assert!(matches!(c.acquire(), Err(Failure::Unavailable)));
    }
}

#[cfg(unix)]
#[test]
fn inherited_clock_refuses_in_an_actual_fork() {
    let mut c = clock();
    let now = Stamp::read().unwrap();
    for source in &mut c.counted {
        source.started = now;
        source.received = now;
    }
    c.last_read = AtomicU64::new(now.monotonic);
    assert!(c.now().is_ok());
    // Child calls only clock/getpid/atomic operations and _exit, never allocator,
    // test assertions, logging or inherited locks. Parent retains sole cleanup.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        let refused = c.now().is_err();
        unsafe { libc::_exit(if refused { 0 } else { 1 }) };
    }
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    assert!(libc::WIFEXITED(status));
    assert_eq!(libc::WEXITSTATUS(status), 0);
    assert!(c.now().is_ok());
}

// --- The quorum (docs/design/approval-clock-quorum.md, "Conformance scenarios").

#[test]
fn quorum_answer_is_hull_of_quorum_coverage() {
    // The design's example table, quorum 2, intervals in ms from a common origin
    // (an origin far from any UTC midnight); an `Err` is a refusal.
    type Row<'a> = (
        &'a [(u128, u128)],
        std::result::Result<(u128, u128), Combined>,
    );
    let origin: u128 = 1_789_076_000_000;
    let rows: [Row; 6] = [
        (&[(0, 2000), (500, 2500), (9000, 11000)], Ok((500, 2000))),
        (&[(0, 2000), (1500, 3500), (3000, 5000)], Ok((1500, 3500))),
        (
            &[(0, 2000), (5000, 7000), (10000, 12000)],
            Err(Combined::Split),
        ),
        (
            &[(0, 6000), (5000, 11000), (10000, 16000)],
            Ok((5000, 11000)),
        ),
        (
            &[(0, 7000), (6000, 13000), (12000, 19000)],
            Ok((6000, 13000)),
        ),
        (
            &[(0, 4000), (3000, 7000), (8000, 12000), (11000, 15000)],
            Err(Combined::TooImprecise),
        ),
    ];
    for (intervals, expected) in rows {
        let ns: Vec<(u128, u128)> = intervals
            .iter()
            .map(|(l, u)| ((origin + l) * NS_PER_MS, (origin + u) * NS_PER_MS))
            .collect();
        assert_eq!(
            quorum_answer(&ns, 2, MAX_WIDTH_MS),
            expected.map(|(l, u)| (origin + l, origin + u)),
            "{intervals:?}"
        );
    }
    // Rounded outward: floor the lower bound, ceil the upper bound.
    let ns = [(1_500_001, 3_999_999), (1_000_000, 3_000_001)];
    assert_eq!(quorum_answer(&ns, 2, MAX_WIDTH_MS), Ok((1, 4)));
    // Closed intervals: touching at one instant is coverage.
    let ns = [(0, 5 * NS_PER_MS), (5 * NS_PER_MS, 9 * NS_PER_MS)];
    assert_eq!(quorum_answer(&ns, 2, MAX_WIDTH_MS), Ok((5, 5)));
}

#[test]
fn quorum_contains_truth_with_one_disjoint_liar() {
    let (a, b, liar) = (
        answering(7, T, 1),
        answering(8, T, 1),
        answering(9, T + 20, 1),
    );
    let config = quorum(2, vec![source(&a, 7), source(&liar, 9), source(&b, 8)]);
    let view = config.acquire().unwrap().observe().unwrap();
    let truth = i64::try_from(T * 1000).unwrap();
    assert!(
        view.lower_unix_ms <= truth && truth <= view.upper_unix_ms,
        "{view:?}"
    );
    assert_eq!(
        results(&view.sources),
        [
            SourceResult::Agreed,
            SourceResult::Outlier,
            SourceResult::Agreed
        ]
    );
    assert_eq!(
        view.sources.iter().map(|s| s.index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(view.sources[1].address, liar.address);
    assert_eq!(view.sources[1].radius_ms, Some(1000));
}

#[test]
fn quorum_overlapping_liar_cannot_exclude_truth() {
    // The liar's [T + 1 s, T + 3 s] meets both honest [T - 1 s, T + 1 s]
    // intervals near T + 1 s without containing T: their intersection excludes
    // the truth, the hull of the doubly covered instants keeps it.
    let (a, b, liar) = (
        answering(7, T, 1),
        answering(8, T, 1),
        answering(9, T + 2, 1),
    );
    let config = quorum(2, vec![source(&a, 7), source(&b, 8), source(&liar, 9)]);
    let view = config.acquire().unwrap().observe().unwrap();
    let truth = i64::try_from(T * 1000).unwrap();
    assert!(
        view.lower_unix_ms <= truth && truth <= view.upper_unix_ms,
        "{view:?}"
    );
    assert_eq!(
        results(&view.sources),
        [SourceResult::Agreed; 3],
        "an overlapping liar is undetectable and harmless"
    );
}

#[test]
fn quorum_below_quorum_names_each_source() {
    let good = answering(7, T, 1);
    let silent = Server::silent();
    let wrong_key = answering(8, T, 1);
    // A 5 s radius, an interval wider than the cap: it still counts, so the
    // quorum is 3 for two verified sources to fall short of it.
    let wide = answering(10, T, 5);
    let mut sources = vec![
        source(&good, 7),
        source(&silent, 11),
        source(&wrong_key, 9),
        source(&wide, 10),
    ];
    let refused = quorum(3, sources.clone())
        .acquire_explained()
        .err()
        .unwrap();
    assert_eq!(refused.failure, Failure::Unavailable);
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [
            SourceResult::Verified,
            SourceResult::Unreachable,
            SourceResult::Unverified,
            SourceResult::Verified
        ]
    );
    assert_eq!(
        refused
            .sources
            .iter()
            .map(|s| s.radius_ms)
            .collect::<Vec<_>>(),
        [Some(1000), None, None, Some(5000)]
    );
    for (i, line) in refused.sources.iter().enumerate() {
        assert_eq!(
            (line.index, line.address.as_str()),
            (i, sources[i].address.as_str())
        );
    }
    // Next action: an unverified source names check_configuration, else a
    // retry; a wide verified source never names choose_clock_source.
    assert_eq!(refused.next_action(), NextAction::CheckConfiguration);
    sources.remove(2);
    let refused = quorum(3, sources.clone())
        .acquire_explained()
        .err()
        .unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [
            SourceResult::Verified,
            SourceResult::Unreachable,
            SourceResult::Verified
        ]
    );
    assert_eq!(refused.next_action(), NextAction::RetryExplicitly);
    sources.remove(2);
    let refused = quorum(2, sources).acquire_explained().err().unwrap();
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Verified, SourceResult::Unreachable]
    );
    assert_eq!(refused.next_action(), NextAction::RetryExplicitly);
}

#[test]
fn quorum_split_refuses() {
    let (a, b) = (answering(7, T, 1), answering(8, T + 20, 1));
    let refused = quorum(2, vec![source(&a, 7), source(&b, 8)])
        .acquire_explained()
        .err()
        .unwrap();
    assert_eq!(refused.refusal, Some(Refusal::Split));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Verified, SourceResult::Verified]
    );
    assert_eq!(refused.next_action(), NextAction::ChooseClockSource);
}

#[test]
fn quorum_combined_width_over_cap_refuses() {
    // The design's last row: four 4 s intervals starting at 0, 3, 8 and 11 s.
    let servers = [
        answering(7, T + 2, 2),
        answering(8, T + 5, 2),
        answering(9, T + 10, 2),
        answering(10, T + 13, 2),
    ];
    let sources = servers
        .iter()
        .zip(7..)
        .map(|(server, seed)| source(server, seed))
        .collect();
    let refused = quorum(2, sources).acquire_explained().err().unwrap();
    assert_eq!(refused.refusal, Some(Refusal::TooImprecise));
    assert_eq!(results(&refused.sources), [SourceResult::Verified; 4]);
    assert_eq!(refused.next_action(), NextAction::ChooseClockSource);
}

#[test]
fn quorum_wide_source_counts_and_only_the_answer_is_capped() {
    // The live pair of 2026-10-14: a 5 s signed radius (an interval of about
    // 10 s, wider than the 8,000 ms cap) and a 3 s one. The wide source counts,
    // and their intersection, about 6 s, is the answer.
    let (wide, narrow) = (answering(7, T, 5), answering(8, T, 3));
    let config = quorum(2, vec![source(&wide, 7), source(&narrow, 8)]);
    let view = config.acquire().unwrap().observe().unwrap();
    let width = view.upper_unix_ms - view.lower_unix_ms;
    assert!((6000..=8000).contains(&width), "{view:?}");
    assert!(view.lower_unix_ms <= (T * 1000) as i64 && view.upper_unix_ms >= (T * 1000) as i64);
    assert_eq!(
        results(&view.sources),
        [SourceResult::Agreed, SourceResult::Agreed]
    );
    assert_eq!(
        view.sources.iter().map(|s| s.radius_ms).collect::<Vec<_>>(),
        [Some(5000), Some(3000)]
    );
    // Only the answer is capped: two wide sources count, and their combined
    // answer, about 10 s, refuses too_imprecise with both lines verified.
    let (a, b) = (answering(9, T, 5), answering(10, T, 5));
    let refused = quorum(2, vec![source(&a, 9), source(&b, 10)])
        .acquire_explained()
        .err()
        .unwrap();
    assert_eq!(refused.failure, Failure::Unavailable);
    assert_eq!(refused.refusal, Some(Refusal::TooImprecise));
    assert_eq!(
        results(&refused.sources),
        [SourceResult::Verified, SourceResult::Verified]
    );
    assert_eq!(refused.next_action(), NextAction::ChooseClockSource);
}

#[test]
fn quorum_width_cap_boundary() {
    let ms = |l: u128, u: u128| (l * NS_PER_MS, u * NS_PER_MS);
    let base = 1_789_076_000_000;
    assert_eq!(MAX_WIDTH_MS, 8000);
    assert_eq!(
        quorum_answer(
            &[ms(base, base + 8000), ms(base, base + 8000)],
            2,
            MAX_WIDTH_MS
        ),
        Ok((base, base + 8000))
    );
    assert_eq!(
        quorum_answer(
            &[ms(base, base + 8001), ms(base, base + 8001)],
            2,
            MAX_WIDTH_MS
        ),
        Err(Combined::TooImprecise)
    );
    // One nanosecond past 8,000 ms rounds outward to 8,001 ms.
    let (l, u) = ms(base, base + 8000);
    assert_eq!(
        quorum_answer(&[(l, u + 1), (l, u + 1)], 2, MAX_WIDTH_MS),
        Err(Combined::TooImprecise)
    );
    // The same cap applies to the clock's own readings.
    let mut c = clock();
    for source in &mut c.counted {
        source.sample.radius = 3;
    }
    let width = |c: &BoundedClock, at: u64| {
        c.interval_at(stamp(at), c.process)
            .map(|i| i.upper_unix_ms - i.lower_unix_ms)
    };
    assert!(width(&c, 1_100_000_000).is_ok_and(|w| w <= 8000));
}

#[test]
fn quorum_shared_deadline() {
    // Two silent sources: queried in sequence they would cost 4 s; in
    // parallel under one deadline the whole acquisition costs 2 s.
    let (a, b, silent_1, silent_2) = (
        answering(7, T, 1),
        answering(8, T, 1),
        Server::silent(),
        Server::silent(),
    );
    let config = quorum(
        2,
        vec![
            source(&silent_1, 9),
            source(&a, 7),
            source(&silent_2, 10),
            source(&b, 8),
        ],
    );
    let started = Instant::now();
    let clock = config.acquire().unwrap();
    let spent = started.elapsed();
    assert!(spent >= Duration::from_millis(1900), "{spent:?}");
    assert!(spent < Duration::from_millis(3000), "{spent:?}");
    let view = clock.observe().unwrap();
    assert_eq!(
        results(&view.sources),
        [
            SourceResult::Unreachable,
            SourceResult::Agreed,
            SourceResult::Unreachable,
            SourceResult::Agreed
        ]
    );
    for server in [&a, &b, &silent_1, &silent_2] {
        assert_eq!(server.requests.load(Ordering::SeqCst), 1, "no retry");
    }
}

#[test]
fn quorum_version_mismatch_is_unverified() {
    // Each source is asked at its configured version only, the draft version
    // 0x8000000c, and the reply must sign exactly that VER and list it in VERS.
    const OTHER: u32 = 0x8000000b;
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = |fixture: fixture::Fixture| {
        let seen = seen.clone();
        Server::spawn(Box::new(move |_, request| {
            let fields = fixture::fields(&request[12..]);
            seen.lock().unwrap().push(fields[b"VER\0"].clone());
            Some(fixture.reply(request))
        }))
    };
    let signed_other_ver = recorder(fixture::Fixture {
        seed: 7,
        version: OTHER,
        versions: vec![OTHER, 0x8000000c],
        ..Default::default()
    });
    let vers_without_it = recorder(fixture::Fixture {
        seed: 8,
        versions: vec![OTHER],
        ..Default::default()
    });
    let at_draft = |seed| fixture::Fixture {
        seed,
        ..Default::default()
    };
    let (good_1, good_2) = (recorder(at_draft(9)), recorder(at_draft(10)));
    let refused = quorum(
        3,
        vec![
            source(&signed_other_ver, 7),
            source(&vers_without_it, 8),
            source(&good_1, 9),
            source(&good_2, 10),
        ],
    )
    .acquire_explained()
    .err()
    .unwrap();
    assert_eq!(refused.refusal, Some(Refusal::BelowQuorum));
    assert_eq!(
        results(&refused.sources),
        [
            SourceResult::Unverified,
            SourceResult::Unverified,
            SourceResult::Verified,
            SourceResult::Verified
        ]
    );
    assert_eq!(refused.next_action(), NextAction::CheckConfiguration);
    for server in [&signed_other_ver, &vers_without_it, &good_1, &good_2] {
        assert_eq!(
            server.requests.load(Ordering::SeqCst),
            1,
            "no second request"
        );
    }
    assert_eq!(
        *seen.lock().unwrap(),
        vec![0x8000000cu32.to_le_bytes().to_vec(); 4]
    );
}

#[test]
fn quorum_configuration_refusals() {
    let at = |port: u16, seed: u8| Source {
        address: format!("127.0.0.1:{port}"),
        public_key: STANDARD.encode(fixture::key(seed)),
        version: DRAFT,
    };
    let sources =
        |n: u8| -> Vec<Source> { (0..n).map(|i| at(2002 + u16::from(i), 20 + i)).collect() };
    quorum(2, sources(2)).validate().unwrap();
    quorum(8, sources(8)).validate().unwrap();
    let mut duplicate_address: Configuration = quorum(2, sources(3));
    duplicate_address.sources[2].address = duplicate_address.sources[0].address.clone();
    let mut duplicate_key = quorum(2, sources(3));
    duplicate_key.sources[2].public_key = duplicate_key.sources[1].public_key.clone();
    for (case, config) in [
        ("one source", quorum(2, sources(1))),
        ("nine sources", quorum(2, sources(9))),
        ("quorum 1", quorum(1, sources(3))),
        ("quorum 9", quorum(9, sources(8))),
        ("quorum above the sources", quorum(4, sources(3))),
        ("duplicate address", duplicate_address),
        ("duplicate key", duplicate_key),
    ] {
        assert_eq!(
            config.validate(),
            Err(Failure::InvalidConfiguration),
            "{case}"
        );
        assert_eq!(
            config
                .acquire_explained()
                .err()
                .map(|r| (r.failure, r.refusal)),
            Some((Failure::InvalidConfiguration, None)),
            "{case}"
        );
    }
    // A source names its version: no default, no other spelling.
    let table = |version: &str| {
        format!(
            "format='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=100\n[[sources]]\naddress='127.0.0.1:2002'\npublic_key='{}'\n{version}[[sources]]\naddress='127.0.0.1:2003'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n",
            STANDARD.encode(fixture::key(7)),
            STANDARD.encode(fixture::key(8)),
        )
    };
    let parsed: ApprovalClock = toml::from_str(&table("version='roughtime-0x8000000c'\n")).unwrap();
    parsed.validate().unwrap();
    // roughtime-1 (RFC 10049's 0x00000001) has no verifier and no recorded
    // reply, so it is refused as an unknown version.
    for version in [
        "",
        "version='roughtime-1'\n",
        "version='roughtime-0x8000000b'\n",
        "version=1\n",
    ] {
        assert!(
            toml::from_str::<ApprovalClock>(&table(version)).is_err(),
            "{version:?}"
        );
    }
    // The digest covers the whole table, source order included.
    let mut reordered = quorum(2, sources(3));
    let original = reordered.sha256();
    reordered.sources.swap(0, 1);
    assert_ne!(reordered.sha256(), original);
}

#[test]
fn quorum_counted_set_is_fixed() {
    // Source 0 was asked 20 s before the others; quorum 2 of 3. Once its own
    // 30 s window has passed the clock refuses, although sources 1 and 2 alone
    // would still answer: no reading drops a counted source.
    let mut c = clock();
    c.counted = vec![
        counted(0, T - 20, 1, 1_000_000_000, 1_100_000_000),
        counted(1, T, 1, 21_000_000_000, 21_100_000_000),
        counted(2, T, 1, 21_000_000_000, 21_100_000_000),
    ];
    c.reports = vec![report(0), report(1), report(2)];
    c.last_read = AtomicU64::new(21_100_000_000);
    assert!(c.interval_at(stamp(21_100_000_000), c.process).is_ok());
    assert!(c.interval_at(stamp(30_698_000_000), c.process).is_ok());
    assert!(c.interval_at(stamp(30_698_000_001), c.process).is_err());
    assert!(c.interval_at(stamp(40_000_000_000), c.process).is_err());
    let mut rest = clock();
    rest.counted = vec![
        counted(1, T, 1, 21_000_000_000, 21_100_000_000),
        counted(2, T, 1, 21_000_000_000, 21_100_000_000),
    ];
    rest.last_read = AtomicU64::new(21_100_000_000);
    assert!(
        rest.interval_at(stamp(40_000_000_000), rest.process)
            .is_ok()
    );
}

#[test]
fn legacy_single_source_format_refuses_only_the_clock() {
    let retired = |key: &str| {
        format!(
            "format='roughtime-clock/1'\naddress='127.0.0.1:2002'\npublic_key='{key}'\nmax_rate_error_ppm=10000\n"
        )
    };
    let selection: ApprovalClock =
        toml::from_str(&retired(&STANDARD.encode(fixture::root_key()))).unwrap();
    // Parsed and admitted by the configuration, so reads that never use the
    // clock keep working; never read as a one-source quorum.
    selection.validate().unwrap();
    assert!(matches!(selection, ApprovalClock::Retired(_)));
    assert!(selection.is_retired());
    // Approval issuance, execution and recovery refuse it as invalid
    // configuration naming the retired format.
    let error = crate::local::owner::Error::clock(&selection, Failure::InvalidConfiguration);
    assert_eq!(error.code, crate::local::owner::Code::InvalidConfiguration);
    assert!(error.clock_format_retired);
    assert_eq!(
        selection.configuration().err(),
        Some(Failure::InvalidConfiguration)
    );
    assert!(matches!(
        selection.acquire(),
        Err(Failure::InvalidConfiguration)
    ));
    let refused = selection.acquire_explained().err().unwrap();
    assert_eq!(refused.failure, Failure::InvalidConfiguration);
    assert!(refused.retired);
    assert_eq!(refused.refusal, None);
    assert!(refused.sources.is_empty());
    // The retired shape with a malformed key is still only the clock's refusal.
    let malformed: ApprovalClock = toml::from_str(&retired("not-a-key")).unwrap();
    malformed.validate().unwrap();
    // Any other format string stays a whole-configuration refusal.
    let other: ApprovalClock =
        toml::from_str(&retired("k").replace("roughtime-clock/1", "roughtime-clock/2")).unwrap();
    assert!(other.validate().is_err());

    // The whole configuration loads; only its clock refuses.
    #[cfg(unix)]
    {
        use crate::local::config::{Config, Paths};
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir_in(std::env::var_os("TMPDIR").unwrap()).unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let paths = Paths {
            config: root.path().join("config/config.toml"),
            state: root.path().join("state"),
        };
        Config::initialize(&paths).unwrap();
        let mut text = std::fs::read_to_string(&paths.config).unwrap();
        text.push_str("\n[approval_clock]\n");
        text.push_str(&retired(&STANDARD.encode(fixture::root_key())));
        std::fs::write(&paths.config, &text).unwrap();
        let config = Config::load(&paths.config).unwrap();
        let clock = config.approval_clock.as_ref().unwrap();
        assert!(matches!(
            clock.acquire(),
            Err(Failure::InvalidConfiguration)
        ));
        assert_eq!(
            clock.configuration().err(),
            Some(Failure::InvalidConfiguration)
        );
        std::fs::write(
            &paths.config,
            text.replace("roughtime-clock/1", "roughtime-clock/9"),
        )
        .unwrap();
        assert!(Config::load(&paths.config).is_err());
    }
}

#[test]
#[ignore = "Explicit public Roughtime network interoperability; no physical clock guarantee"]
fn independent_live_quorum() {
    // docs/design/approval-clock-quorum.md, "Sources".
    let source = |address: &str, public_key: &str, version| Source {
        address: address.into(),
        public_key: public_key.into(),
        version,
    };
    let c = Configuration {
        format: "roughtime-quorum/1".into(),
        quorum: 2,
        max_rate_error_ppm: 10_000,
        sources: vec![
            source(
                "162.159.200.123:2003",
                "0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg=",
                RoughtimeVersion::Draft0x8000000c,
            ),
            source(
                "51.81.57.55:2002",
                "AW5uAoTSTDfG5NfY1bTh08GUnOqlRb+HVhbJ3ODJvsE=",
                RoughtimeVersion::Draft0x8000000c,
            ),
            source(
                "192.36.143.134:2002",
                "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=",
                RoughtimeVersion::Draft0x8000000c,
            ),
            source(
                "148.113.255.71:2002",
                "iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA=",
                RoughtimeVersion::Draft0x8000000c,
            ),
        ],
    };
    match c.acquire_explained() {
        Ok(clock) => {
            let o = clock.observe().unwrap();
            println!("{}", serde_json::to_string(&o).unwrap());
            assert!(o.upper_unix_ms - o.lower_unix_ms <= 8000);
        }
        Err(refused) => panic!("{refused:?}"),
    }
}

// --- Adversary cases (design "Threat model", "Quorum rule" step 1; contract
// "Configuration and trust").

/// A source signing `midpoint` and `radius` with a delegation valid for any time.
fn answering_any_time(seed: u8, midpoint: u64, radius: u32) -> Server {
    Server::fixture(fixture::Fixture {
        seed,
        midpoint,
        radius,
        mint: 0,
        maxt: u64::MAX,
        ..Default::default()
    })
}

#[test]
fn adversary_liar_on_another_utc_day_is_only_an_outlier() {
    // Threat model: one lying source among honest ones "cannot move the answer
    // off true time"; a disjoint liar "shows as outlier". A liar a day behind
    // must not take the whole clock down.
    let (a, b, liar) = (
        answering(7, T, 1),
        answering(8, T, 1),
        answering_any_time(9, T - 86_400, 1),
    );
    let config = quorum(2, vec![source(&a, 7), source(&liar, 9), source(&b, 8)]);
    let view = config
        .acquire_explained()
        .expect("two honest sources form the quorum")
        .observe()
        .unwrap();
    let truth = i64::try_from(T * 1000).unwrap();
    assert!(view.lower_unix_ms <= truth && truth <= view.upper_unix_ms);
    assert_eq!(
        results(&view.sources),
        [
            SourceResult::Agreed,
            SourceResult::Outlier,
            SourceResult::Agreed
        ]
    );
}

#[test]
fn adversary_liar_signing_an_early_midpoint_is_only_an_outlier() {
    // A liar's signed midpoint below its radius must not refuse the whole
    // acquisition through interval underflow.
    let (a, b, liar) = (
        answering(7, T, 1),
        answering(8, T, 1),
        answering_any_time(9, 0, 1),
    );
    let config = quorum(2, vec![source(&a, 7), source(&liar, 9), source(&b, 8)]);
    let view = config
        .acquire_explained()
        .expect("two honest sources form the quorum")
        .observe()
        .unwrap();
    assert_eq!(
        results(&view.sources),
        [
            SourceResult::Agreed,
            SourceResult::Outlier,
            SourceResult::Agreed
        ]
    );
}

#[test]
fn adversary_wide_honest_source_counts_whatever_its_radius() {
    // Quorum rule step 1: a source counts "whatever its own radius or interval
    // width". An honest source with a radius of a few days contains T and must
    // count; the answer is the narrow sources' interval.
    let (wide, a, b) = (
        answering(7, T, 200_000),
        answering(8, T, 1),
        answering(9, T, 1),
    );
    let config = quorum(2, vec![source(&wide, 7), source(&a, 8), source(&b, 9)]);
    let view = config
        .acquire_explained()
        .expect("a wide honest source counts")
        .observe()
        .unwrap();
    let truth = i64::try_from(T * 1000).unwrap();
    assert!(view.lower_unix_ms <= truth && truth <= view.upper_unix_ms);
    assert_eq!(results(&view.sources), [SourceResult::Agreed; 3]);
}

#[test]
fn adversary_ipv4_mapped_address_is_the_same_address() {
    // "No two sources share an address": the IPv4-mapped IPv6 spelling of an
    // address is that address.
    let config = quorum(
        2,
        vec![
            Source {
                address: "127.0.0.1:2002".into(),
                public_key: STANDARD.encode(fixture::key(7)),
                version: DRAFT,
            },
            Source {
                address: "[::ffff:127.0.0.1]:2002".into(),
                public_key: STANDARD.encode(fixture::key(8)),
                version: DRAFT,
            },
        ],
    );
    assert_eq!(config.validate(), Err(Failure::InvalidConfiguration));
}
