---
format: aep.planning-md/3
id: review-result:grafana-catalog-security-r2-20261111
kind: review-result
status: active
title: Security review pass 2 on the fixes to the Grafana catalog and proxy reads
relations:
- reviews: story:catalog-grafana-reads
- reviews: story:catalog-grafana-datasource-proxy-reads
revision: 1
---
# Security review pass 2: fixes to pass 1 on the Grafana catalog and proxy reads

Reviewed the unit tree `grafana-reads` with story 1 staged and story 2 plus the pass-1 fixes
unstaged. Cases in `adapters/catalog/tests/grafana_security_review_r2.rs`: 5 cases, 4 green, 1 red.

Held: every proxy read refuses a uid outside `^[A-Za-z0-9_-]{1,40}$` before any request, across
line breaks, Unicode look-alikes, encoded separators and non-string values; the engine refuses a
bundle with a malformed, empty or oversized pattern; the definition route carries the pattern into
its engine; a repeated `catalog.allowed_values` key is refused at start in every YAML spelling tried;
no other provider's bundle changed; the uid rule matches Grafana v13.3.1
`pkg/util/shortid_generator.go`. The adversary case changed by the implementor now requires zero
requests for every tricky uid, which is stronger than before.

Not held: the anchoring check accepts patterns that are not anchored over the whole value
(`r2_a_pattern_accepted_as_anchored_holds_the_whole_value`). Note: on the served route the pattern
runs after the credential read; nothing is sent.

```findings
[
{"file":"crates/connectors-catalog/src/inventory.rs","line":192,"category":"contract-drift","severity":"warning","verdict":"INFEASIBLE","origin":"introduced","message":"pattern_is_anchored accepts `^a|x$`, `^(?m)…$` and `^…(?x)#$`, so a pattern recorded as anchored need not hold the whole value, and the engine then sends a uid holding `/`; only a repo-authored amendment or bundle reaches it."},
{"file":"adapters/catalog/src/served.rs","line":833,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"on the served route the uid pattern runs in the engine after the mounted credential is read, so a uid outside the rule is refused after the credential read (and answered `unavailable` when none is mounted), though nothing is sent."}
]
```
