//! Adversary cases for story:revision-conflict-wire-code. Each case drives
//! the implementation from a statement the unit itself wrote:
//! `contracts/cli/v1alpha1/semantics.md` now says a metadata write "not
//! committed because the store's recorded revision moved under it" answers
//! `revision_conflict` with `stage = publication` and
//! `next_action = retry_explicitly`.
use connectors_host::local::{self, approval_keys, approval_policy, owner};

/// The approval key and approval policy stores persist through the same
/// Entity Runtime batch that raises `local::Failure::ConcurrentRevision`, and
/// their `From<local::Failure>` conversions are the only path that failure
/// takes to the CLI. A conflict there must not read as an unreadable store.
#[test]
fn approval_stores_do_not_fold_a_revision_conflict_into_an_unreadable_store() {
    let keys = approval_keys::Failure::from(local::Failure::ConcurrentRevision);
    let policy = approval_policy::Failure::from(local::Failure::ConcurrentRevision);
    assert_ne!(
        keys,
        approval_keys::Failure::MetadataUnavailable,
        "approval_keys: a revision conflict becomes metadata_unavailable"
    );
    assert_ne!(
        policy,
        approval_policy::Failure::MetadataUnavailable,
        "approval_policy: a revision conflict becomes metadata_unavailable"
    );
}

/// A guarded write's `capture_read` converts `registry::Failure` through
/// `owner::Error::from`, so a lost revision race there is a host failure with
/// `Code::RevisionConflict` before anything was sent. The semantics row for
/// `revision_conflict` excludes guarded writes: they keep the guarded-write
/// rule (semantics.md, "Every other write failure ... reports
/// `next_action = retry_status`"), at `stage = admission` because nothing was
/// attempted.
#[test]
fn a_guarded_write_revision_conflict_reports_what_the_semantics_row_says() {
    use owner::mutation::{Classification, Failure};
    let failure = Failure::from(owner::Error::from(
        local::registry::Failure::ConcurrentRevision,
    ));
    assert_eq!(
        (
            failure.stage(Classification::NotAttempted),
            failure.next_action(Classification::NotAttempted),
        ),
        ("admission", "retry_status"),
        "a guarded write's revision conflict follows the guarded-write rule \
         (semantics.md: every other write failure reports retry_status)"
    );
}
