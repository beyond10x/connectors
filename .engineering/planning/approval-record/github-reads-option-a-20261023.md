---
format: aep.planning-md/3
id: approval-record:github-reads-option-a-20261023
kind: approval-record
status: draft
title: github-reads lands as an engine fix here and a GitHub definition in connectors-definitions
summary: The orchestrator's decisions of 2026-10-23 on github-reads fix pass 1, the no-split rule and the release
relations:
- decides: story:catalog-github-reads
revision: 1
---
## Stop

On 2026-10-23 the directing session `b10x-chef-orchestrator` decided how github-reads fix pass 1 lands, under its grant in the operator's workspace instructions. It based the decision on this session's report. That report found three things. `unit/github-reads` (396a146f8) adds `adapters/catalog/providers/github/`, against architecture-decision-record:provider-definitions-and-runtimes D1 and epic:catalog-connectors-only-in-definitions. connectors-definitions had no `github.yaml`. The branch was 271 commits behind `main`.

## Decisions

- D1: option A. The connectors engine PR carries F1, so a 403 or 429 with `Retry-After` maps to `rate_limited` (`spent()` in `adapters/catalog/src/lib.rs`, the contract row, `selection.yaml`), tested with a synthetic definition in `adapters/catalog/tests/engine.rs`. It merges through the bot route once green.
- D2: story:catalog-github-reads is accepted. It implements the accepted ADR D1 and changes no ADR. This record is the approval.
- D3: GitHub becomes `connectors/github.yaml` in connectors-definitions, with F2 (the cap) and F5 (the descriptions). The `connectors-definitions` session owns that PR (https://github.com/beyond10x/connectors-definitions/pull/9); this session reviewed it.
- D4: branch `unit/github-reads` (396a146f8) is kept, unmerged and superseded by D1 and D3. Deleting an unmerged branch is not this session's to do.

## Rule: one provider per service

Timo, 2026-10-23: "we once decided we dont split providers into $P-(read|write)", and "i think gitlab or jira was the same, there we took this back already, so dont introduce that pattern now in another prodicer". No provider in connectors or connectors-definitions is split into a read provider and a write provider. A service has one definition file, and its read and write operations live in it. There is no per-direction provider, file, bundle or instance pair. A story id may say `-reads` or `-writes` because it names work, not a provider. The rule goes into `AGENTS.md` in its own PR, citing ADR connector-vocabulary:56.

## Release

The orchestrator approved a connectors release under its grant (class C: release), to be cut once this F1 PR has merged and `main` CI is green. If F1 has not merged within 2 hours of 2026-10-23, `main` is released without it.
