//! Redact personal data out of a vendor document before it is pinned, and check that a
//! pinned redacted document still holds none.
//!
//! A vendor's published specification can carry what looks like a real person's email
//! address or phone number, or a credential, in its examples. Those bytes may not enter
//! version control,
//! so such a document is pinned redacted: `connectors-build redact` writes the redacted
//! copy, and its `*source-hashes.json` record names the rule, the upstream digest and the
//! redacted digest. The gate re-reads every record that names a rule and refuses a file
//! whose digest drifted or which the rule would still change.
//!
//! Rule `upstream-redaction/2`, applied to the whole file (`/1` was the first two
//! items alone):
//!
//! - every credential value becomes `example-redacted-token`: the value of a key,
//!   quoted or not, whose name ends in `token`, `secret`, `password`, `passwd`,
//!   `api_key`, `apikey`, `access_key`, `private_key` or `credential`, when that value
//!   is one scalar without whitespace (a nested mapping, a block, `null` or a boolean
//!   is left alone); and every JSON Web Token (`eyJ….eyJ…`) wherever it stands;
//! - every email address whose domain is not reserved for documentation (`example.com`,
//!   `example.net`, `example.org` and their subdomains, or a name under the `example`,
//!   `test`, `invalid` or `localhost` top-level domains) becomes `user@example.com`;
//! - every phone number written as `+` and 8 to 15 digits, or as
//!   `+<country> <3>-<3>-<4>`, becomes `+15555550100` or `+1 555-555-0100`, unless it
//!   is a North American number in the unassigned area code 555 or the fictional
//!   exchange 555-01XX.
use super::Result;
use regex::{Captures, Regex};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// The rule's name as a manifest records it.
pub const RULE: &str = "upstream-redaction/2";
pub const CREDENTIAL: &str = "example-redacted-token";
pub const EMAIL: &str = "user@example.com";
pub const PHONE: &str = "+15555550100";
pub const PHONE_FORMATTED: &str = "+1 555-555-0100";

static EMAIL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@((?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,})").expect("email pattern")
});
static PHONE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\+[0-9]{8,15}|\+[0-9]{1,3} [0-9]{3}-[0-9]{3}-[0-9]{4}").expect("phone pattern")
});

/// A key, optionally quoted, naming a credential, then `:` and its value. The key's
/// own quotes are compared in code; the pattern language has no back references.
static CREDENTIAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(["']?)([A-Za-z0-9_-]*(?:token|secret|password|passwd|api_?key|access_?key|private_?key|credentials?))(["']?)([ \t]*:[ \t]*)([^\r\n]*)"#,
    )
    .expect("credential pattern")
});
static JWT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}(?:\.[A-Za-z0-9_-]+)?")
        .expect("token pattern")
});

/// What one redaction pass replaced.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub credentials: usize,
    pub emails: usize,
    pub phones: usize,
}

/// The credential value a key carries, and where it stands in the text after the
/// colon: the span inside its quotes, or the whole unquoted scalar. `None` for a
/// value that is not one secret-shaped scalar.
fn credential_span(value: &str) -> Option<(usize, usize)> {
    let (start, end) = match value.chars().next()? {
        quote @ ('"' | '\'') => (1, 1 + value[1..].find(quote)?),
        _ => {
            let trimmed = value.trim_end();
            // An unquoted JSON value inside a block ends at a comma or a brace.
            let end = trimmed.find([',', '}', ']']).unwrap_or(trimmed.len());
            (0, end)
        }
    };
    let inner = &value[start..end];
    let structural = inner.is_empty()
        || inner.starts_with(['{', '[', '|', '>', '&', '*', '#', '!'])
        || inner.chars().any(char::is_whitespace)
        || ["null", "~", "true", "false"].contains(&inner.to_ascii_lowercase().as_str())
        || inner == CREDENTIAL;
    (!structural).then_some((start, end))
}

fn redact_credentials(text: &str, counts: &mut Counts) -> String {
    // The value group runs to the end of the line, so what follows the value — another
    // key on a one-line JSON object — is read again on its own.
    let text = CREDENTIAL_PATTERN.replace_all(text, |captures: &Captures| {
        let whole = &captures[0];
        let value = &captures[5];
        let prefix = &whole[..whole.len() - value.len()];
        let span = if captures[1] == captures[3] {
            credential_span(value)
        } else {
            None
        };
        match span {
            Some((start, end)) => {
                counts.credentials += 1;
                let rest = redact_credentials(&value[end..], &mut *counts);
                format!("{prefix}{}{CREDENTIAL}{rest}", &value[..start])
            }
            None => format!("{prefix}{}", redact_credentials(value, &mut *counts)),
        }
    });
    JWT_PATTERN
        .replace_all(&text, |_: &Captures| {
            counts.credentials += 1;
            CREDENTIAL.to_owned()
        })
        .into_owned()
}

fn reserved_domain(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    ["example.com", "example.net", "example.org"]
        .iter()
        .any(|reserved| domain == *reserved || domain.ends_with(&format!(".{reserved}")))
        || ["example", "test", "invalid", "localhost"]
            .iter()
            .any(|tld| domain.ends_with(&format!(".{tld}")))
}

fn fictional_phone(digits: &str) -> bool {
    // North American numbering: area code 555 is unassigned, and 555-0100 to
    // 555-0199 is reserved for fiction.
    match digits.strip_prefix('1') {
        Some(national) if national.len() == 10 => {
            national.starts_with("555") || &national[3..8] == "55501"
        }
        _ => false,
    }
}

/// Apply the rule to a whole document.
pub fn redact(text: &str) -> (String, Counts) {
    let mut counts = Counts::default();
    let text = redact_credentials(text, &mut counts);
    let text = EMAIL_PATTERN.replace_all(&text, |captures: &Captures| {
        if reserved_domain(&captures[1]) {
            captures[0].to_owned()
        } else {
            counts.emails += 1;
            EMAIL.to_owned()
        }
    });
    // A number stands alone: no letter, digit or `+` directly before it and no letter
    // or digit directly after, so a longer digit run or an identifier is not one.
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for found in PHONE_PATTERN.find_iter(&text) {
        let before = found.start().checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(found.end()).copied();
        let alone = !before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'+')
            && !after.is_some_and(|b| b.is_ascii_alphanumeric());
        let number = found.as_str();
        let digits: String = number.chars().filter(char::is_ascii_digit).collect();
        if !alone || fictional_phone(&digits) {
            continue;
        }
        counts.phones += 1;
        out.push_str(&text[copied..found.start()]);
        out.push_str(if number.contains(' ') {
            PHONE_FORMATTED
        } else {
            PHONE
        });
        copied = found.end();
    }
    out.push_str(&text[copied..]);
    (out, counts)
}

/// `connectors-build redact`: write the redacted copy of `input` to `output`.
pub fn write(input: &Path, output: &Path) -> Result<()> {
    let original = fs::read(input)?;
    let text =
        std::str::from_utf8(&original).map_err(|_| format!("{}: is not UTF-8", input.display()))?;
    let (redacted, counts) = redact(text);
    fs::write(output, redacted.as_bytes())?;
    println!(
        "{RULE}: {} -> {}; upstream sha256 {} ({} bytes), redacted sha256 {} ({} bytes); \
         {} credential(s), {} email address(es) and {} phone number(s) replaced",
        input.display(),
        output.display(),
        super::source_hashes::digest(&original)?,
        original.len(),
        super::source_hashes::digest(redacted.as_bytes())?,
        redacted.len(),
        counts.credentials,
        counts.emails,
        counts.phones
    );
    Ok(())
}

#[derive(Deserialize)]
struct Record {
    file: String,
    sha256: String,
    bytes: u64,
    #[serde(default)]
    redaction: Option<String>,
    #[serde(default)]
    upstream_sha256: Option<String>,
}

fn manifests(path: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for entry in entries {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if fs::symlink_metadata(&entry)?.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            manifests(&entry, output)?;
        } else if name.ends_with("source-hashes.json") {
            output.push(entry);
        }
    }
    Ok(())
}

/// The gate's check. Returns the number of redacted sources checked, so a caller can
/// tell a check that ran from a walk that selected nothing.
pub fn run(root: &Path) -> Result<usize> {
    let mut paths = Vec::new();
    manifests(&root.join("adapters"), &mut paths)?;
    let mut checked = 0usize;
    for manifest in &paths {
        let directory = manifest.parent().ok_or("manifest has no directory")?;
        let Ok(records) = serde_json::from_slice::<Vec<Record>>(&fs::read(manifest)?) else {
            continue;
        };
        for record in records {
            let Some(rule) = &record.redaction else {
                continue;
            };
            let label = format!("{}: {}", manifest.display(), record.file);
            if rule != RULE {
                return Err(format!("{label}: names redaction `{rule}`, not `{RULE}`").into());
            }
            if record.upstream_sha256.is_none() {
                return Err(format!("{label}: a redacted record names no upstream_sha256").into());
            }
            if Path::new(&record.file).is_absolute() || record.file.contains("..") {
                return Err(format!("{label}: leaves its directory").into());
            }
            let bytes = fs::read(directory.join(&record.file))?;
            if bytes.len() as u64 != record.bytes
                || super::source_hashes::digest(&bytes)? != record.sha256
            {
                return Err(format!("{label}: differs from its recorded digest or length").into());
            }
            let text = std::str::from_utf8(&bytes).map_err(|_| format!("{label}: is not UTF-8"))?;
            let (_, counts) = redact(text);
            if counts != Counts::default() {
                return Err(format!(
                    "{label}: {RULE} would still replace {} credential(s), {} email \
                     address(es) and {} phone number(s)",
                    counts.credentials, counts.emails, counts.phones
                )
                .into());
            }
            checked += 1;
        }
    }
    println!(
        "gate: {checked} redacted upstream source(s) match their recorded digests and hold \
         nothing {RULE} replaces; exit=0"
    );
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_looking_addresses_are_replaced_and_reserved_ones_kept() {
        let (text, counts) = redact(
            "a: someone+tag@vendor.com\nb: agent@example.com\nc: x@help.example.org\n\
             d: `{local-part}@{account}.vendor.com`\ne: y@host.test\nf: z@shop.co.uk\n",
        );
        assert_eq!(
            text,
            "a: user@example.com\nb: agent@example.com\nc: x@help.example.org\n\
             d: `{local-part}@{account}.vendor.com`\ne: y@host.test\nf: user@example.com\n"
        );
        assert_eq!(
            counts,
            Counts {
                credentials: 0,
                emails: 2,
                phones: 0
            }
        );
    }

    #[test]
    fn real_looking_phone_numbers_are_replaced_and_fictional_ones_kept() {
        let (text, counts) = redact(
            "phone: \"+16617480240\"\nto: \"+3211223213\"\nkept: \"+15551234567\"\n\
             fiction: +12125550142\nformatted: +44 207-946-0000\nkept: +1 555-123-4567\n\
             time: +0000 2012\nid: 4928-9046\nadjacent: +16617480240,+18917389123\n",
        );
        assert_eq!(
            text,
            "phone: \"+15555550100\"\nto: \"+15555550100\"\nkept: \"+15551234567\"\n\
             fiction: +12125550142\nformatted: +1 555-555-0100\nkept: +1 555-123-4567\n\
             time: +0000 2012\nid: 4928-9046\nadjacent: +15555550100,+15555550100\n"
        );
        assert_eq!(
            counts,
            Counts {
                credentials: 0,
                emails: 0,
                phones: 5
            }
        );
    }

    #[test]
    fn credential_values_are_replaced_and_structure_kept() {
        let (text, counts) = redact(
            "  access_token: fixture-one\n\
             - token: 'fixture-two'\n\
             authenticity_token: fixture+three/four\n\
             {\"refresh_token\": \"fixture-five\", \"client_secret\": \"fixture-six\", \"n\": 1}\n\
             password: \"123456\"\n\
             token:\n    type: string\n\
             refresh_token: null\n\
             api_token: true\n\
             secret: The shared secret for the target\n\
             token_type: bearer\n\
             header: Bearer eyJfixturefixture.eyJfixturefixture.sig\n",
        );
        assert_eq!(
            text,
            "  access_token: example-redacted-token\n\
             - token: 'example-redacted-token'\n\
             authenticity_token: example-redacted-token\n\
             {\"refresh_token\": \"example-redacted-token\", \"client_secret\": \
             \"example-redacted-token\", \"n\": 1}\n\
             password: \"example-redacted-token\"\n\
             token:\n    type: string\n\
             refresh_token: null\n\
             api_token: true\n\
             secret: The shared secret for the target\n\
             token_type: bearer\n\
             header: Bearer example-redacted-token\n"
        );
        assert_eq!(
            counts,
            Counts {
                credentials: 7,
                emails: 0,
                phones: 0
            }
        );
    }

    #[test]
    fn a_redacted_document_is_a_fixed_point() {
        let (once, _) = redact(
            "a: p@vendor.com +16617480240 +44 207-946-0000\naccess_token: fixture-seven\n\
             {\"token\": \"x1\"}\n",
        );
        let (twice, counts) = redact(&once);
        assert_eq!(once, twice);
        assert_eq!(counts, Counts::default());
    }

    #[test]
    fn the_gate_refuses_a_redacted_record_the_rule_would_still_change() {
        let temp = tempfile::tempdir().unwrap();
        let upstream = temp.path().join("adapters/vendor/upstream");
        fs::create_dir_all(&upstream).unwrap();
        let record = |body: &[u8]| {
            fs::write(upstream.join("doc.yaml"), body).unwrap();
            fs::write(
                upstream.join("vendor-source-hashes.json"),
                format!(
                    r#"[{{"file":"doc.yaml","url":"https://example.invalid/doc.yaml",
                         "upstream_sha256":"00","redaction":"{RULE}",
                         "sha256":"{}","bytes":{}}}]"#,
                    super::super::source_hashes::digest(body).unwrap(),
                    body.len()
                ),
            )
            .unwrap();
        };
        record(b"email: user@example.com\n");
        assert_eq!(run(temp.path()).unwrap(), 1);
        record(b"email: person@vendor.com\n");
        assert!(run(temp.path()).is_err());
    }
}
