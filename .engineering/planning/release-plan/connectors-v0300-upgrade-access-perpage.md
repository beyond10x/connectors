---
format: aep.planning-md/3
id: release-plan:connectors-v0300-upgrade-access-perpage
kind: release-plan
status: active
title: 'Release 0.30.0: configuration upgrades, access probes and Zendesk per_page'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-06T04:34:29Z", actor: "human:timo", revision: 3}
---
## Outcome and authorization

Prepare minor release 0.30.0 from origin/main f7f6fbb. Operator rule "ready means ship"
(2026-09-25) and the operator's standing authority for this session's decisions until
10:00 on 2026-10-06. The tag namespace ends at v0.29.0, an ancestor of that base. Recheck
both before tagging.

Minor, not patch: new configuration values (catalog `auth.access`,
`connectors-source-amendments/1`, `--amendments`) and changed CLI refusals
(`next_action: create_connection` on a changed binding). Existing configurations, stored
metadata and connections are unchanged; a configuration without `access` keeps its
revision.

## Released scope (v0.29.0..f7f6fbb)

| PR | Change |
|---|---|
| #100 | Cited source amendments; Zendesk `tickets.incremental` accepts `per_page` (1 to 1,000). |
| #107 | Catalog `auth.access` read at validation; Confluence guide declares a v2 page read (fixes #102). |
| #108 | A connection follows a configuration upgrade without credential re-entry (story:connection-follows-configuration-upgrade). |
| #98, #99, #106 | Planning only: the consumer launch plan. |

## Runtime evidence

- 2026-10-06, a live Zendesk Support account: a 30-day ticket export page with `per_page`
  100 answered 200 with 100 tickets and a cursor, where the same window without it was
  refused as `capacity`; a 30-day walk at 200 per page completed.
- 2026-10-06, all 32 permitted reads across five live connections (Zendesk 7, GitLab 12,
  Jira 3, Tavily 3) answered except Confluence's 4, refused `unauthorized` (#102).

## Not in this release

- #101 store growth (CLI open verifies the whole store; clock floor re-recorded per
  command); upstream beyond10x/entity-runtime#55.
- #103 owner memory; #104 executable re-pin; #105 schemas as JSON text.
- An adapter-reported identity mismatch during an upgrade still names
  `repair_connection`.
