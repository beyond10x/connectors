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
use serde_json::{Value, json};
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
    let observed = secret["metadata"]["namespace"].as_str();
    if observed.is_some_and(|value| value != namespace) {
        return Err(protocol("release Secret reports a different namespace"));
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
    if name != object_name(release, number) {
        return Err(protocol("release Secret name does not match its labels"));
    }
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
        source_revision: secret["metadata"]["resourceVersion"]
            .as_str()
            .map(str::to_owned),
        namespace: namespace.to_owned(),
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
    connectors_core::read_json(&bytes)
        .map_err(|_| protocol("stored release body is not valid JSON"))
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

fn children<'a>(path: &str, value: &'a Value) -> Result<Vec<(String, &'a Value)>> {
    let entries: Vec<(String, &Value)> = match value {
        Value::Object(map) => {
            if map.len() > MAX_CONTAINER_ENTRIES {
                return Err(capacity("recorded value container exceeds the read budget"));
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            keys.into_iter()
                .map(|key| {
                    let child = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    (child, &map[key])
                })
                .collect()
        }
        Value::Array(items) => {
            if items.len() > MAX_CONTAINER_ENTRIES {
                return Err(capacity("recorded value container exceeds the read budget"));
            }
            items
                .iter()
                .enumerate()
                .map(|(index, item)| (format!("{path}[{index}]"), item))
                .collect()
        }
        _ => Vec::new(),
    };
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
    let config = match &body["config"] {
        // `config` carries `omitempty`, so an install with no supplied values
        // stores no key at all. That is an empty recorded set, not a defect.
        Value::Null => Value::Object(serde_json::Map::new()),
        Value::Object(map) => Value::Object(map.clone()),
        _ => return Err(protocol("recorded release values are not an object")),
    };
    let mut pending = children("", &config)?;
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
        let mut next = children(&path, value)?;
        next.reverse();
        pending.extend(next);
    }
    Ok((items, true))
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
    let manifest = match &body["manifest"] {
        // `manifest` carries `omitempty` too; a release that rendered nothing
        // stores no key.
        Value::Null => "",
        Value::String(text) => text.as_str(),
        _ => return Err(protocol("rendered release manifest is not text")),
    };
    let mut items = Vec::new();
    let mut document = String::new();
    let mut complete = true;
    let emit = |text: &str, items: &mut Vec<ManifestDocument>| -> bool {
        let text = text.trim();
        if text.is_empty() {
            return true;
        }
        if items.len() == limit {
            return false;
        }
        items.push(ManifestDocument {
            source_secret: source_secret.to_owned(),
            index: items.len() as u32,
            bytes: text.len() as u64,
            content_digest: digest(&json!(text)),
        });
        true
    };
    for line in manifest.split('\n') {
        if line.trim_end_matches('\r') == "---" {
            if !emit(&document, &mut items) {
                complete = false;
                break;
            }
            document.clear();
        } else {
            document.push_str(line);
            document.push('\n');
        }
    }
    if complete && !emit(&document, &mut items) {
        complete = false;
    }
    Ok((items, complete))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(items[1].bytes, "kind: Service".len() as u64);
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
