---
format: aep.planning-md/3
id: review-result:mysql-loki-scope-round-1
kind: review-result
status: active
title: 'mysql/loki story: scope critic, round 1'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: approve

Scope critic, round 1, story:released-install-reads-mysql-and-loki-per-environment at revision 20.
12 promises extracted from the Request and "The consumer's reads"; 11 traced to an acceptance
line, the Loki window units only partly (acceptance 10). No finding has severity error.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 188
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The consumer's window is in s, m, h or d and it sends a positive control; acceptance 10 tests `[1h]` with one selector. `query_metric` (adapters/loki/src/lib.rs:607-626) forwards the query string unchanged, so one window is enough: say so in acceptance 10."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 131
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Design 2 drafts an edit to website/docs/connectors/mysql.md:28-36 that no acceptance line claims. Add it to acceptance 4 as a fourth named file, or drop it."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 39
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`scope` lists website/docs/connectors/loki.md and CHANGELOG.md; neither the Design nor the Acceptance says what changes in them. Name the change or remove the entries."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 60
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A query is at most 8,192 characters; no acceptance claims that the consumer's largest shape with its paging text and the COUNT wrapper fits. Add the bound to the guide text of acceptance 9."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 171
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 5's open fix ('the fix is in this story') must stay inside the Boundary: add that the fix keeps every session READ ONLY and adds no write path."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 172
  category: scope
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Acceptance 6 adds the sentence that a Loki without any credential is not served; it documents a limit and adds no route. Keep it and say so."
```
