//! A fresh checkout has the committed generated output and no ESS ownership record (`.ess-output/`
//! is git-ignored). The documented regenerate commands must work there after an ESS source change
//! without a manual adoption step, adopting only bytes the pinned ESS itself generates and never
//! overwriting anything else. Needs the pinned `ess` (`CONNECTORS_ESS`); no network.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SERVICE_WIRE_OUTPUT: &str = "crates/connectors-core/generated/operation-types";
const LAUNCH_TYPES: &str = "adapters/mcp/generated/launch-types";
const CONFIGURATION_TYPES: &str = "adapters/mcp/generated/configuration-types";
const SUPERVISION: &str = "adapters/mcp/generated/supervision";
const LAUNCH_CLI: &str = "adapters/mcp/generated/launch-cli";
const CLI_CONTRACT: &str = "apps/connectors-cli-contract";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn ess() -> PathBuf {
    connectors_spec::toolchain::resolve(None).unwrap()
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == ".ess-output" {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&source, &target);
        } else {
            fs::copy(&source, &target).unwrap();
        }
    }
}

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// A committed checkout of the inputs and committed generated output, without ownership records.
fn fresh_checkout() -> tempfile::TempDir {
    let source = repository();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for path in [
        "ess",
        "adapters/mcp/spec",
        "apps/connectors/spec",
        "contracts/cli/v1alpha1",
        "crates/connectors-core/tests/vectors/v1alpha2",
        SERVICE_WIRE_OUTPUT,
        LAUNCH_TYPES,
        CONFIGURATION_TYPES,
        SUPERVISION,
        LAUNCH_CLI,
        CLI_CONTRACT,
    ] {
        copy_tree(&source.join(path), &root.join(path));
    }
    for (program, args) in [
        ("init", vec!["-q"]),
        ("add", vec!["-A"]),
        (
            "-c",
            vec![
                "user.name=t",
                "-c",
                "user.email=t@example.invalid",
                "commit",
                "-q",
                "-m",
                "fresh checkout",
            ],
        ),
    ] {
        run(Command::new("git")
            .current_dir(root)
            .arg(program)
            .args(args));
    }
    temp
}

fn edit(root: &Path, file: &str, from: &str, to: &str) {
    let path = root.join(file);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{file} lacks {from:?}");
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}

fn changed_ess_source(root: &Path) {
    edit(
        root,
        "ess/domains/service_wire.yaml",
        "summary: Public service envelope",
        "summary: Revised public service envelope",
    );
    edit(
        root,
        "adapters/mcp/spec/ess/domains/local_server.yaml",
        "summary: Explicit local deployment",
        "summary: Revised explicit local deployment",
    );
    edit(
        root,
        "adapters/mcp/spec/ess/domains/protocol.yaml",
        "summary: MCP-owned protocol values",
        "summary: Revised MCP-owned protocol values",
    );
    edit(
        root,
        "apps/connectors/spec/cli.yaml",
        "about: Local integration access",
        "about: Revised local integration access",
    );
    edit(
        root,
        "adapters/mcp/spec/cli.yaml",
        "about:",
        "about: Revised.",
    );
}

fn build(root: &Path, subcommand: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors-build"))
        .arg("--root")
        .arg(root)
        .arg("--ess")
        .arg(ess())
        .arg(subcommand)
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn read(root: &Path, relative: &str) -> Vec<u8> {
    fs::read(root.join(relative)).unwrap()
}

fn owned(root: &Path, destination: &str) -> bool {
    root.join(destination).join(".ess-output").is_dir()
}

#[test]
fn service_metadata_regenerates_after_a_source_change_in_a_fresh_checkout() {
    let temp = fresh_checkout();
    let root = temp.path();
    assert!(!owned(root, SERVICE_WIRE_OUTPUT));
    let before = read(root, &format!("{SERVICE_WIRE_OUTPUT}/types.rs"));
    changed_ess_source(root);
    let output = build(root, "service-metadata");
    assert!(output.status.success(), "{}", text(&output));
    assert!(owned(root, SERVICE_WIRE_OUTPUT));
    assert_ne!(
        read(root, &format!("{SERVICE_WIRE_OUTPUT}/types.rs")),
        before
    );
    let check = build(root, "service-metadata");
    assert!(check.status.success(), "{}", text(&check));
}

#[test]
fn mcp_bindings_regenerates_after_a_source_change_in_a_fresh_checkout() {
    let temp = fresh_checkout();
    let root = temp.path();
    let before = read(root, &format!("{CONFIGURATION_TYPES}/types.rs"));
    changed_ess_source(root);
    let output = build(root, "mcp-bindings");
    assert!(output.status.success(), "{}", text(&output));
    for destination in [LAUNCH_CLI, LAUNCH_TYPES, CONFIGURATION_TYPES, SUPERVISION] {
        assert!(owned(root, destination), "{destination} was not adopted");
    }
    assert_ne!(
        read(root, &format!("{CONFIGURATION_TYPES}/types.rs")),
        before
    );
}

#[test]
fn cli_regenerates_after_a_source_change_in_a_fresh_checkout() {
    let temp = fresh_checkout();
    let root = temp.path();
    let before = read(root, &format!("{CLI_CONTRACT}/help.txt"));
    changed_ess_source(root);
    let output = build(root, "cli");
    assert!(output.status.success(), "{}", text(&output));
    assert!(owned(root, CLI_CONTRACT));
    assert_ne!(read(root, &format!("{CLI_CONTRACT}/help.txt")), before);
}

#[test]
fn byte_identical_output_is_adopted_in_place_outside_git() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    copy_tree(&repository().join("ess"), &root.join("ess"));
    let reference = root.join("reference");
    run(Command::new(ess())
        .current_dir(root)
        .args([
            "generate",
            "types",
            "--path",
            "ess",
            "--root",
            "connectors.service_wire.OperationCuration",
            "--target",
            "rust",
            "--package",
            "connectors-operation-types",
            "--out",
        ])
        .arg(&reference));
    copy_tree(&reference, &root.join(SERVICE_WIRE_OUTPUT));
    let before = read(root, &format!("{SERVICE_WIRE_OUTPUT}/types.rs"));
    let output = build(root, "service-metadata");
    assert!(output.status.success(), "{}", text(&output));
    assert!(owned(root, SERVICE_WIRE_OUTPUT));
    assert_eq!(
        read(root, &format!("{SERVICE_WIRE_OUTPUT}/types.rs")),
        before
    );
}

#[test]
fn hand_written_output_is_refused_with_the_adoption_command_and_left_intact() {
    let temp = fresh_checkout();
    let root = temp.path();
    let types = format!("{SERVICE_WIRE_OUTPUT}/types.rs");
    let mut edited = read(root, &types);
    edited.extend_from_slice(b"// hand-written\n");
    fs::write(root.join(&types), &edited).unwrap();
    changed_ess_source(root);
    let output = build(root, "service-metadata");
    assert!(!output.status.success());
    let message = text(&output);
    assert!(message.contains("unowned output destination"), "{message}");
    assert!(
        message.contains(&format!(
            "generate output adopt --ownership-root {SERVICE_WIRE_OUTPUT}"
        )) && message.contains("--owner model-types"),
        "{message}"
    );
    assert_eq!(read(root, &types), edited);
    assert!(!owned(root, SERVICE_WIRE_OUTPUT));
}
