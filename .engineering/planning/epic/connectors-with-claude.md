---
format: aep.planning-md/3
id: epic:connectors-with-claude
kind: epic
status: draft
title: Connectors with Claude
summary: Download Connectors, connect Slack and Jira, and use them from Claude through a local MCP server with a token-efficient agent projection
relations:
- serves: vision:independent-contract-adapters
- informed_by: epic:mcp-contracts
revision: 4
---
## Outcome

A person downloads Connectors, connects Slack and Jira, and works with them from Claude:
- Claude Code calls the `connectors` CLI through the Connectors plugin skill, which exists today;
  the tutorial shows the commands it runs, each held to a recorded run.
- Claude connects to Connectors as a local MCP server and sees a small, token-efficient set of
  tools chosen for agent work, while the full selection stays available to the CLI.
- Every write still passes the approval the record can name (O1).

## Operator request

Timo, 2026-10-22 (relayed by the orchestrator, verbatim excerpts):

- "make it a story around connectors and how to use it together with Claude in the most
  meaningful way."
- "this tutorial, it kind of must show how to download the thing ... we don't want to have a
  cargo something command." That holds across all documentation; a small Development section may
  show what only developers need.
- "one section for Slack, one section for Jira, and yeah, one for the connectors in general."
- "it can be run as a server in MCP, and then Claude just connects to the MCP, and by writing
  these projection files ... we basically have a subset of what the connectors usually can, and
  it's for agent tooling"
- "less options, ... a lot more token efficient, and just a handful selected action as of
  compared to having the full-fledged basically one-to-one raw API access"

Out of scope (his words, "I also don't want you to do that now"): a documentation skill or agent.

## What exists (origin/main be11ae1de)

| Concern | State | Source |
|---|---|---|
| Release download | Linux and macOS archives plus `SHA256SUMS` on every release; no installer script, no Homebrew | `README.md:333-383`, `.github/workflows/release-binaries.yml` |
| Plugin installer | `connectors:init` runs `b10x init connectors`, prebuilt archives by default since v0.48.0 | agentplugins `plugins/connectors/skills/init/SKILL.md:13-25` |
| Getting started | builds from source with `cargo build` first; the release binary is a side note | `website/docs/getting-started.md:17-45` |
| `cargo` on the site | 18 occurrences in 15 pages | `website/docs/**/*.md` |
| MCP server | contracts, model and document tests only; `"mcp": "deferred"`; the runtime is on the unmerged branch tracked by `story:mcp-stdio-runtime-branch` | `apps/connectors/spec/compatibility.json:7-12`, `adapters/mcp/contracts/server/v1alpha1/projection.md`, `website/docs/connectors/mcp.md:12-19` |
| Curated subsets | selection sets (`operations.json`, `connectors-catalog-operations/1`) over a whole pinned API; definitions files in connectors-definitions, one per selection | `docs/local-catalog-provider.md:5,158-170`, `adapters/catalog/spec/ess/domains/selection.yaml`, connectors-definitions `README.md:3-14` |
| Agent tool subset | not found: no tool profile, no short descriptions, no reduced input schema | survey 2026-10-22 |
| Slack, Jira | Jira derived from a declaration (15 operations); Slack two pinned instances, no declaration; both fixture-verified for writes | `README.md:291,295`, `website/docs/connectors/index.md:26,30` |

## Stories

- story:claude-getting-started-tutorial
- story:claude-connects-to-the-mcp-server
- story:agent-tool-projection

Order: story:claude-connects-to-the-mcp-server, then story:claude-getting-started-tutorial (its
"With Claude" section links the MCP page) and story:agent-tool-projection.

Added 2026-10-22 from the Loom Kubernetes agent design (questions Q2 and Q3, milestone M6):
- story:readme-names-mounted-custody: Q3. Container custody exists as `mounted_secret`
  (story:mounted-secret-custody, implemented); the README still says there is one fixed binding.
- story:mounted-custody-kind-secret-volume: Q3, the proof on a real Kubernetes Secret volume.
- story:standing-write-approval-for-unattended-agents: Q2, blocked by
  decision-blocker:standing-write-approval-mode.
- M6, the design's MCP read proxy, is served by story:claude-connects-to-the-mcp-server and
  story:agent-tool-projection. Reads only, as both stories state.

The epic's acceptance below covers the first three stories. The added ones carry their own.

## Acceptance

The epic is complete when, on one release cut through `AGENTS.md` § Cutting a release:
- each story's named scenarios pass in that release's gate run:
  - tutorial: `no_cargo_command_outside_development`, `tutorial_commands_match_recorded_runs`, the
    release journey `tutorial_from_download_to_first_reads` recorded as `test_result` evidence;
  - MCP: `claude_code_session_is_answered`, `mcp_is_supported`;
  - projection: `agent_projection_lists_exactly_its_tools`,
    `agent_projection_listing_is_at_most_a_quarter_of_the_full_selection`;
- the release page carries the archives with the service binaries the tutorial names, and
  `sha256sum -c SHA256SUMS` passes on them;
- `aep plan artifact show` reports all three stories `implemented`.

## Boundaries

- An agent projection is a selection: a suffixed connectors-definitions file over a catalog
  bundle, as `slack-user` is. No ESS domain is added.
  - A withheld parameter already leaves the declared input (`adapters/catalog/src/lib.rs:1313-1342`),
    and the definitions notation already carries `withhold` (`crates/connectors-catalog/src/definition.rs:622`).
  - Selection descriptions already replace the bundle text (`lib.rs:2711`).
  - The stdio server serves one instance per process (story:mcp-stdio-runtime-branch), so Claude
    registers one server per projection instance.
- Not in this epic:
  - **Trimming tool answers.** `adapters/mcp/contracts/server/v1alpha1/projection.md:141-146`
    requires the complete response, so this needs an MCP contract change.
  - **Write tools over MCP.** `projection.md:157` keeps them unavailable until a runtime
    mutation binding exists.
- Approval, connection and refusal semantics are those of the MCP server contracts
  (`adapters/mcp/contracts/server/v1alpha1/`); this epic changes none of them, except through
  story:standing-write-approval-for-unattended-agents if its decision picks a contract change.
- Windows binaries stay with `epic:local-cli-on-macos-and-windows`.
