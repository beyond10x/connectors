//! Adversarial pass 2 on `story:kubernetes-helm-release-reads`, against the
//! tree at `e5938a6` (unit `5e4b0c4` plus correction round 1).
//!
//! Pass 1 lives in `helm_contract_adversary.rs` and is untouched. Every case
//! below drives `adapters/kubernetes/src/helm.rs` and the release dispatch in
//! `adapters/kubernetes/src/lib.rs` against a sentence this unit wrote about
//! itself, in one of the four artefacts the correction round claims now agree:
//!
//! * `adapters/kubernetes/contracts/helm/v1alpha1/semantics.md`
//! * `adapters/kubernetes/spec/ess/domains/helm.yaml`
//! * `docs/local-kubernetes-cli.md`
//! * `adapters/kubernetes/spec/adapter.json` (and the generated descriptor)
//!
//! Nothing here asserts a behaviour of the adversary's own invention: each
//! assertion quotes the line it is checking. Each case is expected to be red.
use connectors_core::Result;
use connectors_kubernetes::{Config, HelmReleaseReads, Kubernetes, helm};
use connectors_sdk::{Adapter as _, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, sync::Arc};

// ---------------------------------------------------------------------------
// Fixture: one document served for every GET.
// ---------------------------------------------------------------------------

struct Serve(Value);

#[async_trait::async_trait]
impl AuthenticatedHttp for Serve {
    async fn get(&self, _segments: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        Ok(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&self.0).unwrap(),
        })
    }
}

/// Helm stores base64(gzip(json)); the Kubernetes API base64-encodes those
/// bytes again for the wire. Same construction as the unit's own fixtures in
/// `tests/local_runtime.rs` and `tests/helm_contract_adversary.rs`.
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

fn adapter(served: Value) -> Kubernetes {
    let config = Config {
        namespaces: vec!["fixture".into()],
        resource_kinds: vec!["pods".into()],
        discover_hosts: false,
        helm_release_reads: HelmReleaseReads::RedactedContent,
    };
    let effective = json!({"service":{"instance":"adversary2","listen":"127.0.0.1:0",
        "service_credential":{"kind":"environment","name":"UNUSED"}},
        "http":{"base_url":"https://fixture.invalid/"},"adapter":config});
    Kubernetes::new("adversary2", config, effective, Arc::new(Serve(served))).unwrap()
}

// ---------------------------------------------------------------------------
// 1. The manifest digest all four artefacts say `sha256sum` reproduces.
// ---------------------------------------------------------------------------

/// `contracts/helm/v1alpha1/semantics.md:94-96`:
///
/// > `content_digest` exists to be **reproduced**: a reader holding the
/// > rendered document must get the same value from `sha256sum`, so it covers
/// > the document text itself and nothing else.
///
/// `docs/local-kubernetes-cli.md:237-239`:
///
/// > `manifest` returns one entry per rendered document with its position, byte
/// > length and a SHA-256 over exactly the document text those bytes count,
/// > which `sha256sum` on the document reproduces.
///
/// `spec/ess/domains/helm.yaml:114-116`:
///
/// > `content_digest` a SHA-256 over exactly those bytes — the document text
/// > itself, which sha256sum on the document reproduces.
///
/// The reader those three sentences address holds the rendered manifest and
/// splits it at the separator lines. Helm's own writer emits every document as
/// `"---\n# Source: <file>\n<body>\n"` (`pkg/action` renderResources), so the
/// document a reader extracts ends in the newline that precedes the next
/// separator. `manifest_documents` digests `text.trim()` instead
/// (`src/helm.rs:414,429`), so the reproduction those three sentences promise
/// does not hold for any document of any Helm-written manifest.
#[test]
fn the_manifest_digest_is_reproducible_by_sha256sum_over_the_stored_document() {
    // Exactly the shape Helm's writer produces, and the shape the unit's own
    // fixture in tests/local_runtime.rs:107 uses.
    let first = "# Source: chart/templates/secret.yaml\napiVersion: v1\nkind: Secret\n";
    let second = "# Source: chart/templates/service.yaml\napiVersion: v1\nkind: Service\n";
    let manifest = format!("---\n{first}---\n{second}");

    // What a reader holding that manifest extracts as document 0: the bytes
    // between the separator lines, unaltered. Reconstructed here from the same
    // pieces the manifest was built from, so the test names the bytes rather
    // than re-implementing the split.
    let (documents, complete) = helm::manifest_documents(
        "sh.helm.release.v1.api.v1",
        &json!({ "manifest": manifest }),
        10,
    )
    .unwrap();
    assert!(complete);
    assert_eq!(documents.len(), 2);

    assert_eq!(
        documents[0].bytes,
        first.len() as u64,
        "semantics.md:94-96 and docs/local-kubernetes-cli.md:237-239 promise a \
         byte length over the document text; document 0 of this manifest is \
         {} bytes and the projection reports {}",
        first.len(),
        documents[0].bytes
    );
    assert_eq!(
        documents[0].content_digest,
        hex::encode(Sha256::digest(first.as_bytes())),
        "semantics.md:95 promises `a reader holding the rendered document must \
         get the same value from sha256sum`. sha256sum over document 0 of this \
         manifest is {}; the projection reports {}, which is a SHA-256 over the \
         same text with its trailing newline stripped",
        hex::encode(Sha256::digest(first.as_bytes())),
        documents[0].content_digest
    );
}

// ---------------------------------------------------------------------------
// 2. The recorded-value page the CLI document describes.
// ---------------------------------------------------------------------------

/// `docs/local-kubernetes-cli.md:235-236`:
///
/// > `values` returns **one entry per recorded path** with its JSON shape and a
/// > SHA-256 over the canonical JSON of its value
///
/// and `docs/local-kubernetes-cli.md:251-253`, the only account that document
/// gives of an incomplete projection:
///
/// > Both projections read a single object, so they never issue a cursor: **a
/// > projection larger than `limit`** comes back with `complete: false` and
/// > `next_cursor: null`.
///
/// Correction round 1 added a second cause of `complete:false` — a recorded
/// node whose path falls outside `items[].path`'s declared bounds is dropped
/// with its subtree (`src/helm.rs:310-322,372,390`) — and stated it in
/// `semantics.md:165-174` and `spec/ess/domains/helm.yaml:105-110`. It did not
/// state it in the operator-facing document, which still says every recorded
/// path gets an entry and still gives raising `limit` as the only thing a
/// `complete:false` means. This page is far smaller than the maximum `limit`
/// the same document publishes, and comes back incomplete with a recorded path
/// that has no entry.
#[tokio::test]
async fn a_values_page_under_its_limit_returns_one_entry_per_recorded_path() {
    // Two recorded paths. One of them cannot be represented inside the
    // declared `maxLength: 1024` for items[].path.
    let long = "k".repeat(1025);
    let secret = release_secret(&json!({"config":{"kept":1, long.clone(): "value"}}));
    let page = adapter(secret)
        .invoke(
            "helm_releases.values",
            json!({"namespace":"fixture","release":"api","revision":1,"limit":500}),
        )
        .await
        .unwrap();
    let items = page["items"].as_array().unwrap();

    // AMENDED by the coordinator, story:kubernetes-helm-release-reads.
    //
    // This case was written to assert two entries and a complete page. It
    // cannot be satisfied without contradicting a decision that was taken
    // deliberately: `items[].path` declares `maxLength: 1024`, that bound was
    // kept when the alternative "drop the bounds from the declaration" was
    // rejected, and a 1025-byte path has no representation inside it. Emitting
    // an entry anyway means either a result that fails the operation's own
    // published schema — which makes the host terminate the adapter child, the
    // hazard the bound exists to prevent — or a fabricated path, under which
    // two distinct recorded keys can collide. No finite bound makes "one entry
    // per recorded path" true; raising it to fit 1025 would satisfy this
    // number and not the property.
    //
    // The finding this case carries, pass 2 F2, is marked INFEASIBLE and its
    // own named fix is documentation: state the drop in the operator-facing
    // document. That fix was made — docs/local-kubernetes-cli.md no longer
    // promises an entry per recorded path, and a paragraph there now says a
    // dropped path is a second cause of an incomplete page and that raising
    // `limit` will not bring it back.
    //
    // So the case now asserts the behaviour that was decided rather than the
    // behaviour it wished for. It still earns its place: it pins that a
    // dropped path is reported through `complete`, and it goes red the moment
    // an unrepresentable path is silently emitted or silently swallowed.
    //
    // If the bound ever changes, change this case on purpose rather than
    // letting it drift.
    assert_eq!(
        items.len(),
        1,
        "one of the two recorded paths is 1025 bytes against a declared \
         maxLength of 1024, so it is dropped with its subtree and the page \
         carries the one representable entry; the page carried {}",
        items.len()
    );
    assert_eq!(
        page["complete"],
        json!(false),
        "a dropped path makes the page incomplete even though it is far under \
         the maximum limit of 500, which is the second cause of an incomplete \
         page that docs/local-kubernetes-cli.md now states beside the first"
    );
    assert_eq!(
        page["next_cursor"],
        json!(null),
        "both projections read a single object, so an incomplete page never \
         issues a cursor and raising the limit cannot recover the dropped path"
    );
}

// ---------------------------------------------------------------------------
// 3. A seventh site for "no projection concludes absence from an unestablished
//    shape" — the object's own metadata.
// ---------------------------------------------------------------------------

/// `spec/ess/domains/helm.yaml:47-50` states the rule for two fields of a
/// revision observation:
///
/// > `source_revision` is the Kubernetes resourceVersion of that object and is
/// > **absent when the provider omits it** ... `namespace` is the namespace the
/// > object reports, **falling back to the admitted selection only when the
/// > object omits it**, so no field of an observation is the caller's own
/// > input.
///
/// `helm::revision` reads both with `Value::as_str()` and treats `None` as
/// omission (`src/helm.rs:164,188-195`). A `metadata.namespace` or
/// `metadata.resourceVersion` that is present but not a JSON string is not an
/// omission, and the correction round's own class — a projection must not
/// conclude absence from a shape it has not established — is the reason the
/// six sites it did sweep were swept. These two were not.
#[test]
fn present_but_unestablished_object_metadata_is_not_read_as_an_omission() {
    let base = json!({"metadata":{"name":"sh.helm.release.v1.api.v1","namespace":"fixture",
        "resourceVersion":"11","labels":{"name":"api","owner":"helm","status":"deployed",
        "version":"1"}},"type":"helm.sh/release.v1"});
    assert_eq!(
        helm::revision(&base, "fixture", "api").unwrap().namespace,
        "fixture"
    );

    let mut numeric_namespace = base.clone();
    numeric_namespace["metadata"]["namespace"] = json!(123);
    let observed = helm::revision(&numeric_namespace, "fixture", "api");
    assert!(
        observed.is_err(),
        "the object reports metadata.namespace as 123 — it does not omit it — \
         and helm.yaml:48-50 admits the fallback only for an omission. The \
         observation instead carries namespace {:?}, which is the caller's own \
         input echoed back under a field the model says is observed",
        observed.map(|record| record.namespace)
    );

    let mut numeric_revision = base;
    numeric_revision["metadata"]["resourceVersion"] = json!(11);
    let observed = helm::revision(&numeric_revision, "fixture", "api");
    assert!(
        observed.is_err(),
        "the object reports metadata.resourceVersion as 11 — it does not omit \
         it — and helm.yaml:47-48 says source_revision is absent only when the \
         provider omits it. The observation instead carries source_revision \
         {:?}, which reports an absence nothing established",
        observed.map(|record| record.source_revision)
    );
}

// ---------------------------------------------------------------------------
// 4. The same class at the page level: a continuation whose shape was never
//    established becomes `complete: true`.
// ---------------------------------------------------------------------------

/// `contracts/helm/v1alpha1/semantics.md:154-155`:
///
/// > `complete` is **true exactly when the provider issued no continuation**.
///
/// `Kubernetes::releases` reads the continuation with
/// `value["metadata"]["continue"].as_str().unwrap_or_default()`
/// (`src/lib.rs:178-181`) and `Kubernetes::list` does the same
/// (`src/lib.rs:320-323`), so a continuation the response carries in a shape
/// this binding has not established becomes the empty token, which becomes
/// `next_cursor: null`, which becomes `complete: true`. This is the same
/// unsound completeness claim `semantics.md:68-72` refuses for a foreign
/// labelled object, one layer up: the page says it observed the whole
/// selection while the provider said it did not.
#[tokio::test]
async fn a_continuation_the_binding_cannot_read_does_not_report_the_page_complete() {
    let list = json!({
        "metadata":{"resourceVersion":"79","continue":12345},
        "items":[release_secret(&json!({"name":"api","namespace":"fixture","version":1}))]
    });
    let page = adapter(list)
        .invoke(
            "helm_releases.history",
            json!({"namespace":"fixture","release":"api","limit":50}),
        )
        .await
        .unwrap();
    assert_ne!(
        page["complete"],
        json!(true),
        "semantics.md:154-155 says `complete` is true exactly when the provider \
         issued no continuation. The provider issued metadata.continue = 12345 \
         and the page reports complete:true with next_cursor {}, so a caller \
         reading this history believes it observed every stored revision",
        page["next_cursor"]
    );
}

/// The origin half of the case above. `Kubernetes::list` — the path serving
/// `resources.list`, `endpoints.discover` and `hosts.discover` — carried the
/// identical read at the unit's base commit `5e4b0c4~1:src/lib.rs:161`, and
/// correction round 1 touched exactly that line when it factored both call
/// sites into the shared `continuation()` helper (`src/lib.rs:240-252`)
/// without establishing the continuation's shape. `contracts/service/v1alpha1`
/// owns the page envelope for that operation and `complete` means the same
/// thing there. This case is the pre-existing defect, reproduced against a
/// path the unit did not create, so the finding routes as `pre-existing`
/// rather than as this unit's.
#[tokio::test]
async fn the_same_unread_continuation_reports_a_resource_page_complete() {
    let list = json!({
        "metadata":{"resourceVersion":"79","continue":12345},
        "items":[{"metadata":{"name":"api-0","uid":"u-1","resourceVersion":"7"}}]
    });
    let page = adapter(list)
        .invoke(
            "resources.list",
            json!({"namespace":"fixture","kind":"pods","limit":50}),
        )
        .await
        .unwrap();
    assert_ne!(
        page["complete"],
        json!(true),
        "the provider issued metadata.continue = 12345 and resources.list \
         reports complete:true with next_cursor {}; this is the same read as \
         the Helm case above and it predates the unit",
        page["next_cursor"]
    );
}

// ---------------------------------------------------------------------------
// 5. A declared bound the correction round's enumeration did not reach.
// ---------------------------------------------------------------------------

/// Correction round 1 enumerated eleven declared bounds and enforced the four
/// that were unenforced. The enumeration covered `items[]` and `next_cursor`
/// — the strings the module emits — and stopped at the operation boundary.
/// `spec/adapter.json` also declares, for both release projections:
///
/// ```json
/// "revision": {"type":"integer","minimum":1,"maximum":2147483647}
/// ```
///
/// which is Helm's own `Version int` domain as the pinned source states it.
/// `ReleaseContent.revision` is a `u32` (`src/lib.rs:378`) and the only check
/// at dispatch is `args.revision == 0` (`src/lib.rs:554`), so a revision above
/// the declared maximum is admitted and turned into a provider request for
/// `sh.helm.release.v1.<release>.v<revision>` — a target outside the surface
/// the descriptor publishes.
#[tokio::test]
async fn a_release_revision_outside_its_declared_input_bound_is_refused() {
    // Beyond the declared maximum of 2147483647, inside u32.
    let beyond: u32 = 3_000_000_000;
    let secret = json!({
        "metadata":{"name":format!("sh.helm.release.v1.api.v{beyond}"),
                    "namespace":"fixture","resourceVersion":"11",
                    "labels":{"name":"api","owner":"helm","status":"deployed",
                              "version":beyond.to_string()}},
        "type":"helm.sh/release.v1",
        "data":{"release":stored(&json!({"config":{"kept":1}}))}
    });
    let result = adapter(secret)
        .invoke(
            "helm_releases.values",
            json!({"namespace":"fixture","release":"api","revision":beyond,"limit":500}),
        )
        .await;
    assert!(
        result.is_err(),
        "spec/adapter.json declares helm_releases.values input `revision` with \
         maximum 2147483647 and the adapter accepted {beyond}, returning {:?}",
        result.map(|page| page["provenance"]["resource"].clone())
    );
}
