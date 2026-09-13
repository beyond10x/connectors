---
format: aep.planning-md/1
id: release-plan:connectors-v0100
kind: release-plan
status: active
title: 'Release v0.10.0: GitLab writes from the pinned source through the catalog provider'
relations:
- informed_by: initiative:complete-local-connectors
revision: 2
---
## Scope

Connectors v0.10.0, cut from `main` on 2026-09-13. The GitLab batch handoff: the
catalog provider runs merge-request create and update from the pinned OpenAPI
document with no adapter code per endpoint, and the native adapter gains a raced
`merge_request.update`. `story:gitlab-mr-reads` closes on the nullable-source and
conflict reads. `decision-blocker:gitlab-mr-create-update-head-guard` is cleared
with the accepted race boundary and the finding that the postflight comparison is
not detection.

Ships: `crates/connectors-catalog` (YAML ingest, raw template resolution),
`adapters/catalog` (`connectors-catalog-provider`), `connectors-build catalog`,
the committed GitLab bundle (1,847 operations), `WriteMethod`/`send_json` in the
SDK and host, `post` in the v3 write generator, `docs/local-catalog-provider.md`,
and the evidence under `docs/evidence/gitlab-sandbox-20260913/`.

## Qualification

The full repository gate with MSRV on the release commit, and the live sandbox
runs recorded in the evidence README: through the catalog provider, reads,
create applied (MR 10), stale pin refused, 409 refused, update applied, update
stale refused, raced create uncertain (MR 11). Through the native adapter, the
raced update runs recorded earlier the same day.

## Sequence

1. Gate with MSRV green on the exact tree.
2. Website typecheck and build green.
3. Planning store: this plan active; initiative checkpoint recorded.
4. Bot commit on `main`, annotated `v0.10.0` tag at that commit.
5. Replay onto `next` at `beyond10x/connectors` preserving tree, message,
   author and dates; recreate the tag at the mapped commit; verify tree
   equality and tag ancestry.
6. Hosted release page for the tag through `b10x-gates gh`, as `b10x-bot[bot]`,
   Latest.
7. Record the page as evidence here; move this plan to implemented.

## What this release does not complete

`specification:catalog-http-runtime-handoff` stays draft: one provider has run
through the engine, no TOML-authored action has executed, the bundle carries no
request/response schemas or header parameters, and one auth profile is offered.
Helm reads stay fixture-verified; MCP stays contracts only; Kubernetes and
PostgreSQL keep their open sandbox acceptance. Source release only.

## Rollback

A local tag and commit can be dropped before the push. After the push the tag is
immutable; a defect is fixed by a later release, never by moving the tag.

## Publication

Local commit and annotated `v0.10.0` tag on `main` here, the same commits replayed
onto `next` at `beyond10x/connectors`, the tag recreated at the mapped commit, and
the hosted release page published through `b10x-gates gh` as `b10x-bot[bot]`.
