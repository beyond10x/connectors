---
format: aep.planning-md/3
id: story:claude-mcp-add-registration-is-recorded
kind: story
status: implemented
title: A recorded claude mcp add registers the local stdio MCP server
relations:
- derived_from: story:claude-connects-to-the-mcp-server
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: README.md
- confidence: inferred
  path: apps/connectors/tests/claude_mcp_journey.rs
- confidence: inferred
  path: apps/connectors/tests/fixtures/claude-code/mcp-add.json
- confidence: inferred
  path: website/docs/connectors/mcp.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T17:29:39Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-05T17:29:39Z", actor: "human:operator", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-06T05:34:10Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
<!-- Starting point for a `story` artifact, seeded by `aep artifact new story <name>`.
     No frontmatter here on purpose: the `---` block is written by the CLI from the id, kind, status
     and relations you gave it, and a second copy in this file would be the one that went stale.
     Delete the italic guidance as you fill each section. -->

# Story: <name>

## Outcome

*What is true for whom once this has shipped, in one sentence. If it names a component rather than a
person, it is a task — say what changes for someone.*

## Context

*Why this is worth doing now, and what it depends on. Link the epic or specification it comes from
rather than restating it; the `derived_from` relation already carries the edge.*

## Acceptance

*The conditions under which this is done, each one something a person or a check can observe. "Works
correctly" is not one of them.*

## Out of Scope

*What a reasonable reader would expect to be included and is not — the boundary that stops this
story quietly becoming an epic.*

## Ambiguities

*Each gap this story found and did not close, classified. `inferable` — the answer is already
written down, so give the `path:line` or the artifact id that settles it.
`requires-stakeholder-input` — nobody here can decide it, so name who does, and raise that entry as
a `decision-blocker` with a `blocks` edge to this story, or it is a sentence somebody improvises
later.*

## Open Questions

*Anything still undecided belongs in `## Ambiguities` above, classified and with its citation or its
decider. Keep this section for a question that is neither — an unowned question is a story that
stalls without anybody noticing.*

## Why

The MCP page shows how Claude Code reaches the local stdio MCP server through the `--mcp-config` entry and the one recorded `claude -p` run. `claude mcp add` is the usual way a Claude Code user registers a server. No run has exercised it, so no page shows it as a tested command.

## Outcome

One recorded `claude mcp add --scope project connectors -- connectors --config CONFIG --state-dir STATE_DIR server --transport stdio` is run in an empty working directory. The `.mcp.json` it writes is captured, sanitized and committed as a fixture. The MCP page shows the command with its output.

## Acceptance

- A test checks that the server entry in the captured `.mcp.json` equals the server launch the journey replay uses: command and args.
- The run is made by the coordinator only, with the orchestrator's go, once, on the existing login. Nothing in the user's own Claude configuration is changed.
- The fixture passes `b10x-gates scan-text` and the grep for tokens, emails, hostnames and home paths with 0 hits before it is committed.

## Delivered (wave 2026-11-05b, v0.68.0)

The unit commit cc3260477 was merged in 798b960b7 on `wave/2026-11-05b-connectors`. The local staged gate is green at cd51cd062.

The coordinator made the one recorded `claude mcp add --scope project` run with Claude Code 2.1.318. It ran in a rig with an empty HOME, no network and the filesystem read-only outside the run directory. Nothing asked for a login, and nothing was written outside the rig. The sanitized recording passed scan-text with 0 hits and was copied byte for byte to `apps/connectors/tests/fixtures/claude-code/mcp-add.json`.

`claude_mcp_journey.rs` checks three things: the recorded `.mcp.json` entry against the replayed launch, the same entry against the documented user form, and the page's command and output against the recording. The MCP page and README show the command, its output and the `.mcp.json` it wrote. They say that the recording shows the registration only.

Targeted tests: `claude_mcp_journey` 11 passed, `connectors-docs` 52 passed, and `generate --check` is current.

Scope as landed:

| path | result |
|---|---|
| `apps/connectors/tests/claude_mcp_journey.rs` | held |
| `apps/connectors/tests/fixtures/claude-code/mcp-add.json` (new) | held |
| `website/docs/connectors/mcp.md` | held |
| `README.md` | held |
