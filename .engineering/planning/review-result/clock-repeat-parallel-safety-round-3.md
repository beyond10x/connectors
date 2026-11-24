---
format: aep.planning-md/3
id: review-result:clock-repeat-parallel-safety-round-3
kind: review-result
status: active
title: 'clock repeat story: parallel-safety critic, round 3'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: approve

Round 2 findings of my axis (one line each):
- Generated page `website/docs/reference/contracts/clock.md`: answered. It is in scope, change 5 names it, and statements 20 and 25 check it.
- `clock.md` line shift: answered. The sentence edit is net-zero, the new section goes after line 235, and statement 24 checks line 163.
- CHANGELOG heading: answered, one wording note remains (finding 4).
- `site()` and the query deadline: answered. `Server::dropping` is added and each test sets 500 ms after `site()`.
- Recovery test location: answered. It is `mutation/tests.rs` plus a `#[cfg(test)]` constructor in `supervisor.rs`.
- Pattern-grant head and time: answered by the head `d45310985` and the 08:10 timestamp.

Surfaces: 1 item, 15 of 15 scope paths cited, 0 inferred, 0 unplaced.
- Acceptance reads traced to a producer in the set: 0. The `Site` fixture is changed by this story itself.
- Pattern-grant (head plus uncommitted, read now) does not touch `server.rs`, `supervisor.rs`, `website/.../contracts/clock.md`, `contracts/service/clock.md`, `clock.rs`, `execution.rs`, `approval_issuance.rs`, `local_cli.rs` or `docs/local-clock.md`. Its `owner/mutation/tests.rs` change is the one line at `:577`. It regenerates `website/.../model/domains/connectors-clock.md`, which is a different file.
- `story/connections-through-owner`, read with `git diff` from merge-base 2e5ddabe9, touches none of the three new files and none of the scope files except `CHANGELOG.md`. The story's "Shares files with" claim holds.
- The two trees do not touch the three new files, and no other local `story/*` branch collides.

Thread question: the owner tests reach the acquisition on the thread that set the timeout.
- `query_timeout()` is read on the calling thread at `clock.rs:387`.
- `time()` (`execution.rs:84-97`), `standing_reading` and `recover_observation` are synchronous.
- `site()` sets 50 s at `tests.rs:718`, and the test then overrides it on the same thread.

Tests 1 to 12 build as written. 13 and 14 need the `until` parameter in finding 2. 15 builds from `candidate` and `store` (`mutation.rs:380-388`): `Store::prepare` takes no clock (`mutations.rs:123-126`), and `Quiescent` has private fields (`supervisor.rs:332-335`). 16 needs finding 2. 17 builds on the same terms as 15. 18 to 25 are checkable.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 337
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'Shares files with' omits the sibling draft story:a-clock-refusal-of-issue-and-invoke-names-its-sources. Its body names `clock.rs`, `owner.rs` (`Error::clock`), `apps/connectors/src/local.rs`, `CHANGELOG.md` (sibling lines 20-24, 37-40). Changing `Error::clock` touches the same five call sites that change 3 edits (`approval_issuance.rs:392`, `:632`, `execution.rs:92`, `recovery.rs:52`). Its only edge is depends_on pattern-grant (sibling line 9), none to this story, so the pair reads as parallel (cited, both bodies). Fix: add a bullet. Both edit `clock.rs`, the four call-site expressions and `CHANGELOG.md`. Remedies, not chosen: a depends_on edge on the sibling, or this story keeps `Error::clock(selected, refused.failure)` and the sibling rebases on `acquire_within`. The other sibling already has the edge to this story."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 246
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 13 names only a step parameter (or sibling) of `write()`. `write()` also hardcodes `until: Instant::now() + WAIT` (`mutation/tests.rs:987`), and statement 16 needs `until` 2 s after the call, so 16 cannot be built from `write()` as it is. `write()` also always installs the revoke hook and the `seam` credential (`:932-982`), so a run without a revoke hook still needs the credential. Fix: say `write()` or its sibling takes `until` and an optional revoke step, with the credential seam kept. Also say the two existing callers (`tests.rs:1049`, `:1069`) are edited when `site()` gains a parameter."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 344
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'its tests use `site()` and `write()`' is not what the pattern-grant tree shows. Its diff of `owner/mutation/tests.rs` is the one line at `:577`; the callers at `:1049` and `:1069` are base tests. No collision follows, since the hunks are far apart. Fix: say the pattern-grant diff of this file is `:577` only at head d45310985, and that a new pattern-grant test calling `site()` or `write()` is the case that rebases."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 343
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Pattern-grant creates `### Added`; connections (line 350) and this story both use `### Fixed`. 'adds its own section under it' read literally gives two `### Fixed` headings when the connections branch merges first. Line 147 already says a heading is created only where the base lacks it. Fix: line 343 and 350 say 'adds its entry to the existing `### Fixed` if the base has one'."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 309
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'carries the attribute the file's other constructors carry' is wrong. `server.rs:4` holds one file-level `#![allow(dead_code)]` (with the comment at `:3`, 'Three test crates include this file'), and the constructors carry none. The new constructor needs nothing, and statement 27 passes with it. Fix: 'the file's `#![allow(dead_code)]` (`server.rs:4`) covers it'. Includers are `clock.rs:6`, `local_cli.rs:2`, `adapters/catalog/tests/main.rs:9`."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 291
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Statement 22 counts the lowercase phrase 'second attempt'. Change 5 describes the replacement sentence as 'make at most a second', and the heading is `### Second attempt` (capital S), which a case-sensitive `rg` does not match. Both `clock.md` files need the lowercase phrase in the sentence or the section body. Fix: say the sentence uses the words 'second attempt', or use `rg -ci`. Also `clock.rs:36-45` is the thread-local at `:36-39` and the setter at `:46-48`."
```

Out of my lane: `docs/local-clock.md:92` says "with no DNS, automatic retry, …" in the `clock-check` paragraph. The story keeps lines 91-92, which is right for `clock-check`, but the new sentence should say that "automatic retry" there means `clock-check` only (acceptance or scope critic).
