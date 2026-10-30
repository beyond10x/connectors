---
format: aep.planning-md/3
id: story:catalog-reads-stay-inside-the-instance-repositories
kind: story
status: implemented
title: Catalog reads of a GitHub App instance reach only the repositories the instance lists
summary: 'D275: reads reached all 74 repositories the installation sees.'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/served.rs
- confidence: cited
  path: adapters/catalog/src/served/git.rs
- confidence: cited
  path: adapters/catalog/tests/served.rs
- confidence: cited
  path: docs/running-services.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T13:43:06Z", actor: "human:timo", revision: 9, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "proposed", to: "active", at: "2026-10-30T13:43:06Z", actor: "human:timo", revision: 10, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "active", to: "implemented", at: "2026-10-30T15:04:49Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: catalog-reads-stay-inside-the-instance-repositories

## Outcome

A caller of a served GitHub App instance configured for a set of repositories can read only those
repositories, not every repository the App installation sees.

## Context

D275, from the live-cluster rehearsal of 0.62.0: the installation saw 74 repositories, and reads
reached all of them, while the git relay (`adapters/catalog/src/served/git.rs`, `git.repositories`)
is limited to its list. Builds on story:catalog-github-app-installation-token and
story:git-smart-http-relay-with-quarantine.

## Acceptance

- A served catalog instance takes an optional `catalog.repositories`, a non-empty list of
  `<owner>/<repo>` parsed and matched as `git.repositories` is (case-insensitive, `.git` stripped);
  an invalid or empty list refuses at start.
- With the list set, an operation whose input has both `owner` and `repo` members is admitted only
  for a listed repository; any other value answers `not_found`, the same answer as a missing
  repository, and no upstream request is sent (and no credential read).
- With the list set, every other operation (no `owner` and `repo` input: searches, notifications,
  assigned issues, organization listings, the user read) is refused and hidden from describe,
  because filtering a page would break the "a shorter page ends the walk" rule.
- Writes take the same check as reads.
- Without the list nothing changes. The relay keeps `git.repositories`; docs/running-services.md
  says the two keys and that the relay list is not read for operations.

## Out of Scope

- A repository list for providers other than GitHub.
- Per-operation allow lists.

## Ambiguities

- Whether the list is the existing `git.repositories` or a new instance-level key:
  `inferable` from the served config shape; the scoper says which.

## Open Questions

None.

## Scope

Derived 2026-10-30 by `story-scoper`, condensed by the coordinator. Every line is **cited** or **inferred**.

- **Files:** `adapters/catalog/src/served.rs:91-108` (`Catalog`, `deny_unknown_fields`: the new `catalog.repositories` key), `:127-141` (`run`), `:360-466` (`Bridge`, `described`), `:475-499` (`Bridge::invoke`: check before `credential.read()` at `:485`), `:509-521` (`Bridge::write`) — cited / inferred placement
- **Files:** `adapters/catalog/src/served/git.rs:136-147,232-248,333-339` (`Repository` parsing and matching to share) — cited
- **Tests:** `adapters/catalog/src/served.rs:591-827` (`only_a_listed_read_reaches_the_adapter_and_only_with_a_credential` at `:780`), `adapters/catalog/tests/served.rs` — cited
- **Documents:** `docs/running-services.md:93-130` (Catalog instances), `:262-301` (Git relay) — cited
- **Confidence:** medium (the key's placement is a coordinator decision; the live definition is not in the tree)
- **Ambiguity resolved:** a new `catalog.repositories` key, not `git.repositories`, because `GitConfig` switches the relay on (`git.rs:84-109,246,280-286`) — cited
- **Safety fact:** filtering a listing page breaks the walk's "a page shorter than per_page ends it" rule (`github.yaml:20,25,29,34` in the 0.5.2 fixture), so cross-repository reads are refused rather than filtered — coordinator decision
