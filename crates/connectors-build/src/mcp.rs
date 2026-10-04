//! Regenerate native MCP launch projections with the repository's exact ESS pin.
use std::{collections::BTreeMap, path::Path, process::Command};

use super::Result;

const SPEC: &str = "adapters/mcp/spec/ess";
const CLI: &str = "adapters/mcp/generated/launch-cli";
const TYPES: &str = "adapters/mcp/generated/launch-types";
const CONFIGURATION: &str = "adapters/mcp/generated/configuration-types";
const SUPERVISION: &str = "adapters/mcp/generated/supervision";

pub fn run(root: &Path, ess: &Path, check: bool) -> Result<()> {
    connectors_spec::toolchain::check(ess)?;
    let mut cli = Command::new(ess);
    cli.current_dir(root).args([
        "generate",
        "cli",
        "--path",
        SPEC,
        "--binding",
        "adapters/mcp/spec/cli.yaml",
        "--out",
        CLI,
    ]);
    if check {
        cli.arg("--check");
    }
    execute(&mut cli)?;
    if check {
        let base = root.join(".local/tmp");
        std::fs::create_dir_all(&base)?;
        let temp = tempfile::Builder::new()
            .prefix("mcp-types-")
            .tempdir_in(base)?;
        let fresh = temp.path().join("types");
        generate_types(root, ess, &fresh)?;
        if artifacts(&fresh)? != artifacts(&root.join(TYPES))? {
            return Err(
                "MCP launch types differ from pinned ESS projection; run mcp-bindings".into(),
            );
        }
        let configuration = temp.path().join("configuration");
        generate_configuration(root, ess, &configuration)?;
        if artifacts(&configuration)? != artifacts(&root.join(CONFIGURATION))? {
            return Err(
                "MCP configuration types differ from pinned ESS projection; run mcp-bindings"
                    .into(),
            );
        }
        let supervision = temp.path().join("supervision");
        generate_supervision(root, ess, temp.path(), &supervision)?;
        if artifacts(&supervision)? != artifacts(&root.join(SUPERVISION))? {
            return Err(
                "MCP supervisor differs from the shared session projection; run mcp-bindings"
                    .into(),
            );
        }
    } else {
        generate_types(root, ess, &root.join(TYPES))?;
        generate_configuration(root, ess, &root.join(CONFIGURATION))?;
        let base = root.join(".local/tmp");
        std::fs::create_dir_all(&base)?;
        let temp = tempfile::Builder::new()
            .prefix("mcp-supervision-")
            .tempdir_in(base)?;
        generate_supervision(root, ess, temp.path(), &root.join(SUPERVISION))?;
    }
    println!(
        "MCP launch, configuration and shared session projections {}; runtime admission remains separate",
        if check { "match" } else { "generated" }
    );
    Ok(())
}

fn generate_supervision(root: &Path, ess: &Path, temp: &Path, out: &Path) -> Result<()> {
    // Select the owning shared domain without maintaining a second editable model.
    // The header retains the shared system's format, identity and version. A new
    // cross-domain dependency refuses synthesis until this selection is reviewed.
    let source = temp.join("session-input");
    std::fs::create_dir_all(source.join("domains"))?;
    let mut header: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(&std::fs::read(root.join("ess/system.yaml"))?)?;
    if !header
        .get("domains")
        .and_then(serde_yaml_ng::Value::as_sequence)
        .is_some_and(|domains| {
            domains
                .iter()
                .any(|domain| domain.as_str() == Some("connectors.sessions"))
        })
    {
        return Err("shared ESS header does not declare connectors.sessions".into());
    }
    header["domains"] = serde_yaml_ng::to_value(["connectors.sessions"])?;
    std::fs::write(
        source.join("system.yaml"),
        serde_yaml_ng::to_string(&header)?,
    )?;
    std::fs::copy(
        root.join("ess/domains/sessions.yaml"),
        source.join("domains/sessions.yaml"),
    )?;
    let mut command = Command::new(ess);
    command
        .current_dir(root)
        .args(["generate", "synthesize", "--path"])
        .arg(&source)
        .args(["--target", "rust", "--out"])
        .arg(out);
    execute(&mut command)
}

fn generate_types(root: &Path, ess: &Path, out: &Path) -> Result<()> {
    let mut command = Command::new(ess);
    command
        .current_dir(root)
        .args([
            "generate",
            "types",
            "--path",
            SPEC,
            "--root",
            "connectors_mcp.launch.LocalLaunchInput",
            "--root",
            "connectors_mcp.launch.LocalLaunchCompletion",
            "--root",
            "connectors_mcp.launch.LocalLaunchFailure",
            "--root",
            "connectors_mcp.protocol.LegacySessionPhase",
            "--target",
            "rust",
            "--package",
            "connectors-mcp-launch-types",
            "--out",
        ])
        .arg(out);
    execute(&mut command)
}

fn generate_configuration(root: &Path, ess: &Path, out: &Path) -> Result<()> {
    let mut command = Command::new(ess);
    command
        .current_dir(root)
        .args([
            "generate",
            "types",
            "--path",
            SPEC,
            "--root",
            "connectors_mcp.local_server.Configuration",
            "--target",
            "rust",
            "--package",
            "connectors-mcp-local-server-types",
            "--out",
        ])
        .arg(out);
    execute(&mut command)
}

fn artifacts(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = connectors_spec::v2::tree(path)?;
    // ESS's local output-ownership ledger is not a generated public artifact.
    files.retain(|name, _| !name.starts_with(".ess-output/"));
    Ok(files)
}

fn execute(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("MCP projection command failed: {status}").into());
    }
    Ok(())
}
