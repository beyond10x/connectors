# Independent final semantic review A

Verdict: **approve** for the entire selected specification scope at
`d1dc83f5d600816c699db07dd843f079ded0e72b`. No remaining blocker or new finding.
SCA-01 is resolved. This is an independent agent judgment, not human approval or
runtime conformance evidence.

Reviewer/session: `specs-review-a-20260909`.
Managed read-only checkout:
`/home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-review-a-20260909`.
The initial `stable-core-review.md` at
`eb0815961e6ffee54c50f91f37eff2bbe11811c0` remains immutable. Its SHA-256 remains
`a05798deba9a655ff63db874301d9968de2d0947dc4b8edbf9dbe99e6b4e5dfc`.

## SCA-01 correction and ownership residual

**SCA-01 (P2, pre-existing): resolved.** The initial contradiction remains
documented against its original source. At the final commit,
`contracts/catalog/v1alpha1/semantics.md:140` allows safe auth-profile identifiers
and configuration requirements while explicitly excluding actual runtime
credential locations and values from bundles, descriptors, indexes and catalog
outputs. It requires refusal before publication or serving. The concrete
environment-reference counterexample is now a refusal scenario at line 163.
This agrees with the no-credential-address rule at line 130 and the safe
profile/descriptor projections. An unpopulated protected configuration slot is
distinct from a deployment's private environment, file or custody locator.

**ACO-S-04: remains resolved.** At
`contracts/service/compatibility.md:159`, shared compatibility owns the coverage,
strict-reader and failure rules, while exact target codecs and native scope
interpretation belong to the linked adapter. The seven-field Kubernetes tuple,
explicit all-namespaces Service rule and native SSAR request remain in
`adapters/kubernetes/contracts/auth/v1alpha1/semantics.md`. The immutable ownership
report describes an earlier snapshot and was not rewritten. I did not rerun the
completed ownership/layout audit or inspect the other current final reviewer's
report.

## Whole-scope coverage

The initial review is reused for unchanged stable clauses; the final diff and
affected joins were inspected directly. Coverage includes the following units.

| Unit | Review and conclusion |
|---|---|
| Service v1alpha1/v1alpha2, compatibility and delegation | Strict reader/version refusal, declared payload/profile selection, lookup and disclosure precedence, canonical delegated leaf binding, nonce/approval acknowledgement fences, single dispatch and independent effect knowledge remain coherent. Extended semantics are not implied support in the implemented legacy reader. |
| Operations, idempotency, native mutations | Stable namespace/fingerprint, fixed intent, approval spend, original attempt observation, unknown-outcome quarantine and current replay disclosure remain coherent. Kubernetes restart retains canonical positive resourceVersion and exact UID/version intent without refetch/rebase. Docker keeps exact daemon/full-ID targeting, observed name/label limits, natural start/stop versus non-idempotent restart. SIP separates confirmed establishment from ready-handle delivery and preserves applied/unknown effects through later failure. |
| Auth and management | Profile/acquisition/custody/capability/evidence/connection owners and management target distinctions remain explicit. Current identity, immutable generation, one exchange authorization, guarded publication, local revocation and exact permission budgets are separate facts. Static configuration, anonymous and mediated bindings do not manufacture managed acquisition or a credential identity. |
| Read refresh retry, F15/E05 | Reviewed the native combined-profile opt-in and RR01–RR14 cases against the unchanged binding. A complete definitive first 401 may trigger one source refresh; only its acknowledged, still-current exact successor can receive a new dispatch admission. Two business requests share the original deadline and permission ledger. Earlier consumed target/check slots cannot be silently replenished, and successor evidence may therefore force refusal. No mutation, generic engine, mediated, pagination or implicit activation retry is introduced. |
| Document admission, F14 | Read the complete Atlassian document and CQL contracts and all 32 document/10 CQL manual cases. Fixed-origin scoped collection reads, exact returned identity/current membership, optional PageBulk subtype and storage marker rules, fresh cache metadata with original body provenance, lossless complete Jira fields, scalar-aligned storage prefixes and independent body/result-byte causes are coherent. Separate provider observations explicitly do not claim atomic prevention of a later move. |
| CQL continuation and limits | Challenged grouped predicates with OR/NOT, quoted ORDER BY and escaped configured keys; trusted page/space conjunction remains outside the preserved caller predicate. Candidate metadata never grants body access. Bulk body enumeration must finish within four body calls plus one search call and aggregate bytes/deadline. Search and body cursor lineages are separate; replay creates immutable sibling branches, all branches share chain capacity/expiry, and failed body/disclosure work cannot advance a public cursor. |
| Log continuation, F12 | Read the shared log contract and complete Loki, Kubernetes and Docker bindings plus 23/8/10 manual cases. Loki retains multiplicity and immutable offsets without timestamp requery; saturated source limits remain terminal partial on the final retained page. Kubernetes uses finite native text order, exact local byte cutoff and conservative tail exhaustion. Docker distinguishes driver tail units from decoded LF occurrences; multi-line messages can cause local count omission with terminal page_limit and no cursor. Unverified native exhaustion refuses. Independent simultaneous source/count/result/line causes and unknown versus exact loss totals are stated. |
| Sessions, media, E23/E24 | Read shared lifecycle/media, native SIP/RTVBP and composition joins and the session model/verification limits. Ordinary dial/list operations are separate from duplex offer/accept/reject/cancel/close and media signal/interrupt messages. RTVBP one-use redemption is inbound-verifier capability state, not a new evidence name. Establishment expiry differs from the live lease; current authority, zero post-cutoff data drain, first terminal retention and bounded local accounting remain coherent across both bindings. |
| Discovery and mediated composition | Read shared scope/coverage/publication/history/revalidation and native Kubernetes/Grafana discovery/auth/route owners. Incomplete or denied coverage does not invent absence; withdrawn incarnations remain terminal; a narrower selection cannot withdraw a former scope. Observation, parent credential and private route revisions remain separate. Same-target revalidation cannot repoint an old child. Exact proxy permission differs from list permission. Private mediation requires a constructed same-process composition, one hop and no direct fallback. |
| Records, series and implemented native slices | Read shared records/series, Prometheus range semantics and Kubernetes/GitLab/SQL native read owners. Provider-native representation and continuation stay native. SQL text values preserve native type/value meaning with explicit bounded truncation and independent local execution deadline. Proposed series/query/route bindings require their selected containment/codec/limit guarantees before support can be advertised. |
| Supported vocabulary, E27 | Checked auth profile/capability, media, Prometheus, resource discovery and compatibility joins. http_signing and oauth2_password are reserved/refused; hold/transfer and instant/label profiles are unadvertised/refused; address is removed from resource locator vocabulary and remains under endpoint discovery. Historical names and enum presence do not create support. |
| Index and evidence, E22/E13 and E15/E16/E17/E18/E25/E26/E31/E33 | Compared root indexes with native dependency tables and composition records. Kubernetes/Docker include static_config acquisition plus connection/custody; media includes records and acquisition; RTVBP evidence is none. Examples distinguish deployment choices from selected native/shared numeric bounds. Native sources are identified as vendor or repository-authored, partial-source selection does not claim a wholly GET-only source, source digest prefixes are not usable pins, and historical characterizations/manual traces/ESS compilation are not called runtime tests. |
| Catalog, specification kinds, persistence and extraction | Rechecked catalog privacy and singular generic versus mutation/paged profiles; read adapter v1/v2 authoring semantics and extraction ownership. Native mappings, provider limits and models travel with each owner. Exact shared release/source/license packaging and standalone builds are prerequisites. Design §31 gives logical persistence owners without claiming undeclared Connection/Acquisition/custody/discovery relations are modeled. |
| ESS coverage | Initial shared declaration/lifecycle review plus final datasource_reads, discovery, sessions and all native authored model files. The shared root has 13 domains, 143 types, 11 entities and 30 commands. Native roots type selections/intent/recognition/permission values and introduce no provider entities or commands. Current authority, numeric/cross-field predicates, native parsing, lossless decoding, exact clocks, field assignment, atomic persistence and transport behavior are explicitly UNMAPPED rather than falsely executable proof. |

Historical F01–F15 and E01–E33 intake and relevant immutable correction/review
records were used as context. I independently read the current semantic owners
and evaluated concrete cases; disposition tables alone did not establish closure.
No new counterexample survived comparison with the current selected rules.

## Executed checks

All checks ran in the exact managed checkout above. Tracked source and planning
were read-only. Source inspection used `rg`, `git diff`, `sed`, `cat` and `nl`.

Pinned executable:
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess` (0.20.0).
With `TMPDIR="$PWD/.local/tmp"`, executed separately for each row:

```text
ess specify validate --path ROOT
ess specify compile --path ROOT --format json --out .local/spec-completion-20260909/final-NAME-ir.json
```

| NAME / ROOT | Validate | Compile | Final IR SHA-256 |
|---|---|---|---|
| shared / ess | exit 0; 14 files valid | exit 0 | 48ada7ee575f6f69f3ac979f18adb3c4dec90cd7decf127de1dc521aa6e99f6a |
| atlassian / adapters/atlassian/spec/ess | exit 0; 2 files valid | exit 0 | 230cf351782cde13f59fe09d451d84b382180861ab7438c509b161c02a5ee3a9 |
| docker / adapters/docker/spec/ess | exit 0; 2 files valid | exit 0 | 55559f6721c6fe910f0dc74ce00a2194d9d201933c66f8c4af58af9230a3b888 |
| grafana / adapters/grafana/spec/ess | exit 0; 2 files valid | exit 0 | 3995ee1bda836b059c8feefbd33ddff2dccff328f1d6e934a7cd451c5f6d5359 |
| kubernetes / adapters/kubernetes/spec/ess | exit 0; 4 files valid | exit 0 | 6677a30d0b63278d3771420cb05eedfe25d30adb04a0fc76fc641e9301237391 |
| loki / adapters/loki/spec/ess | exit 0; 2 files valid | exit 0 | 70baafbf6eb852679d7a3a82f9310d17d7563002093e92af1bc594aef8cbda02 |

`cmp shared-compiled.json final-shared-ir.json` in scratch exited 0.
`git diff --name-only eb0815961e6ffee54c50f91f37eff2bbe11811c0 HEAD -- ess adapters/*/spec/ess`
was empty. `jq` structural counts confirmed each native root has no entities or
commands; shared counts are given above. Atlassian/Loki final IR hashes match
their native model-check records.

For the nine new retained source archives (six Atlassian CQL pages, two Loki
parser/AST sources and Kubernetes core types), executed
`gzip -cd ARCHIVE | sha256sum`. Every resulting digest matched the corresponding
native provider-source-hashes manifest. Inspected the pinned Loki root-interface
and Kubernetes PodLogOptions source passages directly. The original archived
Confluence v2 OpenAPI decompressed to its recorded digest
`451377c5a598ee8155acc11b611404f309bed4a4292ea87f88ed3bfed38fa0a8`.
An independent `jq -e --slurpfile facts ...` equality check compared the selected
query parameters and full PageBulk/BodyBulk/BodyType extracts to that original
source: `true`, exit 0. These checks establish retained bytes and extracted facts,
not provider runtime behavior or completeness of source/license adoption.

Own managed session acquisition/heartbeat succeeded. One initial attempted
`worktree session-heartbeat` spelling returned exit 2 without a lifecycle change;
the correct `worktree hook heartbeat --session specs-review-a-20260909 --path ...`
returned exit 0. No other lease was changed. Final HEAD and clean tracked status
were checked before handoff; own lease is released separately after this report.

## Limits of approval

Approval covers the selected textual semantics, declared model coverage and
truthfulness of the proposal/implementation boundary. It does not authorize a
runtime rollout or assert every future native schema, parser, proof verifier,
transport, storage relation/backend, source/license package or standalone release
is already complete. Explicitly named pre-advertisement and entity-modeling gates
remain gates. Deferred features remain deferred.

No Python, new executable program, runtime/provider test, integration call, new
model reducer, planning mutation, tracked fix, commit or full/cold Rust build was
performed. The coordinator's full repository/MSRV gate is separate evidence; this
verdict does not claim to have run it or the other reviewer's checks. ESS commands
validate declarations and compile IR; they do not execute the manual scenarios,
real concurrency, provider responses, clocks or durable transactions.

```findings
[]
```
