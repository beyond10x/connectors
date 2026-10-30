---
format: aep.planning-md/3
id: review-result:adversary-d240-s4-pass-1
kind: review-result
status: active
title: Adversary pass 1 on story:invoke-walks-declared-pages (wave D240)
tags:
- d240
relations:
- reviews: story:invoke-walks-declared-pages
revision: 1
---
## Adversary pass 1, unit S4 (wave D240), as returned

unit: story:invoke-walks-declared-pages (S4, wave D240), tree conn-d240-s4 at 81dc84cce (adversary commit on top of integration head a75706179)
verdict: NEEDS-CHANGE
cases: executed 137→141, red 3
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 9 paths, all under the assigned scratch s4/adv/
needs-coordinator: none

Cases in `apps/connectors/tests/invoke_walks_adversary.rs` (81dc84cce), all red:
- `jailed::a_page_that_cannot_be_written_whole_leaves_no_part_of_it`: under RLIMIT_FSIZE the summary
  says `pages: 1, records: 2, stopped: write`, but the file holds 2600 bytes in 3 lines, the last a
  torn record of page 2. Cause: the sink in `invoke.rs` (`Walking::run`, `write_all`) fails partway.
- `a_resumed_walk_whose_first_page_names_its_own_cursor_stops_as_repeated`: a resumed walk whose
  first page answers its own cursor reads and writes the page twice. Cause: `walk.rs:161` starts the
  walker with no sent positions.
- `missing_names_only_what_the_written_records_lacked`: `missing` is absent although no written
  record has `extra`. Cause: `walk.rs:196-212` updates `missing` before the sink accepts the page.

Suite: lib 12, bins 51, cli_surface_minor 11, explicit_commands_help_completion 5, invoke_shapes 13,
invoke_short 21, invoke_short_adversary 6, invoke_walks 12, json_answers 2, vocabulary_text 4 ok;
invoke_walks_adversary 1 passed 3 failed; SUITE_EXIT=101.

Not broken: `--output human`, negative `--max-pages`, unreadable input leaves no `--out` file,
symlink/dangling symlink/directory/missing directory/trailing slash `--out` refused before any call,
0600 under umask 0, default 20 pages with resume at page 21, 404 on page 2, records not a list,
flag order, `--walk false`.

```findings
- file: apps/connectors/src/local/invoke.rs
  line: 735
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The NDJSON sink is not all-or-nothing per page, so a write that fails partway leaves a torn record line in the --out file that the summary does not count (jailed::a_page_that_cannot_be_written_whole_leaves_no_part_of_it).
- file: apps/connectors/src/walk.rs
  line: 161
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The cursor a resumed walk sends on its first page is never recorded as sent, so a provider that answers it again gets the same page read and written twice before repeated_position (a_resumed_walk_whose_first_page_names_its_own_cursor_stops_as_repeated).
- file: apps/connectors/src/walk.rs
  line: 196
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: missing is computed over the records of a page the sink refused, so a path only those unwritten records had is dropped from missing (missing_names_only_what_the_written_records_lacked).
```
