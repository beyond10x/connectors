---
format: aep.planning-md/3
id: review-result:adversary-google-calendar-writes-pass-2
kind: review-result
status: active
title: Adversary pass 2 on Google Calendar writes
relations:
- reviews: story:catalog-google-calendar-writes
revision: 1
---
```
unit: story:catalog-google-calendar-writes, impl/catalog-google-calendar-writes at e3899a423 (correction 3d5f92417...e3899a423) plus one untracked adversary test file
verdict: CONFIRMED
cases: executed 173→176, red 3
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (listed in part 6)
needs-coordinator: none
```

The correction closes F1, F2 and F4 as far as the code goes. Three statements the correction added to the guide, and one it kept, are false against the code or against the pinned Discovery. All four are documentation-sized; none needs an engine change.

**1. What I touched.** `git status --short` shows only `?? adapters/catalog/tests/google_calendar_writes_adversary_pass2.rs`, a test file. No implementation file touched.

**2. Cases added** (all red now, each run alone with `-- --exact`):

| Case | What it asserts | Red output |
|---|---|---|
| `send_updates_outside_its_three_values_is_refused_before_any_request` | `""`, `"NONE"` and `0` for `sendUpdates` are refused before any request | `:219` `events.insert: sendUpdates="" Ok("applied"), sent query Some([("sendUpdates", "")])`; 9 of 9 sent across insert/patch/delete |
| `a_write_google_answers_429_is_rate_limited_as_the_guide_says` | a write answered 429 is `rate_limited`, as the guide's Limits line says | `:261` `events.insert: 429 classified Ok("unknown UpstreamProtocol")`, same for patch and delete |
| `the_truncated_guest_list_warning_covers_the_answers_of_insert_and_patch` | the guide warns that insert/patch sent with `maxAttendees` answer a truncated guest list | `:311` `neither the truncated-guest-list warning nor the answers bullet says that an insert or patch sent with maxAttendees answers a truncated guest list` |

**3. Suite run.** `cargo test -p connectors-catalog-provider --no-fail-fast`, EXIT=101; only `google_calendar_writes_adversary_pass2` failed (0 passed; 3 failed); `google_calendar` 27, `google_calendar_writes_adversary` 5, `local_runtime` 30 (6 ignored). 176 executed; 173 without the new cases. fmt and clippy clean.

**4. Findings** (e3899a423, all introduced):
- **G1** `docs/catalog-google-calendar.md:154`: requiring `sendUpdates` enforces presence only; it is typed string-or-integer (`lib.rs:804`) and `Parameter` records no enum (`inventory.rs:139-154`), so any value is sent. Fix: the guide sentence.
- **G2** `docs/catalog-google-calendar.md:342`: a write answered 429 is `unknown` (`lib.rs:638`), matching `docs/local-catalog-provider.md:366-368`; the Limits line was written for reads. Fix: limit it to reads and preflights.
- **G3** `docs/catalog-google-calendar.md:202`: the guest-list warning names only get and list; insert and patch take `maxAttendees` with the same truncation, and `:222` tells callers to pin the next etag from that answer.
- **G4** (judgement) `docs/catalog-google-calendar.md:205` and `operations.json:20`: they say a patch from a truncated read removes every other guest unconditionally; the pinned `attendeesOmitted` description says an update carrying it can only update the participant's response. Google's behaviour not verified.

**5. Attacked and could not break.** Guard order (`/status` before `/etag`, both before any write, `lib.rs:586-605`); a cancelled event without `etag` refused `forbidden`, one without `status` refused `upstream_protocol`; insert into a read-only calendar refused `forbidden`; `conferenceDataVersion`, `supportsAttachments`, scope claims hold against the pinned text; recurrence and conference creation are one approval-bound POST each.

**6. Paths written outside the worktree.** `~/.cache/w0930wc/adv2/` (mode 700), `~/.cache/w0930wc/adv2/suite.log`. Lease `wave0930c-calendar-writes-adv2` released.

**7. Findings block.**

```findings
- file: docs/catalog-google-calendar.md
  line: 154
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The guide says an approved input always says whom Google emails, but sendUpdates is typed any string or integer with no enum, so '', 'NONE' and 0 pass the new requirement and are sent verbatim on insert, patch and delete."
- file: docs/catalog-google-calendar.md
  line: 342
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The Limits line says a 429 is returned as rate_limited, but on a page that now documents writes a write answered 429 is classified unknown UpstreamProtocol (lib.rs:638), as the provider guide says."
- file: docs/catalog-google-calendar.md
  line: 202
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The truncated-guest-list warning names only events.get and events.list, while the pinned document gives insert and patch the same maxAttendees truncation and line 222 tells callers to pin the next etag from that answer."
- file: docs/catalog-google-calendar.md
  line: 205
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The guide and patch description say a patch from a truncated read removes every other guest unconditionally, but the pinned attendeesOmitted description says an update carrying it can only update the participant's response."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
