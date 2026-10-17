//! One `connectors-definition/1` file lowered into what the catalog provider
//! runs: the Jira definition against its vendored source, and each refusal the
//! lowering raises, by name.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::Value;
use std::io::Write as _;
use std::path::{Path, PathBuf};

const JIRA_SHA256: &str = "655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/definition/providers/jira.yaml")
}

fn jira_text() -> String {
    std::fs::read_to_string(fixture()).unwrap()
}

struct Lowering {
    temp: tempfile::TempDir,
    outcome: Result<definition::Lowered, Refusal>,
}

impl Lowering {
    fn out(&self) -> PathBuf {
        self.temp.path().join("out")
    }
    fn refusal(self) -> Refusal {
        let out = self.out();
        let refusal = self.outcome.expect_err("the definition is refused");
        assert!(
            !out.join("jira.bundle.json").exists() && !out.join("jira.operations.json").exists(),
            "a refused definition writes nothing to the output directory"
        );
        refusal
    }
}

/// Lower `text` as `providers/jira.yaml`, its source resolved against `root`.
fn lower_in(text: &str, root: &Path) -> Lowering {
    let temp = tempfile::tempdir().unwrap();
    let providers = temp.path().join("providers");
    std::fs::create_dir_all(&providers).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let file = providers.join("jira.yaml");
    std::fs::write(&file, text).unwrap();
    let outcome = definition::lower(&Request {
        definition: &file,
        root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    });
    Lowering { temp, outcome }
}

fn lower(text: &str) -> Lowering {
    lower_in(text, &repository())
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bytes))
}

#[test]
fn the_jira_definition_lowers_to_the_committed_bundle_byte_for_byte() {
    let lowering = lower(&jira_text());
    let lowered = lowering
        .outcome
        .as_ref()
        .expect("the Jira definition lowers");
    assert_eq!(lowered.provider, "jira");
    assert_eq!(lowered.bundle, lowering.out().join("jira.bundle.json"));
    let written = std::fs::read(&lowered.bundle).unwrap();
    let committed =
        std::fs::read(repository().join("adapters/catalog/generated/bundles/jira.bundle.json"))
            .unwrap();
    assert!(
        written == committed,
        "the lowered bundle differs from the committed one"
    );
    let index = connectors_catalog::bundle::read_index(&lowering.out()).unwrap();
    let entry = index.find("jira").unwrap();
    assert_eq!(entry.bundle_sha256, sha256(&committed));
    assert_eq!(entry.source_sha256, JIRA_SHA256);
    connectors_catalog::bundle::load(&lowering.out(), "jira").expect("the index verifies");
}

#[test]
fn the_jira_selection_carries_the_shipped_id_operation_id_and_effect() {
    let lowering = lower(&jira_text());
    let lowered = lowering
        .outcome
        .as_ref()
        .expect("the Jira definition lowers");
    assert_eq!(
        lowered.operations,
        lowering.out().join("jira.operations.json")
    );
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    assert_eq!(selection["format"], "connectors-catalog-operations/1");
    assert_eq!(selection["provider"], "jira");
    let shipped: Value = serde_json::from_slice(
        &std::fs::read(repository().join("adapters/catalog/providers/jira/operations.json"))
            .unwrap(),
    )
    .unwrap();
    let entries = selection["operations"].as_array().unwrap();
    let ids: Vec<&str> = entries.iter().map(|e| e["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["issue.comments", "issue.get", "issues.search"]);
    for entry in entries {
        let id = entry["id"].as_str().unwrap();
        let expected = shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("operations.json lacks `{id}`"));
        assert_eq!(entry["operation_id"], expected["operation_id"], "`{id}`");
        assert_eq!(entry["effect"], expected["effect"], "`{id}`");
        assert!(
            entry["description"].as_str().is_some_and(|d| !d.is_empty()),
            "`{id}` takes the source's summary when the definition gives none"
        );
    }
}

#[test]
fn a_given_description_and_effect_are_kept() {
    let text = jira_text().replace(
        "issue.get: {op: getIssue}",
        "issue.get: {op: getIssue, effect: read, description: Read one issue}",
    );
    let lowering = lower(&text);
    let lowered = lowering.outcome.as_ref().expect("the definition lowers");
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let entry = selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "issue.get")
        .unwrap();
    assert_eq!(entry["description"], "Read one issue");
    assert_eq!(entry["effect"], "read");
}

#[test]
fn a_source_that_does_not_hash_to_its_digest_is_refused() {
    let text = jira_text().replace(JIRA_SHA256, &"0".repeat(64));
    match lower(&text).refusal() {
        Refusal::SourceHashMismatch { expected, actual } => {
            assert_eq!(expected, "0".repeat(64));
            assert_eq!(actual, JIRA_SHA256);
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn an_op_the_bundle_inventory_lacks_is_refused() {
    let text = jira_text().replace("{op: getIssue}", "{op: getIssueThatIsNotThere}");
    match lower(&text).refusal() {
        Refusal::UnknownOperation { key, op } => {
            assert_eq!(key, "issue.get");
            assert_eq!(op, "getIssueThatIsNotThere");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_base_url_variable_no_config_field_declares_is_refused() {
    let text = jira_text().replace(
        "https://{site}.atlassian.net",
        "https://{tenant}.atlassian.net",
    );
    assert!(matches!(
        lower(&text).refusal(),
        Refusal::UndeclaredVariable(name) if name == "tenant"
    ));
    let without_config = jira_text().replace("config:\n  site: {type: string}\n", "");
    assert!(matches!(
        lower(&without_config).refusal(),
        Refusal::UndeclaredVariable(name) if name == "site"
    ));
}

#[test]
fn a_non_get_operation_without_effect_is_refused() {
    let text = jira_text().replace(
        "operations:\n",
        "operations:\n  issue.transition.run: {op: doTransition}\n",
    );
    match lower(&text).refusal() {
        Refusal::EffectRequired { key, op, method } => {
            assert_eq!(key, "issue.transition.run");
            assert_eq!(op, "doTransition");
            assert_eq!(method, "post");
        }
        other => panic!("refused as {other:?}"),
    }
    let declared = jira_text().replace(
        "operations:\n",
        "operations:\n  issue.transition.run: {op: doTransition, effect: write}\n",
    );
    let lowering = lower(&declared);
    let lowered = lowering.outcome.as_ref().expect("a declared effect lowers");
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let entry = selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "issue.transition.run")
        .unwrap();
    assert_eq!(entry["effect"], "write");
}

#[test]
fn two_profiles_need_exactly_one_default_and_none_is_refused() {
    let bearer = "  atlassian.bearer: {scheme: bearer, label: Atlassian access token, identity: {path: myself, kind: atlassian.account}}\n";
    let two = insert_profile(&jira_text(), bearer);
    assert!(matches!(lower(&two).refusal(), Refusal::AuthDefault(0)));
    let both = insert_profile(
        &jira_text(),
        &bearer.replace("{scheme: bearer,", "{default: true, scheme: bearer,"),
    )
    .replace("{scheme: basic,", "{default: true, scheme: basic,");
    assert!(matches!(lower(&both).refusal(), Refusal::AuthDefault(2)));
    let marked = insert_profile(
        &jira_text(),
        &bearer.replace("{scheme: bearer,", "{default: true, scheme: bearer,"),
    );
    let lowering = lower(&marked);
    let lowered = lowering.outcome.as_ref().expect("one default lowers");
    let bundle: Value = serde_json::from_slice(&std::fs::read(&lowered.bundle).unwrap()).unwrap();
    assert_eq!(bundle["auth_profile"], "atlassian.bearer");
    let none = jira_text()
        .lines()
        .filter(|line| !line.starts_with("  atlassian.basic:"))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("auth:\n", "auth: {}\n");
    assert!(matches!(lower(&none).refusal(), Refusal::AuthDefault(0)));
}

/// `text` with `profile` added as the second entry of `auth`.
fn insert_profile(text: &str, profile: &str) -> String {
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        out.push_str(line);
        if line.starts_with("  atlassian.basic:") {
            out.push_str(profile);
        }
    }
    out
}

#[test]
fn an_unknown_field_is_refused() {
    for text in [
        jira_text().replace("operations:\n", "datasources: {}\noperations:\n"),
        jira_text().replace("{op: getIssue}", "{op: getIssue, retry: {}}"),
        jira_text().replace("{scheme: basic,", "{scheme: basic, token: x,"),
        jira_text().replace("{type: string}", "{type: string, secret: true}"),
        jira_text().replace("  sha256:", "  format: openapi\n  sha256:"),
    ] {
        assert!(
            matches!(lower(&text).refusal(), Refusal::UnknownField(_)),
            "{text}"
        );
    }
}

#[test]
fn a_schema_other_than_v1_is_refused() {
    let text = jira_text().replace(
        "urn:beyond10x:connectors-definition:1",
        "urn:beyond10x:connectors-definition:2",
    );
    assert!(matches!(
        lower(&text).refusal(),
        Refusal::WrongSchema(schema) if schema == "urn:beyond10x:connectors-definition:2"
    ));
}

#[test]
fn a_value_outside_the_schema_vocabulary_is_malformed() {
    for text in [
        jira_text().replace("{scheme: basic,", "{scheme: oauth2,"),
        jira_text().replace("{op: getIssue}", "{op: getIssue, effect: delete}"),
        jira_text().replace("{type: string}", "{type: integer}"),
        jira_text().replace(JIRA_SHA256, &JIRA_SHA256.to_uppercase()),
        jira_text().replace("base_url: https://{site}.atlassian.net\n", ""),
        "schema: [".to_owned(),
    ] {
        assert!(
            matches!(lower(&text).refusal(), Refusal::Malformed(_)),
            "{text}"
        );
    }
}

#[test]
fn a_definition_that_cannot_be_read_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let outcome = definition::lower(&Request {
        definition: &temp.path().join("providers/absent.yaml"),
        root: &repository(),
        out: &temp.path().join("out"),
        scratch: temp.path(),
    });
    assert!(matches!(outcome, Err(Refusal::Unreadable(_))));
}

#[test]
fn a_source_outside_the_definitions_root_is_refused() {
    for path in [
        "../connectors/adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz",
        "/etc/jira-platform-v3.json.gz",
    ] {
        let text = jira_text().replace(
            "adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz",
            path,
        );
        assert!(
            matches!(lower(&text).refusal(), Refusal::SourceOutsideRoot(_)),
            "{path}"
        );
    }
}

#[test]
fn a_source_that_is_not_gzip_is_refused() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("plain.json.gz"), b"{}").unwrap();
    std::fs::write(root.path().join("plain.json"), b"{}").unwrap();
    for (name, digest) in [
        ("plain.json.gz", sha256(b"{}")),
        ("plain.json", sha256(b"{}")),
    ] {
        let text = jira_text()
            .replace(
                "adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz",
                name,
            )
            .replace(JIRA_SHA256, &digest);
        assert!(
            matches!(
                lower_in(&text, root.path()).refusal(),
                Refusal::SourceNotGzip(_)
            ),
            "{name}"
        );
    }
}

#[test]
fn a_source_that_decompresses_past_the_ingest_limit_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let block = vec![b' '; 1024 * 1024];
    for _ in 0..=connectors_catalog::SOURCE_LIMIT / block.len() {
        encoder.write_all(&block).unwrap();
    }
    std::fs::write(root.path().join("large.json.gz"), encoder.finish().unwrap()).unwrap();
    let text = jira_text().replace(
        "adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz",
        "large.json.gz",
    );
    assert!(matches!(
        lower_in(&text, root.path()).refusal(),
        Refusal::SourceTooLarge(_)
    ));
}

#[test]
fn a_source_the_pipeline_refuses_is_refused_with_its_step() {
    let root = tempfile::tempdir().unwrap();
    let document = br#"{"swagger":"2.0","info":{"title":"Fixture","version":"1"},"paths":{}}"#;
    std::fs::write(root.path().join("swagger.json.gz"), gzip(document)).unwrap();
    let text = jira_text()
        .replace(
            "adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz",
            "swagger.json.gz",
        )
        .replace(JIRA_SHA256, &sha256(document));
    match lower_in(&text, root.path()).refusal() {
        Refusal::Pipeline(message) => {
            assert!(message.starts_with("ingest step refused"), "{message}")
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn an_output_directory_that_cannot_be_written_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("jira.yaml");
    std::fs::write(&file, jira_text()).unwrap();
    let out = temp.path().join("out");
    std::fs::write(&out, b"not a directory").unwrap();
    let outcome = definition::lower(&Request {
        definition: &file,
        root: &repository(),
        out: &out,
        scratch: temp.path(),
    });
    assert!(
        matches!(outcome, Err(Refusal::Write(_))),
        "{:?}",
        outcome.map(|l| l.provider)
    );
}

#[test]
fn every_refusal_has_a_distinct_name() {
    let names = [
        Refusal::Unreadable(String::new()).name(),
        Refusal::Malformed(String::new()).name(),
        Refusal::UnknownField(String::new()).name(),
        Refusal::WrongSchema(String::new()).name(),
        Refusal::AuthDefault(0).name(),
        Refusal::AuthLibrary(String::new()).name(),
        Refusal::AuthProfileMissing {
            key: String::new(),
            library: String::new(),
        }
        .name(),
        Refusal::AuthIncomplete {
            key: String::new(),
            field: String::new(),
        }
        .name(),
        Refusal::UndeclaredVariable(String::new()).name(),
        Refusal::SourceOutsideRoot(String::new()).name(),
        Refusal::SourceNotGzip(String::new()).name(),
        Refusal::SourceTooLarge(String::new()).name(),
        Refusal::SourceHashMismatch {
            expected: String::new(),
            actual: String::new(),
        }
        .name(),
        Refusal::UnknownOperation {
            key: String::new(),
            op: String::new(),
        }
        .name(),
        Refusal::EffectRequired {
            key: String::new(),
            op: String::new(),
            method: String::new(),
        }
        .name(),
        Refusal::UnknownPaging {
            key: String::new(),
            style: String::new(),
        }
        .name(),
        Refusal::AmbiguousPaging {
            key: String::new(),
            styles: Vec::new(),
        }
        .name(),
        Refusal::PagingParameter {
            key: String::new(),
            style: String::new(),
            parameter: String::new(),
        }
        .name(),
        Refusal::UnknownGuardRead {
            key: String::new(),
            read: String::new(),
        }
        .name(),
        Refusal::GuardParameter {
            key: String::new(),
            parameter: String::new(),
        }
        .name(),
        Refusal::BodyType {
            key: String::new(),
            field: String::new(),
            spec: String::new(),
        }
        .name(),
        Refusal::Pipeline(String::new()).name(),
        Refusal::Write(String::new()).name(),
    ];
    let distinct: std::collections::BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(distinct.len(), names.len());
}
