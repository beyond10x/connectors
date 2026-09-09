---
format: aep.planning-md/1
id: story:local-cli-binding-semantics
kind: story
status: implemented
title: Specify local configuration credential and adapter lifecycle behavior
relations:
- decomposes: epic:local-cli-contracts
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: contracts/cli/v1alpha1/
- confidence: cited
  path: ess/domains/cli.yaml
revision: 8
---
## Context
The observed journeys C01-C05 in docs/recent-adapter-usage-20260909.md require a usable local CLI. docs/design.md sections 17 and 32 require local infrastructure independence. contracts/auth/management.md keeps management with host coordinators; ess/domains/auth_bindings.yaml already models connection/acquisition/custody ownership.
## Acceptance
A reviewed local CLI contract and validated ESS value model specify every selected command's owner, typed input/result, per-adapter startup behavior, protected entry, durable-publication boundary and observable failures, with valid/invalid and lifecycle examples that do not claim production runtime execution.
## Selected behavior
Use setup init/check; adapters list/describe/status/stop; connections list/describe/connect/repair/status/revoke; operations list/describe/invoke. Compatibility describe maps the full service descriptor and invoke preserves current legacy arguments/output; legacy serve stays distinct. Explicit JSON output uses stdout for successful results and stderr for structured errors; exit 0 success, 2 usage, 1 operational failure, 130 interruption. Human mode explains next actions without secrets.
TOML entries select startup on-demand (default) or automatic. Automatic starts at local-host startup. Pure listing neither starts the host nor authenticates. An admitted connect/invoke may start the host and its automatic entries plus the selected on-demand adapter. All launch targets come from explicit configuration, never a discovered path. Concurrent startup coalesces by configured adapter identity; stop addresses only owned incarnations. Local metadata authority and OS keyring are separate owners; keyring failure never publishes a successful connection or silently falls back. Both hidden TTY and protected file/stdin entry are supported; business input is a distinct channel. Local revoke is terminal and does not assert provider revocation.
## ESS
Reuse existing persistent entities; add typed values in ess/domains/cli.yaml only. Do not manufacture an entity or consistency-bearing view for a CLI word. The full presentation binding is integrated by the dependent story using the upstream ess-cli/1 format. New configuration/lifecycle state values describe observations, not a competing persistent ledger.
## Source assignment
contracts/cli/v1alpha1/ (new semantic contract, fixtures and scenario expectations), ess/domains/cli.yaml (new typed values). Coordinator owns ess/system.yaml, indexes, migration docs, planning, gate and generated output. No production runtime code.

## Scope

Derived 2026-09-09 by aep-drive:story-scoper. Primary surface contracts/cli/v1alpha1/ — cited; typed model ess/domains/cli.yaml — cited. Both paths are new and explicitly assigned. Concrete type and fixture names are not assigned yet. Model registration, indexes, migration docs, planning, gates and generated output stay coordinator-owned — cited. No production runtime — cited. Confidence high — cited, explicit source assignment. Would collide with any unit changing either assigned path — inferred.

## Explicit outcome ownership after scope review
This story owns Linux as the first local OS binding and documents the boundary of that support, including keyring availability, protected terminal/file descriptors and owned child processes; it makes no unverified portability claim. Its acceptance explicitly includes still-valid credential reuse after both CLI and local-owner restart, successful same-principal/same-target repair without changing connection identity, different-principal replacement refusal, and repair failure preserving any valid active generation. These are reviewed contract/model scenarios; production runtime conformance remains a separate obligation.
