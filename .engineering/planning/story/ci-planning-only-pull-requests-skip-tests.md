---
format: aep.planning-md/3
id: story:ci-planning-only-pull-requests-skip-tests
kind: story
status: draft
title: Planning-only pull requests pass their checks without building or testing
summary: 'PR #275 changed one planning file and ran every heavy job; one path list in .github/ci/changed-scope.sh skips them'
refs:
- provider: github
  reference: beyond10x/connectors#275
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

Pull request #275 changes one file,
`.engineering/planning/verification-report/connectors-v0741-release-scope.md`, and still runs
every heavy job on `pull_request`: `repository gate` (`rust-gate.yml`, about 48 minutes after a
35-minute runner queue), `ignored CLI journeys` (24 minutes), the three macOS jobs of
`macos-host.yml`, `host and CLI on windows-latest`, both `custody-platforms.yml` jobs and
`Build documentation` (13 minutes).

`~/beyond10x/AGENTS.md`, "Planning-only pull requests skip the tests" (Timo, 2026-11-21, "ROLL the
5", proposal 3): a pull request whose changed files are all planning or prose files passes its
required checks without building or testing; the required job runs, finds only such paths and exits
0; a change to a built docs site still runs the docs check and skips the code build and test jobs;
one path list per repository, in the workflow, with a test case for both sides of it.

## Wanted

One path list, `.github/ci/changed-scope.sh` (POSIX sh), two lanes:

- lane `code`, every job that builds or tests the workspace: skipped when every changed path is
  planning or prose, or a documentation-site file that no build or test reads;
- lane `docs`, `Build documentation`: skipped when every changed path is planning or prose.

Planning or prose: `.engineering/**`, and `*.md`/`*.mdx` outside `adapters/`, `apps/`,
`contracts/`, `crates/`, `deploy/`, `ess/`, `examples/`, `spec-kinds/` and `website/`, except the
files code reads: `.engineering/planning/decision-blocker/**`, the stories
`mcp-inbound-local-binding` and `mcp-outbound-connection-lifecycle`, `AGENTS.md`, `CHANGELOG.md`,
`README.md` and `docs/**`. Documentation-site files the code reads: `website/docs/**`,
`website/examples/**`, `website/data/**`, `website/static/examples/**`, `website/product.json`,
`website/publication.json`.

The changed files are `git diff --name-only HEAD^1 HEAD` on the pull request's merge commit
(checkout `fetch-depth: 2`). Every event other than `pull_request` runs every lane; so does a list
that is empty or cannot be read.

`repository gate` decides in its first step and runs no later step when lane `code` is skipped;
`ignored CLI journeys`, the macOS, Windows and custody jobs and `Build documentation` depend on a
`change scope` job and are skipped. `common / Security and privacy` (a reusable workflow of
`beyond10x/gates`, 37 s) and `planning validate` (24 s) build nothing and keep running.

## Acceptance

`sh .github/ci/changed-scope-test.sh`, run by `connectors-build gate`, passes: a planning-only list
(the #275 path among them) skips both lanes, a list with a code file or a documentation-site page
runs, a real merge commit of a planning-only branch writes `run=false` and of a code branch
`run=true`, a push and a dispatch write `run=true`, and no tracked path a Rust source names under
`.engineering/`, `website/` or as Markdown is skipped by lane `code`. A planning-only pull request
shows `repository gate` green without the build steps.
