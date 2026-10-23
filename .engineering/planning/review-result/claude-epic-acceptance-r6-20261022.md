---
format: aep.planning-md/3
id: review-result:claude-epic-acceptance-r6-20261022
kind: review-result
status: active
title: 'Plan-critic acceptance, round 6: connectors with Claude'
summary: needs-revision, 1 blocker 1 warning; addressed
relations:
- reviews: story:docs-first-run-path
- reviews: story:standing-write-approval-for-unattended-agents
revision: 1
---
# Plan-critic, acceptance lens, round 6: connectors with Claude

**Verdict: needs-revision, 1 blocker and 1 warning. Both are addressed in the revisions after round 6.**

| Story | Severity | Finding | Disposition |
|---|---|---|---|
| docs-first-run-path | blocker | round 5 sized the whole table (14,992 B) as the operations cells; per column: Connector 1,790 B, Implemented operations 3,699 B, Binding 9,307 B (`README.md:286-304`); the kept README is about 25.7 KB, so 20,000 B is unreachable | cap `wc -c README.md` at most 27,000 with the measured parts listed; Binding cells stay, shortening them is later work |
| standing-write | warning | scenarios placed in a domain file; the gate runs only authored traces from `AUTHORED_DIRECTORIES` (`gate.rs:318-335`) and prints one combined line (`gate.rs:449-452`) | four traces in `contracts/operations/v1alpha1/scenarios/standing-approval-{scope-match,expiry,revocation,rate}.yaml`; N and P each at least 4 higher at head; none in `AUTHORED_KNOWN_FAILING` |
| docs-first-run-path | note | `local-runtime.md:87` starts mid-sentence in the `key-*` list | the cut starts at "Issuance needs a bounded clock" and keeps the `key-*` sentence |
