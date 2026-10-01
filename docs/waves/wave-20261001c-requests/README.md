# Wave 2026-10-01 (c): a consumer's deployment read, the gate's compiler wrapper, revision-conflict code

Skill version: aep implementing 0.18.0. Status: approved by the operator 2026-10-01 ("dispatch the next wave,
handle requests from other sessions first").

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:catalog-gitlab-deployment-reads | vision:independent-contract-adapters | cited (one inferred test file) | aep:implementor, aep:adversary | impl/catalog-gitlab-deployment-reads | `<managed-trees>`/wave1001c-deploy | `<tree>`/target | `<wave-scratch>`/deploy | dispatched |
| story:gate-temporary-root-and-compiler-wrapper | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/gate-temporary-root-and-compiler-wrapper | `<managed-trees>`/wave1001c-gate | `<tree>`/target | `<wave-scratch>`/gate | dispatched |
| story:revision-conflict-wire-code | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary | impl/revision-conflict-wire-code | `<managed-trees>`/wave1001c-revision | `<tree>`/target | `<wave-scratch>`/revision | dispatched |

Integration branch: `wave/20261001c-requests` off `main` ac803ab1c.

## Selection

`aep plan artifact waves --kind story --status draft --format json` (aep 0.68.0) on `main` ac803ab1c places the
three in wave 1 with no collision. No pull request was open. Requests from other sessions first: the deployment
read was asked for by a consumer session on 2026-10-01.

| story | verb wave | left out because |
|---|---|---|
| setup-initialises-missing-state | 2 | collides with revision-conflict-wire-code (CLI dispatch, `ess/domains/cli.yaml`) |
| catalog-selection-excludes-parameters | 3 | collides with the deployment read on GitLab `operations.json`, the guide and the digest pin |
| spec-models-operation-admission | 3 | generated CLI contract and ER definitions |
| catalog-validates-enum-and-date-time | 4 | `adapters/catalog/src/lib.rs`, all bundles |
| configuration-change-error, expired-evidence-invoke-advises-revalidate | 5, 6 | `apps/connectors/src/local.rs` |
| service-failure-carries-upstream-reason | 7 | needs a design decision: `contracts/cli/v1alpha1/semantics.md:516-519` says a failure never carries raw provider evidence, and adding a reason changes the private protocol's closed `failed` frame |

## Pre-flight

- `/` 89G free. Units gate package-scoped; the closing gate runs from a managed worktree with `RUSTC_WRAPPER=`
  empty (or with the gate unit's fix).

## Commits approval authorises

One commit per unit, the merges into `wave/20261001c-requests`, the opening and closing store commits, the push and
pull request through `b10x-gates`, and the merge into `main` once the gate is green. No tag and no release.
