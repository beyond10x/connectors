---
title: MCP
sidebar_position: 21
description: Claude Code and other local MCP clients reach connectors through its stdio MCP server on Linux; MCP as a connector has specified contracts and no runtime yet.
lede: connectors runs as a local MCP server, recorded with a headless Claude Code session. Reading from an MCP server as a connector is planned; each step below says which direction exists.
source: docs/local-mcp-cli.md, docs/local-mcp-stdio.md, apps/connectors/tests/claude_mcp_journey.rs, apps/connectors/tests/fixtures/claude-code/mcp-add.json, adapters/mcp/contracts/server/v1alpha1/semantics.md, adapters/mcp/contracts/client/v1alpha1/auth.md
connectors: "mcp"
---

# MCP

:::caution[Planned]

MCP as a connector, reading from an MCP server, does not run. The CLI has no MCP instance
configuration, no MCP auth profile and no MCP operation binding. The other direction runs:
connectors is a local MCP server for Claude Code and other MCP clients, described under
[A first read](#a-first-read). This page keeps the onboarding steps in the same order as every
other connector's so that each says what exists and what does not.

:::

The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[MCP guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mcp-cli.md) records the
selected intent in full.

## What it reads and writes

As a connector, nothing yet. Once a binding exists, an MCP server's tools, resources and prompts
are to be projected as operations; mutation advertisement stays withheld until the mutation and
replay binding, with its approval source and business key, exists.

As a local MCP server, connectors offers the admitted reads of one configured instance as
read-only tools. In the recorded session below a Jira instance listed its
eleven reads: `attachment.content`, `issue.changelog`, `issue.comments`, `issue.create_meta`,
`issue.create_meta.fields`, `issue.get`, `issue.transitions`, `issues.search`, `project.get`,
`projects.list` and `users.search`.

## Prerequisites at the provider

As a connector, none can be stated until a binding exists. The two questions that withheld it
are decided: an inbound MCP caller is the configured owner, one principal, so no
caller-to-connection assignment is modelled; and Connectors owns one pinned, supervised outbound
stdio server process per session. Neither decision is implemented.

As a local MCP server, the prerequisites are those of the instance it serves, such as the
[Jira](./jira.md) account and API token.

## Enable the connector and add an instance

Not possible yet. An MCP instance would take the pinned form `[connectors.mcp.instances.<instance>]`
of the other connectors served by their own executables, but none can be configured; do not add
one to a working configuration.

## Connect

Not available. The generic `connections connect` grammar exists, but no MCP auth profile, browser
OAuth or refresh for an MCP server is implemented.

## A first read

As a connector, not available: reading from an MCP server is not implemented.

As a local MCP server, on Linux, connectors offers admitted read-only operations of a configured
instance as tools, with no resources, prompts, writes or outbound MCP. The macOS and Windows builds
compile and run their lease-clock and launch tests in CI, with a serving scenario against each
platform's own credential store; that scenario has passed on Linux only so far, so serving on macOS
and Windows is not yet shown. The instance needs a connection, and beside the configuration the
tools it exposes (`CONFIG.mcp-stdio.json`) and the declarations of their operations
(`CONFIG.operation-curation.json`), as the
[stdio guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mcp-stdio.md) shows.

A headless Claude Code session was run once, by Claude Code 2.1.318, against a Jira instance
whose site was a local fixture. The `--mcp-config` entry that ran named a capturing wrapper as
its command. The wrapper tees the client's bytes and the server's answers to capture files, then
runs `connectors` with the same arguments inside a bubblewrap sandbox, where `/srv/connectors`
holds the configuration and state. What ran, with `CAPTURE_WRAPPER` and `CAPTURE_DIR` for the
wrapper's path and the capture directory:

```json
{
  "mcpServers": {
    "connectors": {
      "type": "stdio",
      "command": "CAPTURE_WRAPPER",
      "args": ["--config", "/srv/connectors/config.toml", "--state-dir", "/srv/connectors/state", "server", "--transport", "stdio"],
      "env": {"RIG_CAPTURE": "CAPTURE_DIR"}
    }
  }
}
```

The user form, the entry a user writes, names `connectors` itself; `CONFIG` and `STATE_DIR`
stand for the absolute paths of the configuration and the state directory:

```json
{
  "mcpServers": {
    "connectors": {
      "type": "stdio",
      "command": "connectors",
      "args": ["--config", "CONFIG", "--state-dir", "STATE_DIR", "server", "--transport", "stdio"]
    }
  }
}
```

The command that ran, with `FILE` for the path of the file that held the entry that ran:

```sh
claude -p "List my open Jira issues. Use the connectors tool to search Jira, then reply with each issue's key, summary and status." --mcp-config FILE --strict-mcp-config --tools "" --allowedTools "mcp__connectors__c1_6a6972612e776f726b_636174616c6f67_6973737565732e736561726368" --setting-sources project --no-session-persistence --max-budget-usd 1 --output-format stream-json --verbose
```

Claude Code started the wrapper as its child and spoke MCP with the server through it, over
standard input and output. Its answer listed DEMO-1, DEMO-2 and DEMO-3, each with its key, summary and status.
`claude mcp add` was recorded separately; see [Register with `claude mcp add`](#register-with-claude-mcp-add).

The session's frames are replayed against the server by
[`claude_mcp_journey.rs`](https://github.com/beyond10x/connectors/blob/main/apps/connectors/tests/claude_mcp_journey.rs)
in every `cargo test` on Linux, where the mounted custody binding it runs against exists. The client opened with `server/discover` on `2026-07-28`, answered:

```json
{"id":"server-discover-probe-1","jsonrpc":"2.0","result":{"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"connectors","version":"0.66.0"}},"cacheScope":"private","capabilities":{"tools":{}},"resultType":"complete","supportedVersions":["2026-07-28","2025-11-25"],"ttlMs":0}}
```

It also sent an `initialize` on `2025-11-25`, which a session already on `2026-07-28` refuses
as an invalid request, and went on with `2026-07-28`. It listed the eleven tools and called the
search tool, with `walk` and `fields` beside its input (the call's `_meta` left out):

```json
{"method":"tools/call","params":{"name":"c1_6a6972612e776f726b_636174616c6f67_6973737565732e736561726368","arguments":{"input":{"jql":"assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC","fields":["summary","status"],"maxResults":100},"walk":true,"fields":"key,fields.summary,fields.status.name"}},"jsonrpc":"2.0","id":4}
```

The answer, with its text `content` left out: `structuredContent` is the owner's audited service
response, and the text content carries the same response (the ids are placeholders in the
recording):

```json
{"id":4,"jsonrpc":"2.0","result":{"isError":false,"resultType":"complete","structuredContent":{"audit_ref":"00000000-0000-4000-8000-000000000003","audit_status":"complete","request_id":"00000000-0000-4000-8000-000000000004","result":{"complete":true,"end":"no_next_page_token","items":[{"fields":{"status":{"name":"To Do"},"summary":"Login form rejects passwords with a trailing space"},"key":"DEMO-1"},{"fields":{"status":{"name":"In Progress"},"summary":"Add a CSV export to the monthly report"},"key":"DEMO-2"},{"fields":{"status":{"name":"To Do"},"summary":"Document the backup restore procedure"},"key":"DEMO-3"}],"pages":1,"records":3},"status":"success","version":"v1alpha2"}}}
```

A session that starts with `initialize` on `2025-11-25` is not in the recording. The test
constructs one from the recorded `initialize` frame and sends it first: the server answers on
`2025-11-25` with the tools capability and, after `notifications/initialized`, lists the same
tools.

### Register with `claude mcp add`

`claude mcp add` writes the entry for you. It was recorded once, with Claude Code 2.1.318, in an
empty project directory, with `/srv/connectors/config.toml` for `CONFIG` and
`/srv/connectors/state` for `STATE_DIR`:

```sh
claude mcp add --scope project connectors -- connectors --config CONFIG --state-dir STATE_DIR server --transport stdio
```

It exited 0, printed nothing on standard error and printed, with `PROJECT_DIR` for the project
directory:

```text
Added stdio MCP server connectors with command: connectors --config /srv/connectors/config.toml --state-dir /srv/connectors/state server --transport stdio to project config
File modified: PROJECT_DIR/.mcp.json
```

It wrote `.mcp.json`, the project-scope file Claude Code reads, in the project directory: the
user form above on those paths, with an empty `env`.

```json title=".mcp.json"
{
  "mcpServers": {
    "connectors": {
      "type": "stdio",
      "command": "connectors",
      "args": ["--config", "/srv/connectors/config.toml", "--state-dir", "/srv/connectors/state", "server", "--transport", "stdio"],
      "env": {}
    }
  }
}
```

[`claude_mcp_journey.rs`](https://github.com/beyond10x/connectors/blob/main/apps/connectors/tests/claude_mcp_journey.rs)
checks the recorded entry against the launch the replay starts and against the user form. The
recording shows the registration only, not a session that uses it.

## A first guarded write

Not available: no MCP operation, read or write, is bound.

## Reconnect when the connection is Stale

As a connector, not available. The requirements are specified: a reconnect must keep the
identity, and an uncertain rotating refresh cannot be exchanged again. The local MCP server
uses the instance's own connection; reconnect it as that connector's page says.

## Limits

- MCP as a connector: no runtime, no configuration schema and no conformance scenarios. The two
  decision blockers that withheld review of its contracts, on caller-to-connection assignment and
  on outbound stdio process ownership, are cleared in the planning store (2026-10-06 and
  2026-10-03); the contracts epic is still a draft.
- The local MCP server serves one instance and one connection per process, with one request at
  a time. It is shown serving on Linux only; the macOS and Windows jobs run a serving scenario
  that has not yet passed there.
- The recorded session read a local fixture site, not a live Jira; its replay serves the same
  instance description and answers from a fixture, not the catalog provider's HTTPS transport.
