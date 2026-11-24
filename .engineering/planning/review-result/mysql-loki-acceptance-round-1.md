---
format: aep.planning-md/3
id: review-result:mysql-loki-acceptance-round-1
kind: review-result
status: active
title: 'mysql/loki story: acceptance critic, round 1'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: needs-revision

Acceptance critic, round 1, story:released-install-reads-mysql-and-loki-per-environment at revision 20.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 190
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 11 asks for `result.result.status`; the mysql and loki answers have no such field (loki: `result_type` and `series`, adapters/loki/tests/local_runtime.rs:430-441; mysql: `rows` and `truncated`, adapters/sql/tests/mysql_protocol.rs:187). Name exit status, `.result.result.truncated` and the row count for `query.read`, and `.result.result.result_type` for `logs.query_metric`."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 165
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 4 has the guide name the refusal a write statement gets on a replica; no acceptance line produces that refusal. Add to acceptance 5: on the `super_read_only` server run a write statement through `query.read` and record the MySQL error number and the code connectors-sql answers. The guide says the replica was tested as MySQL 8 with `super_read_only`, not as Aurora."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 167
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'The three prose lists name seven programs' has no check. Name it: a grep, or an assertion in release_binaries.rs that each list holds every `BINARIES` name."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 155
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`connectors-build release-binaries` exits 0 before and after the change; the unit test at release_binaries.rs:405-419 is the proof. Name its run command and say the other is a no-regression check."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 153
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 1 names no assertion: the test runs `env!(\"CARGO_BIN_EXE_connectors-sql\")` with `--version` and asserts exit 0 and stdout `connectors-sql <CARGO_PKG_VERSION>`."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 159
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 3 names no command and no value: `tar -tzf` lists `connectors-sql` per target, `--version` equals the tag, the check `artifact:<alias>` has state `ready`. Mark it post-release, not a gate of the pull request."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 169
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptances 5 and 8 name no test names, no command, no data size and no pass condition: at least 201 rows with a unique `ORDER BY id`, the expected `COUNT(*)`, the classification in ignored.rs, the port of the `super_read_only` server."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 182
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 8 'in that order or not' passes whatever the server does. The test asserts the page's row set equals rows 101 to 200; the order is recorded, not asserted."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 175
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Acceptance 7's texts match adapters/sql/src/mysql.rs:536-543. Name the alias list (`c0,…`) and assert the last logged statement, not the list that starts with the two `SET` statements."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 188
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 10 names no assertion: route `{PROXY}loki/api/v1/query`, GET, `Bearer` token, `result_type` `vector`, and the recorded query string (the fixture answers without checking the query text, local_runtime.rs:93)."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 172
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptances 6 and 9 check prose: name a fixed phrase to grep or a reading at review."
```
