//! Rewrites an owner's Markdown into a page the documentation site renders as passive
//! Markdown, without re-typing it.
//!
//! The source text is kept byte for byte except for four kinds of edit, each located by the
//! parser rather than by pattern:
//!
//! - a link whose destination is not a published page loses its link and keeps its label,
//!   so no page links into history, evidence or private planning;
//! - a published link is pointed at the page that renders its target, anchor kept;
//! - outside code, `<` becomes `&lt;` and `{`/`}` are escaped, so a placeholder such as
//!   `<alias>` is shown rather than read as markup, and the unified site's passive-MDX
//!   check accepts the page;
//! - a workstation, `.local/` or `.engineering/` path is replaced by
//!   `[internal source reference]`, and one that survives refuses the page.
//!
//! Reference-style links and images are refused: a source that needs them changes first.

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::Result;

/// What replaces a private path in published text.
const INTERNAL: &str = "[internal source reference]";

/// The paths a published page must not carry.
fn private() -> Regex {
    Regex::new(r"(?:/home/|(?:\.\./)*\.local/|(?:\.\./)*\.engineering/)[^\s`<>\)\]]+")
        .expect("private path pattern")
}

/// `text` with every `<`, `{` and `}` that no backslash escapes made literal. `escaped` says
/// whether the byte before `text` is an unpaired backslash.
fn escape(text: &str, mut escaped: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' if !escaped => out.push_str("&lt;"),
            '{' | '}' if !escaped => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
        escaped = c == '\\' && !escaped;
    }
    out
}

/// Whether the byte before `at` is an unpaired backslash, which escapes the byte at `at`.
fn escaped_at(markdown: &str, at: usize) -> bool {
    markdown.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}

/// The source of `markdown` with its links resolved by `resolve` and its text made passive.
///
/// `resolve` receives a link destination as written and answers the destination to publish,
/// or `None` when the target is not published.
pub fn rewrite(markdown: &str, resolve: &dyn Fn(&str) -> Option<String>) -> Result<String> {
    let private = private();
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut code_block = false;
    // Inside an autolink whose whole text was replaced.
    let mut replaced_link = false;
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    for (event, range) in Parser::new_ext(markdown, options).into_offset_iter() {
        if replaced_link {
            if matches!(event, Event::End(TagEnd::Link)) {
                replaced_link = false;
            }
            continue;
        }
        let source = &markdown[range.clone()];
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block = true,
            Event::End(TagEnd::CodeBlock) => code_block = false,
            Event::Text(_) if code_block => {
                let text = private.replace_all(source, INTERNAL);
                if text != source {
                    edits.push((range, text.into_owned()));
                }
            }
            Event::Text(_) | Event::Html(_) | Event::InlineHtml(_) => {
                let text = escape(
                    &private.replace_all(source, INTERNAL),
                    escaped_at(markdown, range.start),
                );
                if text != source {
                    edits.push((range, text));
                }
            }
            Event::Code(_) => {
                let text = private.replace_all(source, INTERNAL);
                if text != source {
                    edits.push((range, text.into_owned()));
                }
            }
            Event::Start(Tag::Image { .. }) => {
                return Err("an image cannot be published; link to it instead".to_owned());
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => match link_type {
                LinkType::Inline => {
                    let close = source
                        .rfind("](")
                        .filter(|_| source.ends_with(')'))
                        .ok_or_else(|| format!("cannot locate the destination of {source}"))?;
                    let destination = range.start + close + 2..range.end - 1;
                    match resolve(&dest_url) {
                        Some(target) => edits.push((destination, target)),
                        None => {
                            edits.push((range.start..range.start + 1, String::new()));
                            edits.push((range.start + close..range.end, String::new()));
                        }
                    }
                }
                LinkType::Autolink | LinkType::Email => {
                    if resolve(&dest_url).is_none() {
                        edits.push((
                            range,
                            escape(&private.replace_all(&dest_url, INTERNAL), false),
                        ));
                        replaced_link = true;
                    }
                }
                _ => {
                    return Err(format!(
                        "reference-style link {source:?} cannot be resolved here; write it inline"
                    ));
                }
            },
            _ => {}
        }
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(markdown.len());
    let mut at = 0;
    for (range, text) in edits {
        if range.start < at {
            return Err(format!("overlapping edits at byte {}", range.start));
        }
        out.push_str(&markdown[at..range.start]);
        out.push_str(&text);
        at = range.end;
    }
    out.push_str(&markdown[at..]);
    if let Some(found) = private.find(&out) {
        return Err(format!(
            "a private path remains after rewriting: {}",
            found.as_str()
        ));
    }
    Ok(out)
}

/// `markdown` without its source-history sections: a level-two section titled
/// "Old evidence and disposition" runs to the next level-two heading. Every other line stays.
pub fn without_history(markdown: &str) -> String {
    let mut kept = String::with_capacity(markdown.len());
    let mut history = false;
    for line in markdown.lines() {
        if line.starts_with("## ") {
            history = line.contains("Old evidence and disposition");
        }
        if !history {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Built at run time so no home-directory path is written in the source.
    const HOME: &str = "/home";

    fn published(destination: &str) -> Option<String> {
        match destination {
            "../a.md" => Some("../a.md".to_owned()),
            "../b/semantics.md#x" => Some("b.md#x".to_owned()),
            "#local" => Some("#local".to_owned()),
            d if d.starts_with("https://") => Some(d.to_owned()),
            _ => None,
        }
    }

    #[test]
    fn published_links_are_pointed_at_their_pages_and_others_keep_only_their_label() {
        let source = "See [A](../a.md), [B](../b/semantics.md#x), [here](#local), \
                      [web](https://example.org/x) and [plan](../../.engineering/plan.md).\n";
        assert_eq!(
            rewrite(source, &published).unwrap(),
            "See [A](../a.md), [B](b.md#x), [here](#local), [web](https://example.org/x) and plan.\n"
        );
    }

    #[test]
    fn placeholders_and_braces_outside_code_are_made_literal_and_code_is_kept() {
        let source =
            "Use <alias> and {x}, `<alias> {x}` and \\{ kept.\n\n```json\n{\"a\": \"<b>\"}\n```\n";
        assert_eq!(
            rewrite(source, &published).unwrap(),
            "Use &lt;alias> and \\{x\\}, `<alias> {x}` and \\{ kept.\n\n```json\n{\"a\": \"<b>\"}\n```\n"
        );
    }

    #[test]
    fn private_paths_are_replaced_in_text_and_code() {
        let source = format!("Evidence in .local/tmp/x.json and `{HOME}/someone/y`.\n");
        let out = rewrite(&source, &published).unwrap();
        assert_eq!(
            out,
            "Evidence in [internal source reference] and `[internal source reference]`.\n"
        );
    }

    #[test]
    fn a_link_label_keeps_its_escaping_when_the_link_is_dropped() {
        let source = "[the <owner> notes](../notes.md)\n";
        assert_eq!(
            rewrite(source, &published).unwrap(),
            "the &lt;owner> notes\n"
        );
    }

    #[test]
    fn non_web_autolinks_become_text() {
        assert_eq!(
            rewrite("Write to <someone@example.org>.\n", &published).unwrap(),
            "Write to someone@example.org.\n"
        );
        assert_eq!(
            rewrite("See <https://example.org>.\n", &published).unwrap(),
            "See <https://example.org>.\n"
        );
    }

    #[test]
    fn imported_markdown_cannot_execute_html_or_script_links() {
        let out = rewrite(
            &format!("# Rules\n<script>alert(1)</script>\n\n[x](javascript:alert)\n\n`{HOME}/example/private`\n"),
            &published,
        )
        .unwrap();
        assert!(!out.contains("<script>"), "{out}");
        assert!(!out.contains("javascript:"), "{out}");
        assert!(!out.contains("/home/"), "{out}");
        assert!(out.contains("&lt;script>alert(1)&lt;/script>"), "{out}");
    }

    #[test]
    fn reference_links_and_images_are_refused() {
        assert!(rewrite("[a][b]\n\n[b]: ../a.md\n", &published).is_err());
        assert!(rewrite("![x](../a.png)\n", &published).is_err());
    }

    #[test]
    fn history_sections_are_left_out_and_the_rest_is_kept() {
        let source = "# T\n\n## Rules\n\nkeep\n\n## Old evidence and disposition\n\ndrop\n\n## Next\n\nkeep too\n";
        assert_eq!(
            without_history(source),
            "# T\n\n## Rules\n\nkeep\n\n## Next\n\nkeep too\n"
        );
    }
}
