---
format: aep.planning-md/3
id: approval-record:connectors-claude-recording-on-the-existing-login
kind: approval-record
status: approved
title: One recorded Claude Code run on the existing login, limited by --tools, scanned before commit
relations:
- decides: story:claude-connects-to-the-mcp-server
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T09:33:20Z", actor: "human:timo", revision: 2}
---
## The operator's answer on the recorded Claude Code run (2026-11-05)

The directing session `b10x-chef-orchestrator` put one question to the operator: how the one recorded
Claude Code run that story:claude-connects-to-the-mcp-server needs may use a Claude Code login. The
queue row it showed read:

> A: you allow one run on the existing login, limited by `--tools`, transcript scanned before commit

The operator answered, on 2026-11-05: "yes + yes". The directing session relayed the answer verbatim
with its date, and read it as option A.

What it allows:

- Exactly one recorded headless `claude -p` run against `connectors server --transport stdio`, on the
  existing Claude Code login. No new login, and no copy of credentials into another `HOME`; if the
  recording needs the login copied into an isolated `HOME`, the run stops and the directing session is
  told.
- The client's built-in tools are limited with `--tools` to none, so only the connectors MCP tools the
  story needs are callable, and the run has a hard turn and time cap.
- The Connectors configuration and state of the run hold no live credentials (AGENTS.md, real-client
  recordings).
- Before anything is committed, the recording is scanned with `b10x-gates scan-text` and a grep for
  tokens, emails, hostnames and home paths; anything that fails is redacted or dropped.
- The recording and the story's move ride a small connectors wave, gated, merged and released on
  green.
