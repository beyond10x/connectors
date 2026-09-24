---
format: aep.planning-md/2
id: release-plan:connectors-v090
kind: release-plan
status: implemented
title: Release v0.9.0 from the live-provider GitLab acceptance
relations:
- informed_by: initiative:complete-local-connectors
revision: 4
---
<!-- Starting point for a `release-plan` artifact. There is no `artifacts/kinds/release-plan.yaml` yet,
     so these sections are a suggestion rather than a declared expectation. -->

# Release plan: <version>

## Scope

*What ships: changes, artifacts and the stories they close.*

## Qualification

*The evidence required before the release may start, and who produced it.*

## Sequence

*Ordered steps with the environment each targets, and what is checked between them.*

## Rollout Strategy

*Staging, canary, promotion criteria, and how long each observation window is.*

## Monitoring

*The signals watched during rollout, with the thresholds that stop it.*

## Rollback

*How to reverse each step, and the point after which reversal is no longer possible.*

## Approvals

*Which approvals gate which step, and who may grant them.*

## Communications

*Who is told what, and when — including the failure case.*

## Scope

Connectors v0.9.0, cut from `main` on 2026-09-13. No source file changed since
v0.8.0. What changed is the evidence behind the same code.

A dedicated GitLab 19.3.2 sandbox, operated by a non-administrator with `read_api`
and project role Developer, answered all eleven read operations, a pinned
merge-request validation before and after the head moved, and an approved merge
through the full approval chain. Two owner-crash shapes were exercised against the
provider; a replayed business key issued no second merge in either.

`credential-blocker:gitlab-runtime-sandbox` is cleared.
`configuration-blocker:gitlab-v020-publication-target` is cleared: the destination
is `next` at `beyond10x/connectors`, established when v0.8.0 was pushed and its
release page published.

Four GitLab stories move to `implemented` on this evidence:
`story:persistent-gitlab-journey`, `story:gitlab-ci-runtime`,
`story:gitlab-mr-validation` and `story:guarded-gitlab-merge`.

## Evidence

`docs/evidence/gitlab-sandbox-20260913/README.md`, with artifact identities,
the invocation log, the pipeline observation log and the four merge attempt records.

The full repository gate with MSRV: 35 steps, every one exit 0, 85 test targets.

## What this release does not complete

`story:gitlab-mr-reads` stays open. Deleting an open merge request's source branch
closes it in GitLab 19.3.2, and the mergeability check settles faster than a CLI
process starts, so neither the nullable deleted-source shape nor an unknown merge
status could be produced. The acceptance likely needs rewording against what GitLab
does, which is an operator decision and not a release blocker.

`story:guarded-gitlab-merge` closed with one stated deviation: the lost-response
replay reported `applied` rather than leaving the attempt uncertain, because the
ledger already held the outcome when the owner died.

Helm release reads stay fixture-verified. MCP stays contracts only. Kubernetes and
PostgreSQL keep their own open sandbox acceptance. Source release only: no binary,
container image or website deployment.

## Publication

Local commit and annotated `v0.9.0` tag on `main` here, then the same commits
replayed onto `next` at `beyond10x/connectors` preserving tree, message, author and
dates, the tag recreated at the mapped commit, and the hosted release page published
through `b10x-gates gh` as `b10x-bot[bot]`.
