---
format: aep.planning-md/3
id: story:launch-hands-an-oauth2-access-token
kind: story
status: implemented
title: connections launch gives a consumer only the current access token of an OAuth2 grant
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:connections-token-issues-an-access-token
scope:
- confidence: cited
  path: contracts/cli/v1alpha1/consumer-launch.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:56Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-26T08:39:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

`connections launch` on an `oauth2_grant` connection gives the pinned consumer a currently valid access
token on descriptor 3 (Windows: the inherited handle), refreshed first when it expires within the lead
time, and never the stored entry with its refresh token.

## Model

`connectors.oauth2.IssueAccessToken` with delivery `consumer_launch`; the launch contract
`contracts/cli/v1alpha1/consumer-launch.md` §3 and §8; `connectors.cli.ConsumerCredentialDelivery`
(`ess/domains/cli.yaml:622-660`) unchanged.

## Design

- The launch path (`crates/connectors-host/src/local/owner/transport.rs:2709-2740`) asks the owner for an
  issue (`story:connections-token-issues-an-access-token`) instead of reading the custody document when the
  connection's profile is `oauth2_grant`; every other profile launches exactly as before.
- Descriptor 3 holds the bare access token, no newline, read-only, as for the document today.
- `consumer-launch.md` gains a section "OAuth2 grants" stating that rule.

## Acceptance

- `launch_oauth2_grant_hands_only_the_access_token`: a test consumer reads descriptor 3; its bytes equal the
  fake server's current access token and contain no refresh token.
- `launch_oauth2_grant_refreshes_inside_lead_time`: one refresh request, then the consumer gets the new
  token.
- `launch_static_profile_is_unchanged`: the existing launch tests pass byte for byte.
- One `AccessTokenIssue` with delivery `consumer_launch` per launch.

## Out of scope

Re-issuing a token to a long-running consumer after launch.
