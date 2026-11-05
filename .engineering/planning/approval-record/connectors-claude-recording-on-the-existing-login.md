---
format: aep.planning-md/3
id: approval-record:connectors-claude-recording-on-the-existing-login
kind: approval-record
status: approved
title: One recorded Claude Code run on the existing login, limited by --tools, scanned before commit
relations:
- decides: story:claude-connects-to-the-mcp-server
revision: 5
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T09:33:20Z", actor: "human:operator", revision: 2}
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

## Transcript fixture (orchestrator approval, 2026-11-05)

The one recorded run took place 2026-11-05 09:54:58Z to 09:55:16Z, 3 turns, cost 0.14 USD, stderr empty. The scan of the raw transcript found one rule hit: personal-paths in the session-start record (its working-directory fields). The scan found no email, token or credential. Nothing was deleted or committed, and the scan output went to the orchestrator first.

The orchestrator approved the fixture plan as follows:

- The transcript fixture keeps only the assistant, user and result records. The session-start record is dropped: working-directory paths, socket path, local skill and plugin lists, API key source.
- Session ids, request ids, message ids and tool-use ids are replaced on every line with fixed placeholders. The same raw id gets the same placeholder in every file.
- scan-text and the grep for tokens, emails, hostnames and home paths run again on the files to be committed, and both show 0 hits before the commit.
- The client and server frames are the primary evidence for the claim. The raw files stay local and none is committed.
- Phase 2:
  - The acceptance is restated as "11 reads, no write tool listed".
  - The fixtures show both the discover fallback and initialize at protocol 2025-11-25.
  - The journey test uses the stand-in adapter.
- Claude runs only through the coordinator.

Applied beyond the plan, under the redaction rule of the original go:

- The result record's credential-expiry field is removed.
- The rate-limit record (account quota state) is dropped with the other non-kept records.

Re-scan of the three fixture files (transcript, client frames, server frames): scan-text "3 file(s) carry no private rule", and the grep found 0 home paths, 0 emails, 0 tokens and 0 residual raw ids. The only hostnames found are the provider's media host (`api.media.atlassian.com`, in a tool description) and the client's website (`claude.com`, in its clientInfo).

## Correction: the recording shows no protocol fallback (2026-11-05)

The coordinator's report of the run said that the client fell back from `server/discover` to `initialize` at 2025-11-25. The frames show otherwise, read from the sanitized client and server frames:

1. `server/discover` (protocol 2026-07-28) was answered with `supportedVersions` 2026-07-28 and 2025-11-25.
2. `initialize` (2025-11-25) was refused with -32600, because the session was already on 2026-07-28.
3. A second `server/discover`, then `tools/list` and two `tools/call`, all on 2026-07-28.

So the approved point "the fixtures show both the discover fallback and initialize at protocol 2025-11-25" is restated:
- The recording shows discover on 2026-07-28 and the client's refused `initialize`.
- A separate test shows the 2025-11-25 handshake. It is constructed from the recorded `initialize` frame plus a `notifications/initialized` that is not in the recording, and it is labelled as constructed.

The orchestrator agreed with recording the correction here.

The pages show only the registration that ran: the `--mcp-config` server entry and the one `claude -p` command. No second claude call is made in this wave. The `claude mcp add` registration is filed as a draft for a later wave; it needs the orchestrator's go and one recorded run.

## Extension: the thinking signature (orchestrator decision, 2026-11-05)

The U1 adversary found that the transcript fixture kept the thinking block's `signature`, a 1040-character API-issued value. It decodes to protobuf framing around high-entropy bytes, with no readable text. The orchestrator extended the fixture approval: the signature is replaced with a fixed placeholder (`signature_fixture0001`) by the same sanitizer as the ids.

The re-scan of the three sanitized files gave:
- scan-text: "3 file(s) carry no private rule";
- 0 home paths, emails, tokens or residual raw ids;
- no run of 200 or more base64-like characters left in any of them;
- hostnames only `api.media.atlassian.com` (a tool description) and `claude.com` (the client's website field).

Only the signature differs from the earlier transcript fixture.
