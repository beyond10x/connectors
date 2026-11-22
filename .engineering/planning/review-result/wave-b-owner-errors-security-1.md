---
format: aep.planning-md/3
id: review-result:wave-b-owner-errors-security-1
kind: review-result
status: active
title: Security review 1 on the owner error log
relations:
- reviews: story:owner-errors-are-readable
revision: 1
---
## Security review 1 on story:owner-errors-are-readable

Reviewed commit 971212e3ac on impl/owner-errors-are-readable (base d3bd6fcd4). Model: opus.

```
verdict: red
cases: executed 9→14, red 3
origin: introduced 5, pre-existing 0, undecided 0
```

New uncommitted test file `crates/connectors-host/tests/owner_log_security.rs`:

- `owner_error_is_never_a_code_another_connection_logged` (red): `last_code` returns the last
  `code=` written after the command's offset by any connection or owner process.
- `owner_error_is_not_an_older_code_when_the_newest_line_is_a_panic` (red): a panic line has no
  `code=`, so an older unrelated code is reported.
- `the_log_never_writes_through_a_second_link` (red): the open admits a file with more than one
  link; reachable only by the state directory's own user.
- `a_symlink_at_the_log_is_neither_written_nor_named` (green).
- `a_refusal_is_not_logged_into_a_wider_state_directory` (green).

Held: `record` takes only a data-free `Code`, fixed event names and compile-time panic locations,
so no secret can reach the log (credential test green); symlink, directory and FIFO at the log
path are refused; the CLI reads only after admitting the state directory private; `owner_log` is
not projected to MCP clients and the `--output json` stderr stays one envelope.

```findings
[
  {"file": "crates/connectors-host/src/local/owner/diagnostics.rs", "line": 163, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "owner_error is the last code= logged after the command's offset by any connection or owner process, so a concurrent command's or a refusing owner's code is reported as this command's (owner_log_security.rs:49 red)"},
  {"file": "crates/connectors-host/src/local/owner/diagnostics.rs", "line": 163, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "when the newest line after the offset is a panic, last_code skips it and reports an older unrelated code as owner_error (owner_log_security.rs:67 red)"},
  {"file": "crates/connectors-host/src/local/owner/diagnostics.rs", "line": 58, "category": "integrity", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "open admits a log with more than one link, so a hard link at owner.log makes the owner append to, chmod and empty another same-user file; only the directory's own user can create it (owner_log_security.rs:102 red)"},
  {"file": "crates/connectors-host/src/local/owner/diagnostics.rs", "line": 50, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "on Windows the log open follows reparse points and does not check link count or owner-only access of an existing file, against filesystem/windows.rs:1-6 and owner.md's access-list claim; reasoned from code only"},
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 1628, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the panic hook that writes owner.log is installed before the state directory is admitted private at :1634, and record checks no directory; reached only by a direct __connectors-owner run"}
]
```
