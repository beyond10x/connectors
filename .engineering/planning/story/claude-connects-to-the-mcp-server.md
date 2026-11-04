---
format: aep.planning-md/3
id: story:claude-connects-to-the-mcp-server
kind: story
status: active
title: Claude Code connects to connectors as a local stdio MCP server
summary: claude mcp add registers connectors server --transport stdio; tools/list holds the admitted reads, tools/call reads through the owner
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-stdio-runtime-branch
- depends_on: story:docs-first-run-path
- depends_on: story:definition-reader-feed-extensions
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/projection-cases.json
- confidence: cited
  path: adapters/mcp/contracts/server/v1alpha1/projection.md
- confidence: cited
  path: apps/connectors/src/mcp/projection.rs
- confidence: cited
  path: apps/connectors/tests/claude_mcp_journey.rs
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/jira-search.json
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/session.expected.jsonl
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/session.jsonl
- confidence: inferred
  path: apps/connectors/tests/local_mcp_server.rs
- confidence: cited
  path: crates/connectors-build/tests/mcp_inbound_capability_projection.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: website/data/status.json
- confidence: cited
  path: website/docs/connectors/mcp.md
- confidence: cited
  path: website/docs/status.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T17:49:18Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "proposed", to: "active", at: "2026-10-23T17:49:18Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"review_outcome":7}}}
---
## Outcome

Claude Code connects to Connectors as a local stdio MCP server, registered with one command:

    claude mcp add connectors -- connectors --config CONFIG --state-dir STATE_DIR server --transport stdio

This is the spelling chosen in `docs/local-mcp-cli.md:21`. The documentation shows that line and
a recorded session. The support claims move from planned to shipped.

## Ownership

- story:mcp-stdio-runtime-branch owns the server and its generic scenarios in
  `apps/connectors/tests/local_mcp_server.rs`: `stdio_lists_and_calls_an_admitted_read_on_2026_07_28`,
  `…_on_2025_11_25`, the four omitted-or-refused classes with no provider request, and
  `stdout_carries_only_json_rpc_frames`. This story depends on it and repeats none of those tests.
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
  - Depends on story:definition-reader-feed-extensions: its feed block adds 2 reads to the Jira
    selection (`adapters/catalog/src/lib.rs:1579-1597`), so the `tools/list` this story records is
    taken after it lands.
  - story:claude-getting-started-tutorial depends on this story, links `mcp.md` and edits neither
    the support table nor `onboarding.rs`. This story does not wait for the tutorial or for
    story:instances-resolve-through-locked-index.
  - The `compatibility.json` flip is this story's; the runtime story keeps `deferred`.
- Writes are not listed. Mutation tools stay unavailable (`adapters/mcp/contracts/server/v1alpha1/projection.md:157`,
  `mutations.md:3-4`), and this story asserts their absence, not an approval path.
- The closing report of this story asks b10x-chef-orchestrator for connectors-definitions
  `connectors/jira-agent.yaml`, `connectors/slack-agent.yaml` and the `agent` rule, as listed in
  upstream-blocker:definitions-lack-agent-projections; story:agent-tool-projection waits on them.

## Scope

- `apps/connectors/tests/fixtures/claude-code/session.jsonl` (new): the JSON-RPC frames a Claude
  Code client sends: `initialize` (protocol revision `2025-11-25`), `notifications/initialized`,
  `tools/list`, one `tools/call` of the Jira `issues.search` tool. Each frame is recorded once from
  a real client, with the client version in a header line.
- `apps/connectors/tests/fixtures/claude-code/session.expected.jsonl` (new): one expected answer
  per request in `session.jsonl`, written from the wire forms of
  `adapters/mcp/contracts/server/v1alpha1/projection.md` and reviewed in the pull request.
- `apps/connectors/tests/claude_mcp_journey.rs` (new): starts the release-profile `connectors`
  binary with the `claude mcp add` arguments, under the `mounted_secret` binding in the jail of
  `apps/connectors/tests/support/mounted.rs`. The Jira instance's `base_url` is a loopback
  listener the test serves, answering `GET /rest/api/3/search/jql` with
  `apps/connectors/tests/fixtures/claude-code/jira-search.json` (new), as
  `apps/connectors/tests/revalidate_transient.rs` serves its provider. Runs in `cargo test`, not
  `#[ignore]`d.
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

## Scope (wave 2026-11-04)

Derived by `story-scoper` against `a99bab09c` (v0.65.2). Each line is cited (read in the tree) or
inferred. Line numbers in the sections above have drifted; these are current.

- Decided rule (decision-blocker:mcp-tool-call-without-revision, option A), not on main yet:
  `apps/connectors/src/mcp/projection.rs:346-352` refuses a call with no revision with `-32602`
  (cited); the contract rule is `adapters/mcp/contracts/server/v1alpha1/projection.md:124-127` and
  its case table `:84-99`, revision rows `:96-98` (cited); the cases are
  `adapters/mcp/contracts/server/v1alpha1/projection-cases.json:267-478`, which needs a field that
  can say "absent" (inferred), checked by
  `crates/connectors-build/tests/mcp_inbound_capability_projection.rs:41-55`, `:138-148` (cited).
- Runtime cases for an absent, a current and a stale revision: `apps/connectors/tests/local_mcp_server.rs`
  (`Peer::call` at `:539-544` always sends `_meta`, so the absent case needs a raw frame) (inferred).
- The server answers `server/discover` on `2026-07-28` today (`local_mcp_server.rs:627-636`, cited).
- Journey: `apps/connectors/tests/claude_mcp_journey.rs` (new, Linux only like
  `local_mcp_server.rs:11`) and `apps/connectors/tests/fixtures/claude-code/` (new). The debug test
  binary is used, as `local_mcp_server.rs:404-411` does: the gate runs no `--release` build
  (`crates/connectors-build/src/gate.rs:194`), so a release-profile journey could only be an ignored
  test (cited).
- Claims: the MCP row of `crates/connectors-docs/src/onboarding.rs:483-492` and the status item at
  `crates/connectors-docs/src/status.rs:588-593` describe MCP as a connector (outbound), which stays
  planned. The inbound stdio server gets its own shipped status item; `status.rs:591` ("there is no
  MCP runtime") is reworded; `website/docs/connectors/mcp.md` keeps a Planned box for outbound only
  and gains the registration line; `README.md:283-287` gains the registration line; no README
  support-table row (cited, `onboarding.rs:148`, `:323-329`).
- Dropped: `apps/connectors/spec/compatibility.json` was removed in `e431f9573` and
  `apps/connectors/tests/compatibility.rs:55-62` asserts its absence (cited).
- Generated: `website/docs/status.md`, `website/data/status.json` (cited).
- Shared with `story:catalog-github-reads`: `status.rs` (different items; `AS_OF` at `:21`),
  the generated status files, `README.md` (a paragraph here, a table row there), `CHANGELOG.md`.
- Confidence: high for contract, code and claims; medium for the journey's Jira stand-in
  (`apps/connectors/tests/support/mounted.rs:37`, `:79`, `:263`).

## Wave 2026-11-04: the revision rule only

The directing session decided on 2026-11-04: "U1 recording: C. A Claude Code login in an isolated
HOME is a credential, and that is [the operator's] alone. U1 delivers the revision rule and its
tests; the story stays active." This wave implements decision-blocker:mcp-tool-call-without-revision
option A (a `tools/call` with no revision binds to the advertised one; a stale revision is still
refused) with its contract cases and runtime cases. The recorded session, the journey and the
claims wait for the recorded run, and the story stays active until then.
