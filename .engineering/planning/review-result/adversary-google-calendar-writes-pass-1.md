---
format: aep.planning-md/3
id: review-result:adversary-google-calendar-writes-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Google Calendar writes
relations:
- reviews: story:catalog-google-calendar-writes
revision: 1
---
```
unit: story:catalog-google-calendar-writes, impl/catalog-google-calendar-writes at 3d5f9241747bb33928405fe9f497192f45487799 (base 6c8fd8b31) plus one untracked adversary test file
verdict: NEEDS-CHANGE
cases: executed 167→172, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (the scratch directory and one log in it, listed in part 6)
needs-coordinator: none
```

**1. What I touched.** `git --no-pager diff --stat` prints nothing, because the only change is one new file that git does not track yet. `git status --short` shows `?? adapters/catalog/tests/google_calendar_writes_adversary.rs`. It is a test file, and no implementation file was touched.

**2. The cases I added.** All are in `adapters/catalog/tests/google_calendar_writes_adversary.rs`. I ran each one alone with `-- --exact` before running the suite, and each is red now.

| Case | What it asserts | Red output (the case run alone) |
|---|---|---|
| `a_write_to_an_event_google_deleted_sends_no_write` (:191) | The guide says an event Google has deleted is refused with no write sent. The pinned document says a deleted event has `status` `cancelled`, and "The get method always returns them." The case answers the preflight with a cancelled event whose `etag` matches the pin, and asserts that nothing is sent. | `a write was sent to an event Google reports as deleted: ["events.patch: sent Patch calendars/primary/events/fixture-event-1, classified Ok(\"applied\")", "events.delete: sent Delete calendars/primary/events/fixture-event-1, classified Ok(\"refused NotFound\")"]` at :239 |
| `a_write_without_send_updates_is_refused_before_any_request` (:252) | The descriptions say "name it explicitly" and the guide says "Name it in every write input". The case asserts that the declaration requires `sendUpdates` and that an input without it is refused before any request. | `events.insert: declared required ["body","calendarId"] omits sendUpdates`, `events.insert: without sendUpdates Ok("applied"), sent query Some([])`, and the same two lines for patch and delete, at :291 |
| `a_write_naming_both_notification_parameters_is_refused` (:303) | The guide says "do not send both" of `sendUpdates` and the deprecated `sendNotifications`. The case asserts that an input carrying both is refused. | `events.insert: Ok("applied") with query [("sendNotifications","true"),("sendUpdates","none")]`, the same for patch and delete, at :328 |
| `patch_warns_that_a_truncated_guest_list_replaces_every_guest` (:343) | The patch guard passes on a guest list built from a read that used `maxAttendees` (the case checks this part and it passes). It then asserts that the patch description or the guide warns about `maxAttendees` or `attendeesOmitted`. | `neither the events.patch description nor the guide warns that a guest list read with maxAttendees is truncated` at :382 |
| `every_google_guide_names_one_authorize_url` (:393) | All four Google guides name one Google `authorize_url`. | The calendar guide has `https://accounts.google.com/o/oauth2/auth` twice; the Drive, Slides and catalog provider guides have `.../o/oauth2/v2/auth` (:411) |

**3. The suite run, after the cases existed.** The command was `cargo test -p connectors-catalog-provider --no-fail-fast` with the invariants' environment, `CARGO_TARGET_DIR` set to the worktree's own `target`. It exited 101. The only failing binary was `google_calendar_writes_adversary`: `0 passed; 5 failed`. Every other binary passed, including `google_calendar` with `26 passed` and `local_runtime` with `30 passed; 6 ignored`. The total is 172 executed. The "before" figure of 167 comes from the same run with my binary's 5 cases taken out. fmt `--check` is clean, and `clippy --test google_calendar_writes_adversary -- -D warnings` exited 0.

**4. Findings.** All cover 3d5f92417 plus the adversary file. The base has no Calendar writes and its guide used the `v2` URL, so every finding originates in this unit.

- **F1: a deleted event passes the guard.** Where: `docs/catalog-google-calendar.md:182`, `operations.json:23,31`. Measured: the guard compares only `/etag`. A cancelled event with a matching `etag` gets its PATCH sent and reported `applied`. Its DELETE is also sent, and Google's `410` comes back as `refused` `NotFound` even though a write went out. The fixture's `410` on `events.get` is invented; the pinned Discovery says a get returns deleted events. What reaches it: the guide tells agents to take the pin from `events.get` or an `events.list` item. Both hand back deleted events. Fix, named and not applied: correct the guide and descriptions, or add a literal check that `/status` is `confirmed` to the guard, which would also refuse `tentative` events. Verdict: NEEDS-CHANGE, warning.
- **F2: `sendUpdates` is optional.** Where: `operations.json:15,18,26`. Measured: all three writes accept an input without it and send no `sendUpdates` at all. The pinned document gives no default for patch and delete, so the approved input does not say whom Google emails. What reaches it: `operations describe` shows the parameter as optional. Fix: add `"required": ["sendUpdates"]` to each write and regenerate. Verdict: NEEDS-CHANGE, warning.
- **F3: both notification parameters go out together.** Where: `docs/catalog-google-calendar.md:157`. Measured: an input with `sendUpdates=none` and `sendNotifications=true` is sent with both. The pinned document does not say which one Google obeys. A selection has no way to exclude a parameter or make two exclusive, and the story forbids editing `lib.rs`. It can only be documented here. Verdict: INFEASIBLE, note.
- **F4: no warning about truncated guest lists.** Where: `operations.json:18`, `docs/catalog-google-calendar.md:187`. Measured: `events.get` and `events.list` both take `maxAttendees`, which returns "only the participant". A patch whose `attendees` came from such a read passes the guard and sends the caller as the only guest. Inferred, not verified live: that Google's `etag` is the same for a truncated read. Fix: one sentence in the patch description and the guide naming `maxAttendees` and `attendeesOmitted`. Verdict: CONFIRMED, warning.
- **F5: the Google guides disagree on the authorize URL.** Where: `docs/catalog-google-calendar.md:242,282`. Measured: this unit moved only the calendar guide to `/o/oauth2/auth`. Verdict: CONFIRMED, note.

**5. What I attacked and could not break.** A stale `etag`, or the same value without its quotes, is refused after the one read, with no write sent. The pinned `etag` is never sent to Google. A delete's empty `204` comes back as `null`. `oauth_token`, `key` and `alt` are not projected; `fields` is the only global parameter on the writes. The recurring-event claims match what the descriptions say. Guide claims checked against the pinned Discovery and found true: the `calendar.events` description, `guestsCanInviteOthers` and `guestsCanSeeOtherGuests` default to true, the `body.id` collision caveat, conference data created asynchronously, the insert `sendUpdates` default is false, and the `none` warning.

**6. Paths written outside the worktree.** `~/.cache/w0930wc/adv1/` (mode 700) and `~/.cache/w0930wc/adv1/suite.log`. Lease `wave0930c-calendar-writes-adv1` released.

**7. Findings block.**

```findings
- file: docs/catalog-google-calendar.md
  line: 182
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The guide says a deleted event is refused with no write sent, but the pinned Discovery says events.get always returns deleted events as status cancelled with an etag, so a patch or delete pinned to that etag passes the /etag-only guard and is sent (patch reported applied, delete's 410 reported refused although a write went out)."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 18
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "sendUpdates is optional in all three write declarations despite the descriptions' 'name it explicitly', so an approved input without it is sent with no sendUpdates and Google's unstated default for patch and delete decides who is emailed; the selection's required list (lib.rs:117) would enforce it."
- file: docs/catalog-google-calendar.md
  line: 157
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The guide says not to send both sendUpdates and sendNotifications, but an input carrying sendUpdates=none and sendNotifications=true is sent with both, and no selection field can forbid the pair without an engine change the story excludes."
- file: adapters/catalog/providers/google-calendar/operations.json
  line: 18
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Neither the events.patch description nor the guide warns that an event read with maxAttendees lists only the participant (attendeesOmitted), so a patch whose attendees came from such a read passes the etag guard and replaces every guest with the caller."
- file: docs/catalog-google-calendar.md
  line: 242
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The calendar guide moved authorize_url to /o/oauth2/auth while the Drive, Slides and catalog provider guides still configure the same google.oauth profile with /o/oauth2/v2/auth."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
