---
id: S-014
title: "Not-connected is a next step: the response carries a connect URL"
pillar: Platform
status: backlog
priority:
design:
epic: carried-constraints
areas: [protocol, service, server]
note: "research §5 pattern 8, called out there as 'the single most agent-native pattern found' (Arcade authorize→waitForCompletion, Pipedream MCP returning a connect URL, Composio Connect Links mid-conversation). Vision, client contract: not-connected is a next step, not an error"
---

# Not-connected is a next step: the response carries a connect URL

## Goal

When a granted invocation needs a connection that does not exist or has degraded, answer with a
**structured next step** — a connect URL a human can complete, backed by a real connect session —
instead of an error an agent can only report. This is the pattern designed for an agent that hits the
wall mid-task, and it is a client-contract promise in the vision, not a convenience.

## Acceptance

- [ ] A grant-admitted invocation against a missing or degraded connection returns a **distinct,
      structured protocol response** under its own protocol identity, carrying: the integration it
      needs, a connect URL backed by a freshly created connect session, that session's expiry, and how
      to resume. It is not an HTTP status shared with authorization refusals, and it is not an error
      string a client has to pattern-match.
- [ ] It is **not an authorization bypass**: the response is produced only *after* grant admission
      admits the operation. A principal whose grants do not admit the operation receives the ordinary
      refusal and learns nothing about which connections exist or which integrations are configured —
      no enumeration oracle, consistent with the grant rule that a refusal never names the axis that
      refused.
- [ ] The connect URL carries no credential and no tenant secret; the session is single-purpose
      and bound to exactly the integration the operation needs (the domain model's
      `allowed_integrations` binding), so handing the URL to a human cannot widen anything.
- [ ] **Degraded is distinguished from missing**: a degraded connection yields a
      **reauthorize-in-place** URL and the connection id is unchanged afterwards; a missing connection
      yields a create-connection URL. A client that stored the connection id keeps it in the first
      case.
- [ ] The client can complete the loop with the same still-valid personal-local or
      Connectors-audience Identity authority: after the human finishes vendor authorization, the same
      invocation succeeds without a second Identity login or local bootstrap. Identity expiry and
      rotation remain Identity-owned. The end-to-end test includes the wait/poll shape a client uses
      to learn the Connect Session completed.
- [ ] The response shape is fixture-covered in `protocol` conformance, positive and adversarial
      (expired session, session completed for a different integration, session already consumed), so
      a future SDK test suite shares the fixtures verbatim.

## Progress
- (not started)

## Notes

- Research: [unified-api-platforms.md](../research/unified-api-platforms.md) § 5 pattern 8, and § 1's
  note that Pipedream's remote MCP server returns a connect URL when auth is missing. Vision, client
  contract: *"Not-connected is a next step, not an error … auth-as-tool-result, designed for agents
  that hit the wall mid-task."*
- Depends on connect sessions and connection lifecycle from
  [S-008](S-008-m3-connect-a-provider-and-invoke-it.md), and on
  [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md) for what the URL does in
  personal posture (where the human may not be at the machine that will receive the callback).
- The interesting failure mode to design against is a chatty one: an agent that receives a connect URL
  on every call for an integration nobody intends to configure. Decide whether the response is rate-
  limited or session-reused per (principal, integration), and record the reason.
