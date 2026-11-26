// Adversary cases for the reserve each caller of `clock::acquire_within`
// passes (contracts/service/clock.md, "Second attempt"; story:
// an-approved-write-survives-one-refused-clock-acquisition, change 3). This
// file is included into `mutation::tests::recheck` and uses its site.
//
// Every case closes the first source (`close_first_source`) and chooses the
// caller's deadline so that the time left after the first attempt lies
// between two reserves: the one the call site passes and the one a wrong
// call site would pass. The closed source refuses at once and the other
// answers, so no attempt waits for the query deadline and the time left
// after the first attempt is the deadline less the work before it. The count
// of requests to the second source then says which reserve the call site
// holds. A slow machine leaves less time, never more, so each time is 50 ms
// under the upper end of its range: a case that expects one attempt stays
// right however slow the machine is, and a case that expects two has
// `Q` - 50 ms (9.95 s) for the work before the first attempt. Every deadline
// is more than `Q`, so the work after the refusal has that much too.
use super::*;

/// The query deadline of these cases, on the test's thread.
const Q: Duration = LONG_QUERY;
/// Left after one attempt: at least `Q` and under `Q` + 1 s. A reserve of
/// 1 s refuses the second attempt and a reserve of 0 starts it.
const UNDER_ONE_SECOND: Duration = Duration::from_millis(10_950);
/// Left after one attempt: at least `Q` + 1 s and under 2 x `Q` + 1 s. A
/// reserve of 1 s starts the second attempt, and the reserve before the
/// second reading (`Q` + 1 s) refuses it.
const OVER_ONE_SECOND: Duration = Duration::from_millis(20_950);

/// A deadline that leaves `left` after a first attempt that starts and ends
/// now.
fn leaving(left: Duration) -> Instant {
    Instant::now() + left
}

/// A site whose first source is closed, under the deadline `Q`.
fn refusing_site() -> (Site, [usize; 2]) {
    let mut site = site();
    clock::set_test_query_timeout(Q);
    assert_eq!(
        clock::reserve_before_second_reading(),
        Q + clock::RESERVE_AFTER_CLOCK
    );
    close_first_source(&mut site);
    let before = asked(&site);
    (site, before)
}

/// `approvals issue` holds the reserve of 1 s: no second attempt with less
/// than `Q` + 1 s left, and one with that much.
#[test]
fn issue_holds_the_one_second_reserve() {
    let mut site = site();
    clock::set_test_query_timeout(Q);
    let approved = issuance::prepare(
        &site.paths,
        "fixture",
        &keyed_write(&site),
        Instant::now() + WAIT,
    )
    .unwrap();
    let output = site._root.path().join("proof.json");
    close_first_source(&mut site);
    let request = keyed_write(&site);
    let issue = |left: Duration| {
        let before = asked(&site);
        let until = leaving(left);
        let code = issuance::issue(
            &site.paths,
            "fixture",
            &request,
            &approved.subject_sha256,
            &output,
            until,
        )
        .map(|_| ())
        .map_err(|error| error.code);
        assert!(
            Instant::now() < until,
            "the refusal came after the deadline"
        );
        (code, asked_since(&site, before))
    };
    assert_eq!(issue(UNDER_ONE_SECOND), (Err(Code::Unavailable), [0, 1]));
    assert_eq!(issue(OVER_ONE_SECOND), (Err(Code::Unavailable), [0, 2]));
}

/// The reading of the standing commands holds the reserve of 1 s.
#[test]
fn the_standing_reading_holds_the_one_second_reserve() {
    let (site, _) = refusing_site();
    let list = |left: Duration| {
        let before = asked(&site);
        let until = leaving(left);
        let code = issuance::list_standing(&site.paths, "fixture", until)
            .map(|_| ())
            .map_err(|error| error.code);
        assert!(
            Instant::now() < until,
            "the refusal came after the deadline"
        );
        (code, asked_since(&site, before))
    };
    assert_eq!(list(UNDER_ONE_SECOND), (Err(Code::Unavailable), [0, 1]));
    assert_eq!(list(OVER_ONE_SECOND), (Err(Code::Unavailable), [0, 2]));
}

/// Recovery holds the reserve of 1 s and not the larger one: with `Q` + 1 s
/// left after the first attempt it asks again.
/// (`recovery_refused_inside_the_reserve_answers_as_today` is the other side.)
#[test]
fn recovery_asks_again_with_one_deadline_and_one_second_left() {
    let mut site = site();
    clock::set_test_query_timeout(Q);
    let candidate = prepared(&site);
    close_first_source(&mut site);
    let before = asked(&site);
    let delivery = recover(&site, OVER_ONE_SECOND);
    assert!(unavailable_at_observation(&delivery));
    assert!(still_prepared(&site, &candidate));
    assert_eq!(asked_since(&site, before), [0, 2]);
}

/// The first reading of a write holds the reserve before the second reading,
/// not the 1 s constant: with `Q` + 1 s left after the first attempt, and
/// less than 2 x `Q` + 1 s, no second attempt starts.
#[test]
fn the_first_reading_of_a_write_holds_the_larger_reserve() {
    let (mut site, before) = refusing_site();
    let (delivery, _) = write_within(&mut site, OVER_ONE_SECOND, None);
    refused_as_unavailable_before_any_attempt(&delivery);
    assert_eq!(asked_since(&site, before), [0, 1]);
    assert_eq!((count(&site, "prepare"), count(&site, "send")), (0, 0));
}

/// The keyed write of `write_within` under the deadline `until`. Both
/// sources answer the first reading; when `execute` reaches the step
/// `Admitted`, which lies between the two readings, the first source is
/// closed. The second value is how often the first source was asked in all.
fn write_refused_after_admission(site: &mut Site, until: Instant) -> (Delivery, usize) {
    let first = take_first_source(site);
    let asked_first = first.requests.clone();
    let first = Cell::new(Some(first));
    seam::set(Some(seam::Seam {
        credential: b"fictional-only".to_vec(),
        at: Box::new(move |step: seam::Step| {
            if step == seam::Step::Admitted {
                drop(first.take());
            }
        }),
    }));
    let owned = Mutex::new(lifecycle::Control::default());
    let (stopped, pool) = (AtomicBool::new(false), std::sync::Weak::new());
    let delivery = execute(
        &site.paths,
        "fixture",
        Invocation {
            request: issuance::Request {
                connection: &site.connection,
                operation: "write",
                schema: &site.schema,
                revision: "fixture-private-descriptor",
                input: r#"{"value":true}"#,
            },
            key: Some("key-1"),
            proof: None,
            until,
        },
        &mut site.child,
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
    (delivery, asked_first.load(Ordering::SeqCst))
}

/// The second reading of a write, refused with less than `left` after its
/// first attempt (the first reading and the admission came before it): the
/// write's answer and how often each source was asked by the second reading
/// alone.
fn second_reading_refused_with(left: Duration) -> (Site, Delivery, [usize; 2]) {
    let mut site = site();
    clock::set_test_query_timeout(Q);
    let before = asked(&site);
    let (delivery, first) = write_refused_after_admission(&mut site, leaving(left));
    let second = asked(&site)[1];
    // One attempt of the first reading, which both sources answered.
    (
        site,
        delivery,
        [first - before[0] - 1, second - before[1] - 1],
    )
}
/// The write was refused `unavailable` and nothing was sent.
fn refused_as_unavailable_and_not_sent(site: &Site, delivery: &Delivery) {
    let error = delivery.error.as_ref().expect("the write was not refused");
    assert_eq!(error.code, FailureCode::Owner(Code::Unavailable));
    assert!(delivery.result.is_none());
    assert_eq!(count(site, "send"), 0);
}

/// The second reading of a write holds a reserve: with less than `Q` + 1 s
/// left after its first attempt, no second attempt starts.
#[test]
fn the_second_reading_of_a_write_makes_no_second_attempt_inside_its_reserve() {
    let (site, delivery, asked) = second_reading_refused_with(UNDER_ONE_SECOND);
    refused_as_unavailable_and_not_sent(&site, &delivery);
    assert_eq!(asked, [0, 1]);
}

/// The second reading of a write holds the reserve of 1 s, not the larger
/// one of the first reading: with `Q` + 1 s left after its first attempt it
/// asks again.
#[test]
fn the_second_reading_of_a_write_asks_again_with_one_deadline_and_one_second_left() {
    let (site, delivery, asked) = second_reading_refused_with(OVER_ONE_SECOND);
    refused_as_unavailable_and_not_sent(&site, &delivery);
    assert_eq!(asked, [0, 2]);
}
