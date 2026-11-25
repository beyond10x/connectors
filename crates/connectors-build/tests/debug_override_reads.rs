//! A wait that a test build may replace is read through `connectors_sdk::debug_override_ms` and
//! nowhere else (`story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`).
//! The helper is the one place that is shown to ignore the variable in a build without
//! `debug_assertions`; a direct `std::env::var` of an override is outside that proof.
//!
//! This is a scan of the source text. It sees a name written as a literal in the call, and a
//! constant the same file defines from such a name. It does not follow a name through a variable
//! or across files, and it does not tell a comment from code.
//!
//! Forms it does not see, none of them in the sources when it was written: a literal after a
//! comment inside the call; a literal inside a conversion (`var_os(OsStr::new("…"))`); `var`
//! imported under another name; a search of `std::env::vars()` or `vars_os()` for the name.
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Every override added since the helper carries this prefix.
const PREFIX: &str = "CONNECTORS_TEST_OVERRIDE_";
/// The three overrides older than the prefix.
const OLDER: [&str; 3] = [
    "CONNECTORS_TEST_OWNER_IDLE_MS",
    "CONNECTORS_TEST_OWNER_STOP_MS",
    "CONNECTORS_TEST_ACQUISITION_MS",
];
/// The file of the helper, the one file that may read an override.
const HELPER: &str = "crates/connectors-sdk/src/lib.rs";
/// This module. It holds direct reads as text and lies outside every `src/` directory.
const THIS_MODULE: &str = "crates/connectors-build/tests/debug_override_reads.rs";

fn is_override(name: &str) -> bool {
    name.starts_with(PREFIX) || OLDER.contains(&name)
}

/// The direct reads of an override in one source text: the line of each call of `var` or
/// `var_os` whose first argument is an override's name or a constant this text defines from
/// one, and that name.
fn direct_reads(text: &str) -> Vec<(usize, String)> {
    let constant = Regex::new(
        r#"\b(?:const|static)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&\s*(?:'static\s+)?str\s*=\s*"([^"]*)""#,
    )
    .unwrap();
    let constants: Vec<(String, String)> = constant
        .captures_iter(text)
        .filter(|found| is_override(&found[2]))
        .map(|found| (found[1].to_string(), found[2].to_string()))
        .collect();
    let call =
        Regex::new(r#"\bvar(?:_os)?\s*\(\s*&?\s*(?:"([^"]*)"|([A-Za-z_][A-Za-z0-9_:]*))"#).unwrap();
    let mut reads = Vec::new();
    for found in call.captures_iter(text) {
        let name = match (found.get(1), found.get(2)) {
            (Some(literal), _) if is_override(literal.as_str()) => literal.as_str().to_string(),
            (_, Some(path)) => {
                let last = path.as_str().rsplit("::").next().unwrap_or_default();
                match constants.iter().find(|(constant, _)| constant == last) {
                    Some((_, name)) => name.clone(),
                    None => continue,
                }
            }
            _ => continue,
        };
        let start = found.get(0).unwrap().start();
        reads.push((1 + text[..start].matches('\n').count(), name));
    }
    reads
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file below `directory` that lies in a directory named `src`, as a path from the
/// repository root. Build output, dependencies of the website and hidden directories are left
/// out; a symbolic link is not followed.
fn sources(root: &Path, directory: &Path, in_src: bool, found: &mut Vec<String>) {
    let mut entries: Vec<_> = fs::read_dir(root.join(directory))
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.unwrap())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let kind = entry.file_type().unwrap();
        let path = directory.join(&*name);
        if kind.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            sources(root, &path, in_src || name == "src", found);
        } else if kind.is_file() && in_src && name.ends_with(".rs") {
            found.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn no_debug_override_is_read_outside_the_helper() {
    let root = repository();
    let mut files = Vec::new();
    for directory in ["crates", "apps", "adapters"] {
        sources(&root, Path::new(directory), false, &mut files);
    }
    // The scan read the tree: the helper's file, the three files that held a direct read
    // before the helper and one file of an adapter are among the files, so each of the three
    // directories was read. This module with its own direct reads is not among them.
    for expected in [
        HELPER,
        "crates/connectors-host/src/local/owner/transport.rs",
        "crates/connectors-host/src/local/registry/lifecycle.rs",
        "apps/connectors/src/local/owner_stop.rs",
        "adapters/catalog/src/lib.rs",
    ] {
        assert!(
            files.iter().any(|file| file == expected),
            "{expected} was not read"
        );
    }
    assert!(!files.iter().any(|file| file == THIS_MODULE));
    let helper = fs::read_to_string(root.join(HELPER)).unwrap();
    assert!(
        helper.contains("pub fn debug_override_ms("),
        "{HELPER} no longer defines the helper"
    );
    let mut found = Vec::new();
    for file in files.iter().filter(|file| *file != HELPER) {
        let text = fs::read_to_string(root.join(file)).unwrap();
        for (line, name) in direct_reads(&text) {
            found.push(format!(
                "{file}:{line}: reads {name} from the environment; call connectors_sdk::debug_override_ms"
            ));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// A site as it was before the helper.
const DIRECT_READ: &str = r#"const BOUND: u64 = 30_000;
fn bound() -> u64 {
    std::env::var("CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(BOUND)
}
"#;

/// The same site on the helper.
const HELPER_CALL: &str = r#"const BOUND: u64 = 30_000;
fn bound() -> u64 {
    connectors_sdk::debug_override_ms("CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS", BOUND)
}
"#;

#[test]
fn a_direct_read_is_reported_with_its_line_and_a_call_of_the_helper_is_not() {
    assert_eq!(
        direct_reads(DIRECT_READ),
        [(3, "CONNECTORS_TEST_OVERRIDE_EXAMPLE_MS".to_string())]
    );
    assert_eq!(direct_reads(HELPER_CALL), Vec::<(usize, String)>::new());
}

/// The other forms of a direct read: an older name, `var_os`, a constant of the same text, and
/// an argument on the line after the call.
const OTHER_FORMS: &str = r#"const ACQUISITION_MS: &str = "CONNECTORS_TEST_ACQUISITION_MS";
pub(crate) const SWEEP: &'static str =
    "CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS";
fn waits() {
    let idle = std::env::var("CONNECTORS_TEST_OWNER_IDLE_MS");
    let stop = env::var_os("CONNECTORS_TEST_OWNER_STOP_MS");
    let acquisition = std::env::var(ACQUISITION_MS);
    let sweep = std::env::var_os(
        Self::SWEEP,
    );
}
"#;

/// What the scan leaves alone: a test variable that replaces no wait, the two delays a test
/// adds, a constant handed to the helper or set on a child, and a constant of another name.
const NOT_OVERRIDES: &str = r#"const IDLE: &str = "CONNECTORS_TEST_OWNER_IDLE_MS";
const PARENT: &str = "CONNECTORS_TEST_OWNER_PARENT";
fn other() {
    let parent = std::env::var_os(PARENT);
    let follow = std::env::var_os("CONNECTORS_TEST_OAUTH_FOLLOW");
    let start = std::env::var("CONNECTORS_TEST_OWNER_START_DELAY_MS");
    let launch = std::env::var("CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS");
    let idle = connectors_sdk::debug_override_ms(IDLE, 600_000);
    command.env(IDLE, "1500");
    let home = std::env::var(HOME);
}
"#;

#[test]
fn the_older_names_var_os_and_a_constant_are_reported_and_other_variables_are_not() {
    assert_eq!(
        direct_reads(OTHER_FORMS),
        [
            (5, "CONNECTORS_TEST_OWNER_IDLE_MS".to_string()),
            (6, "CONNECTORS_TEST_OWNER_STOP_MS".to_string()),
            (7, "CONNECTORS_TEST_ACQUISITION_MS".to_string()),
            (8, "CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS".to_string()),
        ]
    );
    assert_eq!(direct_reads(NOT_OVERRIDES), Vec::<(usize, String)>::new());
}
