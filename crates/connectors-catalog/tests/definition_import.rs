//! Every selection the catalog ships, imported into a definition and lowered
//! back: the bundle and the connector declaration come out byte for byte the
//! committed ones, and the selection equal to the shipped file as a JSON value,
//! every field of it. The committed definitions (`tests/fixtures/definitions/`,
//! the copy of the definitions repository's files) lower to the same files; their vendored
//! sources and provenance READMEs are the importer's output byte for byte. A
//! committed definition may say more than the importer can read from the shipped
//! files (paging walks, filters, references), so it is not compared with the
//! imported one.
#![cfg(unix)]
use connectors_catalog::{definition, import};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/definitions")
}

/// One shipped selection: the definition it imports into, its catalog
/// provider and the file, under `adapters/catalog/providers/`.
struct Shipped {
    name: &'static str,
    catalog: &'static str,
    selection: &'static str,
}

const SHIPPED: [Shipped; 15] = [
    Shipped {
        name: "confluence",
        catalog: "confluence",
        selection: "confluence/operations.json",
    },
    Shipped {
        name: "confluence-v1",
        catalog: "confluence-v1",
        selection: "confluence-v1/operations.json",
    },
    Shipped {
        name: "gitlab",
        catalog: "gitlab",
        selection: "gitlab/operations.json",
    },
    Shipped {
        name: "google-calendar",
        catalog: "google-calendar",
        selection: "google-calendar/operations.json",
    },
    Shipped {
        name: "google-drive",
        catalog: "google-drive",
        selection: "google-drive/operations.json",
    },
    Shipped {
        name: "google-gmail",
        catalog: "google-gmail",
        selection: "google-gmail/operations.json",
    },
    Shipped {
        name: "google-slides",
        catalog: "google-slides",
        selection: "google-slides/operations.json",
    },
    Shipped {
        name: "hubspot",
        catalog: "hubspot",
        selection: "hubspot/operations.json",
    },
    Shipped {
        name: "jira",
        catalog: "jira",
        selection: "jira/operations.json",
    },
    Shipped {
        name: "openrouter",
        catalog: "openrouter",
        selection: "openrouter/operations.json",
    },
    Shipped {
        name: "opsgenie",
        catalog: "opsgenie",
        selection: "opsgenie/operations.json",
    },
    Shipped {
        name: "runpod",
        catalog: "runpod",
        selection: "runpod/operations.json",
    },
    Shipped {
        name: "slack",
        catalog: "slack",
        selection: "slack/operations.json",
    },
    Shipped {
        name: "slack-user",
        catalog: "slack",
        selection: "slack/user-operations.json",
    },
    Shipped {
        name: "zendesk",
        catalog: "zendesk",
        selection: "zendesk/operations.json",
    },
];

fn shipped(name: &str) -> &'static Shipped {
    SHIPPED.iter().find(|s| s.name == name).unwrap()
}

/// Import `name` into a fresh definitions root under `temp`.
fn imported(temp: &Path, name: &str) -> (PathBuf, import::Imported) {
    let entry = shipped(name);
    let root = temp.join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let selection = repository()
        .join("adapters/catalog/providers")
        .join(entry.selection);
    let imported = import::import(&import::Request {
        connectors: &repository(),
        catalog: entry.catalog,
        selection: Some(&selection),
        definitions: &root,
        name: Some(entry.name),
    })
    .unwrap_or_else(|failure| panic!("`{name}` imports: {failure}"));
    assert_eq!(imported.name, name);
    assert_eq!(
        imported.definition,
        root.join("connectors").join(format!("{name}.yaml"))
    );
    (root, imported)
}

/// Lower `file` against `root` and hold the two lowered files against the
/// committed bundle and the shipped selection.
fn assert_lowers_to_the_shipped_files(temp: &Path, name: &str, file: &Path, root: &Path) {
    let entry = shipped(name);
    let out = temp.join("out");
    let scratch = temp.join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&definition::Request {
        definition: file,
        root,
        out: &out,
        scratch: &scratch,
    })
    .unwrap_or_else(|refusal| panic!("`{name}` lowers: {refusal}"));
    assert_eq!(lowered.provider, entry.catalog);
    assert_eq!(
        lowered.bundle,
        out.join(format!("{}.bundle.json", entry.catalog))
    );
    assert_eq!(lowered.operations, out.join(entry.selection));
    let committed = std::fs::read(
        repository()
            .join("adapters/catalog/generated/bundles")
            .join(format!("{}.bundle.json", entry.catalog)),
    )
    .unwrap();
    assert!(
        std::fs::read(&lowered.bundle).unwrap() == committed,
        "`{name}`: the lowered bundle differs from the committed one"
    );
    let written: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let expected: Value = serde_json::from_slice(
        &std::fs::read(
            repository()
                .join("adapters/catalog/providers")
                .join(entry.selection),
        )
        .unwrap(),
    )
    .unwrap();
    // Field by field first, so a difference names its operation and field;
    // then the whole value, so nothing outside the operations escapes.
    let written_operations = written["operations"].as_array().unwrap();
    let expected_operations = expected["operations"].as_array().unwrap();
    for (index, (got, want)) in written_operations
        .iter()
        .zip(expected_operations)
        .enumerate()
    {
        assert_eq!(got["id"], want["id"], "`{name}` operation {index} id");
        for (field, value) in want.as_object().unwrap() {
            assert_eq!(
                got.get(field),
                Some(value),
                "`{name}` `{}` {field}",
                want["id"]
            );
        }
        for field in got.as_object().unwrap().keys() {
            assert!(
                want.get(field).is_some(),
                "`{name}` `{}` lowers `{field}`, which the shipped file lacks",
                want["id"]
            );
        }
    }
    assert_eq!(
        written_operations.len(),
        expected_operations.len(),
        "`{name}` operation count"
    );
    assert_eq!(written.get("feed"), expected.get("feed"), "`{name}` feed");
    assert_eq!(written, expected, "`{name}`: the whole selection");

    let declaration = repository()
        .join("adapters/catalog/providers")
        .join(entry.catalog)
        .join("connector.json");
    let declared =
        entry.selection == format!("{}/operations.json", entry.catalog) && declaration.exists();
    assert_eq!(
        declared,
        !UNDECLARED.contains(&name),
        "`{name}`: a committed declaration where, and only where, one is expected"
    );
    if declared {
        assert_eq!(
            lowered.connector,
            out.join(entry.catalog).join("connector.json")
        );
        let written = std::fs::read_to_string(&lowered.connector).unwrap();
        assert!(
            written == std::fs::read_to_string(&declaration).unwrap(),
            "`{name}`: the lowered connector.json differs from the committed one:\n{written}"
        );
    }
}

/// The selections with no committed connector declaration: Slack's two run as
/// two instances of one catalog directory, which ships none.
const UNDECLARED: [&str; 3] = ["openrouter", "slack", "slack-user"];

fn round_trip(name: &str) {
    let temp = tempfile::tempdir().unwrap();
    let (root, imported) = imported(temp.path(), name);
    assert_lowers_to_the_shipped_files(temp.path(), name, &imported.definition, &root);
}

/// The committed copy of the definitions repository's files for `name` lowers to
/// the shipped files, and its upstream README is the importer's output.
fn committed(name: &str) {
    let temp = tempfile::tempdir().unwrap();
    let (root, imported) = imported(temp.path(), name);
    let fixtures = fixtures();
    let mut compared = 0;
    for written in &imported.written {
        let relative = written.strip_prefix(&root).unwrap();
        let provenance =
            relative.starts_with("upstream") && relative.extension().is_some_and(|e| e == "md");
        if !provenance {
            continue;
        }
        let fixture = fixtures.join(relative);
        let committed = std::fs::read(&fixture)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        assert!(
            std::fs::read(written).unwrap() == committed,
            "`{name}`: {} is not the importer's output",
            relative.display()
        );
        compared += 1;
    }
    assert!(compared >= 1, "`{name}`: its upstream README is compared");
    // Lower the committed file itself, in a root whose sources the importer wrote.
    let lowered_root = temp.path().join("committed");
    copy_tree(&root, &lowered_root);
    for relative in [PathBuf::from("connectors").join(format!("{name}.yaml"))]
        .into_iter()
        .chain(auth_files())
    {
        std::fs::create_dir_all(lowered_root.join(&relative).parent().unwrap()).unwrap();
        std::fs::copy(fixtures.join(&relative), lowered_root.join(&relative)).unwrap();
    }
    assert_lowers_to_the_shipped_files(
        temp.path(),
        name,
        &lowered_root.join("connectors").join(format!("{name}.yaml")),
        &lowered_root,
    );
}

fn auth_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(fixtures().join("auth"))
        .unwrap()
        .map(|entry| PathBuf::from("auth").join(entry.unwrap().file_name()))
        .collect();
    files.sort();
    files
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

macro_rules! connectors {
    ($($test:ident, $committed:ident => $name:literal;)*) => {
        $(
            #[test]
            fn $test() {
                round_trip($name);
            }
            #[test]
            fn $committed() {
                committed($name);
            }
        )*
        /// Every shipped selection file has its case above, and no other.
        #[test]
        fn every_shipped_selection_is_covered() {
            let names = [$($name),*];
            assert_eq!(names.len(), SHIPPED.len());
            let mut files: Vec<String> = Vec::new();
            for provider in std::fs::read_dir(repository().join("adapters/catalog/providers")).unwrap() {
                let provider = provider.unwrap();
                for file in std::fs::read_dir(provider.path()).unwrap() {
                    let file = file.unwrap().file_name().into_string().unwrap();
                    if file.ends_with("operations.json") {
                        files.push(format!("{}/{file}", provider.file_name().into_string().unwrap()));
                    }
                }
            }
            files.sort();
            let mut covered: Vec<String> = SHIPPED.iter().map(|s| s.selection.to_owned()).collect();
            covered.sort();
            assert_eq!(files, covered);
            for name in names {
                shipped(name);
            }
        }
    };
}

connectors! {
    confluence_round_trips, confluence_is_committed => "confluence";
    confluence_v1_round_trips, confluence_v1_is_committed => "confluence-v1";
    gitlab_round_trips, gitlab_is_committed => "gitlab";
    google_calendar_round_trips, google_calendar_is_committed => "google-calendar";
    google_drive_round_trips, google_drive_is_committed => "google-drive";
    google_gmail_round_trips, google_gmail_is_committed => "google-gmail";
    google_slides_round_trips, google_slides_is_committed => "google-slides";
    hubspot_round_trips, hubspot_is_committed => "hubspot";
    jira_round_trips, jira_is_committed => "jira";
    openrouter_round_trips, openrouter_is_committed => "openrouter";
    opsgenie_round_trips, opsgenie_is_committed => "opsgenie";
    runpod_round_trips, runpod_is_committed => "runpod";
    slack_round_trips, slack_is_committed => "slack";
    slack_user_round_trips, slack_user_is_committed => "slack-user";
    zendesk_round_trips, zendesk_is_committed => "zendesk";
}

/// Importing twice into the same root writes the same bytes.
#[test]
fn an_import_is_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    let (root, first) = imported(temp.path(), "google-drive");
    let before: Vec<Vec<u8>> = first
        .written
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    let again = import::import(&import::Request {
        connectors: &repository(),
        catalog: "google-drive",
        selection: None,
        definitions: &root,
        name: None,
    })
    .unwrap();
    assert_eq!(again.written, first.written);
    let after: Vec<Vec<u8>> = again
        .written
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    assert!(before == after, "a second import wrote other bytes");
}

/// The committed definition of `name`, edited by `edit`, lowered in a root the
/// importer wrote; the refusal it gives.
fn refused(name: &str, edit: impl Fn(&str) -> String) -> definition::Refusal {
    let temp = tempfile::tempdir().unwrap();
    let (root, _) = imported(temp.path(), name);
    std::fs::create_dir_all(root.join("auth")).unwrap();
    for relative in auth_files() {
        std::fs::copy(fixtures().join(&relative), root.join(&relative)).unwrap();
    }
    let text = std::fs::read_to_string(fixtures().join(format!("connectors/{name}.yaml"))).unwrap();
    let file = root.join(format!("connectors/{name}.yaml"));
    std::fs::write(&file, edit(&text)).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    definition::lower(&definition::Request {
        definition: &file,
        root: &root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    })
    .expect_err("the edited definition is refused")
}

fn reference_detail(refusal: definition::Refusal) -> String {
    match refusal {
        definition::Refusal::Reference { detail, .. } => detail,
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_reference_url_on_another_origin_is_refused() {
    let detail = reference_detail(refused("jira", |t| {
        t.replace(
            "https://{domain}.atlassian.net/browse/",
            "https://evil.example/browse/",
        )
    }));
    assert!(detail.contains("is not on base_url's origin"), "{detail}");
}

#[test]
fn a_pattern_the_linear_time_engine_refuses_is_refused() {
    let detail = reference_detail(refused("jira", |t| {
        t.replace("\\\\b(?P<key>", "(?=x)(?P<key>")
    }));
    assert!(detail.starts_with("pattern"), "{detail}");
}

#[test]
fn a_value_not_every_match_binds_is_refused() {
    let detail = reference_detail(refused("gitlab", |t| {
        t.replace("(?P<project>", "(?P<path>")
    }));
    assert!(
        detail.contains("`project` is not bound by every match"),
        "{detail}"
    );
}

#[test]
fn a_reference_resolving_through_a_write_is_refused() {
    let detail = reference_detail(refused("gitlab", |t| {
        t.replace(
            "resolve: {operation: merge_request.get,",
            "resolve: {operation: merge_request.update,",
        )
    }));
    assert!(detail.contains("not a read"), "{detail}");
}

#[test]
fn an_unbound_path_parameter_is_refused() {
    let detail = reference_detail(refused("jira", |t| {
        t.replace(
            "with: {issueIdOrKey: \"{key}\"}",
            "with: {fields: \"{key}\"}",
        )
    }));
    assert!(detail.contains("path parameter `issueIdOrKey`"), "{detail}");
}

#[test]
fn a_view_of_an_undeclared_reference_is_refused() {
    match refused("jira", |t| t.replace("    of: issue\n", "    of: epic\n")) {
        definition::Refusal::ViewReference { view, of } => {
            assert_eq!(view, "issue.card");
            assert_eq!(of, "epic");
        }
        other => panic!("refused as {other:?}"),
    }
}
