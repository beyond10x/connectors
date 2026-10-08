//! The contract and model reference: every page `website/publication.json` selects.
//!
//! The inventory is an allowlist. A contract page renders its owner's Markdown (shared
//! contracts under `contracts/`, native ones under `adapters/<owner>/contracts/`) through
//! [`crate::markdown`]; a model renders the pinned `ess generate --kind docs` output of the
//! owner's ESS root. A route `/x/y` becomes the page `website/docs/reference/x/y.md`, served at
//! `/docs/reference/x/y`. Links between selected sources become links between their pages;
//! a link to anything else keeps only its label.
//!
//! The directories this module writes are wholly generated: `reference/contracts/` and every
//! adapter's `reference/adapters/<owner>/contracts/` and `…/model/`. A file there that
//! generation does not produce is an orphan, which `generate` removes and `--check` refuses.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

use crate::{SOURCE, cell, front_matter, markdown, yaml_string};

/// The publication inventory, relative to the repository root.
const INVENTORY: &str = "website/publication.json";
/// Where reference pages land, relative to the repository root.
const REFERENCE: &str = "website/docs/reference";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    pages: Vec<Page>,
    models: Vec<Model>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    route: String,
    title: String,
    source: String,
    owner: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    source: String,
    route: String,
    owner: String,
}

/// The source of a selected contract, which must lie inside its owner's contract directory.
fn public_source(root: &Path, source: &str, owner: &str) -> crate::Result<PathBuf> {
    let prefix = if owner == "shared" {
        "contracts/".to_owned()
    } else {
        format!("adapters/{owner}/contracts/")
    };
    if !source.starts_with(&prefix)
        || source.contains('\\')
        || Path::new(source).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unpublished source: {source}"));
    }
    let path = root
        .join(source)
        .canonicalize()
        .map_err(|error| format!("{source}: {error}"))?;
    let allowed = root
        .join(&prefix)
        .canonicalize()
        .map_err(|error| format!("{prefix}: {error}"))?;
    if !path.starts_with(&allowed) || !allowed.starts_with(root) || !path.is_file() {
        return Err(format!("source escapes its owner: {source}"));
    }
    Ok(path)
}

/// Record a route, refusing one that is malformed or already taken.
fn route(value: &str, routes: &mut BTreeSet<String>) -> crate::Result<()> {
    if !value.starts_with('/')
        || value.contains("..")
        || value.contains("//")
        || value.ends_with('/')
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/-_".contains(c))
        || !routes.insert(value.to_owned())
    {
        return Err(format!("invalid or duplicate public route: {value}"));
    }
    Ok(())
}

/// The page file of a route, relative to the reference directory: `/a/b` is `a/b.md`.
fn page_file(route: &str) -> PathBuf {
    PathBuf::from(format!("{}.md", &route[1..]))
}

/// The relative link from the page `from` to the page `to`, both relative to one directory.
fn relative_link(from: &Path, to: &Path) -> String {
    let from_dir: Vec<_> = from
        .parent()
        .map(|p| p.components().collect())
        .unwrap_or_default();
    let to_parts: Vec<_> = to.components().collect();
    let common = from_dir
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> = vec!["..".to_owned(); from_dir.len() - common];
    parts.extend(
        to_parts[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    let link = parts.join("/");
    if link.starts_with("..") {
        link
    } else {
        format!("./{link}")
    }
}

/// Where a link written in `source` leads among the published pages, if anywhere.
fn resolve_contract_link(
    source: &Path,
    page: &Path,
    destination: &str,
    links: &BTreeMap<PathBuf, PathBuf>,
) -> Option<String> {
    if destination.starts_with('#') {
        return Some(destination.to_owned());
    }
    if destination.starts_with("https://") {
        return Some(destination.to_owned());
    }
    if destination.contains(':') || destination.starts_with("//") || destination.contains('\\') {
        return None;
    }
    let (file, anchor) = destination
        .split_once('#')
        .map_or((destination, None), |(a, b)| (a, Some(b)));
    let target = source.parent()?.join(file).canonicalize().ok()?;
    let target_page = links.get(&target)?;
    let link = relative_link(page, target_page);
    Some(match anchor {
        Some(anchor) => format!("{link}#{anchor}"),
        None => link,
    })
}

/// The family a shared contract route belongs to on the reference index.
fn family(route: &str) -> &'static str {
    if route.starts_with("/contracts/auth/") {
        "Authentication and access"
    } else if route.starts_with("/contracts/data/") {
        "Data reads"
    } else if route.starts_with("/contracts/discovery/") {
        "Discovery and composition"
    } else if ["/contracts/sessions", "/contracts/media"].contains(&route) {
        "Sessions and media"
    } else {
        "Service and execution"
    }
}

/// The label of a generated directory in the sidebar.
fn category_label(directory: &Path) -> String {
    let name = directory
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name == "contracts" {
        return if directory == Path::new("contracts") {
            "Contracts".to_owned()
        } else {
            "Native contracts".to_owned()
        };
    }
    match name {
        "auth" => "Authentication and access".to_owned(),
        "data" => "Data reads".to_owned(),
        "discovery" => "Discovery and composition".to_owned(),
        "model" => "ESS model".to_owned(),
        "domains" => "Domains".to_owned(),
        other => other.replace(['-', '_'], " "),
    }
}

/// The ESS pages of one model root: page id (path without `.md`) to Markdown.
fn ess_pages(root: &Path, ess: &Path, source: &str) -> crate::Result<BTreeMap<String, String>> {
    let base = root.join(".local/tmp");
    fs::create_dir_all(&base).map_err(|error| format!("creating {}: {error}", base.display()))?;
    let temp = tempfile::Builder::new()
        .prefix("connectors-docs-")
        .tempdir_in(&base)
        .map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = Command::new(ess)
        .args(["generate", "--path"])
        .arg(root.join(source))
        .args(["--kind", "docs", "--out"])
        .arg(temp.path())
        .output()
        .map_err(|error| format!("running {}: {error}", ess.display()))?;
    if !output.status.success() {
        return Err(format!(
            "ess generate --kind docs for {source} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let docs = temp.path().join("docs");
    let mut pages = BTreeMap::new();
    collect(&docs, &docs, &mut pages)?;
    if pages.is_empty() {
        return Err(format!("ess generated no pages for {source}"));
    }
    Ok(pages)
}

fn collect(base: &Path, dir: &Path, pages: &mut BTreeMap<String, String>) -> crate::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|error| format!("reading {}: {error}", dir.display()))?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(base, &path, pages)?;
        } else if let Some(id) = path
            .strip_prefix(base)
            .ok()
            .and_then(Path::to_str)
            .and_then(|relative| relative.strip_suffix(".md"))
        {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("reading {}: {error}", path.display()))?;
            pages.insert(id.replace('\\', "/"), text);
        } else {
            return Err(format!("ess wrote an unexpected file {}", path.display()));
        }
    }
    Ok(())
}

/// The body of an ESS page without the provenance comment it opens with.
fn strip_ess_banner(page: &str) -> crate::Result<&str> {
    let rest = page
        .strip_prefix("<!--")
        .ok_or("ess page does not open with its provenance comment")?;
    let end = rest
        .find("-->")
        .ok_or("ess provenance comment is not closed")?;
    Ok(rest[end + 3..].trim_start_matches('\n'))
}

/// The level-one heading of a Markdown body and its first paragraph after it.
fn title_and_summary(body: &str) -> Option<(String, String)> {
    let mut lines = body.lines().skip_while(|line| line.trim().is_empty());
    let title = lines.next()?.strip_prefix("# ")?.trim().to_owned();
    let summary = lines
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    Some((title, summary))
}

/// A one-line description from Markdown prose: no backticks, links or emphasis markers.
fn plain(text: &str) -> String {
    let link = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("link pattern");
    let text = link.replace_all(text, "$1");
    let text: String = text.chars().filter(|c| !"`*".contains(*c)).collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > 240 {
        let cut: String = text.chars().take(237).collect();
        format!("{}...", cut.trim_end())
    } else {
        text
    }
}

/// Insert `note` after the level-one heading that opens `body`, or open the page with a
/// heading `title` and the note when the body has none.
fn with_note(body: &str, title: &str, note: &str) -> String {
    let leading = body.len() - body.trim_start().len();
    let rest = &body[leading..];
    if let Some(heading) = rest.strip_prefix("# ") {
        let end = heading.find('\n').map_or(heading.len(), |i| i + 1);
        format!("# {}\n{note}{}", heading[..end].trim_end(), &heading[end..])
    } else {
        format!("# {title}\n\n{note}\n\n{rest}")
    }
}

/// Every generated reference file, keyed by its path relative to the repository root.
pub fn outputs(root: &Path, ess: &Path) -> crate::Result<BTreeMap<String, String>> {
    let bytes = fs::read(root.join(INVENTORY)).map_err(|error| format!("{INVENTORY}: {error}"))?;
    let inventory: Inventory =
        serde_json::from_slice(&bytes).map_err(|error| format!("{INVENTORY}: {error}"))?;
    let mut routes = BTreeSet::new();
    // Hand-written pages own these routes; a generated page cannot shadow them.
    for reserved in ["/cli", "/crates", "/adapters", "/contracts"] {
        routes.insert(reserved.to_owned());
    }
    let mut links = BTreeMap::new();
    for page in &inventory.pages {
        route(&page.route, &mut routes)?;
        let path = public_source(root, &page.source, &page.owner)?;
        if links.insert(path, page_file(&page.route)).is_some() {
            return Err(format!("source listed twice: {}", page.source));
        }
    }
    let mut out = BTreeMap::new();
    let mut directories = BTreeSet::new();
    for (index, page) in inventory.pages.iter().enumerate() {
        let source = public_source(root, &page.source, &page.owner)?;
        let file = page_file(&page.route);
        let text =
            fs::read_to_string(&source).map_err(|error| format!("{}: {error}", page.source))?;
        let body = markdown::rewrite(&markdown::without_history(&text), &|destination| {
            resolve_contract_link(&source, &file, destination, &links)
        })
        .map_err(|error| format!("{}: {error}", page.source))?;
        let note = format!(
            "\n:::info[{}]\n\nRendered from [`{}`]({SOURCE}/{}) by `connectors-docs`; the source owns every rule on this page.\n\n:::\n",
            page.status.replace(['[', ']'], ""),
            page.source,
            page.source
        );
        let description = format!("{}. Contract owner: {}.", page.status, page.owner);
        let content = format!(
            "{}{}",
            front_matter(&page.title, &page.title, &description, index + 1),
            with_note(&body, &page.title, &note)
        );
        record_directories(&file, &mut directories);
        out.insert(format!("{REFERENCE}/{}", file.display()), content);
    }
    for model in &inventory.models {
        let expected = if model.owner == "shared" {
            "ess".to_owned()
        } else {
            format!("adapters/{}/spec/ess", model.owner)
        };
        if model.source != expected {
            return Err(format!(
                "model root {} disagrees with its owner",
                model.source
            ));
        }
        route(&model.route, &mut routes)?;
        let pages = ess_pages(root, ess, &model.source)?;
        let ids: BTreeSet<String> = pages.keys().cloned().collect();
        for (position, (id, page)) in pages.iter().enumerate() {
            let page_route = format!("{}/{id}", model.route);
            route(&page_route, &mut routes)?;
            let file = page_file(&page_route);
            let body = strip_ess_banner(page)?;
            let (title, summary) = title_and_summary(body)
                .ok_or_else(|| format!("{}: ess page {id} has no title", model.source))?;
            let here = Path::new(id);
            let rewritten = markdown::rewrite(body, &|destination| {
                if destination.starts_with('#') || destination.starts_with("https://") {
                    return Some(destination.to_owned());
                }
                let (target, anchor) = destination
                    .split_once('#')
                    .map_or((destination, None), |(a, b)| (a, Some(b)));
                let joined = here.parent().unwrap_or(Path::new("")).join(target);
                let mut parts: Vec<&str> = Vec::new();
                for part in joined.to_str()?.split('/') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            parts.pop()?;
                        }
                        part => parts.push(part),
                    }
                }
                let resolved = parts.join("/");
                ids.contains(resolved.strip_suffix(".md")?).then(|| {
                    let link = if destination.starts_with('.') {
                        target.to_owned()
                    } else {
                        format!("./{target}")
                    };
                    anchor.map_or(link.clone(), |anchor| format!("{link}#{anchor}"))
                })
            })
            .map_err(|error| format!("{}: ess page {id}: {error}", model.source))?;
            let label = if id == "index" {
                "Model overview".to_owned()
            } else {
                title.clone()
            };
            let description = if summary.is_empty() {
                format!(
                    "The {title} page of the {} ESS model, generated by ESS.",
                    model.owner
                )
            } else {
                plain(&summary)
            };
            let note = format!(
                "\n:::info[Typed specification]\n\nGenerated by the pinned `ess generate --kind docs` from [`{}`]({}/{}). Structural validation establishes consistency; behaviour, storage and cross-record predicates remain implementation obligations.\n\n:::\n",
                model.source,
                SOURCE.replace("/blob/", "/tree/"),
                model.source
            );
            let content = format!(
                "{}{}",
                front_matter(&title, &label, &description, position + 1),
                with_note(&rewritten, &title, &note)
            );
            record_directories(&file, &mut directories);
            out.insert(format!("{REFERENCE}/{}", file.display()), content);
        }
    }
    out.insert(
        format!("{REFERENCE}/contracts/index.md"),
        index_page(&inventory),
    );
    directories.remove(Path::new("adapters"));
    for directory in directories {
        let name = directory
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if directory.parent() == Some(Path::new("adapters")) {
            // An adapter's directory belongs to its hand-written page.
            continue;
        }
        let position = match name {
            "contracts" if directory == Path::new("contracts") => 3,
            "contracts" => 1,
            "model" => 2,
            _ => 50,
        };
        out.insert(
            format!("{REFERENCE}/{}/_category_.json", directory.display()),
            format!(
                "{{\n  \"label\": {},\n  \"position\": {position}\n}}\n",
                yaml_string(&category_label(&directory))
            ),
        );
    }
    Ok(out)
}

/// Every ancestor directory of a reference page, relative to the reference directory.
fn record_directories(file: &Path, directories: &mut BTreeSet<PathBuf>) {
    let mut current = file.parent();
    while let Some(directory) = current.filter(|d| !d.as_os_str().is_empty()) {
        directories.insert(directory.to_path_buf());
        current = directory.parent();
    }
}

/// The contract reference index: every selected page, grouped, with its status.
fn index_page(inventory: &Inventory) -> String {
    let mut out = front_matter(
        "Contract reference",
        "Contracts",
        "Every published contract and model page: shared families, adapter-owned native profiles and the ESS models, each with its support status.",
        0,
    );
    out.push_str(
        "# Contract reference\n\nEach page below renders its owner's contract source unchanged in meaning: shared families from `contracts/`, native profiles from `adapters/<owner>/contracts/`. The status beside each page is the publication inventory's statement of what runs; a specification is not a runtime. `website/publication.json` selects the pages, and `connectors-docs` generates them.\n"
            .replace("<owner>", "&lt;owner>")
            .as_str(),
    );
    let mut families: Vec<(&str, Vec<&Page>)> = Vec::new();
    for page in inventory.pages.iter().filter(|p| p.owner == "shared") {
        let name = family(&page.route);
        match families.iter_mut().find(|(family, _)| *family == name) {
            Some((_, pages)) => pages.push(page),
            None => families.push((name, vec![page])),
        }
    }
    let here = Path::new("contracts/index.md");
    for (name, pages) in families {
        out.push_str(&format!(
            "\n## {name}\n\n| Contract | Status |\n|---|---|\n"
        ));
        for page in pages {
            out.push_str(&format!(
                "| [{}]({}) | {} |\n",
                cell(&page.title),
                relative_link(here, &page_file(&page.route)),
                cell(&page.status)
            ));
        }
    }
    out.push_str(
        "\n## Adapter-owned native contracts\n\n| Adapter | Contract | Status |\n|---|---|---|\n",
    );
    for page in inventory.pages.iter().filter(|p| p.owner != "shared") {
        out.push_str(&format!(
            "| {} | [{}]({}) | {} |\n",
            page.owner,
            cell(&page.title),
            relative_link(here, &page_file(&page.route)),
            cell(&page.status)
        ));
    }
    out.push_str("\n## ESS models\n\n| Owner | Model |\n|---|---|\n");
    for model in &inventory.models {
        out.push_str(&format!(
            "| {} | [Model overview]({}) |\n",
            model.owner,
            relative_link(here, &page_file(&format!("{}/index", model.route)))
        ));
    }
    out
}

/// Files in the wholly generated directories that `outputs` does not produce.
pub fn orphans(root: &Path, outputs: &BTreeMap<String, String>) -> crate::Result<Vec<String>> {
    let reference = root.join(REFERENCE);
    let mut owned = vec![reference.join("contracts")];
    if let Ok(entries) = fs::read_dir(reference.join("adapters")) {
        let mut adapters: Vec<_> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        adapters.sort();
        for adapter in adapters {
            owned.push(adapter.join("contracts"));
            owned.push(adapter.join("model"));
        }
    }
    let mut found = Vec::new();
    for directory in owned {
        files(root, &directory, &mut found)?;
    }
    found.retain(|relative| !outputs.contains_key(relative));
    Ok(found)
}

fn files(root: &Path, dir: &Path, out: &mut Vec<String>) -> crate::Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    let mut entries: Vec<_> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(root, &path, out)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("{} is outside the repository", path.display()))?;
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_must_be_clean_and_unique() {
        let mut routes = BTreeSet::new();
        route("/contracts/a", &mut routes).unwrap();
        for bad in [
            "/contracts/a",
            "contracts/b",
            "/a/../b",
            "/a/",
            "/a?b",
            "/a b",
            "/a//b",
        ] {
            assert!(route(bad, &mut routes).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_source_must_lie_inside_its_owner() {
        let root = crate::repository_root();
        assert!(public_source(&root, "contracts/service/clock.md", "shared").is_ok());
        for (source, owner) in [
            ("contracts/../README.md", "shared"),
            ("docs/design.md", "shared"),
            ("contracts/service/clock.md", "sql"),
            ("adapters/sql/contracts/../../../README.md", "sql"),
        ] {
            assert!(public_source(&root, source, owner).is_err(), "{source}");
        }
    }

    #[test]
    fn links_are_relative_between_page_files() {
        let from = Path::new("contracts/auth/connection.md");
        assert_eq!(
            relative_link(from, Path::new("contracts/auth/custody.md")),
            "./custody.md"
        );
        assert_eq!(
            relative_link(from, Path::new("contracts/clock.md")),
            "../clock.md"
        );
        assert_eq!(
            relative_link(from, Path::new("adapters/sql/contracts/reads.md")),
            "../../adapters/sql/contracts/reads.md"
        );
    }

    #[test]
    fn only_selected_local_links_resolve_and_keep_their_anchor() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("contracts")).unwrap();
        let a = dir.path().join("contracts/a.md");
        let b = dir.path().join("contracts/b.md");
        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();
        let links =
            BTreeMap::from([(b.canonicalize().unwrap(), PathBuf::from("contracts/x/b.md"))]);
        let page = Path::new("contracts/a.md");
        assert_eq!(
            resolve_contract_link(&a, page, "b.md#rules", &links),
            Some("./x/b.md#rules".to_owned())
        );
        assert!(resolve_contract_link(&a, page, "a.md", &links).is_none());
        assert!(resolve_contract_link(&a, page, "//example.test/pixel", &links).is_none());
        assert!(resolve_contract_link(&a, page, "javascript:alert(1)", &links).is_none());
        assert!(resolve_contract_link(&a, page, "http://example.test/", &links).is_none());
    }

    #[test]
    fn a_symlink_out_of_the_owner_is_not_published() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir(root.join("contracts")).unwrap();
        fs::write(root.join("private.md"), "private").unwrap();
        std::os::unix::fs::symlink(root.join("private.md"), root.join("contracts/link.md"))
            .unwrap();
        assert!(public_source(&root, "contracts/link.md", "shared").is_err());
        fs::write(root.join("contracts/real.md"), "public").unwrap();
        assert!(public_source(&root, "contracts/real.md", "shared").is_ok());
    }

    #[test]
    fn a_page_note_follows_its_heading() {
        assert_eq!(
            with_note("# T\n\nBody\n", "X", "NOTE\n"),
            "# T\nNOTE\n\nBody\n"
        );
        assert_eq!(with_note("Body\n", "X", "NOTE"), "# X\n\nNOTE\n\nBody\n");
    }

    #[test]
    fn descriptions_drop_markup() {
        assert_eq!(plain("`a` is **b** with [c](d.md)."), "a is b with c.");
    }
}
