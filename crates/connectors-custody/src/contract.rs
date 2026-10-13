//! The custody contract suite, generic over [`Backend`]: one set of checks
//! every binding answers, the in-memory reference on every platform and each
//! platform binding on its own platform (`semantics.md` §5.1).
//!
//! Every check uses fresh scope and version ids and deletes what it wrote, so
//! it can run against a real store without touching any other item.
use crate::{
    Backend, Failure, Material, Purpose, Result, SIGNING_SEED_BYTES, Scope, State, Version,
};
use std::{
    fmt::Debug,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Run every check. `open` opens the binding under test for one scope.
pub fn run<B: Backend + Debug>(open: &dyn Fn(Scope) -> Result<B>) {
    let mut suite = Suite {
        open,
        written: Vec::new(),
        outputs: Vec::new(),
    };
    let marker = format!("custody-contract-secret-{}", Uuid::new_v4().simple());
    suite.available();
    suite.write_reads_back_exact_bytes(&marker);
    suite.second_write_of_a_version_conflicts(&marker);
    suite.delete_then_missing(&marker);
    suite.another_scope_is_denied(&marker);
    suite.purposes_are_distinct_items();
    suite.value_bound(&marker);
    suite.guard_refusal_writes_and_deletes_nothing(&marker);
    suite.confirm_reads_under_the_guard();
    suite.no_secret_in_debug_or_error_output(&marker);
    suite.cleanup();
}

struct Suite<'a, B> {
    open: &'a dyn Fn(Scope) -> Result<B>,
    written: Vec<Version>,
    outputs: Vec<String>,
}

fn until() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn ok() -> Result<()> {
    Ok(())
}
fn credential() -> Scope {
    Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap()
}
fn version(scope: Scope) -> Version {
    Version::new(scope, Uuid::new_v4()).unwrap()
}
/// Opaque bytes, including NUL and bytes that are not UTF-8.
fn bytes(marker: &str, tag: &str) -> Vec<u8> {
    let mut bytes = format!("{marker}-{tag}").into_bytes();
    bytes.extend_from_slice(&[0, 0xff, 0xfe, b'\n', 0x80]);
    bytes
}

impl<B: Backend + Debug> Suite<'_, B> {
    fn open(&mut self, scope: Scope) -> B {
        let backend = (self.open)(scope).expect("the binding under test opens");
        self.outputs.push(format!("{backend:?}"));
        backend
    }

    fn record<T: Debug>(&mut self, result: &Result<T>) {
        self.outputs.push(format!("{result:?}"));
        if let Err(failure) = result {
            self.outputs.push(failure.to_string());
        }
    }

    fn write(&mut self, backend: &B, version: Version, material: &[u8]) -> Result<()> {
        let result = backend.write_new(version, &Material::from_slice(material), until(), ok);
        self.record(&result);
        if result.is_ok() {
            self.written.push(version);
        }
        result
    }

    fn read(&mut self, backend: &B, version: Version) -> Result<Material> {
        let result = backend.read(version);
        self.record(&result);
        result
    }

    fn available(&mut self) {
        let backend = self.open(credential());
        assert_eq!(backend.state(), State::Available, "store is available");
        assert_eq!(backend.max_bytes(), backend.kind().max_bytes());
    }

    fn write_reads_back_exact_bytes(&mut self, marker: &str) {
        let scope = credential();
        let backend = self.open(scope);
        let version = version(scope);
        let material = bytes(marker, "exact");
        assert_eq!(self.write(&backend, version, &material), Ok(()), "write");
        let read = self.read(&backend, version).expect("read back");
        assert!(read.ct_eq(&material), "read back the exact bytes");
        // A second handle on the same scope reads the same version.
        let again = self.open(scope);
        assert!(self.read(&again, version).unwrap().ct_eq(&material));
    }

    fn second_write_of_a_version_conflicts(&mut self, marker: &str) {
        let scope = credential();
        let backend = self.open(scope);
        let version = version(scope);
        let first = bytes(marker, "first");
        assert_eq!(self.write(&backend, version, &first), Ok(()));
        assert_eq!(
            self.write(&backend, version, &bytes(marker, "second")),
            Err(Failure::Conflict),
            "a second write of one version"
        );
        assert!(
            self.read(&backend, version).unwrap().ct_eq(&first),
            "a conflicting write never overwrites"
        );
    }

    fn delete_then_missing(&mut self, marker: &str) {
        let scope = credential();
        let backend = self.open(scope);
        let version = version(scope);
        assert!(matches!(
            self.read(&backend, version),
            Err(Failure::Missing)
        ));
        assert_eq!(
            self.write(&backend, version, &bytes(marker, "delete")),
            Ok(())
        );
        let deleted = backend.delete(version, until(), ok);
        self.record(&deleted);
        assert_eq!(deleted, Ok(()), "delete");
        assert!(
            matches!(self.read(&backend, version), Err(Failure::Missing)),
            "read after delete"
        );
        let repeated = backend.delete(version, until(), ok);
        self.record(&repeated);
        assert_eq!(repeated, Ok(()), "a deleted version is already deleted");
    }

    fn another_scope_is_denied(&mut self, marker: &str) {
        let scope = credential();
        let own = self.open(scope);
        let version = version(scope);
        let material = bytes(marker, "scoped");
        assert_eq!(self.write(&own, version, &material), Ok(()));
        let other = self.open(credential());
        assert!(
            matches!(self.read(&other, version), Err(Failure::Denied)),
            "a scope reading another scope's version"
        );
        assert_eq!(self.write(&other, version, &material), Err(Failure::Denied));
        let deleted = other.delete(version, until(), ok);
        self.record(&deleted);
        assert_eq!(deleted, Err(Failure::Denied));
        assert!(self.read(&own, version).unwrap().ct_eq(&material));
    }

    fn purposes_are_distinct_items(&mut self) {
        let (authority, allocation, id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let credential = Scope::new(authority, allocation).unwrap();
        let signing = Scope::approval_signing(authority, allocation).unwrap();
        assert_eq!(signing.purpose(), Purpose::ApprovalSigning);
        let (a, b) = (self.open(credential), self.open(signing));
        let (va, vb) = (
            Version::new(credential, id).unwrap(),
            Version::new(signing, id).unwrap(),
        );
        let seed = [0x5a; SIGNING_SEED_BYTES];
        assert_eq!(self.write(&a, va, b"fictional-credential"), Ok(()));
        assert_eq!(
            self.write(&b, vb, &seed),
            Ok(()),
            "identical ids under another purpose are another item"
        );
        assert!(matches!(self.read(&a, vb), Err(Failure::Denied)));
        assert!(matches!(self.read(&b, va), Err(Failure::Denied)));
        assert!(self.read(&a, va).unwrap().ct_eq(b"fictional-credential"));
        assert!(self.read(&b, vb).unwrap().ct_eq(&seed));
        assert_eq!(
            self.write(&b, version(signing), &[1; SIGNING_SEED_BYTES + 1]),
            Err(Failure::InvalidMaterial),
            "a signing seed is exactly 32 bytes"
        );
    }

    fn value_bound(&mut self, marker: &str) {
        let scope = credential();
        let backend = self.open(scope);
        let max = backend.max_bytes();
        let mut largest = bytes(marker, "largest");
        largest.resize(max, 0x59);
        let fits = version(scope);
        assert_eq!(
            self.write(&backend, fits, &largest),
            Ok(()),
            "a value of exactly the bound ({max} bytes)"
        );
        assert!(self.read(&backend, fits).unwrap().ct_eq(&largest));
        let over = version(scope);
        let mut oversized = largest.clone();
        oversized.push(0x59);
        assert_eq!(
            self.write(&backend, over, &oversized),
            Err(Failure::InvalidMaterial),
            "a value one byte over the bound"
        );
        assert!(
            matches!(self.read(&backend, over), Err(Failure::Missing)),
            "nothing was written for an oversized value"
        );
        let empty = version(scope);
        assert_eq!(
            self.write(&backend, empty, &[]),
            Err(Failure::InvalidMaterial)
        );
        assert!(matches!(self.read(&backend, empty), Err(Failure::Missing)));
    }

    fn guard_refusal_writes_and_deletes_nothing(&mut self, marker: &str) {
        let scope = credential();
        let backend = self.open(scope);
        let refused = version(scope);
        let result = backend.write_new(
            refused,
            &Material::new(bytes(marker, "guarded")),
            until(),
            || Err(Failure::Denied),
        );
        self.record(&result);
        assert_eq!(
            result,
            Err(Failure::Denied),
            "the guard's refusal is returned"
        );
        assert!(matches!(
            self.read(&backend, refused),
            Err(Failure::Missing)
        ));
        let kept = version(scope);
        let material = bytes(marker, "kept");
        assert_eq!(self.write(&backend, kept, &material), Ok(()));
        let deleted = backend.delete(kept, until(), || Err(Failure::Conflict));
        self.record(&deleted);
        assert_eq!(deleted, Err(Failure::Conflict));
        assert!(self.read(&backend, kept).unwrap().ct_eq(&material));
    }

    fn confirm_reads_under_the_guard(&mut self) {
        let signing = Scope::approval_signing(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        let backend = self.open(signing);
        let version = version(signing);
        let seed = [0x33; SIGNING_SEED_BYTES];
        assert_eq!(self.write(&backend, version, &seed), Ok(()));
        let mut seen = false;
        let confirmed = backend.confirm(version, until(), ok, |material| {
            seen = material.ct_eq(&seed);
            Ok(())
        });
        self.record(&confirmed);
        assert_eq!(confirmed, Ok(()));
        assert!(seen, "validate saw the exact stored bytes");
        let refused = backend.confirm(version, until(), ok, |_| Err(Failure::InvalidMaterial));
        assert_eq!(refused, Err(Failure::InvalidMaterial));
        let guarded = backend.confirm(
            version,
            until(),
            || Err(Failure::Denied),
            |_| panic!("validate never runs after a guard refusal"),
        );
        assert_eq!(guarded, Err(Failure::Denied));
        let missing = backend.confirm(
            Version::new(signing, Uuid::new_v4()).unwrap(),
            until(),
            ok,
            |_| Ok(()),
        );
        assert_eq!(missing, Err(Failure::Missing));
    }

    fn no_secret_in_debug_or_error_output(&mut self, marker: &str) {
        let material = Material::new(bytes(marker, "debug"));
        self.outputs.push(format!("{material:?}"));
        assert_eq!(format!("{material:?}"), "Material(redacted)");
        for failure in [
            Failure::Missing,
            Failure::Unavailable,
            Failure::Denied,
            Failure::InvalidMaterial,
            Failure::Conflict,
            Failure::OutcomeUnknown,
        ] {
            self.outputs.push(format!("{failure:?} {failure}"));
        }
        assert!(!self.outputs.is_empty());
        for output in &self.outputs {
            assert!(
                !output.contains(marker),
                "a Debug or error output carries secret bytes"
            );
        }
    }

    fn cleanup(&mut self) {
        for version in std::mem::take(&mut self.written) {
            let backend = (self.open)(version.scope()).expect("reopen for cleanup");
            assert_eq!(backend.delete(version, until(), ok), Ok(()), "cleanup");
            assert!(matches!(backend.read(version), Err(Failure::Missing)));
        }
    }
}
