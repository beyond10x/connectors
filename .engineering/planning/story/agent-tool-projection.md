---
format: aep.planning-md/3
id: story:agent-tool-projection
kind: story
status: draft
title: An agent projection lists a handful of shaped tools from a selection
summary: A projection file narrows a selection to a few operations with short descriptions, fixed or defaulted inputs and projected outputs, for a smaller tools/list
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:claude-connects-to-the-mcp-server
- depends_on: story:definition-reader-feed-extensions
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/providers/jira/agent-operations.json
- confidence: cited
  path: adapters/catalog/providers/slack/agent-operations.json
- confidence: cited
  path: apps/connectors/tests/claude_mcp_journey.rs
- confidence: cited
  path: apps/connectors/tests/fixtures/claude-code/jira-full-tools.json
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: cited
  path: crates/connectors-catalog/tests/definition_agent.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/jira-agent.yaml
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/slack-agent.yaml
- confidence: cited
  path: website/docs/connectors/mcp.md
revision: 4
---
## Outcome

An agent projection is a small selection written for agent use. It sits beside the full selection
of the same catalog bundle, the way `slack-user` sits beside `slack` (connectors-definitions
`connectors/README.md:3-6`; in this repository `adapters/catalog/providers/slack/user-operations.json`
beside `operations.json`). It names a handful of reads, gives each one a short description, and
withholds the parameters an agent does not need to choose. A Claude session pointed at an instance
of that projection sees only those few tools, so its tool definitions cost a fraction of the
tokens of the full selection. The CLI and the full selection are unchanged beside it.

## Design: no new noun

- A projection is a selection.
  - The model is `connectors_catalog.selection` (`adapters/catalog/spec/ess/domains/selection.yaml`)
    plus the connectors-definitions definition (`spec/domains/definition.yaml`: `catalog` :317,
    `Operation.description` :213).
  - No ESS domain is added.
- Withheld parameters already leave the tool input.
  - The notation carries `withhold` (`crates/connectors-catalog/src/definition.rs:622`).
  - `adapters/catalog/src/lib.rs:1313-1342` removes a withheld parameter from the declared
    operation: "undeclared, so the closed input schema refuses it, and absent from the template".
  - So the operation's declared input schema, which `adapters/mcp/contracts/server/v1alpha1/projection.md:141-146,155-158`
    wraps whole, is already the narrowed one. No MCP contract changes.
- Descriptions already exist: a selection's `description` replaces the bundle text
  (`adapters/catalog/src/lib.rs:2711,2785`).
- One instance per server. The stdio server runs one adapter and one connection per process
  (`.engineering/planning/story/mcp-stdio-runtime-branch.md:32`). Claude registers one server for
  the Jira projection instance and one for the Slack projection instance.
- What this story adds:
  - the two projection selections;
  - a description length rule for selections marked as agent projections;
  - the documentation for configuring an instance of one and registering its server.
- Excluded from this story:
  - **Trimming tool answers.** `projection.md:141-146` requires the complete response, so
    narrowing output needs an MCP contract change. It is not drafted here.
  - **Writes.** Mutation tools stay unavailable until a runtime mutation binding exists
    (`projection.md:157`, `mutations.md:3-4`), and the stdio runtime is read-only
    (story:mcp-stdio-runtime-branch). The first projections are reads only.

## Upstream

upstream-blocker:definitions-lack-agent-projections blocks this story until connectors-definitions
`main` carries:
- `connectors/jira-agent.yaml` (`catalog: jira`): `issues.search`, `issue.get`,
  `issue.comments`. `issues.search` withholds `properties`, `fieldsByKeys`, `failFast` and
  `reconcileIssues` (parameters of `searchAndReconsileIssuesUsingJql`, `GET /rest/api/3/search/jql`
  in `adapters/catalog/generated/bundles/jira.bundle.json`).
- `connectors/slack-agent.yaml` (`catalog: slack`): `conversations.list`,
  `conversations.history`, `conversations.replies`, `users.info`.
- The rule in the definitions schema: a definition marked `agent: true` gives every operation a
  `description` of at most 200 bytes and selects at most 8 operations.

Every operation id comes from the shipped selections (`adapters/catalog/providers/jira/operations.json`,
`slack/operations.json`).

## Scope

- `crates/connectors-catalog/src/definition.rs`, `import.rs`: carry `agent` and enforce the rule.
- `crates/connectors-catalog/tests/fixtures/definitions/connectors/{jira-agent,slack-agent}.yaml`:
  vendored from the connectors-definitions commit that clears the blocker, as `slack-user.yaml` is.
- `adapters/catalog/providers/jira/agent-operations.json` and
  `adapters/catalog/providers/slack/agent-operations.json` (new): the shipped selections,
  registered where `slack/user-operations.json` is (`crates/connectors-build/src/main.rs:831`).
- `apps/connectors/tests/fixtures/claude-code/jira-full-tools.json` (new): the `tools/list`
  result of an instance of the full `jira` selection, recorded at this story's base before any
  change, and reviewed in the pull request.
- `apps/connectors/tests/claude_mcp_journey.rs`: the scenarios below.
- `website/docs/connectors/mcp.md`: an "Agent projections" section with both `claude mcp add`
  lines.
- `CHANGELOG.md`.

## Order

- Depends on story:claude-connects-to-the-mcp-server (the server and `claude_mcp_journey.rs`).
- Depends on story:definition-reader-feed-extensions: both change `definition.rs` and `import.rs`,
  and that story changes the Jira selection this story measures against.
- story:definitions-fixture-copy-removed and story:catalog-provider-data-removed depend on this
  story. They delete the vendored copy and `adapters/catalog/providers/` with the two new files in
  them; the definitions then come from the pinned release.

## Acceptance (named conformance scenarios)

`crates/connectors-catalog/tests/definition_agent.rs`:
- `an_agent_definition_with_a_long_description_is_refused`: 201 bytes is refused, naming the
  operation; 200 is accepted.
- `an_agent_definition_with_nine_operations_is_refused`
- `an_agent_definition_without_a_description_is_refused`
- `an_agent_definition_naming_a_write_is_refused`
- `the_jira_and_slack_agent_definitions_lower_to_the_shipped_selections`: the vendored
  `jira-agent.yaml` lowers to `adapters/catalog/providers/jira/agent-operations.json`, and
  `slack-agent.yaml` to `adapters/catalog/providers/slack/agent-operations.json`.

`apps/connectors/tests/claude_mcp_journey.rs`, each server started with the `claude mcp add`
arguments of `mcp.md` and its instance's `base_url` pointed at recorded answers:
- `agent_projection_lists_exactly_its_tools`:
  - the `jira-agent` server's `tools/list` names exactly the 3 Jira tools;
  - the `slack-agent` server's names exactly the 4 Slack tools;
  - each tool's `description` equals the definition's text.
- `withheld_parameter_is_absent_from_the_tool_input`: `reconcileIssues` is in the `issues.search`
  `inputSchema` of the full `jira` server and not in that of the `jira-agent` server; a
  `tools/call` to `jira-agent` that supplies it is refused as an unknown input with no request to
  the recorded server.
- `agent_projection_listing_is_at_most_a_quarter_of_the_full_selection`: the byte length of the
  `jira-agent` server's `tools/list` result is at most 25% of `jira-full-tools.json`. Both numbers
  are printed by the test.
- `full_selection_instance_is_unchanged`: the full `jira` server's `tools/list` result equals
  `jira-full-tools.json`.
