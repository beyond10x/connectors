//! Exact qualified names and resource URIs from server/v1alpha1/projection.md.
//! A decoded name selects no authority; the host must still admit its exact target.

const NAME_LIMIT: usize = 128;
const RESOURCE_PREFIX: &str = "connectors-mcp:///";

/// Encode exact UTF-8 identity components without normalization or truncation.
#[must_use]
pub fn encode_name(parts: [&str; 3]) -> Option<String> {
    let mut length = 5_usize; // c1_ plus the two separators.
    for part in parts {
        if part.is_empty() {
            return None;
        }
        length = length.checked_add(part.len().checked_mul(2)?)?;
    }
    if length > NAME_LIMIT {
        return None;
    }
    Some(format!(
        "c1_{}_{}_{}",
        hex(parts[0]),
        hex(parts[1]),
        hex(parts[2])
    ))
}

/// Refuse noncanonical names rather than accepting aliases for a target.
#[must_use]
pub fn decode_name(name: &str) -> Option<[String; 3]> {
    if name.len() > NAME_LIMIT {
        return None;
    }
    let mut parts = name.strip_prefix("c1_")?.split('_');
    let decoded = [
        unhex(parts.next()?)?,
        unhex(parts.next()?)?,
        unhex(parts.next()?)?,
    ];
    if parts.next().is_some()
        || encode_name(decoded.each_ref().map(String::as_str)).as_deref() != Some(name)
    {
        return None;
    }
    Some(decoded)
}

/// Bind a configured read projection to its exact nonempty descriptor revision.
#[must_use]
pub fn resource_uri(parts: [&str; 3], revision: &str) -> Option<String> {
    if revision.is_empty() {
        return None;
    }
    Some(format!(
        "{RESOURCE_PREFIX}{}?revision={}",
        encode_name(parts)?,
        hex(revision)
    ))
}

/// Parse the selected URI grammar without URL rewriting, percent decoding or fallback.
#[must_use]
pub fn decode_resource_uri(uri: &str) -> Option<([String; 3], String)> {
    let (name, revision) = uri
        .strip_prefix(RESOURCE_PREFIX)?
        .split_once("?revision=")?;
    Some((decode_name(name)?, unhex(revision)?))
}

fn hex(value: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    value
        .bytes()
        .flat_map(|byte| {
            [
                DIGITS[(byte >> 4) as usize] as char,
                DIGITS[(byte & 15) as usize] as char,
            ]
        })
        .collect()
}

fn unhex(value: &str) -> Option<String> {
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return None;
    }
    let digit = |byte| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    let bytes: Option<Vec<u8>> = value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Some(digit(pair[0])? * 16 + digit(pair[1])?))
        .collect();
    String::from_utf8(bytes?).ok()
}
