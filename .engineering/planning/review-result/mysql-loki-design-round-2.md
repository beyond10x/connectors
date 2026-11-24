---
format: aep.planning-md/3
id: review-result:mysql-loki-design-round-2
kind: review-result
status: active
title: 'mysql/loki story: design critic, round 2'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: approve

Design critic, round 2, story:released-install-reads-mysql-and-loki-per-environment. The five
round 1 findings are answered by the present text; the cited code agrees with the story; three
edges walked, no cycle.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 165
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Design point 5 and acceptance 5 cannot show what a replica itself answers to a write: every session runs `SET SESSION TRANSACTION READ ONLY` (adapters/sql/src/mysql.rs:27-29), which refuses a write with 1792, already `forbidden` (mysql.rs:640). Say that the session guard may answer first and record which number answered; name the answer in acceptance 4 as the connector's on a server in `super_read_only`; or add a control run on the server before `SET GLOBAL super_read_only = ON` and record both numbers."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 149
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The new list test is vacuous for `connectors` when it matches substrings: every other name contains it. The test matches whole names, and its negative case removes `connectors` as well as `connectors-sql`."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 144
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Design point 1's decision (a failing release target is fixed in this story's pull request, not excluded) can work: the workflow header (:18-20) says a pull request that changes the file runs the four builds and the image build."
```
