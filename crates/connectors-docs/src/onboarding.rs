//! Connector onboarding: one hand-written page per connector under `website/docs/connectors/`,
//! each with the same eight `##` sections in the same order, and the generated index of every
//! connector.
//!
//! Rule 6 of `vocabulary` ([`problems`]) runs whenever the repository's `README.md` is checked.
//! It fails when
//!
//! - a page under `website/docs/connectors/` (the index aside) does not have exactly the
//!   [`SECTIONS`], in that order, as its `##` headings;
//! - the README has no connector support table (a table whose header's first cell is
//!   `Connector`), or a connector that table names has no page: a row names the page whose
//!   `title` is the row's first cell without its parenthesis and with links reduced to their
//!   text (`[GitLab](…) (catalog provider)` is `GitLab`), or whose `readme_row` is that name
//!   (the PostgreSQL and MySQL pages for the row `SQL`);
//! - a row's first cell links no onboarding page on the site ([`SITE_PAGES`]`<page>/`), or
//!   links one that does not exist;
//! - a page no row names, unless a `:::caution[Planned]` admonition precedes its first
//!   section: every connector that runs is in the README's table, and one that does not run
//!   is marked so on its page.
//!
//! [`index`] renders `website/docs/connectors/index.md`. A row per catalog connector comes from
//! its declaration (`adapters/catalog/providers/<p>/connector.json` and the selection it names);
//! the connectors without one (Slack, OpenRouter, the connectors served by their own executables,
//! MCP) have a row in [`HAND`]. Each row links the page whose front matter `connectors` names it,
//! and generation fails when a row has no page, a page no row, or a catalog provider directory
//! neither a declaration nor a hand row.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::Deserialize;

use crate::{FRONT_MATTER_HEADER, Result, SOURCE, cell, yaml_string};

/// The onboarding pages, relative to the repository root.
pub const DIRECTORY: &str = "website/docs/connectors";
/// The generated index of every connector.
pub const INDEX: &str = "website/docs/connectors/index.md";
/// The catalog provider's per-provider declarations and selections.
const PROVIDERS: &str = "adapters/catalog/providers";

/// The `##` sections of every onboarding page, in order.
pub const SECTIONS: [&str; 8] = [
    "What it reads and writes",
    "Prerequisites at the provider",
    "Enable the connector and add an instance",
    "Connect",
    "A first read",
    "A first guarded write",
    "Reconnect when the connection is Stale",
    "Limits",
];

/// What the generator needs of one onboarding page.
#[derive(Debug, Default)]
struct Page {
    /// The file name without `.md`.
    stem: String,
    title: String,
    /// The README support-table row the page answers when its title differs.
    readme_row: Option<String>,
    /// The configured connector names the page onboards.
    connectors: Vec<String>,
    /// The `##` headings outside code, in order.
    sections: Vec<String>,
    /// Whether a `:::caution[Planned]` admonition precedes the first section: the connector
    /// does not run, and the README's support table does not list it.
    planned: bool,
}

impl Page {
    /// Whether the page answers the README support-table row `name`: its title, or its
    /// `readme_row`.
    fn answers(&self, name: &str) -> bool {
        self.title.eq_ignore_ascii_case(name) || self.readme_row.as_deref() == Some(name)
    }
}

/// A front matter value without its YAML double quotes.
fn unquote(value: &str) -> String {
    let value = value.trim();
    match value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
    {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => value.to_owned(),
    }
}

/// Parse one page: its front matter keys and its `##` headings outside code fences.
fn page(stem: &str, text: &str) -> Page {
    let mut page = Page {
        stem: stem.to_owned(),
        ..Page::default()
    };
    let mut body = text;
    if let Some(rest) = text.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---\n")
    {
        for line in rest[..end].lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            match key {
                "title" => page.title = unquote(value),
                "readme_row" => page.readme_row = Some(unquote(value)),
                "connectors" => {
                    page.connectors = unquote(value)
                        .split(',')
                        .map(|name| name.trim().to_owned())
                        .filter(|name| !name.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
        body = &rest[end + 5..];
    }
    let mut fence: Option<String> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker: String = trimmed
            .chars()
            .take_while(|c| *c == '`' || *c == '~')
            .collect();
        if marker.len() >= 3 {
            match &fence {
                None => {
                    fence = Some(marker);
                    continue;
                }
                Some(open) if marker.starts_with(open.as_str()) => {
                    fence = None;
                    continue;
                }
                Some(_) => {}
            }
        }
        if fence.is_some() {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            page.sections.push(heading.trim().to_owned());
        } else if page.sections.is_empty() && line.trim() == ":::caution[Planned]" {
            page.planned = true;
        }
    }
    page
}

/// Every onboarding page of the repository at `root`, the index aside, sorted by file name. A
/// missing directory holds none.
fn pages(root: &Path) -> Result<Vec<Page>> {
    let directory = root.join(DIRECTORY);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<_> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter(|path| !path.ends_with("index.md"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path)
                .map_err(|error| format!("reading {}: {error}", path.display()))?;
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(page(&stem, &text))
        })
        .collect()
}

/// The first cells of a Markdown table row, trimmed.
fn cells(line: &str) -> Option<Vec<&str>> {
    let row = line.trim().strip_prefix('|')?;
    let row = row.strip_suffix('|').unwrap_or(row);
    Some(row.split('|').map(str::trim).collect())
}

/// Where the site serves the onboarding page `<stem>`: `<SITE_PAGES><stem>/`.
pub const SITE_PAGES: &str = "https://beyond10x.github.io/connectors/docs/connectors/";

/// One row of the README's support table.
#[derive(Debug, PartialEq, Eq)]
pub struct SupportRow {
    /// The first cell without a trailing parenthesis, a link reduced to its text.
    pub name: String,
    /// The onboarding pages the first cell links, by file stem.
    pub pages: Vec<String>,
}

/// The text of a cell with every Markdown link `[text](target)` reduced to `text`.
fn link_text(cell: &str) -> String {
    let mut out = String::new();
    let mut rest = cell;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|at| open + at) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|at| close + at) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The onboarding pages a cell links, by file stem, in order.
fn linked_pages(cell: &str) -> Vec<String> {
    cell.split(&format!("]({SITE_PAGES}"))
        .skip(1)
        .filter_map(|after| after.split_once("/)").map(|(stem, _)| stem.to_owned()))
        .collect()
}

/// The connectors of the README's support table: each row's first cell, without a trailing
/// parenthesis and with links reduced to their text, and the onboarding pages that cell links.
/// `None` when there is no such table or it has no row.
pub fn support_rows(readme: &str) -> Option<Vec<SupportRow>> {
    let lines: Vec<&str> = readme.lines().collect();
    let header = lines.iter().position(|line| {
        cells(line).is_some_and(|cells| cells.first().is_some_and(|first| *first == "Connector"))
    })?;
    let separator = lines.get(header + 1)?;
    if !cells(separator)
        .is_some_and(|cells| cells.iter().all(|c| c.chars().all(|c| "-: ".contains(c))))
    {
        return None;
    }
    let rows: Vec<SupportRow> = lines[header + 2..]
        .iter()
        .map_while(|line| cells(line))
        .filter_map(|cells| cells.first().map(|first| (*first).to_owned()))
        .map(|first| {
            let name = first
                .split_once(" (")
                .map_or(first.as_str(), |(name, _)| name);
            SupportRow {
                name: link_text(name).trim().to_owned(),
                pages: linked_pages(&first),
            }
        })
        .collect();
    (!rows.is_empty()).then_some(rows)
}

/// How a page's sections differ from [`SECTIONS`], if they do.
fn section_problem(sections: &[String]) -> Option<String> {
    for (index, expected) in SECTIONS.iter().enumerate() {
        match sections.get(index) {
            None => {
                return Some(format!(
                    "has no section \"{expected}\" (section {})",
                    index + 1
                ));
            }
            Some(found) if found != expected => {
                return Some(format!(
                    "section {} is \"{found}\", not \"{expected}\"",
                    index + 1
                ));
            }
            Some(_) => {}
        }
    }
    sections.get(SECTIONS.len()).map(|extra| {
        format!(
            "section {} \"{extra}\" follows \"{}\"; the eight onboarding sections are all",
            SECTIONS.len() + 1,
            SECTIONS[SECTIONS.len() - 1]
        )
    })
}

/// Rule 6: the problems of the onboarding pages under `root` against the README `readme`, each
/// as `path: problem`.
pub fn problems(root: &Path, readme: &str) -> Result<Vec<String>> {
    let pages = pages(root)?;
    let mut problems: Vec<String> = pages
        .iter()
        .filter_map(|page| {
            section_problem(&page.sections)
                .map(|problem| format!("{DIRECTORY}/{}.md: {problem}", page.stem))
        })
        .collect();
    match support_rows(readme) {
        None => problems.push(
            "README.md: has no connector support table (a table whose first header cell is `Connector`)"
                .to_owned(),
        ),
        Some(rows) => {
            for row in &rows {
                let name = &row.name;
                if row.pages.is_empty() {
                    problems.push(format!(
                        "README.md: the support table's connector \"{name}\" links no onboarding page ({SITE_PAGES}<page>/)"
                    ));
                }
                for stem in row.pages.iter().filter(|stem| !pages.iter().any(|page| &page.stem == *stem)) {
                    problems.push(format!(
                        "README.md: the support table's connector \"{name}\" links {DIRECTORY}/{stem}.md, which does not exist"
                    ));
                }
                if !pages.iter().any(|page| page.answers(name)) {
                    problems.push(format!(
                        "README.md: the support table's connector \"{name}\" has no page under {DIRECTORY}/"
                    ));
                }
            }
            for page in pages.iter().filter(|page| !page.planned) {
                if !rows.iter().any(|row| page.answers(&row.name)) {
                    problems.push(format!(
                        "{DIRECTORY}/{}.md: no row of README.md's support table names it by its title or readme_row, and it is not marked :::caution[Planned]",
                        page.stem
                    ));
                }
            }
        }
    }
    Ok(problems)
}

/// A connector without a catalog declaration, written here by hand.
pub struct Hand {
    /// The configured connector name.
    pub connector: &'static str,
    pub provider: &'static str,
    /// The catalog provider directory whose selections hold its operations, if any.
    pub catalog: Option<&'static str>,
    /// Its selections under that directory, read for the operation counts.
    pub selections: &'static [&'static str],
    /// Otherwise, its reads and writes as their guides list them.
    pub reads: &'static [&'static str],
    pub writes: &'static [&'static str],
    pub profiles: &'static [&'static str],
    /// How an instance of it is configured.
    pub configured: &'static str,
}

/// How a pinned instance is configured.
const PINNED: &str = "pinned";

/// The connectors with no catalog declaration. Slack is served by the catalog provider from two
/// selections and OpenRouter from one, each through pinned instances; the others by an executable
/// of their own, each pinned by its instance; MCP has no runtime.
pub const HAND: &[Hand] = &[
    Hand {
        connector: "slack",
        provider: "Slack",
        catalog: Some("slack"),
        selections: &["operations.json", "user-operations.json"],
        reads: &[],
        writes: &[],
        profiles: &["slack.bot", "slack.user"],
        configured: PINNED,
    },
    Hand {
        connector: "openrouter",
        provider: "OpenRouter",
        catalog: Some("openrouter"),
        selections: &["operations.json"],
        reads: &[],
        writes: &[],
        profiles: &["openrouter.api-key"],
        configured: PINNED,
    },
    Hand {
        connector: "kubernetes",
        provider: "Kubernetes",
        catalog: None,
        selections: &[],
        reads: &[
            "resources.list",
            "resources.get",
            "namespaces.list",
            "deployments.history",
            "endpoints.discover",
            "hosts.discover",
            "pods.logs",
            "contexts.list",
            "nodes.list",
            "custom_resources.list",
            "helm_releases.history",
            "helm_releases.status",
            "helm_releases.values",
            "helm_releases.manifest",
        ],
        writes: &["pods.exec"],
        profiles: &["kubernetes.token"],
        configured: PINNED,
    },
    Hand {
        connector: "postgres",
        provider: "PostgreSQL",
        catalog: None,
        selections: &[],
        reads: &[
            "schema.list",
            "query.read",
            "database.list",
            "table.list",
            "table.describe",
            "index.list",
            "profile.read",
        ],
        writes: &[],
        profiles: &["postgres.password"],
        configured: PINNED,
    },
    Hand {
        connector: "mysql",
        provider: "MySQL",
        catalog: None,
        selections: &[],
        reads: &[
            "schema.list",
            "query.read",
            "database.list",
            "table.list",
            "table.describe",
            "index.list",
            "profile.read",
        ],
        writes: &[],
        profiles: &["mysql.password"],
        configured: PINNED,
    },
    Hand {
        connector: "tavily",
        provider: "Tavily",
        catalog: None,
        selections: &[],
        reads: &["websearch.search", "websearch.fetch", "websearch.crawl"],
        writes: &[],
        profiles: &["tavily.api-key"],
        configured: PINNED,
    },
    Hand {
        connector: "loki",
        provider: "Grafana Labs",
        catalog: None,
        selections: &[],
        reads: &[
            "logs.query_range",
            "logs.query_metric",
            "logs.labels",
            "logs.profile_lines",
            "logs.call_trace",
        ],
        writes: &[],
        profiles: &["loki.bearer"],
        configured: PINNED,
    },
    Hand {
        connector: "grafana",
        provider: "Grafana Labs",
        catalog: None,
        selections: &[],
        reads: &[
            "datasources.list",
            "irm.alert_groups.list",
            "irm.alert_group.get",
            "irm.incidents.list",
            "irm.incident.get",
        ],
        writes: &[],
        profiles: &["grafana.service_account", "grafana.irm_service_account"],
        configured: PINNED,
    },
    Hand {
        connector: "prometheus",
        provider: "Prometheus",
        catalog: None,
        selections: &[],
        reads: &["series.query", "series.query_range", "rules.list"],
        writes: &[],
        profiles: &["prometheus.bearer"],
        configured: PINNED,
    },
    Hand {
        connector: "mcp",
        provider: "any MCP server",
        catalog: None,
        selections: &[],
        reads: &[],
        writes: &[],
        profiles: &[],
        configured: "not yet: planned",
    },
];

#[derive(Deserialize)]
struct Declaration {
    connector_id: String,
    provider_id: String,
    selection: String,
    profiles: Vec<Profile>,
}

#[derive(Deserialize)]
struct Profile {
    profile: String,
    /// An `oauth2_grant` profile's declaration; one without a `client_id` cannot be derived.
    #[serde(default)]
    oauth2: Option<Grant>,
}

#[derive(Deserialize)]
struct Grant {
    client_id: Option<String>,
}

#[derive(Deserialize)]
struct Selection {
    operations: Vec<Operation>,
}

#[derive(Deserialize)]
struct Operation {
    id: String,
    effect: String,
}

/// One row of the index.
struct Row {
    connector: String,
    provider: String,
    reads: usize,
    writes: usize,
    profiles: Vec<String>,
    configured: String,
    /// Where the row comes from: a declaration's path, or the hand-written table.
    source: Option<String>,
}

fn read_json<T: for<'de> Deserialize<'de>>(root: &Path, relative: &str) -> Result<T> {
    let text = fs::read_to_string(root.join(relative))
        .map_err(|error| format!("reading {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative}: {error}"))
}

/// The distinct read and write ids of `selections` under the catalog provider directory
/// `directory`.
fn counts(root: &Path, directory: &str, selections: &[&str]) -> Result<(usize, usize)> {
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    for selection in selections {
        let relative = format!("{PROVIDERS}/{directory}/{selection}");
        let selection: Selection = read_json(root, &relative)?;
        for operation in selection.operations {
            match operation.effect.as_str() {
                "read" => reads.insert(operation.id),
                "write" => writes.insert(operation.id),
                other => return Err(format!("{relative}: unknown effect `{other}`")),
            };
        }
    }
    Ok((reads.len(), writes.len()))
}

/// Every row: one per declared catalog connector, then [`HAND`].
fn rows(root: &Path) -> Result<Vec<Row>> {
    let mut directories: Vec<String> = fs::read_dir(root.join(PROVIDERS))
        .map_err(|error| format!("reading {PROVIDERS}: {error}"))?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    directories.sort();
    let mut rows = Vec::new();
    for directory in directories {
        let relative = format!("{PROVIDERS}/{directory}/connector.json");
        if !root.join(&relative).is_file() {
            if HAND
                .iter()
                .any(|hand| hand.catalog == Some(directory.as_str()))
            {
                continue;
            }
            return Err(format!(
                "{PROVIDERS}/{directory} has neither a connector declaration nor a hand-written \
                 index row in crates/connectors-docs/src/onboarding.rs"
            ));
        }
        let declaration: Declaration = read_json(root, &relative)?;
        let (reads, writes) = counts(root, &directory, &[declaration.selection.as_str()])?;
        // A connector whose every profile is a grant shipping no client id has only pinned
        // instances: the client id is the instance's configuration.
        let derivable = declaration.profiles.iter().any(|profile| {
            profile
                .oauth2
                .as_ref()
                .is_none_or(|grant| grant.client_id.is_some())
        });
        rows.push(Row {
            connector: declaration.connector_id,
            provider: provider_name(&declaration.provider_id),
            reads,
            writes,
            profiles: declaration
                .profiles
                .into_iter()
                .map(|profile| profile.profile)
                .collect(),
            configured: if derivable { "derived" } else { PINNED }.to_owned(),
            source: Some(relative),
        });
    }
    for hand in HAND {
        let (reads, writes) = match hand.catalog {
            Some(directory) => counts(root, directory, hand.selections)?,
            None => (hand.reads.len(), hand.writes.len()),
        };
        rows.push(Row {
            connector: hand.connector.to_owned(),
            provider: hand.provider.to_owned(),
            reads,
            writes,
            profiles: hand.profiles.iter().map(|p| (*p).to_owned()).collect(),
            configured: hand.configured.to_owned(),
            source: None,
        });
    }
    Ok(rows)
}

/// The display name of a declared provider id.
fn provider_name(id: &str) -> String {
    match id {
        "gitlab" => "GitLab".to_owned(),
        "hubspot" => "HubSpot".to_owned(),
        other => {
            let mut chars = other.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        }
    }
}

/// `website/docs/connectors/index.md`, generated from the declarations, [`HAND`] and the pages.
pub fn index(root: &Path) -> Result<String> {
    let pages = pages(root)?;
    let rows = rows(root)?;
    let mut by_connector: BTreeMap<&str, &Page> = BTreeMap::new();
    for page in &pages {
        if page.connectors.is_empty() {
            return Err(format!(
                "{DIRECTORY}/{}.md: front matter names no `connectors`",
                page.stem
            ));
        }
        for connector in &page.connectors {
            if let Some(other) = by_connector.insert(connector, page) {
                return Err(format!(
                    "connector `{connector}` has two onboarding pages, {}.md and {}.md",
                    other.stem, page.stem
                ));
            }
        }
    }
    let listed: BTreeSet<&str> = rows.iter().map(|row| row.connector.as_str()).collect();
    if let Some(unlisted) = by_connector.keys().find(|name| !listed.contains(*name)) {
        return Err(format!(
            "{DIRECTORY}/{}.md onboards `{unlisted}`, which no declaration and no hand-written \
             row lists",
            by_connector[unlisted].stem
        ));
    }
    let mut table = String::from(
        "| Onboarding page | Provider | Reads | Writes | Auth profiles | Instances | Listed from |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for row in &rows {
        let page = by_connector.get(row.connector.as_str()).ok_or_else(|| {
            format!(
                "connector `{}` has no onboarding page under {DIRECTORY}/ (a page names it in \
                 its front matter `connectors`)",
                row.connector
            )
        })?;
        let profiles = if row.profiles.is_empty() {
            "none".to_owned()
        } else {
            row.profiles
                .iter()
                .map(|profile| format!("`{profile}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let source = row.source.as_ref().map_or_else(
            || "hand-written".to_owned(),
            |source| format!("[declaration]({SOURCE}/{source})"),
        );
        table.push_str(&format!(
            "| [{}](./{}.md) `{}` | {} | {} | {} | {} | {} | {} |\n",
            cell(&page.title),
            page.stem,
            row.connector,
            cell(&row.provider),
            row.reads,
            row.writes,
            profiles,
            row.configured,
            source,
        ));
    }
    let sections = SECTIONS
        .iter()
        .map(|section| section.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("; ");
    Ok(format!(
        "---\n{FRONT_MATTER_HEADER}\ntitle: {}\nsidebar_label: {}\nsidebar_position: 1\ndescription: {}\nlede: {}\nsource: {}\ncustom_edit_url: null\n---\n\n\
         # Connectors\n\n\
         Every connector you can configure, with its provider, how many reads and writes it \
         offers, the auth profiles it declares and how its instances are configured. The words \
         are those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary). \
         Each connector's page walks the same steps, in this order: {sections}.\n\n\
         {table}\n\
         A **derived** instance names only what is yours (its endpoint, an optional description \
         and, where the connector declares more than one auth profile, `profile`); everything \
         else comes from the declaration shipped with the `connectors-catalog-provider` installed \
         beside `connectors`. A **pinned** instance names its executable, that executable's \
         SHA-256 and its configuration revision, because no declaration exists to derive them \
         from; its page says why. Every write is a required-approval mutation: it runs only with \
         a proof issued for its exact input.\n\n\
         The rows of the catalog connectors are generated from their declarations under \
         [`adapters/catalog/providers/`]({SOURCE}/{PROVIDERS}); the others are written by hand in \
         the generator. Start with [Set up the local CLI](../guides/set-up-the-local-cli.md) \
         if no configuration exists yet.\n",
        yaml_string("Connectors"),
        yaml_string("All connectors"),
        yaml_string(
            "Every connector, with its provider, reads, writes, auth profiles and how its instances are configured."
        ),
        yaml_string(
            "Every connector has one onboarding page with the same eight steps, from what it reads and writes to its limits."
        ),
        yaml_string(
            "adapters/catalog/providers/*/connector.json and their selections; Slack, OpenRouter, the connectors served by their own executables and MCP by hand in crates/connectors-docs/src/onboarding.rs"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_root;

    fn sections_page(title: &str, sections: &[&str]) -> String {
        let mut text = format!(
            "---\ntitle: {}\nconnectors: \"x\"\n---\n\n# {title}\n",
            yaml_string(title)
        );
        for section in sections {
            text.push_str(&format!("\n## {section}\n\nText.\n"));
        }
        text
    }

    const README: &str = "# R\n\n| Term | Meaning |\n|---|---|\n| connector | c |\n\n| Connector | Implemented operations | Binding |\n|---|---|---|\n| [GitLab](https://beyond10x.github.io/connectors/docs/connectors/gitlab/) (catalog provider) | a | b |\n| SQL ([PostgreSQL](https://beyond10x.github.io/connectors/docs/connectors/postgres/)) | c | d |\n\nAfter.\n";

    fn row(name: &str, pages: &[&str]) -> SupportRow {
        SupportRow {
            name: name.to_owned(),
            pages: pages.iter().map(|page| (*page).to_owned()).collect(),
        }
    }

    #[test]
    fn the_support_table_is_the_one_headed_connector() {
        assert_eq!(
            support_rows(README),
            Some(vec![row("GitLab", &["gitlab"]), row("SQL", &["postgres"])])
        );
        assert_eq!(
            support_rows(
                "| Connector | B |\n|---|---|\n| Both ([A](https://beyond10x.github.io/connectors/docs/connectors/a/), [B](https://beyond10x.github.io/connectors/docs/connectors/b/)) | x |\n| [Plain](https://example.com/) | y |\n"
            ),
            Some(vec![row("Both", &["a", "b"]), row("Plain", &[])])
        );
        assert_eq!(support_rows("# R\n\n| connector | c |\n"), None);
        assert_eq!(
            support_rows("| Connector | B |\n|---|---|\n\nNo rows.\n"),
            None
        );
    }

    #[test]
    fn headings_in_code_are_not_sections() {
        let text = "---\ntitle: \"A \\\"B\\\"\"\nconnectors: \"a, b\"\nreadme_row: SQL\n---\n\n## One\n\n```md\n## Not\n```\n\n````\n```\n## Nor\n```\n````\n\n## Two\n";
        let page = page("a", text);
        assert_eq!(page.title, "A \"B\"");
        assert_eq!(page.connectors, ["a", "b"]);
        assert_eq!(page.readme_row.as_deref(), Some("SQL"));
        assert_eq!(page.sections, ["One", "Two"]);
    }

    #[test]
    fn a_page_needs_the_eight_sections_in_order() {
        let all: Vec<String> = SECTIONS.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(section_problem(&all), None);
        let mut swapped = all.clone();
        swapped.swap(3, 4);
        assert_eq!(
            section_problem(&swapped).as_deref(),
            Some("section 4 is \"A first read\", not \"Connect\"")
        );
        assert_eq!(
            section_problem(&all[..7]).as_deref(),
            Some("has no section \"Limits\" (section 8)")
        );
        let mut extra = all.clone();
        extra.push("More".to_owned());
        assert!(
            section_problem(&extra)
                .unwrap()
                .starts_with("section 9 \"More\"")
        );
    }

    #[test]
    fn every_support_table_row_needs_a_page_by_title_or_readme_row() {
        let root = crate::scratch("onboarding");
        let directory = root.join(DIRECTORY);
        assert_eq!(
            problems(&root, README).unwrap(),
            [
                "README.md: the support table's connector \"GitLab\" links website/docs/connectors/gitlab.md, which does not exist",
                "README.md: the support table's connector \"GitLab\" has no page under website/docs/connectors/",
                "README.md: the support table's connector \"SQL\" links website/docs/connectors/postgres.md, which does not exist",
                "README.md: the support table's connector \"SQL\" has no page under website/docs/connectors/",
            ]
        );
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("gitlab.md"),
            sections_page("GitLab", &SECTIONS),
        )
        .unwrap();
        fs::write(
            directory.join("postgres.md"),
            sections_page("PostgreSQL", &SECTIONS[..7]),
        )
        .unwrap();
        fs::write(directory.join("index.md"), "# Index, not a page\n").unwrap();
        assert_eq!(
            problems(&root, README).unwrap(),
            [
                "website/docs/connectors/postgres.md: has no section \"Limits\" (section 8)",
                "README.md: the support table's connector \"SQL\" has no page under website/docs/connectors/",
                "website/docs/connectors/postgres.md: no row of README.md's support table names it by its title or readme_row, and it is not marked :::caution[Planned]",
            ]
        );
        let postgres = sections_page("PostgreSQL", &SECTIONS).replace(
            "connectors: \"x\"",
            "connectors: \"x\"\nreadme_row: \"SQL\"",
        );
        fs::write(directory.join("postgres.md"), postgres).unwrap();
        assert_eq!(problems(&root, README).unwrap(), Vec::<String>::new());
        let planned = sections_page("Later", &SECTIONS).replace(
            "# Later\n",
            "# Later\n\n:::caution[Planned]\n\nNot yet.\n\n:::\n",
        );
        fs::write(directory.join("later.md"), planned).unwrap();
        assert_eq!(problems(&root, README).unwrap(), Vec::<String>::new());
        let unlinked = README.replace(
            "| SQL ([PostgreSQL](https://beyond10x.github.io/connectors/docs/connectors/postgres/))",
            "| SQL",
        );
        assert_eq!(
            problems(&root, &unlinked).unwrap(),
            [
                "README.md: the support table's connector \"SQL\" links no onboarding page (https://beyond10x.github.io/connectors/docs/connectors/<page>/)"
            ]
        );
        assert_eq!(
            problems(&root, "# No table\n").unwrap(),
            [
                "README.md: has no connector support table (a table whose first header cell is `Connector`)"
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// The acceptance of the onboarding story: every connector of this README's support table
    /// has a page, and every page has the eight sections in order.
    #[test]
    fn every_support_table_connector_has_an_onboarding_page() {
        let root = repository_root();
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        let problems = problems(&root, &readme).unwrap();
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// Every declared catalog connector and every hand-written row has a page, and every page a
    /// row.
    #[test]
    fn the_index_lists_every_connector_and_links_its_page() {
        let root = repository_root();
        let index = index(&root).unwrap();
        for directory in fs::read_dir(root.join(PROVIDERS)).unwrap() {
            let path = directory.unwrap().path().join("connector.json");
            if path.is_file() {
                let declaration: Declaration =
                    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
                assert!(
                    index.contains(&format!(") `{}` |", declaration.connector_id)),
                    "{}",
                    declaration.connector_id
                );
            }
        }
        for hand in HAND {
            assert!(
                index.contains(&format!(") `{}` |", hand.connector)),
                "{}",
                hand.connector
            );
        }
        assert!(
            index.contains(
                "[MCP](./mcp.md) `mcp` | any MCP server | 0 | 0 | none | not yet: planned |"
            ),
            "MCP is listed as planned"
        );
    }
}
