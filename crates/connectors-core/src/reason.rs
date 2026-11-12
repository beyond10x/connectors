//! The bounded, redacted reason an upstream gives for refusing a request.
//!
//! Conservative by construction: a reason is admitted only from one place,
//! the first of the top-level `message`, `error_description` and `error`
//! members of a JSON object body that holds a string, never from headers or
//! any other body text. It is withheld whole, never partly masked, when it
//! looks like it could carry a secret or an email address, and
//! [`carries_credential`] / [`carries_value`] let the holder of the request's
//! credential withhold it again against that material. What is admitted is at
//! most [`LIMIT`] bytes of UTF-8 with no control or invisible character.
use serde_json::Value;

/// The most bytes an admitted reason holds.
pub const LIMIT: usize = 256;
/// The members a reason is read from, in order of preference.
const FIELDS: [&str; 3] = ["message", "error_description", "error"];
/// How much of a reason the secret checks read. A run that reaches into the
/// first [`LIMIT`] bytes is seen whole unless it is longer than this.
const SCANNED: usize = 4096;
/// Case-insensitive pieces of header- or credential-like content.
const MARKERS: [&str; 22] = [
    "authorization",
    "bearer",
    "basic ",
    "password",
    "passwd",
    "secret",
    "cookie",
    "api_key",
    "apikey",
    "api-key",
    "private_key",
    "private key",
    "-----begin",
    "token=",
    "token:",
    "access_token",
    "refresh_token",
    "id_token",
    "client_assertion",
    "signature=",
    "sig=",
    "session=",
];
/// The shortest credential piece [`carries_credential`] matches, in ASCII
/// letters and digits; a shorter value is matched whole.
const WINDOW: usize = 8;
/// The length from which a space-separated word holding a digit, or mixing
/// letter case often, reads as a token.
const WORD: usize = 16;

/// The admitted reason of a refusal body, if it has one; see [`admit`].
pub fn from_body(body: &[u8]) -> Option<String> {
    let value: Value = crate::read_json(body).ok()?;
    let object = value.as_object()?;
    let text = FIELDS
        .iter()
        .find_map(|field| object.get(*field)?.as_str())?;
    admit(text)
}

/// The admitted form of an upstream's reason text, or `None` when it is
/// withheld. Whitespace runs read as one space and the ends are trimmed. The
/// text is withheld when, within its first 4096 bytes, it holds a control or
/// invisible character, an `@` (an email address), a header- or
/// credential-like marker, or a token-like run or word. A longer text is cut to [`LIMIT`] bytes on a character
/// boundary and then back to the last whole space-separated word; one whose
/// first word does not fit is withheld.
pub fn admit(text: &str) -> Option<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let scanned = &text[..floor(&text, SCANNED)];
    if scanned.is_empty() || scanned.chars().any(hidden) || secret_shaped(scanned) {
        return None;
    }
    if text.len() <= LIMIT {
        return Some(text);
    }
    let end = floor(&text, LIMIT);
    // The cut falls inside a word unless a space follows it; no part of a
    // word is kept, and a first word longer than the limit withholds it all.
    let kept = if text[end..].starts_with(' ') {
        &text[..end]
    } else {
        &text[..text[..end].rfind(' ')?]
    };
    let kept = kept.trim_end();
    (!kept.is_empty()).then(|| kept.to_owned())
}

/// The most bytes a domain refusal holds: one account domain and eight admitted
/// domains of 253 bytes each, with the words around them.
pub const DOMAIN_LIMIT: usize = 2560;
/// The most company domains an instance admits (`allowed_domains`).
pub const COMPANY_DOMAINS: usize = 8;
const DOMAIN_ACCOUNT: &str = "the account's domain ";
const DOMAIN_NOT_ADMITTED: &str = " is not an admitted domain";
const DOMAIN_SEPARATOR: &str = "; admitted: ";
const DOMAIN_UNNAMED: &str = "the account's domain is not an admitted domain";
const DOMAIN_ABSENT: &str =
    "the account has no hd claim, so it is not a Workspace account of an admitted domain";

/// Whether `text` is a company domain: a lower-case ASCII bare host name of at
/// most 253 bytes and at least two dot-separated labels, each of 1 to 63 bytes
/// of letters, digits and hyphens that neither starts nor ends with a hyphen.
/// A scheme, a path, a wildcard, an `@` or a space is no host name.
pub fn company_domain(text: &str) -> bool {
    let label = |label: &str| {
        (1..=63).contains(&label.len())
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    text.len() <= 253 && text.split('.').count() >= 2 && text.split('.').all(label)
}

/// Whether `domains` is a usable list of the company domains an instance
/// admits (`allowed_domains`, matched against the `hd` claim of a Google
/// account's `id_token`): 1 to [`COMPANY_DOMAINS`] distinct [`company_domain`]s.
/// The host's configuration, the provider and the host's check of a
/// [`domain_refusal`] all apply this one rule.
pub fn company_domains_admitted(domains: &[String]) -> bool {
    (1..=COMPANY_DOMAINS).contains(&domains.len())
        && domains
            .iter()
            .enumerate()
            .all(|(index, domain)| company_domain(domain) && !domains[..index].contains(domain))
}

/// The refusal of an account whose Workspace domain is not one of `admitted`
/// (a list [`company_domains_admitted`] accepts), in the one form
/// [`is_domain_refusal`] accepts. `account` is the domain the account's
/// identity assertion named, `None` when it named none. It is shown only when
/// it already is a [`company_domain`], lower-case as given; any other claim
/// takes the unnamed form, which says only that its domain is not admitted.
/// `None` when `admitted` is not a usable list.
///
/// [`admit`] withholds a word of 16 bytes or more that holds a digit, a run of
/// 32 or more, and any word that reads like a credential marker, so it cannot
/// carry a domain such as `branch2026.example.org`. A domain is a name the
/// instance's own configuration or the identity provider's assertion states,
/// never a credential, so this form is checked by its exact shape instead:
/// domains only, in fixed words.
pub fn domain_refusal(account: Option<&str>, admitted: &[String]) -> Option<String> {
    if !company_domains_admitted(admitted) {
        return None;
    }
    let head = match account {
        Some(domain) if company_domain(domain) => {
            format!("{DOMAIN_ACCOUNT}{domain}{DOMAIN_NOT_ADMITTED}")
        }
        Some(_) => DOMAIN_UNNAMED.to_owned(),
        None => DOMAIN_ABSENT.to_owned(),
    };
    Some(format!("{head}{DOMAIN_SEPARATOR}{}", admitted.join(", ")))
}

/// [`carries_credential`] for a [`domain_refusal`]: the one member of the
/// document that is a public name, the OAuth `client_id` of a top-level JSON
/// object, is not matched, because a domain the refusal names (a company whose
/// client id carries its project's domain) may share eight bytes with it. Every
/// other value, a client secret and a refresh token included whatever its
/// shape, is matched as before.
pub fn domain_refusal_carries_credential(reason: &str, document: &[u8]) -> bool {
    let parsed = serde_json::from_slice::<Value>(document);
    let mut values = Vec::new();
    let mut named = false;
    if let Ok(value) = &parsed {
        strings(value, &mut values);
        if let Value::Object(members) = value
            && let Some(Value::String(client_id)) = members.get("client_id")
        {
            let at = values
                .iter()
                .position(|value| *value == client_id.as_bytes());
            named = at.is_some();
            if let Some(at) = at {
                values.remove(at);
            }
        }
    }
    if values.is_empty() && !named {
        values.push(document);
    }
    values.into_iter().any(|value| carries_value(reason, value))
}

/// Whether `text` is exactly a [`domain_refusal`]: a host that does not trust
/// the child to have built it checks it with this, in place of [`admit`].
pub fn is_domain_refusal(text: &str) -> bool {
    if text.len() > DOMAIN_LIMIT {
        return false;
    }
    let Some((head, list)) = text.split_once(DOMAIN_SEPARATOR) else {
        return false;
    };
    let named = head
        .strip_prefix(DOMAIN_ACCOUNT)
        .and_then(|rest| rest.strip_suffix(DOMAIN_NOT_ADMITTED))
        .is_some_and(company_domain);
    let domains: Vec<String> = list.split(", ").map(str::to_owned).collect();
    (named || head == DOMAIN_UNNAMED || head == DOMAIN_ABSENT) && company_domains_admitted(&domains)
}

/// Whether `reason` holds any piece of the credential `document`: of every
/// string value in it when it is JSON with at least one string, otherwise of
/// the whole document; see [`carries_value`].
pub fn carries_credential(reason: &str, document: &[u8]) -> bool {
    let parsed = serde_json::from_slice::<Value>(document);
    let mut values = Vec::new();
    if let Ok(value) = &parsed {
        strings(value, &mut values);
    }
    if values.is_empty() {
        values.push(document);
    }
    values.into_iter().any(|value| carries_value(reason, value))
}

/// Whether `reason` holds any piece of one credential `value`, such as a
/// header value derived from the document: any eight consecutive bytes of it
/// (all of it, when shorter) without regard to ASCII case, or any eight
/// consecutive of its ASCII letters and digits among the reason's, lowercased,
/// so a value echoed with its separators changed still matches. The reason
/// is also read with its `%XX` escapes decoded.
pub fn carries_value(reason: &str, value: &[u8]) -> bool {
    if value.is_empty() {
        return false;
    }
    let decoded = percent_decoded(reason.as_bytes());
    let value_alphanumeric = alphanumeric(value);
    [reason.as_bytes(), decoded.as_slice()]
        .into_iter()
        .any(|reason_bytes| {
            windowed(reason_bytes, value, |a, b| a.eq_ignore_ascii_case(b))
                || (!value_alphanumeric.is_empty()
                    && windowed(&alphanumeric(reason_bytes), &value_alphanumeric, |a, b| {
                        a == b
                    }))
        })
}

/// Whether any [`WINDOW`]-long piece of `value` (all of it, when shorter)
/// occurs in `haystack` under `same`.
fn windowed(haystack: &[u8], value: &[u8], same: impl Fn(&[u8], &[u8]) -> bool) -> bool {
    let width = value.len().min(WINDOW);
    value.windows(width).any(|piece| {
        haystack
            .windows(width)
            .any(|candidate| same(candidate, piece))
    })
}

/// `bytes` with every `%XX` hexadecimal escape replaced by its byte.
fn percent_decoded(bytes: &[u8]) -> Vec<u8> {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(high), Some(low)) = (
                bytes.get(i + 1).copied().and_then(hex),
                bytes.get(i + 2).copied().and_then(hex),
            )
        {
            out.push(high << 4 | low);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// The ASCII letters and digits of `bytes`, lowercased.
fn alphanumeric(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(u8::to_ascii_lowercase)
        .collect()
}

fn strings<'a>(value: &'a Value, into: &mut Vec<&'a [u8]>) {
    match value {
        Value::String(text) => into.push(text.as_bytes()),
        Value::Array(values) => values.iter().for_each(|value| strings(value, into)),
        Value::Object(members) => members.values().for_each(|value| strings(value, into)),
        _ => {}
    }
}

/// The largest character boundary of `text` at or below `limit`.
fn floor(text: &str, limit: usize) -> usize {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// A control character, any Unicode format (Cf) character — the TAG block
/// included — or another character that renders as nothing and can carry
/// hidden data: variation selectors, the combining grapheme joiner and the
/// Hangul fillers.
fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            // General category Cf, Unicode 16.0.
            '\u{00ad}'
                | '\u{0600}'..='\u{0605}'
                | '\u{061c}'
                | '\u{06dd}'
                | '\u{070f}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08e2}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{110bd}'
                | '\u{110cd}'
                | '\u{13430}'..='\u{1343f}'
                | '\u{1bca0}'..='\u{1bca3}'
                | '\u{1d173}'..='\u{1d17a}'
                | '\u{e0000}'..='\u{e007f}'
                // Invisible, though not Cf.
                | '\u{034f}'
                | '\u{115f}'..='\u{1160}'
                | '\u{3164}'
                | '\u{ffa0}'
                | '\u{fe00}'..='\u{fe0f}'
                | '\u{e0100}'..='\u{e01ef}'
        )
}

/// An `@` (an email address), a marker anywhere, a token-like run or a
/// token-like word. A run is a maximal run of ASCII letters, digits and
/// `-_+/=.~`; it is token-like at 32 bytes or more, or at 20 or more holding
/// both a letter and a digit. A word is a space-separated word; it is
/// token-like at 16 bytes or more when it holds a digit, or when its ASCII
/// letters change case at least once per four of them.
fn secret_shaped(text: &str) -> bool {
    if text.contains(['@', '\u{ff20}', '\u{fe6b}']) {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    if MARKERS.iter().any(|marker| lower.contains(marker)) {
        return true;
    }
    let run = text
        .split(|c: char| !(c.is_ascii_alphanumeric() || "-_+/=.~".contains(c)))
        .any(|run| {
            run.len() >= 32
                || (run.len() >= 20
                    && run.bytes().any(|b| b.is_ascii_alphabetic())
                    && run.bytes().any(|b| b.is_ascii_digit()))
        });
    run || text.split(' ').any(|word| {
        if word.len() < WORD {
            return false;
        }
        let letters: Vec<u8> = word.bytes().filter(u8::is_ascii_alphabetic).collect();
        let changes = letters
            .windows(2)
            .filter(|pair| pair[0].is_ascii_uppercase() != pair[1].is_ascii_uppercase())
            .count();
        word.bytes().any(|b| b.is_ascii_digit())
            || (!letters.is_empty() && changes * 4 >= letters.len())
    })
}
