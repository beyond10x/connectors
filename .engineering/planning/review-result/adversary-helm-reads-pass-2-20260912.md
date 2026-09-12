---
format: aep.planning-md/1
id: review-result:adversary-helm-reads-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — Helm release reads, unit 1
relations:
- reviews: story:kubernetes-helm-release-reads
revision: 1
---
```
unit: 1 — story:kubernetes-helm-release-reads, worktree cv2-helm-reads-20260912 at e5938a6 (5e4b0c4 + correction round 1); origins diffed against 5e4b0c4~1
verdict: red
cases: executed 30→36, red 6
origin: introduced 6, pre-existing 1, undecided 0
wrote-outside-worktree: 4 paths under ~/.cache/cv2-wave-b-20260912/adversary-2/
needs-coordinator: yes
```

## 1. Diff

One path, `adapters/kubernetes/tests/helm_contract_adversary_pass2.rs`, 378 lines. No implementation file touched. Pass 1's file is byte-identical and all five of its cases still pass. `cargo fmt` clean.

## 2. Cases added — all six red, each captured running alone

**C1** `the_manifest_digest_is_reproducible_by_sha256sum_over_the_stored_document` — `semantics.md:94-96` and `docs/local-kubernetes-cli.md:237-239` promise a byte length over the document text; document 0 of a Helm-shaped manifest is 66 bytes and the projection reports 65.

**C2** `a_values_page_under_its_limit_returns_one_entry_per_recorded_path` — limit 500, two recorded paths, page carries 1.

**C3** `present_but_unestablished_object_metadata_is_not_read_as_an_omission` — the object reports `metadata.namespace` as `123`, which is not an omission, and `helm.yaml:48-50` admits the fallback only for an omission. The observation carries the caller's own input echoed back under a field the model says is observed.

**C4** `a_continuation_the_binding_cannot_read_does_not_report_the_page_complete` — the provider issued `metadata.continue = 12345` and the page reports `complete:true` with a null cursor, so a caller believes it observed every stored revision.

**C5** `the_same_unread_continuation_reports_a_resource_page_complete` — the origin half of C4, on the pre-unit path.

**C6** `a_release_revision_outside_its_declared_input_bound_is_refused` — `spec/adapter.json` declares `revision` maximum 2147483647 and the adapter accepted 3000000000, issuing a provider request for it.

## 3. Suite

lib 10 ok; pass-1 adversary 5 ok; **pass-2 adversary 0 passed / 6 failed**; local_runtime 12 ok + 4 ignored; provider 3 ok. `error: 1 target failed`, exit 101. Before is 30, from a run with the pass-2 target deselected. The 4 ignored are the CLI journeys, ignored by attribute; `CONNECTORS_TEST_CLI` does not un-ignore them and they additionally need a disposable Secret Service.

## 4. Findings

F1 `helm.rs:414` — `emit` digests and counts `text.trim()`. Document 0 of a Helm-shaped manifest is 66 bytes; the projection reports 65 and a digest over the text minus its trailing newline. Reaches: **the documented workflow.** `docs/local-kubernetes-cli.md:239` tells the operator `sha256sum` over the document reproduces it, and Helm's writer terminates every document with a newline before the next separator, so no document of any Helm manifest reproduces. CONFIRMED, introduced.

F2 `docs/local-kubernetes-cli.md:235` — correction round 1 added a second cause of `complete:false`, a dropped recorded path, to the contract and the model but not to the operator-facing document, which still says `values` returns one entry per recorded path and still gives "larger than limit" as the only cause. Reaches: *nothing found* — no supplied-values structure reaching the 1024-byte bound was shown. The document is wrong regardless, and an operator raises the limit and never learns data was lost. INFEASIBLE, introduced.

F3 `helm.rs:164` — a `metadata.namespace` or `metadata.resourceVersion` present but not a JSON string is read as an omission, so the observation carries the caller's input under a field the model says is observed. A seventh site of the class the round swept. Reaches: *nothing found* — the Kubernetes API always serialises both as strings. INFEASIBLE, introduced.

F4 `lib.rs:180` — a `metadata.continue` the binding cannot read becomes the empty token and the page reports `complete:true`, contradicting `semantics.md:154-155`. Reproduces on `resources.list`, whose read is unchanged from the pre-unit base, and correction round 1 rewrote that exact line into the shared helper without establishing the shape. Reaches: *nothing found* — a non-conforming provider only. INFEASIBLE, **pre-existing**.

F5 `lib.rs:554` — `revision` is declared with maximum 2147483647 and the dispatch checks only `revision == 0`, so a larger value is admitted and turned into a provider request. This is the declared bound the round's eleven-item enumeration missed, by scoping itself to emitted strings. Reaches: *nothing found* — the host validates input against `input_schema` at two sites, which keeps it off the CLI path. INFEASIBLE, introduced.

F6 `semantics.md:102` — the unsalted-guess warning is written about `value_digest` alone. `content_digest` has none, and `ManifestDocumentProjection.bytes` publishes the exact length of a rendered document whose template is public and whose only unknown is the injected secret — a narrower guess space than the field the warning covers. The CLI document does say both digests are unsalted; the contract and the model do not. CONFIRMED, introduced.

F7 `helm.yaml:114` — the model calls `index` the document's position in the stored text while the code assigns its position in the emitted list, which diverge once a whitespace-only document sits between separators. Reaches: *nothing found* — Helm's own splitter drops empty entries before storage. INFEASIBLE, introduced.

Named fixes, not applied: F1 digest the untrimmed bytes, or say "trimmed" in all four artefacts and drop the reproduction promise. F2 state the drop beside line 252. F3 and F4 refuse a present-but-unestablished value rather than defaulting it. F5 bound `revision` at the declared maximum.

## 5. Attacked and could not break

- **The drop-and-complete question the coordinator asked.** No path where a drop happens and `complete` reports true. The subtree really is dropped and the flag really is false.
- **The eleven declared bounds.** Every bound in the specification enumerated programmatically and traced to code. Every emitted-string bound is enforced; only the input `revision` maximum is not, which is F5.
- **Two digests, no third**, across all seven operations.
- **The shared continuation helper's effect on the three pre-existing operations.** No regression: the SDK already refused any token over 16384 at the base, untouched by this unit, so a longer cursor never round-tripped. The check converts a result the host would reject into a bounded refusal.
- **The `omitempty` claim**, verified against the unit's own vendored sources in both pinned Helm lines. Spot-checked citations all resolve to the stated content.
- **The identity cross-check.** An explicit null, a non-string and a float version all take the refusing branch; only genuine absence passes, which is what the pinned `omitempty` licenses.
- **Disclosure.** No path emits a stored scalar or manifest byte.
- **The acceptance statement**, driven directly.
- **The 500-node ceiling**, raised then withdrawn: no realistic supplied set exceeds it.

```findings
- file: adapters/kubernetes/src/helm.rs
  line: 414
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    manifest_documents digests and counts a trimmed document, so the sha256sum
    reproduction promised identically by the contract, the CLI document and the
    model fails for every document of every Helm-written manifest, each of which
    ends in the newline before the next separator.
- file: docs/local-kubernetes-cli.md
  line: 235
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    correction round 1 added a second cause of an incomplete page, a dropped
    recorded path, to the contract and the model but not to the operator-facing
    document, which still says values returns one entry per recorded path and
    still gives a too-large page as the only cause, so an operator raises the
    limit and never learns data was lost.
- file: adapters/kubernetes/src/helm.rs
  line: 164
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    a namespace or resourceVersion that is present but not a JSON string is read
    as an omission, so the observation carries the caller's own input under a
    field the model says is observed - a seventh site of the class the round
    swept.
- file: adapters/kubernetes/src/lib.rs
  line: 180
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: >-
    a continuation token the binding cannot read becomes the empty token and the
    page reports itself complete, contradicting the contract; it reproduces on
    resources.list whose read is unchanged from the pre-unit base, and the
    correction round rewrote that exact line into a shared helper without
    establishing the shape.
- file: adapters/kubernetes/src/lib.rs
  line: 554
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    the two projections declare a revision maximum and the dispatch checks only
    for zero, so a larger value is admitted and turned into a provider request;
    this is the declared bound the eleven-item enumeration missed by scoping
    itself to emitted strings, and the host's own input validation keeps it off
    the CLI path.
- file: adapters/kubernetes/contracts/helm/v1alpha1/semantics.md
  line: 102
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the unsalted-digest warning is written about the value digest alone while the
    content digest is presented as a reproducibility feature, and the manifest
    projection publishes the exact length of a rendered document whose template
    is public - a narrower offline guess space than the field the warning covers.
- file: adapters/kubernetes/spec/ess/domains/helm.yaml
  line: 114
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    the model calls index the document's position in the stored text while the
    code assigns its position in the emitted list, which diverge once a
    whitespace-only document sits between separators; Helm's own splitter drops
    empty entries before storage.
```
