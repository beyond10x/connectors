---
format: aep.planning-md/3
id: review-result:adversary-google-calendar-gmail-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Google Calendar and Gmail reads
relations:
- reviews: story:catalog-google-calendar-reads
- reviews: story:catalog-google-gmail-reads
revision: 1
---
```
unit: story:catalog-google-calendar-reads (6c8fd8b31) + story:catalog-google-gmail-reads (307c82dd0), working tree of impl/catalog-google-calendar-gmail-reads at 307c82dd0 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 177→183, red 6
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: none
```

## 1. Diff stat

`git --no-pager diff --stat` is empty because the only change is untracked. `git status --short` shows one path, and it is a test file:
```
?? adapters/catalog/tests/google_calendar_gmail_adversary.rs
```
No implementation file was touched. `rustfmt --check` passes on the file, and `cargo clippy --locked -p connectors-catalog-provider --tests -- -D warnings` passes.

## 2. Cases added, all red now

All six are in `adapters/catalog/tests/google_calendar_gmail_adversary.rs`. The first run was `cargo test --locked -p connectors-catalog-provider --test google_calendar_gmail_adversary`, and it gave `0 passed; 6 failed`.

| line | case | red output (verbatim, trimmed) |
|---|---|---|
| 105 | `gmail_reset_rule_takes_the_baseline_before_the_full_walk` | `the baseline is taken after the full walk, so changes during the walk are lost: ["docs/catalog-google-gmail.md reset rule: …walk users.messages.list… and take a new baseline from users.getProfile.", "users.history.list description: …do a full sync and take a new baseline from users.getProfile"]` |
| 140 | `every_google_paging_description_warns_that_fields_must_keep_the_end_condition` | `["google-calendar calendarList.list", "google-calendar events.list", "google-gmail users.messages.list", "google-gmail users.threads.list", "google-gmail users.history.list"]` |
| 172 | `a_daily_quota_403_is_rate_limited_not_forbidden` | `[("users.messages.list", Forbidden), ("events.list", Forbidden)]` |
| 214 | `an_expired_sync_token_is_told_apart_from_an_unknown_calendar` | `left: (NotFound, "provider refused the request") right: (NotFound, "provider refused the request")` |
| 250 | `the_calendar_sync_token_section_names_the_show_deleted_restriction` | `` the Sync tokens section omits ["`showDeleted`", "`showHidden`"] `` |
| 282 | `max_attendees_carries_the_discovery_minimum` | `events.list: {"anyOf":[{"type":"integer"},{"pattern":"^-?[0-9]+$","type":"string"}]}` (the same for events.get), `0 sent=true` |

## 3. Suite run, made after the cases existed

Command: `cargo test --locked -p connectors-catalog-provider --no-fail-fast` (own target dir `<worktree>/target`), exit status 101. Every other target is `ok`; the only failure is `google_calendar_gmail_adversary: 0 passed; 6 failed`. The totals are 183 executed and 6 ignored (local_runtime). With my 6 left out of the same run, the count is 177. The names that ran include `google_calendar` (16) and `google_gmail` (20), so it was this tree that ran.

## 4. Findings

1. **Gmail reset rule loses changes.** Measured at `docs/catalog-google-gmail.md:121-123` and `providers/google-gmail/operations.json:23`. Both say to do the full walk first and then read the baseline from `users.getProfile`. A baseline read after the walk is newer than what the walk saw, so a message added or relabelled during the walk never shows up in any later `history.list`. The fix is to take the baseline before the walk starts.
   - What reaches it: the documented recovery after any 404 from `history.list`. Google's pinned text says this happens after as little as "a few hours".
   - NEEDS-CHANGE, introduced, warning.

2. **Paging descriptions do not warn about `fields`.** Measured at `providers/google-calendar/operations.json:6,10` and `providers/google-gmail/operations.json:9,16,23`. The Drive descriptions warn that `fields` must keep `nextPageToken`; these five do not. If a caller's `fields` leaves the token out, the walk stops silently after page 1, and for Calendar the `nextSyncToken` is lost as well.
   - What reaches it: `docs/catalog-google-gmail.md:176` recommends `fields` to narrow answers, and both guides say `fields` is accepted.
   - NEEDS-CHANGE, introduced, warning.

3. **A daily-quota 403 is reported as `forbidden`.** Measured with the `rate_limit_reasons` at `providers/google-gmail/operations.json:10` and `providers/google-calendar/operations.json:18`. A 403 with reason `dailyLimitExceeded` (domain `usageLimits`) reaches the caller as `forbidden`, which an agent reads as a missing scope.
   - What reaches it: Google's published Calendar and Gmail error guides list this 403. That is outside the repository and I did not verify it here.
   - CONFIRMED, introduced, warning.

4. **The Calendar reset rule cannot tell an expired token from a lost calendar.** Measured at `providers/google-calendar/operations.json:10` and `docs/catalog-google-calendar.md:113-116`. Both say "a 410 (not_found) means the sync token expired". But a 404 for a calendar that no longer exists or is no longer shared gives the same code and the same message (the engine maps `404 | 410`, `lib.rs:756`). An agent holding a sync token cannot tell the two apart. `ErrorCode::StaleCursor` exists and is unused here.
   - What reaches it: a calendar that was unshared while the agent held its sync token.
   - CONFIRMED, introduced, note.

5. **The Sync tokens section leaves out `showDeleted` and `showHidden`.** Measured at `docs/catalog-google-calendar.md:105-111`. The pinned Discovery says `showDeleted=false` (and `showHidden=false` for `calendarList`) is not allowed with `syncToken`, and the guide does not say so. Line 100 also says to resend "the same other parameters", so a walk that set `showDeleted: false` produces a delta Google refuses.
   - CONFIRMED, introduced, note.

6. **`maxAttendees` drops the Discovery minimum.** Measured on `events.list` and `events.get`. Discovery declares `minimum: 1`, but the declared input schema has no minimum and `0` is sent. The selection bounds `maxResults`, which comes from the same source, but not this parameter.
   - What reaches it: an agent reading `operations describe`. Google answers with a 400.
   - CONFIRMED, introduced, note.

7. **Judgement only, no case:** `providers/google-calendar/operations.json:10` (the events.list description). It says to walk "optionally within timeMin..timeMax" and to send `syncToken` later "without timeMin…". That invites a windowed full walk followed by an unwindowed delta, which Discovery calls undefined behaviour ("All other query parameters should be the same"). The guide at `:110` gets this right; `describe` does not.
   - CONFIRMED, introduced, warning.

8. **Judgement only, no case:** `docs/catalog-google-gmail.md:173-176`. It says "`metadata` or `fields` narrows the answer", but nothing narrows `format=raw`. A message over about 3 MiB is unreadable in `raw`: the caller gets `capacity` "response exceeds byte limit" (`crates/connectors-client/src/lib.rs:158-161`). Its attachments cannot be reached either, because `messages.attachments.get` is not selected, and the guide does not say so.
   - CONFIRMED, introduced, note.

## 5. Attacked and could not break

- Selection ids and operation ids match Discovery. Scopes: `gmail.readonly` and `calendar.readonly` are accepted by all 10 methods.
- The `maxResults` bounds (250, 2500, 500×3) match the Discovery description text.
- Standard parameters that could carry credentials (`access_token`, `oauth_token`, `key`, `callback`, `alt`) are all excluded by the projection. Only `fields` survives.
- The selected Gmail reads are all GET, and the engine refuses a read selection whose method is not GET (`lib.rs:290`).
- The URL encoder escapes a `#` or `/` inside a path value, e.g. the `en.usa#holiday@…` calendar ids that calendarList returns (`http.rs:252-263`).
- The projection counts in the guides are true (79/78/1, 6 upload methods, 38).
- History paging and the `historyId` baseline on the last page match Discovery.

## 6. Paths written outside the worktree

- `~/.cache/w0930cg/adv1/suite.log` (in the assigned scratch directory, which is mode 700)

The build went to `<worktree>/target`. The lease `wave0930c-cg-adv1` was started and has now been released.

## 7. Findings block

```findings
- file: docs/catalog-google-gmail.md
  line: 121
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The Gmail reset rule and the history.list description take the users.getProfile baseline after the full walk, so any change made during the walk is never replayed by a later delta."
- file: adapters/catalog/providers/google-gmail/operations.json
  line: 9
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Five Calendar and Gmail paging descriptions omit the Drive warning that fields must keep nextPageToken (and nextSyncToken), so a narrowed walk ends silently after one page."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 18
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "A 403 usageLimits dailyLimitExceeded quota answer reaches the caller as forbidden because rate_limit_reasons names only rateLimitExceeded and userRateLimitExceeded."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 10
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The documented rule that not_found means an expired sync token also fires on a 404 for an unknown or unshared calendar, because both reach the caller with the same code and message."
- file: docs/catalog-google-calendar.md
  line: 105
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The Sync tokens section omits the pinned rule that showDeleted (and showHidden for calendarList) cannot be false with syncToken, while telling callers to resend the same other parameters."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 9
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "maxAttendees on events.list and events.get loses its Discovery minimum of 1 in the declared input schema, and 0 is sent."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 10
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "The events.list description invites a windowed full walk followed by an unwindowed syncToken delta, which the pinned document calls undefined behaviour."
- file: docs/catalog-google-gmail.md
  line: 176
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The guide says metadata or fields narrows an oversized answer, but nothing narrows format raw, and attachments are unreachable because messages.attachments.get is not selected."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
