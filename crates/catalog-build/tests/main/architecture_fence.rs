//! **The product is a thin frontend over reusable client and runtime boundaries.**
//!
//! Design 02's 2026-08-15 amendment assigns the backend port to `service`, wire exchange to
//! `connectors-client`, composition to an isolated `connectors-runtime`, and provider/protocol
//! translation to focused Integration packages. These checks inspect manifests and source layout:
//! a prose boundary that can silently regain a forbidden dependency is not a boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MODULE_LINE_LIMIT: usize = 1_500;
const CLI_TOTAL_LINE_LIMIT: usize = 800;

/// Existing large catalog modules are named debts. The ceiling prevents a waiver from becoming
/// permission for unbounded growth; splitting below 1,500 lines must delete the waiver.
const MODULE_LINE_WAIVERS: &[(&str, usize, &str)] = &[
    (
        "crates/connector-spec/src/provider.rs",
        7_291,
        "legacy provider compiler pending grammar-focused extraction; +23 for the closed `audio_v1` \
         driver's lowering and its unary-only coherence rule, +27 for the closed `cdp_v1` driver's \
         lowering and its leased-session-only coherence rule",
    ),
    (
        "crates/connector-spec/src/ir.rs",
        2_717,
        "legacy compiler IR pending declaration-family extraction; +14 for the closed `audio_v1` \
         driver and its request variant, +16 for the closed `cdp_v1` driver and its request variant",
    ),
    (
        "crates/connector-secrets/src/file.rs",
        2_620,
        "cross-platform owner-bound store pending format and platform split",
    ),
    (
        "crates/integration-slack/src/backend.rs",
        2_360,
        "the first hosted companion slice keeps acquisition recovery, Socket Mode supervision, \
         event persistence, and outcome-aware audit together while their shared transaction \
         invariants settle; the final growth closes hosted-session expiry, and no further growth \
         is admitted before those arms split",
    ),
    (
        "crates/catalog-build/src/document.rs",
        2_568,
        "canonical lowering pending section-oriented extraction; +27 for the closed `audio_v1` \
         driver's request marker and schema branch, +21 for the closed `cdp_v1` driver's",
    ),
    (
        "crates/catalog-build/src/scaffold.rs",
        2_170,
        "scaffold workflow pending command-stage extraction",
    ),
    (
        "crates/catalog-build/src/site.rs",
        1_751,
        "site projection pending model and renderer split; the growth past 1,718 is in-flight site \
         work, plus 2 lines for the closed `audio_v1` protocol entry and 4 for the closed `cdp_v1` \
         one",
    ),
];

#[test]
fn service_owns_the_backend_port_and_server_only_adapts_transport() {
    let root = workspace_root();
    let service_sources = production_sources(&root.join("crates/service"));
    let server_sources = production_sources(&root.join("crates/server"));

    assert_source_contains(
        &service_sources,
        "pub trait ConnectorBackend",
        "`service` must publicly own the transport-neutral `ConnectorBackend` port",
    );
    for ownership in ["owns_operation", "owns_connection", "owns_event"] {
        assert_source_contains(
            &service_sources,
            &format!("fn {ownership}"),
            &format!("`ConnectorBackend` must expose exact `{ownership}` dispatch claims"),
        );
    }
    assert_source_excludes(
        &server_sources,
        "trait ConnectorBackend",
        "`server` must consume, not define, the backend port",
    );
    for application_module in ["mod authority", "mod dispatch", "mod sip", "mod voice"] {
        assert_source_excludes(
            &server_sources,
            application_module,
            "`server` is an inbound transport adapter, not an application-logic owner",
        );
    }

    let service = package_manifest(&root.join("crates/service"));
    let server = package_manifest(&root.join("crates/server"));
    assert!(
        !dependencies(&service).contains_key("server"),
        "`service` may not depend on its transport adapter"
    );
    assert!(
        dependencies(&server).contains_key("service"),
        "`server` must depend on the service-owned backend contract"
    );
    assert_forbidden_dependencies(
        "server",
        &server,
        &[
            "connector-secrets",
            "connectors-runtime",
            "voice-runtime",
            "driver-sip",
            "rtvbp-voice-endpoint",
        ],
    );
    for dependency in dependencies(&server).keys() {
        assert!(
            !dependency.starts_with("integration-"),
            "inbound transport `server` may not compose adapter `{dependency}`"
        );
    }
}

#[test]
fn reusable_client_has_no_runtime_or_backend_ownership() {
    let root = workspace_root();
    let client_path = root.join("crates/connectors-client");
    assert!(
        root_members(&root).contains("crates/connectors-client"),
        "`connectors-client` must be a canonical workspace library, not code hidden in the CLI"
    );

    let client = package_manifest(&client_path);
    assert_eq!(package_name(&client), "connectors-client");
    assert_forbidden_dependencies(
        "connectors-client",
        &client,
        &[
            "server",
            "service",
            "connectors-runtime",
            "connector-secrets",
            "voice-runtime",
            "driver-sip",
            "rtvbp-voice-endpoint",
            "kube",
            "k8s-openapi",
        ],
    );
    let sources = production_sources(&client_path);
    assert_source_excludes(
        &sources,
        "impl ConnectorBackend for",
        "the wire client must not implement a runtime backend",
    );
}

#[test]
fn runtime_is_the_only_adapter_composition_root() {
    let root = workspace_root();
    let runtime_path = root.join("crates/connectors-runtime");
    assert_nested_workspace(&root, "crates/connectors-runtime");

    let config = package_manifest(&root.join("crates/connectors-config"));
    assert_forbidden_dependencies(
        "connectors-config",
        &config,
        &["server", "connectors-runtime", "connectors-cli"],
    );

    let runtime = package_manifest(&runtime_path);
    assert_eq!(package_name(&runtime), "connectors-runtime");
    for dependency in ["server", "service"] {
        assert!(
            dependencies(&runtime).contains_key(dependency),
            "`connectors-runtime` must compose `{dependency}` explicitly"
        );
    }
    let runtime_sources = production_sources(&runtime_path);
    assert_source_contains(
        &runtime_sources,
        "pub struct BackendRegistry",
        "the runtime must expose one exact-dispatch `BackendRegistry`",
    );
    for ownership in ["owns_operation", "owns_connection", "owns_event"] {
        assert_source_contains(
            &runtime_sources,
            ownership,
            &format!("`BackendRegistry` must dispatch with `{ownership}` claims"),
        );
    }
    for collision_error in [
        "OperationErrorCode::Protocol",
        "ConnectionErrorCode::Protocol",
        "EventErrorCode::Protocol",
    ] {
        assert_source_contains(
            &runtime_sources,
            collision_error,
            "ambiguous backend ownership must fail as a typed protocol error",
        );
    }
    for inert_wrapper in ["CredentialStoreBackend", "_credential_store:"] {
        assert_source_excludes(
            &runtime_sources,
            inert_wrapper,
            "credential custody must be injected into its consuming adapter, not parked in a wrapper",
        );
    }
    for ordered_probe in [
        "Err(error) if error.code == OperationErrorCode::NotFound",
        "Err(error) if error.code == ConnectionErrorCode::NotFound",
        "Err(error) if error.code == EventErrorCode::NotFound",
    ] {
        assert_source_excludes(
            &runtime_sources,
            ordered_probe,
            "a backend `not_found` may not trigger ordered probing of the next backend",
        );
    }

    let adapter_manifests = integration_members(&root, &runtime);
    assert!(
        adapter_manifests.len() >= 2,
        "the runtime workspace must contain focused `integration-*` adapter packages, not one \
         omnibus adapter"
    );
    let runtime_dependencies = dependencies(&runtime);
    for (name, path, manifest) in adapter_manifests {
        assert!(
            runtime_dependencies.contains_key(&name),
            "the runtime must explicitly compose focused adapter `{name}`"
        );
        assert!(
            dependencies(&manifest).contains_key("service"),
            "focused adapter `{name}` must implement the service-owned port directly"
        );
        assert_forbidden_dependencies(
            &name,
            &manifest,
            &[
                "connectors-cli",
                "connectors-client",
                "connectors-runtime",
                "server",
            ],
        );
        for dependency in dependencies(&manifest).keys() {
            assert!(
                !dependency.starts_with("integration-"),
                "focused adapter `{name}` may not absorb sibling adapter `{dependency}`"
            );
        }
        assert_source_contains(
            &production_sources(&path),
            "ConnectorBackend for",
            &format!("focused adapter `{name}` must implement `ConnectorBackend`"),
        );
    }
}

#[test]
fn product_cli_is_a_thin_frontend() {
    let root = workspace_root();
    let cli_path = root.join("crates/connectors-cli");
    assert_nested_workspace(&root, "crates/connectors-cli");
    let cli = package_manifest(&cli_path);
    let cli_dependencies = dependencies(&cli);
    for required in ["connectors-client", "connectors-runtime"] {
        assert!(
            cli_dependencies.contains_key(required),
            "the product CLI must delegate to `{required}`"
        );
    }
    const CLI_DEPENDENCIES: &[&str] = &[
        "clap",
        "connectors-client",
        "connectors-runtime",
        "protocol",
        "rpassword",
        // A process must install exactly one rustls crypto provider before any TLS is attempted,
        // and only the binary can do that. `0c47258 runtime: select the AWS-LC crypto provider`
        // made that choice in the CLI and did not record it here; this line records the decision
        // already taken, and does not widen the frontend beyond process-level startup.
        "rustls",
        "serde",
        "serde_json",
        "thiserror",
        "tokio",
        "zeroize",
    ];
    for name in cli_dependencies.keys() {
        assert!(
            CLI_DEPENDENCIES.contains(&name.as_str()),
            "the product CLI has direct dependency `{name}`; its dependency surface is limited \
             to parsing/presentation plus `connectors-client` and `connectors-runtime`"
        );
    }

    let sources = production_sources(&cli_path);
    let total_lines: usize = sources.iter().map(|source| source_line_count(source)).sum();
    assert!(
        total_lines <= CLI_TOTAL_LINE_LIMIT,
        "the thin product CLI contains {total_lines} production lines; the reviewed cap is \
         {CLI_TOTAL_LINE_LIMIT}. Move client, runtime, or adapter behavior behind its owned package"
    );
    for forbidden in [
        "impl ConnectorBackend for",
        "struct CompositeBackend",
        "mod composite_backend",
        "mod hosted_vault",
        "mod kubernetes_backend",
        "mod monitoring_backend",
        "mod sip_backend",
        "mod slack_backend",
    ] {
        assert_source_excludes(
            &sources,
            forbidden,
            "the product CLI contains runtime composition or an Integration adapter",
        );
    }
}

#[test]
fn every_runtime_isolation_boundary_is_explicit_and_locked() {
    let root = workspace_root();
    for path in [
        "crates/connectors-cli",
        "crates/connectors-runtime",
        "crates/voice-runtime",
        "crates/driver-sip",
        "crates/rtvbp-voice-endpoint",
    ] {
        assert_nested_workspace(&root, path);
    }

    let cli = package_manifest(&root.join("crates/connectors-cli"));
    assert!(
        dependencies(&cli).contains_key("connectors-runtime"),
        "the release binary closure must point one way into the runtime composition workspace"
    );
    let runtime = package_manifest(&root.join("crates/connectors-runtime"));
    assert!(
        !dependencies(&runtime).contains_key("connectors-cli"),
        "the reusable runtime may not depend back on its product frontend"
    );
    let voice = package_manifest(&root.join("crates/voice-runtime"));
    for dependency in ["driver-sip", "rtvbp-voice-endpoint"] {
        assert!(
            dependencies(&voice).contains_key(dependency),
            "the voice runtime must retain the one-way `{dependency}` composition edge"
        );
    }
}

#[test]
fn production_modules_obey_the_named_size_fence() {
    let root = workspace_root();
    let mut waivers = BTreeMap::new();
    for &(path, ceiling, reason) in MODULE_LINE_WAIVERS {
        assert!(
            waivers.insert(path, (ceiling, reason)).is_none(),
            "duplicate module-size waiver for `{path}`"
        );
        assert!(
            !reason.trim().is_empty(),
            "module-size waiver `{path}` must state the debt being accepted"
        );
    }

    let mut used = BTreeSet::new();
    for source in production_sources(&root.join("crates")) {
        let relative = source
            .strip_prefix(&root)
            .expect("crate source is below workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        let lines = source_line_count(&source);
        if lines <= MODULE_LINE_LIMIT {
            assert!(
                !waivers.contains_key(relative.as_str()),
                "stale module-size waiver `{relative}`: it is now {lines} lines and must be removed"
            );
            continue;
        }

        let Some(&(ceiling, reason)) = waivers.get(relative.as_str()) else {
            panic!(
                "`{relative}` is {lines} lines, over the {MODULE_LINE_LIMIT}-line module cap; \
                 split it or add a reviewed named waiver with a growth ceiling and reason"
            );
        };
        assert!(
            lines <= ceiling,
            "`{relative}` grew to {lines} lines beyond its {ceiling}-line waiver ({reason})"
        );
        used.insert(relative);
    }

    for path in waivers.keys() {
        assert!(
            used.contains(*path),
            "module-size waiver `{path}` names no oversized production module"
        );
    }
}

fn integration_members(
    root: &Path,
    runtime_manifest: &toml::Value,
) -> Vec<(String, PathBuf, toml::Value)> {
    let runtime_workspace = runtime_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .expect("connectors-runtime `[workspace] members`");
    let runtime_path = root.join("crates/connectors-runtime");
    runtime_workspace
        .iter()
        .filter_map(toml::Value::as_str)
        .map(|member| {
            let unresolved = runtime_path.join(member);
            let path = std::fs::canonicalize(&unresolved)
                .unwrap_or_else(|error| panic!("resolve {}: {error}", unresolved.display()));
            let relative = path
                .strip_prefix(root)
                .unwrap_or_else(|_| panic!("{} is outside the repository", path.display()))
                .to_string_lossy()
                .replace('\\', "/");
            assert!(
                path.starts_with(&runtime_path) || root_excludes(root).contains(&relative),
                "runtime workspace member `{relative}` must be nested below the runtime or \
                 explicitly excluded from the canonical workspace"
            );
            assert!(
                !root_members(root).contains(&relative),
                "runtime workspace member `{relative}` may not also join the canonical workspace"
            );
            let manifest = package_manifest(&path);
            (package_name(&manifest).to_owned(), path, manifest)
        })
        .filter(|(name, _, _)| name.starts_with("integration-"))
        .collect()
}

fn assert_nested_workspace(root: &Path, relative: &str) {
    assert!(
        root_excludes(root).contains(relative),
        "`{relative}` must be explicitly excluded from the canonical workspace"
    );
    let path = root.join(relative);
    let manifest = package_manifest(&path);
    assert!(
        manifest.get("workspace").is_some(),
        "`{relative}` must declare its intentional nested workspace"
    );
    assert!(
        path.join("Cargo.lock").is_file(),
        "`{relative}` must lock its independently reviewed dependency closure"
    );
}

fn assert_forbidden_dependencies(owner: &str, manifest: &toml::Value, forbidden: &[&str]) {
    let dependencies = dependencies(manifest);
    for name in forbidden {
        assert!(
            !dependencies.contains_key(*name),
            "`{owner}` has forbidden direct dependency `{name}`"
        );
    }
}

fn assert_source_contains(sources: &[PathBuf], needle: &str, message: &str) {
    assert!(
        sources
            .iter()
            .any(|source| read_source(source).contains(needle)),
        "{message}"
    );
}

fn assert_source_excludes(sources: &[PathBuf], needle: &str, message: &str) {
    for source in sources {
        assert!(
            !read_source(source).contains(needle),
            "{message}: {} contains `{needle}`",
            source.display()
        );
    }
}

fn package_manifest(path: &Path) -> toml::Value {
    let manifest_path = path.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()));
    text.parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()))
}

fn package_name(manifest: &toml::Value) -> &str {
    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .expect("manifest `[package] name`")
}

fn dependencies(manifest: &toml::Value) -> BTreeMap<String, toml::Value> {
    manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .map(|dependencies| {
            dependencies
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn root_members(root: &Path) -> BTreeSet<String> {
    root_workspace_paths(root, "members")
}

fn root_excludes(root: &Path) -> BTreeSet<String> {
    root_workspace_paths(root, "exclude")
}

fn root_workspace_paths(root: &Path, key: &str) -> BTreeSet<String> {
    package_manifest(root)
        .get("workspace")
        .and_then(|workspace| workspace.get(key))
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("root `[workspace] {key}`"))
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn production_sources(root: &Path) -> Vec<PathBuf> {
    fn visit(path: &Path, sources: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                let name = path.file_name().and_then(|name| name.to_str());
                if !matches!(name, Some("target" | "tests" | "examples" | "benches")) {
                    visit(&path, sources);
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
                && path
                    .components()
                    .any(|component| component.as_os_str() == "src")
            {
                sources.push(path);
            }
        }
    }

    let mut sources = Vec::new();
    visit(root, &mut sources);
    sources.sort();
    sources
}

fn read_source(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn source_line_count(path: &Path) -> usize {
    read_source(path).lines().count()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the workspace root")
        .to_path_buf()
}
