---
format: aep.planning-md/2
id: story:remove-native-gitlab-adapter
kind: story
status: implemented
title: Delete the native GitLab adapter and re-point everything to the catalog provider
relations:
- decomposes: epic:retire-native-gitlab-adapter
- derived_from: specification:catalog-http-runtime-handoff
- serves: vision:independent-contract-adapters
revision: 4
---
## Outcome

GitLab has one runtime. The native adapter under `adapters/gitlab/` (crate
`connectors-gitlab`, its v3 specification, generated tree, tests and contracts)
is deleted, and everything that named it is re-pointed to the catalog provider
and the shipped selection set `adapters/catalog/providers/gitlab/operations.json`.
`adapters/gitlab/upstream/` stays: it is the pinned source the bundle is compiled
from.

## Scope

- Delete `adapters/gitlab/{Cargo.toml,src,tests,generated,contracts,realizations,spec}`.
- `Cargo.toml`: drop the member and the two generated-Rust excludes.
- `crates/connectors-build`: gate adapter lists become `kubernetes`, `sql` and
  `catalog-provider`; `package` defaults to a new
  `adapters/catalog/realizations/local.json`; the website walkthrough fixture is
  validated against the catalog provider's declared `issues.list`.
- `crates/connectors-spec`: the v3 write generator (`src/v3.rs`, `src/v3/`,
  `tests/write_generation.rs`) is deleted, decided by the operator on
  2026-09-14; the v2 generator keeps its tests against a frozen GET-only fixture
  `tests/fixtures/gitlab-v2.json`; the two tests that need a committed generated
  tree go.
- `crates/connectors-conformance`: the GitLab slice calls the catalog selections
  with the source's parameter names; the project-allowlist refusal check is
  dropped because the catalog provider has no allowlist.
- Website: the four `/adapters/gitlab/contracts/*` routes and their tests go;
  `/adapters/gitlab` describes the catalog-served surface; walkthrough links move
  to `/adapters/catalog`.
- Docs: `local-gitlab-cli.md`, `gitlab-generation.md` and
  `gitlab-write-failure-matrix.md` are deleted, their still-true parts folded
  into `local-catalog-provider.md` and `development.md`; every other mention
  outside `docs/evidence/` and `.engineering/` is re-pointed.

## Acceptance

- `cargo tree -p connectors-gitlab` fails: no such package.
- The repository gate with MSRV passes with the adapter descriptor loop printing
  `kubernetes` and `sql` only; website typecheck and build pass.
- `shipped.sh` from the 2026-09-13 evidence, re-run on the post-removal
  `connectors-catalog-provider` against the sandbox: every read 200, every merge
  and update attempt on the merged MR 10 refused before dispatch with no PUT.
  Recorded under `docs/evidence/gitlab-retirement-20260914/`.
- `grep -rIl 'connectors-gitlab\|adapters/gitlab'` outside `docs/evidence/`,
  `.engineering/`, `adapters/gitlab/upstream/`, the bundle index and CHANGELOG
  history finds nothing.
