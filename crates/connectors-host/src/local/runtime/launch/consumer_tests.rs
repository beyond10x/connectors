//! The consumer launch on Windows (consumer-launch.md §8). The consumer is
//! this test binary, re-entered through libtest's filter syntax.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    os::windows::io::FromRawHandle,
    time::{Duration, Instant},
};

const NAME: &str = "local::runtime::launch::consumer_tests::consumer_fixture";

/// The consumer: reads the credential from the handle its environment names
/// and records it, with the names of the variables it was given, then exits
/// with 7. The ordinary test invocation does nothing.
#[test]
fn consumer_fixture() {
    let Some(out) =
        std::env::args().find_map(|a| a.strip_prefix("consumer-out=").map(String::from))
    else {
        return;
    };
    let value: usize = std::env::var(CREDENTIAL_HANDLE_VARIABLE)
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: the launcher gave this process exactly this inherited handle.
    let mut credential = unsafe { File::from_raw_handle(value as _) };
    let mut material = String::new();
    credential.read_to_string(&mut material).unwrap();
    let mut names: Vec<String> = std::env::vars().map(|(name, _)| name).collect();
    names.sort();
    std::fs::write(out, format!("{material}\n{}", names.join(","))).unwrap();
    std::process::exit(7);
}

/// The credential reaches the consumer as one inherited handle named by
/// `CONNECTORS_CREDENTIAL_HANDLE`; a caller's variable of that name, in any
/// letter case, is not passed, and only the passed prefixes reach it. The
/// launcher returns the consumer's exit code and removes the image copy.
#[test]
fn a_consumer_reads_its_credential_from_the_inherited_handle() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("out");
    let exe = std::env::current_exe().unwrap();
    let hash = hex::encode(Sha256::digest(std::fs::read(&exe).unwrap()));
    let image = super::super::artifact::capture(
        File::open(&exe).unwrap(),
        &hash,
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let copy = crate::local::filesystem::windows::final_path(&image).unwrap();
    let consumer = Consumer::new(
        image,
        sealed(b"fictional credential").unwrap(),
        vec!["--exact".into(), NAME.into(), "--".into()],
        ["CONNECTORS_".to_owned(), "connectors_".to_owned()].into(),
    );
    let caller = [
        ("CONNECTORS_FIXTURE", "passed"),
        ("CONNECTORS_CREDENTIAL_HANDLE", "999"),
        ("connectors_credential_handle", "999"),
        ("UNLISTED", "dropped"),
    ]
    .map(|(name, value)| (OsString::from(name), OsString::from(value)));
    let code = consumer
        .run(&[format!("consumer-out={}", out.display())], caller)
        .unwrap();
    assert_eq!(code, 7);
    let recorded = std::fs::read_to_string(&out).unwrap();
    let (material, names) = recorded.split_once('\n').unwrap();
    assert_eq!(material, "fictional credential");
    let names: Vec<&str> = names
        .split(',')
        .filter(|name| name.to_ascii_uppercase().starts_with("CONNECTORS_") || *name == "UNLISTED")
        .collect();
    assert_eq!(
        names,
        ["CONNECTORS_CREDENTIAL_HANDLE", "CONNECTORS_FIXTURE"]
    );
    assert!(!copy.exists(), "the image copy outlived its consumer");
}

/// An argument holding NUL starts nothing.
#[test]
fn an_argument_holding_nul_is_invalid_input() {
    let exe = std::env::current_exe().unwrap();
    let hash = hex::encode(Sha256::digest(std::fs::read(&exe).unwrap()));
    let image = super::super::artifact::capture(
        File::open(&exe).unwrap(),
        &hash,
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let consumer = Consumer::new(
        image,
        sealed(b"fictional credential").unwrap(),
        Vec::new(),
        BTreeSet::new(),
    );
    assert!(matches!(
        consumer.run(&["a\0b".into()], Vec::new()),
        Err(Failure::InvalidInput)
    ));
}

/// Adversary cases on the capture, the consumer launch and the consumer job.
mod adversary_b;
