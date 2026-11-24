---
format: aep.planning-md/3
id: review-result:mysql-loki-acceptance-round-2
kind: review-result
status: active
title: 'mysql/loki story: acceptance critic, round 2'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: approve

Acceptance critic, round 2, story:released-install-reads-mysql-and-loki-per-environment. The
11 round 1 findings are answered by the present text; two gaps of acceptances 5 and 8 remain
(findings 1 and 4).

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 228
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptances 5 and 8 need printed values (the MySQL error number, the page sequence); `cargo test … -- --ignored` captures the output of passing tests. Add `--nocapture` to the command."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 230
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "No assertion observes `super_read_only`: the session already runs SET SESSION TRANSACTION READ ONLY (adapters/sql/src/mysql.rs:29), so the function write is refused with 1792 (already `forbidden`, mysql.rs:640) on a writable server too. Add to the first test that `SELECT @@global.super_read_only` answers 1, and let acceptance 4 say that a recorded 1792 shows the session's refusal and not the replica's."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 249
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The expected ids and the count 250 hold only when each LEFT JOIN returns at most one row per base id. Say so in the setup text."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 212
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'fails when one name is taken out of one list' has no test behind it: the new test takes its list text through a pure function with a unit test over a string missing one name, or the review records one manual removal. Name which."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 228
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "No acceptance line shows the ignored.rs entry: add that `cargo run --locked -p connectors-build -- ignored` lists the replica tests as Live with prerequisite CONNECTORS_MYSQL_REPLICA_PORT."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 267
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The query string reaches the fixture URL-encoded (local_runtime.rs:86): say 'the decoded `query` parameter equals the text, and `time` equals the input'. Acceptance 7's `<statement>` names no query: give the literal the test uses."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 271
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "State that the story closes on acceptances 1 to 10, so it can be closed without a consumer result."
```
