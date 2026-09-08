//! Repository-owned ESS boundary checks. ESS remains the model/compiler authority.
use super::Result;
use serde::Deserialize;
use serde_yaml_ng::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    version: u32,
    shared_protocol_names: Vec<String>,
    forbidden_terms: Vec<String>,
}

// Split snake/kebab/path names and CamelCase, preserving acronym words. This
// avoids substring false positives such as "argo" in "cargo" or "sql" in "mysql".
fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut word = String::new();
    for (index, &ch) in chars.iter().enumerate() {
        if !ch.is_alphanumeric() {
            if !word.is_empty() {
                result.push(std::mem::take(&mut word));
            }
            continue;
        }
        let prev = index.checked_sub(1).map(|i| chars[i]);
        let next = chars.get(index + 1).copied();
        if ch.is_uppercase()
            && !word.is_empty()
            && (prev.is_some_and(|c| c.is_lowercase() || c.is_ascii_digit())
                || (prev.is_some_and(|c| c.is_uppercase())
                    && next.is_some_and(|c| c.is_lowercase())))
        {
            result.push(std::mem::take(&mut word));
        }
        word.extend(ch.to_lowercase());
    }
    if !word.is_empty() {
        result.push(word);
    }
    result
}

fn matching_term<'a>(text: &str, terms: &'a BTreeSet<Vec<String>>) -> Option<&'a Vec<String>> {
    let words = words(text);
    terms.iter().find(|term| {
        let expected = term.concat();
        // GitLab/LogQL and gitlab/logql are the same known terms. Match only
        // complete words, allowing different case/separator word boundaries.
        (0..words.len()).any(|start| {
            let mut candidate = String::new();
            for word in &words[start..] {
                candidate.push_str(word);
                if candidate == expected {
                    return true;
                }
                if candidate.len() >= expected.len() {
                    break;
                }
            }
            false
        })
    })
}

fn entries(path: &Path) -> Result<Vec<PathBuf>> {
    if !fs::symlink_metadata(path)?.is_dir() {
        return Err(format!("{}: expected a real directory, not a link", path.display()).into());
    }
    let mut entries = fs::read_dir(path)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn files(path: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let kind = fs::symlink_metadata(path)?.file_type();
    if kind.is_symlink() {
        return Err(format!("{}: ESS source links are forbidden", path.display()).into());
    }
    if kind.is_dir() {
        for entry in entries(path)? {
            files(&entry, output)?;
        }
    } else if kind.is_file() {
        output.push(path.to_owned());
    } else {
        return Err(format!("{}: ESS source must be a regular file", path.display()).into());
    }
    Ok(())
}

fn strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(s) => output.push(s.clone()),
        Value::Sequence(items) => {
            for item in items {
                strings(item, output);
            }
        }
        Value::Mapping(map) => {
            for (key, value) in map {
                strings(key, output);
                strings(value, output);
            }
        }
        Value::Tagged(tagged) => {
            output.push(tagged.tag.to_string());
            strings(&tagged.value, output);
        }
        _ => {}
    }
}

fn provider_paths(text: &str) -> bool {
    let text = text.replace('\\', "/");
    text.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`'))
        .any(|part| part.starts_with("adapters/") || part.contains("/adapters/"))
}

fn check_model(
    root: &Path,
    model: &Path,
    system: &str,
    terms: Option<&BTreeSet<Vec<String>>>,
) -> Result<()> {
    let mut sources = Vec::new();
    files(model, &mut sources)?;
    let mut errors = BTreeSet::new();
    let mut declared_domains = BTreeSet::new();
    let mut manifest_domains = BTreeSet::new();
    let mut has_system = false;
    for source in sources {
        let relative = source.strip_prefix(root)?;
        let name = relative.to_string_lossy();
        let local = source.strip_prefix(model)?;
        let text = fs::read_to_string(&source)?;
        if let Some(terms) = terms {
            if let Some(term) = matching_term(&name, terms) {
                errors.insert(format!("{name}: provider term in path: {}", term.join(" ")));
            }
            for (line, text) in text.lines().enumerate() {
                if let Some(term) = matching_term(text, terms) {
                    errors.insert(format!(
                        "{name}:{}: provider term: {}",
                        line + 1,
                        term.join(" ")
                    ));
                }
                if provider_paths(text) {
                    errors.insert(format!(
                        "{name}:{}: shared source references an adapter path",
                        line + 1
                    ));
                }
            }
        }
        if !matches!(
            source.extension().and_then(|s| s.to_str()),
            Some("yaml" | "yml")
        ) {
            continue;
        }
        if local != Path::new("system.yaml") && (local.parent() != Some(Path::new("domains"))) {
            errors.insert(format!(
                "{name}: model YAML must be system.yaml or domains/<name>.yaml"
            ));
        }
        let yaml: Value =
            serde_yaml_ng::from_str(&text).map_err(|e| format!("{name}: invalid YAML: {e}"))?;
        if local == Path::new("system.yaml") {
            has_system = true;
            if yaml["system"].as_str() != Some(system) {
                errors.insert(format!("{name}: expected system {system}"));
            }
            let domains = yaml["domains"]
                .as_sequence()
                .ok_or_else(|| format!("{name}: missing domain list"))?;
            for domain in domains {
                let domain = domain
                    .as_str()
                    .ok_or_else(|| format!("{name}: domain must be a string"))?;
                if !manifest_domains.insert(domain.to_owned()) {
                    errors.insert(format!("{name}: duplicate domain {domain}"));
                }
            }
        } else if let Some(domain) = yaml["domain"].as_str() {
            if !declared_domains.insert(domain.to_owned()) {
                errors.insert(format!("{name}: duplicate domain {domain}"));
            }
        } else {
            errors.insert(format!("{name}: expected a domain source"));
        }
        // Decode escaped scalars, map keys, aliases and tagged values as well as
        // scanning raw text/comments. Encoded provider names must not bypass lint.
        let mut values = Vec::new();
        strings(&yaml, &mut values);
        for value in values {
            if let Some(terms) = terms {
                if let Some(term) = matching_term(&value, terms) {
                    errors.insert(format!(
                        "{name}: decoded YAML contains provider term: {}",
                        term.join(" ")
                    ));
                }
                if provider_paths(&value) {
                    errors.insert(format!("{name}: decoded YAML references an adapter path"));
                }
            }
        }
    }
    if !has_system {
        errors.insert(format!("{}: missing system.yaml", model.display()));
    }
    if manifest_domains.is_empty() || manifest_domains != declared_domains {
        errors.insert(format!(
            "{}: manifest/source domains must match and be nonempty",
            model.display()
        ));
    }
    for domain in manifest_domains.union(&declared_domains) {
        if !domain
            .strip_prefix(&format!("{system}."))
            .is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            })
        {
            errors.insert(format!(
                "{}: foreign or invalid domain {domain}; expected {system}.<domain>",
                model.display()
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors.into_iter().collect::<Vec<_>>().join("\n").into());
    }
    Ok(())
}

fn check(root: &Path) -> Result<Vec<PathBuf>> {
    let policy: Policy = serde_json::from_slice(&fs::read(
        root.join("crates/connectors-build/ess-boundary.json"),
    )?)?;
    if policy.version != 1 || policy.forbidden_terms.is_empty() {
        return Err("unsupported or empty ESS boundary policy".into());
    }
    let mut terms = BTreeSet::new();
    for term in policy.forbidden_terms {
        let term = words(&term);
        if term.is_empty() || !terms.insert(term) {
            return Err("empty or duplicate ESS boundary term".into());
        }
    }
    // A protocol adapter can share its name with legitimate shared protocol
    // vocabulary. This never suppresses a forbidden native term or source path.
    let mut protocols = BTreeSet::new();
    for protocol in policy.shared_protocol_names {
        let protocol = words(&protocol);
        if protocol.is_empty()
            || terms.iter().any(|term| term.concat() == protocol.concat())
            || !protocols.insert(protocol.concat())
        {
            return Err("empty, duplicate or forbidden shared protocol name".into());
        }
    }
    // All adapter owners participate, including specifications without a runtime.
    // The optional declaration supplies an alias id; full adapter-spec schema
    // and semantic validation belong to the specification workflow.
    let mut models = vec![root.join("ess")];
    for adapter in entries(&root.join("adapters"))? {
        let kind = fs::symlink_metadata(&adapter)?.file_type();
        if adapter.file_name().and_then(|s| s.to_str()) == Some("README.md") && kind.is_file() {
            continue;
        }
        if !kind.is_dir() {
            return Err(format!("{}: expected a real adapter directory", adapter.display()).into());
        }
        let owner = adapter
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("non-UTF8 adapter owner")?;
        if owner.is_empty()
            || !owner
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(format!("invalid ESS adapter owner {owner}").into());
        }
        let owner_words = words(owner);
        if !protocols.contains(&owner_words.concat()) {
            terms.insert(owner_words);
        }
        let spec_dir = adapter.join("spec");
        match fs::symlink_metadata(&spec_dir) {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(format!(
                    "{}: expected a real specification directory",
                    spec_dir.display()
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        let specification = spec_dir.join("adapter.json");
        match fs::symlink_metadata(&specification) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(format!(
                        "{}: expected a regular adapter declaration",
                        specification.display()
                    )
                    .into());
                }
                let spec: serde_json::Value = serde_json::from_slice(&fs::read(&specification)?)?;
                let id = spec["id"]
                    .as_str()
                    .ok_or("adapter specification has no id")?;
                let id_words = words(id);
                if id_words.is_empty() {
                    return Err("adapter specification has an empty id".into());
                }
                if !protocols.contains(&id_words.concat()) {
                    terms.insert(id_words);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let model = spec_dir.join("ess");
        match fs::symlink_metadata(&model) {
            Ok(_) => {
                check_model(root, &model, &format!("connectors_{owner}"), None)?;
                models.push(model);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if models.len() == 1 {
        return Err("no adapter-owned ESS models found".into());
    }
    check_model(root, &models[0], "connectors", Some(&terms))?;
    Ok(models)
}

pub fn run(root: &Path, ess: &Path, temp: &Path) -> Result<()> {
    let models = check(root)?;
    println!("gate: shared ESS provider vocabulary and source ownership hold; exit=0");
    for (index, model) in models.iter().enumerate() {
        for action in ["validate", "compile"] {
            let mut cmd = Command::new(ess);
            cmd.current_dir(root)
                .env("TMPDIR", temp)
                .args(["specify", action, "--path"])
                .arg(model);
            if action == "compile" {
                cmd.arg("--out")
                    .arg(temp.join(format!("ess-boundary-{index}.json")));
            }
            let status = cmd.status()?;
            if !status.success() {
                return Err(format!("{}: ESS {action} exited {status}", model.display()).into());
            }
        }
    }
    println!(
        "gate: shared and {} adapter ESS models validated and compiled independently; exit=0",
        models.len() - 1
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for dir in [
            "ess/domains",
            "adapters/futurevendor/spec",
            "adapters/example/spec/ess/domains",
            "crates/connectors-build",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(
            root.join("crates/connectors-build/ess-boundary.json"),
            include_str!("../ess-boundary.json"),
        )
        .unwrap();
        fs::write(
            root.join("adapters/futurevendor/spec/adapter.json"),
            r#"{"id":"futurevendor"}"#,
        )
        .unwrap();
        fs::write(
            root.join("ess/system.yaml"),
            "format: ess/1\nsystem: connectors\nversion: v1\ndomains: [connectors.reads]\n",
        )
        .unwrap();
        fs::write(root.join("ess/domains/reads.yaml"), "domain: connectors.reads\n# HTTP, SIP, OAuth2, cargo, namespace and generic DocumentBodyFacts are shared.\n").unwrap();
        fs::write(root.join("adapters/example/spec/ess/system.yaml"), "format: ess/1\nsystem: connectors_example\nversion: v1\ndomains: [connectors_example.reads]\n").unwrap();
        fs::write(root.join("adapters/example/spec/ess/domains/reads.yaml"), "domain: connectors_example.reads\n# Confluence and Kubernetes detail is permitted in adapter-owned sources.\n").unwrap();
        temp
    }

    #[test]
    fn allows_shared_protocols_and_separate_provider_models() {
        assert_eq!(check(fixture().path()).unwrap().len(), 2);
    }

    #[test]
    fn discovers_design_only_and_spec_only_owners_without_runtime_declarations() {
        let temp = fixture();
        fs::write(temp.path().join("adapters/README.md"), "adapter index").unwrap();
        fs::create_dir(temp.path().join("adapters/newprovider")).unwrap();
        assert_eq!(check(temp.path()).unwrap().len(), 2);
        for name in ["NewProviderTarget", "ExampleTarget"] {
            fs::write(
                temp.path().join("ess/domains/reads.yaml"),
                format!("domain: connectors.reads\n# {name}\n"),
            )
            .unwrap();
            assert!(check(temp.path()).is_err(), "missed owner {name}");
        }
    }

    #[test]
    fn protocol_adapter_does_not_ban_shared_protocol_or_suppress_native_leaks() {
        let temp = fixture();
        fs::create_dir_all(temp.path().join("adapters/sip/spec")).unwrap();
        fs::write(
            temp.path().join("adapters/sip/spec/adapter.json"),
            "{\"id\":\"sip\"}",
        )
        .unwrap();
        assert!(check(temp.path()).is_ok());
        fs::write(
            temp.path().join("ess/domains/reads.yaml"),
            "domain: connectors.reads\n# SIP GitLabTarget\n",
        )
        .unwrap();
        assert!(check(temp.path()).is_err());
        let policy_path = temp
            .path()
            .join("crates/connectors-build/ess-boundary.json");
        let mut policy: serde_json::Value =
            serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
        policy["shared_protocol_names"] = serde_json::json!(["gitlab"]);
        fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        assert!(check(temp.path()).is_err());
    }

    #[test]
    fn rejects_malformed_optional_alias_inputs_and_model_sources() {
        for (path, content) in [
            ("adapters/futurevendor/spec/adapter.json", "{"),
            ("adapters/futurevendor/spec/adapter.json", "{}"),
            (
                "adapters/futurevendor/spec/adapter.json",
                "{\"id\":\"---\"}",
            ),
            (
                "adapters/example/spec/ess/domains/reads.yaml",
                "domain: connectors_foreign.reads\n",
            ),
        ] {
            let temp = fixture();
            fs::write(temp.path().join(path), content).unwrap();
            assert!(check(temp.path()).is_err(), "accepted {path}: {content}");
        }
    }

    #[test]
    fn rejects_names_comments_fields_encoded_values_and_new_adapter_ids() {
        for content in [
            "types: [{name: connectors.reads.ConfluenceStorage, kind: enum, variants: [text]}]",
            "# provider JIRA body behavior",
            "# legacy k8s tuple",
            "types: [{name: connectors.reads.Target, kind: struct, fields: [{name: api_group, type: String}]}]",
            "summary: \"conflu\\u0065nce\"",
            "summary: |\n  futurevendor detail",
            "summary: \"contracts\\u002fadapters/example/ess\"",
            "# ../../adapters/futurevendor/generated/ess",
            "# PreparedDeploymentRestart",
            "types: [{name: connectors.reads.GitLabTarget, kind: enum, variants: [text]}]",
            "# GitHub OpenAI LogQL PromQL BitBucket ArgoCD",
            "summary: \"Git\\u004cab\"",
        ] {
            let temp = fixture();
            fs::write(
                temp.path().join("ess/domains/reads.yaml"),
                format!("domain: connectors.reads\n{content}\n"),
            )
            .unwrap();
            assert!(check(temp.path()).is_err(), "accepted {content}");
        }
    }

    #[test]
    fn matches_case_and_separator_variants_without_substring_false_positives() {
        let terms = [
            "gitlab",
            "github",
            "openai",
            "logql",
            "promql",
            "argocd",
            "resource_version",
            "argo",
            "sql",
        ]
        .into_iter()
        .map(words)
        .collect();
        for name in [
            "GitLabTarget",
            "GitHubTarget",
            "OpenAITarget",
            "LogQLTarget",
            "PromQLTarget",
            "ArgoCDTarget",
            "resourceVersion",
            "resourceversion",
        ] {
            assert!(matching_term(name, &terms).is_some(), "missed {name}");
        }
        for name in [
            "cargo",
            "CargoConfiguration",
            "mysql",
            "namespace",
            "BodyFacts",
        ] {
            assert!(
                matching_term(name, &terms).is_none(),
                "false positive {name}"
            );
        }
    }

    #[test]
    fn checks_all_paths_and_requires_complete_local_domain_inventory() {
        for (path, text) in [
            ("ess/confluence.txt", "plain"),
            ("ess/nested/neutral.yaml", "domain: connectors.neutral"),
            ("ess/domains/extra.yaml", "domain: connectors.extra"),
            (
                "ess/system.yaml",
                "system: connectors\ndomains: [connectors.reads, alien.source]",
            ),
            ("ess/domains/reads.yaml", "domain: alien.source"),
        ] {
            let temp = fixture();
            let path = temp.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
            assert!(check(temp.path()).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_linked_source_files_and_directories() {
        for name in [
            "ess/domains/linked.yaml",
            "ess/linked",
            "adapters/linked",
            "adapters/futurevendor/spec/ess",
        ] {
            let temp = fixture();
            std::os::unix::fs::symlink(
                temp.path().join("adapters/example/spec/ess"),
                temp.path().join(name),
            )
            .unwrap();
            assert!(check(temp.path()).is_err());
        }
    }
}
