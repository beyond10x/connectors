# Repository instructions for agents

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
[generation setup](docs/gitlab-generation.md#generate-and-check) for tool resolution.
The single ESS pin is [toolchain.json](crates/connectors-spec/toolchain.json).
Explicit ESS selections must match it; never silently use an ambient older binary.

For semantic or implementation changes, run the applicable ESS validation,
generation/drift and conformance checks, then the required repository gate. Use a
task-owned `TMPDIR` under `.local/tmp` and bounded Cargo jobs. Keep unrelated
worktrees' build outputs separate. Do not refresh vendor inputs during normal
builds or edit generated output around a refusal.

For documentation-only changes, check links, factual support claims and AEP
consistency; do not rerun unrelated live-provider tests. Report actual commands,
results and material limitations. Relay AEP validation output as its skill requires.

The website follows [docs/website-design.md](docs/website-design.md):
explicitly selected public inputs, adapter-owned native content, and distinct
specification, example and runtime support claims. Use [website checks](website/README.md#build-and-verify)
for presentation, reference tooling or example changes. Browser examples are
bounded teaching implementations; they do not authorize production runtime work.

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

Organization authority comes from a clean Atlas checkout verified against remote
main. Direct commits and pushes use its private `scripts/as-bot.sh` wrapper as
`b10x-bot[bot]`; verify both author and committer. Keep credential machinery outside
this repository and do not bypass hooks.

Atlas integration remains deferred: do not register this repository, alter Atlas
roadmap/catalog records, or wire documentation delivery or consumer dependencies.
The design serves existing Connectors objectives O1 (governed integration access)
and O5 (generic platform integration capabilities); it does not replace the
registered component. A local recovery remote is not public distribution.
