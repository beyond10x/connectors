---
format: aep.planning-md/1
id: review-result:discovery-b-final-20260908
kind: review-result
status: active
title: Discovery coverage and composition reviewer B final approval
relations:
- reviews: story:contracts-discovery-coverage
- reviews: story:contracts-host-composition
revision: 1
---
# Independent reviewer B — final discovery coverage and composition review

Verdict: **approve the F13/E03 semantic checkpoint**. Residual findings: **0 P0, 0 P1, 0 P2, 0 P3**. This is approval of the proposed contracts and evidence, not an assertion of runtime support or rollout authorization.

Scope: story:contracts-discovery-coverage and story:contracts-host-composition; baseline 0e4a8c11885e7008b2825cdcd0d2855cd241d9b4. Discovery-profiles E07/E14/E32 and persistence consolidation remain outside closure. Root was the sole tracked editor. Other reviewer outputs, dispositions, mutable AEP and checkpoint/closure bookkeeping were excluded.

## Final residual closure

| Finding | Final disposition |
|---|---|
| DC-B-R01 — terminal withdrawal during partial merge | Closed. Resource semantics lines110–119 give withdrawn history explicit precedence over stale retention. A withdrawn incarnation cannot become stale or observed during denied/capped/failed scans; reappearance receives a new id even in a partial scan. The type-change scenario at155 now conditions withdrawal on complete comparable absence. Traces DC-T31/32/35 match. |
| DC-B-R02 — provider versus publication deadline | Closed. Resource semantics129–133 pin separate original provider-work and outer execution/publication/response deadlines. The 15-second provider ceiling is nested inside the ordinary 20-second total ceiling; a smaller admitted remaining bound wins. Provider expiry stops sends and discards late responses, while already trustworthy positives may publish only before the outer deadline. PublicationFacts.deadline_current is explicitly the outer deadline. Unknown acknowledgement follows same-attempt observation, never reset/rescan. Traces DC-T19–21 match. |
| DC-B-R03 — all-namespaces Service authorization target | Closed by an explicit coordinated F08 clarification rather than reserving the feature. Auth evidence124–128 selects exactly (list, empty group, v1, services, empty namespace/name/subresource) only for the admitted empty configured namespace-list mode; finite per-namespace mode is distinct. Invalid/missing/caller-empty selection, denial or empty results cannot activate it. Cache/allow evidence cannot substitute across modes; all pages retain the existing object/call/byte/deadline ceilings. Resource/Kubernetes/compatibility text agrees, and traces DC-T15–18 cover the distinctions. The old pinned design and independently inspected official ResourceAttributes reference support the namespace meaning; see namespace-reference-note.md. No live-cluster or provider-exhaustion claim follows. |

## Initial findings

All nine initial DC-B-01–09 findings are closed within this bounded scope:

- Exact logical coverage keys include qualified source/declaration/profile, membership/selection, interpretation/projection and disclosure coordinates. Absence comparison requires equal full selection and exact comparable partition scope. Narrower or differently filtered scans do not delete older-scope facts; current policy exclusion remains immediate and distinct from disappearance.
- Permission coverage and provider collection coverage are separate. Only completely exhausted, coherent, successful comparable partitions supply negative evidence. Unknown preflight authorization still refuses before resource reads under F08. Trustworthy partial positives can coexist with explicitly stale history; prior terminal withdrawals survive. One complete partition may establish its own absence while the overall view stays partial.
- One logical host metadata authority atomically publishes the classified view and private binding index under attempt/predecessor/source/configuration/credential/authority/deadline fences. Losing publishers cannot re-label old content against a new predecessor. Ambiguous acknowledgement supplies no new generation, and restart requires verifiable continuity or a new epoch with old-handle invalidation.
- Provider scan continuations are distinct from public paging of one immutable acknowledged view. Pages retain generation/coverage and deterministic ordering. A newer publication or changed current authorization invalidates old cursors, with access denial first. Final partial pages have next_cursor:null and complete:false. Expiry of any retained historical row retires the immutable view and its cursors rather than filtering pages or extending history. Existing Page readers remain unchanged and unsupported for the new payload.
- Bounded storage and work are selected: at most 500 examined/retained rows, 64 partitions, 256 scopes, 64 resource-list calls plus F08 authorization, four concurrent provider requests, aggregate response bytes and original deadlines; historical retention is at most 600 seconds, with finite cleanup and eviction reported as truncation rather than deletion. Current positive-evidence/cursor limits remain bounded separately.
- Observation incarnation, public publication generation, parent material generation and private route revision are distinct. Title-only rename preserves identity; semantic target/type/port/authority-placement change or UID replacement cannot repoint an old child. Same-target revalidation is a separately admitted host action requiring fresh observed/equal target evidence and exact permission; old capabilities never silently advance. Retained history is no admission authority, and list denial is not substituted for the exact services/proxy permission.
- An authored composition executable alone links and constructs concrete parent/child libraries and injects their private ports into provider-neutral host/SDK infrastructure. Same-process live ports are required; common machine/network or independent processes do not implement that binding. Generic host and sibling libraries gain no concrete adapter dependency, registry, loader or universal driver enum. Standalone/direct adapters remain independently usable. Discovery never installs, starts or grants access to a target implementation.
- Persistent observations/routes/Connection/Composition relationships remain explicitly unmodeled. Thirteen new ESS values describe settled shapes; they do not invent persistent owners, reverse relations, transactions, authority or a running composition.

## Independent evidence inspection

source-hashes.json and snapshot/ freeze **52 files**: 32 normative/reference/model inputs and 20 final evidence inputs. Every reviewed live byte matched its snapshot at audit completion. Historical design, compiled IR and separate session artifacts are additionally preserved under auxiliary/ with auxiliary-hashes.json. The complete offline check is recorded in evidence-audit.json.

- **63/63** schema expectations independently matched the actual retained generated schemas, including **7 expected shape rejections**. The **4 deliberately accepted semantic counterexamples** were confirmed: complete-with-denied coverage, omitted namespace for namespace kind, negative generation, and a standalone published decision even when a predecessor fact is false. The last is two independently accepted shapes, not an executed cross-value decision rule.
- The two existing projection trees matched all **204** manifest hashes and each other; all **13** selected discovery schema copies matched their generated bytes.
- All **46 textual traces** were inspected against the final contracts. They remain authored expectations, not executed provider/clock/storage/publication/route/port behavior.
- The completed gate record contains **50 passing existing Rust tests**, the successful Rust 1.88 MSRV check, ESS **13 files / 197 declarations**, and **222 compiled scenarios including 34 authored**, with zero synthesis refusals. This reviewer inspected the record and did not rerun the runtime suite or gate.
- Separate session artifacts contain **13 authored / 201 synthesized** expectations with equal provenance and every authored scenario included unchanged. Compilation/synthesis is not sequential state-machine execution.
- compiled.json and compiled-final.json are byte-identical, supporting the evidence claim that the post-gate deadline comment changed no compiled value definitions. No compiler or schema generator was rerun by this reviewer.

## Explicit limits

Exact provider exhaustion/coherence and hidden target comparison, trusted scope/fact provenance, current authority and clocks, scan limits, atomic view/private-index publication, stale-publisher CAS, acknowledgement recovery, retention cleanup, persistent incarnation continuity, final dispatch/revocation fences, route validation, live process/port identity and supporting strict codecs remain implementation/advertisement gates. Generic Optional omission and Timestamp representation do not implement the required-null millisecond wire payload. These exclusions are stated consistently and do not prevent approval of the bounded semantic selections.

The official namespace field lookup supports only that field distinction ([Kubernetes ResourceAttributes](https://kubernetes.io/docs/reference/kubernetes-api/definitions/resource-attributes-v1-authorization/)); no new recognition rule, physical cluster identity, API compatibility or provider completeness predicate was inferred. No tracked/planning/runtime mutation, runtime test suite, provider API call or other reviewer report was used by this reviewer.
