//! The local metadata authority's mutation emissions, made once and committed under
//! [`DIRECTORY`] with the digest of everything they are made from, and its synthesized baseline
//! suite, made once per digest into a cache under `target/`.
//!
//! `mutate::emit_for` over this model takes minutes. Tests never run it: they read the committed
//! manifests, and `the_committed_metadata_emissions_are_current` compares the recorded digest of
//! the inputs with the current one, which takes milliseconds.
//! `connectors-build metadata-conformance emissions` regenerates the manifests and checks
//! everything the tests used to check on each emission; with `--check` it writes nothing and fails
//! when a committed byte differs. The `slow` workflow runs `--check` when an input changes. The
//! baseline suite (12 MB) is not committed: [`baseline_suite`] synthesizes it once for a digest
//! and every later reader in any process reads the cached file.
//!
//! The inputs are the ESS crates in `Cargo.lock` (their release decides synthesis, emission and
//! the manifest reader), the `ess` pin (its `mutate --collect` reads the emission), every file
//! under `ess/`, and this file and `metadata_entities.rs`, which make the emissions. They hold
//! for any ESS pin: a new pin changes the digest, and the files are regenerated with it.
use super::Result;
use crate::metadata_entities;
use ess_conformance::{
    AdmittedSuite,
    mutate::{self, EmittedSuite, Manifest, MutantClass, RefusalKey},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The committed emissions, relative to the repository root.
pub const DIRECTORY: &str = "crates/connectors-build/generated/metadata-emissions";
/// The record of the inputs and their digest.
pub const INPUTS_FILE: &str = "inputs.json";
const FORMAT: &str = "connectors-metadata-emissions/1";
/// The component emission: precedence swaps of the whole component. Its manifest must be in a
/// format that holds them (`ess-mutation-manifest/4` and later).
pub const COMPONENT_MANIFEST: &str = "component/manifest.json";
/// The domains whose own emission (every mutant class, the component scoped to the domain) is
/// committed as `<domain>/manifest.json`.
pub const SCOPED_DOMAINS: &[&str] = &["connectors.oauth2", "connectors.approval_issuers"];
/// The class of [`COMPONENT_MANIFEST`].
pub const COMPONENT_CLASSES: &[MutantClass] = &[MutantClass::PrecedenceSwap];
/// Source files that make the emissions, besides `ess/`.
const SOURCES: &[&str] = &[
    "crates/connectors-spec/toolchain.json",
    "crates/connectors-build/src/metadata_emissions.rs",
    "crates/connectors-build/src/metadata_entities.rs",
];

/// `<domain>/manifest.json`.
pub fn scoped_manifest(domain: &str) -> String {
    format!("{domain}/manifest.json")
}

/// The ESS crates `Cargo.lock` selects, as `name version` lines.
fn ess_crates(root: &Path) -> Result<Vec<String>> {
    let lock: toml::Table = toml::from_str(&std::fs::read_to_string(root.join("Cargo.lock"))?)?;
    let mut crates: Vec<String> = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("Cargo.lock lists no packages")?
        .iter()
        .filter_map(|package| {
            let name = package.get("name")?.as_str()?;
            let version = package.get("version")?.as_str()?;
            (name.starts_with("ess-") || name.starts_with("entity-"))
                .then(|| format!("{name} {version}"))
        })
        .collect();
    crates.sort();
    if !crates
        .iter()
        .any(|line| line.starts_with("ess-conformance "))
    {
        return Err("Cargo.lock selects no ess-conformance".into());
    }
    Ok(crates)
}

fn files_under(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            files_under(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// The record of the current inputs: each input file's digest, the ESS crates, and one digest
/// over all of them.
pub fn inputs(root: &Path) -> Result<Value> {
    let mut paths = Vec::new();
    files_under(&root.join("ess"), &mut paths)?;
    paths.extend(SOURCES.iter().map(|source| root.join(source)));
    let mut files = Vec::new();
    for path in paths {
        let relative = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("input path is not UTF-8")?
            .replace('\\', "/");
        files.push((relative, connectors_spec::v2::hash(&std::fs::read(&path)?)));
    }
    files.sort();
    let crates = ess_crates(root)?;
    let mut summary = String::new();
    for line in &crates {
        summary.push_str(&format!("crate {line}\n"));
    }
    for (path, digest) in &files {
        summary.push_str(&format!("file {digest} {path}\n"));
    }
    Ok(json!({
        "format": FORMAT,
        "regenerate": "cargo run --locked -p connectors-build -- metadata-conformance emissions",
        "digest": connectors_spec::v2::hash(summary.as_bytes()),
        "ess_crates": crates,
        "files": files.iter().map(|(path, digest)| json!({"path": path, "sha256": digest})).collect::<Vec<_>>(),
    }))
}

/// The component's synthesized baseline suite (`ess-conformance` canonical JSON), the one the
/// gate runs. Synthesis takes ESS a minute or more, so it is made once per input digest into
/// `<target>/metadata-emissions/<digest>/suite.json` (`CARGO_TARGET_DIR`, else `target/`) and read
/// from there afterwards, by every test process of the run and by later runs. The file is written
/// beside its place and renamed into it, so a reader never sees half of it.
pub fn baseline_suite(root: &Path) -> Result<String> {
    let digest = inputs(root)?["digest"]
        .as_str()
        .ok_or("inputs record has no digest")?
        .to_owned();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let directory = target.join("metadata-emissions").join(&digest);
    let path = directory.join(mutate::SUITE_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        return Ok(text);
    }
    let loaded = metadata_entities::load(root)?;
    let ir = mutate::compile(loaded.documents.clone(), &loaded.sources)
        .map_err(|stillborn| format!("{stillborn:?}"))?;
    let synthesis = ess_conformance::synthesize::synthesize_for(&ir, metadata_entities::COMPONENT)
        .map_err(|error| error.to_string())?;
    let text = synthesis.suite.to_canonical_json()?;
    std::fs::create_dir_all(&directory)?;
    let mut partial = tempfile::NamedTempFile::new_in(&directory)?;
    std::io::Write::write_all(&mut partial, text.as_bytes())?;
    partial.persist(&path).map_err(|error| error.error)?;
    Ok(text)
}

/// The gate's suite: this component's synthesis of `ir`, written under `out/dir`.
pub fn emit_suite(ir: &ess_compiler::ir::EssIr, dir: &str, out: &Path) -> Result<EmittedSuite> {
    let synthesis = ess_conformance::synthesize::synthesize_for(ir, metadata_entities::COMPONENT)
        .map_err(|error| error.to_string())?;
    let json = synthesis.suite.to_canonical_json()?;
    let mut refused: Vec<RefusalKey> = synthesis.refusals.iter().map(RefusalKey::of).collect();
    refused.sort();
    AdmittedSuite::from_json(&json)?;
    let target = out.join(dir);
    std::fs::create_dir_all(&target)?;
    std::fs::write(target.join(mutate::SUITE_FILE), &json)?;
    std::fs::write(
        target.join(mutate::MODEL_FILE),
        format!("{}\n", ir.to_compact_json()),
    )?;
    Ok(EmittedSuite {
        dir: dir.to_owned(),
        refusals: refused.len(),
        refused: Some(refused),
        scenarios: synthesis.suite.len(),
        spec_digest: synthesis.suite.provenance.spec_digest.to_string(),
        // A manifest field; the gate writes no manifest.
        suite_digest: None,
    })
}

/// Writes ESS's own mutation emission of `classes`, scoped to the component the host implements,
/// under `out` (which must be absent or empty), and returns its manifest.
///
/// `mutate::emit_for` is what `ess verify conform mutate --emit --component` runs: it decides the
/// manifest format from the mutants it writes, records each suite's digest, which guard mutants
/// left their outcome unsatisfiable, which answer identically and which overlap, and marks the
/// mutants and unavailable sites of other components out of scope. Writing its files unchanged
/// keeps every claim in the manifest one the pinned ESS made itself.
pub fn emit_classes(root: &Path, out: &Path, classes: &[MutantClass]) -> Result<Manifest> {
    if out.exists() && std::fs::read_dir(out)?.next().is_some() {
        return Err(format!("{} is not empty", out.display()).into());
    }
    let loaded = metadata_entities::load(root)?;
    let emission = mutate::emit_for(
        &loaded.documents,
        &loaded.sources,
        classes,
        Some(metadata_entities::COMPONENT),
    )?;
    for (relative, contents) in &emission.files {
        let relative = Path::new(relative);
        if !relative
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
        {
            return Err(format!("emission file {} leaves the output", relative.display()).into());
        }
        let file = out.join(relative);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, contents)?;
    }
    // What `mutate --collect` reads back, read back the same way.
    Ok(Manifest::from_json(&std::fs::read_to_string(
        out.join(mutate::MANIFEST_FILE),
    )?)?)
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// A copy of `ess/` under `into` whose `local-metadata-authority` owns only `domain` and accepts
/// only its commands, so an emission visits that domain's mutants alone. The mutants are the ones
/// the real component's emission visits for `domain`: `mutate::mutants` reads the documents, not
/// the component.
pub fn scoped_root(root: &Path, domain: &str, into: &Path) -> Result<()> {
    copy_tree(&root.join("ess"), &into.join("ess"))?;
    let path = into.join("ess/components.yaml");
    let mut components: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&path)?)?;
    let component = components["components"]
        .as_sequence_mut()
        .ok_or("components.yaml lists no components")?
        .iter_mut()
        .find(|component| component["component"].as_str() == Some(metadata_entities::COMPONENT))
        .ok_or("components.yaml has no local-metadata-authority")?;
    component["owns"]["domains"] = serde_yaml_ng::to_value(vec![domain])?;
    let prefix = format!("{domain}.");
    let commands: Vec<serde_yaml_ng::Value> = component["accepts"]["commands"]
        .as_sequence()
        .ok_or("local-metadata-authority accepts no commands")?
        .iter()
        .filter(|command| {
            command
                .as_str()
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .cloned()
        .collect();
    if commands.is_empty() {
        return Err(format!("{domain} has no accepted command").into());
    }
    component["accepts"]["commands"] = serde_yaml_ng::Value::Sequence(commands);
    std::fs::write(&path, serde_yaml_ng::to_string(&components)?)?;
    Ok(())
}

fn ensure(condition: bool, message: impl FnOnce() -> String) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message().into())
    }
}

/// The checks on the component emission that need the whole emission and ESS itself: every
/// mutant suite has the digest the manifest records, the baseline's refusals are the ones a
/// separate synthesis of the same model reports, and the pinned `ess` reads the emission.
fn check_component_emission(
    root: &Path,
    ess: &Path,
    out: &Path,
    manifest: &Manifest,
    ir: &ess_compiler::ir::EssIr,
) -> Result<()> {
    for mutant in &manifest.mutants {
        if let Some(dir) = &mutant.dir {
            let text = std::fs::read_to_string(out.join(dir).join(mutate::SUITE_FILE))?;
            let suite = AdmittedSuite::from_json(&text)?;
            ensure(
                mutant.suite_digest.as_deref() == Some(suite.digest()),
                || format!("{} records another suite's digest", mutant.id),
            )?;
        }
    }
    let independent = ess_conformance::synthesize::synthesize_for(ir, metadata_entities::COMPONENT)
        .map_err(|error| error.to_string())?;
    let mut expected: Vec<RefusalKey> = independent.refusals.iter().map(RefusalKey::of).collect();
    expected.sort();
    ensure(
        manifest.baseline.refused.as_ref() == Some(&expected),
        || "the baseline's refusals are not the ones a separate synthesis reports".to_owned(),
    )?;
    // No refused scenario is in the suite the manifest points at.
    let suite: Value = serde_json::from_slice(&std::fs::read(
        out.join(&manifest.baseline.dir).join(mutate::SUITE_FILE),
    )?)?;
    let scenarios = suite["scenarios"]
        .as_object()
        .ok_or("the baseline suite has no scenarios")?;
    for key in &expected {
        if let Some(scenario) = &key.scenario {
            ensure(!scenarios.contains_key(scenario), || {
                format!("{scenario} is refused and emitted")
            })?;
        }
    }
    // No report is written, so nothing is scored (exit 3 at the most); a refused manifest is
    // exit 2 and names its file.
    let collect = std::process::Command::new(ess)
        .current_dir(root)
        .args(["verify", "conform", "mutate", "--collect"])
        .arg(out)
        .args(["--component", metadata_entities::COMPONENT])
        .output()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&collect.stdout),
        String::from_utf8_lossy(&collect.stderr)
    );
    ensure(
        !text.contains("manifest.json:") && collect.status.code() != Some(2),
        || {
            format!(
                "the pinned ess refuses the manifest ({:?}):\n{text}",
                collect.status
            )
        },
    )
}

/// The parts `--part` selects: `component` and each of [`SCOPED_DOMAINS`].
pub fn parts() -> Vec<String> {
    ["component"]
        .into_iter()
        .chain(SCOPED_DOMAINS.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// The committed files of the `selected` parts (every part when empty), relative to
/// [`DIRECTORY`], and their bytes, made from the sources now, with [`INPUTS_FILE`].
fn generate(
    root: &Path,
    ess: &Path,
    scratch: &Path,
    selected: &[String],
) -> Result<Vec<(String, Vec<u8>)>> {
    let known = parts();
    if let Some(unknown) = selected.iter().find(|part| !known.contains(part)) {
        return Err(format!("unknown part {unknown}; the parts are {}", known.join(", ")).into());
    }
    let wanted = |part: &str| selected.is_empty() || selected.iter().any(|name| name == part);
    let mut files = Vec::new();

    if wanted("component") {
        let loaded = metadata_entities::load(root)?;
        let ir = mutate::compile(loaded.documents.clone(), &loaded.sources)
            .map_err(|stillborn| format!("{stillborn:?}"))?;
        let component = scratch.join("component");
        let manifest = emit_classes(root, &component, COMPONENT_CLASSES)?;
        check_component_emission(root, ess, &component, &manifest, &ir)?;
        let text = std::fs::read(component.join(mutate::MANIFEST_FILE))?;
        check_component_manifest(&Manifest::from_json(std::str::from_utf8(&text)?)?)?;
        files.push((COMPONENT_MANIFEST.to_owned(), text));
        println!(
            "metadata emissions: component precedence swaps emitted and read by the pinned ess"
        );
    }

    for domain in SCOPED_DOMAINS.iter().filter(|domain| wanted(domain)) {
        let scoped = scratch.join(format!("root-{domain}"));
        scoped_root(root, domain, &scoped)?;
        let out = scratch.join(format!("emission-{domain}"));
        let manifest = emit_classes(&scoped, &out, MutantClass::ALL)?;
        println!(
            "metadata emissions: {domain}: {} with {} mutants",
            manifest.format,
            manifest.mutants.len()
        );
        files.push((
            scoped_manifest(domain),
            std::fs::read(out.join(mutate::MANIFEST_FILE))?,
        ));
    }

    let mut record = serde_json::to_vec_pretty(&inputs(root)?)?;
    record.push(b'\n');
    files.push((INPUTS_FILE.to_owned(), record));
    Ok(files)
}

/// What the component manifest must hold: a format that holds precedence swaps, this component,
/// a suite for every class, and the baseline's refusals.
pub fn check_component_manifest(manifest: &Manifest) -> Result<()> {
    ensure(
        [
            mutate::MANIFEST_FORMAT_4,
            mutate::MANIFEST_FORMAT_5,
            mutate::MANIFEST_FORMAT_6,
        ]
        .contains(&manifest.format.as_str()),
        || {
            format!(
                "{} is not a format that holds these classes",
                manifest.format
            )
        },
    )?;
    ensure(
        manifest.component.as_deref() == Some(metadata_entities::COMPONENT),
        || format!("the manifest names component {:?}", manifest.component),
    )?;
    for class in COMPONENT_CLASSES {
        ensure(
            manifest
                .mutants
                .iter()
                .any(|mutant| mutant.class == *class && mutant.dir.is_some()),
            || format!("no {class:?} mutant with a suite"),
        )?;
    }
    let refused = manifest
        .baseline
        .refused
        .as_ref()
        .ok_or("the baseline lists no refusals")?;
    ensure(!refused.is_empty(), || {
        "the model synthesizes with no refusals".to_owned()
    })?;
    ensure(manifest.baseline.refusals == refused.len(), || {
        "the baseline's refusal count is not its list's".to_owned()
    })
}

/// `metadata-conformance emissions`: regenerates the committed files of the `selected` parts
/// (all when empty) and the inputs record, or with `check` compares them byte for byte and writes
/// nothing. A part runs for minutes; selecting parts splits the work into separate runs.
pub fn run(root: &Path, ess: &Path, check: bool, selected: &[String]) -> Result<()> {
    connectors_spec::v2::check_ess(ess)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let scratch = tempfile::Builder::new()
        .prefix("metadata-emissions-")
        .tempdir_in(base)?;
    let files = generate(root, ess, scratch.path(), selected)?;
    let directory = root.join(DIRECTORY);
    if check {
        let mut drift = Vec::new();
        for (relative, bytes) in &files {
            if std::fs::read(directory.join(relative)).ok().as_ref() != Some(bytes) {
                drift.push(relative.clone());
            }
        }
        if !drift.is_empty() {
            return Err(format!(
                "metadata emissions differ from what the sources make: {}",
                drift.join(", ")
            )
            .into());
        }
        println!("metadata emissions: {} files current", files.len());
    } else {
        for (relative, bytes) in &files {
            connectors_spec::v2::write(&directory.join(relative), bytes)?;
        }
        println!("metadata emissions: wrote {} files", files.len());
    }
    Ok(())
}
