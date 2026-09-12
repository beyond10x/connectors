//! Helm release-storage reads.
//!
//! Everything here is read from a Secret written by Helm's own storage driver.
//! The object name, type, data key, labels and payload encoding are pinned in
//! `../contracts/helm/v1alpha1/evidence/20260912/provider-sources.md`, which
//! cites both the Helm v4.3.0 and v3.22.0 sources line by line. Nothing in this
//! module infers a shape that is not cited there.
//!
//! The recorded values of a release routinely contain credentials, so no stored
//! scalar and no rendered manifest byte leaves this module. Both content reads
//! return a projection of structure and digests, and there is no code path that
//! returns the literal.
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use connectors_core::{Error, ErrorCode, Result, digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::io::Read as _;

/// `storage.go:38` (v4.3.0) and `storage.go:35` (v3.22.0).
pub const STORAGE_TYPE: &str = "sh.helm.release.v1";
/// `driver/secrets.go:284` (v4.3.0) and `driver/secrets.go:254` (v3.22.0).
const SECRET_TYPE: &str = "helm.sh/release.v1";
/// `driver/secrets.go:247,264` (v4.3.0) and `driver/secrets.go:217,234` (v3.22.0).
pub const OWNER: &str = "helm";
/// `common/status.go:25-41` (v4.3.0) and `status.go:25-41` (v3.22.0).
const STATUSES: [&str; 9] = [
    "unknown",
    "deployed",
    "uninstalled",
    "superseded",
    "failed",
    "uninstalling",
    "pending-install",
    "pending-upgrade",
    "pending-rollback",
];
/// `driver/util.go:32` (v4.3.0) and `driver/util.go:31` (v3.22.0).
const GZIP_MAGIC: [u8; 3] = [0x1f, 0x8b, 0x08];
/// A Kubernetes Secret is capped near 1 MiB, so a decoded release body larger
/// than this is not a release this binding is willing to walk.
const MAX_BODY_BYTES: usize = 1024 * 1024;
/// A single container with more entries than this refuses rather than being
/// expanded into an unbounded intermediate.
const MAX_CONTAINER_ENTRIES: usize = 65_536;
/// `items[].path` is declared `minLength 1, maxLength 1024` in
/// `spec/adapter.json`, and the host terminates the local runtime child when a
/// result fails its own published schema
/// (`crates/connectors-host/src/local/runtime/process.rs:248-253`). Every
/// provider-derived string this module emits is therefore checked against the
/// bound the schema declares for it, before it is emitted.
const MAX_PATH_BYTES: usize = 1024;
/// `items[].source_revision` is declared `maxLength 512`.
const MAX_SOURCE_REVISION_BYTES: usize = 512;
/// `namespace` is declared `maxLength 63`, which is also the Kubernetes limit.
const MAX_NAMESPACE_BYTES: usize = 63;
/// `items[].source_secret` is declared `maxLength 253`.
const MAX_OBJECT_NAME_BYTES: usize = 253;

fn protocol(message: &str) -> Error {
    Error::new(ErrorCode::UpstreamProtocol, message)
}
fn capacity(message: &str) -> Error {
    Error::new(ErrorCode::Capacity, message)
}

/// `validate_name.go:35,60` (v4.3.0) and `validate_name.go:36,61` (v3.22.0):
/// `^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$`, at most
/// 53 bytes. A name Helm itself would refuse cannot name a stored release, so
/// it is rejected before any provider request rather than looked up.
pub fn valid_release(name: &str) -> bool {
    if name.is_empty() || name.len() > 53 {
        return false;
    }
    name.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

/// `storage.go:327-328` (v4.3.0) and `storage.go:251-252` (v3.22.0).
pub fn object_name(release: &str, revision: u32) -> String {
    format!("{STORAGE_TYPE}.{release}.v{revision}")
}

/// The label selector Helm's own storage uses: `{name, owner}` for history and
/// `{name, owner, status}` for the deployed set (`storage.go:212-215` and
/// `:193-200` in v4.3.0; `:153-156` and `:134-141` in v3.22.0).
pub fn selector(release: &str, deployed: bool) -> String {
    let mut selector = format!("owner={OWNER},name={release}");
    if deployed {
        selector.push_str(",status=deployed");
    }
    selector
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Revision {
    pub source_secret: String,
    pub source_revision: Option<String>,
    pub namespace: String,
    pub release: String,
    pub revision: u32,
    pub status: String,
    pub created_at_unix_s: Option<u64>,
    pub modified_at_unix_s: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedValue {
    pub source_secret: String,
    pub path: String,
    pub kind: String,
    pub value_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestDocument {
    pub source_secret: String,
    pub index: u32,
    pub bytes: u64,
    pub content_digest: String,
}

/// Canonical unsigned decimal: no sign, no leading zero, no separator. Helm
/// writes these with `strconv.Itoa` and `FormatInt`, so a value that is not
/// canonical did not come from the pinned writer and is not reinterpreted.
fn decimal(text: &str) -> Option<u64> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return None;
    }
    text.parse().ok()
}

fn label<'a>(secret: &'a Value, name: &str) -> Option<&'a str> {
    secret["metadata"]["labels"][name].as_str()
}

/// One optional provider string, distinguishing three states the rest of this
/// module must not conflate: `Some(Some(text))` present and readable,
/// `Some(None)` genuinely absent, and `None` present in a shape this binding
/// has not established — which is not an absence and must refuse. An explicit
/// JSON null counts as present: the provider wrote something, and reporting it
/// as "not recorded" would claim an absence nothing established.
pub fn optional_string<'a>(object: &'a Value, key: &str) -> Option<Option<&'a str>> {
    match object.get(key) {
        None => Some(None),
        Some(Value::String(text)) => Some(Some(text.as_str())),
        Some(_) => None,
    }
}

/// Read one release-Secret's metadata as a revision observation. Every field
/// comes from the object's own name, type and labels; the payload is not
/// decoded, because a revision record needs none of it.
pub fn revision(secret: &Value, namespace: &str, release: &str) -> Result<Revision> {
    if secret["type"].as_str() != Some(SECRET_TYPE) {
        // A foreign object carrying Helm's labels refuses the page instead of
        // being skipped: a skipped row would make a complete page a lie.
        return Err(protocol("labelled object is not a Helm release Secret"));
    }
    if label(secret, "owner") != Some(OWNER) {
        return Err(protocol("release Secret is not owned by Helm"));
    }
    if label(secret, "name") != Some(release) {
        return Err(protocol("release Secret names a different release"));
    }
    // The observed namespace is what the object says it is. A mismatch with the
    // admitted selection still refuses, but an object that reports its own
    // namespace is never re-labelled with the caller's input. A value that is
    // present in a shape this binding has not established is not an omission,
    // so the fallback is reached only when the key is genuinely absent.
    let observed = optional_string(&secret["metadata"], "namespace")
        .ok_or_else(|| protocol("release Secret namespace is not a string"))?;
    if observed.is_some_and(|value| value != namespace) {
        return Err(protocol("release Secret reports a different namespace"));
    }
    let observed = observed.unwrap_or(namespace);
    if observed.is_empty() || observed.len() > MAX_NAMESPACE_BYTES {
        return Err(protocol(
            "release Secret namespace exceeds its declared bound",
        ));
    }
    let number = label(secret, "version")
        .and_then(decimal)
        .filter(|value| (1..=u64::from(u32::MAX)).contains(value))
        .ok_or_else(|| protocol("release Secret has no canonical revision label"))?;
    let number = u32::try_from(number).map_err(|_| protocol("release revision overflows"))?;
    let status = label(secret, "status")
        .filter(|value| STATUSES.contains(value))
        .ok_or_else(|| protocol("release Secret has an unknown status label"))?;
    let name = secret["metadata"]["name"]
        .as_str()
        .ok_or_else(|| protocol("release Secret has no name"))?;
    if name != object_name(release, number) || name.len() > MAX_OBJECT_NAME_BYTES {
        return Err(protocol("release Secret name does not match its labels"));
    }
    let source_revision = match optional_string(&secret["metadata"], "resourceVersion")
        .ok_or_else(|| protocol("release Secret resourceVersion is not a string"))?
    {
        Some(value) if value.len() > MAX_SOURCE_REVISION_BYTES => {
            return Err(protocol(
                "release Secret resourceVersion exceeds its declared bound",
            ));
        }
        value => value.map(str::to_owned),
    };
    let timestamp = |key: &str| -> Result<Option<u64>> {
        match label(secret, key) {
            None => Ok(None),
            Some(text) => decimal(text)
                .map(Some)
                .ok_or_else(|| protocol("release Secret has a non-canonical timestamp label")),
        }
    };
    Ok(Revision {
        source_secret: name.to_owned(),
        source_revision,
        namespace: observed.to_owned(),
        release: release.to_owned(),
        revision: number,
        status: status.to_owned(),
        created_at_unix_s: timestamp("createdAt")?,
        modified_at_unix_s: timestamp("modifiedAt")?,
    })
}

/// Decode the stored release body. The Kubernetes API base64-encodes the bytes
/// Helm stored, and those bytes are already Helm's own base64 of the gzipped
/// JSON (`driver/util.go:30,56` and `:39,44` in v4.3.0), so the decode is two
/// base64 passes and a gzip pass that runs only when the magic is present
/// (`driver/util.go:72`). The decompressed size is bounded; a body that would
/// exceed the bound refuses rather than being consumed.
pub fn body(secret: &Value) -> Result<Value> {
    let stored = secret["data"]["release"]
        .as_str()
        .ok_or_else(|| protocol("release Secret has no stored release payload"))?;
    if stored.len() > 4 * MAX_BODY_BYTES {
        return Err(capacity("stored release payload exceeds the read budget"));
    }
    let transport = BASE64
        .decode(stored)
        .map_err(|_| protocol("stored release payload is not base64"))?;
    let encoded = BASE64
        .decode(&transport)
        .map_err(|_| protocol("stored release payload is not a Helm base64 record"))?;
    let bytes = if encoded.len() > 3 && encoded[..3] == GZIP_MAGIC {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&encoded[..])
            .take(MAX_BODY_BYTES as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|_| protocol("stored release payload is not valid gzip"))?;
        if out.len() > MAX_BODY_BYTES {
            return Err(capacity("decoded release body exceeds the read budget"));
        }
        out
    } else {
        encoded
    };
    if bytes.len() > MAX_BODY_BYTES {
        return Err(capacity("decoded release body exceeds the read budget"));
    }
    let body: Value = connectors_core::read_json(&bytes)
        .map_err(|_| protocol("stored release body is not valid JSON"))?;
    // Helm marshals a struct, so a release body is a JSON object. A payload
    // that decodes to null, an array or a scalar is not a release record, and
    // projecting it as one would report an empty, complete recorded-value set
    // over a body nothing observed — the same unsound completeness claim this
    // contract refuses one layer up for a foreign labelled object.
    release_record(&body)?;
    Ok(body)
}

/// The decoded body as the release record it claims to be.
fn release_record(body: &Value) -> Result<&serde_json::Map<String, Value>> {
    body.as_object()
        .ok_or_else(|| protocol("stored release body is not a release record"))
}

/// Cross-check the decoded body against the object's own labels. The provenance
/// is built from the labels, so a body that names a different release, revision
/// or namespace must not be served under the labels' identity. Each field
/// carries `omitempty` in the pinned source, so absent is not a disagreement.
pub fn check_identity(body: &Value, record: &Revision) -> Result<()> {
    let map = release_record(body)?;
    let differs = |key: &str, expected: &str| {
        map.get(key)
            .is_some_and(|value| value.as_str() != Some(expected))
    };
    let disagrees = differs("name", &record.release)
        || differs("namespace", &record.namespace)
        || map
            .get("version")
            .is_some_and(|value| value.as_u64() != Some(u64::from(record.revision)));
    if disagrees {
        return Err(protocol(
            "stored release body disagrees with its object's labels",
        ));
    }
    Ok(())
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}

/// The children of one recorded node, each with the path it would be reported
/// under. A child whose path falls outside the bounds the output schema
/// declares for `items[].path` — an empty recorded key against `minLength 1`,
/// or a concatenation past `maxLength 1024` — is not projected, and `dropped`
/// records that so the page can report itself incomplete. Emitting the path
/// anyway would produce a result the host rejects, and the local runtime
/// terminates the child on that rejection
/// (`crates/connectors-host/src/local/runtime/process.rs:248-253`).
fn children<'a>(
    path: &str,
    value: &'a Value,
    dropped: &mut bool,
) -> Result<Vec<(String, &'a Value)>> {
    let mut entries: Vec<(String, &Value)> = Vec::new();
    let mut push = |child: String, value: &'a Value, entries: &mut Vec<(String, &'a Value)>| {
        if child.is_empty() || child.len() > MAX_PATH_BYTES {
            *dropped = true;
        } else {
            entries.push((child, value));
        }
    };
    match value {
        Value::Object(map) => {
            if map.len() > MAX_CONTAINER_ENTRIES {
                return Err(capacity("recorded value container exceeds the read budget"));
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                push(child, &map[key], &mut entries);
            }
        }
        Value::Array(items) => {
            if items.len() > MAX_CONTAINER_ENTRIES {
                return Err(capacity("recorded value container exceeds the read budget"));
            }
            for (index, item) in items.iter().enumerate() {
                push(format!("{path}[{index}]"), item, &mut entries);
            }
        }
        _ => {}
    }
    Ok(entries)
}

/// The safe projection of a release's recorded values (`release.go:41` in
/// v4.3.0, `:31` in v3.22.0). Every node of the recorded structure becomes a
/// path, a JSON shape and a SHA-256 over the canonical JSON of its value. No
/// recorded literal is carried, for any shape, at any depth: this is the whole
/// reason the operation exists. Returns the page and whether it is complete.
pub fn recorded_values(
    source_secret: &str,
    body: &Value,
    limit: usize,
) -> Result<(Vec<RecordedValue>, bool)> {
    let config = match release_record(body)?.get("config") {
        // `config` carries `omitempty`, so an install with no supplied values
        // stores no key at all. That is an empty recorded set, not a defect —
        // but only once the body itself is established as a release record.
        None | Some(Value::Null) => Value::Object(serde_json::Map::new()),
        Some(Value::Object(map)) => Value::Object(map.clone()),
        Some(_) => return Err(protocol("recorded release values are not an object")),
    };
    // A node whose path the declared bounds cannot represent is not projected,
    // and its absence makes the page incomplete rather than silent.
    let mut dropped = false;
    let mut pending = children("", &config, &mut dropped)?;
    pending.reverse();
    let mut items = Vec::new();
    while let Some((path, value)) = pending.pop() {
        if items.len() == limit {
            return Ok((items, false));
        }
        items.push(RecordedValue {
            source_secret: source_secret.to_owned(),
            path: path.clone(),
            kind: kind(value).to_owned(),
            value_digest: digest(value),
        });
        let mut next = children(&path, value, &mut dropped)?;
        next.reverse();
        pending.extend(next);
    }
    Ok((items, !dropped))
}

/// The safe projection of a revision's rendered manifest (`release.go:43` in
/// v4.3.0, `:33` in v3.22.0). The stored text is split on lines that are
/// exactly the YAML document separator and each part is reported by position,
/// byte length and digest. No manifest byte is carried: a rendered manifest
/// contains the Secret bodies the release applied.
pub fn manifest_documents(
    source_secret: &str,
    body: &Value,
    limit: usize,
) -> Result<(Vec<ManifestDocument>, bool)> {
    let manifest = match release_record(body)?.get("manifest") {
        // `manifest` carries `omitempty` too; a release that rendered nothing
        // stores no key.
        None | Some(Value::Null) => "",
        Some(Value::String(text)) => text.as_str(),
        Some(_) => return Err(protocol("rendered release manifest is not text")),
    };
    let mut items = Vec::new();
    let mut complete = true;
    let emit = |stored: &str, items: &mut Vec<ManifestDocument>| -> bool {
        // A chunk that is only whitespace is framing, not a document: the
        // leading separator of every Helm-written manifest produces one.
        if stored.trim().is_empty() {
            return true;
        }
        if items.len() == limit {
            return false;
        }
        items.push(ManifestDocument {
            source_secret: source_secret.to_owned(),
            index: items.len() as u32,
            // Exactly the stored bytes between the separators, with nothing
            // stripped — including the newline that terminates the document's
            // last line, which Helm's writer emits for every document
            // including the final one (`pkg/action/action.go:476`, pinned).
            // A reader that extracts the document and runs sha256sum over it
            // therefore reproduces `content_digest`, and `bytes` is the length
            // of those same bytes. This is the one digest in this module that
            // exists to be reproduced; see `value_digest`, which exists to be
            // compared.
            bytes: stored.len() as u64,
            content_digest: hex::encode(Sha256::digest(stored.as_bytes())),
        });
        true
    };
    // Split on separator lines by byte range rather than by rebuilding lines,
    // so each document is the stored bytes and not a reconstruction of them.
    let mut start = 0usize;
    let mut cursor = 0usize;
    loop {
        let newline = manifest[cursor..].find('\n').map(|offset| cursor + offset);
        let line_end = newline.unwrap_or(manifest.len());
        if manifest[cursor..line_end].trim_end_matches('\r') == "---" {
            if !emit(&manifest[start..cursor], &mut items) {
                complete = false;
                break;
            }
            start = newline.map_or(manifest.len(), |at| at + 1);
        }
        match newline {
            Some(at) => cursor = at + 1,
            None => break,
        }
    }
    if complete && !emit(&manifest[start..], &mut items) {
        complete = false;
    }
    Ok((items, complete))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stored(body: &Value, compress: bool) -> Value {
        let json = serde_json::to_vec(body).unwrap();
        let inner = if compress {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            std::io::Write::write_all(&mut encoder, &json).unwrap();
            encoder.finish().unwrap()
        } else {
            json
        };
        json!({"data":{"release":BASE64.encode(BASE64.encode(inner).as_bytes())}})
    }

    #[test]
    fn a_release_name_helm_would_refuse_is_refused_here() {
        for name in ["api", "api-0", "a.b", "0"] {
            assert!(valid_release(name), "{name}");
        }
        for name in ["", "Api", "-api", "api-", "a..b", "a_b", &"a".repeat(54)] {
            assert!(!valid_release(name), "{name}");
        }
    }

    #[test]
    fn both_stored_encodings_decode_and_neither_invents_a_shape() {
        let body = json!({"name":"api","version":1,"config":{"a":1}});
        for compress in [true, false] {
            assert_eq!(super::body(&stored(&body, compress)).unwrap(), body);
        }
        assert!(super::body(&json!({"data":{"release":"!!!"}})).is_err());
        assert!(super::body(&json!({"data":{}})).is_err());
    }

    #[test]
    fn every_recorded_shape_is_projected_and_no_literal_survives() {
        let body = json!({"config":{"s":"literal-value","n":1,"b":true,"z":null,
                                    "o":{"k":"nested-literal"},"a":["first"]}});
        let (items, complete) = recorded_values("secret", &body, 100).unwrap();
        assert!(complete);
        let paths: Vec<(&str, &str)> = items
            .iter()
            .map(|item| (item.path.as_str(), item.kind.as_str()))
            .collect();
        assert_eq!(
            paths,
            [
                ("a", "array"),
                ("a[0]", "string"),
                ("b", "boolean"),
                ("n", "number"),
                ("o", "object"),
                ("o.k", "string"),
                ("s", "string"),
                ("z", "null"),
            ]
        );
        let rendered = serde_json::to_string(&items).unwrap();
        for literal in ["literal-value", "nested-literal", "first"] {
            assert!(!rendered.contains(literal), "{literal} was disclosed");
        }
        // A bounded page reports its own incompleteness rather than pretending.
        let (items, complete) = recorded_values("secret", &body, 3).unwrap();
        assert_eq!(items.len(), 3);
        assert!(!complete);
        // A release stored with no recorded values is an empty complete set.
        assert_eq!(
            recorded_values("secret", &json!({}), 10).unwrap(),
            (Vec::new(), true)
        );
    }

    #[test]
    fn manifest_documents_are_positions_and_digests_not_text() {
        let body = json!({"manifest":"---\nkind: Secret\nstringData:\n  p: literal\n---\nkind: Service\n"});
        let (items, complete) = manifest_documents("secret", &body, 10).unwrap();
        assert!(complete);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].index, 0);
        assert_eq!(items[1].index, 1);
        assert_eq!(items[1].bytes, "kind: Service\n".len() as u64);
        let rendered = serde_json::to_string(&items).unwrap();
        for literal in ["literal", "kind: Secret", "Service"] {
            assert!(!rendered.contains(literal), "{literal} was disclosed");
        }
        let (items, complete) = manifest_documents("secret", &body, 1).unwrap();
        assert_eq!(items.len(), 1);
        assert!(!complete);
        assert_eq!(
            manifest_documents("secret", &json!({}), 10).unwrap(),
            (Vec::new(), true)
        );
    }

    /// The class behind the manifest-digest defect: this module emits exactly
    /// two digests, they cover different bytes on purpose, and three documents
    /// state which. Each is asserted here against a value computed from the
    /// byte sequence spelled out in the test, so a change to either one breaks
    /// a case that names the bytes rather than a case that mirrors the code.
    #[test]
    fn each_published_digest_covers_exactly_the_bytes_its_documents_name() {
        // content_digest: the document exactly as stored between its
        // separators, reproducible with sha256sum. Helm terminates every
        // document with a newline, including the last, so the stored document
        // includes it and this test names both stored documents in full.
        let first = "# Source: a.yaml\napiVersion: v1\nkind: Secret\n";
        let last = "# Source: b.yaml\napiVersion: v1\nkind: Service\n";
        let manifest = format!("---\n{first}---\n{last}");
        let (documents, _) = manifest_documents("s", &json!({ "manifest": manifest }), 10).unwrap();
        assert_eq!(documents.len(), 2);
        for (document, stored) in documents.iter().zip([first, last]) {
            assert_eq!(
                document.bytes,
                stored.len() as u64,
                "bytes must count the document as stored, trailing newline included"
            );
            assert_eq!(
                document.content_digest,
                hex::encode(Sha256::digest(stored.as_bytes())),
                "content_digest must be a SHA-256 over the stored document bytes"
            );
        }
        // The last document is framed exactly like the others: nothing about
        // reaching end-of-file changes what its bytes are.
        assert_eq!(documents[1].bytes, last.len() as u64);

        // value_digest: the canonical JSON of the value, comparable across
        // revisions. Key order in the source must not change it.
        let value = json!({"b":2,"a":[1,"x"]});
        let (values, _) =
            recorded_values("s", &json!({ "config": {"k": value.clone()} }), 10).unwrap();
        let canonical = br#"{"a":[1,"x"],"b":2}"#;
        assert_eq!(
            values[0].value_digest,
            hex::encode(Sha256::digest(canonical)),
            "value_digest must be a SHA-256 over the canonical JSON of the value"
        );
        let reordered = json!({"a":[1,"x"],"b":2});
        let (again, _) = recorded_values("s", &json!({ "config": {"k": reordered} }), 10).unwrap();
        assert_eq!(values[0].value_digest, again[0].value_digest);

        // The two are deliberately different functions of the same input.
        let (one, _) = manifest_documents("s", &json!({"manifest": "---\nx\n"}), 10).unwrap();
        let (other, _) = recorded_values("s", &json!({"config": {"k": "x"}}), 10).unwrap();
        assert_ne!(one[0].content_digest, other[0].value_digest);
    }

    /// The class behind the path bound: every provider-derived string this
    /// module emits is checked against the bound its output schema declares,
    /// and a value that cannot be represented is dropped with the page marked
    /// incomplete rather than emitted into a result the host would reject.
    #[test]
    fn a_path_outside_its_declared_bound_is_dropped_and_the_page_says_so() {
        let deep = "a".repeat(180);
        let mut config = json!("leaf");
        for _ in 0..6 {
            let mut level = serde_json::Map::new();
            level.insert(deep.clone(), config);
            config = Value::Object(level);
        }
        let (items, complete) = recorded_values("s", &json!({ "config": config }), 500).unwrap();
        assert!(!complete, "a dropped node must make the page incomplete");
        assert_eq!(
            items.len(),
            5,
            "the five representable levels are projected"
        );
        assert!(
            items
                .iter()
                .all(|item| { (1..=MAX_PATH_BYTES).contains(&item.path.len()) })
        );

        // An empty recorded key is legal JSON and has no representable path.
        let (items, complete) =
            recorded_values("s", &json!({"config":{"":"value","kept":1}}), 500).unwrap();
        assert!(!complete);
        assert_eq!(
            items.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(),
            ["kept"]
        );
        // An array index that would push the path over the bound goes too.
        let long = "b".repeat(MAX_PATH_BYTES);
        let (items, complete) =
            recorded_values("s", &json!({"config":{ long.clone(): [1] }}), 500).unwrap();
        assert!(!complete);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, long);
    }

    /// The class behind the completeness defect: no projection concludes
    /// "empty and complete" from a payload whose shape it has not established.
    #[test]
    fn a_payload_that_is_not_a_release_record_never_projects_as_complete() {
        for payload in [
            json!(null),
            json!([1, 2, 3]),
            json!("not a release"),
            json!(7),
        ] {
            assert!(
                recorded_values("s", &payload, 500).is_err(),
                "recorded_values accepted {payload}"
            );
            assert!(
                manifest_documents("s", &payload, 500).is_err(),
                "manifest_documents accepted {payload}"
            );
        }
        // A release record with neither key is genuinely empty and complete.
        assert_eq!(
            recorded_values("s", &json!({"name":"api"}), 500).unwrap(),
            (Vec::new(), true)
        );
        assert_eq!(
            manifest_documents("s", &json!({"name":"api"}), 500).unwrap(),
            (Vec::new(), true)
        );
    }

    /// The class behind the namespace defect: no field of an observation is
    /// the caller's own input echoed back, and every provider string is inside
    /// the bound its schema declares.
    #[test]
    fn an_observation_carries_observed_values_inside_their_declared_bounds() {
        let base = json!({"metadata":{"name":"sh.helm.release.v1.api.v1","namespace":"ns",
            "resourceVersion":"9","labels":{"name":"api","owner":"helm","status":"deployed",
            "version":"1"}},"type":"helm.sh/release.v1"});
        assert_eq!(revision(&base, "ns", "api").unwrap().namespace, "ns");

        // An object that omits its namespace falls back to the admitted
        // selection, which is the only value there is.
        let mut omitted = base.clone();
        omitted["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("namespace");
        assert_eq!(revision(&omitted, "ns", "api").unwrap().namespace, "ns");

        let mut over = base.clone();
        over["metadata"]["resourceVersion"] = json!("9".repeat(513));
        assert!(revision(&over, "ns", "api").is_err());
        let mut at = base.clone();
        at["metadata"]["resourceVersion"] = json!("9".repeat(512));
        assert!(revision(&at, "ns", "api").is_ok());

        let long = "n".repeat(64);
        let mut wide = base.clone();
        wide["metadata"]["namespace"] = json!(long.clone());
        assert!(revision(&wide, &long, "api").is_err());

        // Present in a shape this binding has not established is not an
        // omission, for either field, for any non-string shape.
        for shape in [json!(123), json!(null), json!(true), json!([]), json!({})] {
            for key in ["namespace", "resourceVersion"] {
                let mut unreadable = base.clone();
                unreadable["metadata"][key] = shape.clone();
                assert!(
                    revision(&unreadable, "ns", "api").is_err(),
                    "{key} = {shape} was read as an omission"
                );
            }
        }
    }

    /// A body that disagrees with the labels its provenance is built from is
    /// not served under those labels. Absent fields carry `omitempty` and are
    /// not a disagreement.
    #[test]
    fn a_body_disagreeing_with_its_object_labels_is_refused() {
        let secret = json!({"metadata":{"name":"sh.helm.release.v1.api.v2","namespace":"ns",
            "resourceVersion":"9","labels":{"name":"api","owner":"helm","status":"deployed",
            "version":"2"}},"type":"helm.sh/release.v1"});
        let record = revision(&secret, "ns", "api").unwrap();
        check_identity(&json!({"config":{}}), &record).unwrap();
        check_identity(
            &json!({"name":"api","namespace":"ns","version":2,"config":{}}),
            &record,
        )
        .unwrap();
        for wrong in [
            json!({"name":"other"}),
            json!({"namespace":"elsewhere"}),
            json!({"version":3}),
        ] {
            assert!(check_identity(&wrong, &record).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_revision_record_is_read_only_from_a_helm_release_secret() {
        let secret = json!({"metadata":{"name":"sh.helm.release.v1.api.v2","namespace":"ns",
            "resourceVersion":"9","labels":{"name":"api","owner":"helm","status":"deployed",
            "version":"2","modifiedAt":"1757000000"}},"type":"helm.sh/release.v1"});
        let observed = revision(&secret, "ns", "api").unwrap();
        assert_eq!(observed.revision, 2);
        assert_eq!(observed.modified_at_unix_s, Some(1_757_000_000));
        assert_eq!(observed.created_at_unix_s, None);
        let mut foreign = secret.clone();
        foreign["type"] = json!("Opaque");
        assert!(revision(&foreign, "ns", "api").is_err());
        let mut renamed = secret.clone();
        renamed["metadata"]["name"] = json!("sh.helm.release.v1.api.v3");
        assert!(revision(&renamed, "ns", "api").is_err());
        let mut padded = secret.clone();
        padded["metadata"]["labels"]["version"] = json!("02");
        assert!(revision(&padded, "ns", "api").is_err());
        let mut unknown = secret.clone();
        unknown["metadata"]["labels"]["status"] = json!("rolled-forward");
        assert!(revision(&unknown, "ns", "api").is_err());
        let mut elsewhere = secret;
        elsewhere["metadata"]["namespace"] = json!("other");
        assert!(revision(&elsewhere, "ns", "api").is_err());
    }
}
