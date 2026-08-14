//! **Every workspace member is classified, and the build path reaches no socket.**
//!
//! Design 02 §5: "a dependency fence classifies every crate as network/no-network and fails on
//! drift". [`no_network.rs`](../no_network.rs) proves that `catalog-build` never opens a socket
//! during a build. That proof is only as strong as the crate's dependency closure: the moment
//! `connector-secrets` — which ships an HTTP client for Vault — appears anywhere in it, "generation
//! is explicit, committed, deterministic and offline" stops being a statement about the build, and
//! `no_network.rs`'s source audit stops being able to see the violation, because the socket now
//! lives in a different crate's `src/`.
//!
//! So the fence is asserted over the **dependency graph**, not by convention. It is read out of
//! `Cargo.lock`, which is deliberate: the lock records the resolved graph including *optional*
//! dependencies, so adding the edge behind a feature flag trips this too.
//!
//! The three buckets are reclassified for this workspace. There is **no network crate yet** — the
//! platform family of design 02 §2 is M2's, and its `server` crate's egress module is where the one
//! allowed socket will live. That an allow-list is currently empty is itself asserted, so the day
//! a crate is added to it is a decision somebody wrote down.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The host library. Nothing in the build path may reach it.
const HOST_LIBRARY: &str = "connector-secrets";

/// The crates that make up the offline build path.
///
/// `catalog` is here because it is the leaf the pipeline writes into and reads back; `catalog-cli`
/// because it is the binary that runs the build; `connector-address` because it is the address
/// vocabulary everything else is written against, and a socket reachable from a crate every one of
/// them links would end the offline guarantee from below.
const COMPILER_CRATES: &[&str] = &[
    "catalog-cli",
    "catalog-build",
    "connector-spec",
    "catalog",
    // The catalog-pack reader: like `catalog`, a leaf the pipeline writes into — it embeds the
    // compiled pack — with zero non-optional dependencies of its own. It is in this bucket for the
    // same reason `catalog` is: `catalog` links it, so a socket reachable from it would end the
    // offline guarantee from below.
    "catalog-reader",
    "connector-address",
];

/// **Crates allowed to open sockets, named so the allowance is a decision rather than a silence.**
///
/// A fence whose guarantee is only "the build path is offline" is scoped narrowly enough that a
/// network-capable crate elsewhere in the workspace passes it without being looked at. That is the
/// "merely unexamined" state: a reader who finds `reqwest` in `Cargo.lock` and a passing test
/// cannot tell whether the edge was considered.
///
/// **Empty, today, and that is the state design 02 §5 describes for M1.** The one module allowed to
/// dial a vendor is `crates/server`'s egress, and the platform family does not exist yet. A crate
/// added here must be a **leaf** — nothing in the build path may depend on it, which
/// [`a_compiler_crate_cannot_reach_a_network_crate`] enforces in the direction that matters.
const NETWORK_CRATES: &[&str] = &[];

/// The build path does not reach the secret store — asserted over the dependency graph, not by
/// convention.
#[test]
fn the_build_path_does_not_depend_on_the_secret_store() {
    let lock = Lock::read();

    // A fence around a crate that does not exist is vacuous, and would go on passing for exactly as
    // long as it took someone to notice. Assert the subject is real first.
    assert!(
        lock.contains(HOST_LIBRARY),
        "`{HOST_LIBRARY}` is not a package in {}; this fence has nothing to fence",
        lock.path.display()
    );

    for crate_name in COMPILER_CRATES {
        let closure = lock.closure(crate_name);
        assert!(
            !closure.contains(HOST_LIBRARY),
            "`{crate_name}` depends on `{HOST_LIBRARY}`, directly or transitively: {}\n\
             `{HOST_LIBRARY}` opens sockets, so this edge would put an HTTP client in the compile \
             path and make `no_network.rs` a statement about less than the build.",
            lock.path_to(crate_name, HOST_LIBRARY)
                .unwrap_or_else(|| "<no path found>".to_owned()),
        );
    }
}

/// The host libraries: built and tested here, excluded from the build path, and opening no socket
/// of their own on any path a build takes. They are neither build-path nor network crates, and they
/// are listed so that [`every_workspace_member_is_classified`] has three buckets to sort into
/// rather than an unexplained remainder.
///
/// **Being in this list is a classification, not a fence.**
const HOST_LIBRARIES: &[&str] = &[
    // The plan-deriving core: it reads the canonical document and returns a request plan, and it
    // opens no socket — no HTTP client, no DNS resolver, no transport at all. The egress seam that
    // will carry one lives in the platform family. It is not in `COMPILER_CRATES` because nothing
    // in the build path depends on it, and it is not in `NETWORK_CRATES` because it may not dial.
    "connector-resolve",
    // Protocol-neutral proof types and their wire projection. They are shared by runtime
    // composition, but neither belongs to the catalogue compiler nor owns I/O.
    "domain",
    "protocol",
    // Admission turns a reviewed canonical operation into a zero-I/O plan. It may read the
    // compiler-owned document types, but the compiler has no reverse edge into this runtime use
    // case.
    "service",
    // The policy composition and authority implementation. The first network driver is deliberately
    // a separate leaf; `server` itself still has no socket dependency.
    "server",
    // The owner-bound secret store. Its Vault backend is an **optional** feature that is off by
    // default, and `reqwest` arrives only with it — which is exactly why the fence below reads the
    // lock rather than the feature-resolved graph: turning the feature on must not be able to put
    // an HTTP client in the build path unnoticed.
    HOST_LIBRARY,
];

/// **The edge that actually earns the allow-list its keep.**
///
/// `NETWORK_CRATES` on its own is a comment. This is what makes it structural: a compiler crate
/// reaching the host — `connector-cli -> connectors-api`, say, to "just reuse the server's types" —
/// puts an HTTP client, a DNS resolver and a listener back in the compile path by a different route
/// than the one [`connector_cli_does_not_depend_on_connector_secrets`] guards.
#[test]
fn a_compiler_crate_cannot_reach_a_network_crate() {
    let lock = Lock::read();

    // An empty allow-list is the current, deliberate state — see `NETWORK_CRATES`. The loop below
    // asserts nothing when it is empty, which is correct: there is no allowance to bound yet.
    for network_crate in NETWORK_CRATES {
        assert!(
            lock.contains(network_crate),
            "`{network_crate}` is not a package in {}; this fence has nothing to fence",
            lock.path.display()
        );
        for crate_name in COMPILER_CRATES {
            let closure = lock.closure(crate_name);
            assert!(
                !closure.contains(*network_crate),
                "`{crate_name}` depends on `{network_crate}`, directly or transitively: {}\n\
                 `{network_crate}` is allowed to open sockets precisely because it is a leaf. An \
                 edge into it from the compiler ends the offline guarantee `no_network.rs` states.",
                lock.path_to(crate_name, network_crate)
                    .unwrap_or_else(|| "<no path found>".to_owned()),
            );
        }
    }
}

/// **Every workspace member is deliberately one of three things.**
///
/// Without this, a new crate is simply not asked about: it is not a compiler crate so nothing fences
/// it, and it is not on the allow-list so nothing records that it may open a socket. That is the
/// "merely unexamined" state the allow-list exists to end, and it is why this asserts over the
/// workspace's own membership rather than over a second hand-kept list.
#[test]
fn every_workspace_member_is_classified() {
    let root = workspace_root();
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    let document: toml::Value = manifest.parse().expect("the workspace manifest parses");
    let members = document
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .expect("`[workspace] members`");

    let mut checked = 0usize;
    for member in members.iter().filter_map(toml::Value::as_str) {
        let path = root.join(member).join("Cargo.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let member_document: toml::Value = text
            .parse()
            .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
        let name = member_document
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{} has no `[package] name`", path.display()))
            .to_owned();

        assert!(
            COMPILER_CRATES.contains(&name.as_str())
                || NETWORK_CRATES.contains(&name.as_str())
                || HOST_LIBRARIES.contains(&name.as_str()),
            "`{name}` ({member}) is a workspace member classified as neither a compiler crate, a \
             host library, nor a network crate.\n\
             Add it to exactly one list in this file, with a comment saying why. A crate nobody \
             classified is a crate whose right to open a socket nobody decided."
        );
        checked += 1;
    }

    assert!(
        checked > 0,
        "no workspace members were read, so this asserted nothing"
    );
}

/// RTVBP's feature closure may not participate in canonical artifact generation.
///
/// The final Rust SDK enables `serde_json/preserve_order`. Cargo features unify within a workspace,
/// so making its adapter a root member changes JSON map traversal and therefore committed OpenAPI
/// and catalogue bytes even though no compiler crate directly imports RTVBP. The adapter is a
/// nested workspace with its own lock for exactly that reason.
#[test]
fn the_rtvbp_runtime_dependency_is_isolated_from_the_canonical_workspace() {
    let root = workspace_root();
    let root_manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    let root_document: toml::Value = root_manifest
        .parse()
        .expect("the workspace manifest parses");
    let excluded = root_document
        .get("workspace")
        .and_then(|workspace| workspace.get("exclude"))
        .and_then(toml::Value::as_array)
        .expect("`[workspace] exclude`");
    assert!(
        excluded
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|member| member == "crates/rtvbp-voice-endpoint"),
        "the RTVBP adapter must remain excluded from the canonical workspace"
    );
    assert!(
        !Lock::read().contains("rtvbp"),
        "the canonical workspace lock must not resolve RTVBP's feature closure"
    );

    let adapter_manifest_path = root.join("crates/rtvbp-voice-endpoint/Cargo.toml");
    let adapter_manifest = std::fs::read_to_string(&adapter_manifest_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", adapter_manifest_path.display()));
    let adapter_document: toml::Value = adapter_manifest
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", adapter_manifest_path.display()));
    assert!(
        adapter_document.get("workspace").is_some(),
        "the RTVBP adapter must remain a nested workspace with an independent feature closure"
    );
    assert!(
        root.join("crates/rtvbp-voice-endpoint/Cargo.lock")
            .is_file(),
        "the RTVBP adapter's independently reviewed dependency graph must stay locked"
    );
}

/// sipx owns real sockets, so its closure and bind call stay in one explicitly isolated crate.
#[test]
fn the_sipx_network_dependency_is_exactly_pinned_and_isolated() {
    const SIPX_REVISION: &str = "004ac534b8b222060ad2d2308763efe6e1dedc10";
    const SIPX_CRATES: &[&str] = &["sipx-call", "sipx-media", "sipx-sip", "sipx-transport"];

    let root = workspace_root();
    let root_manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    let root_document: toml::Value = root_manifest
        .parse()
        .expect("the workspace manifest parses");
    let excluded = root_document
        .get("workspace")
        .and_then(|workspace| workspace.get("exclude"))
        .and_then(toml::Value::as_array)
        .expect("`[workspace] exclude`");
    assert!(
        excluded
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|member| member == "crates/driver-sip"),
        "the socket-capable SIP driver must remain excluded from the canonical workspace"
    );
    let root_lock = Lock::read();
    for package in SIPX_CRATES {
        assert!(
            !root_lock.contains(package),
            "the canonical workspace lock must not resolve `{package}`"
        );
    }

    let driver = root.join("crates/driver-sip");
    let manifest_path = driver.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()));
    let document: toml::Value = manifest
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()));
    assert!(
        document.get("workspace").is_some(),
        "the SIP driver must remain a nested workspace with an independent feature closure"
    );
    assert!(
        driver.join("Cargo.lock").is_file(),
        "the SIP driver's independently reviewed dependency graph must stay locked"
    );
    let dependencies = document
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("driver dependencies");
    for package in SIPX_CRATES {
        let dependency = dependencies
            .get(*package)
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| panic!("`{package}` must be an explicit dependency table"));
        assert_eq!(
            dependency.get("git").and_then(toml::Value::as_str),
            Some("https://github.com/codewandler/sipx")
        );
        assert_eq!(
            dependency.get("rev").and_then(toml::Value::as_str),
            Some(SIPX_REVISION),
            "`{package}` must resolve from the reviewed immutable commit"
        );
    }

    let bind_symbol = ["sipx_transport", "::bind"].concat();
    let driver_source = std::fs::read_to_string(driver.join("src/lib.rs"))
        .expect("the SIP driver implementation source");
    assert!(
        driver_source.contains(&bind_symbol),
        "the named network-capable driver must contain the reviewed sipx bind"
    );
    for source in rust_sources_below(&root.join("crates")) {
        if source.starts_with(&driver) {
            continue;
        }
        let text = std::fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        assert!(
            !text.contains(&bind_symbol),
            "{} opens sipx sockets outside the sole network-capable driver",
            source.display()
        );
    }
}

fn rust_sources_below(root: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, sources: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("target") {
                    visit(&path, sources);
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                sources.push(path);
            }
        }
    }

    let mut sources = Vec::new();
    visit(root, &mut sources);
    sources
}

/// The resolved dependency graph, as `Cargo.lock` records it.
struct Lock {
    path: PathBuf,
    /// Package name to the names of its recorded dependencies.
    packages: BTreeMap<String, Vec<String>>,
}

impl Lock {
    /// Parse the workspace lockfile.
    ///
    /// # Panics
    ///
    /// If the lockfile cannot be found, read or parsed — in a test, each of those is the assertion
    /// failing rather than a condition to recover from.
    fn read() -> Self {
        let path = workspace_root().join("Cargo.lock");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let document: toml::Value = text
            .parse()
            .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));

        let mut packages = BTreeMap::new();
        let entries = document
            .get("package")
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| panic!("{} has no `[[package]]` entries", path.display()));
        for entry in entries {
            let Some(name) = entry.get("name").and_then(toml::Value::as_str) else {
                continue;
            };
            let dependencies = entry
                .get("dependencies")
                .and_then(toml::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(toml::Value::as_str)
                        // An entry is `name`, `name version`, or `name version (source)`.
                        .filter_map(|value| value.split_whitespace().next())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            packages.insert(name.to_owned(), dependencies);
        }

        Self { path, packages }
    }

    /// A graph stated directly, for asserting the walk itself.
    fn over(edges: &[(&str, &[&str])]) -> Self {
        Self {
            path: PathBuf::from("<synthetic>"),
            packages: edges
                .iter()
                .map(|(name, dependencies)| {
                    (
                        (*name).to_owned(),
                        dependencies.iter().map(|d| (*d).to_owned()).collect(),
                    )
                })
                .collect(),
        }
    }

    /// Whether the lockfile records a package by this name at all.
    fn contains(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Every package reachable from `root`, `root` excluded.
    fn closure(&self, root: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![root.to_owned()];
        while let Some(name) = pending.pop() {
            for dependency in self.packages.get(&name).into_iter().flatten() {
                if seen.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }
        seen
    }

    /// A rendered `a -> b -> c` chain from `root` to `target`, for the failure message. Breadth
    /// first, so the reported chain is a shortest one and names the edge worth deleting.
    fn path_to(&self, root: &str, target: &str) -> Option<String> {
        let mut previous: BTreeMap<String, String> = BTreeMap::new();
        let mut queue = std::collections::VecDeque::from([root.to_owned()]);
        let mut seen = BTreeSet::from([root.to_owned()]);
        while let Some(name) = queue.pop_front() {
            for dependency in self.packages.get(&name).into_iter().flatten() {
                if !seen.insert(dependency.clone()) {
                    continue;
                }
                previous.insert(dependency.clone(), name.clone());
                if dependency == target {
                    let mut chain = vec![target.to_owned()];
                    let mut step = target;
                    while let Some(parent) = previous.get(step) {
                        chain.push(parent.clone());
                        step = parent;
                    }
                    chain.reverse();
                    return Some(chain.join(" -> "));
                }
                queue.push_back(dependency.clone());
            }
        }
        None
    }
}

/// The workspace root, from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the workspace root")
        .to_path_buf()
}

/// The fence above is only as good as the walk under it, and the walk is the part that could pass
/// while seeing nothing.
///
/// A **direct** edge is easy to check by hand: adding `connector-secrets.workspace = true` to
/// `crates/catalog-build/Cargo.toml` fails the test above with
/// `catalog-build -> connector-secrets`. What cannot be checked that way without editing another
/// crate's manifest is the **transitive** case, which is the one that would actually happen — some
/// future crate takes the host library, and `catalog-build` takes that crate. So it is asserted
/// here over a synthetic graph instead.
#[test]
fn the_walk_finds_an_edge_that_is_not_direct() {
    let lock = Lock::over(&[
        ("catalog-build", &["connector-spec", "helper"]),
        ("connector-spec", &["thiserror"]),
        ("helper", &[HOST_LIBRARY]),
        (HOST_LIBRARY, &["reqwest"]),
        ("reqwest", &[]),
        ("thiserror", &[]),
    ]);

    let closure = lock.closure("catalog-build");
    assert!(
        closure.contains(HOST_LIBRARY),
        "a dependency two edges away must still be in the closure: {closure:?}"
    );
    // And what the operator is told is the chain to break, not just that one exists.
    assert_eq!(
        lock.path_to("catalog-build", HOST_LIBRARY).as_deref(),
        Some(format!("catalog-build -> helper -> {HOST_LIBRARY}").as_str())
    );

    // The control: with the middle crate's edge removed, the same walk reports nothing.
    let severed = Lock::over(&[
        ("catalog-build", &["connector-spec", "helper"]),
        ("connector-spec", &["thiserror"]),
        ("helper", &["thiserror"]),
        (HOST_LIBRARY, &["reqwest"]),
        ("reqwest", &[]),
        ("thiserror", &[]),
    ]);
    assert!(!severed.closure("catalog-build").contains(HOST_LIBRARY));
    assert_eq!(severed.path_to("catalog-build", HOST_LIBRARY), None);
}
