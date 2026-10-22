---
format: aep.planning-md/3
id: story:claude-connects-to-the-mcp-server
kind: story
status: draft
title: Claude Code connects to connectors as a local stdio MCP server
summary: claude mcp add registers connectors server --transport stdio; tools/list holds the admitted reads, tools/call reads through the owner
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-stdio-runtime-branch
- depends_on: story:docs-first-run-path
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: apps/connectors/spec/compatibility.json
- confidence: cited
  path: apps/connectors/tests/claude_mcp_journey.rs
- confidence: cited
  path: apps/connectors/tests/compatibility.rs
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/session.expected.jsonl
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/session.jsonl
- confidence: cited
  path: crates/connectors-docs/src/onboarding.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: website/docs/connectors/index.md
- confidence: cited
  path: website/docs/connectors/mcp.md
- confidence: cited
  path: website/docs/status.md
revision: 4
---
## Outcome

Claude Code connects to Connectors as a local stdio MCP server, registered with one command:

    claude mcp add connectors -- connectors --config CONFIG --state-dir STATE_DIR server --transport stdio

This is the spelling chosen in `docs/local-mcp-cli.md:21`. The documentation shows that line and
a recorded session. The support claims move from planned to shipped.

## Ownership

- story:mcp-stdio-runtime-branch owns the server: the generic `tools/list`, `tools/call`,
  refusal and stdout tests (its acceptance, `.engineering/planning/story/mcp-stdio-runtime-branch.md:44-45`).
  This story depends on it and repeats none of those tests.
- This story owns what is specific to Claude Code and the claims:
  - the recorded Claude Code handshake;
  - the `claude mcp add` spelling;
  - `website/docs/connectors/mcp.md`;
  - the README support-table row;
  - `website/docs/status.md`;
  - `apps/connectors/spec/compatibility.json`.
- Order:
  - Depends on story:mcp-stdio-runtime-branch and story:docs-first-run-path. That story moves the
    README install text; this story then adds the support-table row (`onboarding.rs` reads the
    README support table).
  - story:claude-getting-started-tutorial depends on this story, links `mcp.md` and edits neither
    the support table nor `onboarding.rs`. This story does not wait for the tutorial or for
    story:instances-resolve-through-locked-index.
  - The `compatibility.json` flip is this story's. If the runtime branch carries it, the runtime
    story leaves it out when it re-lands.
- Writes are not listed. Mutation tools stay unavailable (`adapters/mcp/contracts/server/v1alpha1/projection.md:157`,
  `mutations.md:3-4`), and this story asserts their absence, not an approval path.

## Scope

- `apps/connectors/tests/fixtures/claude-code/session.jsonl` (new): the JSON-RPC frames a Claude
  Code client sends: `initialize` (protocol revision `2025-11-25`), `notifications/initialized`,
  `tools/list`, one `tools/call` of the Jira `issues.search` tool. Each frame is recorded once from
  a real client, with the client version in a header line.
- `apps/connectors/tests/fixtures/claude-code/session.expected.jsonl` (new): one expected answer
  per request in `session.jsonl`, written from the wire forms of
  `adapters/mcp/contracts/server/v1alpha1/projection.md` and reviewed in the pull request.
- `apps/connectors/tests/claude_mcp_journey.rs` (new): starts the release-profile `connectors`
  binary with the `claude mcp add` arguments. Its configuration points the Jira instance at the
  recorded fixture server the runtime story's tests use.
- `website/docs/connectors/mcp.md`: the `:::caution[Planned]` box goes; the page gains the
  registration line and the recorded session. `getting-started.md` is not edited here; the
  tutorial links this page when it lands.
- `README.md`: an MCP server row in the support table.
- `crates/connectors-docs/src/onboarding.rs`:
  - the MCP `Hand` row (:493-497) loses "planned";
  - the test at :919-923 asserting "MCP is listed as planned" asserts the shipped row instead;
  - the generated `website/docs/connectors/index.md` is regenerated with
    `connectors-docs generate`.
- `website/docs/status.md:104` and `crates/connectors-docs/src/status.rs`: shipped, held by the
  test below.
- `apps/connectors/spec/compatibility.json:7-12`: `"mcp"` leaves `deferred`.
- `CHANGELOG.md`.

## Acceptance (named conformance scenarios)

`apps/connectors/tests/claude_mcp_journey.rs`:
- `claude_code_session_is_answered`: replaying `session.jsonl` gives one answer per request.
  - `initialize` answers revision `2025-11-25` with the `tools` capability.
  - `tools/list` holds the Jira instance's admitted reads and no write.
  - `tools/call` answers `isError:false` with the fixture's issues.
  - Every stdout line parses as a JSON-RPC frame.
  - Each answer equals its line in `session.expected.jsonl` after the JSON-RPC `id` and every
    member named `*_at` are removed from both.
- `claude_mcp_add_arguments_start_the_server`: the arguments in `website/docs/connectors/mcp.md`'s
  registration line, parsed from the page, start the server and it answers `initialize`.
- `no_write_tool_is_listed`: with `issue.comment.add` enabled and an approval source configured,
  `tools/list` still lists no write.

`apps/connectors/tests/compatibility.rs`:
- `mcp_is_supported`: `compatibility.json` declares `mcp` supported.

`crates/connectors-docs` (`cargo test -p connectors-docs`):
- `status.rs`: the MCP server item is shipped and names `claude_code_session_is_answered`.
- `onboarding.rs::the_index_lists_every_connector_and_links_its_page` (:897): the assertion at
  :919-923 changes from "MCP is listed as planned" to the shipped row; it fails before the `Hand`
  row change and passes after.

Try-it (operator, not CI): Claude Code with the server registered answers "list my open Jira
issues" from a sandbox site.
