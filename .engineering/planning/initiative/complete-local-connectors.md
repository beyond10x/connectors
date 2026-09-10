---
format: aep.planning-md/1
id: initiative:complete-local-connectors
kind: initiative
status: active
title: Complete the local Connectors CLI with reusable MCP support
relations:
- informed_by: specification:core-model-closure-20260909
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: epic:mcp-contracts
- informed_by: story:kubernetes-spec-service
- serves: vision:independent-contract-adapters
revision: 16
---
## Outcome and authority

Implement the operator-approved local Connectors product through seven successive working runtime milestones. Connectors owns product authority; ../mcp owns reusable protocol mechanics. This record carries the implementation plan supplied on 2026-09-10. It does not close the completed specification milestones or treat them as runtime proof.

Linux x86_64 and Rust 1.88 are the initial platform. SQLite owns non-secret metadata, with versioned migrations, WAL and full synchronization. The admitted OS Secret Service collection owns credentials. Existing installed Connectors configuration and credentials are not automatically migrated. Local native credential entry is protected; native SaaS OAuth onboarding is excluded. MCP browser OAuth and refresh are included.

## Delivery sequence

The operator's updated order is GitLab, two more native providers, MCP, then the remaining providers. Kubernetes and PostgreSQL are selected as those next two because README.md and their adapter sources already establish runnable implementations. This is a scheduling choice within the approved scope. MySQL stays in the remaining-provider phase; recorded Helm workflows accompany Kubernetes.

1. Persistent GitLab: fresh setup, protected entry, saved connection, existing read, CLI and owner restart, reuse without re-entry. story:persistent-gitlab-journey continues to own this first concrete delivery.
2. Complete the selected GitLab workflows: exact-commit CI pipelines/jobs/traces, MR and changed-record reads, validation and admitted MR create/update/merge. Bring forward the required management and approval/audit/attempt/dispatch/idempotency controls, including exact-head checks and possible-write uncertainty. Do not postpone these GitLab workflows until after MCP.
3. Complete Kubernetes, then PostgreSQL through the same persistent local lifecycle. Kubernetes includes discovery, diagnosis, selected changes and rollout observation, bounded exec/copy/tunnels and associated Helm history/rollback. PostgreSQL includes the selected typed, parameterized, bounded read-only incident queries. Integrate story:kubernetes-spec-service here. Complete and verify all specified setup/adapter/connection/operation management across the three adapters, including repair/revoke/dispatch races, exact child ownership, concurrent startup and durable stop suppression.
4. Expand ../mcp with resources, prompts, servers, progress/cancellation, consumer-owned stdio and explicit OAuth one-use exchange/publication seams. Adopt AEP there, preserve its primary documentation workflow edit, verify and publish source commits through verified Atlas bot authority.
5. Pin that exact published MCP revision here and implement outbound and inbound tools/resources/prompts over stdio and Streamable HTTP, with version-specific 2026-07-28 and explicit 2025-11-25 interoperability, caller isolation and authenticated local verification of the cloud-capable server profile.
6. Complete remaining selected providers: MySQL, Jira, Slack, Confluence, Loki, Prometheus, Grafana and Docker. Deliver each provider's required reads and, where selected, admitted writes or execution using the established shared controls. SQL remains read-only. Jira/Slack updates, Confluence collection, observability queries and Docker build/registry/container workflows retain their full original scope.
7. Prove the complete cross-provider incident journey, required upgrade refusal cases, exact sandbox cleanup, Rust 1.88 and existing tools-only MCP/Harness compatibility, deterministic generation and two isolated reproducible distributable builds; finish support documentation and integration.

Each provider phase includes generation, packaging, documentation and its runtime acceptance before advancing to MCP or the remaining providers. Shared lifecycle, mutation and execution foundations are implemented with the first selected provider that needs them. This replaces the original ordering that put MCP before completing the first three providers' selected reads and writes; it does not remove any acceptance requirement.

Acceptance remains docs/recent-adapter-usage-20260909.md C01-C17 and C20-C22 plus the supplied MCP interoperability and deterministic failure cases. GitLab owns C09/C14 and its part of collection; Kubernetes owns C12/C15/C16, including the Helm actions; PostgreSQL owns its part of C06 before MCP, while the MySQL part follows MCP. Common lifecycle cases apply across the initial three providers. Cross-provider C20, the complete C21 collection and C22 capability-upgrade cases finish after their required providers exist. Every selected workflow requires runtime evidence; fixtures cannot satisfy missing dedicated-provider sandbox evidence.

This order change introduces no new entity or decomposition. The initiative still has one decomposing story, so the planning skill's fewer-than-two-child rule skips a critic panel for this scheduling update. Model and review each unresolved native/shared semantic before creating its dependent implementation stories.

## Specification ownership and dependencies

contracts/cli/v1alpha1/semantics.md and ess/domains/cli.yaml own the existing local surface. docs/design.md section 31 inventories distinct logical metadata owners and atomic groups. ess/domains/auth_bindings.yaml, credentials.yaml, credential_evidence.yaml, declarations.yaml and connection_admission.yaml supply existing typed homes; native profiles belong to adapters. The implementation must model and review missing semantics before decomposing them. Later milestones remain ordered outcomes here until their contractual gaps are resolved, not invented runtime stories.

The existing epic:mcp-contracts remains the Connectors MCP contract owner. The Kubernetes generation story and tooling-blocker:kubernetes-driver-protocol-loading retain their existing meaning; paid driver runs are not required. Existing specification milestones remain historical evidence.

## Verification and publication

Use task-owned TMPDIR under .local/tmp and two Cargo jobs. Run the affected ESS validation, generation/drift/conformance, website checks where affected, and the required Connectors gate with MSRV. MCP uses cargo xtask gate; recheck remote policy before publishing already locally verified source. Keep timestamped receipts separate from deterministic payloads. Reuse evidence only when its relevant inputs are unchanged.

Connectors publication and cloud deployment are excluded, as are GitHub/AWS adapters, optional MCP extensions, Atlas registration/delivery wiring and unrelated Harness repinning. Publish only verified MCP source commits and consume their exact revision. Keep Connectors local. No new bare recovery repository. Follow the explicit single-agent direct-checkout rule here; use managed worktrees and leases for MCP. Final integration requires clean Connectors main, published MCP goal commits and no task-owned linked worktrees.

## Current evidence and open prerequisites

At the start of the implementation initiative, Connectors had compatibility describe/invoke/serve only; the grouped parser was generated but not bound to production management, and there was no persistent metadata authority. That is the historical starting point, not the current runtime support claim.

Implementation commit 2c41fc49e00549ddb671a68845ec5fcf6ee4277b adds production setup/inventory handlers, private configuration/state admission, a versioned SQLite authority and non-interactive keyring availability inspection. The gate including Rust 1.88, generation, conformance, tests and Clippy passed; docs/evidence/local-runtime-20260910/README.md retains exact inputs, commands and limits. Integration checkpoint 680f41eb8a6633063f87c280b934e0e3f369fe4d leaves clean local main with no task-owned linked worktrees. No persistent GitLab connection or restarted credential reuse has passed acceptance. story:persistent-gitlab-journey remains the active next delivery.

The installed 0.7.0 CLI is a separate product baseline and is not migrated. Dedicated GitLab sandbox target and protected credential-file path have been requested; credential-blocker:gitlab-runtime-sandbox remains open. The other provider sandboxes, Secret Service durable write/restart/deletion qualification and independent MCP interoperability remain required evidence. They are not assumed from installed credential presence. The Kubernetes driver protocol-loading blocker remains open for its distinct governed-run scope; paid driver runs are not required for interactive implementation.

The provider-order update is an interactive planning change with one store writer and no approval bypass records. No runtime acceptance or lifecycle completion is claimed from this update.

## Current runtime checkpoint — 2026-09-10

The earlier current-evidence paragraph records the initiative's initial runtime state. Persistent GitLab restart/revalidation and local CI now have executable fixture evidence. story:gitlab-ci-runtime adds five generated CI read operations beside the original three, bounded trace transport and safe provider failure projection. The final repository gate and four production CLI/private HTTPS/keyring journeys pass; local GitLab image packaging also passes. docs/evidence/gitlab-ci-20260910/README.md records exact commands, source/artifact identities and remaining limits.

The initiative now has two decomposing stories. Unlike the earlier one-child scheduling update, this CI decomposition ran all four planning critics independently in two bounded rounds, with the acceptance finding fixed and all second-round verdicts approved. Existing completed specification milestones, the related MCP epic and Kubernetes story remain unchanged.

Neither GitLab story is complete without its dedicated sandbox evidence. Missing GitLab access remains an explicit credential blocker. The next independent implementation is GitLab MR/changed-record reads and admitted MR create/update/merge with the required shared write controls; Kubernetes and PostgreSQL follow the full selected GitLab work. MCP and the remaining providers retain their required order and scope. The paid AEP driver blocker, full cross-provider acceptance and reproducible distribution obligations remain open. Connectors stays local on main with no task-owned linked trees; MCP's pre-existing primary documentation-workflow edit is preserved.

## MR read checkpoint — 2026-09-10

The initiative now has three decomposing runtime stories. story:gitlab-mr-reads adds merge_request.get and fixed-window merge_requests.list, bringing GitLab to ten generated read operations. Four planning critics reviewed the partial decomposition in two bounded rounds; both acceptance findings were fixed and all final verdicts approve. The exact critic records and outcomes remain in the store.

The full required gate, five production CLI journeys, website/reference checks and local image build pass. docs/evidence/gitlab-mr-20260910/README.md preserves source/artifact identities and the initial setup-contention test failure with its bounded-wait verification follow-up. Dedicated sandbox evidence remains required for all three GitLab stories; none is complete from local fixtures.

Next is native MR validation and guarded create/update/merge with shared approval, audit, durable attempts, dispatch and idempotency. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers. Missing write semantics must be modeled and reviewed before their implementation decomposition. Other GitLab collection, cross-provider acceptance, complete three-adapter management and reproducible distribution remain parent-owned ordered work.

Worktree inspection finds no Connectors linked trees; the local main integration checkpoint retains verified artifacts and active-goal caches. MCP's primary checkout was separately observed clean at bf7f9415fbd8a28abff789d7d39d2584a8f6d2b2; the earlier workflow edit is no longer a local diff, and this increment made no MCP change or publication claim. Re-read that updated baseline when the MCP phase begins. The distinct AEP driver protocol-loading blocker remains unchanged, with paid governed runs outside this implementation requirement.

## Mutation ledger increment — 2026-09-10

story:local-mutation-ledger is the fourth partial runtime child. It owns the existing mutation metadata group's SQLite attempts, exact keyed reservations, conservative retention, fault tests and migration continuity; it does not advertise a business write or duplicate the earlier GitLab read journeys. Model ownership is ess/domains/mutations.yaml and idempotency.yaml, including the newly explicit trusted settlement-clock interval value. The local production clock source still requires qualification; no ordinary wall-clock sample is accepted as proof of safe key expiry.

The next GitLab increments retained here are native MR validation/write semantics, local approval issuance and verification/spend, audit, clock qualification, authored CLI/wire inputs and the fully admitted connection-bound dispatch coordinator. Pinned native REST and GraphQL create/update have no source-SHA precondition, unlike merge; the full C14 guarantee must be resolved before those operations are advertised. The native semantics and dedicated sandbox gaps remain open and are not replaced by ledger tests. Full GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider ordering is unchanged.

Implementation, planning writes, overlapping source edits and build outputs remain serialized on primary main. The four-child partial decomposition is reviewed by the four planning critics; existing completed specifications remain preserved. This is an interactive implementation, with no approval bypass or paid governed run.

## Completed private ledger checkpoint — 2026-09-10

story:local-mutation-ledger is implemented at its deliberately private storage-port boundary. The final Rust 1.88 gate, 14 ledger tests including four abrupt process exits, website checks and all five optimized CLI journeys pass. docs/evidence/local-mutation-ledger-20260910/README.md retains source and artifact identities plus the failed intermediate runs. Ordinary setup/read management keeps schema 3; admitted mutation preparation alone installs schema 4. Public CLI writes remain unadvertised.

The initial unoptimized CLI rerun exposed owner-startup timeouts also reproduced by the unchanged prior verified binary. The optimized CLI passes all five journeys in 86.42 seconds under unchanged deadlines, and the guide now selects the optimized build. This evidence does not replace dedicated GitLab sandbox acceptance or assert an instrumented cause for the earlier timeout.

Next implement the shared audit admission/finalization port, then local approval issuance/verification/spending and qualified clock/dispatch composition for GitLab writes. Model and review any missing binding semantics before those decompositions. Native MR validation and governed writes still belong to this GitLab phase; decision-blocker:gitlab-mr-create-update-head-guard holds the unresolved C14 create/update guard interpretation, while credential-blocker:gitlab-runtime-sandbox holds dedicated access. Neither stops independent shared implementation. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers, with all original acceptance and reproducible-distribution requirements retained.

## Execution audit increment — 2026-09-10

The fifth partial runtime child, story:local-execution-audit, binds the existing AuditRecord model and contract to the same local SQLite authority with a separately acknowledged anchor and one immutable final observation. It depends on the implemented mutation ledger only for retained attempt references and migration continuity. It does not expose audited CLI operations or provider writes before the full coordinator exists. Current private-port evidence remains distinct from dedicated provider acceptance.

Single implementation and planning-store writer on primary main serializes the shared host, migrations, docs and website surfaces with prior GitLab increments. The four planning critics review this five-child partial decomposition independently. Local approval issuance/verification/spending and qualified clock/dispatch composition follow audit; native GitLab validation/writes, the C14 guard decision and dedicated sandbox evidence remain open. GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider order and scope are unchanged.

## Completed private audit checkpoint — 2026-09-10

story:local-execution-audit completes its private host persistence boundary, with the full Rust 1.88 gate, 12 audit tests including four abrupt process exits, website/reference checks and all five optimized production CLI journeys passing. docs/evidence/local-execution-audit-20260910/README.md retains exact source and executable identities and the corrected ESS-summary/Clippy failures. The audit port acknowledges anchors and final observations separately and cannot reconstruct execution authority during recovery.

The next GitLab implementation is local approval issuance/verification/spending, followed by qualified clock and complete connection-bound dispatch composition, authored mutation ingress and native MR validation/writes. Public audited operations and business effects remain pending those controls. The operator has been asked to resolve C14 create/update head-guard semantics; no answer or weaker guarantee is assumed. Dedicated GitLab sandbox acceptance remains open. All original acceptance, compatibility, packaging and reproducible-delivery obligations remain required, in the unchanged GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider order.

## Approval proof and spend increment — 2026-09-10

The sixth partial runtime child, story:local-approval-binding, implements the selected canonical proof/signature and durable one-use approval spend boundary over the existing mutation ledger. The prepared attempt gains an optional complete historical approval subject, closing missing authority/origin binding for unkeyed attempts; old records cannot invent it during migration. ConfiguredApprovalKey is a trusted current configuration value, with consumer-owned policy guards and qualified clock evidence required at spend.

This is an independently tested internal proof/spend increment, not public local approval onboarding or a provider write. Protected persistent local issuer-key custody/publication, CLI preparation/approval issuance, concrete caller policy, production clock qualification, generated mutation ingress and complete connection-bound dispatch remain required next GitLab work before native validation/writes and dedicated sandbox acceptance. C14 head-guard clarification remains pending. Shared source/planning/build changes are serialized on primary main; four independent planning critics review this six-child partial decomposition. Full GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider scope and order are unchanged.


## Approval binding checkpoint — 2026-09-10

story:local-approval-binding completes its private proof/spend and approved ledger-gate boundary. docs/evidence/local-approval-binding-20260910/README.md retains 17 approval tests with four actual process exits, the passing Rust 1.88 repository gate, website/reference and 15 example tests, and all five optimized GitLab CLI journeys. Exact source and executable digests, failed checks and eight planning critic records are retained. One missing audit dependency was fixed; all four final critics approve.

Next is protected persistent local issuer-key custody/publication/rotation and actual approval preparation/issuance CLI, with authenticated caller policy and qualified time, followed by generated mutation ingress and complete connection-bound provider dispatch. Native GitLab validation/writes and dedicated sandbox acceptance remain required. The C14 create/update head-guard question is unanswered; no weaker semantics or approval are inferred. GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider scope and order remain unchanged, and this initiative stays active.

## Persistent approval-key checkpoint — 2026-09-10

story:local-approval-keys completes local `approvals key-init`, `key-status`, `key-rotate`, `key-recover`, `key-revoke` and `key-retire`. Migration seven and qualified purpose-separated Secret Service custody retain immutable key identities and exact publication/revocation/retirement fences. Current-key leases exclude management through protected use; they supply no caller or subject authority. docs/evidence/local-approval-keys-20260910/README.md retains the Rust 1.88 gate, eleven key tests with three actual child exits and all six production commands, five optimized GitLab CLI journeys, website/reference checks, fifteen example tests and exact source/tool/executable identities. All four final planning critics approve after three recorded fixes.

Next implement actual local approval preparation/issuance with authenticated caller/subject policy and qualified time, then generated mutation ingress and complete connection-bound provider dispatch. Native GitLab validation/writes, the unanswered C14 create/update head-guard choice and dedicated GitLab sandbox acceptance remain open. The key-management CLI is not an approval-proof or provider-write interface. The initiative remains active and retains the complete GitLab → Kubernetes → PostgreSQL → MCP → remaining-provider order and all original acceptance, compatibility, publication-boundary and reproducible-delivery requirements.

## Pinned MR validation increment — 2026-09-10

story:gitlab-mr-validation adds a native read that compares an exact requested MR SHA and selected head pipeline with bounded current observations through the existing CLI. It does not issue approval proofs, admit mutations or claim exhaustive project-rule validation. Dedicated GitLab sandbox evidence is still required; the existing credential blocker also names this story. Native ESS value validation precedes its decomposition, and all four independent planning critics review its boundaries against the existing children.

Time-source qualification is still unresolved: the researched Roughenough 2.0.0 default radius is five seconds, wider than the selected approval proof's maximum four-second full interval; documentation/authentic signatures alone establish no qualified production clock. No clock dependency or trust configuration was added. Independent native validation proceeds while actual issuance/caller policy, qualified time and full mutation dispatch remain parent-owned next work. The create/update head-guard decision remains unanswered. GitLab stays first, followed by Kubernetes, PostgreSQL, MCP and remaining providers. The initial cleanup is verified: local main contains 4aa3354788172616202b0d7dacea65bd5b9a35b0 and only the primary Connectors checkout exists; no Connectors publication is performed.

## Verified native MR validation checkpoint — 2026-09-10

The local CLI now invokes eleven GitLab read operations, including merge_request.validate. The new read checks an exact requested MR source SHA and selected successful head pipeline with opened/non-draft/mergeable predicates, reports ordered blockers and always reports merge_performed=false. It grants no future write. Twelve native MR tests, the complete Rust 1.88 gate, six production CLI/private HTTPS/qualified-keyring journeys, local GitLab image packaging and website/reference/fifteen example checks pass. docs/evidence/gitlab-mr-validation-20260910/README.md retains exact inputs, artifact identities, failed-test corrections and eight independent critic records; both acceptance findings are fixed and all final critics approve.

story:gitlab-mr-validation stays active pending dedicated GitLab sandbox evidence, alongside the earlier read stories. Next implementation remains actual approval preparation/issuance with authenticated subject policy and a qualified clock, then complete connection-bound mutation ingress/dispatch and native GitLab writes. Clock qualification and the C14 create/update head-guard choice remain unresolved; the validation read does not substitute for those controls. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers, with all original acceptance, reproducible-delivery and publication boundaries retained. This checkpoint is local main integration only; no Connectors publication or Atlas registration is performed.
