---
format: aep.planning-md/1
id: release-plan:connectors-v030
kind: release-plan
status: draft
title: Release v0.3.0 from the merged local Connectors increment
relations:
- informed_by: initiative:complete-local-connectors
revision: 1
---
## Scope

Connectors v0.3.0, cut from `main` on 2026-09-12, 74 commits after v0.2.0.

Guarded GitLab writes, Kubernetes and PostgreSQL through the persistent local
lifecycle, Helm release reads, the catalog ingest-to-bundle track, source-identity
tool pinning, and the first two MCP contract records. `CHANGELOG.md` § 0.3.0
carries the user-visible list.

## Evidence

The full repository gate with `--msrv` on the merged `main`, every step exit 0.
Retained at `docs/evidence/release-v030-20260912/gate.log`.

Two constituent waves gated separately before merging, and both logs are retained
in the repository: `docs/evidence/helm-reads-20260912/gate.log` (33 steps, 71 test
targets) and `docs/evidence/mcp-contracts-20260912/gate.log` (34 steps, 74 test
targets).

This release is the first to gate the **merged** state. Both wave gates ran on
their integration branches, below the merge commits, so until this run nothing had
checked what `main` actually holds.

## What this release does not establish

- **No dedicated GitLab sandbox evidence.** All five GitLab stories stay `active`
  behind `credential-blocker:gitlab-runtime-sandbox`. The guarded merge and its
  write controls are verified by disposable CLI journeys against a private HTTPS
  fixture, not against a GitLab server.
- **Helm release reads are fixture-verified.** No real cluster has answered those
  four operations, and the website entry says so.
- **MCP is contracts only.** No connection, no server, no persisted credential, no
  selected transport. Ten of that epic's twelve stories are undrafted.
- Helm families B and C stay outside Connectors pending
  `decision-blocker:helm-execution-family`.

Kubernetes and PostgreSQL are the exception: both were verified against real
servers on 2026-09-11 — k3s `v1.31.5+k3s1` via k3d and `postgres:17` — and that run
found a defect no fixture had.

## Publication

**Blocked, and the release is therefore incomplete.**
`configuration-blocker:gitlab-v020-publication-target` is open: this checkout has
no publication remote. Its only configured remote, `ess-recovery`, is a local
recovery repository, and `AGENTS.md` § Workspace states that a local recovery
remote is not public distribution.

`AGENTS.md` § Cutting a release step 6 requires pushing `main` and the exact tag to
an authorized source remote through `b10x-gates bot`, and says plainly that a local
commit or local tag alone is not a cut release. Steps 1 through 5 are complete;
step 6 has no destination.

A hosted release page is separately excluded by the same section and is not
possible here in any case: no remote points at a GitHub host.

## What would clear it

One sentence from the operator: the source remote URL, or a statement that
`ess-recovery` is the intended destination for this release. The same input clears
`configuration-blocker:gitlab-v020-publication-target`, which has blocked
`release-plan:gitlab-v020` since 2026-09-10.
