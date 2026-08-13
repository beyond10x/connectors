//! **No engine crate anywhere in this workspace.** This is the hard constraint M1 exists to prove.
//!
//! The predecessor compiled connectors to Flux and linked `codewandler-flux-*` from four crates. A
//! consumer that wanted nothing but a connector's *request* resolved an engine line to get it, and
//! moved when that line moved even though no catalogue content changed. Design 02 §2 states the
//! catalog family as "engine-free by construction"; this file is what makes that structural rather
//! than a sentence in a manifest.
//!
//! # Three assertions, weakest to strongest
//!
//! 1. [`no_manifest_in_this_workspace_requires_an_engine_crate`] — a text audit of every
//!    `Cargo.toml`, which catches an edge somebody wrote before it is ever resolved.
//! 2. [`the_lockfile_names_no_engine_crate_at_all`] — over `Cargo.lock`, which records **every**
//!    resolved edge including dev-dependencies and optional ones. This is the strongest form of the
//!    constraint: not a dependency, not a dev-dependency, not behind a feature.
//! 3. [`no_workspace_member_reaches_an_engine_crate`] — over the graph cargo itself resolves, per
//!    member, so a failure names the member and the chain to break.
//!
//! # Why the lockfile is enough here, when the predecessor needed `cargo metadata`
//!
//! The predecessor's version of this fence had to read `cargo metadata --locked --offline` and
//! filter dependency kinds, because its own `catalog` crate took `flux-lang` as a *dev*-dependency
//! and a lock-based walk therefore reported the plan-deriving core as engine-coupled. No consumer
//! resolved that edge, so the lock was the wrong instrument for that question.
//!
//! Here the question is stronger and the instrument is simpler: **no engine crate may be in the
//! lock at all.** That dev-dependency is one of the things this migration dropped. The
//! feature-resolved walk is kept anyway, because a per-member failure that names the chain is worth
//! more than a single "it is in the lock somewhere".
//!
//! `--locked` and `--offline` are both deliberate: this must describe the committed lockfile, must
//! not be able to change it, and a test in this repository reaching the network would be its own
//! defect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// **The engine line, by package prefix.**
///
/// Every crate flux publishes carries it, so the fence is stated over the prefix rather than over a
/// list — a list is wrong the moment flux adds a crate, and wrong *silently*, which is the failure
/// mode a fence must not have.
///
/// `codewandler-flux-spec` is deliberately inside the prefix even though it moves on its own `1.x`
/// line: it is the frozen wire vocabulary a guest plugin compiles against, and an artifact that
/// named a `flux_spec::ToolSpec` would still be one a consumer needs flux's vocabulary to read. The
/// whole point of the split is that every artifact here is **data**.
const ENGINE_PREFIX: &str = "codewandler-flux-";

/// The bare-name prefix, for the manifest audit: a requirement is written under its package name,
/// which may be spelled with or without the vanity prefix depending on whether the manifest uses a
/// `package = "…"` alias.
const ENGINE_NAMES: &[&str] = &[
    "codewandler-flux",
    "flux-lang",
    "flux-core",
    "flux-flow",
    "flux-runtime",
    "flux-spec",
    "flux-web",
    "flux-system",
    "flux-credentials",
    "flux-provider",
    "flux-plugin",
];

/// **No workspace member reaches an engine crate**, over the graph cargo resolves.
///
/// Per member rather than over one crate, which is the strengthening design 02 §7 asks for: the
/// predecessor fenced its plan-deriving core alone, because four of its other crates were on the
/// engine line on purpose. Here every member answers the same question.
#[test]
fn no_workspace_member_reaches_an_engine_crate() {
    let graph = Resolved::consumer_graph();
    let members = workspace_members();
    assert!(
        !members.is_empty(),
        "no workspace members were read, so this asserted nothing"
    );

    for member in &members {
        assert!(
            graph.contains(member),
            "`{member}` is a workspace member but not a package in {}",
            graph.source
        );
        let reached: Vec<String> = graph
            .closure(member)
            .into_iter()
            .filter(|name| name.starts_with(ENGINE_PREFIX))
            .collect();
        assert!(
            reached.is_empty(),
            "`{member}` reaches the engine: {reached:?}\n\
             Design 02 §2 states this workspace as engine-free by construction. The edge to break \
             is: {}",
            reached
                .first()
                .and_then(|target| graph.path_to(member, target))
                .unwrap_or_else(|| "<no path found>".to_owned())
        );
    }
}

/// **The strongest form: no engine crate is in `Cargo.lock` at all** — not as a dependency, not as
/// a dev-dependency, not behind an off-by-default feature.
///
/// The lock records the resolved graph including edges no build takes, which for every other
/// question in this repository makes it the wrong instrument. For *this* question it is exactly
/// right: "the engine does not enter the workspace" is a claim about the resolved set, and a
/// dev-dependency is how the predecessor's catalogue crate kept `flux-lang` in the tree.
#[test]
fn the_lockfile_names_no_engine_crate_at_all() {
    let root = workspace_root();
    let path = root.join("Cargo.lock");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let document: toml::Value = text
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));

    let mut resolved = 0usize;
    let mut engine = Vec::new();
    for entry in document
        .get("package")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{} has no `[[package]]` entries", path.display()))
    {
        let Some(name) = entry.get("name").and_then(toml::Value::as_str) else {
            continue;
        };
        resolved += 1;
        if name.starts_with(ENGINE_PREFIX) {
            engine.push(name.to_owned());
        }
    }

    // A fence over an empty lock would pass forever while asserting nothing.
    assert!(
        resolved > 50,
        "{} resolved only {resolved} packages, which is not this workspace's graph",
        path.display()
    );
    assert!(
        engine.is_empty(),
        "{} resolves engine crates: {engine:?}\n\
         No `{ENGINE_PREFIX}*` crate may enter this workspace, in any dependency kind. This is the \
         constraint the whole M1 migration exists to prove.",
        path.display()
    );
}

/// **The audit of what somebody wrote**, before cargo ever resolves it.
///
/// The two tests above read resolved state. This reads the manifests, so an engine requirement is
/// caught in the diff that adds it rather than in the lock update that follows — and it catches the
/// spelling a `package = "…"` alias would hide from a name-based walk.
#[test]
fn no_manifest_in_this_workspace_requires_an_engine_crate() {
    let root = workspace_root();
    let mut manifests = vec![root.join("Cargo.toml")];
    manifests.extend(
        workspace_members()
            .iter()
            .map(|member| root.join("crates").join(member).join("Cargo.toml")),
    );

    let mut offences = Vec::new();
    for manifest in &manifests {
        let text = std::fs::read_to_string(manifest)
            .unwrap_or_else(|error| panic!("read {}: {error}", manifest.display()));
        for (number, line) in text.lines().enumerate() {
            // Prose names the engine on purpose: every one of these manifests explains what was
            // left behind, and a comment saying "no flux crate here" must not trip the fence.
            let code = line.trim_start();
            if code.starts_with('#') {
                continue;
            }
            for name in ENGINE_NAMES {
                if code.contains(name) {
                    offences.push(format!(
                        "{}:{}: {}",
                        manifest.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "an engine requirement is written in a manifest:\n{}",
        offences.join("\n")
    );
}

/// Every workspace member's directory name under `crates/`, which is also its package name here.
fn workspace_members() -> Vec<String> {
    let root = workspace_root();
    let path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let document: toml::Value = text
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    document
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .expect("`[workspace] members`")
        .iter()
        .filter_map(toml::Value::as_str)
        .filter_map(|member| member.strip_prefix("crates/"))
        .map(str::to_owned)
        .collect()
}

/// **The control, and the walk itself.**
///
/// A fence that finds no engine has said one of two things, and they are not the same: *there is no
/// engine here*, or *this walk cannot see an engine at all*. There is no engine crate anywhere in
/// this workspace by construction, so the control cannot be a real edge — it is a graph stated
/// directly, carrying exactly the shape the predecessor had.
#[test]
fn the_walk_finds_an_engine_two_edges_away_and_stops_at_a_severed_one() {
    const CORE: &str = "connector-resolve";

    let coupled = Resolved::over(&[
        (CORE, &["catalog"]),
        ("catalog", &["codewandler-flux-lang"]),
        ("codewandler-flux-lang", &[]),
    ]);
    assert!(coupled
        .closure(CORE)
        .iter()
        .any(|name| name.starts_with(ENGINE_PREFIX)));
    assert_eq!(
        coupled.path_to(CORE, "codewandler-flux-lang").as_deref(),
        Some("connector-resolve -> catalog -> codewandler-flux-lang")
    );

    // The shape this workspace actually has: `catalog` is a shim over the pack reader, and the
    // engine edge the predecessor carried is gone, so the same walk reports nothing.
    let severed = Resolved::over(&[
        (CORE, &["catalog"]),
        ("catalog", &["catalog-reader"]),
        ("catalog-reader", &[]),
    ]);
    assert!(!severed
        .closure(CORE)
        .iter()
        .any(|name| name.starts_with(ENGINE_PREFIX)));
    assert_eq!(severed.path_to(CORE, "codewandler-flux-lang"), None);
}

/// The dependency graph cargo resolved, with features applied and only the edges a **consumer**
/// links.
struct Resolved {
    /// Where it came from, for a failure message.
    source: String,
    /// Package name to the names of its normal dependencies.
    packages: BTreeMap<String, Vec<String>>,
}

impl Resolved {
    /// Ask cargo for the resolved graph, keeping `[dependencies]` edges only.
    ///
    /// Package *names* collapse versions, which is the same simplification `dependency_fence.rs`
    /// makes: for a fence keyed on "is this crate here at all" the version is not the question.
    ///
    /// # Panics
    ///
    /// If cargo cannot be run, exits non-zero, or emits something that is not the metadata
    /// document — in a test each of those is the assertion failing, not a condition to recover from.
    fn consumer_graph() -> Self {
        let root = workspace_root();
        let output = Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1", "--locked", "--offline"])
            .current_dir(&root)
            .output()
            .unwrap_or_else(|error| panic!("run `cargo metadata` in {}: {error}", root.display()));
        assert!(
            output.status.success(),
            "`cargo metadata` failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr)
        );

        let document: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("`cargo metadata` emits JSON");

        // Package ids are opaque; the names live on the package entries.
        let mut name_of: BTreeMap<&str, &str> = BTreeMap::new();
        for package in document["packages"]
            .as_array()
            .expect("`packages` is an array")
        {
            let (Some(id), Some(name)) = (package["id"].as_str(), package["name"].as_str()) else {
                continue;
            };
            name_of.insert(id, name);
        }

        let mut packages: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in document["resolve"]["nodes"]
            .as_array()
            .expect("`resolve.nodes` is an array — `--no-deps` was not passed")
        {
            let Some(name) = node["id"].as_str().and_then(|id| name_of.get(id)) else {
                continue;
            };
            let mut dependencies = Vec::new();
            for dep in node["deps"].as_array().into_iter().flatten() {
                // `cargo metadata` spells a normal dependency as a null `kind`; `dev` and `build`
                // name themselves. No `dep_kinds` at all is an older metadata format, treated as
                // normal rather than silently dropping the edge.
                let normal = dep["dep_kinds"]
                    .as_array()
                    .map(|entries| entries.iter().any(|entry| entry["kind"].is_null()))
                    .unwrap_or(true);
                if !normal {
                    continue;
                }
                if let Some(dependency) = dep["pkg"].as_str().and_then(|id| name_of.get(id)) {
                    dependencies.push((*dependency).to_owned());
                }
            }
            packages.insert((*name).to_owned(), dependencies);
        }

        Self {
            source: format!("the feature-resolved consumer graph of {}", root.display()),
            packages,
        }
    }

    /// A graph stated directly, for asserting the walk itself.
    fn over(edges: &[(&str, &[&str])]) -> Self {
        Self {
            source: "<synthetic>".to_owned(),
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

    /// A rendered `a -> b -> c` chain from `root` to `target`, breadth first, so the reported chain
    /// is a shortest one and names the edge worth deleting.
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
