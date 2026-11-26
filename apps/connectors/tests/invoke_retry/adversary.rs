//! Adversary cases for
//! story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice,
//! against the same stand-in and site as the acceptance statements.
use super::*;

/// Waits until the stand-in's log holds `count` lines that start with
/// `kind`, at most `within`.
fn wait_for(site: &Site, kind: &str, count: usize, within: Duration) {
    let until = Instant::now() + within;
    while site
        .log()
        .iter()
        .filter(|line| line.starts_with(kind))
        .count()
        < count
    {
        assert!(Instant::now() < until, "log: {:?}", site.log());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Two calls with one key and two inputs. The first takes its three tries and
/// commits; the second was queued on the instance's worker while the first
/// still retried its preflight, so the owner answers it
/// `idempotency_conflict` from the key's ledger row before any try of its
/// own: no preflight of its input is ever sent. `tries` is "the number of
/// tries made" (semantics.md, "Invoking an operation"), so it is 0, as the
/// same conflict answered by the CLI's own observation of the key already
/// is (operations.rs `replayed`).
#[test]
fn a_conflict_answered_from_the_ledger_names_no_try() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, PROVIDER_TIMEOUT],
        delay_ms: 700,
        ..Default::default()
    });
    let call = [
        "--idempotency-key",
        "shared-1",
        "--retry-not-attempted",
        "3",
    ];
    let first = site.spawn(true, &call);
    wait_for(&site, "preflight", 1, Duration::from_secs(5));
    let second = site.spawn(false, &call);
    let winner = answered(&first.wait_with_output().unwrap());
    assert_eq!(winner["mutation"]["classification"], "applied", "{winner}");
    assert_eq!(winner["mutation"]["tries"], 3, "{winner}");
    let (code, data) = refused(&second.wait_with_output().unwrap());
    // The safety half: one commit, of the first input only.
    assert_eq!(
        site.log(),
        [
            "preflight true",
            "preflight true",
            "preflight true",
            "commit true"
        ]
    );
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(data["code"], "idempotency_conflict", "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    // The second call sent nothing: no try was made.
    assert_eq!(data["mutation"]["tries"], 0, "{data}");
}

/// Ten tries of a preflight the provider answers with its own timeout at
/// once: all ten are made, none is committed.
#[test]
fn ten_instant_provider_timeouts_take_ten_tries_and_commit_nothing() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        rest: PROVIDER_TIMEOUT,
        ..Default::default()
    });
    let (code, data) = refused(&site.invoke(
        true,
        &["--idempotency-key", "ten-1", "--retry-not-attempted", "10"],
    ));
    assert_eq!(code, Some(1), "{data}");
    assert_eq!(
        data["mutation"]["classification"], "not_attempted",
        "{data}"
    );
    assert_eq!(data["mutation"]["tries"], 10, "{data}");
    assert!(data["mutation"]["attempt"].is_null(), "{data}");
    assert_eq!(site.log(), vec!["preflight true".to_owned(); 10]);
}

/// An unknown commit after two retried preflights is never sent again, not
/// by a later try and not by a same-key call with the option.
#[test]
fn an_unknown_commit_after_retries_is_sent_once_and_replayed() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, "provider_capacity"],
        commit: "unknown",
        ..Default::default()
    });
    let call = [
        "--idempotency-key",
        "unknown-r",
        "--retry-not-attempted",
        "5",
    ];
    let (code, data) = refused(&site.invoke(true, &call));
    assert_eq!(code, Some(3), "{data}");
    assert_eq!(data["mutation"]["classification"], "unknown", "{data}");
    assert_eq!(data["mutation"]["tries"], 3, "{data}");
    let sent = [
        "preflight true",
        "preflight true",
        "preflight true",
        "commit true",
    ];
    assert_eq!(site.log(), sent);
    let (code, data) = refused(&site.invoke(true, &call));
    assert_eq!(code, Some(3), "{data}");
    assert_eq!(data["mutation"]["classification"], "unknown", "{data}");
    assert_eq!(data["mutation"]["tries"], 0, "{data}");
    assert_eq!(site.log(), sent);
}

/// Two calls with one key and one input, both with the option: one commit;
/// the second is the first's replay.
#[test]
fn one_key_one_input_twice_at_once_commits_once() {
    let site = Site::new(Grant::default());
    site.script(&Script {
        preflights: vec![PROVIDER_TIMEOUT, PROVIDER_TIMEOUT],
        delay_ms: 700,
        ..Default::default()
    });
    let call = ["--idempotency-key", "same-1", "--retry-not-attempted", "3"];
    let first = site.spawn(true, &call);
    wait_for(&site, "preflight", 1, Duration::from_secs(5));
    let second = site.spawn(true, &call);
    let winner = answered(&first.wait_with_output().unwrap());
    assert_eq!(winner["mutation"]["tries"], 3, "{winner}");
    let replay = answered(&second.wait_with_output().unwrap());
    assert_eq!(replay["mutation"]["classification"], "applied", "{replay}");
    assert_eq!(replay["mutation"]["replayed"], true, "{replay}");
    assert_eq!(replay["mutation"]["tries"], 0, "{replay}");
    assert_eq!(
        site.log(),
        [
            "preflight true",
            "preflight true",
            "preflight true",
            "commit true"
        ]
    );
}

/// A per-input proof admits three tries, two of them made again, and is
/// spent once, by the commit.
#[test]
fn a_proof_admits_two_tries_made_again() {
    let site = Site::new(Grant {
        values: vec![false],
        max_per_hour: 1,
    });
    site.proof(true, "/work/proof.json");
    site.script(&Script {
        preflights: vec!["provider_capacity", PROVIDER_TIMEOUT],
        ..Default::default()
    });
    let result = answered(&site.invoke(
        true,
        &[
            "--approval-file",
            "/work/proof.json",
            "--idempotency-key",
            "proof-3",
            "--retry-not-attempted",
            "3",
        ],
    ));
    assert_eq!(result["mutation"]["classification"], "applied", "{result}");
    assert_eq!(result["mutation"]["tries"], 3, "{result}");
    assert_eq!(
        site.log(),
        [
            "preflight true",
            "preflight true",
            "preflight true",
            "commit true"
        ]
    );
}
