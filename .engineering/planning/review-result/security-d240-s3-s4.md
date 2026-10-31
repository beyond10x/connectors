---
format: aep.planning-md/3
id: review-result:security-d240-s3-s4
kind: review-result
status: active
title: Security review of story:catalog-operation-declares-its-walk and story:invoke-walks-declared-pages (wave D240)
tags:
- d240
relations:
- reviews: story:invoke-walks-declared-pages
- reviews: story:catalog-operation-declares-its-walk
revision: 1
---
## Security review, units S3 and S4 (wave D240), as returned

unit: S3+S4 of wave D240 (story:catalog-operation-declares-its-walk, story:invoke-walks-declared-pages): implementation at a75706179, cases committed as 8918769e5 on impl/invoke-walks-declared-pages
verdict: CONFIRMED
cases: executed 25→29, red 3
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 6 paths kept, 4 written and deleted, all under s4/sec/
needs-coordinator: none

Cases in `apps/connectors/tests/security_invoke_walks.rs` (8918769e5), red:
- `a_file_walk_holds_no_more_memory_after_ten_thousand_pages_than_after_one_hundred`: a 10,000-page
  walk held 41480798 bytes at most, a 100-page walk 426314.
- `a_walk_resumed_at_a_position_never_sends_that_position_twice`: `c7` sent twice.
- `jailed::a_walk_refuses_the_duplicate_key_document_a_plain_invoke_refuses`: a plain invoke refuses
  `{"ticket_id":101,"ticket_id":102}` as `cli_dynamic_input`; the walk read 2 pages with ticket_id 102.

Suite: invoke_walks 12 ok; security_invoke_walks 1 passed 3 failed (exit 101); walk_declarations 10
and walk_adversary 3 ok.

Cleared: walk flags never reach owner, audit or provider; one admitted read per page and never a
write; a provider position cannot change host, path or other parameters; withheld, fixed and
credential parameters cannot be walked; `--out` created new, 0600, no symlink; page and run bounds;
`resume` and `error` carry no credential.

```findings
- file: adapters/catalog/src/walk.rs
  line: 338
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the walker keeps every position's full text for the whole walk and a plain position has no length cap, so a 10,000-page walk held 41 MB for 2 KiB positions and a provider can push it to about 10 GiB
- file: apps/connectors/src/walk.rs
  line: 161
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the position a walk starts from (resume input) is not recorded as sent, so a provider repeating it gets the same request twice and that page's records are written twice
- file: apps/connectors/src/local/invoke.rs
  line: 791
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the walk parses the caller's document with serde_json (last duplicate key wins), so a document the plain invoke refuses as cli_dynamic_input is walked and sent to the provider
- file: apps/connectors/src/local/invoke.rs
  line: 639
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a write failure partway through a page can leave a torn record line in the --out file while the summary counts none of that page; not demonstrated because a write failure could not be built in the jail
```
