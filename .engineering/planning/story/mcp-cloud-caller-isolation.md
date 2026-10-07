---
format: aep.planning-md/3
id: story:mcp-cloud-caller-isolation
kind: story
status: draft
title: Caller isolation for a multi-caller cloud MCP placement
relations:
- informed_by: epic:mcp-contracts
- informed_by: decision-blocker:mcp-caller-connection-assignment
- serves: vision:independent-contract-adapters
revision: 1
---
## Why this exists

Moved out of `epic:mcp-contracts` acceptance criterion 3 on 2026-10-07, when
`decision-blocker:mcp-caller-connection-assignment` was answered: the inbound MCP
caller is the configured owner, a single principal, and a multi-caller cloud
placement is deferred. Criterion 3 read "A fixture verifies caller isolation and no
provider-secret disclosure". The no-secret half stays in the epic with
`story:mcp-inbound-cloud-profile`. This story holds the isolation half.

With one principal there is nothing to isolate: `contracts/cli/v1alpha1/semantics.md`
section 1 admits the effective Linux UID as the only owner, and the owner transport
refuses a peer whose UID is not its own. Caller isolation becomes a question only
when a placement serves more than one caller.

## Waits on

A selected multi-caller cloud placement for `$BIN server`
(`decision-blocker:mcp-cloud-placement-selection`). Until then this story stays a
draft and nothing is built for it.

## Acceptance (when the placement is selected)

1. `McpCaller → connectors.auth_bindings.Connection` is modelled in ESS first, with
   its ownership, cardinality and lifecycle coupling, before any story is written
   around it.
2. A fixture shows that caller A reaches only the connections assigned to A, and
   that a request from A naming a connection of B is refused by name with no provider
   dispatch.
3. The answer does not add a hosted identity service or federation to local use. A
   design that needs an Identity dependency goes back to the operator before it is
   modelled.
