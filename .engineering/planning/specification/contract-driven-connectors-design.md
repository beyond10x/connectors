---
format: aep.planning-md/1
id: specification:contract-driven-connectors-design
kind: specification
status: draft
title: Connectors v2 contract-driven design handoff
summary: Local design context and specification-extraction guidance for independent adapter services; implementation remains proposed.
revision: 4
---
## Context

The operator requested a fresh local Connectors rewrite design with all context needed by a future implementer, then explicitly invoked the worktree and AEP planning skills. The operator also explicitly deferred Atlas integration. This artifact records that design deliverable; it does not approve a rewrite, schedule the proposed stages, or claim that any adapter has been implemented.

## Design source

[docs/design.md](../../../docs/design.md) is the complete design source. Keep the architecture prose there rather than creating a second editable specification in this record.

It includes the specification-first extraction approach, observed legacy coupling and consumer pins, monorepo boundaries, operations/events/duplex sessions, discovery, configuration, independent authentication and credential custody, SaaS resource assignments, SIP/RTP and RTVBP media, webhooks, federation, local Cargo/service packaging, the Connectors-owned custom ESS adapter kind, provider scope, conformance, migration, open decisions, and implementation order.

## Evidence

- `docs/design.md:26`: operator direction, including local-only scope and explicit deferral of Atlas integration.
- `docs/design.md:51`: recorded source/tool revisions, useful old behaviors, and consumer coupling.
- `docs/design.md:166`: monorepo layout, followed by the proposed contract boundaries through section 18.
- `docs/design.md:969`: conformance strategy and independently testable decoupling.
- `docs/design.md:1021`: proposed implementation order, followed by extraction/migration records and open decisions.
- `docs/design.md:1150`: managed-worktree and local AEP handoff.
- `docs/design.md:1172`: source map and pinned evidence references.
- `aep plan reverse scan --format json`: no packages, tests, CI jobs, or published API surfaces found in this new repository.
- `aep plan reverse history --format json`: one local bootstrap commit, dated 2026-09-08; no implementation history or tickets to migrate.

## Acceptance

A future implementer can begin a bounded specification-extraction slice from docs/design.md without needing the prior conversation, while distinguishing operator decisions, recommended design, existing evidence, and unresolved semantics.

## Scope and status

At the original design handoff, the authored deliverables were docs/design.md, local repository guidance, and this AEP record with its project configuration. No implementation stories had been decomposed, and there was no release, remote, Atlas catalog/roadmap change, or consumer rollout. This historical design artifact remains at its initial draft status; later delivery is recorded separately below.

Before implementation, establish the exact source/tool baseline, create the typed domain specification for the selected slice, retain UNMAPPED markers for unresolved semantics, and create the required governed work records through AEP. The proposed stages in the document are starting context, not an independently maintained backlog.

## Provenance

Created for the operator's local design request in the interactive session recorded 2026-09-08. No legacy backlog was migrated. The implementation observations and their limitations are recorded in docs/design.md rather than asserted as current deployed behavior.

## Subsequent delivery status — 2026-09-08

The later operator-authorized local implementation is recorded in `story:three-adapters-e2e`, `story:gitlab-spec-service`, `story:full-review-remediation`, `story:ess-executable-pin` and `story:ess-pin-upgrade`, all implemented. Design §27–30 describe those slices and the proposed semantic contracts. The mutation/idempotency and auth semantic hardening stories record their own ESS evidence. `story:independent-review-remediation` owns the independent review fixes. These later records do not retroactively change the original handoff scope or approve a release. The only Git remote is now the local bare recovery backup; Atlas integration and external publication remain deferred.
