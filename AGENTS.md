# Connectors v2 local design workspace

Read [docs/design.md](docs/design.md) before proposing implementation. It preserves the operator's direction, existing-system evidence, proposed boundaries, unsettled decisions, and extraction order. This is a local exploratory repository; no remote or implementation rollout is configured. The operator explicitly deferred Atlas integration: do not register this repository, alter Atlas roadmap/catalog records, or wire documentation/consumer delivery as part of this handoff.

## Serves

The proposed design serves the existing Connectors objectives O1 (governed integration access) and O5 (generic platform integration capabilities). This local repository does not replace the registered Connectors component or change its consumers.

## Worktree and planning

Use the `workspace-hygiene:worktree` skill and managed `worktree` CLI for repository changes. Reuse an existing managed task tree, acquire and maintain your own session lease, and keep the primary checkout clean. Keep local-only work in an explicit handoff; publish wanted commits before `worktree finish`. Review exact IDs with `worktree gc --dry-run` before any cleanup. Never manually remove a managed tree.

Use the `aep-plan:planning` skill and `aep plan artifact` commands for all planning-store mutations. The design handoff is `specification:contract-driven-connectors-design`; implementation stages remain proposals. Model new entities and unresolved relations through the applicable ESS workflow before decomposing implementation work. The adapter specification kind belongs to Connectors.

## Workspace operations

Use `connectors` for engineering integrations; never invoke `fluxplane-plugin`. Report a capability gap before using an alternative client.

Organization authority comes from a clean Atlas checkout verified against remote main. Direct commits and pushes use Atlas's private `scripts/as-bot.sh` wrapper as `b10x-bot[bot]`, with both author and committer verified. Keep credential machinery outside this repository and do not bypass hooks. The current task is local only.

Implement executable project tooling in Rust. Do not import the old repository's implementation constraints merely because they exist there; the design distinguishes preserved behavior from historical structure.
