---
format: aep.planning-md/3
id: review-result:mcp-invocation-followup-20261002
kind: review-result
status: active
title: MCP invocation prose reconciliation followup
relations:
- reviews: story:mcp-outbound-invocation-results
revision: 1
---
unit: outbound invocation documentation correction — cb26g-out over a2955675cb70b5811589ce86b40be95427ead888; invocation.md SHA256 346606033482b34ef76884a6d3dc090e9ac76b7e2f8dffe5195acee300b68b7e
verdict: nothing found
cases: executed 0→0, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none authored; managed lease metadata only
needs-coordinator: integration and retained original review history

## 1. Follow-up diff

Review-only source diff: empty (0 files changed). No test or source edits.
The inherited candidate remains five files; the coordinator replaced only the
four stale lines in invocation.md:140–143. This pass does not claim those inherited
changes as its own.

## 2. Bounded source check

No additional case or Cargo execution. The earlier 31 passing document tests
predate this documentation correction and remain historical execution evidence,
not a run against the corrected document.

`sha256sum -c .local/mcp-invocation-review/corrected-document.sha256` exited 0:

```
adapters/mcp/contracts/client/v1alpha1/invocation.md: OK
```

Read the replacement paragraph and both relative Markdown link targets. The
`semantics.md` link resolves beside invocation.md, and its named
`mcp.outbound.framing-refused` heading exists. The scenario link resolves beneath
that same directory and names outcome `mcp.outbound.framing-refused`.

Both sources state unknown business effects after dispatch, establish neither
non-execution nor rollback, and forbid automatic redispatch. The replacement
paragraph accurately points to that distinction. It no longer quotes the removed
non-execution claim or calls reconciliation pending. The original warning is
resolved for the document hash above.

Hashes of the JSON cases and both lifecycle owners are unchanged from the first
review's input manifest; the test file retains its first-review hash
89dc62ece5f3fe896517b83b53597f912a18527177347b49d18fe941eceda431.
No broader third attack, runtime claim, integration gate or publication was performed.

## 3. Findings and handoff

Nothing found within the requested correction. The original report is preserved
unchanged. No authored paths outside this tree; only this separate report was
written under `.local/mcp-invocation-review`. The coordinator owns AEP, integration
and final evidence retention. The follow-up lease was released.

```findings
[]
```
