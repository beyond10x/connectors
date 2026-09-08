---
format: aep.planning-md/1
id: story:full-review-remediation
kind: story
status: implemented
title: Address the full implementation review
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: story:three-adapters-e2e
- informed_by: story:gitlab-spec-service
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: contracts
- confidence: cited
  path: crates
- confidence: cited
  path: docs
- confidence: cited
  path: spec-kinds
revision: 7
---
## Context

The operator requested remediation of .local/review/2026-09-08-full-review.md after the two local implementation stories completed. This interactive session uses one agent directly in the checkout. Existing declarations in ess/system.yaml and ess/domains/declarations.yaml govern the adapters and service configuration; this work introduces no new domain entities. All changes stay local.

## Scope

Confirmed scope: adapters/sql, crates/connectors-host, crates/connectors-client, apps/connectors, crates/connectors-spec, crates/connectors-build, adapter specifications and generated descriptors, Cargo.toml/Cargo.lock, contracts and documentation. Investigate every numbered finding against current source, fix runtime and coverage defects, and retain explicit evidence for superseded or intentionally bounded behavior. Do not change Atlas, AEP, ESS or the old Connectors repository.

## Acceptance

Custom CA configurations trust only the configured roots. Federation refreshes a stale leaf snapshot while requiring deliberate caller resubmission. Invalid identifiers cannot inject log records. SQL has automated protocol fixtures covering authentication, parsing, limits and sanitized errors. Shared configuration schemas have one editable owner. Replace deprecated direct parser dependencies, make CLI JSON parsing strict, and remove unnecessary retained tokens/descriptor copies. Document actual limits, SQL connection/search-path behavior and toolchain requirements. Provide a Rust gate command, investigate AEP journal identities without editing historical records, and publish a durable disposition for findings 1–17 with test evidence.

## Verification plan

Run targeted regression fixtures, generation/drift checks, workspace formatting/tests/warning-denying Clippy, isolated adapter builds and dependency boundaries through the unified gate. Verify declared Rust compatibility or accurately narrow the claim. Run AEP/ESS validation and record output and any limitations in docs/review-response-2026-09-08.md.

## Verification

The full Rust gate passed on the final implementation: 35 tests passed, none failed or ignored; formatting, descriptor and complete GitLab bundle reproducibility, offline workspace builds, warning-denying Clippy, independent provider library builds and normal dependency boundaries passed. All workspace targets also compile on the declared Rust 1.88.0; runtime tests use Rust 1.98.1. Cargo audit reports 306 dependencies with no vulnerabilities or warnings. ESS 0.9.2 validates the domain. Evidence is in docs/evidence/review-2026-09-08/ and all 17 dispositions are in docs/review-response-2026-09-08.md. AEP's initial missing-scope warning was resolved through the scope command.

The automated additions exercise SQL protocol authentication/input/output/error boundaries, exclusive CA trust and real loopback TLS handshakes, live leaf descriptor replacement and credential rotation with no replay, log escaping, duplicate CLI JSON keys, strict configuration parsing and shared-schema imports. These fixture checks do not claim another live GitLab/k3s/PostgreSQL run or a repackaged image; prior live/image evidence remains tied to its original digests.

## Progress

All review findings have a recorded disposition. Runtime defects, SQL coverage, schema ownership, deprecated direct dependencies, strict CLI input, retained token/descriptor duplication and the unified local gate are addressed. Shared configuration types own their derived schemas; all affected generated descriptors and the GitLab manifest are refreshed. The admitted descriptor revision guards federation route selection during concurrent refreshes.

The per-request SQL connection model remains intentional for this bounded profile, with credential freshness/session-state isolation and its latency cost documented; the fixed search path is explicit. ESS 0.9.2 remains the reproducibility pin, with explicit executable selection when another shell has a different version. The previously missing build crate now exists. Read-only AEP source inspection and both story explanations establish that process-local command target IDs do not identify persistent markdown artifacts; historical journal records remain intact.

This was interactive single-agent work directly in the primary checkout under the project's instruction. No agent delegation, approval bypass, Atlas/AEP/ESS changes, remote publication, container rollout or worktree lifecycle changes were needed. The existing managed design tree remains retained. The final changes and evidence are local.
