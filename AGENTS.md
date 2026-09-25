# Repository instructions for agents

## Serves

This repository advances these objectives from `atlas/ROADMAP.md`:

- **O1 — governed reach.** Every third-party effect runs through a declared
  connection, an admitted operation and an approval the record can name; a call
  outside that is a refusal by name, not an untracked request.
- **O5 — the generic agent platform.** The local CLI, the adapters and the catalog
  provider are the integration surface a tenant configures, connects and invokes.

## Establish scope and authority

Follow the current user request. The reviewed v0.1.0 specification milestone covers
Kubernetes including discovery, GitLab and SQL; that history is not standing
authorization to implement every modeled capability. Keep work local until the
operator explicitly requests external publication or deployment.

Read [docs/design.md](docs/design.md) before proposing implementation. Start with
[README.md](README.md) for current runtime support and [VISION.md](VISION.md) for
direction, then read the affected contract and adapter sources. Design sections
include dated sketches and historical tool observations; check the owning contract,
current source and pinned tool before treating a sketch as an interface.

Use these ownership rules:

| Concern | Source and rule |
|---|---|
| Shared semantics | [contracts/](contracts/README.md) owns generic, versioned guarantees and conformance inputs. |
| Shared typed model | [ess/system.yaml](ess/system.yaml) and its domains model shared types, entities and relations. |
| Native behavior | [adapters/](adapters/README.md) owns provider contracts, authored ESS, upstream inputs, fixtures, design and implementation. |
| Adapter specification kind | [spec-kinds/adapter/](spec-kinds/adapter/) belongs to Connectors; do not add provider vocabulary to ESS itself. |
| Generated output | Change its owning inputs and regenerate; never repair generated files by hand. |
| Historical evidence | Preserve review records and dated observations; add a new result rather than rewriting history. |

If code, model and contract disagree, identify the discrepancy and resolve it under
the current task. Do not silently change semantics to match existing code or make
a tool accept a document. A reviewed specification, structural compilation,
executable example and runtime conformance establish different facts.

## Plan and implement

Use the `aep-plan:planning` skill and `aep plan artifact` commands for **every**
planning-store mutation, including prose, relations and evidence. Discover kinds,
legal lifecycle moves and existing owners through the CLI. Never edit
`.engineering/planning/` directly. Use one writer for the planning store.

The original handoff is `specification:contract-driven-connectors-design`.
`specification:core-model-closure-20260909` records the reviewed model baseline;
`specification:public-documentation-website` owns the documentation, website design
and local implementation. Select the artifact appropriate to the requested outcome.

Before decomposing work that introduces entities or unresolved relations, use
`ess-specify:specify` to model and validate them. Preserve unresolved semantics
explicitly; do not invent identities, ownership, cardinalities or lifecycle states
to satisfy a validator. Ordinary documentation metadata does not need a fictional
runtime entity.

Keep provider semantics out of shared contracts and shared ESS. Native models
compile independently. Keep generic client and host libraries independent of
concrete adapters; keep adapter libraries independent of sibling adapters.
Composition owns concrete pairings. The [adapter boundary gate](adapters/README.md#boundary-gate)
checks these boundaries; do not add exceptions merely to pass it.

Implement executable project tooling and example behavior in Rust. TypeScript and
React may implement the planned website's presentation and Docusaurus integration.
Do not add Python helpers. Do not import historical implementation constraints
unless the current design deliberately preserves them.

## Verify the affected surface

Use [the development guide](docs/development.md) for build and gate commands and
[generation setup](docs/development.md) for tool resolution.
The single ESS pin is [toolchain.json](crates/connectors-spec/toolchain.json).
Explicit ESS selections must match it; never silently use an ambient older binary.

For semantic or implementation changes, run the applicable ESS validation,
generation/drift and conformance checks, then the required repository gate. Use a
task-owned `TMPDIR` under `.local/tmp` and bounded Cargo jobs, and delete that
directory when the task reports — nothing under `.local/` is an artifact, so anything
in `.local/tmp` older than the running task is deletable without asking (measured
2026-09-15: 2.9G in `.local/tmp`, 3.7G in `.local`, against 1.5G of tracked tree).
Keep unrelated worktrees' build outputs separate. Do not refresh vendor inputs during normal
builds or edit generated output around a refusal.

For documentation-only changes, check links, factual support claims and AEP
consistency; do not rerun unrelated live-provider tests. Report actual commands,
results and material limitations. Relay AEP validation output as its skill requires.

The website follows [docs/website-design.md](docs/website-design.md):
explicitly selected public inputs, adapter-owned native content, and distinct
specification, example and runtime support claims. Use [website checks](website/README.md#build-and-verify)
for presentation, reference tooling or example changes. Browser examples are
bounded teaching implementations; they do not authorize production runtime work.

## Cutting a release

For the current provider delivery, cut a release at each verified, usable batch
handoff before moving to the next provider. The order is GitLab, Kubernetes,
PostgreSQL, MCP, then the remaining providers. The operator's instruction authorizes
the source commit, tag, push and release-page sequence below; it supersedes the
earlier local-only Connectors publication boundary for these releases.

"Cutting a release" means completing all of these steps:

1. Establish the exact released scope from runtime evidence. List unfinished
   workflows and missing sandbox evidence explicitly; a release does not by itself
   complete the provider batch or its parent plan. Choose the next version from
   existing tags and compatibility changes, update the owning version inputs and
   regenerate affected outputs and lockfiles.
2. Update [CHANGELOG.md](CHANGELOG.md) with that version, release date, user-visible
   changes, compatibility or migration requirements, and material limitations.
   Preserve earlier entries. Update the website's authored guides, CLI examples
   and support claims to match the released behavior; update selected public
   inputs and regenerate references where needed. Generated pages are not edited
   by hand. Follow [website ownership](website/README.md#edit-the-owning-source).
3. Run the required [repository gate](docs/development.md) and affected
   [website checks](website/README.md#build-and-verify), including Rust 1.88 and
   relevant runtime acceptance. Retain commands, results and artifact identities;
   reuse existing evidence only when its relevant inputs are unchanged.
4. Have the single planning-store writer record the release scope, evidence and
   current lifecycle state through AEP, reconciling earlier publication exclusions
   with this instruction. Do not close unfinished work to make a release look done.
5. Integrate the verified changes and release metadata on `main` of
   `beyond10x/connectors` — the `origin` remote and the repository's default
   branch — the way every change reaches it: a branch pushed through
   `b10x-gates bot`, a pull request opened and merged through `b10x-gates api`,
   and the shared source check green on it. The release commit is the merge
   commit on `origin/main`; `git fetch` and confirm it before tagging.
6. Create an annotated `v<version>` tag on that merge commit through
   `b10x-gates bot` and push the tag. Verify the author and committer of the
   merged commits and the tagger (`b10x-bot[bot]` for each), that the peeled tag
   is an ancestor of `origin/main`, and that its tree is the tree the gate ran on.
   Existing release tags are immutable. Report the version, commit, destination
   and verification result, and clean up task-owned managed worktrees through
   `worktree`.
7. Publish the hosted release page for that exact tag through
   `b10x-gates gh -- release create v<version> --verify-tag --notes-file <file>`,
   so the page is authored by `b10x-bot[bot]` like every commit and tag before it.
   A page created with a personal `gh` login is the same defect as a commit
   authored by a person, and is corrected the same way: delete it and recreate it
   under the bot, never leave it. Take the notes from the annotated tag with
   `git tag -l --format='%(contents)'`; `--notes-from-tag` is refused alongside
   `--repo`. Write no new commit, move no tag and edit no existing release. A
   release cut from `main`, the default branch, is Latest and needs no flag; pass
   `--latest=false` only when the release is deliberately not the newest. A page
   that is not Latest is invisible on the repository's front page, which is how
   v0.8.0 and v0.9.0 went unnoticed after they shipped. Verify the created page's
   author, its Latest state and that it resolves to the tag's peeled commit. No
   workflow builds release artifacts; the page carries the source archives GitHub
   generates and nothing else.

v0.8.0–v0.11.0 were cut from a checkout with no GitHub remote and published by
replaying its history onto the `next` branch with `git commit-tree`, so each of
those tags peels to a `next` commit whose tree equals its counterpart on `main`
(v0.11.0: `5330fb94c` on `next`, `bc0bcb7a6` on `main`). Since #29 the
repository is worked through pull requests to `main`; `next` stopped at the
v0.11.0 plan close (`dec210fc8`) and is not a release destination.

A local commit or local tag alone is not a cut release, and neither is a pushed
tag with no release page. Source release does not implicitly include a
binary/package or container-registry publication, website/cloud deployment, or
Atlas registration; those need their own requested scope. If a release step is blocked, record the exact blocker and
report the release as incomplete.

## Workspace and integrations

For single-agent work, the operator's repository-specific rule is to work directly
in the checkout. Use `workspace-hygiene:worktree` and the `worktree` CLI when multiple
agents need isolated changes. For an existing managed tree, acquire and maintain
your own lease, preserve its lifecycle and never remove it manually. Publish wanted
commits to an authorized recovery destination before `worktree finish`. Review
`worktree gc --dry-run`, then apply cleanup only to exact reviewed ids. Use
`worktree reconcile` for interrupted or missing records; external retirement needs
explicit `--allow-external-retirement`. Never force cleanup or clear another lease.

Use `connectors` for engineering integrations. Never invoke `fluxplane-plugin`,
even if a skill recommends it. Report a missing Connectors capability before using
an alternative integration client.

Gating is Gates, not Atlas. Install coordinated local hooks with
`b10x-gates --repository beyond10x/connectors_v2 install`. Use `b10x-gates bot` for
direct commits, tags and pushes, retaining `b10x-bot[bot]`, and Gates `check`,
`verify` and `publish` for signed common evidence; verify both author and committer.
The hooks scan the index, names, messages, metadata, tags and every outgoing commit.
Ordinary commit and publish paths need no Atlas checkout, current Atlas main or
organization-wide admission. Private policies and enrolled signing keys stay outside
this repository, candidate suppression files carry no authority, and hooks are never
bypassed. A receipt is invalidated by source changes, rebases, policy changes and
scanner upgrades.

Atlas grounds this repository through the fence, not through a catalog row (measured
2026-09-15). `atlas/scripts/fences.sh:25-30` builds the expected-repository roster from
every sibling directory holding a `.git`, so this repository is enumerated, and
`atlas/scripts/check-map.sh:378-389` reads the `## Serves` ids out of this file — keep
that section accurate and keep its ids in `atlas/ROADMAP.md`. There is no
`connectors_v2` row in the Atlas catalog store or in `atlas/docs/catalog.md`, and
Atlas ADR 0051 says why there will not be a separate one: a catalog id **is**
the exact GitHub repository name, `beyond10x/connectors_v2` does not exist (404 on
2026-09-15), and under that ADR this lineage's published identity — and so its
catalog row — is `beyond10x/connectors`, the row that already exists. The lineage
record itself is deferred there for the same reason.

The lineage is Atlas ADR 0051 (*`beyond10x/connectors` is the v2 lineage; v1 is its
predecessor*, accepted 2026-09-15): the name `beyond10x/connectors` denotes this
lineage, which owns that repository's `main` default branch, its `v0.8.0`-and-later
tag namespace and its Latest release. The v1 component at
`/home/timo/beyond10x/connectors`, whose releases end at `v0.7.2`, is the
predecessor, not a parallel current component.

What stays deferred is narrower than before: do not alter Atlas roadmap or catalog
records from here, and do not wire documentation delivery or consumer dependencies.
Consumers are still on the predecessor — `devcenter/Cargo.toml:37-38` pins
`beyond10x/connectors` at rev `e80b7ae1` (`=0.7.0`) — and moving one is its own
requested scope. A local recovery remote is not public distribution.

<!-- b10x-docs-operations:start -->
## Public documentation operations

This repository owns the public source and presentation allowlist in `b10x.docs.yaml`. The generated credential-free `.github/workflows/b10x-docs-bundle.yml` passively packages only those declared files for the exact successful `main` commit; it must never run repository code. The generated `.github/workflows/b10x-docs-check.yml` runs the publisher's per-source checks on every pull request and main push, with read-only contents and no credentials; it is deliberately separate from the shared gate, which runs on `pull_request_target` with a secret and never reads candidate source. Atlas selects the latest successful bundle with every other catalog source, and Website plus Docs System own rendering, shared components, search, and feeds. Do not add a standalone docs deployer or put App credentials in this public repository. If Atlas catalogs a former Pages workflow, that file remains repository-owned validation: preserve its bespoke checks while keeping exact read-only permissions, an unconditional pull-request trigger, and no deployment primitives. Project Pages at `/connectors/` is only the generated stable redirect façade in `.github/workflows/b10x-docs-pages.yml`; content-only publication never rebuilds it.

From the complete organization workspace, verify the contract with a clean Atlas checkout at the current remote `main`. Set `B10X_ATLAS_CHECKOUT` to a managed Atlas worktree when the primary checkout is dirty or stale; never infer command availability from the primary alone.

```bash
atlas_checkout="${B10X_ATLAS_CHECKOUT:-atlas}"
atlas_head="$(git -C "$atlas_checkout" rev-parse HEAD)"
atlas_main="$(git -C "$atlas_checkout" ls-remote origin refs/heads/main | awk '{print $1}')"
test -z "$(git -C "$atlas_checkout" status --porcelain)"
test "$atlas_head" = "$atlas_main"
cargo run --manifest-path "$atlas_checkout/Cargo.toml" --locked -q -- \
  --store "$atlas_checkout/catalog/store" docs reconcile --workspace . --check
```

Keep internal plans, stories, ADRs, decisions, worklogs, security material, and research out of the public allowlist unless a repository authority explicitly declares them public.
<!-- b10x-docs-operations:end -->
