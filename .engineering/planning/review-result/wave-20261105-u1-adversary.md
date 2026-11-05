---
format: aep.planning-md/3
id: review-result:wave-20261105-u1-adversary
kind: review-result
status: active
title: Adversary pass on the recorded Claude Code session
relations:
- reviews: story:claude-connects-to-the-mcp-server
revision: 1
---

**Verdict: red, 2 should-fix confirmed by red cases, 1 should-fix confirmed by the coordinator, 2 notes. The orchestrator decided that all five are fixed in the unit before the merge.**

The adversary ran on `5cd05a187`, wave 2026-11-05 unit U1 (the Claude Code recording). Its cases were added uncommitted to `apps/connectors/tests/claude_mcp_journey.rs`. Executed 5 to 8. Red 2, each run alone first:
- `adversary_transcript_keeps_no_api_issued_signature`: "a raw API-issued thinking signature of 1040 characters is kept".
- `adversary_no_write_case_offers_the_server_a_write`: the case exposes six writes that the instance never describes.

`adversary_normalization_keeps_what_a_client_depends_on` is green. It applies eight mutations to the expected answers (tool name, `inputSchema.required`, `readOnlyHint`, a `structuredContent` item key, the text content, `isError`, `supportedVersions`, -32600 changed to -32601), and the normalization keeps every one visible.

Not broken:
- replay order and completeness: every frame is sent in recorded order, ids are matched, and a stray stdout frame fails the run;
- the stand-in's tool description equals the recorded one, because `tools/list` is compared in full;
- the -32600 on the late initialize is real server state (`protocol.rs:67-69`);
- the doc JSON snippets equal the fixtures;
- fixture hygiene: scan-text is clean, with no home paths or emails.

The coordinator confirmed F3 from the rig: the recorded `--mcp-config` entry named the capturing wrapper as `command`. The wrapper tees both directions to capture files, then runs `connectors` with the same arguments in a bubblewrap sandbox.

Decisions taken (orchestrator):
1. `no_write_tool_is_listed` is restated as "lists exactly the 11 recorded read tools". The no-write claim is dropped from the test, its docs and the story acceptance. U2's cases (story:mcp-write-operation-call-by-name-is-refused) carry the proof of the write guard.
2. The thinking signature is replaced with a fixed placeholder, extending the fixture approval. scan-text and the grep run again, and the adversary's case must be green.
3. The docs show the capturing wrapper that ran as the command, then the `connectors` entry a user writes, labelled as the user form.
4. The page says the replay runs in every cargo test on Linux.
5. The test compares the transcript's tool-call arguments and results with the session frames, not only the names.

```findings
[
 {"file":"apps/connectors/tests/claude_mcp_journey.rs","line":1071,"category":"mutant","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F1: no_write_tool_is_listed exposes writes the stand-in never describes, so no change to the projection's write guard (projection.rs:213-230) can turn it red"},
 {"file":"apps/connectors/tests/fixtures/claude-code/transcript.jsonl","line":3,"category":"judgement","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F2: the thinking signature is an unsanitized 1040-character API-issued blob while every other API id is a placeholder"},
 {"file":"README.md","line":287,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"F3: README and mcp.md:72 say the recorded --mcp-config named command connectors; the rig's recorded entry named the capturing wrapper"},
 {"file":"website/docs/connectors/mcp.md","line":100,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"F4: the page says the replay runs in every cargo test, but claude_mcp_journey.rs is cfg(target_os = \"linux\") only"},
 {"file":"apps/connectors/tests/claude_mcp_journey.rs","line":911,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"F5: the transcript is held only by its tool-use names, so its arguments and results could drift from session.jsonl and session.expected.jsonl unnoticed"}
]
```
