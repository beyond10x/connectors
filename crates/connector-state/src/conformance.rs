//! **The suite every backend runs against itself.**
//!
//! A port with three implementations is a promise that they behave the same. This is where that
//! promise is checked, so "SQLite behaves like PostgreSQL" is a failing test rather than a comment.
//!
//! It lives in the library rather than in each backend's `tests/` so there is exactly one copy —
//! the same reason the port exists at all. A backend's test is one line:
//!
//! ```no_run
//! # use connector_state::{conformance, MemoryState};
//! conformance::run(&MemoryState::new());
//! ```
//!
//! # What it deliberately pins
//!
//! The cases are the ones where an implementation can be plausibly wrong rather than obviously
//! wrong. [`StateStore::append`](crate::StateStore::append) refusing **without mutating** is the
//! sharpest: appending and then discovering the bound was exceeded leaves a half-written cell, and
//! an append-only log that has lost its invariant reads as corruption much later, somewhere else.

use crate::{StateError, StateStore};

/// Run every conformance case. Panics with a named case on the first divergence.
///
/// The store must be empty, or at least free of the keys below.
pub fn run(store: &dyn StateStore) {
    absent_cells_read_as_none(store);
    a_replaced_cell_reads_back_exactly(store);
    replace_overwrites_rather_than_appends(store);
    append_creates_then_extends(store);
    append_refuses_without_mutating_when_the_bound_would_break(store);
    a_cell_longer_than_the_bound_refuses_rather_than_truncates(store);
    delete_is_idempotent(store);
    an_invalid_key_is_refused_by_every_operation(store);
    a_zero_bound_is_refused(store);
}

fn key(case: &str) -> String {
    format!("conformance.{case}")
}

fn absent_cells_read_as_none(store: &dyn StateStore) {
    let key = key("absent");
    let _ = store.delete(&key);
    assert_eq!(
        store.read(&key, 1024),
        Ok(None),
        "an unwritten cell must read as None, not as empty bytes: a caller distinguishes \
         'never configured' from 'configured empty'"
    );
}

fn a_replaced_cell_reads_back_exactly(store: &dyn StateStore) {
    let key = key("roundtrip");
    // Bytes, not text: a state cell carries whatever an Integration encoded, including zero bytes
    // and invalid UTF-8. A backend that round-trips through a string would corrupt both.
    let body = vec![0_u8, 1, 2, 255, 254, b'\n', 0];
    store.replace(&key, &body, 1024).expect("replace");
    assert_eq!(
        store.read(&key, 1024),
        Ok(Some(body)),
        "byte-exact round trip"
    );
    let _ = store.delete(&key);
}

fn replace_overwrites_rather_than_appends(store: &dyn StateStore) {
    let key = key("overwrite");
    store.replace(&key, b"first", 1024).expect("first");
    store.replace(&key, b"second", 1024).expect("second");
    assert_eq!(
        store.read(&key, 1024),
        Ok(Some(b"second".to_vec())),
        "replace must overwrite"
    );
    let _ = store.delete(&key);
}

fn append_creates_then_extends(store: &dyn StateStore) {
    let key = key("append");
    let _ = store.delete(&key);
    assert_eq!(
        store.append(&key, b"one", 1024),
        Ok(3),
        "append to an absent cell must create it and return its new length"
    );
    assert_eq!(store.append(&key, b"two", 1024), Ok(6));
    assert_eq!(store.read(&key, 1024), Ok(Some(b"onetwo".to_vec())));
    let _ = store.delete(&key);
}

fn append_refuses_without_mutating_when_the_bound_would_break(store: &dyn StateStore) {
    let key = key("append-bound");
    let _ = store.delete(&key);
    store.replace(&key, b"12345", 16).expect("seed");
    assert_eq!(
        store.append(&key, b"678901", 8),
        Err(StateError::Capacity),
        "an append past the bound must refuse"
    );
    assert_eq!(
        store.read(&key, 16),
        Ok(Some(b"12345".to_vec())),
        "and it must leave the cell untouched — a partially appended log is corruption that \
         surfaces later, somewhere else"
    );
    // Exactly at the bound is admitted: the check is `>`, not `>=`.
    assert_eq!(
        store.append(&key, b"678", 8),
        Ok(8),
        "exactly the bound fits"
    );
    let _ = store.delete(&key);
}

fn a_cell_longer_than_the_bound_refuses_rather_than_truncates(store: &dyn StateStore) {
    let key = key("read-bound");
    store.replace(&key, b"0123456789", 1024).expect("seed");
    assert_eq!(
        store.read(&key, 4),
        Err(StateError::Capacity),
        "a bounded read of an over-long cell must refuse; a prefix would be parsed as the whole"
    );
    assert_eq!(store.read(&key, 10), Ok(Some(b"0123456789".to_vec())));
    let _ = store.delete(&key);
}

fn delete_is_idempotent(store: &dyn StateStore) {
    let key = key("delete");
    store.replace(&key, b"x", 16).expect("seed");
    assert_eq!(store.delete(&key), Ok(()));
    assert_eq!(
        store.delete(&key),
        Ok(()),
        "deleting an absent cell succeeds: a caller cleaning up should not have to know whether \
         it got there first"
    );
    assert_eq!(store.read(&key, 16), Ok(None));
}

fn an_invalid_key_is_refused_by_every_operation(store: &dyn StateStore) {
    // Every operation, not just the writes: a backend that validates on write and not on read
    // answers `None` for a key it should have refused, which reads as "not configured".
    for bad in ["", "Upper", "has space", "sl/ash"] {
        assert_eq!(
            store.read(bad, 16),
            Err(StateError::Invalid),
            "read {bad:?}"
        );
        assert_eq!(
            store.replace(bad, b"x", 16),
            Err(StateError::Invalid),
            "replace {bad:?}"
        );
        assert_eq!(
            store.append(bad, b"x", 16),
            Err(StateError::Invalid),
            "append {bad:?}"
        );
        assert_eq!(
            store.delete(bad),
            Err(StateError::Invalid),
            "delete {bad:?}"
        );
    }
}

fn a_zero_bound_is_refused(store: &dyn StateStore) {
    let key = key("zero-bound");
    assert_eq!(store.read(&key, 0), Err(StateError::Invalid));
    assert_eq!(store.replace(&key, b"", 0), Err(StateError::Invalid));
    assert_eq!(store.append(&key, b"", 0), Err(StateError::Invalid));
}
