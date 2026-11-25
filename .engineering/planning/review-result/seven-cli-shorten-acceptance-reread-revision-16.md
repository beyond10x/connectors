---
format: aep.planning-md/3
id: review-result:seven-cli-shorten-acceptance-reread-revision-16
kind: review-result
status: active
title: Acceptance critic, read of the seven-cli-shorten story at revision 16
relations:
- reviews: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
revision: 1
---
Verdict: approve, for story:seven-cli-tests-shorten-their-waits-or-a-production-wait at revision
16 of its front matter (the body calls itself revision 11). All findings are notes; none blocks.

Read-only read by the acceptance critic: the story, the two critic records of revision 14, the
diff of the three planning files, and the code on main. Nothing was run.

## The record of revision 10 (approve, no blocker)

- The three items it checked (the probe taking the slow first answer, no claim that the clock
  test proves the variable is read, the summary line in `--source`) are moot: the clock override
  is gone and those statements went with it.
- Its note that the duration checks can be read only in the pull request's gate job still holds,
  now for acceptance 1 and 3. The sibling stories have the same shape.

## Findings, all notes

1. Acceptance 2 and 3, Change 3 (`apps/connectors/tests/local_cli.rs:95-101`, `:325-393`). No
   statement holds that the three roots and the three silent sources are built before the
   threads, or that the same four other sources, three tables and assertions stay. Acceptance 3
   reads only the duration. An implementation that runs `clock_root` (a `setup init` CLI process)
   inside each thread, or drops a row, still passes under 8.5 s. Settles it: one clause in
   acceptance 3 that a reading of the diff checks.
2. Acceptance 2 and 5, Change 4 (`apps/connectors/tests/catalog_provider_failure.rs:105-117`,
   `apps/connectors/tests/support/standin.rs:451-473`). The Change says the provider case's own
   command keeps `env_clear()`, a null stdin and the ETXTBSY retry, and that `prerequisite` is
   not edited. No acceptance statement holds any of these. The wrong binary
   (`CARGO_BIN_EXE_connectors`) is caught: the case would see `missing`. A command without
   `env_clear()` or the retry still passes. Settles it: add them to acceptance 2.
3. Acceptance 2, Change 5 (`apps/connectors/tests/owner_tether_adversary.rs:181`).
   `saturating_sub` and "`start_owner` keeps its signature and callers" are in no acceptance
   statement; the old subtraction still passes every check. Settles it: name `saturating_sub` in
   the tether bullet of acceptance 2.
4. Acceptance 6. `git diff --stat origin/main -- clock.rs standin.rs` depends on the branch being
   current: pull request 303 edits `clock.rs`, so a branch behind main prints lines for it.
   Settles it: `git diff --stat origin/main...HEAD -- …`.
5. Change 4. The story says the field `root` of `Installation` is private (line 413). On main it
   is `pub root: tempfile::TempDir` (`apps/connectors/tests/support/standin.rs:414`); the case can
   read it. The design and the parallel-safety record repeated the error. Settles it: correct the
   sentence.

## Each statement

- Acceptance 1: observable (nextest durations in the gate job) and it names its state (the five
  tests, before 47.0 s). A build that leaves out the walk and clock changes still gives about
  33.5 s, under 38 s; acceptance 2 and 3 catch that. A correct build fails it only when the five
  tests run 12.8 s over the estimate.
- Acceptance 2: checkable by grep and diff. The clause on stale values gives examples that leave
  out the walk's doc comment (`apps/connectors/tests/invoke_walks.rs:775-781`); the general clause
  covers it.
- Acceptance 3: 8.5 s is 2.5 + 3 × 2, so rows one after another cannot pass; the estimate of
  5.0 s leaves 3.5 s. A half-concurrent build passes, see finding 1.
- Acceptance 4: the two test names are observable and fail today, because the functions do not
  exist. The call sites are held by acceptance 5. A direct read of the variable is caught by the
  helper story's check of direct reads.
- Acceptance 5: provider, an unread variable gives 10 s against 6 s. Tether, an owner that ignores
  the variable leaves at 5.0 s or later against 4.2 s; a correct owner (about 2.2 s) must be 2 s
  late to fail.

## Checked and right

- Story arithmetic: 8.0 + 6.4 + 5.0 + 1.5 + 4.3 = 25.2; plus 7.0 = 32.2; 54.0 − 7.0 = 47.0;
  1.5 × 25.2 = 37.8, rounded up to 38; 11.5 − 9.6 + 6.1 = 8.0; 10.9 − 9.0 = 1.9;
  9.0 − 8.5 + 4.5 = 5.0.
- Parent story: 332.8 − 24.5 + 32.2 = 340.5; plus 7.4 = 347.9; 10.2 + 7.4 + 16.5 + 8.3 + 7.0 =
  49.4; 347.9 − 49.4 − 50.6 = 247.9; plus 7.2 and 8.8 = 363.9; plus 5.0 = 352.9;
  2,474.8 − 1,176.8 + 352.9 = 1,650.9; divided by 4 is 412.7. No stale figure is left.
- The idle-exit story's one changed line matches this story's region of the clock test.
- Launch test: the sleep after the cancel at `apps/connectors/tests/owner_cancel_adversary.rs:809`
  is a `saturating_sub` of elapsed time, so "ends 4 s after the cancel" is a change of one value.
  Nothing else in the test reads 5,000 ms or 7 s.
- Walk: a page keeps 2.5 s; the third page ends at 4.5 s, so one cutoff of 4 s for the whole walk
  fails every time, and a slower machine widens it.
- Tether, from `crates/connectors-host/src/local/owner/tether.rs:14` (`EVERY`, 200 ms) and
  `crates/connectors-host/src/local/owner/transport.rs:1918-1930`: the owner notices within
  200 ms and drains 2 s, so it leaves at about 2.2 s; the first window leaves 1.0 s before the
  earliest correct exit; an unbounded drain leaves about 8 s after the removal; the client is
  still counted when the drain ends, 4.2 s after its connect.

## Outside the acceptance lane

- The launch test's last sleep ends 2 s after the hold ends (3.5 s today). The story does not say
  what that 2 s outlasts, which its own rule on negative checks asks for.
- The tether's window of 1,000 ms lets an owner without a drain pass only on a runner more than
  0.8 s late. The rule allows that weakening.

## Not established

- Any "after" figure; none was timed.
- Whether a plain `cargo nextest run` of the clock test needs the gate's setting for the temporary
  directory. The story lists this itself.
