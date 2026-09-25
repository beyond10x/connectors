use super::Result;
use std::{path::Path, process::Command};

pub fn run(root: &Path, ess: &Path, aep: &Path, msrv: bool) -> Result<()> {
    connectors_spec::v2::check_ess(ess)?;
    super::metadata_entities::run(root, true)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new().prefix("gate-").tempdir_in(base)?;
    super::ess_boundary::run(root, ess, temp.path())?;
    super::source_hashes::run(root)?;
    super::cli::run(root, ess, true)?;
    let command = |program: &str| {
        let mut cmd = Command::new(program);
        cmd.current_dir(root)
            .env("TMPDIR", temp.path())
            .env("CARGO_BUILD_JOBS", "2")
            .env("CONNECTORS_ESS", ess);
        cmd
    };
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
        &super::run(command("cargo").args([
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
    let mut format = command("cargo");
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
    for adapter in ["kubernetes", "sql"] {
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
    execute(command("cargo").args(["build", "--workspace", "--locked", "--offline"]))?;
    execute(command("cargo").args(["test", "--workspace", "--locked", "--offline"]))?;
    // Alone: its recovery window is a 250 ms product bound, and parallel neighbours on a 4-thread runner consume it.
    let isolated = [
        "test",
        "--locked",
        "--offline",
        "-p",
        "connectors-host",
        "--lib",
        "--",
        "--ignored",
        "--exact",
        "local::owner::mutation::tests::final_audit_recovery_preserves_every_live_business_result",
        "--test-threads=1",
    ];
    println!("gate: {isolated:?}");
    let output = command("cargo")
        .args(isolated)
        .stderr(std::process::Stdio::inherit())
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    println!("gate: exit={} {isolated:?}", output.status);
    if !output.status.success() {
        return Err("gate command \"cargo\" failed".into());
    }
    ran_exactly_one(&stdout)?;
    execute(command("cargo").args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--offline",
        "--",
        "-D",
        "warnings",
    ]))?;
    for adapter in ["kubernetes", "sql", "catalog-provider"] {
        let package = format!("connectors-{adapter}");
        execute(command("cargo").args([
            "build",
            "--locked",
            "--offline",
            "-p",
            &package,
            "--lib",
            "--no-default-features",
        ]))?;
        let output = super::run(command("cargo").args([
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
            "connectors-kubernetes",
            "connectors-sql",
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
    let output = super::run(command("cargo").args([
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
            Some("connectors-kubernetes" | "connectors-sql" | "tokio-postgres")
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
                command("cargo")
                    .args([
                        "+1.88.0",
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
            "connectors-kubernetes",
            "connectors-sql",
        ] {
            execute(
                command("cargo")
                    .args([
                        "+1.88.0",
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
            command("cargo")
                .args([
                    "+1.91.0",
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--offline",
                ])
                .env("CARGO_TARGET_DIR", target.join("msrv-1.91")),
        )?;
    }
    // The pinned ESS authoring reader is non-recursive. Collect each explicitly
    // registered directory with a unique prefix so none silently disappears.
    let scenarios = temp.path().join("authored-scenarios");
    std::fs::create_dir(&scenarios)?;
    for (index, directory) in [
        "contracts/operations/v1alpha1/scenarios",
        "contracts/auth/acquisition/v1alpha1/scenarios",
        "contracts/auth/evidence/v1alpha1/scenarios",
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
    ]
    .iter()
    .enumerate()
    {
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
    }
    // Compile generated obligations and authored traces. This checks the model;
    // the mutation and auth runtime conformance bindings remain subsequent work.
    execute(
        Command::new(ess)
            .current_dir(root)
            .env("TMPDIR", temp.path())
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
            .arg(temp.path().join("contract-conformance.json")),
    )?;
    let mut planning = Command::new(aep);
    planning.current_dir(root).env("TMPDIR", temp.path());
    execute(planning.args(["plan", "artifact", "validate"]))?;
    println!("gate: all checks passed");
    Ok(())
}

/// A name filter that matches nothing still exits 0, so an isolated step is
/// green only when its single binary reports exactly the one case passing.
fn ran_exactly_one(output: &str) -> Result<()> {
    let summaries = output
        .lines()
        .filter(|line| line.starts_with("test result: "))
        .collect::<Vec<_>>();
    match summaries.as_slice() {
        [summary] if summary.starts_with("test result: ok. 1 passed; 0 failed;") => Ok(()),
        _ => Err(
            format!("isolated test step did not run exactly one passing case: {summaries:?}")
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_isolated_run_counts_only_when_its_one_case_passed() {
        assert!(
            ran_exactly_one("running 1 test\ntest x ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 184 filtered out; finished in 24.61s\n")
                .is_ok()
        );
    }

    #[test]
    fn an_isolated_run_that_selected_nothing_or_more_is_refused() {
        for output in [
            // A renamed or moved case: the filter matches nothing and cargo still exits 0.
            "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 185 filtered out; finished in 0.00s\n",
            "running 2 tests\n\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 183 filtered out; finished in 1.00s\n",
            "",
            // Another binary of the same package also printed a summary.
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n",
        ] {
            assert!(ran_exactly_one(output).is_err(), "accepted {output:?}");
        }
    }
}
