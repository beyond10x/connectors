---
format: aep.planning-md/3
id: review-result:adversary-google-calendar-gmail-reads-pass-2
kind: review-result
status: active
title: Adversary pass 2 on Google Calendar and Gmail reads
relations:
- reviews: story:catalog-google-calendar-reads
- reviews: story:catalog-google-gmail-reads
revision: 1
---
```
unit: story:catalog-google-calendar-reads + story:catalog-google-gmail-reads, impl/catalog-google-calendar-gmail-reads at a01bab892 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 186→191, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: none
```

Five new cases are red, all introduced. The 20 `dailyLimitExceeded` selections, the resync order and the `fields` warnings all held. What broke: the new attachment route, the new Calendar re-check, and the new `fields` examples.

## 1. Diff stat
`git status --short`: `?? adapters/catalog/tests/google_calendar_gmail_adversary_pass2.rs`. No implementation file touched; rustfmt and clippy pass.

## 2. Cases added (all red now)
First run: `cargo test --locked -p connectors-catalog-provider --test google_calendar_gmail_adversary_pass2` gave `0 passed; 5 failed`.

| line | case | red output (verbatim, trimmed) |
|---|---|---|
| 115 | `an_attachment_under_the_named_limit_is_refused_and_the_texts_state_the_real_ceiling` | `a part of 3670016 bytes (`size`, under the 4 MiB named) is refused as capacity because its base64url `data` is 4/3 larger; these texts bound the read only by 4 MiB …` |
| 185 | `the_calendar_reset_recheck_does_not_misjudge_primary_or_hidden_calendars` | `the re-check looks the calendarId up in calendarList.list, which never lists `primary` by that id and omits hidden calendars by default …` |
| 240 | `the_sync_fields_examples_keep_the_deletion_markers` | `"events.list description: items(id,summary,start,end) without status", "calendarList.list description: items(id,summary) without deleted"`, and the guide twice |
| 303 | `the_sync_token_descriptions_name_the_show_deleted_restriction` | `["events.list: showDeleted", "calendarList.list: showDeleted", "calendarList.list: showHidden"]` |
| 340 | `the_first_gmail_baseline_is_ordered_before_the_first_walk` | `neither the first-baseline paragraph nor the users.getProfile description orders the baseline before the first full walk` |

## 3. Suite run
`cargo test --locked -p connectors-catalog-provider --no-fail-fast`, exit 101; every target ok except the pass-2 file (0 passed; 5 failed). 191 executed, 6 ignored; 186 without the new file.

## 4. Findings
1. An attachment under the stated 4 MiB cannot be read: `MessagePartBody.size` is pre-encoding and `data` is base64url, 4/3 larger, so the real ceiling on `size` is about 3 MiB (`providers/google-gmail/operations.json:16,13`, `docs/catalog-google-gmail.md:96`). NEEDS-CHANGE, warning.
2. The Calendar re-check wrongly declares `primary` and hidden calendars gone (`docs/catalog-google-calendar.md:133`, `providers/google-calendar/operations.json:10`). Fix: re-walk without `syncToken`; `not_found` there means gone. NEEDS-CHANGE, warning.
3. The `fields` examples for sync walks drop `status` / `deleted`/`hidden`, so a narrowed delta returns deleted entries as bare ids a mirror upserts. NEEDS-CHANGE, warning.
4. The `events.list` and `calendarList.list` descriptions still omit the `showDeleted`/`showHidden` restriction beside `syncToken`. CONFIRMED, note.
5. The first Gmail sync does not order the baseline before the first walk (`docs/catalog-google-gmail.md:122`, `users.getProfile` description). CONFIRMED, note.

## 5. Attacked and could not break
All 20 Google selections carry all three quota reasons and all four guides name them; `dailyLimitExceeded` reaches `rate_limited`; attachments.get path binding, scopes, and `data`/`size` unchanged; the reset rule and history.list take the baseline first; all five paging descriptions carry the `fields` warning.

## 6. Paths written outside the worktree
`~/.cache/w0930cg/adv2/` (mode 700), `~/.cache/w0930cg/adv2/suite.log`. Lease `wave0930c-cg-adv2` released.

## 7. Findings block

```findings
- file: adapters/catalog/providers/google-gmail/operations.json
  line: 16
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A Gmail part whose size is under the 4 MiB the attachments.get and messages.get descriptions and the guide name is refused as capacity because its base64url data is 4/3 larger, so the real ceiling on size is about 3 MiB and no text says so."
- file: docs/catalog-google-calendar.md
  line: 133
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The reset re-check looks the calendarId up in calendarList.list, which never lists the primary keyword and omits hidden calendars by default, so an expired token on primary leads the guide to drop the mirror."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 10
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The fields examples for sync walks keep neither Event status nor CalendarListEntry deleted, so a delta narrowed by them returns deleted entries as bare ids that a mirror upserts."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 6
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The events.list and calendarList.list descriptions still omit that showDeleted and showHidden cannot be false beside syncToken, while calendarList.list says to repeat the same other parameters."
- file: docs/catalog-google-gmail.md
  line: 122
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The first-baseline paragraph and the users.getProfile description do not order the baseline before the first full walk, which leaves the first sync open to the race the reset rule fixed."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
