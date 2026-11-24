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

/// The Windows target the gate lints the host and the CLI for.
const WINDOWS_TARGET: &str = "x86_64-pc-windows-gnu";

/// The C compiler for [`WINDOWS_TARGET`]: the build scripts of `ring` and `libsqlite3-sys`
/// compile C for the target, also under Clippy.
const WINDOWS_CC: &str = "x86_64-w64-mingw32-gcc";

/// The gate step that lints the host and the CLI for [`WINDOWS_TARGET`]: the lint of the
/// forge's Windows job (`.github/workflows/windows-host.yml`) with `--offline` and the target
/// added. Code under `cfg(windows)`, and code that is dead on Windows only, fails here and
/// not first on the forge. Clippy does not link; the forge's job stays the check of the
/// build and the tests.
fn windows_lint(root: &Path, temp: &Path, ess: &Path) -> Result<Command> {
    let mut cmd = cargo(root, temp, ess, None)?;
    cmd.args([
        "clippy",
        "--locked",
        "--offline",
        "-p",
        "connectors-host",
        "-p",
        "connectors",
        "--all-targets",
        "--target",
        WINDOWS_TARGET,
        "--",
        "-D",
        "warnings",
    ]);
    Ok(cmd)
}

/// The names of what [`windows_lint`] needs and `probe` does not find: the Rust target
/// [`WINDOWS_TARGET`] of the active toolchain and the C compiler [`WINDOWS_CC`]. `probe`
/// answers whether the host has the thing it is given the name of.
fn windows_lint_prerequisites(probe: &dyn Fn(&str) -> bool) -> Vec<&'static str> {
    [WINDOWS_TARGET, WINDOWS_CC]
        .into_iter()
        .filter(|name| !probe(name))
        .collect()
}

/// Fails the gate, naming each missing prerequisite of [`windows_lint`]. The step is never
/// skipped: a gate that passed without it would say nothing about Windows.
fn windows_lint_ready(probe: &dyn Fn(&str) -> bool) -> Result<()> {
    let missing = windows_lint_prerequisites(probe);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "gate: the windows lint needs {}, missing on this host (docs/development.md, \"The Windows lint\")",
        missing.join(" and ")
    )
    .into())
}

/// The gate's probe of this host. The Rust target is present when the toolchain that is
/// active in `root` holds the target's `core`; the C compiler when it runs.
fn host_probe(root: &Path) -> impl Fn(&str) -> bool + '_ {
    move |name| {
        if name != WINDOWS_TARGET {
            return Command::new(name)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success());
        }
        // rustc prints the directory for a target it has no libraries of as well.
        let Ok(output) = Command::new("rustc")
            .current_dir(root)
            .args(["--print", "target-libdir", "--target", name])
            .output()
        else {
            return false;
        };
        output.status.success()
            && std::fs::read_dir(String::from_utf8_lossy(&output.stdout).trim()).is_ok_and(
                |mut entries| {
                    entries.any(|entry| {
                        entry.is_ok_and(|entry| {
                            entry.file_name().to_string_lossy().starts_with("libcore-")
                        })
                    })
                },
            )
    }
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

/// The provider-neutral OAuth 2.0 client library.
const OAUTH2: &str = "connectors-oauth2";

/// The gate step that keeps `connectors-oauth2` free of every other `connectors-*` package:
/// its normal dependency tree, for every target and with every feature, may name no
/// `connectors-` package but itself, so the host, client, adapters and shared contracts can all
/// build on it and it on none of them.
fn oauth2_boundary(root: &Path, temp: &Path, ess: &Path) -> Result<()> {
    let output = super::run(cargo(root, temp, ess, None)?.args([
        "tree",
        "--locked",
        "--offline",
        "--edges",
        "normal",
        "--target",
        "all",
        "--all-features",
        "--prefix",
        "none",
        "-p",
        OAUTH2,
    ]))?;
    let tree = String::from_utf8(output.stdout)?;
    let mut forbidden: Vec<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| name.starts_with("connectors-") && *name != OAUTH2)
        .collect();
    forbidden.sort_unstable();
    forbidden.dedup();
    if !forbidden.is_empty() {
        return Err(format!("{OAUTH2} depends on forbidden {}", forbidden.join(", ")).into());
    }
    if !tree
        .lines()
        .any(|line| line.split_whitespace().next() == Some(OAUTH2))
    {
        return Err(format!("cargo tree did not list {OAUTH2}").into());
    }
    Ok(())
}

pub fn run(root: &Path, ess: &Path, aep: &Path, msrv: bool) -> Result<()> {
    // Asked first: the Windows lint runs late, and a host that cannot run it should not
    // learn so after the build and the tests.
    windows_lint_ready(&host_probe(root))?;
    // The CI's path list for pull requests that change only planning or prose files, both
    // sides of it, and its guard against a planning file a test reads (`~/beyond10x/AGENTS.md`,
    // "Planning-only pull requests skip the tests").
    let scope_test = ".github/ci/changed-scope-test.sh";
    if !Command::new("sh")
        .arg(scope_test)
        .current_dir(root)
        .status()?
        .success()
    {
        return Err(format!("gate: {scope_test} failed").into());
    }
    println!("gate: {scope_test}; exit=0");
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
    super::mcp::run(root, ess, true)?;
    super::service_metadata::run(root, ess, true)?;
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
    test_step(root, &|| command(None), &execute)?;
    // Each adapter's committed tool contract (contracts/tools/v1alpha1) from its descriptor
    // and the tool profiles authored beside it.
    execute(command(None)?.args([
        "run",
        "--locked",
        "--offline",
        "-p",
        "connectors-tool-contracts",
        "--",
        "generate",
        "--check",
    ]))?;
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
    execute(&mut windows_lint(root, temp.path(), ess)?)?;
    println!("gate: windows lint holds; exit=0");
    execute(&mut docs_check(root, temp.path(), ess)?)?;
    for adapter in [
        "grafana",
        "kubernetes",
        "loki",
        "prometheus",
        "sql",
        "tavily",
        "catalog-provider",
        "mcp",
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
            "connectors-mcp",
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
    oauth2_boundary(root, temp.path(), ess)?;
    println!("gate: {OAUTH2} library boundary holds; exit=0");
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
            "connectors-mcp",
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
                    // It builds on b10x-tool-contract, whose rust-version is 1.98.
                    "--exclude",
                    "connectors-tool-contracts",
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

/// The JUnit report `.config/nextest.toml` has nextest write, relative to the repository root.
const JUNIT: &str = "target/nextest/default/junit.xml";

/// The test step's wall-time budget (`~/beyond10x/AGENTS.md`, "Tests stay fast").
const TEST_STEP_BUDGET: std::time::Duration = std::time::Duration::from_secs(120);

/// The gate's test step. cargo-nextest runs each test in its own process under
/// `.config/nextest.toml`, which kills a test at 60 s (`slow-timeout` 30 s, `terminate-after`
/// 2) and fails it. The build and the shared fixtures run first, so the printed time is the
/// tests' own. The ten slowest
/// tests are printed from nextest's JUnit report whether the run passed or not. nextest runs no
/// doctests; `cargo test --doc` runs them after.
fn test_step(
    root: &Path,
    command: &dyn Fn() -> Result<Command>,
    execute: &dyn Fn(&mut Command) -> Result<()>,
) -> Result<()> {
    let nextest = |extra: &[&str]| -> Result<Command> {
        let mut cmd = command()?;
        // cargo-nextest resolves target runners itself and does not see a `--config` given
        // to cargo before the subcommand, so the TMPDIR runner is repeated after `run`.
        // Without it every test that writes a state directory ran under `/tmp` and was
        // refused `invalid_configuration` (PR #271, 1,337 of 4,230 tests).
        let runner: Vec<_> = cmd
            .get_args()
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|pair| pair[0] == "--config")
            .map(|pair| pair[1].to_owned())
            .collect();
        cmd.args(["nextest", "run"]);
        for config in runner {
            cmd.arg("--config").arg(config);
        }
        cmd.args(["--workspace", "--locked", "--offline"])
            .args(extra);
        Ok(cmd)
    };
    execute(&mut nextest(&["--no-run"])?)?;
    // The setup step: the metadata authority's baseline suite is synthesized once into its
    // digest-keyed cache under `target/`, so no test pays for it.
    let synthesis = std::time::Instant::now();
    super::metadata_emissions::baseline_suite(root)?;
    println!(
        "gate: metadata baseline suite ready in {:.1} s; exit=0",
        synthesis.elapsed().as_secs_f64()
    );
    let junit = root.join(JUNIT);
    match std::fs::remove_file(&junit) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    let started = std::time::Instant::now();
    let outcome = execute(&mut nextest(&[])?);
    let elapsed = started.elapsed();
    match std::fs::read_to_string(&junit) {
        Ok(report) => {
            println!("gate: ten slowest tests");
            for (seconds, name) in slowest(&report, 10) {
                println!("gate: {seconds:>8.1} s  {name}");
            }
        }
        Err(error) => println!("gate: no nextest report at {}: {error}", junit.display()),
    }
    println!(
        "gate: test step ran {:.1} s (budget {} s)",
        elapsed.as_secs_f64(),
        TEST_STEP_BUDGET.as_secs()
    );
    outcome?;
    execute(command()?.args(["test", "--workspace", "--doc", "--locked", "--offline"]))
}

/// The `limit` slowest test cases of a nextest JUnit report, slowest first, as
/// `(seconds, "<binary> <test>")`.
fn slowest(junit: &str, limit: usize) -> Vec<(f64, String)> {
    let attribute = |tag: &str, name: &str| -> Option<String> {
        let start = tag.find(&format!(" {name}=\""))? + name.len() + 3;
        let end = start + tag[start..].find('"')?;
        Some(
            tag[start..end]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&amp;", "&"),
        )
    };
    let mut cases: Vec<(f64, String)> = junit
        .split("<testcase")
        .skip(1)
        .filter_map(|rest| {
            let tag = &rest[..rest.find('>')?];
            let seconds = attribute(tag, "time")?.parse().ok()?;
            let name = attribute(tag, "name")?;
            Some(match attribute(tag, "classname") {
                Some(class) => (seconds, format!("{class} {name}")),
                None => (seconds, name),
            })
        })
        .collect();
    cases.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    cases.truncate(limit);
    cases
}

/// The authored scenario directories the gate compiles and executes.
const AUTHORED_DIRECTORIES: &[&str] = &[
    "contracts/operations/v1alpha1/scenarios",
    "contracts/auth/acquisition/v1alpha1/scenarios",
    "contracts/auth/evidence/v1alpha1/scenarios",
    "contracts/auth/connection/v1alpha1/scenarios",
    "contracts/service/v1alpha1/scenarios",
    "contracts/tools/v1alpha1/scenarios",
    // Inbound only. These are `ess-scenario/1` and ESS owns them, so synthesising them
    // against the shared IR is a real check.
    //
    // The outbound directory beside it is deliberately absent. Those files are
    // `mcp-outbound-lifecycle/1`, a scenario type this repository defined. ESS refuses
    // a foreign scenario type by design, so they stay out of the shared IR for good and
    // are held by `mcp_outbound_connection_lifecycle`'s own package-scoped cases.
    // Adding the directory here would fail the gate on every one of them.
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
    if passed == collected {
        println!("gate: executed {collected} authored scenarios, {passed} passed; exit=0");
    } else {
        println!(
            "gate: executed {collected} authored scenarios, {passed} passed, {} known failing; exit=0",
            collected - passed
        );
    }
    Ok(())
}

/// Authored scenarios that fail when the pinned ESS interpreter executes them. Empty: every
/// authored scenario passes, and a scenario that fails fails the gate. An entry needs its
/// owner's triage and a story; none is waived silently. A listed scenario that starts passing
/// fails the gate too, so the list cannot outlive its cause.
const AUTHORED_KNOWN_FAILING: &[&str] = &[];

/// Holds an `ess-conformance-run/2` report's `outcomes` to the authored scenarios the gate
/// collected: every one ran, every one not in [`AUTHORED_KNOWN_FAILING`] passed, and every
/// listed one still failed. Returns the number that passed.
fn authored_execution(outcomes: &serde_json::Value, collected: usize) -> Result<usize> {
    authored_execution_with(outcomes, collected, AUTHORED_KNOWN_FAILING)
}

/// [`authored_execution`] against an explicit known-failing list.
fn authored_execution_with(
    outcomes: &serde_json::Value,
    collected: usize,
    known_failing: &[&str],
) -> Result<usize> {
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
        if !known_failing.contains(&name.as_str()) {
            problems.push(format!("failed: {name}"));
        }
    }
    for name in known_failing {
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
    use super::{
        AUTHORED_KNOWN_FAILING, authored_execution, authored_execution_with, authored_scenarios,
    };
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

    /// Every authored scenario passes when executed: none is waived as a known failure, and the
    /// pinned ESS interpreter reports no failed, errored, unsupported or skipped one.
    #[test]
    fn no_authored_scenario_is_waived_as_known_failing() {
        assert!(
            AUTHORED_KNOWN_FAILING.is_empty(),
            "authored scenarios still waived: {AUTHORED_KNOWN_FAILING:#?}"
        );
    }

    /// Names a real-shaped list so the mechanism is held without waiving any scenario.
    const WAIVED: &[&str] = &[
        "connectors.refresh/authored/waived-a",
        "connectors.refresh/authored/waived-b",
    ];

    fn report(passed: &[&str], failed: &[&str]) -> serde_json::Value {
        json!({"passed": passed, "failed": failed, "error": [], "unsupported": [], "skipped": []})
    }

    #[test]
    fn every_unlisted_scenario_must_pass() {
        let failed: Vec<&str> = WAIVED
            .iter()
            .copied()
            .chain(["connectors.auth_bindings/authored/stale"])
            .collect();
        let error =
            authored_execution_with(&report(&[], &failed), failed.len(), WAIVED).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("failed: connectors.auth_bindings/authored/stale")
        );
    }

    #[test]
    fn with_nothing_listed_a_failed_scenario_fails_the_gate() {
        let error = authored_execution(
            &report(
                &["connectors.declarations/authored/a"],
                &["connectors.refresh/authored/b"],
            ),
            2,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("failed: connectors.refresh/authored/b")
        );
    }

    #[test]
    fn with_nothing_listed_every_passed_scenario_counts() {
        let passed = [
            "connectors.declarations/authored/a",
            "connectors.refresh/authored/b",
        ];
        assert_eq!(
            authored_execution(&report(&passed, &[]), passed.len()).unwrap(),
            passed.len()
        );
    }

    #[test]
    fn a_listed_scenario_that_passes_fails_the_gate() {
        let error =
            authored_execution_with(&report(&WAIVED[..1], &WAIVED[1..]), WAIVED.len(), WAIVED)
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("passes but is listed as known failing")
        );
    }

    #[test]
    fn a_scenario_that_did_not_run_fails_the_gate() {
        let error = authored_execution_with(
            &report(&["connectors.declarations/authored/a"], WAIVED),
            WAIVED.len() + 2,
            WAIVED,
        )
        .unwrap_err();
        assert!(error.to_string().contains("collected"));
    }

    #[test]
    fn known_failures_and_passes_hold() {
        let passed = ["connectors.declarations/authored/a"];
        assert_eq!(
            authored_execution_with(&report(&passed, WAIVED), WAIVED.len() + 1, WAIVED).unwrap(),
            1
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Result, cargo, docs_check, test_step, windows_lint, windows_lint_prerequisites,
        windows_lint_ready,
    };
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

    /// cargo-nextest does not read a `--config` given to cargo before the subcommand, so
    /// the test step repeats the runner after `nextest run`; otherwise the test binaries
    /// keep the caller's `TMPDIR`.
    #[test]
    fn the_test_step_hands_the_runner_to_nextest_itself() {
        let commands = std::cell::RefCell::new(Vec::new());
        let execute = |cmd: &mut std::process::Command| -> Result<()> {
            commands
                .borrow_mut()
                .push(cmd.get_args().map(OsStr::to_owned).collect::<Vec<_>>());
            Err("stop after the first command".into())
        };
        let _ = test_step(Path::new("/checkout"), &|| Ok(build(None)), &execute);
        let commands = commands.into_inner();
        let first = commands.first().expect("the test step runs nextest");
        let run = first
            .windows(2)
            .position(|pair| pair[0] == "nextest" && pair[1] == "run")
            .expect("nextest run");
        let after = &first[run + 2..];
        let host = rustc_host();
        let expected = format!(
            r#"target."{host}".runner=["sh","-c","export TMPDIR=\"$0\"; exec \"$@\"","{ROOT}"]"#
        );
        assert!(
            after
                .windows(2)
                .any(|pair| pair[0] == "--config" && pair[1] == OsStr::new(&expected)),
            "nextest run carries no runner: {first:?}"
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

    /// The ten slowest come from nextest's JUnit report, slowest first, named by binary and test.
    #[test]
    fn the_slowest_tests_are_read_from_the_junit_report() {
        let mut report = String::from(r#"<?xml version="1.0"?><testsuites><testsuite name="a">"#);
        for index in 0..12 {
            report.push_str(&format!(
                r#"<testcase name="t{index}" classname="connectors-build::bin/connectors-build" timestamp="2026-11-22T00:00:00Z" time="{index}.5"></testcase>"#
            ));
        }
        report.push_str(r#"<testcase name="a &amp; b" classname="x" time="99.25"/></testsuite>"#);
        let slowest = super::slowest(&report, 10);
        assert_eq!(slowest.len(), 10);
        assert_eq!(slowest[0], (99.25, "x a & b".to_owned()));
        assert_eq!(
            slowest[1],
            (
                11.5,
                "connectors-build::bin/connectors-build t11".to_owned()
            )
        );
        assert_eq!(slowest[9].0, 3.5);
    }

    /// `.config/nextest.toml` kills a test at 60 s and fails it, and writes the JUnit report the
    /// gate reads.
    #[test]
    fn nextest_kills_a_test_at_sixty_seconds_and_writes_the_report_the_gate_reads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config: toml::Table =
            toml::from_str(&std::fs::read_to_string(root.join(".config/nextest.toml")).unwrap())
                .unwrap();
        let profile = &config["profile"]["default"];
        assert_eq!(profile["slow-timeout"]["period"].as_str(), Some("30s"));
        assert_eq!(
            profile["slow-timeout"]["terminate-after"].as_integer(),
            Some(2)
        );
        let junit = format!(
            "{}/{}/{}",
            config["store"]["dir"].as_str().unwrap(),
            "default",
            profile["junit"]["path"].as_str().unwrap()
        );
        assert_eq!(junit, super::JUNIT);
    }

    /// A package that builds its integration tests into one binary (`autotests = false`) builds
    /// only what its manifest names, so a file added under `tests/` and declared nowhere would
    /// never run. Every top-level `tests/*.rs` file is a `[[test]]` root or a module one declares.
    #[test]
    fn every_integration_test_file_is_built() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut checked = 0;
        for group in ["crates", "adapters", "apps"] {
            for package in std::fs::read_dir(root.join(group)).unwrap() {
                let package = package.unwrap().path();
                let Ok(text) = std::fs::read_to_string(package.join("Cargo.toml")) else {
                    continue;
                };
                let manifest: toml::Table = toml::from_str(&text).unwrap();
                if manifest["package"]
                    .get("autotests")
                    .and_then(toml::Value::as_bool)
                    != Some(false)
                {
                    continue;
                }
                let roots: Vec<String> = manifest
                    .get("test")
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|test| test.get("path")?.as_str().map(str::to_owned))
                    .collect();
                let declared: String = roots
                    .iter()
                    .map(|path| std::fs::read_to_string(package.join(path)).unwrap())
                    .collect();
                for file in std::fs::read_dir(package.join("tests")).unwrap() {
                    let file = file.unwrap().path();
                    if file.extension().is_none_or(|extension| extension != "rs") {
                        continue;
                    }
                    let name = file.file_name().unwrap().to_str().unwrap();
                    let stem = name.strip_suffix(".rs").unwrap();
                    assert!(
                        roots.iter().any(|path| path == &format!("tests/{name}"))
                            || declared.lines().any(|line| {
                                line.trim() == format!("mod {stem};")
                                    || line.contains(&format!("#[path = \"{name}\"]"))
                            }),
                        "{} is built into no test binary",
                        file.display()
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no package builds its tests into one binary");
    }

    /// A rustup toolchain selector must stay the first argument for the proxy to see it.
    #[test]
    fn gate_cargo_keeps_the_toolchain_selector_first() {
        let command = build(Some("1.88.0"));
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.first(), Some(&OsStr::new("+1.88.0")));
        assert_eq!(args.get(1), Some(&OsStr::new("--config")));
    }

    /// The Windows lint's arguments from `clippy` on, after the `--config` pair of `cargo(...)`.
    fn windows_lint_arguments() -> Vec<String> {
        let command = windows_lint(Path::new("/checkout"), Path::new(ROOT), Path::new("/ess"))
            .expect("windows lint command");
        assert_eq!(command.get_current_dir(), Some(Path::new("/checkout")));
        command
            .get_args()
            .skip(2)
            .map(|argument| argument.to_str().expect("utf-8 argument").to_owned())
            .collect()
    }

    /// The gate lints what the forge's Windows job lints: the `cargo clippy` line of
    /// `.github/workflows/windows-host.yml`, with only `--offline` and the target added. A
    /// package or a flag the workflow's line gains or loses fails here.
    #[test]
    fn the_windows_lint_is_the_forges_lint_for_the_gnu_target() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let workflow =
            std::fs::read_to_string(root.join(".github/workflows/windows-host.yml")).unwrap();
        let lines: Vec<&str> = workflow
            .lines()
            .filter(|line| line.contains("cargo clippy"))
            .collect();
        assert_eq!(lines.len(), 1, "one lint step expected: {lines:?}");
        let (_, forge) = lines[0].split_once("run: cargo ").expect("a run line");
        let forge: Vec<&str> = forge.split_whitespace().collect();
        let local: Vec<String> = windows_lint_arguments()
            .into_iter()
            .filter(|argument| {
                !["--offline", "--target", "x86_64-pc-windows-gnu"].contains(&argument.as_str())
            })
            .collect();
        assert_eq!(local, forge);
    }

    /// The whole command: offline like the gate's other cargo steps, and the target before
    /// the `--` that starts Clippy's own arguments.
    #[test]
    fn the_windows_lint_runs_offline_for_the_gnu_target() {
        assert_eq!(
            windows_lint_arguments(),
            [
                "clippy",
                "--locked",
                "--offline",
                "-p",
                "connectors-host",
                "-p",
                "connectors",
                "--all-targets",
                "--target",
                "x86_64-pc-windows-gnu",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn windows_lint_prerequisites_name_a_missing_rust_target() {
        assert_eq!(
            windows_lint_prerequisites(&|name| name != "x86_64-pc-windows-gnu"),
            ["x86_64-pc-windows-gnu"]
        );
    }

    #[test]
    fn windows_lint_prerequisites_name_a_missing_c_compiler() {
        assert_eq!(
            windows_lint_prerequisites(&|name| name != "x86_64-w64-mingw32-gcc"),
            ["x86_64-w64-mingw32-gcc"]
        );
    }

    /// A host that lacks a prerequisite fails the gate by name; the step is never skipped.
    #[test]
    fn the_gate_names_each_missing_prerequisite_of_the_windows_lint() {
        let error = windows_lint_ready(&|_| false)
            .expect_err("a host with neither prerequisite")
            .to_string();
        for name in ["x86_64-pc-windows-gnu", "x86_64-w64-mingw32-gcc"] {
            assert!(
                error.contains(name),
                "the error does not name {name}: {error}"
            );
        }
        let error = windows_lint_ready(&|name| name != "x86_64-w64-mingw32-gcc")
            .expect_err("a host without the C compiler")
            .to_string();
        assert!(error.contains("x86_64-w64-mingw32-gcc"), "{error}");
        assert!(!error.contains("x86_64-pc-windows-gnu"), "{error}");
        windows_lint_ready(&|_| true).expect("a host with both prerequisites");
    }

    /// The build server and the forge's gate job have both prerequisites, read with the
    /// probe the gate itself uses.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_host_probe_finds_what_the_windows_lint_needs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let probe = super::host_probe(&root);
        assert_eq!(windows_lint_prerequisites(&probe), Vec::<&str>::new());
        // The probe can answer no: a compiler no host has is not found.
        assert!(!probe("x86_64-w64-mingw32-gcc-of-no-host"));
    }
}

/// The `connectors-oauth2` boundary step run by Cargo itself, against a scratch workspace whose
/// manifests are the case and against this repository.
#[cfg(test)]
mod oauth2_boundary_tests {
    use super::oauth2_boundary;
    use std::{
        path::{Path, PathBuf},
        process::Command,
    };

    /// `connectors-host` as a path dependency.
    const HOST: &str = "connectors-host = { path = \"../connectors-host\" }\n";

    /// A workspace of two empty packages, `connectors-oauth2` and `connectors-host`, the first
    /// with `dependencies` (manifest tables) after its package table, with its lockfile
    /// generated offline.
    fn workspace(dependencies: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let write = |path: &str, text: &str| {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"connectors-oauth2\", \"connectors-host\"]\n",
        );
        write(
            "connectors-oauth2/Cargo.toml",
            &format!(
                "[package]\nname = \"connectors-oauth2\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n{dependencies}"
            ),
        );
        write("connectors-oauth2/src/lib.rs", "");
        write(
            "connectors-host/Cargo.toml",
            "[package]\nname = \"connectors-host\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        );
        write("connectors-host/src/lib.rs", "");
        let status = Command::new("cargo")
            .current_dir(root.path())
            .args(["generate-lockfile", "--offline"])
            .status()
            .unwrap();
        assert!(status.success(), "cargo generate-lockfile: {status}");
        root
    }

    fn check(root: &Path) -> Result<(), String> {
        let temp = tempfile::tempdir().unwrap();
        oauth2_boundary(root, temp.path(), Path::new("/ess")).map_err(|error| error.to_string())
    }

    fn assert_refused(dependencies: &str) {
        let refused = check(workspace(dependencies).path()).unwrap_err();
        assert!(
            refused.contains("connectors-oauth2") && refused.contains("connectors-host"),
            "{dependencies}: {refused}"
        );
    }

    #[test]
    fn the_oauth2_boundary_refuses_a_manifest_that_adds_connectors_host() {
        assert_eq!(check(workspace("").path()), Ok(()));
        assert_refused(&format!("[dependencies]\n{HOST}"));
    }

    /// A dependency for another platform, or behind a feature, is still a dependency: the step
    /// reads every target and every feature, not the build host's default set.
    #[test]
    fn the_oauth2_boundary_sees_target_specific_and_feature_gated_dependencies() {
        let other = if cfg!(windows) { "unix" } else { "windows" };
        assert_refused(&format!("[target.'cfg({other})'.dependencies]\n{HOST}"));
        assert_refused(
            "[dependencies]\nconnectors-host = { path = \"../connectors-host\", optional = true }\n\n[features]\nhost = [\"dep:connectors-host\"]\n",
        );
    }

    #[test]
    fn the_repository_oauth2_crate_holds_its_boundary() {
        let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(check(&root), Ok(()));
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

/// Adversary pass on the Windows lint step: what the six tests beside `windows_lint` leave
/// open. They test the command builder and the prerequisite list; none of them fails when
/// `run` stops calling either, or when the probe of the Rust target answers yes for a target
/// rustc has no libraries of.
#[cfg(all(test, unix))]
mod windows_lint_adversary_tests {
    use super::{WINDOWS_TARGET, host_probe, windows_lint_ready};
    use std::{os::unix::fs::PermissionsExt, path::Path, process::Command};

    const READY: &str = "    windows_lint_ready(&host_probe(root))?;";
    const LINT: &str = "    execute(&mut windows_lint(root, temp.path(), ess)?)?;";
    const HOLDS: &str = "    println!(\"gate: windows lint holds; exit=0\");";

    /// The body of `run`, read from this file: `run` builds and tests the whole workspace,
    /// so no test can call it.
    fn run_body() -> &'static str {
        let source = include_str!("gate.rs");
        let signature =
            "pub fn run(root: &Path, ess: &Path, aep: &Path, msrv: bool) -> Result<()> {\n";
        let start = source.find(signature).expect("the signature of `run`") + signature.len();
        let end = source[start..].find("\n}\n").expect("the end of `run`");
        &source[start..start + end]
    }

    /// What is wrong with the Windows lint in a body of `run`, if anything: the prerequisite
    /// check is its first statement and returns its error; the lint is executed once, at the
    /// top level of `run` (under no `if`), after the Linux lint, and returns its error; the
    /// line that says it holds follows it directly.
    fn windows_lint_defect(run: &str) -> Option<&'static str> {
        let lines: Vec<&str> = run.lines().collect();
        let first = lines.iter().find(|line| {
            let line = line.trim_start();
            !line.is_empty() && !line.starts_with("//")
        });
        if first != Some(&READY) {
            return Some(
                "the prerequisite check is not the first statement that returns its error",
            );
        }
        let places: Vec<usize> = (0..lines.len()).filter(|at| lines[*at] == LINT).collect();
        let [lint] = places[..] else {
            return Some("the lint is not executed once at the top level, returning its error");
        };
        if lines.get(lint + 1) != Some(&HOLDS) {
            return Some("the line that says the lint holds does not follow the lint");
        }
        match lines.iter().position(|line| line.trim() == "\"clippy\",") {
            Some(linux) if linux < lint => None,
            _ => Some("the Windows lint does not follow the Linux lint"),
        }
    }

    /// `run` asks for the prerequisites first and executes the Windows lint after the Linux
    /// lint, each with `?`: a red lint or a missing prerequisite ends the gate non-zero.
    #[test]
    fn run_checks_the_prerequisites_first_and_executes_the_windows_lint() {
        assert_eq!(windows_lint_defect(run_body()), None);
    }

    /// The mutants of `run` the six tests beside `windows_lint` leave green, each made from
    /// the body above and refused by the check of the test before this one.
    #[test]
    fn each_mutant_of_run_that_loses_the_windows_lint_is_refused() {
        let run = run_body();
        let mutants = [
            (
                "the lint is dropped",
                run.replacen(&format!("{LINT}\n"), "", 1),
            ),
            (
                "the lint's result is discarded",
                run.replacen(
                    LINT,
                    "    let _ = execute(&mut windows_lint(root, temp.path(), ess)?);",
                    1,
                ),
            ),
            (
                "the lint runs without --msrv only",
                run.replacen(LINT, &format!("    if !msrv {{\n    {LINT}\n    }}"), 1),
            ),
            (
                "the prerequisite check is dropped",
                run.replacen(&format!("{READY}\n"), "", 1),
            ),
            (
                "the prerequisite check's result is discarded",
                run.replacen(
                    READY,
                    "    let _ = windows_lint_ready(&host_probe(root));",
                    1,
                ),
            ),
            (
                "the prerequisite check moves behind the build and the tests",
                run.replacen(&format!("{READY}\n"), "", 1).replacen(
                    LINT,
                    &format!("{READY}\n{LINT}"),
                    1,
                ),
            ),
        ];
        for (name, mutant) in mutants {
            assert_ne!(mutant, run, "{name}: the mutant equals `run`");
            assert!(
                windows_lint_defect(&mutant).is_some(),
                "{name}: the check finds nothing wrong"
            );
        }
    }

    /// The error of a host with neither prerequisite is the line the guide quotes, and the
    /// section it sends the reader to exists.
    #[test]
    fn the_error_for_missing_prerequisites_is_the_line_the_guide_quotes() {
        let error = windows_lint_ready(&|_| false)
            .expect_err("a host with neither prerequisite")
            .to_string();
        assert_eq!(
            error,
            "gate: the windows lint needs x86_64-pc-windows-gnu and x86_64-w64-mingw32-gcc, \
             missing on this host (docs/development.md, \"The Windows lint\")"
        );
        let guide = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/development.md"),
        )
        .unwrap();
        assert!(guide.lines().any(|line| line == "## The Windows lint"));
        assert!(guide.contains("(`gate: the windows lint needs …`)"));
    }

    /// Set in the child process of the tests below to the probe that answers there.
    const PLANTED: &str = "CONNECTORS_GATE_ADVERSARY_PLANTED_RUSTC";
    /// The directory the planted `rustc` prints.
    const LIBDIR: &str = "CONNECTORS_GATE_ADVERSARY_LIBDIR";
    const ANSWER: &str = "planted-rustc-answer=";

    /// The mutant of `host_probe`'s target branch: rustc's exit status alone, without the
    /// look into the directory it prints.
    fn target_probe_without_the_library_check(root: &Path) -> bool {
        Command::new("rustc")
            .current_dir(root)
            .args(["--print", "target-libdir", "--target", WINDOWS_TARGET])
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// In the child process: prints the answer of the probe `PLANTED` names for the Windows
    /// target and returns true. `host_probe` starts `rustc` from the process's own `PATH`,
    /// so a planted `rustc` needs a process of its own.
    fn answered_as_the_child() -> bool {
        let Some(probe) = std::env::var_os(PLANTED) else {
            return false;
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let present = if probe == "mutant" {
            target_probe_without_the_library_check(&root)
        } else {
            host_probe(&root)(WINDOWS_TARGET)
        };
        println!("\n{ANSWER}{present}");
        true
    }

    /// The answer of `probe` (`gate` or `mutant`) for the Windows target in a process whose
    /// first `rustc` on `PATH` prints `libdir` and exits 0, as rustc does for a target it
    /// has no libraries of. The process is this test binary running `test` alone.
    fn answer_under_a_planted_rustc(test: &str, probe: &str, libdir: &Path) -> bool {
        let bin = tempfile::tempdir().unwrap();
        let rustc = bin.path().join("rustc");
        std::fs::write(&rustc, format!("#!/bin/sh\nprintf '%s\\n' \"${LIBDIR}\"\n")).unwrap();
        std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut path = vec![bin.path().to_owned()];
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let output = Command::new(std::env::current_exe().unwrap())
            .arg(format!("gate::windows_lint_adversary_tests::{test}"))
            .args(["--exact", "--nocapture"])
            .env("PATH", std::env::join_paths(path).unwrap())
            .env(PLANTED, probe)
            .env(LIBDIR, libdir)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let answer = stdout.split(ANSWER).nth(1).unwrap_or_else(|| {
            panic!(
                "the child printed no answer: {stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert!(output.status.success(), "the child failed: {stdout}");
        answer.starts_with("true")
    }

    /// The states of the directory rustc names for the Windows target: `absent` (the target
    /// is not installed; `rustc --print target-libdir` still names it and exits 0), `empty`,
    /// `mingw` (only the linker's `self-contained` files, no `core`) and `installed`.
    fn libdirs(scratch: &Path) -> [std::path::PathBuf; 4] {
        let [absent, empty, mingw, installed] =
            ["absent", "empty", "mingw", "installed"].map(|name| scratch.join(name).join("lib"));
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(mingw.join("self-contained")).unwrap();
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("libcore-0123456789abcdef.rlib"), b"").unwrap();
        [absent, empty, mingw, installed]
    }

    /// The gate's probe answers no for a target rustc names a directory for and has no
    /// `core` of, and yes when the directory holds one.
    #[test]
    fn the_host_probe_misses_a_target_rustc_has_no_core_of() {
        if answered_as_the_child() {
            return;
        }
        let test = "the_host_probe_misses_a_target_rustc_has_no_core_of";
        let scratch = tempfile::tempdir().unwrap();
        let [absent, empty, mingw, installed] = libdirs(scratch.path());
        assert!(
            answer_under_a_planted_rustc(test, "gate", &installed),
            "a directory with libcore-*.rlib is an installed target"
        );
        for libdir in [absent, empty, mingw] {
            assert!(
                !answer_under_a_planted_rustc(test, "gate", &libdir),
                "the probe finds a target in {}",
                libdir.display()
            );
        }
    }

    /// The control of the test above: under the same planted `rustc` the mutant without the
    /// library check answers yes for a target that is not installed, so the test above is
    /// red on that mutant.
    #[test]
    fn a_probe_without_the_library_check_finds_a_target_that_is_not_installed() {
        if answered_as_the_child() {
            return;
        }
        let test = "a_probe_without_the_library_check_finds_a_target_that_is_not_installed";
        let scratch = tempfile::tempdir().unwrap();
        let [absent, ..] = libdirs(scratch.path());
        assert!(answer_under_a_planted_rustc(test, "mutant", &absent));
        assert!(!answer_under_a_planted_rustc(test, "gate", &absent));
    }
}
