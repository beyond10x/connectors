---
format: aep.planning-md/3
id: review-result:mysql-loki-parallel-safety-round-1
kind: review-result
status: active
title: 'mysql/loki story: parallel-safety critic, round 1'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: needs-revision

Parallel-safety critic, round 1, story:released-install-reads-mysql-and-loki-per-environment at
revision 20. 10 scope paths cited, 6 inferred, 1 test file unnamed. No `depends_on` edge is
missing; the story names none of the stories it shares files with.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 121
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Unnamed collision with story:claude-getting-started-tutorial (draft): both edit the `BINARIES` line of .github/workflows/release-binaries.yml and both add `connectors-sql`; both land on README.md, adapters/loki/README.md, docs/local-mysql-cli.md, docs/development.md, website/docs/connectors/loki.md, website/docs/connectors/mysql.md and website/docs/getting-started.md. Add a section 'Order and ownership' naming the pair and the shared files, with an ordering edge or a split of the surface. The tutorial's text is stale: it says the archive carries two binaries and never lists crates/connectors-build/src/release_binaries.rs."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 15
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "CHANGELOG.md is shared with story:standing-grant-binds-a-pattern-or-a-value-set (active, being implemented) and other stories; README.md with story:local-invoke-admits-a-keyed-write-under-a-standing-grant. Say that this story adds its own entries under the next unreleased heading and name the release it lands in."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "story:test-binary-per-package-rest (draft, no scope) plans to fold the integration tests of adapters/sql and adapters/loki into one `tests/main.rs` per package; it would move mysql_local.rs, mysql_protocol.rs and the loki local_runtime.rs, and the names ignored.rs keys on (ignored.rs:439-441). Name the pair and the order."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 138
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Design point 4 and acceptances 5 and 8 name no file for the ignored real-server tests. Name it, add it to `scope`, and make crates/connectors-build/src/ignored.rs a cited entry (the arm at :432-441)."
```
