---
format: aep.planning-md/1
id: review-result:adversary-helm-reads-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — Helm release reads, unit 1
relations:
- reviews: story:kubernetes-helm-release-reads
revision: 1
---
```
unit: 1 — story:kubernetes-helm-release-reads, worktree cv2-helm-reads-20260912 at 5e4b0c4 + 1 untracked test file
verdict: red
cases: executed 20→25, red 5
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/cv2-wave-b-20260912/adversary-1/{tmp,vendor}
needs-coordinator: no
```

## 1. `git --no-pager diff --stat`

```
 .../kubernetes/tests/helm_contract_adversary.rs    | 268 +++++++++++++++++++++
 1 file changed, 268 insertions(+)
```

One path, a test file. No implementation file touched.

## 2. Cases added

All five red now. Red output captured when each was written, run alone, before any suite run.

**2.1 `manifest_content_digest_is_sha256_over_the_document_text` (:105)** — asserts the digest `semantics.md:87` and `docs/local-kubernetes-cli.md` both promise. Reported `53ff35ffbaff05ebfd1fc2dba937d5c7c886ee840a67c48516d3be11afeba489` (SHA-256 over the JSON encoding); promised `656a791a9d5a3a64b549aa27371de9ec7a4b1eff753529affc9d6fef910f89bd` (SHA-256 over the text). Both reproduce outside Rust with `sha256sum`. The `bytes` field the same sentence promises does hold, which fixes which bytes "that text" names.

**2.2 `a_deeply_nested_recorded_value_stays_inside_the_published_path_bound` (:142)** — `items[].path` declares `maxLength 1024`; the projection bounds nothing. `InvalidInput: value does not match its declared schema`.

**2.3 `an_empty_recorded_key_stays_inside_the_published_path_bound` (:174)** — `items[].path` declares `minLength 1`; an empty recorded key projects to the empty string.

**2.4 `the_cli_document_does_not_contradict_its_own_limit_examples` (:199)** — `docs/local-kubernetes-cli.md` states "`limit` is between 1 and 100." and gives request examples at `(197, 200)` and `(198, 200)`.

**2.5 `a_stored_payload_that_is_not_a_release_record_is_not_reported_complete` (:246)** — a stored body of `null` projected as `([], true)` instead of refusing.

`cargo fmt` and `cargo clippy -- -D warnings` are both exit 0 with the file present, so the red is the assertions and not the gate.

## 3. The suite, after the cases existed

| target | executed | result |
|---|---|---|
| unittests src/lib.rs | 5 | ok |
| unittests src/main.rs | 0 | ok |
| **tests/helm_contract_adversary.rs** | **5** | **FAILED, 0 passed / 5 failed** |
| tests/local_runtime.rs | 12 (+4 ignored) | ok |
| tests/provider.rs | 3 | ok |

`error: 1 target failed`, exit 101. Deselecting the adversary file gives 20, matching the implementor's reported count, so executed 20→25.

CLI-journey lane separately: 4 passed, exit 0, unchanged, with `CONNECTORS_TEST_CLI` set. Without that variable it is 3 red on `built production CLI required` (cli_journey.rs:128) — an environment prerequisite, not a defect.

## 4. Findings

Commit covered: `5e4b0c4`. Base for origin: `5e4b0c4~1`; `src/helm.rs`, the Helm contract, the ESS domain and the Helm rows of the CLI doc do not exist there, so every row is `introduced`.

1. `semantics.md:87` and the CLI doc promise "a SHA-256 over that text"; `helm.rs:340` computes a SHA-256 over the JSON encoding of the text. The unit's own ESS model says "canonical JSON string" and matches the code — two of the unit's three documents disagree with the third and with the implementation, in one commit. Reaches: every `helm_releases.manifest` result; the CLI doc sells this digest as the comparison mechanism. NEEDS-CHANGE.
2. `docs/local-kubernetes-cli.md:197-198` document `limit 200`; line 200 states 1 to 100. Code and contract allow 1 to 500 for the projections, so the prose is the wrong half. At the base every example was 50. Reaches: an operator following the documented invocation table. NEEDS-CHANGE.
3. `recorded_values` (`helm.rs:247-251`) concatenates provider-owned keys with no bound against a declared `maxLength 1024`, and the host terminates the child on validation failure (`process.rs:248-253`), so the failure mode is a killed adapter rather than a bounded refusal. Reaches: *nothing found* — built from 180-character keys; realistic Helm value trees stay under ~300 characters. INFEASIBLE.
4. Same declaration carries `minLength: 1`; an empty map key projects to the empty path. Reaches: *nothing found*. INFEASIBLE.
5. `semantics.md:68-72` argues a foreign labelled object must refuse the page rather than be skipped, because a skipped row would leave `complete:true` claiming a history it did not observe. One layer down that reasoning is not applied: a payload decoding to `null`, an array, a string or a number reports `items:[], complete:true`. Reaches: *nothing found* — Helm's writer cannot produce it. INFEASIBLE.
6. `value_digest` is an unsalted SHA-256 over the canonical JSON of the value, so a guessed recorded literal is confirmable offline with one `sha256sum`. `semantics.md:95-97` documents the equality property but nothing says the projection is a verification oracle for the credentials the contract says it exists to protect. No case added, because asserting non-reproducibility would contradict a property the contract states on purpose. Reaches: every `helm_releases.values` result under `redacted_content`. CONFIRMED.
7. `provider-sources.md` heads a table "Release fields this binding reads" and lists ten; the binding reads two — `body["config"]` and `body["manifest"]`. The body's `version`/`name`/`namespace` are never cross-checked against the labels the provenance is built from, so a body disagreeing with its object's labels is served under the labels' identity. CONFIRMED.
8. `Revision.namespace` (`helm.rs:178`) is the requested namespace, not the observed one: `helm.rs:147-148` only rejects a mismatch. Same class: `source_revision` copies `metadata.resourceVersion` unbounded into a field the schema caps at `maxLength 512`. Reaches: *nothing found* — a namespaced list response always carries `metadata.namespace`. INFEASIBLE.

Named fixes, not applied: (1) change the contract and CLI doc to "over the canonical JSON of that text", or change `helm.rs:340` to digest `text.as_bytes()` — pick one and make `helm.yaml` agree; (2) restate the limit sentence as 1 to 100 for the collection reads and 1 to 500 for the two projections; (3)+(4) bound `path` in `recorded_values` and refuse with `capacity`/`invalid_input`, or drop the bounds from the declaration; (5) refuse a decoded body that is not a JSON object; (8) carry the observed `metadata.namespace`.

## 5. Attacked, could not break

- **Provenance naming a reconstructed object.** The reconstructed name is provably the observed name on every success path (`helm.rs:162`, `lib.rs:224`). No case exists.
- **The fourteen pinned sources.** All 14 uncompressed SHA-256 values and byte lengths reproduce exactly from the archived `.gz`; roughly 50 cited lines say what is claimed, in both the v4.3.0 and v3.22.0 archives.
- **v3/v4 agreement on every field read.** Checked field by field; they agree.
- **`resource_kinds` not widened.** No helm path consults it; the fixture configures `pods`+`endpointslices` and all four reads work.
- **Disclosure through any other channel.** `path`, `kind`, `bytes`, `index`, every error message and the provenance strings carry no stored scalar or manifest byte in any probe.
- **Gzip expansion.** `.take(MAX_BODY_BYTES+1)` caps decompression before the JSON parse, behind a 4 MiB transport cap — no bomb.
- **Both paging routines.** `complete:true` is never returned over a truncated projection.
- **Cursor binding.** A history cursor is unreadable as a status cursor.
- **The three-outcome discrimination.** Scope refusal (`forbidden`, 0 requests), RBAC denial (`forbidden`, 1 request) and empty history (success, `complete:true`, 0 items) are all distinct.
- **Boundaries.** limit 0 / 101 / >u16, revision 0, release names of 53 and 54 bytes, `Api`, `a..b`, `a_b`, a non-canonical `02` version label, a tenth status string — all refuse correctly.

## 6. Paths written outside the worktree

`~/.cache/cv2-wave-b-20260912/adversary-1/tmp/` and `.../vendor/`, 108K total. Inside the worktree, `target/debug/connectors` was built because the CLI-journey lane requires `CONNECTORS_TEST_CLI`.

```findings
- file: adapters/kubernetes/contracts/helm/v1alpha1/semantics.md
  line: 87
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the contract and docs/local-kubernetes-cli.md promise a SHA-256 over the manifest document text while helm.rs:340 digests the JSON encoding of that text, and the unit's own ESS model states the third thing, so a consumer cannot reproduce the digest the documented comparison workflow tells it to use.
- file: docs/local-kubernetes-cli.md
  line: 200
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the request table added at lines 197-198 documents limit 200 for the two projections and the sentence two lines below states limit is between 1 and 100, so the operator-facing document forbids its own example.
- file: adapters/kubernetes/src/helm.rs
  line: 249
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: recorded_values concatenates provider-owned keys with no bound while the published output_schema declares path maxLength 1024, and the host terminates the local runtime child when a result fails that validation; I built the 1085-character path and can show no chart that produces one.
- file: adapters/kubernetes/src/helm.rs
  line: 247
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: an empty recorded map key projects to the empty path, which the published output_schema rejects under minLength 1; legal JSON and YAML but no caller shown.
- file: adapters/kubernetes/src/helm.rs
  line: 281
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a stored payload decoding to a JSON scalar, array or null is reported as items empty with complete true, which is the same unsound completeness claim the contract refuses at semantics.md:68-72 for a foreign labelled object; Helm's writer cannot produce such a payload.
- file: adapters/kubernetes/src/helm.rs
  line: 299
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: value_digest is an unsalted SHA-256 over the canonical JSON of the value, so a guessed recorded credential is confirmable offline with one sha256sum, and nothing in the contract says the projection is a verification oracle for the values it exists to protect.
- file: adapters/kubernetes/contracts/helm/v1alpha1/evidence/20260912/provider-sources.md
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the table headed "Release fields this binding reads" lists ten fields of which the binding reads two, and the body's own name, version and namespace are never cross-checked against the labels the provenance is built from.
- file: adapters/kubernetes/src/helm.rs
  line: 178
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Revision.namespace carries the requested namespace rather than the observed one because helm.rs:147-148 only rejects a mismatch, so an object without metadata.namespace is reported under the caller's own input; a namespaced list response always carries it.
```
