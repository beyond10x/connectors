---
format: aep.planning-md/3
id: review-result:knowledge-sources-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1
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
story:catalog-jira-cloud-reads — this and story:catalog-zendesk-reads, story:catalog-confluence-reads and story:catalog-slack-reads each run `connectors-build catalog` into `adapters/catalog/generated/bundles`, which rewrites the one shared `index.json`, and no body says so (cited: index.json holds a single `entries` list and `bundle::write` reads and rewrites it whole; inferred: each story commits its own row) — .engineering/planning/story/catalog-jira-cloud-reads.md:14
story:catalog-zendesk-reads — same `adapters/catalog/generated/bundles/index.json` collision with the jira, confluence and slack stories, unnamed; the remedy is either an ordering edge recording that file as its reason, or splitting the surface (inferred) — .engineering/planning/story/catalog-zendesk-reads.md:14
story:catalog-confluence-reads — same `adapters/catalog/generated/bundles/index.json` collision with the jira, zendesk and slack stories, unnamed (inferred) — .engineering/planning/story/catalog-confluence-reads.md:14
story:catalog-slack-reads — same `adapters/catalog/generated/bundles/index.json` collision with the jira, zendesk and confluence stories, unnamed (inferred) — .engineering/planning/story/catalog-slack-reads.md:13
story:catalog-jira-cloud-reads — "the provider guide" names no file, and the only guide in the tree is `docs/local-catalog-provider.md` (titled "GitLab through the catalog provider", one file); if all four provider stories write pagination and time-window notes there they collide with each other and with story:catalog-basic-auth-profile's Limits edit (inferred) — .engineering/planning/story/catalog-jira-cloud-reads.md:25
story:catalog-slack-reads — has no `depends_on` on story:catalog-basic-auth-profile, so it can run alongside that story while both may edit `docs/local-catalog-provider.md` (basic-auth cites it at its acceptance line, the Slack guide target is unnamed), and the plan reads as parallel-safe when it is unproven (cited for basic-auth, inferred for Slack) — .engineering/planning/story/catalog-slack-reads.md:23
story:catalog-basic-auth-profile — the body names no code surface; the auth profile lives in `adapters/catalog/src/local.rs` (`AuthConfig`, `bearer`, `http_bearer`) and the fixture harness in `adapters/catalog/tests/local_runtime.rs`, which the provider stories' fixture tests may also touch, and the body does not say so (inferred) — .engineering/planning/story/catalog-basic-auth-profile.md:22
story:catalog-jira-cloud-reads — `<provider>` stays unfilled in `adapters/<provider>/upstream/`, and `adapters/atlassian/` already exists, so this and story:catalog-confluence-reads may both land in one directory and one bundle name; the body should fix the provider id and upstream path (inferred) — .engineering/planning/story/catalog-jira-cloud-reads.md:14
story:catalog-confluence-reads — the same unfilled `<provider>` slot and the same `adapters/atlassian/` neighbour as story:catalog-jira-cloud-reads; the body should fix the provider id and upstream path (inferred) — .engineering/planning/story/catalog-confluence-reads.md:14
story:catalog-zendesk-reads — "a test against a recorded fixture per operation" does not say where the tests land, and `adapters/catalog/tests/bundle_drift.rs` and `shipped.rs` are hard-coded to gitlab, so each provider story is likely to edit the same two files unless each gets its own test file (inferred) — .engineering/planning/story/catalog-zendesk-reads.md:26

What I read: 6 artifacts (`aep plan artifact show` on the epic and five stories, plus `aep plan artifact graph`). I also read `adapters/catalog/{src/local.rs,tests/*,realizations/local.json,generated/bundles/index.json}`, `crates/connectors-catalog/src/bundle.rs` and `docs/local-catalog-provider.md`.

Surfaces established: 1 cited (story:catalog-basic-auth-profile, `docs/local-catalog-provider.md`), 4 inferred (the four provider stories), 0 unplaceable.

Could not establish, and out of my lane: whether the pagination and end-condition acceptances are checkable (acceptance critic); whether Slack's `bearer: true` covers its case and whether Atlassian header parameters are refused at load (design critic); what `adapters/catalog/tests/local_runtime/` will need for a basic profile.

```findings
- file: .engineering/planning/story/catalog-jira-cloud-reads.md
  line: 14
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: this and story:catalog-zendesk-reads, story:catalog-confluence-reads and story:catalog-slack-reads each run connectors-build catalog into adapters/catalog/generated/bundles, which rewrites the one shared index.json, and no body says so (cited index.json single entries list and bundle::write read-modify-write; each story committing its own row is inferred)
- file: .engineering/planning/story/catalog-zendesk-reads.md
  line: 14
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: same adapters/catalog/generated/bundles/index.json collision with the jira, confluence and slack stories, unnamed; remedies are an ordering edge naming the file or splitting the surface (inferred)
- file: .engineering/planning/story/catalog-confluence-reads.md
  line: 14
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: same adapters/catalog/generated/bundles/index.json collision with the jira, zendesk and slack stories, unnamed (inferred)
- file: .engineering/planning/story/catalog-slack-reads.md
  line: 13
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: same adapters/catalog/generated/bundles/index.json collision with the jira, zendesk and confluence stories, unnamed (inferred)
- file: .engineering/planning/story/catalog-jira-cloud-reads.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the provider guide names no file and the only guide in the tree is docs/local-catalog-provider.md, so the four provider stories and story:catalog-basic-auth-profile's Limits edit may all land in one file (inferred)
- file: .engineering/planning/story/catalog-slack-reads.md
  line: 23
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: no depends_on on story:catalog-basic-auth-profile, so it can run alongside that story while both may edit docs/local-catalog-provider.md; basic-auth's edit is cited, Slack's guide target is unnamed (inferred)
- file: .engineering/planning/story/catalog-basic-auth-profile.md
  line: 22
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the body names no code surface; the auth profile is in adapters/catalog/src/local.rs (AuthConfig, bearer) and the fixture harness in adapters/catalog/tests/local_runtime.rs, which provider fixture tests may also touch, unsaid (inferred)
- file: .engineering/planning/story/catalog-jira-cloud-reads.md
  line: 14
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the provider slot in adapters/<provider>/upstream/ is unfilled and adapters/atlassian/ exists, so this and story:catalog-confluence-reads may share one directory and bundle name; fix the provider id and path (inferred)
- file: .engineering/planning/story/catalog-confluence-reads.md
  line: 14
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: same unfilled provider slot and adapters/atlassian/ neighbour as story:catalog-jira-cloud-reads (inferred)
- file: .engineering/planning/story/catalog-zendesk-reads.md
  line: 26
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the fixture-test acceptance does not say where tests land, and adapters/catalog/tests/bundle_drift.rs and shipped.rs are hard-coded to gitlab, so each provider story is likely to edit the same two files unless each gets its own test file (inferred)
```
