//! Custody of the values the plugin is given (`contracts/reads/v1alpha1/semantics.md`,
//! "Exec-plugin credentials": the adapter's own buffers holding the environment
//! strings built for the exec are zeroized before their memory is released). An
//! allow-listed variable such as `AWS_SECRET_ACCESS_KEY` can be a secret; no heap
//! block the adapter frees during a run may still hold its value. Measured with a
//! probe allocator, so this file is its own test binary. Fixture values only.
#![cfg(target_os = "linux")]
use connectors_kubernetes::exec_credential::{Binding, Credentials, rfc3339_ms};
use sha2::{Digest, Sha256};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

const MARKER: &[u8] = b"fixture-env-custody-marker";
const V1: &str = "client.authentication.k8s.io/v1";
static ARMED: AtomicBool = AtomicBool::new(false);
static FREED_WITH_MARKER: AtomicUsize = AtomicUsize::new(0);

/// While armed, every freed block is searched for the marker before it is
/// returned; a zeroized block holds none.
struct Probe;
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ARMED.load(Ordering::Relaxed) && (MARKER.len()..=1 << 20).contains(&layout.size()) {
            // SAFETY: the block is still allocated and `size` bytes long; it is
            // read byte by byte, never as a reference to possibly uninitialised
            // memory.
            let at = |i: usize| unsafe { std::ptr::read_volatile(ptr.add(i)) };
            let found = (0..=layout.size() - MARKER.len())
                .any(|start| MARKER.iter().enumerate().all(|(k, b)| at(start + k) == *b));
            if found {
                FREED_WITH_MARKER.fetch_add(1, Ordering::Relaxed);
            }
        }
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static GLOBAL: Probe = Probe;

/// The value exists only as what `ambient` returns: assembled into one
/// exactly sized buffer, so no partial copy is freed on the way.
fn secret_value() -> String {
    let (head, tail) = MARKER.split_at(12);
    let mut value = Vec::with_capacity(MARKER.len());
    value.extend_from_slice(head);
    value.extend_from_slice(tail);
    String::from_utf8(value).unwrap()
}

#[test]
fn exec_plugin_environment_values_are_zeroized_before_release() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture-plugin");
    let seen = dir.path().join("seen");
    let token = "fixture-custody-token";
    fs::write(
        &path,
        format!(
            "#!/bin/sh\n[ -n \"$AWS_SECRET_ACCESS_KEY\" ] && : > {}\n\
             printf '%s' '{{\"apiVersion\":\"{V1}\",\"kind\":\"ExecCredential\",\"status\":{{\"token\":\"{token}\"}}}}'\n",
            seen.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let binding = Binding {
        sha256: hex::encode(Sha256::digest(fs::read(&path).unwrap())),
        path: path.clone(),
        user: "eks-fixture-user".into(),
        env: vec!["AWS_SECRET_ACCESS_KEY".into()],
        home: None,
        timeout_ms: Some(5_000),
    };
    let config = format!(
        "users:\n- name: eks-fixture-user\n  user:\n    exec:\n      apiVersion: {V1}\n      \
         command: {}\n",
        path.display()
    );
    let credentials = Credentials::new(binding).unwrap();
    let now = rfc3339_ms("2029-01-01T00:00:00Z").unwrap();

    // The probe sees a block released with the marker in it.
    ARMED.store(true, Ordering::SeqCst);
    drop(secret_value());
    ARMED.store(false, Ordering::SeqCst);
    assert_eq!(
        FREED_WITH_MARKER.swap(0, Ordering::SeqCst),
        1,
        "probe self-check"
    );

    ARMED.store(true, Ordering::SeqCst);
    let result = credentials.token(config.as_bytes(), &|| now, &|name| {
        (name == "AWS_SECRET_ACCESS_KEY").then(secret_value)
    });
    let answered = result.as_ref().is_ok_and(|t| &t[..] == token.as_bytes());
    drop(result);
    drop(credentials);
    ARMED.store(false, Ordering::SeqCst);
    assert!(answered, "the plugin did not answer");
    assert!(seen.exists(), "the plugin was not given the value");
    assert_eq!(
        FREED_WITH_MARKER.swap(0, Ordering::SeqCst),
        0,
        "heap blocks holding an environment value were released without being zeroized"
    );
}
