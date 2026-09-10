use super::*;

fn config(address: SocketAddr) -> Configuration {
    Configuration {
        format: "roughtime-clock/1".into(),
        address: address.to_string(),
        public_key: STANDARD.encode(fixture::root_key()),
        max_rate_error_ppm: 10_000,
    }
}
fn stamp(monotonic: u64) -> Stamp {
    Stamp {
        monotonic,
        offset_lower: -1000,
        offset_upper: 1000,
    }
}
fn clock() -> BoundedClock {
    BoundedClock {
        sample: protocol::Sample {
            midpoint: 1_789_076_475,
            radius: 1,
        },
        started: stamp(1_000_000_000),
        received: stamp(1_100_000_000),
        rate_error_ppm: 10_000,
        configuration_sha256: "a".repeat(64),
        process: std::process::id(),
        last_read: AtomicU64::new(1_100_000_000),
    }
}
#[test]
fn authenticated_acquisition_and_wrong_key_refusal() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut config = config(socket.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let mut b = [0; 1024];
            let (n, addr) = socket.recv_from(&mut b).unwrap();
            assert_eq!(n, 1024);
            let reply = fixture::Fixture::default().reply(&b[..n]);
            socket.send_to(&reply, addr).unwrap();
        }
    });
    let clock = config.acquire().unwrap();
    let view = clock.observe().unwrap();
    assert_eq!(view.configuration_sha256, config.sha256());
    assert!((2000..=4000).contains(&(view.upper_unix_ms - view.lower_unix_ms)));
    assert!(clock.now().is_ok());
    config.public_key = STANDARD.encode([17; 32]);
    assert!(matches!(config.acquire(), Err(Failure::Unavailable)));
    server.join().unwrap();
}
#[test]
fn configuration_never_discovers_or_defaults_trust() {
    let good = config("127.0.0.1:2002".parse().unwrap());
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
        c.address = address.into();
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
        good.public_key.trim_end_matches('=').into(),
    ] {
        let mut c = good.clone();
        c.public_key = key;
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
        c.sample.midpoint = midpoint;
        assert!(c.interval_at(c.received, c.process).is_err());
    }
    let mut before = clock();
    before.sample.midpoint = midnight - 3;
    assert!(before.interval_at(before.received, before.process).is_ok());
    assert!(
        before
            .interval_at(
                stamp(before.received.monotonic + 1_000_000_000),
                before.process
            )
            .is_err()
    );
    let mut after = clock();
    after.sample.midpoint = midnight + 3;
    assert!(after.interval_at(after.received, after.process).is_ok());
}
#[test]
fn process_suspend_backward_time_and_unsafe_bounds_refuse() {
    let c = clock();
    assert!(
        c.interval_at(stamp(c.received.monotonic), c.process.wrapping_add(1))
            .is_err()
    );
    let mut suspended = stamp(c.received.monotonic + 1);
    suspended.offset_lower += 2_000_000;
    suspended.offset_upper += 2_000_000;
    assert!(c.interval_at(suspended, c.process).is_err());
    let mut changed = stamp(c.received.monotonic + 2);
    changed.offset_lower -= 2_000_000;
    changed.offset_upper -= 2_000_000;
    assert!(c.interval_at(changed, c.process).is_err());
    assert!(
        c.interval_at(stamp(c.received.monotonic), c.process)
            .is_err()
    );
    for midpoint in [0, 1, u64::MAX] {
        let mut c = clock();
        c.sample.midpoint = midpoint;
        assert!(c.interval_at(c.received, c.process).is_err());
    }
    for radius in [2, u32::MAX] {
        let mut c = clock();
        c.sample.radius = radius;
        assert!(c.interval_at(c.received, c.process).is_err());
    }
    let mut c = clock();
    c.received = stamp(c.started.monotonic + 2_000_000_000);
    assert!(c.interval_at(c.received, c.process).is_err());
}
#[test]
fn oversized_and_unavailable_peers_refuse() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let c = config(socket.local_addr().unwrap());
    let t = std::thread::spawn(move || {
        let mut b = [0; 1024];
        let (_, a) = socket.recv_from(&mut b).unwrap();
        socket.send_to(&[0; 2048], a).unwrap();
    });
    assert!(matches!(c.acquire(), Err(Failure::Unavailable)));
    t.join().unwrap();
    assert!(matches!(c.acquire(), Err(Failure::Unavailable)));
}

#[test]
#[ignore = "Explicit public Roughtime network interoperability; no physical clock guarantee"]
fn independent_live_source() {
    let c = Configuration {
        format: "roughtime-clock/1".into(),
        address: "192.36.143.134:2002".into(),
        public_key: "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=".into(),
        max_rate_error_ppm: 10_000,
    };
    let o = c.acquire().unwrap().observe().unwrap();
    println!("{}", serde_json::to_string(&o).unwrap());
    assert!(o.upper_unix_ms - o.lower_unix_ms <= 4000);
}

#[test]
fn timeout_and_excessive_signed_radius_supply_no_capability() {
    for (delay, radius) in [(Duration::from_millis(2200), 1), (Duration::ZERO, 2)] {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let c = config(socket.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut b = [0; 1024];
            let (n, address) = socket.recv_from(&mut b).unwrap();
            std::thread::sleep(delay);
            let reply = fixture::Fixture {
                radius,
                ..Default::default()
            }
            .reply(&b[..n]);
            socket.send_to(&reply, address).unwrap();
        });
        assert!(matches!(c.acquire(), Err(Failure::Unavailable)));
        server.join().unwrap();
    }
}

#[test]
fn inherited_clock_refuses_in_an_actual_fork() {
    let mut c = clock();
    c.started = Stamp::read().unwrap();
    c.received = c.started;
    c.last_read = AtomicU64::new(c.received.monotonic);
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
