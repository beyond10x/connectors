---
format: aep.planning-md/3
id: story:mcp-stdio-runtime-branch
kind: story
status: draft
title: Re-land the read-only MCP stdio server kept on branch unit/mcp-local-runtime-20261003-rebased
relations:
- decomposes: epic:mcp-contracts
revision: 1
---
## Outcome

The production `connectors server --transport stdio` command lists and invokes admitted read-only
tools through the protected local owner, so an MCP client can use Connectors operations directly.

## Where the work is

Branch `unit/mcp-local-runtime-20261003-rebased` on `origin` (head `5140636d9`), 14 commits,
195 files, about 19,000 inserted lines, last rebased onto `main` on 2026-10-05. It was proposed as
pull request https://github.com/beyond10x/connectors/pull/93, closed on 2026-10-08 unmerged
because `main` has moved by many releases since; the branch is kept.

What the branch adds:

- native MCP framing and lease supervision (`adapters/mcp/runtime`, generated launch CLI, launch
  and configuration types, supervision plan);
- audited lossless reads for governed bindings in the host;
- complete operation metadata, modeled and validated, with local operation curation bound to
  executable and bootstrap pins;
- a shared provider deadline across HTTP capabilities, carried through native catalog execution
  and owner admission with the original deadline preserved;
- the stdio server: one adapter and Connection per process, inline schemas, one active owner
  request, protocol revisions `2026-07-28` and `2025-11-25`; disabled, hidden, approval-required
  and ungranted operations omitted or refused before provider work;
- planning records it carried: stories `mcp-local-stdio-runtime`, `mcp-outbound-stdio-runtime`,
  `metadata-sidecar-fixture-isolation`, decision-blockers `helm-execution-family` and
  `mcp-outbound-stdio-process-ownership`, a delivery specification and four plan reviews.

## Acceptance

- The branch's behaviour is re-landed on current `main` in reviewed units (a fresh rebase or a
  re-implementation against the current ESS), each with its tests; the planning records it carried
  are re-created through the planning CLI, not copied.
- The stdio server answers `tools/list` and `tools/call` for an admitted read over both protocol
  revisions in an end-to-end test against the installed binary.
- Spec first: the CLI and service-wire changes are declared in ESS before the code.
