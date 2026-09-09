---
format: aep.planning-md/1
id: specification:recent-agent-adapter-usage-20260909
kind: specification
status: draft
title: Recent agent adapter usage and specification coverage
relations:
- informed_by: specification:core-model-closure-20260909
- informed_by: specification:contract-driven-connectors-design
revision: 3
---
## Purpose

Analyze the operator's local Claude and Codex session events from 2026-09-04T10:37:24Z through 2026-09-09T10:37:24Z. Extract actual adapter-related invocations, including Fluxplane/fluxplane-plugin and current Connectors CLI use, then compare observed workflows and failures with the reviewed Connectors v2 specification baseline.

## Scope and handling

One checkout writer. Read local session stores only, including agent child sessions and older sessions resumed within the window. Do not replay any recorded command or call external provider APIs. Deduplicate copied tool-call history and distinguish actual executions, discovery/help, development/testing and incidental mentions. Preserve result uncertainty when only a wrapper call, missing output or incomplete asynchronous execution is observable.

Keep source locators and extraction evidence in ignored local storage. The repository report contains sanitized use cases, failure classes, proposed invariants, specification coverage and explicit gaps, with opaque evidence identifiers. Do not publish private session bodies, credentials, customer data or host/project identifiers. Any temporary extraction program is Rust; no new permanent helper is part of the deliverable.

## Acceptance

An evidence-backed report identifies the minimum workflows the replacement must support, where current shared/native contracts already specify them, where they are partial or absent, and which scenario/specification work should come next. State the exact scan interval, corpus and selection counts, deduplication policy, unresolved outcomes and limitations. No runtime implementation or external publication is authorized by this analysis. Formal ESS changes and implementation-story decomposition follow reviewed semantic decisions.

## Operator direction during analysis

The operator clarified the intended common workflow: run a local CLI and persist credentials locally. This milestone requires no cloud control plane or federation. Local credential custody and execution must remain replaceable capability bindings so a later remote binding can support the same user workflows; that future binding is not required now.

Evaluate coverage against that local-first baseline, including stable local connections/profiles, credential reuse across CLI invocations and process restarts, clear active configuration/state selection, and local adapter discovery/invocation. Remote provider APIs remain ordinary integration targets; they do not imply hosted Connectors orchestration. Preserve the separation between generic interfaces and provider-specific authentication semantics.

## Analysis delivered

The local evidence report is docs/recent-adapter-usage-20260909.md, with a sanitized command-site archive, action CSV, separately reviewed programmatic invocation CSV, scan summary and verification under docs/evidence/recent-adapter-usage-20260909/. docs/design.md section 32 records the operator's local CLI and persistent local credential direction.

The fixed-window scan covers 6,268 JSONL files and identifies 86,673 unique in-window tool-call identities. Static shell analysis recognizes 7,723 integration command sites in 4,928 tool calls across 103 session identifiers; 39 selected helper sites are separate. Counts include help/development/conditional syntax and are not provider request or success counts. Programmatic Confluence/Jira/GitLab calls and their asynchronous continuation results are reviewed separately. The report discloses malformed records, dynamic/unresolved commands, truncation and incomplete background outcomes.

Delivered 16 workflow coverage rows, 15 reviewed failure classes, 17 proposed invariants and 22 proposed acceptance scenarios. Key required gaps include local management/custody persistence, MySQL/Aurora, GitLab CI/MR and incremental reads, Jira JQL, Confluence collection compatibility, Slack threads/writes, monitoring discovery/instant/derived metrics, Kubernetes/Docker execution and the observed GitHub/AWS workflows. Proposed priority order does not claim runtime completion or silently remove used workflows from parity.

The operator's additional MCP requirement is tracked separately as epic:mcp-contracts: authenticated outbound MCP from the CLI and inbound MCP via `$BIN server` for local clients or optional cloud placement. Protocol selection, auth/capability/lifecycle mapping, applicable ESS modeling, independent review and conformance are required before implementation decomposition.

No runtime or ESS contract/model changed, no historical provider command was replayed, and no external integration was called. Raw source locators and payloads remain in ignored private storage. The extraction utilities are temporary Rust; no permanent helper was added. No commit, push or deployment was performed.

## Verification

The inventory verifier confirms 7,762 unique exported sites including the selected helpers, known in-window evidence IDs, reconciled client totals, 332 distinct cited evidence IDs, 29 relative links, all expected workflow/failure/scenario rows, and the restricted metadata-only export schema. Gzip integrity and git diff --check pass. Manual privacy review and credential/private-identifier scans cover tracked report/evidence; the public AEP log normalizes the repository root only. AEP store validation retains its existing historical review warnings; no new implementation or conformance success is inferred from planning validity.
