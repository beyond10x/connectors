//! The query template of a time-position feed, parsed once for both readers of it: the catalog
//! provider's feed engine, which refuses a malformed template when an instance loads, and the
//! definition reader, which refuses the same template when the definition is lowered. One parser,
//! so the two cannot disagree on what a template is.

/// The longest query template.
pub const TEMPLATE_BYTES: usize = 1024;

/// One piece of a parsed query template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Text(String),
    Container,
    Since,
}

/// `text`, the template `name`, as text and slots. Refused, with the sentence the engine's
/// refusal carries: a template longer than [`TEMPLATE_BYTES`], a control character, a brace
/// outside a slot, a slot other than `{container}` and `{since}`, a slot beside a `"`, `'` or `\`
/// or inside a literal the template opens with `"` or `'` (where a value would be read as part of
/// that literal, not as its own), a literal left open and a `\` outside a literal. How many times
/// each slot appears is the caller's rule: `first` and `resume` differ.
pub fn parse(name: &str, text: &str) -> Result<Vec<Part>, String> {
    if text.len() > TEMPLATE_BYTES {
        return Err(format!(
            "feed `{name}` is longer than {TEMPLATE_BYTES} bytes"
        ));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("feed `{name}` holds a control character"));
    }
    let stray = || format!("feed `{name}` has a stray brace");
    let mut parts = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let at = rest.find(['{', '}']).unwrap_or(rest.len());
        if at > 0 {
            parts.push(Part::Text(rest[..at].to_owned()));
        }
        rest = &rest[at..];
        if rest.is_empty() {
            break;
        }
        if rest.starts_with('}') {
            return Err(stray());
        }
        let end = rest.find('}').ok_or_else(stray)?;
        let slot = &rest[1..end];
        if slot.contains('{') {
            return Err(stray());
        }
        parts.push(match slot {
            "container" => Part::Container,
            "since" => Part::Since,
            other => {
                return Err(format!("feed `{name}` names an unknown slot `{{{other}}}`"));
            }
        });
        rest = &rest[end + 1..];
    }
    let quoting = |c: char| matches!(c, '"' | '\'' | '\\');
    for (index, part) in parts.iter().enumerate() {
        if matches!(part, Part::Text(_)) {
            continue;
        }
        let before = index
            .checked_sub(1)
            .and_then(|i| parts.get(i))
            .and_then(|p| match p {
                Part::Text(text) => text.chars().next_back(),
                _ => None,
            });
        let after = parts.get(index + 1).and_then(|p| match p {
            Part::Text(text) => text.chars().next(),
            _ => None,
        });
        if before.is_some_and(quoting) || after.is_some_and(quoting) {
            return Err(format!(
                "feed `{name}` puts a slot beside a quote or backslash; the engine quotes each value"
            ));
        }
    }
    // A slot inside a literal the template opens would let its value close that literal.
    let mut open: Option<char> = None;
    let mut escaped = false;
    for part in &parts {
        let Part::Text(text) = part else {
            if open.is_some() {
                return Err(format!(
                    "feed `{name}` puts a slot inside a literal; the engine quotes each value"
                ));
            }
            continue;
        };
        for c in text.chars() {
            match open {
                Some(_) if escaped => escaped = false,
                Some(_) if c == '\\' => escaped = true,
                Some(quote) if c == quote => open = None,
                Some(_) => {}
                None if matches!(c, '"' | '\'') => open = Some(c),
                // Outside a literal a provider may or may not read `\` as an escape, so where a
                // literal starts would be the provider's guess, not the template's.
                None if c == '\\' => {
                    return Err(format!("feed `{name}` has a backslash outside a literal"));
                }
                None => {}
            }
        }
    }
    if open.is_some() {
        return Err(format!("feed `{name}` leaves a literal open"));
    }
    Ok(parts)
}
