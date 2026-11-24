---
format: aep.planning-md/3
id: review-result:mysql-loki-parallel-safety-round-2
kind: review-result
status: active
title: 'mysql/loki story: parallel-safety critic, round 2'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: approve

Parallel-safety critic, round 2, story:released-install-reads-mysql-and-loki-per-environment.
Round 1: the tutorial collision is answered (section "Order and ownership", the tutorial's
`depends_on` edge); the unnamed test file is answered (`mysql_replica.rs` and `ignored.rs` in
`scope`); the fold story is answered by the order decision; the CHANGELOG and README sharing is
partly answered.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 202
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'No other active story lists a file of this story's scope' is contradicted: story:entity-runtime-0331-pin (active) lists CHANGELOG.md and docs/development.md; story:local-invoke-admits-a-keyed-write-under-a-standing-grant (active) lists README.md, CHANGELOG.md and crates/connectors-build/src/ignored.rs. Name them and say the hunks are disjoint and the story merging second rebases."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 186
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Draft stories on the same files are not named: story:instances-resolve-through-locked-index adds a step to .github/workflows/release-binaries.yml; story:definitions-pin-covers-catalog lists docs/development.md and the workflows; story:mounted-custody-kind-secret-volume and story:main-failing-ci-journeys touch ignored.rs. Add one bullet naming them."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 196
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'This story lands first' binds nobody: say that story:test-binary-per-package-rest takes a depends_on edge to this story when it gets a scope that names adapters/sql or adapters/loki."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 192
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "The tutorial's text still adds connectors-sql to `BINARIES`; that edit belongs to the tutorial's own revision."
```
