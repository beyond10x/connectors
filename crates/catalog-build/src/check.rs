//! Independent verification of `connectors.lock` against committed inputs and artifacts.
//!
//! A build computes the lockfile it would write. A check starts from the other direction: it reads
//! the committed lock, hashes the committed bytes named there, independently rebuilds the expected
//! rows, and refuses every disagreement by class. It performs no writes and no network IO.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use connector_spec::{LockEntry, LockSpec, Lockfile};

use crate::workspace::Workspace;
use crate::{discovery, pipeline};

const MAX_VERIFIED_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// The clean-tree counts printed by `catalog check`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Provider definitions covered in both the workspace and the lockfile.
    pub providers: usize,
    /// Generated outputs verified, including the structurally checked lockfile.
    pub artifacts: usize,
}

/// Verify every committed lock input and every generated artifact, without writing or networking.
pub fn verify(workspace: &Workspace) -> Result<Report> {
    let committed_text =
        read_utf8_file(&workspace.lockfile_path())?.context("connectors.lock is missing")?;
    let committed = Lockfile::parse(&committed_text).context("cannot parse connectors.lock")?;

    preflight_provider_definitions(workspace)?;
    let providers = discovery::discover(workspace, None)?;
    let provider_ids: BTreeSet<&str> = providers
        .iter()
        .map(|provider| provider.name.as_str())
        .collect();
    let (recorded, duplicate_rows) = rows(&committed);
    let recorded_ids: BTreeSet<&str> = recorded.keys().copied().collect();

    let mut problems = Vec::new();
    for duplicate in duplicate_rows {
        problems.push(format!(
            "lock coverage drift: duplicate row for provider `{duplicate}`"
        ));
    }
    for provider in provider_ids.difference(&recorded_ids) {
        problems.push(format!(
            "lock coverage drift: provider `{provider}` has no lock row"
        ));
    }
    for provider in recorded_ids.difference(&provider_ids) {
        problems.push(format!(
            "lock coverage drift: lock row `{provider}` has no provider"
        ));
    }
    refuse(&problems)?;

    // Hash the committed inputs before compiling them. If a pinned spec moved, the compiler may
    // correctly refuse its declared digest before it can produce a fresh lock row; the verifier
    // still owes the caller the more useful diagnosis that names the moved file.
    let mut input_problems = Vec::new();
    for provider in &providers {
        let entry = recorded[provider.name.as_str()];
        let provider_key = workspace.artifact_key(&provider.definition);
        let provider_path = locked_path(workspace, &provider_key)?;
        compare_input(
            &mut input_problems,
            "provider declaration drift",
            &provider_key,
            entry.toml_sha256.as_deref(),
            hash_file(&provider_path)?.as_deref(),
        );
        for spec in &entry.specs {
            let path = locked_path(workspace, &spec.path)?;
            compare_input(
                &mut input_problems,
                "vendored spec drift",
                &spec.path,
                spec.sha256.as_deref(),
                hash_file(&path)?.as_deref(),
            );
        }
    }

    // `pipeline::plan` compares generated output with committed bytes. Prove those paths are
    // bounded regular files before the planner reads them, so a generated-artifact symlink cannot
    // turn this verifier into an arbitrary or unbounded reader.
    let mut generated = vec![
        workspace.document_schema_path(),
        workspace.pack_path(),
        workspace.lockfile_path(),
        workspace.site_catalog_path(),
    ];
    generated.extend(
        providers
            .iter()
            .map(|provider| workspace.document_path(&provider.name)),
    );
    for path in generated {
        let key = workspace.artifact_key(&path);
        let path = locked_path(workspace, &key)?;
        let _ = read_bounded_file(&path)?;
    }

    let plan = match pipeline::plan(workspace, None) {
        Ok(plan) => plan,
        Err(error) if !input_problems.is_empty() => {
            refuse(&input_problems)?;
            return Err(error);
        }
        Err(error) => return Err(error).context("cannot rebuild the catalogue for verification"),
    };

    let expected_text = plan
        .artifacts
        .iter()
        .find(|planned| planned.path == workspace.lockfile_path())
        .map(|planned| planned.contents.as_str())
        .context("a full catalogue plan produced no connectors.lock")?;
    let expected = Lockfile::parse(expected_text).context("cannot parse the planned lockfile")?;
    let (expected_rows, expected_duplicates) = rows(&expected);
    debug_assert!(
        expected_duplicates.is_empty(),
        "the writer emitted duplicate rows"
    );

    problems.extend(input_problems);
    if committed_text != expected_text {
        problems.push(
            "generated artifact drift: `connectors.lock` does not match the current inputs".into(),
        );
    }
    compare_pack(workspace, &mut problems, committed.pack(), expected.pack())?;

    for provider in &providers {
        let id = provider.name.as_str();
        let expected_entry = expected_rows.get(id).copied().with_context(|| {
            format!("planned lockfile has no row for discovered provider `{id}`")
        })?;
        compare_entry(workspace, &mut problems, id, recorded[id], expected_entry)?;
    }

    let mut lock_artifacts = BTreeSet::new();
    if let Some(pack) = committed.pack() {
        lock_artifacts.insert(pack.path.as_str());
    }
    for entry in committed.entries() {
        lock_artifacts.extend(entry.artifacts.keys().map(String::as_str));
    }

    // The lockfile verifies itself structurally rather than by a self-referential hash: its parsed
    // rows have now been compared against both disk bytes and the freshly planned rows. Count it
    // alongside the other generated artifacts the command verified.
    let artifacts = plan.artifacts.len();
    for planned in &plan.artifacts {
        if planned.path == workspace.lockfile_path() {
            continue;
        }
        let key = workspace.artifact_key(&planned.path);
        if lock_artifacts.contains(key.as_str()) {
            continue;
        }
        match &planned.current {
            Some(current) if *current == planned.contents => {}
            Some(_) => problems.push(format!(
                "generated artifact drift: `{key}` does not match the current inputs"
            )),
            None => problems.push(format!("generated artifact drift: `{key}` is missing")),
        }
    }
    for orphan in &plan.orphans {
        problems.push(format!(
            "generated artifact coverage drift: `{}` is not produced by any provider",
            workspace.display_path(&orphan.path).display()
        ));
    }

    refuse(&problems)?;
    Ok(Report {
        providers: providers.len(),
        artifacts,
    })
}

fn rows(lockfile: &Lockfile) -> (BTreeMap<&str, &LockEntry>, Vec<&str>) {
    let mut rows = BTreeMap::new();
    let mut duplicates = Vec::new();
    for entry in lockfile.entries() {
        if rows.insert(entry.id.as_str(), entry).is_some() {
            duplicates.push(entry.id.as_str());
        }
    }
    (rows, duplicates)
}

fn compare_pack(
    workspace: &Workspace,
    problems: &mut Vec<String>,
    recorded: Option<&connector_spec::LockPack>,
    expected: Option<&connector_spec::LockPack>,
) -> Result<()> {
    match (recorded, expected) {
        (None, Some(_)) => {
            problems.push("lock coverage drift: the catalog pack has no lock row".into())
        }
        (Some(recorded), None) => problems.push(format!(
            "lock coverage drift: pack row `{}` has no generated artifact",
            recorded.path
        )),
        (None, None) => {}
        (Some(recorded), Some(expected)) => {
            if recorded.path != expected.path {
                problems.push(format!(
                    "lock row drift: pack path is `{}`, expected `{}`",
                    recorded.path, expected.path
                ));
                return Ok(());
            }
            if recorded.schema_version != expected.schema_version {
                problems.push(format!(
                    "lock row drift: `{}` records schema version {}, expected {}",
                    recorded.path, recorded.schema_version, expected.schema_version
                ));
            }
            let path = locked_path(workspace, &recorded.path)?;
            let actual = hash_file(&path)?;
            compare_artifact(
                problems,
                &recorded.path,
                &recorded.sha256,
                &expected.sha256,
                actual.as_deref(),
            );
        }
    }
    Ok(())
}

fn compare_entry(
    workspace: &Workspace,
    problems: &mut Vec<String>,
    id: &str,
    recorded: &LockEntry,
    expected: &LockEntry,
) -> Result<()> {
    compare_value(
        problems,
        id,
        "generator",
        &recorded.generator,
        &expected.generator,
    );
    compare_value(
        problems,
        id,
        "source_url",
        &recorded.source_url,
        &expected.source_url,
    );
    compare_value(
        problems,
        id,
        "upstream_version",
        &recorded.upstream_version,
        &expected.upstream_version,
    );
    compare_value(
        problems,
        id,
        "spec_sha256",
        &recorded.spec_sha256,
        &expected.spec_sha256,
    );
    compare_value(
        problems,
        id,
        "ir_sha256",
        &recorded.ir_sha256,
        &expected.ir_sha256,
    );

    // No current row populates this predecessor-reserved field, and no local path identifies the
    // unscrubbed bytes it would hash. Refuse rather than claiming to have verified a hash whose
    // bytes are unavailable; S-016 owns wiring source provenance into a checkable local record.
    if recorded.upstream_spec_sha256.is_some() || expected.upstream_spec_sha256.is_some() {
        problems.push(format!(
            "unverifiable lock hash: provider `{id}` carries `upstream_spec_sha256` without a local input path"
        ));
    }

    let (recorded_specs, recorded_duplicates) = specs(&recorded.specs);
    let (expected_specs, expected_duplicates) = specs(&expected.specs);
    debug_assert!(
        expected_duplicates.is_empty(),
        "the writer emitted duplicate spec rows"
    );
    for duplicate in recorded_duplicates {
        problems.push(format!(
            "lock coverage drift: provider `{id}` records spec `{duplicate}` twice"
        ));
    }
    let recorded_paths: BTreeSet<&str> = recorded_specs.keys().copied().collect();
    let expected_paths: BTreeSet<&str> = expected_specs.keys().copied().collect();
    for path in expected_paths.difference(&recorded_paths) {
        problems.push(format!(
            "lock coverage drift: provider `{id}` uses spec `{path}` but has no lock row for it"
        ));
    }
    for path in recorded_paths.difference(&expected_paths) {
        problems.push(format!(
            "lock coverage drift: provider `{id}` records unused spec `{path}`"
        ));
    }
    for path in recorded_paths.intersection(&expected_paths) {
        let recorded = recorded_specs[path];
        let expected = expected_specs[path];
        compare_value(
            problems,
            id,
            &format!("spec `{path}` service"),
            &recorded.service,
            &expected.service,
        );
        compare_value(
            problems,
            id,
            &format!("spec `{path}` source_url"),
            &recorded.source_url,
            &expected.source_url,
        );
        compare_value(
            problems,
            id,
            &format!("spec `{path}` upstream_version"),
            &recorded.upstream_version,
            &expected.upstream_version,
        );
    }

    let recorded_paths: BTreeSet<&str> = recorded.artifacts.keys().map(String::as_str).collect();
    let expected_paths: BTreeSet<&str> = expected.artifacts.keys().map(String::as_str).collect();
    for path in expected_paths.difference(&recorded_paths) {
        problems.push(format!(
            "lock artifact coverage drift: provider `{id}` emits `{path}` but has no hash for it"
        ));
    }
    for path in recorded_paths.difference(&expected_paths) {
        problems.push(format!(
            "lock artifact coverage drift: provider `{id}` records unowned artifact `{path}`"
        ));
    }
    for path in recorded_paths.intersection(&expected_paths) {
        let disk = locked_path(workspace, path)?;
        let actual = hash_file(&disk)?;
        compare_artifact(
            problems,
            path,
            &recorded.artifacts[*path],
            &expected.artifacts[*path],
            actual.as_deref(),
        );
    }

    Ok(())
}

fn specs(specs: &[LockSpec]) -> (BTreeMap<&str, &LockSpec>, Vec<&str>) {
    let mut rows = BTreeMap::new();
    let mut duplicates = Vec::new();
    for spec in specs {
        if rows.insert(spec.path.as_str(), spec).is_some() {
            duplicates.push(spec.path.as_str());
        }
    }
    (rows, duplicates)
}

fn compare_input(
    problems: &mut Vec<String>,
    kind: &str,
    path: &str,
    recorded: Option<&str>,
    actual: Option<&str>,
) {
    match (recorded, actual) {
        (Some(recorded), Some(actual)) if recorded == actual => {}
        (Some(_), Some(_)) => problems.push(format!(
            "{kind}: `{path}` bytes moved since `catalog build`"
        )),
        (None, Some(_)) => problems.push(format!("{kind}: `{path}` has no recorded digest")),
        (Some(_), None) => problems.push(format!("{kind}: `{path}` is missing")),
        (None, None) => problems.push(format!(
            "{kind}: `{path}` is missing and has no recorded digest"
        )),
    }
}

fn compare_artifact(
    problems: &mut Vec<String>,
    path: &str,
    recorded: &str,
    expected: &str,
    actual: Option<&str>,
) {
    match actual {
        None => problems.push(format!("artifact drift: `{path}` is missing")),
        Some(actual) if actual == recorded && actual == expected => {}
        Some(actual) if recorded == expected && actual != recorded => problems.push(format!(
            "artifact drift: `{path}` bytes moved since `catalog build`"
        )),
        Some(actual) if actual == expected && recorded != actual => problems.push(format!(
            "lock row drift: `{path}` records a digest that does not match the generated artifact"
        )),
        Some(_) => problems.push(format!(
            "artifact and lock drift: `{path}` matches neither the recorded digest nor current inputs"
        )),
    }
}

fn compare_value<T: std::fmt::Debug + PartialEq>(
    problems: &mut Vec<String>,
    provider: &str,
    field: &str,
    recorded: &T,
    expected: &T,
) {
    if recorded != expected {
        problems.push(format!(
            "lock row drift: provider `{provider}` field `{field}` is {recorded:?}, expected {expected:?}"
        ));
    }
}

fn read_bounded_file(path: &Path) -> Result<Option<Vec<u8>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect {}", path.display()))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "refusing non-regular verification input `{}`",
            path.display()
        );
    }
    if metadata.len() > MAX_VERIFIED_FILE_BYTES {
        bail!(
            "verification input `{}` exceeds the {} byte limit",
            path.display(),
            MAX_VERIFIED_FILE_BYTES
        );
    }
    let file = fs::File::open(path)
        .with_context(|| format!("cannot open verification input {}", path.display()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_VERIFIED_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("cannot read verification input {}", path.display()))?;
    if bytes.len() as u64 > MAX_VERIFIED_FILE_BYTES {
        bail!(
            "verification input `{}` grew beyond the {} byte limit",
            path.display(),
            MAX_VERIFIED_FILE_BYTES
        );
    }
    Ok(Some(bytes))
}

fn read_utf8_file(path: &Path) -> Result<Option<String>> {
    read_bounded_file(path)?
        .map(|bytes| {
            String::from_utf8(bytes)
                .with_context(|| format!("verification input {} is not UTF-8", path.display()))
        })
        .transpose()
}

fn hash_file(path: &Path) -> Result<Option<String>> {
    Ok(read_bounded_file(path)?.map(|bytes| connector_spec::sha256_hex(&bytes)))
}

fn preflight_provider_definitions(workspace: &Workspace) -> Result<()> {
    let directory = workspace.providers_dir();
    let metadata = fs::symlink_metadata(&directory)
        .with_context(|| format!("cannot inspect {}", directory.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!(
            "refusing non-directory provider root `{}`",
            directory.display()
        );
    }
    for entry in
        fs::read_dir(&directory).with_context(|| format!("cannot read {}", directory.display()))?
    {
        let path = entry
            .with_context(|| format!("cannot read entry under {}", directory.display()))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            let _ = read_bounded_file(&path)?;
        }
    }
    Ok(())
}

fn locked_path(workspace: &Workspace, key: &str) -> Result<PathBuf> {
    let path = Path::new(key);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "unsafe path `{key}` in connectors.lock: paths must be repository-relative and contain no traversal"
        );
    }
    let mut current = workspace.root().to_path_buf();
    for component in path.components() {
        let Component::Normal(name) = component else {
            unreachable!("non-normal components were rejected above")
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => bail!(
                "unsafe path `{key}` in connectors.lock: symlink component `{}` is not verifiable",
                workspace.display_path(&current).display()
            ),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("cannot inspect lock path component {}", current.display())
                })
            }
        }
    }
    Ok(workspace.root().join(path))
}

fn refuse(problems: &[String]) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    bail!(
        "catalog check found {} problem{}:\n  {}",
        problems.len(),
        if problems.len() == 1 { "" } else { "s" },
        problems.join("\n  ")
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::artifact::tests::{scratch, Scratch};
    use crate::cli::{Command, Invocation};

    const HAND_AUTHORED: &str = r#"id = "acme"
vendor = "Acme Inc."
base_url = "https://api.acme.example"
description = "A hand-authored fixture connector."

[[operations]]
id = "acme-thing-get"
method = "GET"
direction = "read"
path = "/v1/things/{thing_id}"
description = "Fetch one thing."
risk = "low"
idempotency = "idempotent"

[[operations.params.path]]
name = "thing_id"
description = "The thing to fetch."
required = true
schema = { type = "integer" }
"#;

    const SPEC_BACKED: &str = r#"id = "acme"
vendor = "Acme Inc."
base_url = "https://api.acme.example"

[spec]
path = "specs/acme/v1.json"

[patch.directions.default]
showThing = "read"

[[patch.operations]]
select = "showThing"
rename = "acme-thing-get"
risk = "low"
idempotency = "idempotent"
"#;

    const SPEC: &str = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Acme", "version": "1.0.0" },
  "servers": [{ "url": "https://api.acme.example" }],
  "paths": {
    "/v1/things/{thing_id}": {
      "get": {
        "operationId": "showThing",
        "summary": "Fetch one thing.",
        "parameters": [{
          "name": "thing_id",
          "in": "path",
          "required": true,
          "schema": { "type": "integer" }
        }]
      }
    }
  }
}
"#;

    struct Fixture {
        root: Scratch,
    }

    impl Fixture {
        fn hand_authored(label: &str) -> Self {
            let fixture = Self {
                root: scratch(label),
            };
            fixture.write("providers/acme.toml", HAND_AUTHORED);
            fixture.build();
            fixture
        }

        fn spec_backed(label: &str) -> Self {
            let fixture = Self {
                root: scratch(label),
            };
            fixture.write("providers/acme.toml", SPEC_BACKED);
            fixture.write("specs/acme/v1.json", SPEC);
            fixture.build();
            fixture
        }

        fn workspace(&self) -> Workspace {
            Workspace::new(self.root.to_path_buf())
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture parent");
            }
            fs::write(path, contents).expect("write fixture file");
        }

        fn read(&self, relative: &str) -> String {
            fs::read_to_string(self.root.join(relative)).expect("read fixture file")
        }

        fn build(&self) {
            let invocation = Invocation {
                command: Command::Build,
                root: Some(self.root.to_path_buf()),
                ..Default::default()
            };
            crate::run(&invocation, &mut Vec::new()).expect("build fixture");
        }

        fn check(&self) -> Result<Report> {
            verify(&self.workspace())
        }
    }

    fn rendered(error: anyhow::Error) -> String {
        format!("{error:#}")
    }

    #[test]
    fn a_clean_tree_reports_the_provider_and_artifact_counts_without_writing() {
        let fixture = Fixture::hand_authored("check-clean");
        let before = snapshot(&fixture.root);

        assert_eq!(
            fixture.check().expect("a clean lock verifies"),
            Report {
                providers: 1,
                artifacts: 5,
            }
        );
        let invocation = Invocation {
            command: Command::Check,
            root: Some(fixture.root.to_path_buf()),
            ..Default::default()
        };
        let mut output = Vec::new();
        crate::run(&invocation, &mut output).expect("the CLI check succeeds");
        assert_eq!(
            String::from_utf8(output).expect("CLI output is UTF-8"),
            "1 provider, 5 artifacts verified\n"
        );
        assert_eq!(before, snapshot(&fixture.root), "check wrote to the tree");
    }

    #[test]
    fn a_mutated_artifact_is_named_as_artifact_drift() {
        let fixture = Fixture::hand_authored("check-artifact");
        fixture.write("catalog/acme.catalog.json", "hand edited\n");

        let error = rendered(fixture.check().expect_err("artifact drift must fail"));
        assert!(error.contains("artifact drift"), "{error}");
        assert!(error.contains("catalog/acme.catalog.json"), "{error}");
    }

    #[test]
    fn a_comment_only_provider_edit_is_named_as_declaration_drift() {
        let fixture = Fixture::hand_authored("check-provider");
        fixture.write(
            "providers/acme.toml",
            &format!("{HAND_AUTHORED}\n# review note changed\n"),
        );

        let error = rendered(fixture.check().expect_err("provider drift must fail"));
        assert!(error.contains("provider declaration drift"), "{error}");
        assert!(error.contains("providers/acme.toml"), "{error}");
    }

    #[test]
    fn a_revendored_spec_is_named_as_spec_drift() {
        let fixture = Fixture::spec_backed("check-spec");
        fixture.write("specs/acme/v1.json", &format!("{SPEC}\n"));

        let error = rendered(fixture.check().expect_err("spec drift must fail"));
        assert!(error.contains("vendored spec drift"), "{error}");
        assert!(error.contains("specs/acme/v1.json"), "{error}");
    }

    #[test]
    fn a_wrong_lock_artifact_hash_is_named_as_lock_row_drift() {
        let fixture = Fixture::hand_authored("check-lock-row");
        let lock = fixture.read("connectors.lock");
        let artifact_hash =
            connector_spec::sha256_hex(fixture.read("catalog/acme.catalog.json").as_bytes());
        fixture.write(
            "connectors.lock",
            &lock.replacen(&artifact_hash, &"0".repeat(64), 1),
        );

        let error = rendered(fixture.check().expect_err("a lying row must fail"));
        assert!(error.contains("lock row drift"), "{error}");
        assert!(error.contains("catalog/acme.catalog.json"), "{error}");
    }

    #[test]
    fn provider_coverage_is_checked_in_both_directions() {
        let missing_row = Fixture::hand_authored("check-provider-missing-row");
        let lock = missing_row.read("connectors.lock");
        let provider_at = lock.find("[[provider]]").expect("provider row");
        missing_row.write("connectors.lock", &lock[..provider_at]);
        let error = rendered(missing_row.check().expect_err("missing row must fail"));
        assert!(error.contains("provider `acme` has no lock row"), "{error}");

        let extra_row = Fixture::hand_authored("check-provider-extra-row");
        fs::rename(
            extra_row.root.join("providers/acme.toml"),
            extra_row.root.join("providers/other.toml"),
        )
        .expect("rename provider definition");
        let error = rendered(extra_row.check().expect_err("extra row must fail"));
        assert!(error.contains("lock row `acme` has no provider"), "{error}");
        assert!(
            error.contains("provider `other` has no lock row"),
            "{error}"
        );
    }

    #[test]
    fn artifact_coverage_is_checked_in_both_directions() {
        let missing = Fixture::hand_authored("check-artifact-missing-row");
        let lock = missing.read("connectors.lock");
        let artifact = "\"catalog/acme.catalog.json\"";
        let line_start = lock.find(artifact).expect("artifact row");
        let line_end = lock[line_start..]
            .find('\n')
            .map_or(lock.len(), |offset| line_start + offset + 1);
        missing.write(
            "connectors.lock",
            &format!("{}{}", &lock[..line_start], &lock[line_end..]),
        );
        let error = rendered(missing.check().expect_err("missing artifact row must fail"));
        assert!(error.contains("lock artifact coverage drift"), "{error}");
        assert!(error.contains("catalog/acme.catalog.json"), "{error}");

        let extra = Fixture::hand_authored("check-artifact-extra-row");
        let lock = extra.read("connectors.lock");
        extra.write(
            "connectors.lock",
            &lock.replace(
                "[provider.artifacts]\n",
                &format!(
                    "[provider.artifacts]\n\"catalog/unowned.catalog.json\" = \"{}\"\n",
                    "0".repeat(64)
                ),
            ),
        );
        let error = rendered(extra.check().expect_err("extra artifact row must fail"));
        assert!(error.contains("lock artifact coverage drift"), "{error}");
        assert!(error.contains("catalog/unowned.catalog.json"), "{error}");
    }

    #[test]
    fn spec_coverage_is_checked_in_both_directions() {
        let missing = Fixture::spec_backed("check-spec-missing-row");
        let lock = missing.read("connectors.lock");
        let spec_start = lock.find("[[provider.specs]]").expect("spec row");
        let artifact_start = lock[spec_start..]
            .find("[provider.artifacts]")
            .map(|offset| spec_start + offset)
            .expect("artifact table follows spec row");
        missing.write(
            "connectors.lock",
            &format!("{}{}", &lock[..spec_start], &lock[artifact_start..]),
        );
        let error = rendered(missing.check().expect_err("missing spec row must fail"));
        assert!(error.contains("lock coverage drift"), "{error}");
        assert!(error.contains("specs/acme/v1.json"), "{error}");

        let extra = Fixture::hand_authored("check-spec-extra-row");
        let lock = extra.read("connectors.lock");
        extra.write(
            "connectors.lock",
            &lock.replace(
                "[provider.artifacts]",
                &format!(
                    "[[provider.specs]]\npath = \"specs/acme/unowned.json\"\nsha256 = \"{}\"\n\n[provider.artifacts]",
                    "0".repeat(64)
                ),
            ),
        );
        let error = rendered(extra.check().expect_err("extra spec row must fail"));
        assert!(error.contains("lock coverage drift"), "{error}");
        assert!(error.contains("specs/acme/unowned.json"), "{error}");
    }

    #[test]
    fn unsafe_and_symlinked_lock_paths_are_refused_without_reading_the_target() {
        let traversal = Fixture::hand_authored("check-path-traversal");
        let lock = traversal.read("connectors.lock");
        traversal.write(
            "connectors.lock",
            &lock.replace(
                "[provider.artifacts]",
                &format!(
                    "[[provider.specs]]\npath = \"../outside.json\"\nsha256 = \"{}\"\n\n[provider.artifacts]",
                    "0".repeat(64)
                ),
            ),
        );
        let error = rendered(traversal.check().expect_err("traversal must fail"));
        assert!(error.contains("unsafe path `../outside.json`"), "{error}");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let linked = Fixture::hand_authored("check-path-symlink");
            fs::create_dir_all(linked.root.join("specs")).expect("create spec parent");
            symlink("/dev/zero", linked.root.join("specs/unbounded"))
                .expect("create hostile fixture symlink");
            let lock = linked.read("connectors.lock");
            linked.write(
                "connectors.lock",
                &lock.replace(
                    "[provider.artifacts]",
                    &format!(
                        "[[provider.specs]]\npath = \"specs/unbounded\"\nsha256 = \"{}\"\n\n[provider.artifacts]",
                        "0".repeat(64)
                    ),
                ),
            );
            let error = rendered(linked.check().expect_err("symlink must fail"));
            assert!(error.contains("symlink component"), "{error}");

            let generated = Fixture::hand_authored("check-artifact-symlink");
            let artifact = generated.root.join("catalog/acme.catalog.json");
            fs::remove_file(&artifact).expect("remove generated fixture artifact");
            symlink("/dev/zero", &artifact).expect("create hostile generated-artifact symlink");
            let error = rendered(generated.check().expect_err("artifact symlink must fail"));
            assert!(error.contains("symlink component"), "{error}");
        }
    }

    fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        fn collect(
            root: &std::path::Path,
            dir: &std::path::Path,
            into: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        ) {
            for entry in fs::read_dir(dir).expect("read fixture directory") {
                let path = entry.expect("fixture entry").path();
                if path.is_dir() {
                    collect(root, &path, into);
                } else {
                    into.insert(
                        path.strip_prefix(root)
                            .expect("relative path")
                            .to_path_buf(),
                        fs::read(path).expect("read fixture path"),
                    );
                }
            }
        }

        let mut files = std::collections::BTreeMap::new();
        collect(root, root, &mut files);
        files
    }
}
