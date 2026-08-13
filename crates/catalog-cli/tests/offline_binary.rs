//! **The offline guarantee, read literally: run the shipped binary with no network at all.**
//!
//! `crates/catalog-build/tests/main/no_network.rs` proves the same invariant two other ways — a
//! seam counter armed to refuse, and a source audit that keeps the counter honest. This is the
//! third and bluntest form, and it lives here because only the crate that declares a `[[bin]]` gets
//! `CARGO_BIN_EXE_*`.
//!
//! Skipped, loudly, where unprivileged network namespaces are unavailable.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A complete hand-authored provider definition the real loader accepts.
fn definition(id: &str) -> String {
    format!(
        r#"id = "{id}"
vendor = "{id} Inc."
base_url = "https://api.{id}.example"
description = "A hand-authored fixture connector."

[[operations]]
id = "{id}-thing-get"
method = "GET"
direction = "read"
path = "/v1/things/{{thing_id}}"
description = "Fetch one thing."
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[[operations.params.path]]
name = "thing_id"
description = "The thing to fetch."
required = true
schema = {{ type = "integer" }}
"#
    )
}

/// A throwaway tree under the build directory, removed when it drops.
///
/// Not `env::temp_dir()`, deliberately: that is a bounded tmpfs every concurrent agent on the
/// machine writes to, and filling it turns a fixture write into a failure that reads like a
/// regression in the code under test. `current_exe()` follows `CARGO_TARGET_DIR`, is per-worktree,
/// and is already git-ignored — the same rule `catalog-build`'s own fixtures follow.
struct Fixture(PathBuf);

impl Fixture {
    fn new(provider: &str) -> Self {
        let root = std::env::current_exe()
            .expect("the test binary knows its own path")
            .parent()
            .and_then(Path::parent)
            .expect("the test binary sits in <build>/<profile>/deps")
            .join("integration-fixtures")
            .join(format!(
                "offline-binary-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |since| since.as_nanos())
            ));
        fs::create_dir_all(root.join("providers")).expect("create the fixture");
        fs::write(
            root.join("providers").join(format!("{provider}.toml")),
            definition(provider),
        )
        .expect("write the fixture provider");
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Best effort, and deliberately not asserted: a `Drop` that panics while the thread is
        // already unwinding from a failed assertion aborts the process and buries the real failure.
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn build_succeeds_with_networking_unavailable() {
    let Some((program, args)) = network_namespace_sandbox() else {
        eprintln!(
            "skipping: `unshare --user --map-root-user --net` is unavailable on this host, so \
             networking cannot be removed from the child; the seam-counter and source-audit tests \
             in `catalog-build` still cover the invariant"
        );
        return;
    };

    let fixture = Fixture::new("acme");
    let output = Command::new(program)
        .args(args)
        .arg(env!("CARGO_BIN_EXE_catalog"))
        .arg("build")
        .arg("--root")
        .arg(&fixture.0)
        .output()
        .expect("run the binary under a network namespace");

    assert!(
        output.status.success(),
        "build failed with networking unavailable:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        fixture.0.join("catalog").join("acme.catalog.json").exists(),
        "the offline build wrote no canonical document"
    );

    let check = Command::new(program)
        .args(args)
        .arg(env!("CARGO_BIN_EXE_catalog"))
        .arg("check")
        .arg("--root")
        .arg(&fixture.0)
        .output()
        .expect("run check under a network namespace");
    assert!(
        check.status.success(),
        "check failed with networking unavailable:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&check.stdout),
        "1 provider, 4 artifacts verified\n"
    );
}

/// `Some((program, args))` if this host can drop the child into an empty network namespace.
fn network_namespace_sandbox() -> Option<(&'static str, &'static [&'static str])> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let program = "unshare";
    let args: &[&str] = &["--user", "--map-root-user", "--net", "--"];
    let probe = Command::new(program).args(args).arg("true").output().ok()?;
    probe.status.success().then_some((program, args))
}
