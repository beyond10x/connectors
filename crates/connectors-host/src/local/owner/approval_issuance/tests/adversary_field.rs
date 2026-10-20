//! Adversary case for story:approval-issue-slack-bot-post (issue #180), driven
//! from the unit's contract text (`contracts/cli/v1alpha1/semantics.md`, "The
//! refused argument").
//!
//! A path through a symbolic link to a private directory meets every other
//! listed condition: each component is a plain name, the directory it names
//! exists, is the caller's own and private, and its ancestors are the
//! caller's and not group- or world-writable. `filesystem::directory` opens
//! each component with `O_NOFOLLOW`, so `issue` refuses it. That refusal
//! stays (decided in correction round 1: it is the safer behaviour), and the
//! contract lists it: a symbolic link in any component is `invalid_input`
//! naming `proof_output`. macOS's own `TMPDIR` (`/var/folders/…`, `/var` →
//! `/private/var`) is such a path.
use super::*;

#[cfg(unix)]
#[test]
fn issue_refuses_a_proof_output_named_through_a_symlink_naming_proof_output() {
    let root = root();
    let t = setup(root.path(), None, NO_CLOCK);
    t.fake_issuer();
    t.policy();
    let approved = prepare(&t.paths, "fixture", &t.request(), until())
        .unwrap()
        .subject_sha256;
    let private = root.path().join("private");
    std::fs::create_dir(&private).unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&private, &link).unwrap();
    let answer = |output: &Path| match issue(
        &t.paths,
        "fixture",
        &t.request(),
        &approved,
        output,
        until(),
    ) {
        Err(error) => (error.code, error.field.map(Argument::as_str)),
        Ok(_) => panic!("issued to {}", output.display()),
    };
    // Control: the same directory named directly passes every proof-output
    // check and stops later, at the absent clock.
    assert_eq!(
        answer(&private.join("proof.json")),
        (Code::Unavailable, None)
    );
    assert_eq!(
        answer(&link.join("proof.json")),
        (Code::InvalidInput, Some("proof_output"))
    );
    assert!(!private.join("proof.json").exists());
}
