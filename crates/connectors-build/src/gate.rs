use super::Result;
use std::{path::Path, process::Command, sync::OnceLock};

/// The host triple from `rustc -vV`, read once per process.
fn host_triple() -> Result<&'static str> {
    static HOST: OnceLock<std::result::Result<String, String>> = OnceLock::new();
    let host = HOST.get_or_init(|| {
        let output = Command::new("rustc")
            .arg("-vV")
            .output()
            .map_err(|error| format!("rustc -vV: {error}"))?;
        if !output.status.success() {
            return Err(format!("rustc -vV failed: {}", output.status));
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .map(str::to_owned)
            .ok_or_else(|| "rustc -vV names no host".to_owned())
    });
    host.as_deref().map_err(|error| error.clone().into())
}

/// Cargo as the gate runs it. Test binaries get the task-owned temporary root as `TMPDIR`
/// through a runner. Cargo itself, and so the compiler wrapper it starts, keeps the caller's
/// `TMPDIR`: sccache binds its startup socket there, and the checkout-local root of a long
/// checkout pushes that path past `SUN_LEN`. The runner passes the root as the shell's `$0`
/// and the binary and its arguments as `"$@"`, so no path is parsed as an assignment or as
/// shell text; it is keyed to the host triple so it outranks a user's host runner.
fn cargo(root: &Path, temp: &Path, ess: &Path, toolchain: Option<&str>) -> Result<Command> {
    let temp = temp
        .to_str()
        .ok_or("gate temporary root is not valid UTF-8")?;
    let runner = serde_json::to_string(&["sh", "-c", r#"export TMPDIR="$0"; exec "$@""#, temp])?;
    let host = serde_json::to_string(host_triple()?)?;
    let mut cmd = Command::new("cargo");
    if let Some(toolchain) = toolchain {
        cmd.arg(format!("+{toolchain}"));
    }
    cmd.current_dir(root)
        .arg("--config")
        .arg(format!("target.{host}.runner={runner}"))
        .env("CARGO_BUILD_JOBS", "2")
        .env("CONNECTORS_ESS", ess);
    Ok(cmd)
}

/// The gate step that fails when a page the documentation site generates (CLI, crates, contract
/// and model reference, status) differs from what its sources generate now.
fn docs_check(root: &Path, temp: &Path, ess: &Path) -> Result<Command> {
    let mut cmd = cargo(root, temp, ess, None)?;
    cmd.args([
        "run",
        "--locked",
        "--offline",
        "-q",
        "-p",
        "connectors-docs",
        "--",
        "generate",
        "--check",
        "--ess",
    ])
    .arg(ess);
    Ok(cmd)
}

pub fn run(root: &Path, ess: &Path, aep: &Path, msrv: bool) -> Result<()> {
    connectors_spec::v2::check_ess(ess)?;
    super::metadata_entities::run(root, true)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new().prefix("gate-").tempdir_in(base)?;
    super::ess_boundary::run(root, ess, temp.path())?;
    super::source_hashes::run(root)?;
    super::upstream_redaction::run(root)?;
    super::release_binaries::run(root)?;
    super::cli::run(root, ess, true)?;
    // The website build's examples step, without its WASM build: a target refusing the
    // example model fails here rather than only in `npm run build`.
    super::docs::synthesize_examples(root, ess, &temp.path().join("examples"))?;
    println!("gate: website example model synthesizes for rust and web; exit=0");
    let command = |toolchain: Option<&str>| cargo(root, temp.path(), ess, toolchain);
    let execute = |cmd: &mut Command| -> Result<()> {
        println!("gate: {:?}", cmd.get_args().collect::<Vec<_>>());
        let status = cmd.status()?;
        println!(
            "gate: exit={status} {:?}",
            cmd.get_args().collect::<Vec<_>>()
        );
        if !status.success() {
            return Err(format!("gate command {:?} failed", cmd.get_program()).into());
        }
        Ok(())
    };
    // `fmt --all` follows excluded path dependencies too. Only authored workspace
    // members belong to rustfmt; generated dependencies retain their pinned bytes.
    let metadata: serde_json::Value = serde_json::from_slice(
        &super::run(command(None)?.args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ]))?
        .stdout,
    )?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("missing Cargo workspace members")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing Cargo packages")?;
    let mut format = command(None)?;
    format.arg("fmt");
    let mut selected = 0;
    for package in packages.iter().filter(|p| members.contains(&p["id"])) {
        format.args([
            "--package",
            package["name"]
                .as_str()
                .ok_or("missing Cargo package name")?,
        ]);
        selected += 1;
    }
    if selected == 0 || selected != members.len() {
        return Err("incomplete Cargo formatting selection".into());
    }
    execute(format.args(["--", "--check"]))?;
    for adapter in [
        "grafana",
        "kubernetes",
        "loki",
        "prometheus",
        "sql",
        "tavily",
    ] {
        let source = root.join(format!("adapters/{adapter}/spec/adapter.json"));
        let descriptor = connectors_spec::compile(&std::fs::read(source)?)?;
        if connectors_spec::descriptor_bytes(&descriptor)?
            != std::fs::read(root.join(format!("adapters/{adapter}/generated/descriptor.json")))?
        {
            return Err(format!("{adapter}: generated descriptor drift").into());
        }
        println!("gate: {adapter} descriptor matches; exit=0");
    }
    // Generation fixtures verify the full pinned bundle, reproducibility and refusals.
    execute(command(None)?.args(["build", "--workspace", "--locked", "--offline"]))?;
    execute(command(None)?.args(["test", "--workspace", "--locked", "--offline"]))?;
    execute(command(None)?.args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--offline",
        "--",
        "-D",
        "warnings",
    ]))?;
    execute(&mut docs_check(root, temp.path(), ess)?)?;
    for adapter in [
        "grafana",
        "kubernetes",
        "loki",
        "prometheus",
        "sql",
        "tavily",
        "catalog-provider",
    ] {
        let package = format!("connectors-{adapter}");
        execute(command(None)?.args([
            "build",
            "--locked",
            "--offline",
            "-p",
            &package,
            "--lib",
            "--no-default-features",
        ]))?;
        let output = super::run(command(None)?.args([
            "tree",
            "--locked",
            "--offline",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "--no-default-features",
            "-p",
            &package,
        ]))?;
        let tree = String::from_utf8(output.stdout)?;
        for name in [
            "connectors-host",
            "connectors-client",
            "connectors-grafana",
            "connectors-kubernetes",
            "connectors-loki",
            "connectors-prometheus",
            "connectors-sql",
            "connectors-tavily",
        ] {
            if name != package
                && tree
                    .lines()
                    .any(|line| line.split_whitespace().next() == Some(name))
            {
                return Err(format!("{package} library depends on forbidden {name}").into());
            }
        }
        println!("gate: {package} library boundary holds; exit=0");
    }
    let output = super::run(command(None)?.args([
        "tree",
        "--locked",
        "--offline",
        "--edges",
        "normal",
        "--prefix",
        "none",
        "-p",
        "connectors",
    ]))?;
    let tree = String::from_utf8(output.stdout)?;
    if tree.lines().any(|line| {
        matches!(
            line.split_whitespace().next(),
            Some(
                "connectors-grafana"
                    | "connectors-kubernetes"
                    | "connectors-loki"
                    | "connectors-prometheus"
                    | "connectors-sql"
                    | "connectors-tavily"
                    | "tokio-postgres"
            )
        )
    }) {
        return Err("generic CLI depends on a provider".into());
    }
    println!("gate: generic CLI boundary holds; exit=0");
    if msrv {
        // Keep compiler metadata separate. The Eventlog-backed host graph needs
        // Rust 1.91; libraries which do not select that graph retain 1.88.
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("target"));
        for package in [
            "connectors-catalog",
            "connectors-client",
            "connectors-contracts",
            "connectors-core",
            "connectors-sdk",
        ] {
            execute(
                command(Some("1.88.0"))?
                    .args([
                        "check",
                        "--package",
                        package,
                        "--all-targets",
                        "--locked",
                        "--offline",
                    ])
                    .env("CARGO_TARGET_DIR", target.join("msrv-1.88")),
            )?;
        }
        for package in [
            "connectors-catalog-provider",
            "connectors-grafana",
            "connectors-kubernetes",
            "connectors-loki",
            "connectors-prometheus",
            "connectors-sql",
            "connectors-tavily",
        ] {
            execute(
                command(Some("1.88.0"))?
                    .args([
                        "check",
                        "--package",
                        package,
                        "--lib",
                        "--no-default-features",
                        "--locked",
                        "--offline",
                    ])
                    .env("CARGO_TARGET_DIR", target.join("msrv-1.88")),
            )?;
        }
        execute(
            command(Some("1.91.0"))?
                .args([
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--offline",
                ])
                .env("CARGO_TARGET_DIR", target.join("msrv-1.91")),
        )?;
    }
    authored_scenarios(root, ess, temp.path())?;
    // Execute the local metadata authority's synthesized scenarios against the lowered
    // definitions the host runs, not only the model.
    let metadata_conformance = temp.path().join("metadata-conformance");
    std::fs::create_dir_all(&metadata_conformance)?;
    super::metadata_conformance::gate(root, &metadata_conformance)?;
    let mut planning = Command::new(aep);
    planning.current_dir(root).env("TMPDIR", temp.path());
    execute(planning.args(["plan", "artifact", "validate"]))?;
    println!("gate: all checks passed");
    Ok(())
}

/// The authored scenario directories the gate compiles and executes.
const AUTHORED_DIRECTORIES: &[&str] = &[
    "contracts/operations/v1alpha1/scenarios",
    "contracts/auth/acquisition/v1alpha1/scenarios",
    "contracts/auth/evidence/v1alpha1/scenarios",
    "contracts/auth/connection/v1alpha1/scenarios",
    "contracts/service/v1alpha1/scenarios",
    // Inbound only. These are `ess-scenario/1` and ESS owns them, so synthesising them
    // against the shared IR is a real check.
    //
    // The outbound directory beside it is deliberately absent. Those files are
    // `mcp-outbound-lifecycle/1`, a format this repository defined and ESS does not
    // know — and ESS refuses an unknown key rather than ignoring it, so collecting
    // them here fails the gate on every one of them. They are held by
    // `mcp_outbound_connection_lifecycle`'s own package-scoped cases instead, which is
    // a weaker guarantee than the shared IR gives the inbound set, and saying so here
    // is the point: adding the directory would turn a stated gap into a broken gate.
    "adapters/mcp/contracts/server/v1alpha1/scenarios",
];

/// Compiles the generated obligations with the authored traces of [`AUTHORED_DIRECTORIES`],
/// then executes the authored traces against the model with the pinned ESS interpreter.
fn authored_scenarios(root: &Path, ess: &Path, temp: &Path) -> Result<()> {
    let checked = |cmd: &mut Command| -> Result<()> {
        println!("gate: {:?}", cmd.get_args().collect::<Vec<_>>());
        let status = cmd.status()?;
        println!(
            "gate: exit={status} {:?}",
            cmd.get_args().collect::<Vec<_>>()
        );
        if !status.success() {
            return Err(format!("gate command {:?} failed", cmd.get_program()).into());
        }
        Ok(())
    };
    // The pinned ESS authoring reader is non-recursive. Collect each explicitly
    // registered directory with a unique prefix so none silently disappears.
    let scenarios = temp.join("authored-scenarios");
    std::fs::create_dir(&scenarios)?;
    let mut collected = 0;
    for (index, directory) in AUTHORED_DIRECTORIES.iter().enumerate() {
        let mut files = std::fs::read_dir(root.join(directory))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        files.sort();
        let mut count = 0;
        for source in files {
            if source.is_file()
                && matches!(
                    source.extension().and_then(|ext| ext.to_str()),
                    Some("yaml" | "yml")
                )
            {
                let name = source.file_name().ok_or("scenario has no filename")?;
                std::fs::copy(
                    &source,
                    scenarios.join(format!("{index}-{}", name.to_string_lossy())),
                )?;
                count += 1;
            }
        }
        if count == 0 {
            return Err(format!("{directory}: no authored scenarios").into());
        }
        println!("gate: collected {count} authored scenarios from {directory}; exit=0");
        collected += count;
    }
    // Compile generated obligations and authored traces. This checks the model.
    checked(
        Command::new(ess)
            .current_dir(root)
            .env("TMPDIR", temp)
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "ess",
                "--target",
                "ir",
                "--scenarios",
            ])
            .arg(&scenarios)
            .arg("--out")
            .arg(temp.join("contract-conformance.json")),
    )?;
    // Execute the authored traces, not only compile them. Compilation resolves their names;
    // only execution holds a timeline to the model, so a lifecycle the model contradicts (a
    // move from a state it does not admit) passes the step above and fails here. `author`
    // compiles the authored scenarios alone and refuses one it cannot read; the pinned ESS
    // interpreter then runs exactly that suite against the model.
    let authored = temp.join("authored-suite.json");
    checked(
        Command::new(ess)
            .current_dir(root)
            .env("TMPDIR", temp)
            .args([
                "verify",
                "conform",
                "author",
                "--path",
                "ess",
                "--scenarios",
            ])
            .arg(&scenarios)
            .arg("--out")
            .arg(&authored),
    )?;
    let report = temp.join("authored-report.json");
    // The run exits non-zero while a known failure remains; the report decides.
    let status = Command::new(ess)
        .current_dir(root)
        .env("TMPDIR", temp)
        .args([
            "verify",
            "conform",
            "run",
            "--target",
            "interpreted",
            "--path",
            "ess",
        ])
        .arg("--suite")
        .arg(&authored)
        .args(["--report-format", "2", "--report-out"])
        .arg(&report)
        .status()?;
    let outcomes: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&report)
            .map_err(|error| format!("authored execution wrote no report ({status}): {error}"))?,
    )?;
    let passed = authored_execution(&outcomes["outcomes"], collected)?;
    println!(
        "gate: executed {collected} authored scenarios, {passed} passed, {} known failing; exit=0",
        collected - passed
    );
    Ok(())
}

/// Authored scenarios that fail when the pinned ESS 0.59.0 interpreter executes them, measured
/// 2026-10-16 when the gate first executed authored traces. Each needs its owner's triage; none
/// is waived silently. Most claim an error field this ess/18 model never fills (`state`,
/// `reason`: an error's fields need payload sources, ess/19); the mutations traces also
/// expect outcomes the model answers differently (`subject-missing`, `wrong-state`). A listed
/// scenario that starts passing fails the gate too, so the list cannot outlive its cause.
const AUTHORED_KNOWN_FAILING: &[&str] = &[
    "connectors.credential_evidence/authored/configured-replacement-identity",
    "connectors.credential_evidence/authored/retained-lineage-expires-before-dispatch",
    "connectors.credential_evidence/authored/revocation-before-dispatch",
    "connectors.credential_evidence/authored/rotation-before-dispatch",
    "connectors.credential_evidence/authored/same-identity-refresh",
    "connectors.credential_evidence/authored/unvalidated-configured-replacement",
    "connectors.idempotency/authored/idempotency-known-result-expiry",
    "connectors.idempotency/authored/idempotency-namespace-values",
    "connectors.idempotency/authored/idempotency-pending-no-expiry",
    "connectors.idempotency/authored/idempotency-unknown-no-expiry",
    "connectors.mutations/authored/deadline-after-gate",
    "connectors.mutations/authored/deadline-before-gate",
    "connectors.mutations/authored/definitive-refusal",
    "connectors.mutations/authored/definitive-success",
    "connectors.mutations/authored/indeterminate-is-terminal",
    "connectors.mutations/authored/recovery-after-gate",
    "connectors.mutations/authored/recovery-before-gate",
    "connectors.refresh/authored/authorization-wins-fence-race",
    "connectors.refresh/authored/authorized-owner-loss-no-second-exchange",
    "connectors.refresh/authored/binding-replacement-before-publication",
    "connectors.refresh/authored/durable-response-wins-owner-loss-race",
    "connectors.refresh/authored/expired-candidate-before-publication",
    "connectors.refresh/authored/late-response-after-quarantine",
    "connectors.refresh/authored/published-reply-loss-no-reexchange",
    "connectors.refresh/authored/recovery-owner-loss-never-reopens-source",
    "connectors.refresh/authored/reserved-owner-fenced-before-successor",
    "connectors.refresh/authored/revocation-before-authorization",
    "connectors.refresh/authored/revocation-before-publication",
    "connectors.refresh/authored/stored-response-recovery-rejects-stale-publisher",
    "connectors.refresh/authored/two-replicas-one-authorization",
    "connectors.sessions/authored/graceful-stdin-close-ends-the-session",
    "connectors.sessions/authored/partial-result-never-reported-complete",
    "connectors.sessions/authored/transport-drop-loses-the-session",
    "connectors.sessions/authored/unsupported-version-denies-readiness",
];

/// Holds an `ess-conformance-run/2` report's `outcomes` to the authored scenarios the gate
/// collected: every one ran, every one not in [`AUTHORED_KNOWN_FAILING`] passed, and every
/// listed one still failed. Returns the number that passed.
fn authored_execution(outcomes: &serde_json::Value, collected: usize) -> Result<usize> {
    let names = |status: &str| -> Result<Vec<String>> {
        outcomes[status]
            .as_array()
            .ok_or_else(|| format!("authored execution report has no `{status}` list"))?
            .iter()
            .map(|name| {
                name.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "authored execution report names a non-string".into())
            })
            .collect()
    };
    let passed = names("passed")?;
    let failed = names("failed")?;
    let mut ran = passed.len() + failed.len();
    let mut problems = Vec::new();
    for status in ["error", "unsupported", "skipped"] {
        for name in names(status)? {
            ran += 1;
            problems.push(format!("{status}: {name}"));
        }
    }
    for name in &failed {
        if !AUTHORED_KNOWN_FAILING.contains(&name.as_str()) {
            problems.push(format!("failed: {name}"));
        }
    }
    for name in AUTHORED_KNOWN_FAILING {
        if passed.iter().any(|passed| passed == name) {
            problems.push(format!("passes but is listed as known failing: {name}"));
        } else if !failed.iter().any(|failed| failed == name) {
            problems.push(format!("listed as known failing but did not run: {name}"));
        }
    }
    if ran != collected {
        problems.push(format!(
            "{collected} authored scenarios collected, {ran} executed"
        ));
    }
    if problems.is_empty() {
        Ok(passed.len())
    } else {
        Err(format!("authored scenario execution: {}", problems.join("; ")).into())
    }
}

#[cfg(test)]
mod authored_execution_tests {
    use super::{AUTHORED_KNOWN_FAILING, authored_execution, authored_scenarios};
    use serde_json::json;
    use std::path::Path;

    /// The gate's own lane over the repository's model and scenarios, with the pinned ESS: a
    /// lifecycle mutant of `ess/` (`mark_stale` from Stale, `publish` or `revoke` not from
    /// Stale) fails here.
    #[test]
    fn the_registered_authored_scenarios_execute_against_the_model() {
        let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let temp = tempfile::tempdir().unwrap();
        authored_scenarios(&root, &ess, temp.path()).unwrap();
    }

    fn report(passed: &[&str], failed: &[&str]) -> serde_json::Value {
        json!({"passed": passed, "failed": failed, "error": [], "unsupported": [], "skipped": []})
    }

    #[test]
    fn every_unlisted_scenario_must_pass() {
        let failed: Vec<&str> = AUTHORED_KNOWN_FAILING
            .iter()
            .copied()
            .chain(["connectors.auth_bindings/authored/stale"])
            .collect();
        let error = authored_execution(&report(&[], &failed), failed.len()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("failed: connectors.auth_bindings/authored/stale")
        );
    }

    #[test]
    fn a_listed_scenario_that_passes_fails_the_gate() {
        let error = authored_execution(
            &report(&AUTHORED_KNOWN_FAILING[..1], &AUTHORED_KNOWN_FAILING[1..]),
            AUTHORED_KNOWN_FAILING.len(),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("passes but is listed as known failing")
        );
    }

    #[test]
    fn a_scenario_that_did_not_run_fails_the_gate() {
        let error = authored_execution(
            &report(
                &["connectors.declarations/authored/a"],
                AUTHORED_KNOWN_FAILING,
            ),
            AUTHORED_KNOWN_FAILING.len() + 2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("collected"));
    }

    #[test]
    fn known_failures_and_passes_hold() {
        let passed = ["connectors.declarations/authored/a"];
        assert_eq!(
            authored_execution(
                &report(&passed, AUTHORED_KNOWN_FAILING),
                AUTHORED_KNOWN_FAILING.len() + 1
            )
            .unwrap(),
            1
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{cargo, docs_check};
    use std::{ffi::OsStr, path::Path};

    const ROOT: &str = "/checkout/.local/tmp/gate-AbCdEf";

    fn build(toolchain: Option<&str>) -> std::process::Command {
        cargo(
            Path::new("/checkout"),
            Path::new(ROOT),
            Path::new("/ess"),
            toolchain,
        )
        .expect("gate cargo command")
    }

    /// The compiler wrapper cargo starts (sccache) inherits cargo's own `TMPDIR` and puts
    /// its startup socket there, so the gate's long root must not be cargo's `TMPDIR`.
    #[test]
    fn gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir() {
        let command = build(None);
        assert!(
            command
                .get_envs()
                .all(|(name, _)| name != OsStr::new("TMPDIR")),
            "gate cargo sets or clears TMPDIR: {:?}",
            command.get_envs().collect::<Vec<_>>()
        );
    }

    /// The host triple as rustc reports it, read independently of the gate.
    fn rustc_host() -> String {
        let output = std::process::Command::new("rustc")
            .arg("-vV")
            .output()
            .expect("rustc -vV");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .expect("rustc -vV names its host")
            .to_owned()
    }

    /// Test binaries still run under the gate's task-owned root, through a runner that
    /// cargo applies to target executables and never to the compiler. The root is the
    /// shell's `$0`, never parsed as an assignment or as shell text.
    #[test]
    fn gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner() {
        let args: Vec<_> = build(None).get_args().map(OsStr::to_owned).collect();
        let host = rustc_host();
        let expected = format!(
            r#"target."{host}".runner=["sh","-c","export TMPDIR=\"$0\"; exec \"$@\"","{ROOT}"]"#
        );
        assert_eq!(
            args,
            [OsStr::new("--config"), OsStr::new(&expected)],
            "gate cargo arguments"
        );
    }

    /// The runner is keyed to the host triple, so it outranks a user's
    /// `[target.<triple>] runner` and `CARGO_TARGET_<TRIPLE>_RUNNER`; a `cfg(...)` key
    /// would lose to either.
    #[test]
    fn gate_cargo_keys_the_runner_to_the_host_triple() {
        let host = rustc_host();
        assert!(
            host.starts_with(std::env::consts::ARCH) && host.contains(std::env::consts::OS),
            "rustc host {host} is not this machine"
        );
        let command = build(None);
        let config = command
            .get_args()
            .nth(1)
            .and_then(OsStr::to_str)
            .expect("--config value");
        assert!(
            config.starts_with(&format!(r#"target."{host}".runner="#)),
            "runner key is not the host triple {host}: {config}"
        );
    }

    /// The gate checks the generated documentation pages with the pinned ess it was given.
    #[test]
    fn gate_checks_the_generated_documentation_pages_for_drift() {
        let command = docs_check(Path::new("/checkout"), Path::new(ROOT), Path::new("/ess"))
            .expect("documentation check command");
        let args: Vec<_> = command.get_args().skip(2).collect();
        assert_eq!(
            args,
            [
                "run",
                "--locked",
                "--offline",
                "-q",
                "-p",
                "connectors-docs",
                "--",
                "generate",
                "--check",
                "--ess",
                "/ess"
            ]
            .map(OsStr::new),
            "documentation check arguments"
        );
        assert_eq!(command.get_current_dir(), Some(Path::new("/checkout")));
    }

    /// A rustup toolchain selector must stay the first argument for the proxy to see it.
    #[test]
    fn gate_cargo_keeps_the_toolchain_selector_first() {
        let command = build(Some("1.88.0"));
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.first(), Some(&OsStr::new("+1.88.0")));
        assert_eq!(args.get(1), Some(&OsStr::new("--config")));
    }
}

/// Adversary cases: the runner the gate configures is executed the way Cargo executes it,
/// `<runner...> <test binary> <args...>`, against real programs.
/// The runner is `sh -c`; these run on Unix only.
#[cfg(all(test, unix))]
mod adversary_tests {
    use super::cargo;
    use std::{os::unix::fs::symlink, path::Path, process::Command};

    fn runner(root: &str) -> Vec<String> {
        let command = cargo(
            Path::new("/checkout"),
            Path::new(root),
            Path::new("/ess"),
            None,
        )
        .expect("gate cargo command");
        let config = command
            .get_args()
            .map(|arg| arg.to_str().expect("utf-8 argument").to_owned())
            .find_map(|arg| {
                arg.split_once(".runner=")
                    .map(|(_, runner)| runner.to_owned())
            })
            .expect("runner --config argument");
        serde_json::from_str(&config).expect("runner is a JSON/TOML string array")
    }

    /// Run `program` through the gate's runner as Cargo would, with an empty environment
    /// so nothing sensitive can be printed.
    fn through_runner(root: &str, program: &Path) -> std::process::Output {
        let argv = runner(root);
        Command::new(&argv[0])
            .args(&argv[1..])
            .arg(program)
            .env_clear()
            .output()
            .expect("spawn runner")
    }

    fn link(dir: &Path, name: &str, target: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        symlink(target, &path).unwrap();
        path
    }

    /// A test binary under a target directory whose path contains `=` (a checkout or
    /// `CARGO_TARGET_DIR` such as `.../build=1/target`) must still be executed, and its
    /// failure must still fail. `env` reads every leading `NAME=VALUE` word as an
    /// assignment, so it never runs the binary, prints its environment and exits 0.
    #[test]
    fn runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(
            &scratch.path().join("build=1/debug/deps"),
            "fails",
            "/usr/bin/false",
        );
        let output = through_runner("/checkout/.local/tmp/gate-AbCdEf", &binary);
        assert!(
            !output.status.success(),
            "a failing test binary at {} reported success through the gate runner \
             (status {:?}, {} bytes of stdout instead of running it)",
            binary.display(),
            output.status,
            output.stdout.len()
        );
    }

    /// Same path shape, observable effect: the binary runs and sees the gate root.
    #[test]
    fn runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(&scratch.path().join("a=b"), "printenv", "/usr/bin/printenv");
        let root = "/checkout/.local/tmp/gate-AbCdEf";
        let argv = runner(root);
        let output = Command::new(&argv[0])
            .args(&argv[1..])
            .arg(&binary)
            .arg("TMPDIR")
            .env_clear()
            .output()
            .expect("spawn runner");
        assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{root}\n"));
    }

    /// Roots with spaces, quotes, a single quote, a backslash, `=` and shell metacharacters
    /// reach the binary byte for byte.
    #[test]
    fn runner_preserves_awkward_roots() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(scratch.path(), "printenv", "/usr/bin/printenv");
        for root in [
            "/c/gate a b",
            "/c/gate\"q",
            "/c/gate'q",
            "/c/gate\\b",
            "/c/gate=e",
            "/c/gate$x`y`;|&",
            "/c/gaté",
        ] {
            let argv = runner(root);
            let output = Command::new(&argv[0])
                .args(&argv[1..])
                .arg(&binary)
                .arg("TMPDIR")
                .env_clear()
                .output()
                .expect("spawn runner");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{root}\n"),
                "root {root:?}"
            );
        }
    }
}

/// Adversary pass 2: the `sh -c` runner must be transparent to the test binary's exit
/// status, its signals and the arguments Cargo appends, and must hand over any root.
#[cfg(all(test, unix))]
mod adversary_pass2_tests {
    use super::cargo;
    use std::{
        os::unix::process::{CommandExt, ExitStatusExt},
        path::Path,
        process::{Command, Output},
    };

    fn runner(root: &str) -> Vec<String> {
        let command = cargo(
            Path::new("/checkout"),
            Path::new(root),
            Path::new("/ess"),
            None,
        )
        .expect("gate cargo command");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_str().expect("utf-8 argument").to_owned())
            .collect();
        assert_eq!(args.len(), 2, "exactly --config and its value: {args:?}");
        assert_eq!(args[0], "--config");
        let (_, value) = args[1].split_once("\".runner=").expect("host-keyed runner");
        serde_json::from_str(value).expect("runner is a string array")
    }

    /// `<runner...> <binary> <args...>`, exactly as Cargo spawns a test binary.
    fn run(root: &str, binary: &str, args: &[&str]) -> Output {
        let argv = runner(root);
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .arg(binary)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin");
        // A child killed by SIGABRT or SIGSEGV would otherwise leave a `core` file in the
        // working directory; the signal is what the tests observe, not the dump.
        // SAFETY: `setrlimit` is async-signal-safe and the closure touches no other state.
        unsafe {
            command.pre_exec(|| {
                let none = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::setrlimit(libc::RLIMIT_CORE, &none) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
        command.output().expect("spawn runner")
    }

    const ROOT: &str = "/checkout/.local/tmp/gate-AbCdEf";

    /// A test binary exiting 101 (libtest's failure code) or any other code > 1 keeps it.
    #[test]
    fn runner_preserves_exit_codes_above_one() {
        for code in [2, 101, 255] {
            let output = run(ROOT, "/bin/sh", &["-c", &format!("exit {code}")]);
            assert_eq!(output.status.code(), Some(code), "{:?}", output.status);
        }
    }

    /// A test binary killed by a signal is reported as killed, not as a shell exit code.
    #[test]
    fn runner_preserves_a_killing_signal() {
        for signal in [9, 6, 11] {
            let output = run(ROOT, "/bin/sh", &["-c", &format!("kill -{signal} $$")]);
            assert_eq!(output.status.signal(), Some(signal), "{:?}", output.status);
        }
    }

    /// Filters with spaces and newlines, empty arguments, leading dashes and shell text
    /// reach the test binary unchanged and in order.
    #[test]
    fn runner_passes_test_arguments_byte_for_byte() {
        let args = [
            "--exact",
            "module::name with space",
            "line1\nline2",
            "--nocapture",
            "-",
            "--",
            "",
            "$HOME `id` *",
            "--test-threads=1",
        ];
        let mut argv = vec!["-c", r#"printf '%s\0' "$@""#, "argv0"];
        argv.extend(args);
        let output = run(ROOT, "/bin/sh", &argv);
        assert!(output.status.success(), "{:?}", output.status);
        let expected: String = args.iter().map(|arg| format!("{arg}\0")).collect();
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }

    /// The binary is executed even when its path begins with a dash or holds a newline.
    #[test]
    fn runner_executes_awkward_binary_paths() {
        let scratch = tempfile::tempdir().unwrap();
        for name in ["-dash", "new\nline", "sp ace", "--"] {
            let path = scratch.path().join(name);
            std::os::unix::fs::symlink("/usr/bin/printenv", &path).unwrap();
            let output = run(ROOT, path.to_str().unwrap(), &["TMPDIR"]);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{ROOT}\n"),
                "binary {name:?}: {:?}",
                output.status
            );
        }
    }

    /// A root that begins with a dash or holds a newline still becomes `TMPDIR`.
    #[test]
    fn runner_hands_over_roots_with_a_leading_dash_or_newline() {
        for root in [
            "-x/gate",
            "--/gate",
            "-c",
            "+o/gate",
            "/c/new\nline",
            "/c/tab\tx",
        ] {
            let output = run(root, "/usr/bin/printenv", &["TMPDIR"]);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{root}\n"),
                "root {root:?}: {:?}",
                output.status
            );
        }
    }
}
