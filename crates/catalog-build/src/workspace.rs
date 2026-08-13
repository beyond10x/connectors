//! Where a build reads its inputs and writes its artifacts.
//!
//! Every path a command touches is derived here from a single root, so a test can point a whole
//! build at a fixture tree and the production layout is the same code path.

use std::path::{Path, PathBuf};

/// Committed provider definitions: `providers/<name>.toml`.
pub const PROVIDERS_DIR: &str = "providers";

/// The vendored spec cache: `specs/<name>/<version>.json`.
///
/// Committed deliberately — it is what makes a build hermetic, offline and reviewable years later.
pub const SPECS_DIR: &str = "specs";

/// How a vendored spec spells itself in a provider file: `specs/<provider>/<file>`.
///
/// Derived from the provider name and the file name rather than by stripping a workspace root, so it
/// is the same string no matter where the repository is checked out — which matters because it is
/// compared for equality against `[spec] path`, a value an author typed by hand (C-4).
pub fn spec_path(provider: &str, file: &Path) -> String {
    format!(
        "{SPECS_DIR}/{provider}/{}",
        file.file_name().unwrap_or_default().to_string_lossy()
    )
}

/// The canonical per-provider catalog documents: `catalog/<name>.catalog.json` (C-536).
///
/// At the repository root beside `connectors/`, as the catalog-artifact design's diagram places
/// them: the reviewed artifact of Decision 0022, one deterministic JSON document per provider.
pub const DOCUMENTS_DIR: &str = "catalog";

/// The suffix a canonical document's file name carries: `zendesk.catalog.json`.
pub const DOCUMENT_SUFFIX: &str = "catalog.json";

/// The committed JSON Schema the documents validate against, beside them under [`DOCUMENTS_DIR`].
pub const DOCUMENT_SCHEMA_FILE: &str = "connector-document.schema.json";

/// The dependency-free reader crate, which embeds the compiled catalog pack (C-537).
pub const READER_DIR: &str = "crates/catalog-reader";

/// The pack's file name inside the reader crate: `catalog.pack`.
pub const PACK_FILE: &str = "catalog.pack";

/// The public site's data directory (C-42), holding the generated `catalog.json`.
///
/// Outside `connectors/` deliberately: that directory holds what a user *installs* into
/// `~/.flux/flows`, and a JSON document a website fetches is not that.
///
/// It is VitePress's `public/` directory (C-44), which is served verbatim at the site root — so the
/// explorer fetches `/flux-connectors/catalog.json` with no copy step and no build plumbing between
/// the Rust pipeline and the Node one. A sibling directory at the repository root was the original
/// choice; it meant two top-level directories for one website, and a copy step that could ship a
/// stale document. This pipeline still owns the file; the site merely reads it.
pub const SITE_DIR: &str = "web/public";

/// The site's generated catalogue: `web/public/catalog.json`.
pub const SITE_CATALOG: &str = "catalog.json";

/// A repository root plus the layout convention applied to it.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Treat `root` as a flux-connectors repository.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The repository root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/providers`.
    pub fn providers_dir(&self) -> PathBuf {
        self.root.join(PROVIDERS_DIR)
    }

    /// `<root>/specs/<provider>`.
    pub fn spec_dir(&self, provider: &str) -> PathBuf {
        self.root.join(SPECS_DIR).join(provider)
    }

    /// `<root>/catalog`.
    ///
    /// The **artifact root** of the canonical-document family (C-536, C-429): every `.json` in it
    /// is a per-provider document or their shared schema, all written by a build, so one no plan
    /// claims is a document whose provider stopped existing.
    pub fn documents_dir(&self) -> PathBuf {
        self.root.join(DOCUMENTS_DIR)
    }

    /// `<root>/catalog/<provider>.catalog.json` — the canonical per-provider document (C-536).
    pub fn document_path(&self, provider: &str) -> PathBuf {
        self.documents_dir()
            .join(format!("{provider}.{DOCUMENT_SUFFIX}"))
    }

    /// `<root>/catalog/connector-document.schema.json` — the schema the documents validate against.
    pub fn document_schema_path(&self) -> PathBuf {
        self.documents_dir().join(DOCUMENT_SCHEMA_FILE)
    }

    /// `<root>/crates/catalog-reader/catalog.pack` — the compiled catalog pack (C-537).
    ///
    /// Inside the reader crate for the same reason the renderings live inside `crates/catalog`:
    /// the crate embeds the file with `include_bytes!`, and a path that escapes the package root
    /// is one `cargo package` would not carry — a pack that resolved here and nowhere else.
    ///
    /// **Whole-catalogue**: one file holding every provider's canonical document, so only a full
    /// run can write it honestly. See [`crate::pipeline::plan_selected`].
    pub fn pack_path(&self) -> PathBuf {
        self.root.join(READER_DIR).join(PACK_FILE)
    }

    /// `<root>/web/public/catalog.json` — the whole catalogue as one JSON document (C-42).
    ///
    /// One file for every provider, not one per provider: a website wants one fetch, and the
    /// explorer's filters are queries across the whole catalogue. The cost is that it is not a
    /// function of a `--provider` run, which is why [`crate::pipeline::plan`] emits it only for a
    /// full build. See [`crate::site`].
    pub fn site_catalog_path(&self) -> PathBuf {
        self.root.join(SITE_DIR).join(SITE_CATALOG)
    }

    /// `<root>/connectors.lock` — the drift record for the whole catalogue (C-7, written by C-189).
    ///
    /// At the repository root rather than under `connectors/`, and the name is
    /// [`connector_spec::LOCKFILE_NAME`] rather than a literal here, so the writer and
    /// `flux-connectors check` (C-14) cannot disagree about which file they mean. The root is where
    /// a reader looks for a lockfile — it is a property of the repository, not of one directory of
    /// artifacts — and it is the same place `Cargo.lock` sits.
    ///
    /// **Whole-catalogue**: it holds one row per provider, so only a full run can write it. See
    /// [`crate::pipeline::plan_selected`].
    pub fn lockfile_path(&self) -> PathBuf {
        self.root.join(connector_spec::LOCKFILE_NAME)
    }

    /// `path` as `connectors.lock` keys it: repository-relative, `/`-separated on every platform.
    ///
    /// [`Self::display_path`] is the human-facing form and renders with the host's separator, which
    /// is fine for a message and wrong for a hashed artifact — a lockfile built on Windows would
    /// not be byte-identical to one built here, and every key would read as drift.
    pub fn artifact_key(&self, path: &Path) -> String {
        self.display_path(path)
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// `path` relative to the root when it is below it, for stable, machine-independent output.
    pub fn display_path<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }
}
