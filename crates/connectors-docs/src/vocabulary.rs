//! The vocabulary check: the words a user of the CLI, the configuration or the library meets
//! are provider, connector, auth profile, instance, connection and operation, defined once in
//! the repository's `README.md` (`docs/design/connector-vocabulary.md`). "Adapter" is the
//! internal name of an implementation and its process, never of the configured thing.
//!
//! `vocabulary [--no-pins] [--one-instance] <file>...` fails when, in any given file:
//!
//! 1. a line says "adapter" (any case) and none of `adapters/` (a repository path),
//!    `adapter_id`, `` `adapter` `` (the JSON field kept for the deprecation window), or one of
//!    the words `deprecated`, `legacy` or `internal`;
//! 2. a file other than the repository's `README.md` holds a Markdown table row whose first cell
//!    is one of the six terms;
//! 3. with `--no-pins`, a line holds `configuration_revision`, `sha256` or `bundle_directory`;
//! 4. with `--one-instance`, more than one line starts with `[connectors.` and holds
//!    `.instances.`;
//! 5. the repository's `README.md` is given and has no `## Vocabulary` section with a table row
//!    for each term, or its `AGENTS.md` is given and links no `README.md#vocabulary`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Result;

/// The six terms, in the order `README.md` defines them.
pub const TERMS: [&str; 6] = [
    "provider",
    "connector",
    "auth profile",
    "instance",
    "connection",
    "operation",
];

/// What a run checks besides rules 1, 2 and 5.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Rule 3: no pinned value a user would copy by hand.
    pub no_pins: bool,
    /// Rule 4: at most one configured instance.
    pub one_instance: bool,
}

/// The role a checked file plays in rules 2 and 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The repository's `README.md`, where the terms are defined.
    Readme,
    /// The repository's `AGENTS.md`, which links the definitions.
    Agents,
    /// Any other file.
    Other,
}

/// Substrings that let a line say "adapter" (rule 1).
const ALLOWED: [&str; 3] = ["adapters/", "adapter_id", "`adapter`"];
/// Whole words, in any case, that let a line say "adapter" (rule 1).
const ALLOWED_WORDS: [&str; 3] = ["deprecated", "legacy", "internal"];
/// Values a user would copy by hand (rule 3).
const PINS: [&str; 3] = ["configuration_revision", "sha256", "bundle_directory"];

/// Whether `line` holds `word` as a whole word, in any case.
fn has_word(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|token| token.eq_ignore_ascii_case(word))
}

/// The term a Markdown table row defines: its first cell, without emphasis, code marks and
/// surrounding space, when that is one of [`TERMS`].
fn term_row(line: &str) -> Option<&'static str> {
    let row = line.trim().strip_prefix('|')?;
    let (first, _) = row.split_once('|')?;
    let cell = first
        .trim()
        .trim_matches(|c| c == '*' || c == '`' || c == '_')
        .trim()
        .to_ascii_lowercase();
    TERMS.into_iter().find(|term| *term == cell)
}

/// The level of a Markdown ATX heading line, `None` for any other line.
fn heading_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|b| *b == b'#').count();
    (1..=6)
        .contains(&level)
        .then(|| line[level..].starts_with(' ').then_some(level))
        .flatten()
}

/// Rule 5 for `README.md`: the terms its `## Vocabulary` section defines in table rows.
fn readme_problems(text: &str) -> Vec<String> {
    let mut lines = text.lines();
    if !lines.any(|line| line.trim_end() == "## Vocabulary") {
        return vec![": has no `## Vocabulary` section".to_owned()];
    }
    let defined: Vec<_> = lines
        .take_while(|line| heading_level(line).is_none_or(|level| level > 2))
        .filter_map(term_row)
        .collect();
    TERMS
        .iter()
        .filter(|term| !defined.contains(term))
        .map(|term| format!(": `## Vocabulary` has no table row for \"{term}\""))
        .collect()
}

/// The problems of one file's `text`, each as `:line: problem` or `: problem`.
pub fn problems(text: &str, role: Role, options: Options) -> Vec<String> {
    let mut problems = Vec::new();
    let mut first_instance = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.to_ascii_lowercase().contains("adapter")
            && !ALLOWED.iter().any(|allowed| line.contains(allowed))
            && !ALLOWED_WORDS.iter().any(|word| has_word(line, word))
        {
            problems.push(format!(":{number}: says \"adapter\" outside the allowlist"));
        }
        if role != Role::Readme
            && let Some(term) = term_row(line)
        {
            problems.push(format!(
                ":{number}: defines the term \"{term}\", which only README.md defines"
            ));
        }
        if options.no_pins {
            for pin in PINS.iter().filter(|pin| line.contains(*pin)) {
                problems.push(format!(":{number}: holds the pin `{pin}`"));
            }
        }
        if options.one_instance {
            let trimmed = line.trim_start();
            if trimmed.starts_with("[connectors.") && trimmed.contains(".instances.") {
                match first_instance {
                    None => first_instance = Some(number),
                    Some(first) => problems.push(format!(
                        ":{number}: configures a second instance (the first is on line {first})"
                    )),
                }
            }
        }
    }
    match role {
        Role::Readme => problems.extend(readme_problems(text)),
        Role::Agents
            if !text.contains("](README.md#vocabulary)")
                && !text.contains("](./README.md#vocabulary)") =>
        {
            problems.push(": links no `README.md#vocabulary`".to_owned());
        }
        _ => {}
    }
    problems
}

/// Every problem of every file in `files`, as `path:line: problem`. `root` is the repository
/// whose `README.md` and `AGENTS.md` play their roles; a file is named relative to it where it
/// lies inside it.
pub fn check(root: &Path, files: &[PathBuf], options: Options) -> Result<Vec<String>> {
    let canonical = |path: &Path| {
        path.canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))
    };
    let root = canonical(root)?;
    let readme = root.join("README.md");
    let agents = root.join("AGENTS.md");
    let mut found = Vec::new();
    for file in files {
        let path = canonical(file)?;
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", file.display()))?;
        let role = if path == readme {
            Role::Readme
        } else if path == agents {
            Role::Agents
        } else {
            Role::Other
        };
        let name = path
            .strip_prefix(&root)
            .map_or_else(|_| file.display().to_string(), |p| p.display().to_string());
        found.extend(
            problems(&text, role, options)
                .into_iter()
                .map(|problem| format!("{name}{problem}")),
        );
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_root;

    const README: &str = "# Title\n\n## Vocabulary\n\n| Term | Meaning |\n|---|---|\n| provider | a |\n| connector | b |\n| auth profile | c |\n| instance | d |\n| connection | e |\n| operation | f |\n\n## Next\n";

    fn other(text: &str) -> Vec<String> {
        problems(text, Role::Other, Options::default())
    }

    #[test]
    fn adapter_outside_the_allowlist_is_refused() {
        assert_eq!(
            other("Configure an adapter.\nAn Adapter entry.\n"),
            vec![
                ":1: says \"adapter\" outside the allowlist".to_owned(),
                ":2: says \"adapter\" outside the allowlist".to_owned(),
            ]
        );
        assert_eq!(other("The ADAPTERS group.\n").len(), 1);
    }

    #[test]
    fn adapter_with_an_allowlisted_neighbour_passes() {
        for line in [
            "See adapters/catalog/README.md for the adapter.",
            "adapter_id = \"catalog\"",
            "The result's `adapter` field stays.",
            "`--adapter` is deprecated.",
            "The Deprecated adapters group.",
            "A legacy `[adapters.<alias>]` table.",
            "Adapter is the internal name of an implementation.",
        ] {
            assert_eq!(other(line), Vec::<String>::new(), "{line}");
        }
    }

    #[test]
    fn allowlisted_words_count_only_as_whole_words() {
        assert_eq!(other("An adapter, internally.\n").len(), 1);
        assert_eq!(other("An undeprecated adapter.\n").len(), 1);
    }

    #[test]
    fn a_term_row_outside_the_readme_is_refused() {
        let table = "| Term | Meaning |\n|---|---|\n| **Instance** | x |\n| `connection` | y |\n| endpoint | z |\n";
        assert_eq!(
            other(table),
            vec![
                ":3: defines the term \"instance\", which only README.md defines".to_owned(),
                ":4: defines the term \"connection\", which only README.md defines".to_owned(),
            ]
        );
        assert_eq!(
            problems(README, Role::Readme, Options::default()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn pins_are_refused_only_when_asked() {
        let text = "configuration_revision = \"x\"\nsha256 = \"y\"\nbundle_directory = \"z\"\nSHA-256 and SHA256SUMS\n";
        assert_eq!(other(text), Vec::<String>::new());
        let pinned = problems(
            text,
            Role::Other,
            Options {
                no_pins: true,
                ..Options::default()
            },
        );
        assert_eq!(
            pinned,
            vec![
                ":1: holds the pin `configuration_revision`".to_owned(),
                ":2: holds the pin `sha256`".to_owned(),
                ":3: holds the pin `bundle_directory`".to_owned(),
            ]
        );
    }

    #[test]
    fn a_second_instance_is_refused_only_when_asked() {
        let one = "[connectors.gmail]\nenabled = true\n\n[connectors.gmail.instances.private]\n";
        let two = format!("{one}\n  [connectors.drive.instances.private]\n");
        let options = Options {
            one_instance: true,
            ..Options::default()
        };
        assert_eq!(problems(one, Role::Other, options), Vec::<String>::new());
        assert_eq!(other(&two), Vec::<String>::new());
        assert_eq!(
            problems(&two, Role::Other, options),
            vec![":6: configures a second instance (the first is on line 4)".to_owned()]
        );
    }

    #[test]
    fn the_readme_defines_every_term_in_its_vocabulary_section() {
        let readme = |text: &str| problems(text, Role::Readme, Options::default());
        assert_eq!(
            readme("# Title\n\nNo section.\n"),
            vec![": has no `## Vocabulary` section".to_owned()]
        );
        let missing = README.replace("| auth profile | c |\n", "");
        assert_eq!(
            readme(&missing),
            vec![": `## Vocabulary` has no table row for \"auth profile\"".to_owned()]
        );
        let elsewhere = README.replace(
            "| operation | f |\n\n## Next\n",
            "\n## Next\n\n| operation | f |\n",
        );
        assert_eq!(
            readme(&elsewhere),
            vec![": `## Vocabulary` has no table row for \"operation\"".to_owned()]
        );
        let deeper = README.replace(
            "| operation | f |\n\n## Next\n",
            "\n### Example\n\n| operation | g |\n",
        );
        assert_eq!(readme(&deeper), Vec::<String>::new());
    }

    #[test]
    fn agents_links_the_readme_vocabulary() {
        let agents = |text: &str| problems(text, Role::Agents, Options::default());
        assert_eq!(
            agents("See the README.\n"),
            vec![": links no `README.md#vocabulary`".to_owned()]
        );
        assert_eq!(
            agents("The [terms](README.md#vocabulary).\n"),
            Vec::<String>::new()
        );
        assert_eq!(
            agents("The [terms](./README.md#vocabulary).\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn check_names_each_file_and_plays_the_root_files_roles() {
        let root = crate::scratch("vocabulary");
        std::fs::create_dir_all(root.join("adapters/x")).unwrap();
        std::fs::write(root.join("README.md"), "# No vocabulary\n").unwrap();
        std::fs::write(root.join("AGENTS.md"), "[t](README.md#vocabulary)\n").unwrap();
        std::fs::write(root.join("adapters/x/README.md"), "| instance | x |\n").unwrap();
        let files = ["README.md", "AGENTS.md", "adapters/x/README.md"]
            .map(|file| root.join(file))
            .to_vec();
        let found = check(&root, &files, Options::default()).unwrap();
        assert_eq!(
            found,
            vec![
                "README.md: has no `## Vocabulary` section".to_owned(),
                "adapters/x/README.md:1: defines the term \"instance\", which only README.md defines"
                    .to_owned(),
            ]
        );
        assert!(check(&root, &[root.join("missing.md")], Options::default()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The story's checked set: the files this repository's README, AGENTS.md and site present
    /// the vocabulary in. `concepts/adapters.md` describes implementations and is not checked.
    #[test]
    fn the_readme_agents_and_site_speak_the_vocabulary() {
        let root = repository_root();
        let mut files: Vec<_> = [
            "README.md",
            "AGENTS.md",
            "website/product.json",
            "website/data/terminal.json",
            "website/docs/index.md",
            "website/docs/getting-started.md",
        ]
        .map(|file| root.join(file))
        .to_vec();
        for directory in ["website/docs/guides", "website/docs/concepts"] {
            let mut pages: Vec<_> = std::fs::read_dir(root.join(directory))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|e| e == "md"))
                .filter(|path| !path.ends_with("concepts/adapters.md"))
                .collect();
            pages.sort();
            files.extend(pages);
        }
        assert!(files.len() > 10, "{files:?}");
        let problems = check(&root, &files, Options::default()).unwrap();
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
