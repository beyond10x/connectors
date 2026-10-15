---
format: aep.planning-md/3
id: story:slack-thread-reply-and-channel-audience
kind: story
status: draft
title: Slack thread replies, updates and a channel-audience read that refuses outside readers
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:parity-slack-message-writes
revision: 1
---
## Outcome

A caller answers in the Slack thread a message came from, edits its own answer, and learns
before posting who can read the channel, so it can refuse where people outside the workspace
can read.

## What exists

- `chat.postMessage` (with `thread_ts`) and `chat.update` are selected on the bot selection set
  (`story:slack-message-send`, `story:parity-slack-message-writes`), each under approval.
- No read names a channel's audience; `conversations.info` is not selected.

## Wanted

- **Reply in a thread.** A reply is `chat.postMessage` with `thread_ts` set to the parent
  message; the guide shows it as a reply operation, with the parent named, and a reply to a
  message that is not a thread parent is a typed refusal, not a new top-level post.
- **Update.** `chat.update` on the caller's own message (channel, ts, text), unchanged path.
- **Channel audience.** A read of one channel answering `internal` (members of this workspace
  only), `shared` (shared with other workspaces of the same organization) or `external` (Slack
  Connect: members outside the organization), derived from `conversations.info`'s
  `is_shared`, `is_org_shared`, `is_ext_shared` and `is_pending_ext_shared`; a pending external
  share counts as `external`.
- **Refusal by audience.** A reply or update may name the audience it requires (for example
  "internal only"); the provider reads the channel's audience in the same invocation and refuses
  by name (for example `audience_not_permitted`, with the audience found) before any write.

## Acceptance

- Fixture scenarios: a thread reply lands under its parent; an update changes only the named
  message; `conversations.info` answers each of the four flag combinations and the audience read
  names `internal`, `shared` or `external`; a reply requiring `internal` into an `external` or
  pending-external channel is refused before any request to `chat.postMessage`.
- The parity page lists the reply, update and audience rows.

Spec first: the audience values and the refusal go into the Slack ESS domain and the catalog
selection model before code. Merges with the planned Slack parity rows (files, single-user
lookups) only where they share the selection file; it does not depend on them.
