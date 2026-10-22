//! Repository build and acceptance executor.
use clap::{Parser, Subcommand};
use connectors_spec::v2::{check_ess, hash, json_bytes, tree, write};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

mod aep_toolchain;
#[cfg(test)]
mod catalog_feed_conformance;
mod cli;
#[cfg(test)]
mod confluence_feed_conformance;
mod docs;
mod ess_boundary;
#[cfg(test)]
mod feed_conformance;
mod gate;
#[cfg(test)]
mod gitlab_feed_conformance;
mod ignored;
#[cfg(test)]
mod jira_feed_conformance;
mod live_read_loop;
mod metadata_conformance;
mod metadata_entities;
mod platform_gated;
mod release_binaries;
mod source_hashes;
mod upstream_redaction;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Parser)]
#[command(about = "Repository build, generation and acceptance checks")]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    ess: Option<PathBuf>,
    #[arg(long)]
    aep: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    /// Account for every ignored workspace test and run explicitly selected families.
    Ignored(ignored::Options),
    /// Run a declared set of live reads through the `connectors` binary for a bounded time,
    /// with no revalidation, and report each call. On demand only; never in CI.
    LiveReadLoop(live_read_loop::Options),
    /// Generate or compare the selected local CLI contract fixture.
    Cli {
        #[arg(long)]
        check: bool,
    },
    /// Generate and build the bounded Rust/WASM contract examples.
    Examples,
    /// Check generated public files for local/private source paths.
    DocsAudit {
        /// Explicit output directory for isolated website preview builds.
        #[arg(long)]
        directory: Option<PathBuf>,
    },
    /// Check shared ESS ownership and compile shared and adapter semantic models.
    EssBoundary,
    /// Check that every binary a guide runs as an installed command is one the release
    /// workflow's `BINARIES` list packages, and that the list names workspace binaries.
    ReleaseBinaries,
    /// Re-derive every recorded upstream source digest from its archived bytes, and
    /// check every redacted upstream source against its record and its rule.
    SourceHashes,
    /// Write the `upstream-redaction/3` copy of a vendor document: example credentials,
    /// real-looking email addresses and phone numbers replaced. Never touches the network.
    Redact {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Generate or compare the local metadata Entity Runtime definitions.
    MetadataEntities {
        #[arg(long)]
        check: bool,
    },
    /// Run synthesized ESS suites against the local metadata authority's committed
    /// Entity Runtime definitions, or emit a component-scoped mutation audit for them.
    MetadataConformance {
        #[command(subcommand)]
        action: MetadataConformanceAction,
    },
    /// Build one provider bundle from its pinned OpenAPI source into a bundle
    /// directory, and index it there. Deterministic; never touches the network.
    Catalog {
        #[arg(long)]
        provider: String,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        auth_profile: String,
        /// Replace a bundle the index already carries for this provider.
        #[arg(long)]
        replace: bool,
        /// The pinned Google Discovery or Swagger 2.0 document `--source` was
        /// projected from (told apart by content).
        /// The projection is recomputed and must equal `--source` byte for byte;
        /// the bundle then records the derivation.
        #[arg(long)]
        derived_from: Option<PathBuf>,
        /// A `connectors-source-amendments/1` file of cited corrections to the
        /// pinned source (a query parameter it leaves out, or a path template it
        /// writes in a vendor notation), applied to its inventory and recorded in
        /// the bundle.
        #[arg(long)]
        amendments: Option<PathBuf>,
    },
    /// Lower one `connectors-definition/1` file into a provider bundle, its index
    /// entry, its `connectors-catalog-operations/1` selection and its
    /// `connectors-catalog-connector/1` declaration in `--out`; or, with
    /// `import`, write a definition from a shipped selection.
    /// Deterministic; never touches the network.
    #[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
    Definition {
        #[command(subcommand)]
        action: Option<DefinitionAction>,
        /// The definition, `connectors/<id>.yaml`; the file stem names the
        /// selection, and the catalog provider unless the file names another.
        #[arg(long, required = true)]
        file: Option<PathBuf>,
        /// The definitions checkout the definition's source paths are under.
        #[arg(long = "root", required = true)]
        definitions: Option<PathBuf>,
        /// The bundle directory to write the bundle and index into; the
        /// declaration and the selection it names go to `<catalog>/` under it.
        #[arg(long, required = true)]
        out: Option<PathBuf>,
    },
    /// Project a pinned Google Discovery document into OpenAPI 3.0.3, and write
    /// its projection record beside it as `<name>.projection.json`.
    /// Deterministic; never touches the network.
    Discovery {
        #[arg(long)]
        source: PathBuf,
        /// The OpenAPI document to write; its name ends in `.json`.
        #[arg(long)]
        out: PathBuf,
    },
    /// Project a pinned Swagger 2.0 document into OpenAPI 3.1.0 under
    /// `swagger2-openapi/2`, and write its projection record beside it as
    /// `<name>.projection.json`. Deterministic; never touches the network.
    Swagger {
        #[arg(long)]
        source: PathBuf,
        /// The OpenAPI document to write; its name ends in `.json`.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the local repository acceptance gates, without live provider credentials.
    Gate {
        /// Also check every target on the declared minimum Rust version.
        #[arg(long)]
        msrv: bool,
    },
}
#[derive(Subcommand)]
enum DefinitionAction {
    /// Write `connectors/<name>.yaml`, the shared auth files it uses and
    /// `upstream/<name>/` into a definitions checkout from one shipped selection,
    /// its bundle and the pinned source in this repository. Refused, writing
    /// nothing, when the definition would not lower back to the selection.
    Import {
        /// The catalog provider, the bundle's name.
        #[arg(long)]
        catalog: String,
        /// The selection file, repository-relative; default
        /// `adapters/catalog/providers/<catalog>/operations.json`.
        #[arg(long)]
        selection: Option<PathBuf>,
        /// The definitions checkout to write into.
        #[arg(long)]
        definitions: PathBuf,
        /// The definition's file stem; default `<catalog>`, or
        /// `<catalog>-<x>` for a selection named `<x>-operations.json`.
        #[arg(long)]
        name: Option<String>,
    },
}
#[derive(Subcommand)]
enum MetadataConformanceAction {
    /// Run one suite and write its `ess-conformance-report/2`.
    Run {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        report_out: PathBuf,
    },
    /// Write `ess verify conform mutate --emit` output scoped to the local metadata authority.
    Emit {
        #[arg(long)]
        out: PathBuf,
    },
    /// Run every suite of an emission, writing `report.json` beside each.
    RunEmission {
        #[arg(long)]
        dir: PathBuf,
    },
}
fn save_json(path: &Path, value: &Value) -> Result<()> {
    Ok(write(path, &json_bytes(value)?)?)
}
fn run(command: &mut Command) -> Result<Output> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "command {:?} failed: {}{}",
            command.get_program(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}
fn main() -> Result<()> {
    execute(Args::parse())
}
fn execute(args: Args) -> Result<()> {
    let root = args.root.canonicalize()?;
    if let Action::Ignored(options) = args.command {
        return ignored::run(&root, options);
    }
    if let Action::LiveReadLoop(options) = args.command {
        return live_read_loop::run(&root, options);
    }
    if let Action::DocsAudit { directory } = args.command {
        return docs::audit(&directory.unwrap_or_else(|| root.join("website/build")));
    }
    if let Action::Redact { input, output } = &args.command {
        return upstream_redaction::write(input, output);
    }
    if let Action::ReleaseBinaries = args.command {
        return release_binaries::run(&root);
    }
    if let Action::SourceHashes = args.command {
        source_hashes::run(&root)?;
        upstream_redaction::run(&root)?;
        return Ok(());
    }
    if let Action::MetadataConformance { action } = args.command {
        return match action {
            MetadataConformanceAction::Run { suite, report_out } => {
                metadata_conformance::run(&root, &suite, &report_out)
            }
            MetadataConformanceAction::Emit { out } => metadata_conformance::emit(&root, &out),
            MetadataConformanceAction::RunEmission { dir } => {
                metadata_conformance::run_emission(&root, &dir)
            }
        };
    }
    if let Action::MetadataEntities { check } = args.command {
        return metadata_entities::run(&root, check);
    }
    if let Action::Catalog {
        provider,
        source,
        directory,
        auth_profile,
        replace,
        derived_from,
        amendments,
    } = &args.command
    {
        let amendments = amendments
            .as_ref()
            .map(|path| inside(&root, path))
            .transpose()?;
        let request = connectors_catalog::pipeline::Request {
            provider,
            source: &inside(&root, source)?,
            directory: &inside(&root, directory)?,
            auth_profile,
            replace: *replace,
            amendments: amendments.as_deref(),
        };
        let run = match derived_from {
            Some(discovery) => {
                connectors_catalog::pipeline::run_derived(&request, &inside(&root, discovery)?)
            }
            None => connectors_catalog::pipeline::run(&request),
        }
        .map_err(|failure| failure.to_string())?;
        println!("{}", serde_json::to_string_pretty(&run.coverage)?);
        return Ok(());
    }
    if let Action::Definition {
        action:
            Some(DefinitionAction::Import {
                catalog,
                selection,
                definitions,
                name,
            }),
        ..
    } = &args.command
    {
        let selection = selection
            .as_ref()
            .map(|path| inside(&root, path))
            .transpose()?;
        let imported = connectors_catalog::import::import(&connectors_catalog::import::Request {
            connectors: &root,
            catalog,
            selection: selection.as_deref(),
            definitions,
            name: name.as_deref(),
        })?;
        for path in &imported.written {
            println!("{}", path.display());
        }
        return Ok(());
    }
    if let Action::Definition {
        action: None,
        file: Some(file),
        definitions: Some(definitions),
        out: Some(out),
    } = &args.command
    {
        let scratch = root.join(".local/tmp");
        std::fs::create_dir_all(&scratch)?;
        let lowered =
            connectors_catalog::definition::lower(&connectors_catalog::definition::Request {
                definition: file,
                root: definitions,
                out,
                scratch: &scratch,
            })?;
        println!("{}", serde_json::to_string_pretty(&lowered.entry)?);
        return Ok(());
    }
    if let Action::Discovery { source, out } = &args.command {
        return discovery(&root, source, out);
    }
    if let Action::Swagger { source, out } = &args.command {
        return swagger(&root, source, out);
    }
    let ess = connectors_spec::toolchain::resolve(args.ess.as_deref())?;
    if let Action::Cli { check } = args.command {
        return cli::run(&root, &ess, check);
    }
    if let Action::Examples = args.command {
        check_ess(&ess)?;
        return docs::examples(&root, &ess);
    }
    if let Action::EssBoundary = args.command {
        check_ess(&ess)?;
        let base = root.join(".local/tmp");
        std::fs::create_dir_all(&base)?;
        let temp = tempfile::Builder::new()
            .prefix("ess-boundary-")
            .tempdir_in(base)?;
        return ess_boundary::run(&root, &ess, temp.path());
    }
    if let Action::Gate { msrv } = args.command {
        let aep = aep_toolchain::resolve(args.aep.as_deref())?;
        return gate::run(&root, &ess, &aep, msrv);
    }
    Ok(())
}
/// Project the Discovery document at `source` and write the OpenAPI document to
/// `out` and its record beside it as `<name>.projection.json`. Nothing is
/// written unless the projection succeeds.
fn discovery(root: &Path, source: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(inside(root, source)?)?;
    let projection = connectors_catalog::discovery::project(&bytes)?;
    let (document, record) =
        write_projection(root, out, &projection.openapi, &projection.record_bytes())?;
    let record_value = &projection.record;
    println!(
        "discovery: {} methods, {} projected, {} excluded; wrote {} and {}",
        record_value.method_count,
        record_value.operations.len(),
        record_value.excluded_methods.len(),
        document.display(),
        record.display()
    );
    Ok(())
}
/// Project the Swagger 2.0 document at `source` and write the OpenAPI document
/// to `out` and its record beside it as `<name>.projection.json`. Nothing is
/// written unless the projection succeeds.
fn swagger(root: &Path, source: &Path, out: &Path) -> Result<()> {
    let bytes = std::fs::read(inside(root, source)?)?;
    let projection = connectors_catalog::swagger::project(&bytes)?;
    let (document, record) =
        write_projection(root, out, &projection.openapi, &projection.record_bytes())?;
    println!(
        "swagger: {} operations, {} projected, {} unprojected; wrote {} and {}",
        projection.record.operation_count,
        projection.record.operations.len(),
        projection.record.unprojected_operations.len(),
        document.display(),
        record.display()
    );
    Ok(())
}
/// Write a projected document to `out`, a `.json` file inside the repository,
/// and its record beside it as `<name>.projection.json`; returns both paths.
fn write_projection(
    root: &Path,
    out: &Path,
    openapi: &[u8],
    record_bytes: &[u8],
) -> Result<(PathBuf, PathBuf)> {
    let name = out
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".json"))
        .filter(|stem| !stem.is_empty())
        .ok_or("the projected document's name ends in `.json`")?;
    let parent = out
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let directory = inside(root, parent)?;
    let document = directory.join(format!("{name}.json"));
    let record = directory.join(format!("{name}.projection.json"));
    write(&document, openapi)?;
    write(&record, record_bytes)?;
    Ok((document, record))
}
fn inside(root: &Path, path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Err("declaration paths must be repository-relative".into());
    }
    let path = root.join(path).canonicalize()?;
    if !path.starts_with(root) {
        return Err("declaration path escapes the repository".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors_catalog::{bundle, discovery};
    use serde_json::json;

    /// The pinned Drive Discovery document, read in place.
    fn pinned_drive() -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../adapters/google/upstream/drive/drive-api.json"),
        )
        .expect("the pinned Drive document")
    }

    /// A throwaway repository root holding a copy of the pinned document at
    /// `upstream/drive-api.json` and an empty `bundles/` directory.
    fn workspace() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("upstream")).unwrap();
        std::fs::create_dir_all(temp.path().join("bundles")).unwrap();
        std::fs::create_dir_all(temp.path().join("projected")).unwrap();
        std::fs::write(temp.path().join("upstream/drive-api.json"), pinned_drive()).unwrap();
        temp
    }

    fn cli(root: &Path, arguments: &[&str]) -> Result<()> {
        let root = root.to_str().unwrap();
        let argv = ["connectors-build", "--root", root]
            .into_iter()
            .chain(arguments.iter().copied());
        execute(Args::try_parse_from(argv)?)
    }

    fn catalog(root: &Path, source: &str) -> Result<()> {
        cli(
            root,
            &[
                "catalog",
                "--provider",
                "google-drive",
                "--source",
                source,
                "--directory",
                "bundles",
                "--auth-profile",
                "google.oauth",
                "--derived-from",
                "upstream/drive-api.json",
            ],
        )
    }

    fn project_into(root: &Path) {
        cli(
            root,
            &[
                "discovery",
                "--source",
                "upstream/drive-api.json",
                "--out",
                "projected/drive-openapi.json",
            ],
        )
        .expect("discovery subcommand");
    }

    #[test]
    fn discovery_writes_the_projection_and_its_record() {
        let temp = workspace();
        project_into(temp.path());
        let expected = discovery::project(&pinned_drive()).expect("projection");
        let written = std::fs::read(temp.path().join("projected/drive-openapi.json")).unwrap();
        assert!(
            written == expected.openapi,
            "the written document is the projection"
        );
        let record =
            std::fs::read(temp.path().join("projected/drive-openapi.projection.json")).unwrap();
        assert!(
            record == expected.record_bytes(),
            "the written record is the projection's"
        );
        let entries = std::fs::read_dir(temp.path().join("projected"))
            .unwrap()
            .count();
        assert_eq!(
            entries, 2,
            "the subcommand writes the document and its record only"
        );
    }

    /// The pinned Slack Swagger 2.0 document is projected by the `swagger`
    /// subcommand into exactly the bytes and record the library computes, and
    /// the Discovery subcommand still refuses it rather than guessing.
    #[test]
    fn swagger_writes_the_projection_and_its_record() {
        let pinned = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../adapters/slack/upstream/slack_web_openapi_v2_without_examples.json"),
        )
        .expect("the pinned Slack document");
        let temp = workspace();
        std::fs::write(temp.path().join("upstream/slack.json"), &pinned).unwrap();
        cli(
            temp.path(),
            &[
                "swagger",
                "--source",
                "upstream/slack.json",
                "--out",
                "projected/slack.openapi.json",
            ],
        )
        .expect("swagger subcommand");
        let expected = connectors_catalog::swagger::project(&pinned).expect("projection");
        let written = std::fs::read(temp.path().join("projected/slack.openapi.json")).unwrap();
        assert!(
            written == expected.openapi,
            "the written document is the projection"
        );
        let record =
            std::fs::read(temp.path().join("projected/slack.openapi.projection.json")).unwrap();
        assert!(
            record == expected.record_bytes(),
            "the written record is the projection's"
        );
        let entries = std::fs::read_dir(temp.path().join("projected"))
            .unwrap()
            .count();
        assert_eq!(
            entries, 2,
            "the subcommand writes the document and its record only"
        );
        let refused = cli(
            temp.path(),
            &[
                "swagger",
                "--source",
                "upstream/drive-api.json",
                "--out",
                "projected/drive.openapi.json",
            ],
        );
        assert!(refused.is_err(), "a Discovery document is not Swagger 2.0");
        assert!(!temp.path().join("projected/drive.openapi.json").exists());
    }

    #[test]
    fn catalog_records_derivation() {
        let temp = workspace();
        project_into(temp.path());
        catalog(temp.path(), "projected/drive-openapi.json").expect("catalog run");
        let bundle = bundle::load(&temp.path().join("bundles"), "google-drive").unwrap();
        let pinned = pinned_drive();
        let derivation = bundle
            .source
            .derivation
            .expect("the bundle records its derivation");
        assert_eq!(derivation.from_sha256, connectors_spec::v2::hash(&pinned));
        assert_eq!(derivation.from_bytes, pinned.len());
        assert_eq!(derivation.from_file, "drive-api.json");
        assert_eq!(derivation.format, "google-discovery/v1");
        assert_eq!(derivation.projector, "discovery-openapi/1");
        let revision = serde_json::from_slice::<Value>(&pinned).unwrap()["revision"].clone();
        assert_eq!(json!(derivation.discovery_revision), revision);
        assert_eq!(bundle.source.file_name, "drive-openapi.json");
    }

    #[test]
    fn catalog_derived_from_mismatch_refused() {
        let temp = workspace();
        project_into(temp.path());
        let path = temp.path().join("projected/drive-openapi.json");
        let mut bytes = std::fs::read(&path).unwrap();
        // One byte: the final newline becomes a space, which is still JSON the
        // ingest would accept, so only the derivation check can refuse it.
        let last = bytes.len() - 1;
        assert_eq!(bytes[last], b'\n');
        bytes[last] = b' ';
        std::fs::write(&path, &bytes).unwrap();
        let refused = catalog(temp.path(), "projected/drive-openapi.json")
            .expect_err("a source that is not the projection is refused");
        assert!(
            refused.to_string().contains("projection"),
            "the refusal names the mismatch: {refused}"
        );
        let written: Vec<_> = std::fs::read_dir(temp.path().join("bundles"))
            .unwrap()
            .collect();
        assert!(written.is_empty(), "no bundle and no index is written");
    }

    #[test]
    fn catalog_applies_and_records_amendments() {
        let temp = workspace();
        project_into(temp.path());
        // The source digest, as an unamended run records it.
        std::fs::create_dir_all(temp.path().join("plain")).unwrap();
        cli(
            temp.path(),
            &[
                "catalog",
                "--provider",
                "google-drive",
                "--source",
                "projected/drive-openapi.json",
                "--directory",
                "plain",
                "--auth-profile",
                "google.oauth",
            ],
        )
        .expect("unamended catalog run");
        let digest = bundle::load(&temp.path().join("plain"), "google-drive")
            .unwrap()
            .source
            .source_sha256;
        let amendments = json!({
            "format": "connectors-source-amendments/1",
            "source_sha256": digest,
            "amendments": [{
                "operation_id": "drive.about.get",
                "add_parameter": {"name": "fixture_page_size", "location": "query",
                                  "required": false, "type": "integer"},
                "cite": "https://developers.google.com/workspace/drive/api/reference/rest/v3/about/get",
                "reason": "fixture: a documented parameter the pinned document leaves out",
            }],
        });
        std::fs::write(
            temp.path().join("projected/amendments.json"),
            serde_json::to_vec(&amendments).unwrap(),
        )
        .unwrap();
        cli(
            temp.path(),
            &[
                "catalog",
                "--provider",
                "google-drive",
                "--source",
                "projected/drive-openapi.json",
                "--directory",
                "bundles",
                "--auth-profile",
                "google.oauth",
                "--amendments",
                "projected/amendments.json",
            ],
        )
        .expect("catalog run");
        let bundle = bundle::load(&temp.path().join("bundles"), "google-drive").unwrap();
        let record = bundle.source.amendments.expect("amendment record");
        assert_eq!(record.file_name, "amendments.json");
        assert_eq!(record.count, 1);
        let about = bundle
            .inventory
            .operations
            .iter()
            .find(|operation| operation.operation_id.as_deref() == Some("drive.about.get"))
            .unwrap();
        assert!(
            about
                .parameters
                .iter()
                .any(|parameter| parameter.name == "fixture_page_size")
        );
        // Amendments written against another source are refused, and nothing is written.
        let other = temp.path().join("other");
        std::fs::create_dir_all(&other).unwrap();
        let mut stale = amendments.clone();
        stale["source_sha256"] = json!("00");
        std::fs::write(
            temp.path().join("projected/stale.json"),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();
        let refused = cli(
            temp.path(),
            &[
                "catalog",
                "--provider",
                "google-drive",
                "--source",
                "projected/drive-openapi.json",
                "--directory",
                "other",
                "--auth-profile",
                "google.oauth",
                "--amendments",
                "projected/stale.json",
            ],
        )
        .expect_err("amendments for another source are refused");
        assert!(refused.to_string().contains("amends a source"), "{refused}");
        assert_eq!(std::fs::read_dir(&other).unwrap().count(), 0);
    }

    #[test]
    fn catalog_without_derived_from_records_none() {
        let temp = workspace();
        project_into(temp.path());
        cli(
            temp.path(),
            &[
                "catalog",
                "--provider",
                "google-drive",
                "--source",
                "projected/drive-openapi.json",
                "--directory",
                "bundles",
                "--auth-profile",
                "google.oauth",
            ],
        )
        .expect("catalog run");
        let bundle = bundle::load(&temp.path().join("bundles"), "google-drive").unwrap();
        assert_eq!(bundle.source.derivation, None);
        let bytes = std::fs::read(temp.path().join("bundles/google-drive.bundle.json")).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains("derivation"));
    }

    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    /// `definition` against the Jira fixture, its vendored source resolved
    /// against `--root`, into `out` under a throwaway repository root.
    fn definition(temp: &Path, file: &Path) -> Result<()> {
        let repository = repository();
        let out = temp.join("out");
        cli(
            temp,
            &[
                "definition",
                "--file",
                file.to_str().unwrap(),
                "--root",
                repository.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ],
        )
    }

    #[test]
    fn definition_lowers_the_jira_definition_to_the_committed_bundle_and_a_selection() {
        let temp = tempfile::tempdir().unwrap();
        let file = repository()
            .join("crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml");
        definition(temp.path(), &file).expect("definition run");
        let written = std::fs::read(temp.path().join("out/jira.bundle.json")).unwrap();
        let committed =
            std::fs::read(repository().join("adapters/catalog/generated/bundles/jira.bundle.json"))
                .unwrap();
        assert!(
            written == committed,
            "the lowered bundle is the committed one"
        );
        bundle::load(&temp.path().join("out"), "jira").expect("the index verifies");
        let selection: Value = serde_json::from_slice(
            &std::fs::read(temp.path().join("out/jira/operations.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(selection["format"], "connectors-catalog-operations/1");
        assert_eq!(selection["operations"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn definition_import_writes_the_slack_user_definition_and_it_lowers_to_the_shipped_files() {
        let temp = tempfile::tempdir().unwrap();
        let definitions = temp.path().join("definitions");
        cli(
            &repository(),
            &[
                "definition",
                "import",
                "--catalog",
                "slack",
                "--selection",
                "adapters/catalog/providers/slack/user-operations.json",
                "--definitions",
                definitions.to_str().unwrap(),
            ],
        )
        .expect("import run");
        let file = definitions.join("connectors/slack-user.yaml");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\ncatalog: slack\n"), "{text}");
        let out = temp.path().join("out");
        cli(
            temp.path(),
            &[
                "definition",
                "--file",
                file.to_str().unwrap(),
                "--root",
                definitions.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ],
        )
        .expect("definition run");
        assert!(
            std::fs::read(out.join("slack.bundle.json")).unwrap()
                == std::fs::read(
                    repository().join("adapters/catalog/generated/bundles/slack.bundle.json")
                )
                .unwrap(),
            "the lowered bundle is the committed one"
        );
        let lowered: Value =
            serde_json::from_slice(&std::fs::read(out.join("slack/user-operations.json")).unwrap())
                .unwrap();
        let shipped: Value = serde_json::from_slice(
            &std::fs::read(
                repository().join("adapters/catalog/providers/slack/user-operations.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(lowered, shipped);
    }

    #[test]
    fn definition_import_refuses_a_selection_name_it_cannot_derive() {
        let temp = tempfile::tempdir().unwrap();
        let refused = cli(
            &repository(),
            &[
                "definition",
                "import",
                "--catalog",
                "slack",
                "--selection",
                "adapters/catalog/tests/feed/time.operations.json",
                "--definitions",
                temp.path().to_str().unwrap(),
            ],
        )
        .expect_err("a selection named neither operations.json nor <x>-operations.json");
        assert!(refused.to_string().starts_with("malformed: "), "{refused}");
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    #[test]
    fn definition_refusal_is_reported_by_name_and_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let text = std::fs::read_to_string(
            repository()
                .join("crates/connectors-catalog/tests/fixtures/definition/providers/jira.yaml"),
        )
        .unwrap()
        .replace("{op: getIssue}", "{op: getIssueThatIsNotThere}");
        let file = temp.path().join("jira.yaml");
        std::fs::write(&file, text).unwrap();
        let refused = definition(temp.path(), &file).expect_err("an unknown op is refused");
        assert!(
            refused.to_string().starts_with("unknown_operation: "),
            "{refused}"
        );
        assert!(!temp.path().join("out").exists());
    }
}
