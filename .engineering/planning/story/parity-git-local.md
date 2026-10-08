---
format: aep.planning-md/3
id: story:parity-git-local
kind: story
status: draft
title: Local git status, diff and writes
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Local git repository status, diff and writes, so Connectors covers the fluxplane `git` plugin (`docs/fluxplane-plugin-parity.md`, section `git`).

## Operations

`git.status`, `git.diff`, `git.add`, `git.commit`, `git.tag`, `git.push`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call git …` sites in Bash tool calls, each tool call
counted once).

## Surface

A new native adapter over a local repository. Writes (add, commit, tag, push) need an approval and an identity decision (which author and committer a write records).

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.
