//! Checks every documentation page, generated or written by hand:
//!
//! - front matter with `title`, `sidebar_position` and `description`, and on concept and guide
//!   pages also `lede` and `source`;
//! - no admonition whose title is not in brackets (`:::caution Planned`), which Docusaurus prints
//!   as raw text;
//! - passive Markdown: outside code, no `{`, no `import`/`export` line, no capitalised tag and no
//!   event-handler attribute, in every `.md` page. The unified documentation site, which still
//!   lists this repository until it leaves for its own site, refuses each of those as executable
//!   MDX. The interactive examples are `.mdx` pages by design and are held to front matter and
//!   admonition titles only.

use std::{fs, path::Path};

use crate::{DOCS, Result};

/// The front matter keys every page carries.
const REQUIRED: &[&str] = &["title", "sidebar_position", "description"];
/// The keys a hand-written concept or guide page carries besides.
const REQUIRED_HAND: &[&str] = &["lede", "source"];

/// Every problem of every Markdown page of the repository at `root`, as `path:line: problem`.
pub fn check(root: &Path) -> Result<Vec<String>> {
    let docs = root.join(DOCS);
    let mut problems = Vec::new();
    visit(&docs, &docs, &mut problems)?;
    Ok(problems)
}

fn visit(docs: &Path, dir: &Path, problems: &mut Vec<String>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|error| format!("reading {}: {error}", dir.display()))?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            visit(docs, &path, problems)?;
            continue;
        }
        let markdown = path.extension().is_some_and(|ext| ext == "md");
        if !markdown && path.extension().is_none_or(|ext| ext != "mdx") {
            continue;
        }
        let relative = path
            .strip_prefix(docs)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        let hand = relative.starts_with("concepts/") || relative.starts_with("guides/");
        problems.extend(
            page_problems(&source, hand, markdown)
                .into_iter()
                .map(|problem| format!("{relative}{problem}")),
        );
    }
    Ok(())
}

/// The problems of one page, each starting with `:line: ` or `: `.
fn page_problems(source: &str, hand: bool, markdown: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let (front, body_start) = front_matter(source);
    match front {
        None => problems.push(": no front matter".to_owned()),
        Some(front) => {
            let required = REQUIRED
                .iter()
                .chain(if hand { REQUIRED_HAND } else { &[] });
            for key in required {
                if !front
                    .lines()
                    .any(|line| line.strip_prefix(key).is_some_and(|r| r.starts_with(':')))
                {
                    problems.push(format!(": front matter has no `{key}`"));
                }
            }
        }
    }
    let mut fence: Option<(char, usize)> = None;
    // The lines of the current paragraph: a code span may wrap a line, so spans are blanked
    // across a paragraph before each line is checked.
    let mut paragraph: Vec<(usize, &str)> = Vec::new();
    for (index, line) in source.lines().enumerate().skip(body_start) {
        let number = index + 1;
        if line.trim().is_empty() {
            flush(&mut paragraph, markdown, &mut problems);
        }
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        if let Some(c) = marker.filter(|_| indent < 4) {
            let run = trimmed.chars().take_while(|x| *x == c).count();
            if run >= 3 {
                match fence {
                    None => {
                        flush(&mut paragraph, markdown, &mut problems);
                        fence = Some((c, run));
                        continue;
                    }
                    Some((open, length)) if open == c && run >= length => {
                        if trimmed[run..].trim().is_empty() {
                            fence = None;
                            continue;
                        }
                    }
                    Some(_) => {}
                }
            }
        }
        if fence.is_some() {
            continue;
        }
        if is_raw_admonition(line) {
            problems.push(format!(
                ":{number}: admonition title not in brackets (write `:::kind[Title]`)"
            ));
        }
        if !line.trim().is_empty() {
            paragraph.push((number, line));
        }
    }
    flush(&mut paragraph, markdown, &mut problems);
    if fence.is_some() {
        problems.push(": a code fence is never closed".to_owned());
    }
    problems
}

/// Check the passive-Markdown rule on the lines of one paragraph, with the code spans of the
/// whole paragraph blanked out, and start a new paragraph.
fn flush(paragraph: &mut Vec<(usize, &str)>, markdown: bool, problems: &mut Vec<String>) {
    if markdown && !paragraph.is_empty() {
        let joined: Vec<&str> = paragraph.iter().map(|(_, line)| *line).collect();
        let blanked = without_code_spans(&joined.join("\n"));
        for ((number, _), line) in paragraph.iter().zip(blanked.split('\n')) {
            if let Some(problem) = executable(line) {
                problems.push(format!(":{number}: {problem}"));
            }
        }
    }
    paragraph.clear();
}

/// The front matter of `source` and the index of the first body line.
fn front_matter(source: &str) -> (Option<&str>, usize) {
    let Some(rest) = source.strip_prefix("---\n") else {
        return (None, 0);
    };
    match rest.find("\n---\n") {
        Some(end) => {
            let front = &rest[..end];
            (Some(front), front.lines().count() + 2)
        }
        None => (None, 0),
    }
}

/// Whether `line` matches `^:::[a-z]+ +\S`: an admonition whose title is not in brackets.
fn is_raw_admonition(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(":::") else {
        return false;
    };
    let kind = rest.bytes().take_while(u8::is_ascii_lowercase).count();
    let rest = &rest[kind..];
    let spaces = rest.bytes().take_while(|b| *b == b' ').count();
    kind > 0
        && spaces > 0
        && rest[spaces..]
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace())
}

/// `line` with the contents of every inline code span blanked out.
fn without_code_spans(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        let run = rest[start..].bytes().take_while(|b| *b == b'`').count();
        let after = &rest[start + run..];
        let closing = "`".repeat(run);
        // The span closes at the next run of exactly as many backticks.
        let mut search = 0;
        let mut end = None;
        while let Some(at) = after[search..].find(&closing) {
            let at = search + at;
            let longer = after[at + run..].starts_with('`');
            if !longer {
                end = Some(at);
                break;
            }
            search = at + run + after[at + run..].bytes().take_while(|b| *b == b'`').count();
        }
        match end {
            Some(end) => {
                // Blank the span, keeping its line breaks so lines still line up.
                let span = &rest[start..start + run * 2 + end];
                out.extend(span.chars().map(|c| if c == '\n' { '\n' } else { ' ' }));
                rest = &after[end + run..];
            }
            None => {
                out.push_str(&rest[start..start + run]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// What the unified site would refuse in a prose line, if anything.
fn executable(prose: &str) -> Option<&'static str> {
    let trimmed = prose.trim_start();
    if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
        || trimmed.starts_with("import{")
        || trimmed.starts_with("export{")
    {
        return Some("an import or export line (write it as code)");
    }
    let bytes = prose.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        let escaped = bytes[..at]
            .iter()
            .rev()
            .take_while(|b| **b == b'\\')
            .count()
            % 2
            == 1;
        if *byte == b'{' && !escaped {
            return Some(
                "a `{` outside code, which MDX reads as an expression (escape it or write it as code)",
            );
        }
        if *byte == b'<' {
            let tag = prose[at + 1..].trim_start_matches('/');
            if tag.starts_with(|c: char| c.is_ascii_uppercase()) {
                return Some(
                    "a capitalised tag outside code, which MDX reads as a component (write it as code)",
                );
            }
        }
    }
    let lower = prose.to_ascii_lowercase();
    if lower.contains(" onclick=") || lower.contains(" onload=") || lower.contains(" onerror=") {
        return Some("an event-handler attribute");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRONT: &str = "---\ntitle: T\nsidebar_position: 1\ndescription: D\n---\n\n";

    #[test]
    fn raw_admonition_titles_are_found_and_bracketed_ones_pass() {
        for raw in [":::caution Planned", ":::note  These", ":::info Decided"] {
            assert!(is_raw_admonition(raw), "{raw}");
        }
        for fine in [
            ":::caution[Planned]",
            ":::",
            ":::note",
            ":::note ",
            "::: note T",
        ] {
            assert!(!is_raw_admonition(fine), "{fine}");
        }
    }

    #[test]
    fn braces_and_capitalised_tags_in_code_pass_and_in_prose_are_found() {
        let page = format!(
            "{FRONT}Use `{{\"a\": 1}}` and `<VALUE>` and ``a ` {{ b``.\n\n```json\n{{\"x\": 1}}\n```\n\n<img src=\"/a.svg\" alt=\"x\" />\n\nEscaped \\{{ is fine.\n"
        );
        assert_eq!(page_problems(&page, false, true), Vec::<String>::new());
        let page = format!("{FRONT}A {{ in prose.\n\nA <Value> tag.\n\nimport X from 'y';\n");
        let problems = page_problems(&page, false, true);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].starts_with(":7: a `{`"), "{problems:?}");
        assert!(
            problems[1].starts_with(":9: a capitalised tag"),
            "{problems:?}"
        );
        assert!(problems[2].starts_with(":11: an import"), "{problems:?}");
    }

    #[test]
    fn front_matter_keys_are_required_and_hand_pages_need_lede_and_source() {
        assert_eq!(page_problems("# T\n", false, true), [": no front matter"]);
        assert!(page_problems(FRONT, false, true).is_empty());
        let problems = page_problems(FRONT, true, true);
        assert_eq!(
            problems,
            [
                ": front matter has no `lede`",
                ": front matter has no `source`"
            ]
        );
        let partial = "---\ntitle: T\ndescription: D\n---\n";
        assert_eq!(
            page_problems(partial, false, true),
            [": front matter has no `sidebar_position`"]
        );
    }

    #[test]
    fn a_code_span_that_wraps_a_line_is_still_code() {
        let page =
            format!("{FRONT}The limits are `{{a: 1,\nb: 2}}` and nothing else.\n\nA {{ here.\n");
        let problems = page_problems(&page, false, true);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with(":10: a `{`"), "{problems:?}");
    }

    #[test]
    fn an_unclosed_fence_is_found_and_a_longer_fence_holds_a_shorter_one() {
        let page = format!("{FRONT}````md\n```\n{{ inside }}\n```\n````\n");
        assert!(page_problems(&page, false, true).is_empty());
        let page = format!("{FRONT}```\n{{\n");
        assert_eq!(
            page_problems(&page, false, true),
            [": a code fence is never closed"]
        );
    }
}
