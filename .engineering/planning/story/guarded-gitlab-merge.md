---
format: aep.planning-md/1
id: story:guarded-gitlab-merge
kind: story
status: active
title: Perform an approved GitLab merge without duplicate effects after restart
relations:
- decomposes: initiative:complete-local-connectors
- depends_on: story:local-approval-binding
- depends_on: story:local-approval-keys
- depends_on: story:local-bounded-clock
- depends_on: story:local-execution-audit
- depends_on: story:local-mutation-ledger
- informed_by: story:gitlab-mr-validation
- informed_by: story:persistent-gitlab-journey
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: adapters/gitlab
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: contracts/cli
- confidence: cited
  path: contracts/service
- confidence: inferred
  path: crates/connectors-build
- confidence: inferred
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/src/local
- confidence: cited
  path: crates/connectors-host/tests/http_write.rs
- confidence: cited
  path: crates/connectors-sdk
- confidence: cited
  path: crates/connectors-spec
- confidence: inferred
  path: docs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/declarations.yaml
- confidence: cited
  path: ess/domains/delegation.yaml
- confidence: cited
  path: ess/domains/local_approval_policy.yaml
- confidence: cited
  path: spec-kinds/adapter
- confidence: inferred
  path: website
revision: 19
---
## Outcome

Make the local production CLI perform an explicitly approved, SHA-guarded GitLab merge through a single connection-bound dispatch, including honest recovery when the merge response is lost. This is one integrated runtime slice: policy/issuance, native preparation, host ledger admission and final write must work together before it is complete.

## Acceptance

After an approved CLI merge changes a fixture MR and its response is lost, invoking the same business key after CLI/owner restart returns the original uncertain attempt without issuing a second native merge.

## Selected contracts and model

contracts/service/local-mutations.md owns serialized local policy, owner identity, canonical subject preparation, protected issuance, audit and dispatch ordering. ess/domains/local_approval_policy.yaml declares the retained policy entity and its references to the existing service configuration and issuer; existing delegation/approval/attempt/audit entities keep their ownership. Its process-only NativePreparation value is not a persistent session. Pinned ESS validation before decomposition reports `connectors v1 — 23 file(s), valid`. Runtime predicates marked UNMAPPED require implementation tests, not additional invented lifecycle states.

contracts/cli/v1alpha1/private-mutations.md selects explicit local configuration/private transport version two and bounded prepare/commit/cancel exchanges, preserving version-one codecs and read projections. spec-kinds/adapter/v3/semantics.md selects a closed write array, scalar JSON body mappings, separate descriptor projections and one-use generated write execution; ess/domains/declarations.yaml gives write mapping values a typed home. Author and test each closed schema before activating its new format; current v1/v2 inputs remain usable. Generated outputs are never edited manually.

adapters/gitlab/contracts/guarded-merge.md owns merge semantics against the unchanged pinned upstream. Reuse native ValidationInput/Observation in adapters/gitlab/spec/ess/domains/merge_requests.yaml. Native preflight checks current MR/head/pipeline before the host final gate; commit performs one SHA-guarded PUT with immediate merge and no source-branch deletion. Pipeline-ID checks are preflight observations, not an atomic provider guarantee. GitLab remains the owner of native scopes, schemas, request mapping and effect interpretation.

## Runtime implementation

Publish a revisioned owner policy using metadata migration eight, active issuer identity, admitted operation metadata, exact config/descriptor/executable/clock selections and bounded shared/exclusive use leases. The initial metadata/lease port was prototyped under the parent before this decomposition; its twelve tests are preliminary evidence only, not a CLI capability or completed story. All subsequent changes to that port and its integration belong here.

Generate production `approvals policy-set`, `policy-status`, `prepare` and `issue` handlers. Reconstruct the exact subject from authenticated owner/configuration and retained selected connection; explicit `--approve-subject` must match. Obtain current bounded authenticated time, hold current policy/key leases, read the qualified signing seed and durably publish a protected proof file without stdout disclosure or overwrite. Update authored CLI input/output types and parser bindings, including protected approval input, idempotency key and existing MutationObservation/SourceAudit results. No new public delegated protocol is advertised.

On invocation, perform current result-access admission before key lookup. Exact retained key observations do not do native preflight, secret reads or new approval spend. For a new candidate, validate proof and credential readiness, perform bounded native preflight through the owned child, then recheck current time/policy/key/connection use and separately acknowledge audit admission, attempt/key prepare, approval spend, connection dispatch admission and attempt dispatch gate. Only its original live winner can send the matching private commit. Consume the native preparation and authenticated write capability once. Any post-gate uncertainty remains quarantined across restarts; neither timeout nor a later exact-target read authorizes a resend or attributes another actor's merge to this attempt. Audit finalization cannot rewrite a known effect.

## Verification and completion boundary

Run production CLI fixtures with disposable HTTPS GitLab, qualified Secret Service and authenticated clock fixtures. Prove known applied and refused merges as well as the acceptance's lost-response path. Include protected proof publication failure, policy/key revision changes and held-use exclusion, wrong subject/caller/clock/key, absent/expired/reused approval, duplicate business keys and conflicts, concurrent repair/revoke/dispatch, schema/permission refusals, preflight changes, every storage acknowledgement fault, preparation substitution/cancel/expiry, duplicate commit, old-peer refusal and exact child cleanup. Preserve already valid credentials after failed repair, and prove no new effect on replay after restart. Verify migrations one through seven remain unchanged and setup stays at version three.

Run affected shared/native ESS, CLI and adapter generation/drift, conformance, the full Connectors gate with Rust 1.88, website/reference/example checks and existing GitLab CLI journeys using task-owned TMPDIR and two Cargo jobs. Retain exact source, tool and artifact identities and failures/corrections. Reproduce generated output and the selected distributable payload from identical pinned inputs; retain time-stamped receipts separately. Dedicated GitLab sandbox merge/read verification followed by exact cleanup is additionally required before this story is implemented; credential-blocker:gitlab-runtime-sandbox remains open until explicit access exists. Fixture evidence alone cannot close it.

## Scope and sequencing

Root is the sole implementation/planning writer on primary main. Cited surfaces: crates/connectors-host/src/local (policy, existing ledgers, owner and private child transport), crates/connectors-sdk (separate one-use write capability), crates/connectors-spec and spec-kinds/adapter (v3 generation), adapters/gitlab (native semantics/model/spec/bindings/fixtures), apps/connectors and apps/connectors-cli-contract (authored/generated CLI), contracts/service and contracts/cli, ess/domains/declarations.yaml, ess/domains/local_approval_policy.yaml and ess/domains/cli.yaml. Inferred verification/publication surfaces: crates/connectors-build, crates/connectors-conformance, docs, website and Cargo.lock. All overlap with existing local approval/key/clock/audit/attempt and GitLab read stories is serialized; no parallel implementation or shared build is scheduled. The four required planning critics are read-only; root records their outputs.

The parent continues to own full GitLab completion, C14 create/update under its open atomic-head decision, dedicated read acceptance, other recorded GitLab collection, complete management across three adapters and remaining reproducible distribution acceptance. Kubernetes including Helm follows full GitLab, then PostgreSQL, then MCP, then remaining providers. This merge slice neither substitutes for those workflows nor moves their lifecycle. MCP publication/exact pin is later; Connectors publication, deployment, Atlas integration and paid governed runs remain outside this story. Preserve the unrelated AGENTS.md release edit without claiming it as this work's authorization.

## Guarded merge implementation checkpoint — 2026-09-11

The proposed contracts/model and governed decomposition are recorded in story:guarded-gitlab-merge, now active. The initial policy metadata port is implemented with migration eight, retained identity/revision CAS, exact issuer/instance binding, sorted bounded operation sets, passive inspection and process-bound shared/exclusive leases. Twelve policy tests pass, including actual child exit after commit before acknowledgement, cross-process held-use exclusion, fork refusal, failed writes, revision exhaustion and restart. This storage port consumes an already admitted set from its trusted host caller; it does not itself authenticate callers, classify native operations, issue proofs or dispatch writes.

The complete Connectors gate with Rust 1.88 passes, including 103 host tests, six independent native model compilations, CLI/adapter generation, Clippy, conformance and planning validation. Retained evidence is docs/evidence/local-approval-policy-20260911/README.md. CLI policy/prepare/issue, private protocol two, v3 write generation, one-use write capability and native guarded merge remain required implementation under the active story. Its lost-response CLI acceptance, provider sandbox evidence and full distribution reproducibility have not been satisfied.

Design, scope and parallel-safety critics approved round one. The first acceptance reading reused the scope critic's context; it is retained as an archived review-result and is not counted as the independent acceptance review. A fresh-context acceptance reviewer approved revision five without reading other critic outputs. Sonnet was unavailable, so inherited models were used; the fourth lane was delayed by available slots. No reviewer wrote code or planning state. AEP initially refused promotion because the existing-vision serves edge was missing, then refused a direct draft-to-active move; the edge was added and the legal draft-to-proposed-to-active transitions succeeded. These are planning checks, not runtime acceptance.

The owner remains root, with serialized implementation on main. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers. The dedicated GitLab sandbox and create/update atomic-head decision remain open. Preserve the unrelated AGENTS.md release edit; this checkpoint does not publish Connectors or register Atlas.

## Verified write-generation checkpoint — 2026-09-11

The v3 frontend and separate SDK write capability are implemented under story:guarded-gitlab-merge. The generator retains the v2 read projection, emits an independently revised private descriptor and ESS-generated typed write values, and consumes an immutable prepared request and authenticated write capability once. Native effect knowledge survives safe-result validation failure. Strict codecs, pinned source constraints, exact request mapping, output ownership, deterministic regeneration, lost replies and GET/write separation are exercised by an independently compiled consumer fixture. Production adapters still select v1/v2; no callable merge command is claimed.

docs/evidence/write-generation-20260911/README.md retains the passing repository gate with Rust 1.88, the complete five-test write suite on Rust 1.88, three consumer runtime and three compile-fail cases, website/reference checks, exact source manifests and failed/corrected runs. A test checks the selected GitLab merge mapping against its existing pinned vendor source without provider I/O. After the full gate, the v1 rejection test gained an explicit valid-v1 baseline; the complete minimum-version suite and affected Clippy/format checks pass on that final test. Implementation bytes are unchanged across those checks.

A separate release integration advanced main to f475e0b5610b1d7334094e5ab507cfa1aa030e45 during a gate's final planning read. Its transient journal/file mismatch is retained; the integrated store subsequently validated and the entire gate passed again with the integrated Cargo version inputs. Its release records and instructions are preserved. This generator increment adds no release, provider access or publication claim. Only the primary checkout remains; this increment created no linked tree.

Next implement private version-two prepare/commit/cancel and production CLI policy/preparation/issuance, then join the existing approval/audit/attempt ledgers to one SHA-guarded GitLab merge. The story and initiative stay active: lost-response CLI/restart acceptance, dedicated sandbox evidence, full distribution reproducibility, complete GitLab workflows and the open create/update atomic-head decision remain required. Full GitLab continues before Kubernetes with Helm, PostgreSQL, MCP and remaining providers. Root remains the sole writer for this implementation and planning increment; paid governed runs are not required and the recorded driver blocker is unchanged.

## Verified private mutation transport checkpoint — 2026-09-11

Private protocol two and explicit local configuration version two are implemented under story:guarded-gitlab-merge. Version-one codecs and implicit selection digests remain unchanged; the old configuration refuses the new field. Readiness and cached descriptor admission enforce the selected effect profile. A live preparation retains immutable native input/credentials and occupies its exact child across the original deadline. Commit and cancel consume it once; malformed/replaced controls, duplicate commit, EOF and expiry refuse without another write. Cancellation acknowledges destruction. Lost commit replies remain unknown; a known applied effect survives failure to validate or disclose safe output. Production GitLab still advertises its read-only projection, and this trusted transport port does not itself admit an approval or write.

docs/evidence/private-protocol-v2-20260911/README.md retains commands, full logs, 385 source/dependency hashes, tested CLI/adapter/keyring identities and limitations. The repository gate with Rust 1.88 passes with two test threads, along with all six disposable GitLab CLI journeys, affected Clippy, website build/typecheck and reference drift checks. The host suite has 113 passes and 18 existing ignored cases; integration tests and the one-use compile-fail example pass. The first full gate encountered MetadataUnavailable in an unchanged audit-capacity test; its exact isolated recheck and the bounded full gate passed. No audit code, contention timeout or test assertion was changed, and the initial failure is preserved without asserting a proven root cause.

Next implement the production CLI policy/preparation/issuance handlers and owner mutation IPC, then join current approval/audit/attempt/connection dispatch to one native SHA-guarded GitLab merge. The lost-response CLI/restart acceptance, dedicated sandbox evidence, full distribution reproducibility and complete GitLab workflows remain required. The sandbox and create/update atomic-head blockers stay open. Root remains the sole writer on main; no linked tree was created. The full GitLab → Kubernetes with Helm → PostgreSQL → MCP → remaining-provider order is unchanged. This is an implementation checkpoint, not a completed guarded merge, provider batch, release or publication; the paid driver blocker is unchanged.

## Verified local approval CLI checkpoint — 2026-09-11

The generated production CLI now implements approvals policy-status, policy-set, prepare and issue under story:guarded-gitlab-merge. Policy publication admits current private-protocol-two write metadata and operation/profile permissions before the existing revision CAS. Preparation resolves the exact retained connection, current policy and strict native input without secret/provider/clock access, service startup or metadata writes. Issuance reconstructs the approved subject independently, acquires fresh authenticated time before policy/key leases, rechecks current selection, reads the qualified seed and durably publishes a new protected proof file. It preserves possible publication after a failed acknowledgement and never overwrites or automatically reissues. No business effect or proof spend is performed by these commands.

The canonical preparation fixture exposed an existing projection discrepancy: optional object fields omit members, while the selected canonical codec requires explicit nulls. The delegation owner now models required nullable values and the canonical authority-scope projection; unrelated idempotency scope projections keep their existing meaning. ess/domains/delegation.yaml is added to the story's cited scope. This corrects the model to the existing canonical proof contract; it adds no persistent entity, new protocol format or separate foundation story. The file-only policy carrier uses a generated path option plus the trusted bounded reader because pinned ESS requires all three sources for a document carrier.

docs/evidence/local-approval-issuance-20260911/README.md retains failed/corrected logs, 412 source/dependency hashes and runtime binary identities. The full repository gate with Rust 1.88 passes, including 122 host unit tests, CLI/adapter generation, 81 structural CLI fixtures, conformance and boundaries. The explicit production approval CLI journey passes with qualified disposable custody and an independent synthetic clock, including protected publication, printed-subject digest verification, restart reuse, overwrite refusal and locked custody. All six disposable production GitLab read journeys pass in 256.70 seconds. Website build/typecheck and reference drift checks pass. These facts do not establish dedicated provider acceptance or full distributable reproducibility.

The guarded-merge story and initiative remain active. Next integrate protected proof consumption and owner mutation IPC with the existing approval/audit/attempt/connection dispatch gates, then implement one SHA-guarded GitLab merge and its lost-response/restart acceptance. Full GitLab workflows, dedicated sandbox access, the create/update atomic-head decision and distribution reproducibility remain open. Full GitLab still precedes Kubernetes with Helm, PostgreSQL, MCP and remaining providers. Root remains the sole implementation/planning writer on primary main; no linked worktree, release, external publication or paid governed run is created by this checkpoint. The recorded driver blocker remains unchanged.

## Verified mutation integration checkpoint — 2026-09-11

An integration test exposed a concrete incompatibility: audit, mutation and approval-spend ports accepted only through metadata version seven, while current local policy publication requires version eight. Their version guards now admit eight without changing migration bytes or acknowledgement boundaries. A combined test exercises audit admission, preparation, spend, audit confirmation, dispatch and store reconstruction/recovery against that schema, retaining the exact key and spent proof after an unknown result. It uses fixture policy/clock/key inputs; it does not claim a native effect or production owner restart.

The private proof-file decoder consumes the existing issuance document under explicit bounds without sending compact proof bytes through ordinary JSON Values. Closed and duplicate/escaped/oversized refusals are tested, as is verification of a decoded valid document. The concrete host HTTP capability consumes itself for one PUT over captured target/TLS/credential configuration. Four disposable network tests cover exact framing, disabled redirects/retries, a response lost after a complete request, native error responses, response bounds and path refusal before credential resolution. Provider effect interpretation stays with the native adapter. These APIs are not yet called by CLI write invocation.

docs/evidence/mutation-composition-20260911/README.md retains verification commands, logs, 386 source/dependency hashes, runtime binary identities and the initial fixture correction. The full repository gate with Rust 1.88 passes, including 124 host unit tests, four HTTP write tests, three existing HTTP tests, a consuming-capability compile-fail case, generation/drift, boundaries and conformance. All six disposable production GitLab CLI journeys pass in 206.80 seconds. Dependency pins, generated output and public website inputs are unchanged. Two local implementation guides now reflect already delivered approval issuance and the still-unfinished coordinator.

The story and initiative stay active. The next required implementation is protected proof consumption and explicitly selected owner IPC version two joined to current result-access admission, audit, mutation, approval and connection-use dispatch, then the native SHA-guarded merge. No callable GitLab write, full lost-response CLI acceptance, dedicated sandbox evidence or reproducible distribution is claimed. Dedicated sandbox access and C14 create/update atomic-head semantics remain open. Full GitLab still precedes Kubernetes including Helm, PostgreSQL, MCP and remaining providers. Root remains the single implementation/planning writer on primary main; this checkpoint creates no linked tree, release, external publication, Atlas registration or paid governed run.

## Integrated guarded merge in progress — 2026-09-11

The production owner/2 coordinator, generated CLI approval-file/idempotency-key inputs and GitLab native v3 merge binding are now joined locally. The v1/public projection retains eleven reads. One production CLI fixture passes all three outcomes (Applied, Refused and lost-response Unknown), preserving the original attempt across owner shutdown and same-key replay with a deleted proof file, no provider/clock call and no second PUT. Fourteen native MR tests pass, including exact target/response classification, current preflight refusal and no retry. Proof reuse and unavailable-custody replay are being added before the required full gate; this is not final release or sandbox evidence.

The execution audit is acknowledged before provider preflight reads and its original receipt/facts are confirmed at attempt preparation. contracts/service/local-mutations.md now states this ordering explicitly; no new audit entity or transaction is introduced. Pinned CLI generation refused with `error: unsupported CLI primitive `Uuid``. The CLI presentation therefore projects the existing attempt UUID as canonical string correlation while the owner validates it; durable identity and the service-wire model remain unchanged. The native spec reuses the existing ValidationInput and Observation models and the pinned merge endpoint.

Cargo.toml excludes the new generated write-type package from the authored workspace, and the adapter consumes that generated path dependency. GitLab business code still has no host or sibling-adapter dependency. Current changes remain uncommitted under this active story. Owner-crash acknowledgement cases, the wider failure matrix, dedicated sandbox acceptance, remaining GitLab workflows and reproducible distribution are still required. Kubernetes including Helm, then PostgreSQL, MCP and remaining providers follow the agreed provider order.

## Integrated CLI merge verification — 2026-09-11

The full Connectors gate with Rust 1.88 passes for the joined owner/private-transport/CLI/native implementation. All seven production GitLab CLI journeys pass in 290.91 seconds. The new journey verifies Applied, Refused and lost-response Unknown, exact PUT/effect counts, a spent proof refused under another key, conflict without original-result disclosure, and same-key passive replay after owner shutdown with proof deleted and the keyring daemon stopped. Fourteen native MR tests pass. Website typecheck/build/reference drift, fifteen Rust example tests and both browser/UI suites pass. Retained commands, logs, input hashes and runtime artifact identities are in docs/evidence/guarded-gitlab-cli-20260911/README.md.

This is CLI process restart and passive observation after owner shutdown; it does not prove a newly started owner's owner/2 replay path or crashes between storage acknowledgements. Those cases, final-admission failure/audit completion and the broader joined failure matrix remain required. Dedicated sandbox access, create/update atomic-head semantics, other selected GitLab workflows and reproducible distribution remain open. No story/provider/initiative completion or release is claimed.

Correction to the preceding Cargo scope wording: the root manifest excludes generated workspace directories, but Cargo still lists generated type path dependencies as implicit workspace members. The gate checks their formatting and complete generated bundles; no generated source was manually changed. The pinned CLI refusal `unsupported CLI primitive Uuid` was addressed only by a canonical-string presentation of the existing UUID identity. The original shared type and ledger identity are unchanged.

## Verified owner restart, crash and revocation increment — 2026-09-11

The joined production CLI fixture now proves actual owner/2 observation through a newly started owner for Applied, Refused and lost-response Unknown, with the proof file deleted, no proof supplied, custody stopped and no provider/clock call. A fourth case holds the provider response after one applied PUT, observes the pending live attempt, kills the exact owner through its socket-derived pidfd, verifies both owner and native child exit, then observes the same attempt through a new owner without another PUT. Approval spend remains enforced under a different business key after that crash. The original crash attempt remains pending/unknown; no automatic recovery or new send authority is claimed.

A deterministic revocation-during-preflight fixture reproduced an unfinished admitted audit after the current-admission refusal. Both affected coordinator paths now finish their audit before propagating the refusal. The fixed fixture verifies the exact original audit ends refused/revoked and no PUT occurs. All eight disposable production CLI journeys pass in 421.43 seconds. The full gate with Rust 1.88 passes, including 124 host unit tests, generation/drift, shared/native ESS, Clippy, boundaries and 315 conformance scenarios. Website typecheck/build and reference drift pass. The first barrier mismatch and actual audit regression are retained separately.

docs/evidence/gitlab-owner-recovery-20260911/README.md retains commands, logs, 188 selected source hashes and tested runtime identities. The gate's workspace build produced different executable bytes; its hashes are recorded separately, and rebuilding the exact acceptance package selections restored both tested hashes byte-for-byte. No registry version, generated native/CLI output, native write semantics or protocol shape changed. This corrects existing audit completion and adds runtime evidence under the active guarded-merge scope; it introduces no entity, relation or new decomposition.

Automatic abandoned-attempt recovery, every storage acknowledgement crash, the remaining concurrent admission/final-audit failure matrix, dedicated provider sandbox acceptance, C14 create/update atomic-head semantics and distribution reproducibility stay open. The sandbox URL/project/protected-file details were requested again while independent work continued. The story and initiative remain active. Root is the sole writer on primary main; no linked tree, release tag, push, deployment or paid governed run is part of this increment. Full GitLab still precedes Kubernetes including Helm, PostgreSQL, MCP and remaining providers; the driver protocol-loading blocker retains its existing scope.

## Exact-key recovery and startup verification in progress — 2026-09-11

Exact-key abandoned-attempt recovery is implemented on the serialized instance worker, guarded by its exclusive child-slot borrow and the owner's process-lifetime lock. Prepared settlement requires fresh configured trusted time; Dispatching recovery quarantines without time. Terminal replay remains passive, pending recovery spends no new approval and starts no native child. Background/unkeyed recovery and the approved native pre-dispatch crash matrix remain open.

The production suite exposed repeated admission timeouts in several startup journeys. Exact isolated revocation and concurrent-restart rechecks passed without changed deadlines, but the complete rerun still failed. A task-scoped syscall trace and current source identify three full hashes of the roughly 99 MB debug owner executable before launch, consuming almost its ten-second budget. Remove the redundant pre-capture content hash while retaining descriptor-relative ownership/type/path checks and the authoritative hash of the sealed snapshot. Public Executable::open/check retain their existing content-verification contract; a private capture method binds path admission directly to verified immutable capture. This bounded startup fix stays within the existing host-local implementation scope and introduces no typed entity, storage change, new decomposition or deadline extension. Complete the runtime regression and required gate before integration; preserve failed runs and do not treat isolated passes as full acceptance.

## Verified exact-key recovery and startup increment — 2026-09-11

The production CLI now recovers a pending exact-key original on its one serialized instance worker, after earlier native exchanges have ended. An exclusive child-slot borrow and the owner's lock retained through kernel process exit establish recovery ownership; the busy flag is only a latency hint. Current result admission is rechecked. Prepared becomes Aborted only with fresh configured trusted time for replay retention; unavailable time leaves it pending. Dispatching becomes Indeterminate with a quarantined key without needing time. Recovery reads no proof or credential and creates no provider send authority. Its task starts no native child or suppression change; ordinary configured automatic owner startup remains separate. Terminal replay stays passive. No persisted schema, migration, wire format, native semantics or dependency pin changes.

Runtime acceptance uncovered repeated startup timeouts before provider work. Source and syscall evidence identified three full hashes of the roughly 99 MB debug owner executable, consuming almost the original ten-second budget. The private launch path now combines admitted source opening with digest-verified sealed capture, removing the redundant pre-capture hash without changing deadlines or public Executable::open/check semantics. Unverified source opening remains private. Focused tests retain changed-content, symlink, broad-permission, snapshot and deadline refusals.

All nine disposable production CLI journeys plus the inert subprocess fixture driver pass in 368.12 seconds. The owner-death fixture now proves durable quarantine of the original attempt after one PUT, no new child, deleted proof and unavailable custody. The preparation fixture exits a durable-port producer after acknowledged preparation, tests one failed clock request then successful clock recovery, retains the original attempt/request and fixed replay interval, and sends no provider requests. This producer uses Approval::NotRequired with no native preflight/spend/gate; it does not establish an approved native pre-dispatch crash. Failed full runs and isolated rechecks are retained, not substituted for the passing final regression.

The full gate with Rust 1.88 passes: 125 host tests, shared and six native ESS roots, generation/drift, Clippy, dependency boundaries and conformance. Reference drift passes. Unchanged website/example inputs reuse the preceding verified checks. Exact package rebuilds after the gate restore all tested runtime executable hashes. docs/evidence/gitlab-keyed-recovery-20260911/README.md retains commands, logs, 100 source/dependency hashes, intermediate and accepted executable identities and limitations.

Background/unkeyed/unobserved recovery, all storage-acknowledgement crashes and the wider concurrent failure matrix remain open, as do dedicated sandbox acceptance, C14 create/update atomic-head semantics, remaining GitLab workflows and distributable reproducibility. The story, initiative and goal stay active. Full GitLab still precedes Kubernetes including Helm, PostgreSQL, MCP and remaining providers. Root remains the sole implementation/planning writer on primary main, with no linked tree, release, push, deployment, Atlas registration or paid governed run. The driver blocker and existing sandbox input request remain unchanged. This increment introduces no entity or new decomposition and does not require a new critic panel.
