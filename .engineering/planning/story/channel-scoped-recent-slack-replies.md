---
format: aep.planning-md/1
id: story:channel-scoped-recent-slack-replies
kind: story
status: draft
title: Expose bounded channel-scoped discovery of recent Slack replies
relations:
- decomposes: epic:incremental-source-reads
- informed_by: story:brain-source-read-operations
revision: 1
---
## Consumer gap
The brain consumer needs to discover recent replies to old channel messages without completing an entire retained parent-history traversal before every current coverage check. The installed 0.7.1 local catalog returned an empty operation set for `operation search --query "slack search messages"`. This is an observed catalog gap, not proof that any particular vendor API or credential supports the required operation.

## Requested contract
Provide an admitted read operation, or document a supported equivalent, that discovers messages and thread replies changed within an explicit bounded interval in exactly one declared channel. Preserve channel and parent identities, message clocks, canonical citations, page continuation, bounded response size and explicit partial/failure results. The consumer selects the existing connection, configuration and state context; authentication and provider limitations remain explicit. Validate whether the available credential type can support the contract before choosing an upstream endpoint. Refuse an unsupported contract rather than silently changing credentials, broadening channel scope or reporting complete coverage.

The consumer must verify returned channel identity and preserve its public/private authorization policy. Direct messages, multi-person messages, undeclared channels and membership-only enrollment remain excluded. Discovery is a read and never publishes a communication. No private consumer evidence, customer identities, actual channels, credentials or source messages belong in this public request or its fixtures.

## Acceptance and handoff
The owning Connectors agents select and verify authoritative vendor contracts, curate the operation and its effect/audience declarations, add independently authored synthetic contract tests, and run the owning catalog and repository gates before any implementation is declared green. Demonstrate a new reply to an old parent, an empty completed interval, bounded multi-page continuation, scope mismatch, unsupported authentication, provider denial and rate-limit handling. Explain any search-index lag or coverage limitation that prevents an exact checked-through claim.

This is a scoped consumer request only. Implementation, merge and release belong to the Connectors repository's agents. The brain session may consume and validate a published release later; it does not own this product's feature or release work.
