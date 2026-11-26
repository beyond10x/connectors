//! Adversary cases for `story:cli-spec-mapping-fixes` (wave 20261001b, unit commit
//! a63a7f9dd on base b19027874).
//!
//! The unit's own checks (`ess specify validate`, `cli --check`, `metadata-entities
//! --check`) prove the specification is well formed and that the generated outputs are a
//! fixed point of it. None of them compares `ess/domains/cli.yaml` with the owner wire
//! types in `crates/connectors-host/src/local/owner.rs`, or a citation with the line it
//! cites. These cases do: they read the host's `Request`/`Reply` field lists from source
//! and the spec's struct fields from the parsed domain document.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Field names of `enum <enum_name> { <variant> { ... } }` in `owner.rs`, read from source.
fn host_variant_fields(enum_name: &str, variant: &str) -> BTreeSet<String> {
    let source = read("crates/connectors-host/src/local/owner.rs");
    let start = source
        .find(&format!("enum {enum_name} {{"))
        .unwrap_or_else(|| panic!("enum {enum_name} not found"));
    let body = &source[start..];
    let open = body
        .find(&format!("    {variant} {{"))
        .unwrap_or_else(|| panic!("{enum_name}::{variant} not found"));
    let body = &body[open..];
    let close = body.find("\n    },").expect("variant close");
    let field = regex::Regex::new(r"^\s+([a-z_][a-z0-9_]*):\s").unwrap();
    body[..close]
        .lines()
        .skip(1)
        .filter(|line| !line.trim_start().starts_with("//") && !line.trim_start().starts_with('#'))
        .filter_map(|line| field.captures(line).map(|c| c[1].to_owned()))
        .collect()
}

fn cli_types() -> Vec<serde_yaml_ng::Value> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/cli.yaml")).unwrap();
    document["types"].as_sequence().unwrap().clone()
}

fn struct_fields(ty: &serde_yaml_ng::Value) -> BTreeSet<String> {
    ty["fields"]
        .as_sequence()
        .into_iter()
        .flatten()
        .map(|f| f["name"].as_str().unwrap().to_owned())
        .collect()
}

/// The fields a frame carries besides its serde `kind` tag, which the host's variant
/// does not list as a field (decided in the pass-2 correction).
fn payload_fields(ty: &serde_yaml_ng::Value) -> BTreeSet<String> {
    let mut fields = struct_fields(ty);
    fields.remove("kind");
    fields
}

fn field_type(ty: &serde_yaml_ng::Value, name: &str) -> String {
    ty["fields"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|f| f["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no field {name}"))["type"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn named(name: &str) -> serde_yaml_ng::Value {
    cli_types()
        .into_iter()
        .find(|t| t["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("{name} is not declared"))
}

/// Control: the reply greeting the owner sends is declared with exactly its fields.
#[test]
fn the_owner_greeting_reply_is_declared_with_the_hosts_fields() {
    let host = host_variant_fields("Reply", "Hello");
    assert_eq!(
        host,
        [
            "authority",
            "build",
            "challenge",
            "host_incarnation",
            "version"
        ]
        .map(String::from)
        .into(),
        "parser read the wrong Reply::Hello"
    );
    let greeting = named("connectors.cli.LocalOwnerGreeting");
    assert_eq!(payload_fields(&greeting), host);
    assert_eq!(
        field_type(&greeting, "kind"),
        "connectors.cli.LocalOwnerHelloKind",
        "the greeting declares its serde tag"
    );
    assert_eq!(
        field_type(&greeting, "build"),
        "Optional<connectors.cli.BuildDigest>",
        "the greeting carries the optional 64-hex build digest (decided in the pass-1 correction)"
    );
}

/// The greeting the CLI sends (`Request::Hello`) carries its own `build` and the
/// `configuration` path; acceptance cites that field. No spec type carries this shape.
#[test]
fn the_owner_greeting_request_the_cli_sends_is_declared() {
    let host = host_variant_fields("Request", "Hello");
    assert_eq!(
        host,
        [
            "authority",
            "build",
            "challenge",
            "configuration",
            "version"
        ]
        .map(String::from)
        .into(),
        "parser read the wrong Request::Hello"
    );
    let declared: Vec<String> = cli_types()
        .iter()
        .filter(|t| payload_fields(t) == host && struct_fields(t).contains("kind"))
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        !declared.is_empty(),
        "no connectors.cli struct declares the Request::Hello fields {host:?}"
    );
}

/// The owner writes `hex::encode(Sha256)` (64 lowercase hex) and the CLI's build probe
/// admits only a 64-character `build` (owner/transport.rs `probe_build`). ESS 0.45 accepts
/// a hex `alphabet:` newtype here (precedent: connectors.artifact_provenance.Sha256Hex),
/// and `generate cli` accepts it on these non-CLI types, yet both fields are bare `String`.
#[test]
fn the_build_digest_format_is_declared() {
    let types = cli_types();
    for (ty, wrapped) in [
        ("connectors.cli.LocalOwnerBuild", false),
        ("connectors.cli.LocalOwnerGreeting", true),
    ] {
        let declared = field_type(&named(ty), "build");
        let inner = if wrapped {
            declared
                .strip_prefix("Optional<")
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(&declared)
                .to_owned()
        } else {
            declared.clone()
        };
        let constrained = types.iter().any(|t| {
            t["name"].as_str() == Some(inner.as_str())
                && t["alphabet"]
                    .as_str()
                    .is_some_and(|a| a.chars().all(|c| c.is_ascii_hexdigit()) && a.contains('f'))
        });
        assert!(
            constrained,
            "{ty}.build is `{declared}`: the 64-hex digest format is not declared"
        );
    }
}

/// The lines of the one function `name` in `source`, the text of a Rust file: from the one
/// line that starts with `fn <name>(` to the next line that is `}` in the first column. It
/// takes the text and no line number, so it finds the function wherever it stands.
fn function_body<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
    let head = format!("fn {name}(");
    let lines: Vec<&str> = source.lines().collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with(&head))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "exactly one line starts with `{head}`; found at lines {:?}",
        starts.iter().map(|at| at + 1).collect::<Vec<_>>()
    );
    let start = starts[0];
    let length = lines[start..]
        .iter()
        .position(|line| *line == "}")
        .unwrap_or_else(|| panic!("no `}}` in the first column after `{head}`"));
    lines[start..=start + length].to_vec()
}

/// The index of the one line of `body` that holds `text`.
fn the_one_line_with(body: &[&str], text: &str, function: &str) -> usize {
    let at: Vec<usize> = body
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(text))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        at.len(),
        1,
        "exactly one line of `{function}` in er.rs holds {text}; its body reads:\n{}",
        body.join("\n")
    );
    at[0]
}

/// `spec` holds `citation` as one literal on one comment line.
fn assert_cites_by_name(spec: &str, citation: &str) {
    let text = read(spec);
    assert!(
        text.lines()
            .any(|line| line.trim_start().starts_with('#') && line.contains(citation)),
        "{spec} no longer holds the citation {citation} on one comment line"
    );
}

/// Every citation of `er.rs` by line (`er.rs:<digits>`) in the files under `directory`, as
/// `<path>:<line>: <text>`. It means the file `er.rs` itself: `owner.rs:` and `server.rs:`
/// do not match.
fn er_rs_line_citations(directory: &Path, found: &mut Vec<String>) {
    let cite = regex::Regex::new(r"(?:^|[^A-Za-z0-9_])er\.rs:[0-9]").unwrap();
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("{}: {e}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            er_rs_line_citations(&path, found);
            continue;
        }
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let shown = path.strip_prefix(root()).unwrap_or(&path).display();
        for (at, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
            if cite.is_match(line) {
                found.push(format!("{shown}:{}: {}", at + 1, line.trim()));
            }
        }
    }
}

/// A comment under `ess/` cites `er.rs` by function name, never by line: the lines of
/// `er.rs` move with every change above them, and `ess/domains/*.yaml` are inputs of the
/// committed metadata emissions, so a corrected line number costs a regeneration.
///
/// Two comments make a claim about the host's mapping of row changes to commands. For
/// each, the file still holds the citation by name, and the function it names still holds
/// the arm the comment speaks of, as text. Whoever changes such an arm (another state, a
/// second arm for the same command) fails this test and reads the comment again:
///
/// - `ess/domains/cli.yaml`: "the host removal path (metadata/er.rs `removal_command`)
///   only expires Active cursors".
/// - `ess/domains/auth_bindings.yaml`: "The host
///   (crates/connectors-host/src/local/metadata/er.rs `update_command`) maps no Revoked row
///   change to this command", which is `PublishBinding`.
///
/// A citation `er.rs:<digits>` in any file under `ess/` fails the test, which names the
/// file and the line.
#[test]
fn the_er_rs_citation_lands_on_the_cursor_expiry_path() {
    let mut by_line = Vec::new();
    er_rs_line_citations(&root().join("ess"), &mut by_line);
    assert!(
        by_line.is_empty(),
        "a comment under ess/ cites er.rs by line; cite the function by name instead \
         (metadata/er.rs `<function>`):\n{}",
        by_line.join("\n")
    );

    let er = read("crates/connectors-host/src/local/metadata/er.rs");

    assert_cites_by_name("ess/domains/cli.yaml", "metadata/er.rs `removal_command`");
    let body = function_body(&er, "removal_command");
    let arm = the_one_line_with(
        &body,
        r#""connectors.cli.ConnectionListCursor""#,
        "removal_command",
    );
    assert_eq!(
        body[arm].trim(),
        r#"("connectors.cli.ConnectionListCursor", "Active") => {"#,
        "cli.yaml says the host removal path only expires Active cursors"
    );
    let command = the_one_line_with(
        &body,
        r#""connectors.cli.ExpireConnectionListCursor""#,
        "removal_command",
    );
    assert_eq!(
        command,
        arm + 1,
        "the one line of removal_command that names ExpireConnectionListCursor follows the \
         arm for an Active cursor; it reads:\n{}",
        body[command]
    );

    assert_cites_by_name(
        "ess/domains/auth_bindings.yaml",
        "metadata/er.rs `update_command`",
    );
    let body = function_body(&er, "update_command");
    let command = the_one_line_with(
        &body,
        r#""connectors.auth_bindings.PublishBinding""#,
        "update_command",
    );
    assert_eq!(
        body[command - 1].trim(),
        r#"("connectors.auth_bindings.Connection", ("Live" | "Stale", "Live")) => {"#,
        "auth_bindings.yaml says the host maps no Revoked row change to PublishBinding"
    );
}

/// The body finder does not depend on where a function stands: two texts hold the same
/// function at different lines, each after another function with another body, and it
/// returns the same lines from both.
#[test]
fn the_body_finder_returns_the_same_function_from_two_texts() {
    let wanted = "\
fn wanted(state: &str) -> Option<&'static str> {
    match state {
        \"Active\" => Some(\"Expire\"),
        _ => None,
    }
}
";
    let other = "\
fn other() -> u8 {
    if true {
        7
    } else {
        8
    }
}
";
    let near = format!("{other}\n{wanted}\nfn wanted_later() {{}}\n");
    let far = format!(
        "use std::fmt;\n{}{other}\n// A comment.\n\n{wanted}",
        "\n".repeat(10)
    );
    let expected: Vec<&str> = wanted.lines().collect();
    assert_eq!(function_body(&near, "wanted"), expected);
    assert_eq!(function_body(&far, "wanted"), expected);
    assert_ne!(
        near.lines().position(|line| line == expected[0]),
        far.lines().position(|line| line == expected[0]),
        "the two texts hold the function at different lines"
    );
}

/// Control: the two semantics.md citations the unit corrected land on their rules.
#[test]
fn the_corrected_semantics_citations_land_on_their_rules() {
    let semantics = read("contracts/cli/v1alpha1/semantics.md");
    let lines: Vec<&str> = semantics.lines().collect();
    let spec = read("ess/domains/cli.yaml");
    let cite = regex::Regex::new(r"cli/v1alpha1/semantics\.md:(\d+)\b").unwrap();
    let cited: Vec<&str> = cite
        .captures_iter(&spec)
        .map(|c| lines[c[1].parse::<usize>().unwrap() - 1])
        .collect();
    for rule in ["500 maximum list items per page", "stale cursors refuse"] {
        assert!(
            cited.iter().any(|line| line.contains(rule)),
            "no semantics.md citation in cli.yaml lands on `{rule}`; cited lines:\n{}",
            cited.join("\n")
        );
    }
}
