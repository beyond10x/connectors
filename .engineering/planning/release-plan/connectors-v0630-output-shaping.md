---
format: aep.planning-md/3
id: release-plan:connectors-v0630-output-shaping
kind: release-plan
status: draft
title: 'Release 0.63.0: output shaping in invoke and MCP'
tags:
- d240
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Minor release 0.63.0 closes wave D240 under `approval-record:wave-d240-output-shaping`.

- `invoke --fields` and `--each` print only the named members of a provider result.
- A catalog read can declare a page walk, published as `x-connectors-walk`.
- `invoke --walk --out <file|->` follows a declared walk, page by page, and writes NDJSON.
- MCP `tools/call` accepts the same shaping and walk arguments.

Breaking: MCP `outputSchema.properties.result` is now any JSON, and the operation schema moves to
`_meta`.

## Scope

| story | status |
|---|---|
| story:invoke-shapes-its-result | implemented |
| story:catalog-operation-declares-its-walk | implemented |
| story:invoke-walks-declared-pages | implemented |
| story:mcp-tool-call-shapes-its-result | implemented |

## Limitations

The CHANGELOG's 0.63.0 Limitations list them. The walk in the definition lowerer is the draft
`story:definition-lowerer-carries-walk`. Locally only the touched targets and the projection
checks ran; the full gate is CI's gate on the pull request.
