//! Read uses that fell due are recorded as expired in bounded batches before
//! the caller's own batch, so one write never carries every expiry the store
//! accumulated (Entity Runtime batch cost grows faster than its member count).
use super::tests::{NOW, binding, fixture, prepared, publish_fixture};
use super::*;
use crate::local::metadata::with_expiry_batches;

/// One read invoke at `now` whose read use is released, and so falls due
/// 1 s later.
fn read_invoke(registry: &Registry, reference: &str, now: u64) {
    registry
        .admit_read(&binding(), reference, &BTreeSet::new(), now)
        .unwrap();
    let captured = registry
        .capture_read(&binding(), reference, &BTreeSet::new(), now, now + 1000)
        .unwrap();
    let dispatched = registry.dispatch_read(captured, now).unwrap();
    registry.release_read(dispatched, now).unwrap();
}

/// Read uses the store records in a state other than `Expired`.
fn unexpired_read_uses(root: &Path) -> usize {
    Metadata::inspect(root)
        .unwrap()
        .recorded_rows()
        .iter()
        .filter(|row| {
            row.contains("entity: \"connectors.credential_evidence.ReadUse\"")
                && !row.contains("lifecycle_state: \"Expired\"")
        })
        .count()
}

/// A store with `n` released read uses, all due by `NOW + 10_000`.
fn store_with_due_read_uses(n: u64) -> (tempfile::TempDir, Registry, String) {
    let (root, registry) = fixture();
    let (_, candidate) = prepared(&registry, "one", NOW);
    let reference = publish_fixture(&registry, candidate, NOW);
    for invoke in 1..=n {
        read_invoke(&registry, &reference, NOW + invoke);
    }
    assert_eq!(unexpired_read_uses(root.path()), n as usize);
    (root, registry, reference)
}

const K: usize = 4;
const DUE: u64 = 10;

#[test]
fn more_than_two_batches_of_due_expiries_are_recorded_in_bounded_batches_first() {
    let (root, registry, reference) = store_with_due_read_uses(DUE);
    let later = NOW + 10_000;
    let (captured, expiry_batches) = with_expiry_batches(K, None, || {
        registry.capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            later,
            later + 1000,
        )
    });
    let captured = captured.unwrap();
    // ceil(10 / 4) expiry batches, then the caller's own batch.
    assert_eq!(expiry_batches, (DUE as usize).div_ceil(K));
    // Every read use that was due is expired; only the one just captured is not.
    assert_eq!(unexpired_read_uses(root.path()), 1);
    let dispatched = registry.dispatch_read(captured, later).unwrap();
    registry.release_read(dispatched, later).unwrap();
}

#[test]
fn a_failed_second_expiry_batch_keeps_the_first_and_skips_the_callers_action() {
    let (root, registry, reference) = store_with_due_read_uses(DUE);
    let later = NOW + 10_000;
    let (refused, expiry_batches) = with_expiry_batches(K, Some(2), || {
        registry.capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            later,
            later + 1000,
        )
    });
    assert!(refused.is_err());
    assert_eq!(expiry_batches, 2);
    // The first batch's K expiries committed; the caller's read use was not
    // captured, so the count only fell by K.
    assert_eq!(unexpired_read_uses(root.path()), DUE as usize - K);
    // The next call records the expiries still due, then its own action.
    let (captured, expiry_batches) = with_expiry_batches(K, None, || {
        registry.capture_read(
            &binding(),
            &reference,
            &BTreeSet::new(),
            later,
            later + 1000,
        )
    });
    captured.unwrap();
    assert_eq!(expiry_batches, (DUE as usize - K).div_ceil(K));
    assert_eq!(unexpired_read_uses(root.path()), 1);
}

/// A write that records nothing but due expiries, more than two batches of
/// them: every batch but the last runs ahead, and the last is the final batch
/// that carries the post-commit checks. No expiry is dropped.
#[test]
fn a_write_of_only_due_expiries_records_every_one_with_the_last_batch_final() {
    let (root, _registry, _reference) = store_with_due_read_uses(DUE);
    let (persisted, expiry_batches) = with_expiry_batches(K, None, || {
        // The sweep every registry use transaction runs, and nothing else.
        let mut metadata = Metadata::update(root.path(), true).unwrap();
        metadata
            .connection
            .execute(
                "DELETE FROM registry_uses WHERE expires_at_ms<=?1",
                [timestamp(NOW + 10_000).unwrap()],
            )
            .unwrap();
        metadata.persist()
    });
    persisted.unwrap();
    // ceil(10 / 4) = 3 batches: two ahead, the last as the final batch.
    assert_eq!(expiry_batches, (DUE as usize).div_ceil(K) - 1);
    assert_eq!(unexpired_read_uses(root.path()), 0);
}
