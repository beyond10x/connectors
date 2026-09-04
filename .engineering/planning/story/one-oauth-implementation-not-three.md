---
format: aep.planning-md/1
id: story:one-oauth-implementation-not-three
kind: story
status: implemented
title: One OAuth implementation, not three
refs:
- provider: legacy
  reference: S-069
relations:
- derived_from: epic:oauth-consolidation
scope:
- confidence: cited
  path: crates/integration-gitlab
- confidence: cited
  path: crates/integration-jira
- confidence: cited
  path: crates/integration-slack
- confidence: cited
  path: crates/service
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-069-one-oauth-implementation-not-three.md:21`. **read**

- [x] A new `crates/connector-oauth` carries: state generation with a single-use TTL map, S256
      challenge derivation, authorize-URL construction, bounded token exchange with per-field
      validation, and refresh with a configurable skew under double-checked locking.
- [x] `integration-gitlab` migrates first and is the extraction source — it is the only current
      PKCE implementation (`crates/integration-gitlab/src/backend.rs:678-687`, `:739-750`, `:752`,
      `:806-855`). Its behaviour is unchanged, proven by its existing tests passing untouched.
- [x] `integration-jira` migrates second. Its refresh path is the best in the repo
      (`crates/integration-jira/src/backend/auth.rs:479-518` skew + double-checked lock,
      `:416-451` crash recovery) and the extracted crate must not regress it — if the shared shape
      cannot express it, the shared shape is wrong.
- [x] `integration-slack` migrates third, keeping its egress-gate routing
      (`crates/integration-slack/src/backend/api_runtime.rs:43-179`) rather than a bare client.
      **Partially, on purpose** — only the pending-state table moved; see the note below. Two of
      three is the honest result, not a shortfall hidden behind a tick.
- [x] **Behaviour-preserving.** Jira and Slack stay non-PKCE; neither provider declares
      `public_client`, so adding PKCE here would be an undeclared change to a live integration.
- [x] One commit per integration, gate green between each, so a regression bisects to one vendor.
      `f7149ab` GitLab, `ce04be1` Jira, `d9fc657` Slack. The first draft landed all three as one
      commit; independent review caught it and the branch was split before it reached `main`.
- [x] The prepared-transaction commit path stays the only way a token pair is persisted; no point
      write appears in the extracted crate.

## Context

Replace three hand-rolled OAuth implementations with one, so the next connector that needs a
browser flow inherits PKCE, single-use state, bounded exchange and crash-safe refresh instead of
copying whichever neighbour it read first.

Source frontmatter: pillar Platform · areas [service, integration-gitlab, integration-jira, integration-slack] · priority 1. **read**

Source `note:` field, quoted: “GitLab, Jira and Slack each hand-rolled authorization-code OAuth; only GitLab does PKCE. Extract one crate from the GitLab implementation and migrate the other two onto it, behaviour-preserving. Prerequisite Timo chose for the Claude connector work; also unblocks S-013's loopback callback.”

## Status

`done` in the source. Quoted from `docs/stories/S-069-one-oauth-implementation-not-three.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-069-one-oauth-implementation-not-three.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-08-25 · 3 revision(s)
- Legacy id `S-069`, recorded as the reference `legacy:S-069`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
