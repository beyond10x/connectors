---
format: aep.planning-md/3
id: story:cli-timing-cases-under-load
kind: story
status: draft
title: Two timing cases of the CLI suite pass under build-pool load
summary: A lost-session MCP adversary and a lease recollection case failed under load, passed alone
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:mcp-deadline-adversary-under-load
revision: 1
---
# Two timing cases of the CLI suite pass under build-pool load

## Need

On 2026-11-13 a full `cargo test -p connectors --no-fail-fast` run, while the shared build pool was
full, failed two cases that passed when rerun alone:

- `adversary_local_mcp_server::adversary_a_lost_session_leaves_no_provider_read_open`: "the server
  did not reply" / `mcp_unavailable`.
- `connection_lease::a_read_at_10_5_s_of_a_20_s_lifetime_recollects_once`: `left: 0 right: 1`.

Neither touches code the change under test edited.

## Acceptance

- Each case's cause is found: a deadline shorter than a loaded machine needs, or a real defect.
- Both pass in a full run under the same load, or the time they hold is stated and justified.
