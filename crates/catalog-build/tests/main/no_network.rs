//! The build is hermetic and offline. This file is the proof.
//!
//! `connector-spec` is pure by construction, so the network can only enter through this crate. The
//! invariant is therefore local to it, and it is proven two ways here, weakest to strongest:
//!
//! 1. [`build_records_no_network_attempt`] — runs a real build with the [`catalog_build::net`]
//!    seam armed to refuse, and asserts the seam was never reached.
//! 2. [`the_network_seam_is_the_only_door`] — a source audit, so that (1) cannot be defeated by
//!    code that opens a socket without going through the seam.
//!
//! The third form — running the real binary inside a network namespace with no interfaces — lives
//! in `crates/catalog-cli/tests/offline_binary.rs`, because only the crate that declares the
//! binary gets `CARGO_BIN_EXE_*`. [`dependency_fence.rs`](../dependency_fence.rs) is what keeps the
//! source audit meaningful: without it, a socket could move into a dependency's `src/` and this
//! file would stop being able to see it.

use crate::common::Fixture;

fn build(root: &str) -> anyhow::Result<String> {
    let invocation = catalog_build::cli::Invocation {
        command: catalog_build::cli::Command::Build,
        root: Some(std::path::PathBuf::from(root)),
        ..Default::default()
    };
    let mut out = Vec::new();
    catalog_build::run(&invocation, &mut out)?;
    Ok(String::from_utf8(out).expect("CLI output is UTF-8"))
}

fn check(root: &str) -> anyhow::Result<String> {
    let invocation = catalog_build::cli::Invocation {
        command: catalog_build::cli::Command::Check,
        root: Some(std::path::PathBuf::from(root)),
        ..Default::default()
    };
    let mut out = Vec::new();
    catalog_build::run(&invocation, &mut out)?;
    Ok(String::from_utf8(out).expect("CLI output is UTF-8"))
}

/// The build performs **no network IO**.
#[test]
fn build_records_no_network_attempt() {
    let fixture = Fixture::with_provider("no-network", "zendesk");

    let denial = catalog_build::net::deny();
    build(fixture.root().to_str().unwrap()).expect("build succeeds offline");

    assert_eq!(
        denial.attempts(),
        0,
        "`build` reached the network seam. Nothing in this surface may: re-vendoring a spec is a \
         script under `scripts/`, run deliberately and reviewed as a diff."
    );
    assert!(fixture.exists("catalog/zendesk.catalog.json"));
}

/// Check recompiles and hashes committed bytes, but never treats upstream freshness as a reason to
/// contact a vendor. That belongs to `catalog sources diff`, not the lock verifier.
#[test]
fn check_records_no_network_attempt() {
    let fixture = Fixture::with_provider("check-no-network", "zendesk");
    build(fixture.root().to_str().unwrap()).expect("build the fixture");

    let denial = catalog_build::net::deny();
    let output = check(fixture.root().to_str().unwrap()).expect("check succeeds offline");

    assert_eq!(denial.attempts(), 0, "`check` reached the network seam");
    assert_eq!(output, "1 provider, 4 artifacts verified\n");
}

/// Every network primitive in this crate must live behind `src/net.rs`, or the counter above is
/// measuring nothing.
#[test]
fn the_network_seam_is_the_only_door() {
    const FORBIDDEN: &[&str] = &[
        "std::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "reqwest",
        "ureq",
        "hyper",
        "curl",
    ];

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offences = Vec::new();
    for entry in std::fs::read_dir(&src).expect("read src/") {
        let path = entry.expect("src entry").path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "net.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read source file");
        for (number, line) in text.lines().enumerate() {
            // Doc comments and ordinary comments name these primitives on purpose.
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for needle in FORBIDDEN {
                if code.contains(needle) {
                    offences.push(format!("{}:{}: {needle}", path.display(), number + 1));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "network primitives outside src/net.rs:\n{}",
        offences.join("\n")
    );
}
