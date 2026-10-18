//! Adversary, story:invoke-definition-reuse-credential, round 1: a source
//! connection revoked while the owner validates its copy.
//!
//! The owner holds the source's version as a dispatched read use while the
//! definition's adapter validates the copy (owner/transport.rs `reuse`). A
//! revocation only releases undispatched uses (registry/lifecycle.rs
//! `revoke_admitted`), and nothing after `release_read` looks at the source
//! again, so a copy of a source the operator revoked mid-validation would be
//! published. The fixture's request log is held so the copy's identity probe
//! waits at the fixture until the revocation is recorded.
use super::*;

const RACE_NAME: &str = "cli_journey::invoke_definition::adversary_reuse_race::adversary_reuse_a_source_revoked_while_its_copy_validates_is_not_published";

/// Waits until a connection to the fixture's port 443 is established in this
/// network namespace: the copy's identity probe, which the owner sends only
/// after it dispatched the source's read use (owner/transport.rs `reuse`).
/// The fixture's earlier exchanges closed their connections.
fn dispatched() -> Result<(), String> {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let table = fs::read_to_string("/proc/net/tcp").map_err(|error| error.to_string())?;
        let established = table.lines().skip(1).any(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            fields.len() > 3
                && fields[3] == "01"
                && (fields[1].ends_with(":01BB") || fields[2].ends_with(":01BB"))
        });
        if established {
            return Ok(());
        }
        if Instant::now() >= until {
            return Err(format!("no connection to the fixture: {table}"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "requires built debug production CLI, qualified disposable Secret Service and unprivileged user namespaces"]
fn adversary_reuse_a_source_revoked_while_its_copy_validates_is_not_published() {
    let Some(base) = std::env::var_os(INSIDE) else {
        let output = reenter(INSIDE, RACE_NAME);
        assert!(!carries_token(&output.stdout) && !carries_token(&output.stderr));
        return;
    };
    let fixture = Forge::new(Path::new(&base));
    let journey = Journey::new();
    let forge = journey.write("forge", HOST, None, &["project.get"]);
    journey.configure_main(&forge);
    let source = journey.connect_main();
    let before = journey.describe("main", &source);
    assert_eq!(before["summary"]["state"], "ready", "{before}");
    let revision = before["summary"]["revision"].as_str().unwrap().to_owned();

    let revoked = std::thread::scope(|scope| {
        // The fixture answers nothing while this is held: the copy's
        // identity probe waits here, after the source's use was dispatched.
        let held = fixture.requests.lock().unwrap();
        let run = scope.spawn(|| {
            journey.on_terminal(
                &invoke(&forge, "project.get"),
                &[(REUSE_QUESTION, b"1\n"), (QUESTION, b"n\n")],
            )
        });
        if let Err(seen) = dispatched() {
            drop(held);
            let shown = run
                .join()
                .map(|(output, shown)| {
                    format!(
                        "{} / {}",
                        String::from_utf8_lossy(&shown),
                        String::from_utf8_lossy(&output.stdout)
                    )
                })
                .unwrap_or_default();
            panic!("{seen}; the run showed: {shown}");
        }
        let revoked = journey
            .command(&[
                "connections",
                "revoke",
                "--connector",
                "forge",
                "--instance",
                "main",
                "--connection",
                &source,
                "--expected-revision",
                &revision,
            ])
            .output()
            .unwrap();
        drop(held);
        // A published copy runs the operation and never shows the second
        // question, which `on_terminal` then reports; the assertion below is
        // the finding either way.
        let _ = run.join();
        revoked
    });
    success(revoked);
    let copies = journey.listed("definition");
    assert!(
        copies.is_empty(),
        "the source was revoked while its copy validated, and the copy was published anyway: {copies:?}"
    );
}
