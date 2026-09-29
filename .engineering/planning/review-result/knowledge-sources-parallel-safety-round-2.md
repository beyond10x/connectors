---
format: aep.planning-md/3
id: review-result:knowledge-sources-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2
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
epic:catalog-knowledge-sources — `bundle::load` refuses a provider whose index row is absent or whose bundle digest differs, so no provider unit's own drift, invoke or paging test can pass while "a unit never merges `index.json`" and "the integration regenerates it"; `generated/bundles/<provider>.bundle.json` is also in no story's "Own files only" list. Remedies, not chosen: state the index row as one shared file with an ordering edge between the four stories, or split it (one index directory per provider). (cited) — .engineering/planning/epic/catalog-knowledge-sources.md:39
epic:catalog-knowledge-sources — the existing `adapters/catalog/tests/bundle_drift.rs` asserts `fresh.entries == indexed.entries` against a temp directory holding only gitlab, so it fails once the regenerated index carries a second provider, and no story or the epic owns that file; the one that lands first collides with the rest. Remedies, not chosen: name an owner and an ordering edge, or split that assertion out of the shared file. (cited: bundle_drift.rs:27-29, `bundle::read_index` rows) — .engineering/planning/epic/catalog-knowledge-sources.md:39
story:catalog-jira-cloud-reads — this and story:catalog-confluence-reads both pin sources under `adapters/atlassian/upstream/`, which does not exist yet, so both create it, and neither names its file names (Confluence pins two documents); the epic's "share no authored file" is unproven for that directory. Remedies, not chosen: fix distinct file names in each body, or give each a directory. (cited, both bodies; the directory is not yet there) — .engineering/planning/story/catalog-jira-cloud-reads.md:14

Round-1 findings resolved: the guide, test file and provider-id slots are now fixed per story, and the shared index is named. Slack has no edge on basic-auth, and that is safe now: its files are its own and nothing it touches overlaps `local.rs` or the harness.

Surfaces: 6 cited, 0 inferred, 0 unplaceable. Out of my lane: whether `catalog-basic-auth-profile` naming the harness as a surface is a design matter, and whether "byte-identical body" is checkable (acceptance).

```findings
[
  {"file": ".engineering/planning/epic/catalog-knowledge-sources.md", "line": 39, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "bundle::load refuses a provider with no index row or a digest mismatch (crates/connectors-catalog/src/bundle.rs:299-318), so no provider unit's drift/invoke/paging test can pass while units never merge index.json and the integration regenerates it; generated/bundles/<provider>.bundle.json is also in no story's own-files list. Remedies: an ordering edge recording index.json as the reason, or splitting the index so units share no file. Cited."},
  {"file": ".engineering/planning/epic/catalog-knowledge-sources.md", "line": 39, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "adapters/catalog/tests/bundle_drift.rs:27-29 asserts fresh.entries equals indexed.entries with a one-provider temp index, so it fails once the regenerated index holds a second provider; no story or the epic owns that file, and the first provider to land collides with the rest. Remedies: name an owner plus an ordering edge, or split the assertion out of the shared file. Cited."},
  {"file": ".engineering/planning/story/catalog-jira-cloud-reads.md", "line": 14, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "this and story:catalog-confluence-reads both pin sources under adapters/atlassian/upstream/, which does not exist yet, so both create it, and neither body names its file names (Confluence pins two documents). Remedies: fix distinct file names in each body, or give each provider its own directory. Cited in both bodies; the directory is not there yet."}
]
```
