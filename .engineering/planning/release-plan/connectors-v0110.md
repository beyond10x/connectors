---
format: aep.planning-md/2
id: release-plan:connectors-v0110
kind: release-plan
status: implemented
title: 'Release v0.11.0: GitLab served by the catalog provider only; native adapter retired'
relations:
- informed_by: initiative:complete-local-connectors
- delivers: epic:retire-native-gitlab-adapter
revision: 3
---
## Scope

Connectors v0.11.0, cut from `main` on 2026-09-14. GitLab has one runtime: the
catalog provider serves every operation the native adapter carried, from the
pinned OpenAPI source and the shipped selection set
`adapters/catalog/providers/gitlab/operations.json`, and the native adapter is
deleted. This closes `epic:retire-native-gitlab-adapter`.

Ships: the shipped selection set and `operations_file` (format
`connectors-catalog-local/2`); multi-check guards with literal expectations and
the declared five-check merge guard; `response: text`; the deletion of
`adapters/gitlab/{src,tests,generated,contracts,realizations,spec}`, the
`connectors-gitlab` crate, the v3 write generator and the `connectors.adapter/v3`
kind; `adapters/catalog/tests/local_runtime.rs` (host-child checks over the
catalog provider); `adapters/catalog/realizations/local.json`; the walkthrough
validated against the shipped `issues.list`; docs and website re-pointed.
`adapters/gitlab/upstream/` stays as the pinned source.

## Qualification

- Gate with MSRV on the removal tree: all checks passed, 78 test targets, 0
  failed (`docs/evidence/gitlab-retirement-20260914/gate.log`); the descriptor
  loop prints `kubernetes` and `sql`, the boundary loop adds `catalog-provider`;
  `cargo tree -p connectors-gitlab` finds no package.
- Live sandbox on the post-removal provider
  (`docs/evidence/gitlab-retirement-20260914/`): 11 reads 200, the trace as
  text; merge on the merged MR 10, merge on the missing MR 999 and update on MR
  10 each refused before dispatch, `not_attempted`, 0 PUTs in GitLab's access
  log. The applied merge, create and update are the 2026-09-13 record
  (`docs/evidence/gitlab-sandbox-20260913/`) with unchanged engine code.
- Website typecheck, build, reference check and example tests on the release
  tree; the gate with MSRV again on the exact release commit.

## Sequence

1. Gate with MSRV green on the exact release tree.
2. Website checks green.
3. Planning store: this plan active; `story:remove-native-gitlab-adapter` and
   the epic implemented on the evidence above.
4. Bot commit on `main`, annotated `v0.11.0` tag at that commit.
5. Replay onto `next` at `beyond10x/connectors` preserving tree, message,
   author and dates; recreate the tag at the mapped commit; verify tree
   equality and tag ancestry.
6. Hosted release page for the tag through `b10x-gates gh`, as `b10x-bot[bot]`,
   Latest.
7. Record the page as evidence here; move this plan to implemented.

## What this release does not complete

`specification:catalog-http-runtime-handoff` stays draft: one provider through
the engine, no TOML-authored action executed, no request or response schemas in
the bundle, header and cookie parameters not carried, one token-header auth
profile. Of the 1,847 GitLab operations, 53 need multipart or form bodies and 32
answer binary or YAML, which the engine does not send or read; 3 are HEAD. The
production CLI mutation journeys (settlement, owner crash, background recovery,
revocation) have not been re-run with the catalog provider as the child
(`story:catalog-cli-journeys`). GitLab has no standalone HTTP service any more,
so the live conformance runner covers Kubernetes and PostgreSQL. Helm reads stay
fixture-verified; MCP stays contracts only; Kubernetes and PostgreSQL keep their
open sandbox acceptance. Source release only.

## Rollback

A local tag and commit can be dropped before the push. After the push the tag is
immutable; a defect is fixed by a later release, never by moving the tag.

## Publication

Local commit and annotated `v0.11.0` tag on `main` here, the same commits replayed
onto `next` at `beyond10x/connectors`, the tag recreated at the mapped commit, and
the hosted release page published through `b10x-gates gh` as `b10x-bot[bot]`.
