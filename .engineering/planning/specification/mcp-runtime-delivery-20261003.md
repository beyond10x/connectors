---
format: aep.planning-md/3
id: specification:mcp-runtime-delivery-20261003
kind: specification
status: draft
title: Deliver local inbound and persistent outbound MCP after library lifecycle
relations:
- informed_by: epic:mcp-contracts
revision: 3
---
## Delivery state and scope

AEP implementing skill 0.19.1. Operator authority remains
approval-record:milestones-delivery-20261002 and the explicit two-wave/full-MCP
instructions. This record preserves the full outcome: local authenticated outbound
MCP with restart-safe credential reuse; local inbound server with admitted operations;
selected cloud contract/isolation fixture; source verification and a new release.
A library or parser checkpoint does not complete that outcome.

The sibling library's HTTP lifecycle is implemented and published through MCP PR12:
https://github.com/beyond10x/mcp/pull/12. Candidate 960aaf30345444dd0961818b30d9c65801fe2ea7
merged as 0e3ff5f677b914cdf86c1f39f12b3abf04b4b266 with the same candidate tree
72487a7aad23931095e075df843cf202246f46d6. Its local Rust1.88 gate passed116 native
functions; HTTP lifecycle105, worker15 and typed lifecycle31 are separate ESS
selections. All PR checks passed; main source gate was still running at this record.
This is not a new source release or evidence Connectors uses the library yet.

## Current unit and evidence locations

Coordinator: codex-cb26l-runtime, sole AEP writer. Branch unit/mcp-local-runtime-20261003,
base af8aefad3c83195068e68e606d174f9b125d091c, managed worktree id cb26l-runtime.
Resolve the task checkout through worktree inspect; exact machine-local paths remain
in the private handoff. Scratch is repository-relative .local/mcp-runtime; builds
use this tree's target. No shared target or primary edits. All committed running
code is Rust. Existing workers are quota-exhausted; no replacement worker or
independent review claim is made. Work proceeds through disclosed coordinator passes.

Stage: native launch/name/framing/session-reducer/lease components integrated.
Full connectors-build gate --msrv passed on2026-10-03 with1285passing tests and
65existing ignored tests. Exact pinned generation matches. The lossless JSON decoder
is tested in an isolated probe and not yet linked. Runtime entry, protected host
admission/audit interface, actual inbound/outbound journeys and final release remain
unfinished. No runtime command is advertised and compatibility remains deferred.

## Next units, preserving dependencies

1. Native launch and framing: finish the native parser/type projection and drift
   gate; wire a production stdio entry to the existing local owner boundary, with
   real frames and stdout isolation. Reuse shared session lifecycle and lease
   semantics explicitly. Add named actual-process tests for owner refusal, malformed
   and truncated input, both revisions, required capabilities, finite bounds and
   connection loss. Do not substitute an always-empty server for admitted discovery.
2. Local inbound projection: tools/resources/prompts from implemented, configured,
   enabled and metadata-admitted operations; current owner entry and target admission
   before each execution. Implement full result/error, cancellation, progress and
   lost-reply/mutation semantics from the landed contracts. Mutation advertisement
   stays withheld until its protected approval and durable key binding exists.
3. Persistent outbound HTTP: select and review the cohesive native projection in
   specification:mcp-local-http-binding-design-scope before runtime uses it. Bind
   configured instance, exact endpoint/resource/issuer, profile, Connection selection,
   credential publication/custody, revocation and alias exclusion under the existing
   linearizable authority. Select snapshot/session lifetime explicitly. Then use
   the published strict library with real authentication, restart, repair and revoke
   evidence. Anonymous-only or memory-only exchange cannot satisfy the requirement.
4. Compose inbound with outbound without forwarding caller credentials, preserving
   target provenance, effect policy and all budgets; update actual CLI discovery,
   docs and compatibility only after working journeys. Run required gates, integrate
   and cut/verify the new version and hosted source release.

These are dependent runtime stages, not a claim the earlier two independent contract
waves are still undispatched. New runtime stories follow the actual validated models
and reviewed ports; no story status is advanced from a probe alone.

## Decisions still open

Multi-caller cloud caller-to-Connection assignment remains the existing decision
blocker; outbound stdio spawning/attachment ownership remains its separate blocker.
Neither blocks local single-owner inbound stdio or outbound HTTP. The five native
configuration/custody/snapshot/session questions are design work under existing
authority, not new operator approval gates. Cloud deployment, Harness repinning and
unrelated provider expansion are not included. Existing provider acceptance and
external-family/rollback deferrals retain their earlier records.

## Sibling main verification

The exact PR12 merge 0e3ff5f677b914cdf86c1f39f12b3abf04b4b266 passed main
Rust1.88 gate 37115341130, shared source gates 37115341678, documentation check
37115341162 and passive bundle 37115341125. Author is b10x-bot[bot], the App merged
it, and GitHub's merge committer is the permitted web-flow identity. Candidate and
merge trees match. Two unstaged AEP evidence records were found after publication;
they are being published in a separate bot follow-up, not silently discarded.

Native launch preparation now has draft story:mcp-local-stdio-runtime. It covers
the full selected local runtime, actual owner entry and real admitted capabilities,
not an empty-list surrogate. The existing shared session lifecycle/lease contract
has no reusable Rust runtime found by the source audit; runtime port/state-machine
implementation must be proved rather than inferred from model declarations.
