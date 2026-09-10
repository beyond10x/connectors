---
format: aep.planning-md/1
id: story:local-execution-audit
kind: story
status: implemented
title: Persist execution audit anchors and immutable final observations
relations:
- decomposes: initiative:complete-local-connectors
- depends_on: story:local-mutation-ledger
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/service/audit.md
- confidence: inferred
  path: crates/connectors-host
- confidence: inferred
  path: docs
- confidence: inferred
  path: ess/domains/execution_audit.yaml
- confidence: inferred
  path: website
revision: 6
---
## Outcome

Implement the host's durable execution-audit anchor and one-final-observation port required by GitLab governed writes, starting from local main 04a7e22110d3e5b36d3f39b178d32fd7ee283c0e where execution audit remains specification only.

## Acceptance

After concurrent append attempts and process interruption against a real local SQLite authority, an acknowledged host-qualified audit anchor retains at most one immutable final observation, with exact retry acknowledgement and no recovery-issued execution permit.

## Scope and binding

Existing typed ownership is ess/domains/execution_audit.yaml AuditRecord and its instance, mutation_attempt and selected_connection references; contracts/service/audit.md selects the exact private identity encoding, bounded fields, lifecycle and retention. Pinned ESS validate and compile pass before this decomposition. No new entity or provider semantics are introduced.

Bind this separate logical port to the existing private WAL/FULL metadata authority. Migration 5 adds bounded retained audit records and optional independently retained connection/attempt references while preserving migration bytes 1–4 and their identities. Only admitted audit anchoring installs version 5; ordinary setup and passive inspection do not install the unused port. Existing mutation preparation must continue to work on both versions 4 and 5.

Allocate the public opaque audit ref inside the host owner and derive the exact contract-selected tagged length-prefixed base64url private identity. Check all declared byte limits, strict safe field shapes and same-instance/connection references. No record holds provider output, input, credential, approval proof or native target. Timestamp values are host-observed Unix milliseconds for audit description, never evidence of current authorization or retention expiry.

Return a process-local, non-Clone admission receipt only after definite anchor commit acknowledgement. Early-refusal records and internal read/recovery cannot create an execution receipt. The receipt proves only audit acknowledgement and exact anchor facts; the eventual coordinator must still enforce current caller policy, approval, connection/credential and mutation gates. Anchor failure or uncertain acknowledgement provides no receipt and no provider work. Public responses and the CLI remain the later coordinator's responsibility.

Append one final observation in a separate acknowledged transaction, borrowing the caller's exact observation so a storage failure cannot consume or alter its live business answer. Same observation id and exact semantic bytes recover the first acknowledgement; any changed field or different id conflicts after finalization. Internal exact-pair lookup distinguishes absence from unavailability and supplies no dispatch authority. Retain anchors and final observations without automatic expiry or deletion; bounded per-instance capacity refuses new anchors and never evicts another owner's evidence.

## Verification prerequisites

Use task-owned real SQLite databases for exact instance/ref identity and noncanonical decoding refusal; same-ref different-instance isolation; anchor rollback and lost acknowledgement; immutable concurrent final append and exact retry after lost acknowledgement; early-refusal versus admitted receipt shape; retained references across revoke and mutation replay retirement; mismatched instance/connection refusal; capacity and all field/aggregate bounds; old-schema inspection and admitted migration continuity; four abrupt child-process exits around anchor and final commits. Exercise simulated dispatch only to demonstrate receipt ordering; this is not provider runtime acceptance. Run the required cargo run --locked --offline -p connectors-build -- gate --msrv and affected reference/website checks, using task-owned TMPDIR and two Cargo jobs. Retain results and limitations in docs/evidence/local-execution-audit-20260910.

## Dependencies and sequencing

The implemented story:local-mutation-ledger supplies the referenced durable attempts and schema-4 baseline. Existing persistent GitLab, CI and MR reads remain separate owners with their dedicated sandbox acceptance open. This private audit port is independently demonstrable and does not claim the complete write coordinator. The parent retains local approval issuance/verification/spending, production clock qualification, generated mutation inputs, connection-bound dispatch, native MR validation/write semantics and dedicated sandbox acceptance. The open C14 create/update head-guard decision does not block this shared port.

One implementation and planning writer works directly on primary main; no concurrent implementation is scheduled. Inferred source scope is crates/connectors-host, including local/metadata.rs, local/mutations.rs and new local/audit modules, plus Cargo.lock, docs, contracts/service/audit.md, ess/domains/execution_audit.yaml status wording and website/publication.json. These overlap the earlier runtime stories and are explicitly serialized, as are builds and CLI fixtures sharing executables. Four independent planning critics remain read-only. This interactive increment has no approval bypass or paid governed run. Full GitLab still precedes Kubernetes, PostgreSQL, MCP and remaining providers; all original runtime and reproducible-delivery obligations remain parent-owned.

## Verified private audit checkpoint — 2026-09-10

The private audit port in crates/connectors-host/src/local/audit.rs, audit/types.rs, audit/tests.rs and metadata/audit.sql is implemented. It independently acknowledges host-allocated anchors and one immutable final observation, compares exact public-pair/private identity and retained references, and supplies no recovery-issued execution receipt. Migration five preserves earlier migration bytes and identities; ordinary setup retains version three and mutation operations work on versions four and five.

The final required gate passes, including Rust 1.88, shared/native ESS, parser and descriptor drift, reproducible generation, conformance, workspace tests, Clippy and dependency boundaries. Twelve new audit tests include eight-way races, fault injection, four abrupt subprocess exits, exact final retry, capacity, byte bounds, reference retention and migration continuity. Website build/typecheck and reference checks pass (114 indexed pages, 468 audited public files, no reference drift). All five optimized CLI/private HTTPS/keyring journeys pass in 85.32 seconds. docs/evidence/local-execution-audit-20260910/README.md retains exact source/artifact identities, commands and failed intermediate gates.

The ESS boundary rejected infrastructure wording in the shared summary, which was corrected without changing its model. Clippy then required boxing the retained audit record; the final required gate passes that correction. Four independent planning critics approved round one with zero findings; there is no second round or review outcome to invent. Machine-readable scope is now recorded; the validator's existing empty-findings notices remain verbatim. Sonnet unavailability and the three-slot scheduling deviation are recorded. No approval bypass or paid governed run occurred.

This completes the deliberately internal port acceptance. Public audited responses and business writes remain unadvertised pending the full coordinator. Local approval issuance/verification/spending and qualified clock/dispatch composition are next. The native C14 create/update guard question has been sent to the operator and remains unresolved; missing dedicated GitLab sandbox evidence remains a separate blocker. Full GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider order is unchanged. No Connectors publication or deployment occurred.
