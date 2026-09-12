//! Adversarial cases driving `adapters/kubernetes/src/helm.rs` and the Helm
//! release-read dispatch in `adapters/kubernetes/src/lib.rs` against the
//! documents this unit shipped beside them:
//!
//! * `adapters/kubernetes/contracts/helm/v1alpha1/semantics.md`
//! * `adapters/kubernetes/generated/descriptor.json` (from `spec/adapter.json`)
//! * `docs/local-kubernetes-cli.md`
//!
//! Nothing here asserts a behaviour of the adversary's own invention: every
//! assertion quotes the sentence or the schema keyword it is checking. Each
//! case is expected to be red.
use connectors_core::Result;
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes, helm};
use connectors_sdk::{Adapter as _, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, sync::Arc};

// ---------------------------------------------------------------------------
// Fixture: one release Secret served for any GET.
// ---------------------------------------------------------------------------

struct OneObject(Value);

#[async_trait::async_trait]
impl AuthenticatedHttp for OneObject {
    async fn get(&self, _segments: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        Ok(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&self.0).unwrap(),
        })
    }
}

/// Helm stores base64(gzip(json)); the Kubernetes API base64-encodes those
/// bytes again for the wire. Same construction as the unit's own fixture in
/// `tests/local_runtime.rs`.
fn stored(body: &Value) -> String {
    use base64::Engine as _;
    let json = serde_json::to_vec(body).unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    std::io::Write::write_all(&mut encoder, &json).unwrap();
    let helm = base64::engine::general_purpose::STANDARD.encode(encoder.finish().unwrap());
    base64::engine::general_purpose::STANDARD.encode(helm.as_bytes())
}

fn release_secret(body: &Value) -> Value {
    json!({
        "metadata":{"name":"sh.helm.release.v1.api.v1","namespace":"fixture",
                    "resourceVersion":"11",
                    "labels":{"name":"api","owner":"helm","status":"deployed","version":"1"}},
        "type":"helm.sh/release.v1",
        "data":{"release":stored(body)}
    })
}

fn adapter(secret: Value) -> Kubernetes {
    let config = Config {
        namespaces: vec!["fixture".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::RedactedContent,
    };
    let effective = json!({"service":{"instance":"adversary","listen":"127.0.0.1:0",
        "service_credential":{"kind":"environment","name":"UNUSED"}},
        "http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    Kubernetes::new("adversary", config, effective, Arc::new(OneObject(secret))).unwrap()
}

/// The published output schema for one operation, as any consumer of the
/// descriptor sees it, and as `crates/connectors-host/src/local/runtime/
/// process.rs:248` and `server.rs:237` enforce it on every result.
async fn invoke_and_validate(secret: Value, operation: &str, input: Value) -> Result<()> {
    let adapter = adapter(secret);
    let descriptor = adapter.descriptor();
    let schema = descriptor
        .operation(operation)
        .unwrap()
        .output_schema
        .clone();
    let result = adapter.invoke(operation, input).await?;
    connectors_sdk::validate(&schema, &result)
}

// ---------------------------------------------------------------------------
// 1. The manifest digest the contract promises.
// ---------------------------------------------------------------------------

/// `contracts/helm/v1alpha1/semantics.md:87`:
///
/// > `helm_releases.manifest` returns one item per document of the rendered
/// > text: its position, its UTF-8 byte length and **a SHA-256 over that
/// > text**.
///
/// and `docs/local-kubernetes-cli.md`:
///
/// > `manifest` returns one entry per rendered document with its position,
/// > byte length and **a SHA-256 of its text**.
///
/// The neighbouring sentence for `helm_releases.values` says "a SHA-256 over
/// the canonical JSON of its value", so the two phrasings are a deliberate
/// distinction, not a loose restatement of one rule.
#[test]
fn manifest_content_digest_is_sha256_over_the_document_text() {
    let text = "kind: Service\nmetadata:\n  name: api";
    // AMENDED by the coordinator, story:kubernetes-helm-release-reads.
    //
    // This fixture appends a newline after `text`, so the document as Helm
    // stores it is `text` plus that newline — 36 bytes, not 35. When this case
    // was written the defect under test was a digest over the JSON encoding
    // rather than the text, and the implementation trimmed, so both readings
    // agreed at 35 and the distinction did not matter. The correction stopped
    // trimming, and the two readings separated.
    //
    // As-stored is the right reading and it is pinned, not assumed:
    // vendor/helm-v4.3.0-action.go:358,476 and vendor/helm-v3.22.0-action.go:183
    // write every document, the last one included, as "---\n# Source: %s\n%s\n".
    // Under a rule that stripped the newline at end-of-input, the final
    // document of every real Helm manifest would fail `sha256sum` — which is
    // the very defect adversary pass 2 filed as F1, relocated rather than
    // fixed.
    //
    // The assertion now names the stored document. If the projection ever
    // trims again, this goes red.
    let document = format!("{text}\n");
    let body = json!({ "manifest": format!("---\n{document}") });
    let (items, complete) =
        helm::manifest_documents("sh.helm.release.v1.api.v1", &body, 10).unwrap();
    assert!(complete);
    assert_eq!(items.len(), 1);
    // The byte length the same sentence promises does hold, which fixes which
    // bytes "that text" names: exactly these.
    assert_eq!(items[0].bytes, document.len() as u64);

    let over_the_text = hex::encode(Sha256::digest(document.as_bytes()));
    let over_the_json_encoding = hex::encode(Sha256::digest(
        serde_json::to_vec(&json!(document)).unwrap().as_slice(),
    ));
    assert_eq!(
        items[0].content_digest, over_the_text,
        "semantics.md:87 promises a SHA-256 over the document text \
         ({over_the_text}); the reported digest is a SHA-256 over the JSON \
         encoding of that text ({over_the_json_encoding}), so a consumer \
         cannot reproduce it with sha256sum over the document"
    );
}

// ---------------------------------------------------------------------------
// 2 and 3. The bounds the published output schema declares on `path`.
// ---------------------------------------------------------------------------

/// `spec/adapter.json` and the generated descriptor declare
/// `items[].path` as `{"type":"string","minLength":1,"maxLength":1024}` for
/// `helm_releases.values`. `helm::recorded_values` concatenates provider-owned
/// keys without any bound, so a release whose recorded values nest deeply
/// enough emits a path longer than the schema the adapter publishes. The host
/// validates every result against that schema and, in the local runtime,
/// terminates the child when validation fails
/// (`crates/connectors-host/src/local/runtime/process.rs:248-253`).
#[tokio::test]
async fn a_deeply_nested_recorded_value_stays_inside_the_published_path_bound() {
    // Six levels of Kubernetes-annotation-length keys: legal map keys in any
    // values file, and 1085 characters of path.
    let key = "a".repeat(180);
    let mut config = json!("leaf");
    for _ in 0..6 {
        let mut level = serde_json::Map::new();
        level.insert(key.clone(), config);
        config = Value::Object(level);
    }
    let secret = release_secret(&json!({ "config": config }));

    let deepest = format!("{key}.{key}.{key}.{key}.{key}.{key}");
    assert!(deepest.len() > 1024, "the fixture must exceed the bound");

    invoke_and_validate(
        secret,
        "helm_releases.values",
        json!({"namespace":"fixture","release":"api","revision":1,"limit":500}),
    )
    .await
    .expect(
        "helm_releases.values emitted a result its own published output_schema \
         rejects: items[].path declares maxLength 1024 and the projection \
         bounds nothing",
    );
}

/// The same declaration carries `minLength: 1`. An empty map key is legal
/// JSON and legal YAML, and `helm::recorded_values` turns it into the empty
/// path.
#[tokio::test]
async fn an_empty_recorded_key_stays_inside_the_published_path_bound() {
    let secret = release_secret(&json!({"config":{"":"value"}}));
    invoke_and_validate(
        secret,
        "helm_releases.values",
        json!({"namespace":"fixture","release":"api","revision":1,"limit":500}),
    )
    .await
    .expect(
        "helm_releases.values emitted a result its own published output_schema \
         rejects: items[].path declares minLength 1 and an empty recorded key \
         projects to the empty string",
    );
}

// ---------------------------------------------------------------------------
// 4. The limit range the operator-facing document states.
// ---------------------------------------------------------------------------

/// `docs/local-kubernetes-cli.md` lists the request shape of every advertised
/// operation in one table and then states the rule for the field it just used:
///
/// > `limit` is between 1 and 100.
///
/// This unit added two rows to that table using `"limit":200`, which the
/// contract and the descriptor do allow (1-500 for the two projections) and
/// which the sentence immediately below the table forbids. An operator copying
/// the documented request reads, two lines later, that it is out of range.
#[test]
fn the_cli_document_does_not_contradict_its_own_limit_examples() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/local-kubernetes-cli.md"
    );
    let document = std::fs::read_to_string(path).unwrap();
    let stated = "`limit` is between 1 and 100.";
    assert!(
        document.contains(stated),
        "the sentence this case is about moved; re-read the document"
    );
    let mut offending: Vec<(usize, u32)> = Vec::new();
    for (number, line) in document.lines().enumerate() {
        let mut rest = line;
        while let Some(at) = rest.find("\"limit\":") {
            rest = &rest[at + "\"limit\":".len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(value) = digits.parse::<u32>()
                && !(1..=100).contains(&value)
            {
                offending.push((number + 1, value));
            }
        }
    }
    assert_eq!(
        offending,
        Vec::new(),
        "docs/local-kubernetes-cli.md states {stated:?} and gives request \
         examples outside that range at these (line, limit) pairs"
    );
}

// ---------------------------------------------------------------------------
// 5. Completeness over a body that was never a release record.
// ---------------------------------------------------------------------------

/// `contracts/helm/v1alpha1/semantics.md:68-72` states the soundness rule this
/// unit chose over Helm's own driver:
///
/// > An object carrying Helm's labels whose type is not `helm.sh/release.v1`
/// > refuses the page. It is deliberately not skipped: ... a skipped row would
/// > leave `complete:true` claiming a history it did not observe.
///
/// The same reasoning is not applied one layer down. `helm::body` parses the
/// stored payload as arbitrary JSON and neither it nor `recorded_values`
/// checks that the result is a release record, so a payload that decodes to a
/// JSON scalar or array is reported as a release that recorded no values, with
/// `complete: true` — a completeness claim over a body nothing observed.
#[test]
fn a_stored_payload_that_is_not_a_release_record_is_not_reported_complete() {
    for payload in [
        json!(null),
        json!([1, 2, 3]),
        json!("not a release"),
        json!(7),
    ] {
        let projected = helm::recorded_values("sh.helm.release.v1.api.v1", &payload, 500);
        assert!(
            projected.is_err(),
            "a stored body of {payload} projected as {:?} instead of refusing; \
             reporting complete:true over it claims a recorded-value set that \
             was never observed",
            projected.unwrap()
        );
    }
}
