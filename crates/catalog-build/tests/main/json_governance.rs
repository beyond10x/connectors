//! Every repository-owned JSON document has one declared validation contract.
//!
//! The inventory is `json-schemas.toml`, not a directory convention: vendored source inputs get an
//! exact, syntax-only entry, while repository-authored documents name a local JSON Schema. Adding a
//! JSON file without adding one classification is therefore a failure rather than an implicit
//! exemption. JSON Schema documents are data too and validate against the meta-schema they declare.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    #[serde(default)]
    schema: Vec<SchemaEntry>,
    #[serde(default)]
    document: Vec<DocumentEntry>,
    #[serde(default)]
    generated: Vec<DocumentEntry>,
    #[serde(default)]
    syntax: Vec<SyntaxEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaEntry {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentEntry {
    path: Option<String>,
    pattern: Option<String>,
    schema: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyntaxEntry {
    path: String,
    origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Class<'a> {
    Schema,
    Document(&'a str),
    Syntax(&'a str),
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate is two directories below the repository root")
        .to_path_buf()
}

fn load_registry(root: &Path) -> Registry {
    let text = std::fs::read_to_string(root.join("json-schemas.toml"))
        .expect("the JSON governance inventory exists");
    toml::from_str(&text).expect("the JSON governance inventory is valid and closed TOML")
}

fn repository_json(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.json",
        ])
        .current_dir(root)
        .output()
        .expect("git enumerates tracked JSON");
    assert!(
        output.status.success(),
        "git JSON inventory failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8(path.to_vec()).expect("repository paths are UTF-8"))
        .collect()
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        return pattern == path;
    };
    assert!(
        !suffix.contains('*'),
        "JSON inventory patterns admit one `*`, got `{pattern}`"
    );
    path.starts_with(prefix)
        && path.ends_with(suffix)
        && path[prefix.len()..path.len() - suffix.len()]
            .find('/')
            .is_none()
}

fn document_matches<'a>(entry: &'a DocumentEntry, path: &str) -> Option<Class<'a>> {
    match (&entry.path, &entry.pattern) {
        (Some(exact), None) if exact == path => Some(Class::Document(&entry.schema)),
        (None, Some(pattern)) if pattern_matches(pattern, path) => {
            Some(Class::Document(&entry.schema))
        }
        (Some(_), None) | (None, Some(_)) => None,
        _ => panic!("a document classification must declare exactly one of `path` or `pattern`"),
    }
}

fn classes<'a>(registry: &'a Registry, path: &str) -> Vec<Class<'a>> {
    let mut classes = Vec::new();
    if registry.schema.iter().any(|entry| entry.path == path) {
        classes.push(Class::Schema);
    }
    for entry in &registry.document {
        if let Some(class) = document_matches(entry, path) {
            classes.push(class);
        }
    }
    for entry in &registry.syntax {
        if entry.path == path {
            classes.push(Class::Syntax(&entry.origin));
        }
    }
    classes
}

fn classification_errors(registry: &Registry, paths: &[String]) -> Vec<String> {
    let paths: BTreeSet<_> = paths.iter().map(String::as_str).collect();
    let schemas: BTreeSet<_> = registry
        .schema
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    let mut errors = Vec::new();
    for path in &paths {
        match classes(registry, path) {
            found if found.is_empty() => errors.push(format!("unclassified JSON: {path}")),
            found if found.len() > 1 => {
                errors.push(format!("JSON has {} classifications: {path}", found.len()))
            }
            _ => {}
        }
    }

    for entry in &registry.schema {
        if !paths.contains(entry.path.as_str()) {
            errors.push(format!(
                "schema classification names no tracked JSON: {}",
                entry.path
            ));
        }
    }
    for entry in &registry.syntax {
        if !paths.contains(entry.path.as_str()) {
            errors.push(format!(
                "syntax classification names no tracked JSON: {}",
                entry.path
            ));
        }
        if entry.origin != "vendored-source" {
            errors.push(format!(
                "syntax-only JSON must identify `vendored-source` origin: {}",
                entry.path
            ));
        }
    }
    for entry in &registry.document {
        if !schemas.contains(entry.schema.as_str()) {
            errors.push(format!(
                "document classification names unregistered schema: {}",
                entry.schema
            ));
        }
        if let Some(path) = &entry.path {
            if !paths.contains(path.as_str()) {
                errors.push(format!(
                    "document classification names no tracked JSON: {path}"
                ));
            }
        } else if let Some(pattern) = &entry.pattern {
            if !paths.iter().any(|path| pattern_matches(pattern, path)) {
                errors.push(format!(
                    "document pattern matches no tracked JSON: {pattern}"
                ));
            }
        }
    }
    for entry in &registry.generated {
        if !schemas.contains(entry.schema.as_str()) {
            errors.push(format!(
                "generated classification names unregistered schema: {}",
                entry.schema
            ));
        }
        match (&entry.path, &entry.pattern) {
            (Some(_), None) => {}
            _ => errors.push(
                "generated JSON classifications must declare one exact path, never a pattern"
                    .to_owned(),
            ),
        }
    }
    errors.sort();
    errors
}

fn parse_json_bytes(relative: &str, bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("`{relative}` is not JSON: {error}"))
}

fn parse_json(root: &Path, relative: &str) -> Value {
    let bytes = std::fs::read(root.join(relative))
        .unwrap_or_else(|error| panic!("cannot read `{relative}`: {error}"));
    parse_json_bytes(relative, &bytes).unwrap_or_else(|error| panic!("{error}"))
}

fn schemas(root: &Path, registry: &Registry) -> BTreeMap<String, Value> {
    registry
        .schema
        .iter()
        .map(|entry| (entry.path.clone(), parse_json(root, &entry.path)))
        .collect()
}

fn assert_valid(schema_path: &str, schema: &Value, document_path: &str, document: &Value) {
    let validator = jsonschema::draft202012::new(schema)
        .unwrap_or_else(|error| panic!("cannot compile schema `{schema_path}`: {error}"));
    let errors: Vec<_> = validator
        .iter_errors(document)
        .map(|error| error.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "`{document_path}` does not validate against `{schema_path}`:\n{}",
        errors.join("\n")
    );
}

#[test]
fn every_repository_json_is_classified_and_valid() {
    let root = repo_root();
    let registry = load_registry(&root);
    assert_eq!(
        registry.version, 1,
        "only JSON inventory version 1 is understood"
    );
    let repository_json = repository_json(&root);
    let errors = classification_errors(&registry, &repository_json);
    assert!(errors.is_empty(), "{}", errors.join("\n"));

    let schemas = schemas(&root, &registry);
    for (path, schema) in &schemas {
        let declared = schema
            .get("$schema")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                panic!("JSON Schema `{path}` must declare its `$schema` meta-schema")
            });
        assert_eq!(
            declared, "https://json-schema.org/draft/2020-12/schema",
            "JSON Schema `{path}` must declare the pinned Draft 2020-12 meta-schema"
        );
        jsonschema::draft202012::meta::validate(schema).unwrap_or_else(|error| {
            panic!("JSON Schema `{path}` is invalid against declared meta-schema `{declared}`: {error}")
        });
    }

    for path in &repository_json {
        let document = parse_json(&root, path);
        if let [Class::Document(schema_path)] = classes(&registry, path).as_slice() {
            let schema = schemas
                .get(*schema_path)
                .unwrap_or_else(|| panic!("`{path}` names unregistered schema `{schema_path}`"));
            assert_valid(schema_path, schema, path, &document);
        }
    }

    for entry in &registry.generated {
        let path = entry
            .path
            .as_deref()
            .unwrap_or_else(|| panic!("generated JSON must use an exact path"));
        assert!(
            entry.pattern.is_none(),
            "generated JSON cannot use a pattern"
        );
        if root.join(path).exists() {
            let document = parse_json(&root, path);
            let schema = schemas
                .get(&entry.schema)
                .unwrap_or_else(|| panic!("`{path}` names unregistered schema `{}`", entry.schema));
            assert_valid(&entry.schema, schema, path, &document);
        }
    }
}

#[test]
fn an_unclassified_json_file_fails_by_name() {
    let registry: Registry = toml::from_str("version = 1\n").unwrap();
    let errors = classification_errors(&registry, &["new-owned.json".to_owned()]);
    assert_eq!(errors, ["unclassified JSON: new-owned.json"]);
}

#[test]
fn an_invalid_owned_document_fails_its_schema() {
    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["answer"],
        "additionalProperties": false,
        "properties": {"answer": {"type": "integer"}}
    });
    let document = serde_json::json!({"answer": "not-an-integer"});
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let errors: Vec<_> = validator.iter_errors(&document).collect();
    assert!(
        !errors.is_empty(),
        "an invalid owned document must be refused"
    );
}

#[test]
fn malformed_vendored_json_fails_even_though_it_is_syntax_only() {
    let error = parse_json_bytes("specs/vendor/broken.json", br#"{"unfinished": true"#)
        .expect_err("syntax-only means parse it; malformed input must still fail");
    assert!(error.starts_with("`specs/vendor/broken.json` is not JSON:"));
}

#[test]
fn a_json_schema_invalid_against_its_declared_meta_schema_fails() {
    let invalid = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": 7
    });
    assert!(
        jsonschema::draft202012::meta::validate(&invalid).is_err(),
        "an invalid schema must not compile as repository governance"
    );
}
