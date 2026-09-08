# Independent specification review B

Verdict: **approve** for the selected textual specification scope.

Reviewed commit: `d1dc83f5d600816c699db07dd843f079ded0e72b`.
Reviewer: independent agent B, session `specs-review-b-20260909`.
Read-only source checkout: `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-review-b-20260909`.

I found no concrete contradiction that blocks the integrated selected semantic proposals or their declared ESS coverage. This is an independent agent judgment about specifications. It is not human approval, runtime conformance certification, or a claim that all proposed adapters are ready to advertise their profiles.

## Scope and independence

I reviewed the current shared semantic contracts, their native bindings, cross-document joins, explicit model coverage, and the separation between shared and provider ownership. The review includes:

- All 17 shared `contracts/**/semantics.md` documents: configured service v1alpha1/v1alpha2; operations; six auth families; logs, records and series; resource discovery and mediated routes; sessions, media and catalog.
- All 17 native `adapters/**/contracts/**/semantics.md` documents: Kubernetes reads/auth/discovery/routes/mutations/logs; GitLab and SQL reads; Atlassian documents; Loki logs; Prometheus series; Docker logs/mutations; Grafana discovery/routes; SIP dial; RTVBP session.
- The service compatibility and delegation rules, auth management and selected read-refresh-once binding, discovery and media compositions, native CQL grammar, adapter specification kinds v1/v2, shared/native ESS declarations and explicit UNMAPPED coverage statements.
- `AGENTS.md`, `docs/design.md`, shared and adapter indexes, the extraction/build boundary policy and its enforcement code, and the seven remaining original story records plus ownership closure context.
- Original immutable semantic and external-document reviews; original datasource reviewer reports; the ownership semantics recheck; current document/log/retry/media/vocabulary/index/evidence-precision records and provider-source supplements. Disposition tables were checked against present clauses and concrete cases rather than accepted as proof.

I did not inspect reviewer A's current report, findings or scratch. I made no source, planning or runtime changes and performed no integration/provider requests. All generated review output is inside the assigned scratch directory.

## Concrete semantic challenges

### Logs and bounded partial results

The shared complete/partial facts agree with native readers. Completion requires native exhaustion, no omitted occurrences and a final retained page; clipping an occurrence can coexist with entry completeness. Independent causes can coexist, and unknown source loss is not manufactured as zero.

For Loki, a source-cap result containing equal timestamps and duplicates retains occurrence multiplicity and a fixed order. With a 1,000-entry cap and 200-entry pages, the fifth page remains partial when native saturation prevents exhaustion proof. Replaying a token returns the same retained slice, does not renew the original 300-second retention, and requires current authority before disclosure. No timestamp-edge requery attempts to reconstruct omitted equal-timestamp entries. Independent local count and byte predicates can both contribute causes.

For Kubernetes, relative `since_seconds` and provider tail selection do not acquire an invented absolute upper bound. The fixed request and decoder have no continuation. A deliberate source-byte cutoff is distinct from transport interruption or malformed UTF-8; simultaneous provider, source and line ceilings keep their respective causes.

For Docker, multiplexed frames are not log-line occurrences, and per-channel buffers do not mix. Two provider messages may contain hundreds of decoded lines. If a verified driver mapping establishes source exhaustion but local output omits 100 decoded lines, the page is partial with the exact local omission count and no invented provider saturation. Without the native tail-unit proof, fewer LF lines than the requested tail cannot establish exhaustion. The contract explicitly refuses that unsupported inference.

### Scoped documents, cache correlation and CQL

The current bindings separate admitted metadata resolution from full body access. Confluence uses scoped v2 retrieval and requires current page/type/space/storage interpretation; optional source fields do not silently relax the independently selected endpoint filters. Contradictory returned fields refuse. Jira project resolution becomes a fixed scoped search rather than permitting full unscoped issue retrieval.

Cached body disclosure requires current scoped metadata and an equal non-null version correlation; current metadata and old body provenance remain distinguishable. Moves, deletion, revocation and version changes cannot use old cached membership as authority.

Native CQL parsing and trusted predicate composition preserve the complete caller expression, including OR/NOT structure. Key quoting is exact or refused. Continuation parsing constructs a fresh fixed-endpoint request using only an admitted cursor. Search/v2 cursor histories remain separate; immutable ancestry, depth, aggregate byte/node capacity and whole-chain eviction bound branching. Live CQL replay is explicitly distinct from immutable Loki page replay.

Body, result, encoded/decoded source and metadata/framing limits are separate. Whole Jira JSON objects are omitted rather than silently pruned or stringified; text truncation preserves valid UTF-8. Independent byte causes remain observable when more than one bound applies.

### Read refresh, authority and consumed budgets

Retry is explicitly selected through the combined native profile and compatible configured-service revision. It does not change the original profile or allow mutations, generic operations, streaming, static credentials, mediated/federated paths or ambiguous failures to acquire retry behavior.

A definite complete first 401 permits at most one coordinated refresh attempt and one second business dispatch. The second dispatch needs an acknowledged eligible successor and a fresh DispatchAdmission; it cannot replace credential material inside the terminal first admission. A second 401 is terminal. The original deadline and permission-call budget remain consumed across generations.

In particular, when a target permission check already used its once-per-target call slot under g0, refreshing to g1 invalidates the old evidence without granting another call. Only independently fresh g1 evidence can permit progress; otherwise the invocation refuses unavailable. Waiters may observe one shared publication but receive no extra token exchange, business dispatch or time window.

### Media, vocabulary, catalog and ownership

SIP dial is an ordinary admitted mutation. Close/offer/accept/reject/cancel are session messages, while DTMF/interruption are negotiated media messages. RTVBP authority redemption is capability state, not a new auth.evidence check. Establishment authority lifetime cannot extend the selected two-second live-data lease or the five-second teardown/accounting bound. Known dial effect and later composite readiness failure remain distinguishable. Hold/transfer are reserved and refused.

Reserved auth and Prometheus names are not advertised as selected support. Configuration remains explicitly deferred, removed locator address forms remain refused, and the index counts match the actual shared documents. Safe catalog metadata may describe unpopulated schema slots but cannot expose credential values or executable env/file/custody locators. Inventory does not imply a callable realization.

Exact provider selectors, decoding, mutation intents, recognition rules and mediated tuples now live with their adapters. The residual ownership issue in shared service compatibility is closed by the generic requirement and native reference. Shared roots do not require a specific provider vocabulary. Specification-only adapter roots do not need dummy runtime crates; the runtime workspace still contains the selected Kubernetes, GitLab and SQL adapters.

## Executed checks

Working directory for all checks was the exact review checkout. Scratch and TMPDIR remained inside `.local/spec-completion-20260909`.

- `git rev-parse HEAD` returned the reviewed commit. `git status --short` was empty, and `git diff --exit-code` returned 0 at the end of substantive review.
- Pinned `/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess --version` returned `ess 0.20.0`.
- Executed `ess specify validate --path <root>` and `ess specify compile --path <root> --format json` independently for every root below. All fourteen commands exited 0; validation and compiler outputs are retained in this scratch directory.

| ESS root | Validated files | Result |
|---|---:|---|
| `ess` | 14 | validate + compile pass |
| `adapters/atlassian/spec/ess` | 2 | validate + compile pass |
| `adapters/docker/spec/ess` | 2 | validate + compile pass |
| `adapters/grafana/spec/ess` | 2 | validate + compile pass |
| `adapters/kubernetes/spec/ess` | 4 | validate + compile pass |
| `adapters/loki/spec/ess` | 2 | validate + compile pass |
| `adapters/gitlab/generated/ess` | 3 | validate + compile pass |

- Independently enumerated the supplemental manifests with `jq` and checked each archive using `gzip -cd`, `sha256sum` and `wc -c`. All nine matched both declared uncompressed SHA256 and byte count: Loki AST/parser, six Atlassian CQL pages and Kubernetes core types.
- `sha256sum docs/evidence/evidence-precision-20260909/source-excerpts.txt` returned `37fae11fd3924227ac88e29b80c4c10f2023abc6192adb1ef6b1e6a64c1a2ecc`, matching the audit record.
- Shared semantic-document enumeration returned 17. Native semantic-document enumeration returned 17. Reviewed the runtime Cargo membership and the declared extraction boundary policy/code without executing a new Rust build.
- Managed lease was acquired and renewed for this session. One attempted heartbeat used the unsupported `session-heartbeat` spelling and exited 2 without mutation; the corrected `worktree hook heartbeat --session specs-review-b-20260909` succeeded. The lease is released as the report is handed off.

## Limits of approval

The manual cases above are textual reasoning, not executed provider, parser, decoder, coordinator, clock, custody or transport fixtures. No Rust build, repository/MSRV gate, live provider test, Python, runtime implementation or planning mutation was performed. The coordinator owns the combined repository gate on the same source commit; this report does not borrow its result.

The ESS checks establish valid declarations and lowering, not enforcement of prose. Eleven declared entities are explicitly distinguished from value shapes and from the still-UNMAPPED persistent ownership/cardinality/lifecycle graph. Native query containment and strict bounded reader behavior require the stated native bindings and conformance evidence before advertisement. For example, the Prometheus proposal's metric-prefix/label-matcher scope must satisfy the shared native-containment gate; this review does not certify an existing PromQL containment implementation or a finished broader scoped binding. Docker's driver-tail mapping has the same explicit proof-before-support character.

Broader adapter codecs, native schemas and fixtures, source/license refresh where required, timed media cutoff, independent extraction/build/package proof and declared durable entity relations remain prerequisites for the respective implementation slices. Their honest deferral is compatible with approval of these selected textual decisions; it must not be reported as implementation completion.

## Structured findings

```json
[]
```
