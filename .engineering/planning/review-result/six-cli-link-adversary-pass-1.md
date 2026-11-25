---
format: aep.planning-md/3
id: review-result:six-cli-link-adversary-pass-1
kind: review-result
status: active
title: 'Adversary read of the six-cli link change: three red cases, none on a path a caller reaches'
relations:
- reviews: story:six-cli-tests-take-jail-binaries-built-once-per-run
revision: 1
---
Verdict: needs-change, for the first commit of the change of
story:six-cli-tests-take-jail-binaries-built-once-per-run. No finding is on a path a caller
reaches today; the two that name a change are fixed in the second commit of the same pull request.

Adversary read of the committed change (`Jail::install` links in place of copying). The adversary
added three cases to `crates/connectors-custody/src/mounted/tests.rs`, uncommitted, and ran them
with the 14 committed tests of `mounted::tests::` on the build host of the shared pool, with the
temporary directory under the checkout as the gate sets it: 17 tests run, 14 passed, 3 failed. It
edited no product file.

## Findings

1. Warning, introduced. `crates/connectors-custody/src/mounted/fixture.rs`, `Jail::install`. The
   link rule read only the group and other write bits. A source with no execute bit was linked
   and could not be run in the jail (mode 644, permission denied), and a source with a set-id bit
   kept it (mode 4755); the copy before the story always had mode 0700. The product refuses an
   adapter file with no execute bit (`crates/connectors-host/src/local/config.rs`, about line
   2432). No caller installs such a source: every one installs a binary cargo built or the running
   test binary.
   Fixed: a source is linked only when it has the owner execute bit, no group or other write bit
   and none of the set-user-id, set-group-id and sticky bits. Every other source is copied with
   mode 0700. The adversary's two cases are tests of the fixture now.
2. Note, introduced. Same function. Where the installed file is a link, another jail's install
   and teardown of the same source move its link count and change time (links 2, 3, 2 in the
   adversary's case). The product keys a derivation cache on the change time (`file_identity`,
   `crates/connectors-host/src/local/config.rs`, about lines 1655 to 1673).
   `apps/connectors/tests/setup_migrate_short.rs` (about line 280) installs its test binary as
   the provider; no second jail links that binary today. If reached, the owner would derive
   again per request (inferred, not run). Not changed: a second name of one inode moves both by
   definition. The doc comment of `Jail::install` and the story's "As built" section state it.
   The adversary's third case is not kept, because it cannot hold where a link is made.
3. Note, introduced. The story's "As built" section named a test
   `install_copies_a_source_that_is_a_symbolic_link`; the test is
   `install_copies_what_a_symbolic_link_names`. Fixed in the section.
4. Note, introduced. `install_copies_a_source_on_another_file_system`: on a host whose cargo
   binaries carry a group write bit (umask 002) the install copies because of that bit, and the
   refused link is not what the test shows. The forge's gate job and the build host both have no
   such bit (read from the test's own output). The test asserts `EXDEV` from the link call itself
   before it installs. Not changed.

## Attacked and held

- The 46 test files that use `Jail` or `Container` do not write, truncate, rename, change the
  mode of, re-time or delete an installed path (grep and read).
- The adapter executable check: a linked file of mode 0755 of the test's user passes as the copy
  of mode 0700 did, and the check reads no link count (read).
- The link-count checks of the product cover private files and custody items, never an installed
  binary (read).
- The product's change of mode to 0500 lands on the sealed memory file, not on the source; the
  owner launch only reads the CLI (read).
- Removing a jail unlinks names only. A destination that is a symbolic link to an outside file
  left that file's bytes and mode alone in both arms (run, green, not kept).
- A directory as source or destination panics as before; `..` in the inside path leaves the jail
  as before. No caller does either.
- A source owned by another user: not exercised. Inferred: with `protected_hardlinks` set to 1 the
  link is refused and the copy runs.
- Mutants argued from code, not run: always-copy, a change of mode after the link, and dropping
  the 0700 each turn a committed test red.
- The forge, inferred from the workflow files: the "repository gate" job has `target/` and its
  temporary directory under the checkout and takes the link arm; the journeys job takes the copy
  arm, as before; the macOS and Windows jobs do not compile the fixture.

```findings
- file: crates/connectors-custody/src/mounted/fixture.rs
  line: 139
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the link rule reads only the group and other write bits, so a source without an execute bit is linked non-runnable and a set-id source keeps its bit, where the copy was always 0700; no caller installs such a source
- file: crates/connectors-custody/src/mounted/fixture.rs
  line: 140
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: another jail's install and teardown move the link count and change time of this jail's file, which file_identity in connectors-host config.rs keys a cache on; no second jail links a derived provider today
- file: .engineering/planning/story/six-cli-tests-take-jail-binaries-built-once-per-run.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: As built item 1 names install_copies_a_source_that_is_a_symbolic_link, the test is install_copies_what_a_symbolic_link_names
- file: crates/connectors-custody/src/mounted/tests.rs
  line: 712
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: on a host whose cargo binaries carry a group write bit the other-file-system test passes through the write-bit rule and does not test the refused-link fallback
```
