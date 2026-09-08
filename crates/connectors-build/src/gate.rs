use super::Result;
use std::{path::Path, process::Command};

pub fn run(root: &Path, ess: &Path, msrv: bool) -> Result<()> {
    connectors_spec::v2::check_ess(ess)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new().prefix("gate-").tempdir_in(base)?;
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
        if !cmd.status()?.success() {
            return Err(format!("gate command {:?} failed", cmd.get_program()).into());
        }
        Ok(())
    };
    execute(command("cargo").args(["fmt", "--all", "--", "--check"]))?;
    for adapter in ["gitlab", "kubernetes", "sql"] {
        let source = root.join(format!("adapters/{adapter}/spec/adapter.json"));
        let descriptor = connectors_spec::compile(&std::fs::read(source)?)?;
        if connectors_spec::descriptor_bytes(&descriptor)?
            != std::fs::read(root.join(format!("adapters/{adapter}/generated/descriptor.json")))?
        {
            return Err(format!("{adapter}: generated descriptor drift").into());
        }
        println!("gate: {adapter} descriptor matches");
    }
    // Generation fixtures verify the full pinned bundle, reproducibility and refusals.
    execute(command("cargo").args(["build", "--workspace", "--locked", "--offline"]))?;
    execute(command("cargo").args(["test", "--workspace", "--locked", "--offline"]))?;
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
    for adapter in ["gitlab", "kubernetes", "sql"] {
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
            "connectors-gitlab",
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
        println!("gate: {package} library boundary holds");
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
            Some(
                "connectors-gitlab" | "connectors-kubernetes" | "connectors-sql" | "tokio-postgres"
            )
        )
    }) {
        return Err("generic CLI depends on a provider".into());
    }
    println!("gate: generic CLI boundary holds");
    if msrv {
        // Keep compiler metadata separate without duplicating a full binary build.
        execute(
            command("cargo")
                .args([
                    "+1.88.0",
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--offline",
                ])
                .env("CARGO_TARGET_DIR", root.join("target/msrv")),
        )?;
    }
    execute(
        Command::new(ess)
            .current_dir(root)
            .env("TMPDIR", temp.path())
            .args(["validate", "--path", "ess"]),
    )?;
    execute(command("aep").args(["plan", "artifact", "validate"]))?;
    println!("gate: all checks passed");
    Ok(())
}
