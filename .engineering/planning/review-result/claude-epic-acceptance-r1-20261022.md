---
format: aep.planning-md/3
id: review-result:claude-epic-acceptance-r1-20261022
kind: review-result
status: active
title: 'Plan-critic acceptance, round 1: connectors with Claude'
summary: needs-revision; findings addressed in revision 2
relations:
- reviews: epic:connectors-with-claude
- reviews: story:claude-getting-started-tutorial
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:agent-tool-projection
- reviews: story:definition-reader-feed-extensions
revision: 1
---
# Plan-critic, acceptance lens, round 1: connectors with Claude and the definition feed story

**Verdict: needs-revision. 16 findings, 4 blockers. All are addressed in revision 2 of each body.**

The set reviewed (revision 1) was epic:connectors-with-claude, story:claude-getting-started-tutorial,
story:claude-connects-to-the-mcp-server, story:agent-tool-projection and
story:definition-reader-feed-extensions, at base `9b7676614`.

| Story | Finding | Disposition in revision 2 |
|---|---|---|
| tutorial | recorded runs had no location, format or comparison rule | `website/data/runs/getting-started/*.json`, its fields, and placeholders for version and timestamps |
| tutorial | Slack section had no check; the blocker fallback passed every scenario | `getting_started_sections_show_their_operations`; Slack ordered behind story:instances-resolve-through-locked-index |
| tutorial | the section-order test checked headings only | the content of With Claude is checked |
| tutorial | the journey had no pass criterion and no record | exit codes, the approval refusal and `test_result` evidence |
| mcp (blocker) | ESS scenarios asserted wire outcomes ESS does not execute | replaced by executing tests in `claude_mcp_journey.rs` |
| mcp | the recorded session and "ids" were undefined | `tests/fixtures/claude-code/session.jsonl`; JSON-RPC `id` and `*_at` members removed before comparing |
| mcp | the write outcome had no scenario | `no_write_tool_is_listed` |
| mcp | four claim changes had no acceptance | `mcp_is_supported`, the `status.rs` and `onboarding.rs` tests |
| projection (blocker) | no check of the exact tool set or descriptions | `agent_projection_lists_exactly_its_tools` |
| projection (blocker) | the write approval could not be observed | writes removed from the first projections |
| projection (blocker) | `search.messages` was not in the Slack selection | Slack ids taken from `slack/operations.json` |
| projection | ESS scenarios for a domain that does not exist | no new domain; executing Rust tests |
| feed | `every_shipped_selection_is_covered` reads the same before and after | the per-selection `*_round_trips` and `*_is_committed` tests |
| feed | refusals did not cover every rule | 13 named refusals for the 10 rules |
| feed | conformance suites named as modules, not tests | shipped-selection read tests and a red check |
| feed | the blocker transition was not checkable | `open` to `cleared` through `aep plan artifact show` |
