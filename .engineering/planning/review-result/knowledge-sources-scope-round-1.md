---
format: aep.planning-md/3
id: review-result:knowledge-sources-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1
relations:
- reviews: epic:catalog-knowledge-sources
- reviews: story:catalog-basic-auth-profile
- reviews: story:catalog-jira-cloud-reads
- reviews: story:catalog-zendesk-reads
- reviews: story:catalog-confluence-reads
- reviews: story:catalog-slack-reads
revision: 1
---
needs-revision
story:catalog-confluence-reads — the epic promises "a time-window filter for deltas (updated since / oldest-latest)" from every source, but the story's page-list operation offers only "filtered or sorted by last modified". Sorting is not a filter, and nothing records that the filter was dropped. The body should name the updated-since filter parameter, or state that the API has none and say how the consumer takes deltas. — .engineering/planning/epic/catalog-knowledge-sources.md:24 vs .engineering/planning/story/catalog-confluence-reads.md:18

What I read: 6 artifacts (the epic and the 5 stories). Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact kinds`, `aep plan artifact relations`. From the epic I extracted 8 promises and traced 7 to an item; the 8th is the Confluence finding above.

The 8 promises:
1. Jira, Zendesk, Confluence and Slack are readable through the catalog provider (one story each).
2. Stable operation ids (in every story's acceptance).
3. Pagination and end condition documented per list operation (every story).
4. A time-window filter for deltas (traced everywhere except Confluence, which is the finding).
5. The provider's item returned unchanged (every story).
6. A basic auth profile as prerequisite (`story:catalog-basic-auth-profile`, with `depends_on` from Jira, Zendesk and Confluence).
7. Slack uses the existing bearer support (`story:catalog-slack-reads`, correctly with no dependency on the basic profile).
8. Out of scope: writes, OAuth, webhooks, extra sources (no story reaches into these).

Not established:
- Whether Confluence v2 has an updated-since filter is an API fact I did not check. The finding rests on the story's wording, not on that.
- The request order (Jira, Zendesk, Confluence, Slack) is not expressed as an edge. I read it as a request order, not an outcome, so it is not a finding.
- Out of my lane: the shared acceptance line "each list operation documents … its time-window filter" cannot hold for Jira comments and changelog or for Zendesk ticket comments, which have no such filter. That is `plan-critic-acceptance`'s call.

```findings
- file: .engineering/planning/story/catalog-confluence-reads.md
  line: 18
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the epic promises "a time-window filter for deltas (updated since / oldest-latest)" from every source, but the story's page-list operation offers only "filtered or sorted by last modified", which narrows a filter to a sort with nothing recording the drop; the body should name the updated-since filter parameter or state that the API has none and how deltas are taken
```
