# Repository instructions for agents

## Serves

This repository advances these objectives from `atlas/ROADMAP.md`:

- **O1 — governed reach.** Every third-party effect runs through a declared
  connection, an admitted operation and an approval the record can name; a call
  outside that is a refusal by name, not an untracked request.
- **O5 — the generic agent platform.** The local CLI, the connectors and the catalog
  provider are the integration surface a tenant configures, connects and invokes.

## Establish scope and authority

Follow the current user request. The reviewed v0.1.0 specification milestone covers
Kubernetes including discovery, GitLab and SQL; that history is not standing
authorization to implement every modeled capability. Keep work local until the
operator explicitly requests external publication or deployment.

Read [docs/design.md](docs/design.md) before proposing implementation. Start with
[README.md](README.md) for current runtime support and [VISION.md](VISION.md) for
direction, then read the affected contract and implementation sources (`adapters/`). Design sections
include dated sketches and historical tool observations; check the owning contract,
current source and pinned tool before treating a sketch as an interface.

Use these ownership rules:

| Concern | Source and rule |
|---|---|
| Shared semantics | [contracts/](contracts/README.md) owns generic, versioned guarantees and conformance inputs. |
| Shared typed model | [ess/system.yaml](ess/system.yaml) and its domains model shared types, entities and relations. |
| Native behavior | [adapters/](adapters/README.md) owns provider contracts, authored ESS, upstream inputs, fixtures, design and implementation. |
| Implementation specification kind | [spec-kinds/adapter/](spec-kinds/adapter/), the internal adapter kind, belongs to Connectors; do not add provider vocabulary to ESS itself. |
| Generated output | Change its owning inputs and regenerate; never repair generated files by hand. |
| Historical evidence | Preserve review records and dated observations; add a new result rather than rewriting history. |

If code, model and contract disagree, identify the discrepancy and resolve it under
the current task. Do not silently change semantics to match existing code or make
a tool accept a document. A reviewed specification, structural compilation,
executable example and runtime conformance establish different facts.

## Vocabulary

The words a user meets (provider, connector, auth profile, instance, connection,
operation) are defined once, in [README.md § Vocabulary](README.md#vocabulary).
Link there; do not restate the definitions in another file.

"Adapter" is the internal name of an implementation (the catalog engine, a
credential binary, the Kubernetes or SQL service) and of the process that serves
an instance. It is never the name of the configured thing. In text a user reads
(the README, the site's hand-written pages, the guides under `docs/`, CLI help and
messages) the configured thing is an instance of a connector. The deprecated
`--adapter` option, the deprecated `adapters` group and `connections repair`, and the
legacy `[adapters.<alias>]` table appear only where the text says so.

`connectors-docs vocabulary` checks a set of files. It fails on a line that names
the internal term without one of `adapters/`, `adapter_id`, `` `adapter` ``,
`deprecated`, `legacy` or `internal`; on a term's table row outside the README; and
on a README without the `## Vocabulary` table or an AGENTS.md without the link.
Given the README, it also fails when a connector of the README's support table (the
table headed `| Connector |`) has no onboarding page under `website/docs/connectors/`
or does not link it on the site, when a page there lacks the eight onboarding sections
in order, and when a page has no row unless it is marked `:::caution[Planned]` (MCP)
(`crates/connectors-docs/src/onboarding.rs`). `--no-pins` also refuses
`configuration_revision`, `sha256` and `bundle_directory`, and `--one-instance` a second
`[connectors.<c>.instances.<i>]` table:

```sh
cargo run --locked -p connectors-docs -- vocabulary README.md AGENTS.md \
  website/product.json website/data/terminal.json website/docs/index.md \
  website/docs/getting-started.md website/docs/guides/*.md \
  website/docs/concepts/catalog-provider.md website/docs/concepts/contracts.md \
  website/docs/concepts/governed-invocation.md website/docs/concepts/local-runtime.md \
  website/docs/connectors/*.md
```

`crates/connectors-docs/src/vocabulary.rs::the_readme_agents_and_site_speak_the_vocabulary`
holds that set in `cargo test -p connectors-docs`, every concept page but the one on
internal implementations included. Those pages, `website/docs/concepts/adapters.md`
and `website/docs/reference/adapters/*/index.md`, are not in it. CLI help and messages
are held to the same deprecated-only rule by
`apps/connectors/tests/vocabulary_text.rs::help_says_adapter_only_on_a_deprecated_line` (deprecated
names only).

`crates/connectors-docs/src/onboarding.rs::every_support_table_connector_has_an_onboarding_page`
holds the onboarding rule. A connector added to the README's support table, or a catalog
connector declared under `adapters/catalog/providers/`, needs its page under
`website/docs/connectors/` with front matter `connectors` naming it, and its row links that
page on the site; `generate` writes the index from the declarations and the hand-written
rows in `onboarding.rs`.

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
concrete implementations (the internal adapter libraries), and keep each of those
libraries independent of its siblings.
Composition owns concrete pairings. The [adapter boundary gate](adapters/README.md#boundary-gate)
checks these boundaries; do not add exceptions merely to pass it.

One connector per service: its read and write operations live in the same
definition. Never split a service into a per-direction provider, connector,
definition file, bundle or read and write instance pair. A write added later joins
the existing definition. The decision is
[ADR connector-vocabulary](.engineering/planning/architecture-decision-record/connector-vocabulary.md)
(line 56, "One instance per account replaces the read and write instance pair"); the
operator restated it on 2026-10-23: "we dont split providers into $P-(read|write)".
A story or epic id may end in `-reads` or `-writes`, because it names work; a
provider, connector, file or bundle name never does.

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

On the operator's shared build host, this repository's own Cargo builds use a compile cache of
their own, so a new worktree does not recompile every dependency: `RUSTC_WRAPPER=sccache`,
`SCCACHE_DIR=~/.cache/sccache/connectors`, `SCCACHE_SERVER_PORT=4301` and
`SCCACHE_CACHE_SIZE=2G`. The server on port 4301 serves this repository only; never the default
port 4226, never a wrapper set globally. Set them in the build's environment, not in a committed
Cargo configuration, and keep the caller's `TMPDIR` short (the server binds its startup socket
there).

For documentation-only changes, check links, factual support claims and AEP
consistency; do not rerun unrelated live-provider tests. Report actual commands,
results and material limitations. Relay AEP validation output as its skill requires.

The website follows [docs/website-design.md](docs/website-design.md):
explicitly selected public inputs, native content owned under `adapters/`, and distinct
specification, example and runtime support claims. Use [website checks](website/README.md#build-and-verify)
for presentation, reference tooling or example changes. Browser examples are
bounded teaching implementations; they do not authorize production runtime work.
`crates/connectors-docs` generates the CLI and crate reference, the contract and model
reference and the status page with its data; never edit them. A change to the CLI, a crate
description, a selected contract or model, or a shipped capability regenerates them with
`cargo run --locked -p connectors-docs -- generate`, and the gate fails on drift. A shipped
status item names the test that holds it (`crates/connectors-docs/src/status.rs`). Every
command a page shows is run in the tree and its output pasted from that run.

A fixture recorded from a real MCP client (for example a headless `claude -p` session) runs
with the client's tool set limited to the MCP server's tools (`--tools`, not only
`--allowedTools`), and with an isolated `HOME` and Connectors configuration that hold no live
credentials, so the client cannot reach a live provider through any other tool. On 2026-10-23
a client limited only by `--allowedTools` fell back to the shell after a refused call and read a
live provider through the operator's own configuration. Before a recording enters the tree, check
it for local paths, account emails and session or organization ids, and run `b10x-gates scan-text`
on it.

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
   `b10x-gates gh --policy <policy> --repository beyond10x/connectors -- release create v<version> --verify-tag --notes-file <file>`,
   so the page is authored by `b10x-bot[bot]` like every commit and tag before it.
   `--policy` names the private Gates policy file and `--repository` the GitHub
   repository; without them the command is refused with `protected file
   unavailable` wherever the default policy path does not exist.
   A page created with a personal `gh` login is the same defect as a commit
   authored by a person, and is corrected the same way: delete it and recreate it
   under the bot, never leave it. Take the notes from the annotated tag with
   `git tag -l --format='%(contents)'`; `--notes-from-tag` is refused alongside
   `--repo`. Write no new commit, move no tag and edit no existing release. A
   release cut from `main`, the default branch, is Latest and needs no flag; pass
   `--latest=false` only when the release is deliberately not the newest. A page
   that is not Latest is invisible on the repository's front page, which is how
   v0.8.0 and v0.9.0 went unnoticed after they shipped. Verify the created page's
   author, its Latest state and that it resolves to the tag's peeled commit. Besides
   the source archives GitHub generates, the page carries the four archives
   `connectors-<version>-<target>.tar.gz` (`x86_64-unknown-linux-gnu`,
   `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`) and
   `SHA256SUMS`, which [`release-binaries.yml`](.github/workflows/release-binaries.yml)
   builds on the tag push and uploads once the page exists; it never creates or
   edits the page. The release is verified only when those five assets are on the
   page and `sha256sum -c SHA256SUMS` passes on the downloaded archives.

v0.8.0–v0.11.0 were cut from a checkout with no GitHub remote and published by
replaying its history onto the `next` branch with `git commit-tree`, so each of
those tags peels to a `next` commit whose tree equals its counterpart on `main`
(v0.11.0: `5330fb94c` on `next`, `bc0bcb7a6` on `main`). Since #29 the
repository is worked through pull requests to `main`; `next` stopped at the
v0.11.0 plan close (`dec210fc8`) and is not a release destination.

A local commit or local tag alone is not a cut release, and neither is a pushed
tag with no release page. A release includes the Linux and macOS archives of step 7; it does not include a
package-registry or container-registry publication, website/cloud deployment, or
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
`~/beyond10x/connectors`, whose releases end at `v0.7.2`, is the
predecessor, not a parallel current component.

What stays deferred is narrower than before: do not alter Atlas roadmap or catalog
records from here, and do not wire documentation delivery or consumer dependencies.
Consumers are still on the predecessor — `devcenter/Cargo.toml:37-38` pins
`beyond10x/connectors` at rev `e80b7ae1` (`=0.7.0`) — and moving one is its own
requested scope. A local recovery remote is not public distribution.

## Documentation

The site (`website/`, Docusaurus on `@beyond10x/docs-system`) is independent of the unified
site. `pages.yml` (`Documentation validation`) builds it on every pull request and push, and on a
bot push to `main` `b10x-docs-site.yml` (`Documentation site`) deploys that build to
<https://beyond10x.github.io/connectors/> through Website's `project-site.yml`, pinned to an
exact Website commit. Change the site's content through its owning sources
([website ownership](website/README.md#edit-the-owning-source)); generated pages are regenerated,
never edited by hand. Do not add a second deployer, and keep App credentials out of this public
repository.

Keep internal plans, stories, ADRs, decisions, worklogs, security material, and research out of
the site unless a repository authority explicitly declares them public.
