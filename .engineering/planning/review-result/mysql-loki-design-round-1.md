---
format: aep.planning-md/3
id: review-result:mysql-loki-design-round-1
kind: review-result
status: active
title: 'mysql/loki story: design critic, round 1'
relations:
- reviews: story:released-install-reads-mysql-and-loki-per-environment
revision: 1
---
verdict: needs-revision

Design critic, round 1, story:released-install-reads-mysql-and-loki-per-environment at revision 20.

```findings
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 121
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Two stories own the same change and no edge records it: story:claude-getting-started-tutorial (draft) plans the same `connectors-sql` entry in `BINARIES` and never mentions `--version`, which the workflow's version loop (:137-139) needs. Say that this story alone owns `connectors-sql` in `BINARIES`, its `--version` and the seven-name test, and add `story:claude-getting-started-tutorial depends_on` this story."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 138
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The server behind CONNECTORS_MYSQL_ADVERSARY_PORT is writable and one of its tests writes to it (ignored.rs:432-441, mysql_adversary_2.rs:1-4); `super_read_only` is global server state. The new tests read their own variable with their own classification entry, and the order is: create the four tables, then `SET GLOBAL super_read_only=ON`."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 165
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The guide is to state the refusal a write gets on a replica, but no test shows it. A write statement is refused `unsupported` before it runs (mysql_protocol.rs:251-266); a server-refused write is `forbidden` only for 1044, 1142, 1143, 1227, 1370 and 1792 (mysql.rs:640); a read-only server refuses with 1290 or 1836. Add a write through a function against the `super_read_only` server, record the code and the answer, and put the mapping fix in this story when the answer is not `forbidden`."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 180
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 8 records 'in that order or not'; the design has no branch for 'not', and the consumer's paging depends on it. Say what the story does then."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 62
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'there is no outer ORDER BY' holds only without a keyset: with one the wrapper appends `WHERE … ORDER BY <column>` (mysql.rs:517-538)."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 133
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The cited seams hold: the fixture logs statements (mysql_protocol.rs:625-645), each of the two wire statements needs its own `start(Script)`, the Loki fixture routes GET /loki/api/v1/query (local_runtime.rs:92), the image stage copies every executable (workflow :318-326)."
- file: .engineering/planning/story/released-install-reads-mysql-and-loki-per-environment.md
  line: 190
  category: design
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Acceptance 11 is the consumer's and the story is implemented without it; no hidden dependency inside the set, no cycle."
```
