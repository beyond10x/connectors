---
format: aep.planning-md/3
id: story:standing-grant-closes-the-input-to-listed-members
kind: story
status: implemented
title: A standing grant closes a write's input to the members it lists
summary: 'optional input_keys on a grant: a write with an input member outside the list is approval_required with no request, in both placements'
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: inferred
  path: crates/connectors-build/generated/metadata-emissions
- confidence: inferred
  path: crates/connectors-host/src/local/service.rs
- confidence: cited
  path: crates/connectors-host/src/local/standing.rs
- confidence: cited
  path: crates/connectors-host/tests/http_standing_grant.rs
- confidence: inferred
  path: crates/connectors-host/tests/http_standing_grant_adversary.rs
- confidence: inferred
  path: crates/connectors-host/tests/http_standing_grant_review.rs
- confidence: cited
  path: docs/running-services.md
- confidence: cited
  path: ess/domains/standing_approvals.yaml
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T12:03:23Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-23T12:03:24Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-24T06:33:45Z", actor: "human:timo", revision: 19, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Outcome

A standing grant can list the input members its caller may set. A write whose input holds any
other member is outside the grant: `approval_required`, no provider request, no attempt. A grant
without the list admits what it admits today.

## Source

- The `inbox` bridge, 2026-11-23: "a closed list of input members the caller may set"
  (`/body/text`, `/body/thread_ts`), "anything else refused before any request".
- D8 = B (the operator, 2026-11-23): the accepted record stays; the list is a member of the grant.
- `architecture-decision-record:standing-scoped-approval`, decision 1: the checks run before any
  attempt is recorded and before any provider request. The record names no closed input; this
  story adds one check to that list and no other behaviour.
- Cut out of story:local-invoke-admits-a-keyed-write-under-a-standing-grant on its scope
  critic's round 1: the grant model is shared by both placements, so the member is a change of
  the served placement too and gets its own acceptance.

## What the tree holds today

Read at 5983a6b9502 (0.75.0). `host/` is `crates/connectors-host/src/local/`.

- `host/standing.rs`: format `connectors.standing-grants/1` :38; limits :40-48 (`GRANTS_LIMIT`
  256, `TARGET_LIMIT` 16, `POINTER_LIMIT` 256, `MAX_PER_HOUR` 10,000); `Uncovered` :67 and its
  `rank` :243, an exhaustive match (`NoGrant` 0, `OtherOperation` 1, `MissingMember` 2,
  `OtherTarget` 3, `Revoked` 4, `Expired` 5); `Grant` :97 (`deny_unknown_fields`: id, instance,
  operation, target, not_after_unix_ms, max_per_hour); `parse` :130 refuses an empty `target`
  :147; `covering` :191 goes through the grants in file order, compares each target member
  with `input.pointer(..)` (absent: `MissingMember`) and `!=` on `serde_json::Value`, and
  returns every covering grant.
- One caller: `host/service.rs:466` (the served host, `check_grant` :456), which passes
  `write.input`.
- Tests of the served host: `crates/connectors-host/tests/http_standing_grant.rs` (helpers `grant`
  :108, `grant_file` :121, `assert_refused_before_anything` :302; a target outside the grant:
  `a_write_to_another_target_is_refused_with_no_request` :451); two more files call `parse` and
  `covering`: `http_standing_grant_adversary.rs`, `http_standing_grant_review.rs`.
  (acceptance critic, round 2) `-E 'test(standing::)'` selects 8 unit tests today.
- ESS: `ess/domains/standing_approvals.yaml` `StandingGrant` :32-43, admission order :158-175.
- The local owner bounds the input's depth at 64 before it parses it
  (`host/runtime/channel.rs:376-378`, called at `host/owner/approval_issuance.rs:221`). What
  bounds the depth of the served host's input was not read.
- Inferred from the design critic's reading, not read here: `read_json` refuses duplicate member
  names (`crates/connectors-core` `lib.rs:300`), and with `arbitrary_precision` two numbers are
  equal only when their text is equal, so `100` against `1e2` is refused.

## Design

- `Grant` gains `input_keys: Option<Vec<String>>` (JSON pointers, RFC 6901). The format stays
  `connectors.standing-grants/1`. A file without the member reads as today. A host older than
  this change refuses a file that has it (`deny_unknown_fields`): the whole file is unreadable
  there and every keyed write is refused. `contracts/service/compatibility.md` §2.1 says so.
- Validation in `parse`: at most 16 pointers in `input_keys`, each of at most 256 bytes; each
  passes `valid_pointer` as a target pointer does (a leading `/`; `~` only as `~0` or `~1`);
  over all pointers of `target` and `input_keys` of one grant, none equals another and none is
  a prefix of another by reference tokens (`/body` beside `/body/channel` is refused,
  `/body/text` beside `/body/text2` is not). A violation makes the file `Malformed`, as a wrong
  `max_per_hour` does today.
- The check, in `covering`, for a grant whose target members all matched, before the revoked
  and expired checks. Absent `input_keys`: no check. Present (it may be empty): walk the input
  from the root. A non-empty object is descended; a member name becomes a reference token (`~`
  as `~0`, `/` as `~1`). Every other value (scalar, null, array, empty object) is a leaf. A
  leaf is covered when its pointer equals a `target` pointer or an `input_keys` pointer, or
  its reference tokens start with all the tokens of one. A listed pointer therefore admits
  the whole subtree under it, and a listed array admits any elements. One uncovered leaf puts
  the write outside that grant: new variant `Uncovered::OutsideInput`, rank 4; `Revoked`
  becomes 5 and `Expired` 6. It is answered `approval_required` like the others, axis in the
  host's log only. `covering` goes on to the next grant in file order, as for the other
  reasons.
- Arrays are leaves, so a listed pointer that points into an array (`/body/blocks/0`) can
  never cover anything; the grant then refuses every input that has the array. A `target`
  pointer into an array does the same whenever `input_keys` is present: the target member
  matches, and the walk then finds the array as an uncovered leaf. The documents say both.
  `parse` cannot tell such a pointer from a member named `0` and accepts it.
- A closed list limits its own grant only. A second grant for the same instance, operation
  and target without `input_keys` admits what the first refuses (`covering` returns every
  covering grant, and a grant is a sufficient permission, decision 1 of the record). The
  contract text and `docs/running-services.md` say so.
- Depth. The walk counts nesting as `runtime::channel::depth` counts it (the root object is
  depth 1) and does not descend past depth 64: an input nested deeper is `OutsideInput`,
  whatever the caller checked before, so both placements have the bound.
- The walk reads the `Value` the caller passes to `covering`. The local path passes the value
  `connectors_core::read_json` built from the bytes it sends (the other story states it); the
  served host passes `write.input` as today. Target values keep today's comparison (`!=` on
  `Value`).
- Served host: `service.rs:466` passes the same input as today; no other change there.

## Design first, in this story

1. ESS through `ess:specifying`: `ess/domains/standing_approvals.yaml` (`StandingGrant` gains
   `input_keys`, the admission order gains the check after the target members).
2. Contract text: `contracts/operations/v1alpha1/semantics.md` (the list of checks that replace
   steps 5 and 8), `contracts/service/compatibility.md` §2.1, `docs/running-services.md:251-275`
   (the grant file's members, the array rule, and that a closed list limits its own grant
   only).

## Acceptance

Commands run in the repository root. A test runs 60 s at most; this story's test commands end
within 120 s. The unit tests are in the module `local::standing::tests`.

1. `closed_input_refuses_every_leaf_outside_the_grant`: with target `/body/channel` and
   `input_keys` `["/body/text", "/body/thread_ts"]`, `covering` returns the grant for `{body:
   {channel, text}}` and for `{body: {channel, text, thread_ts}}`, and
   `Err(Uncovered::OutsideInput)` for each of: `/body/mrkdwn` added; `/body/text2` added; a
   second top-level member; `/body/extra` holding `{}`; `/body/extra` holding `[]`;
   `/body/extra` holding `null`; a member named `te/xt` (pointer `/body/te~1xt`). A scalar
   root returns `Err(Uncovered::MissingMember)`.
2. `a_listed_pointer_admits_its_subtree`: `input_keys` `["/body/blocks"]` admits `/body/blocks`
   holding an array and holding a nested object; `input_keys` `["/body/blocks/0"]` refuses
   `/body/blocks` holding an array of one element with `OutsideInput`. Depth, with
   `input_keys` `["/body/blocks"]`: an input whose deepest object is at depth 64 is admitted,
   and one whose deepest object is at depth 65, under `/body/blocks`, is `OutsideInput`. With
   target `/body/blocks/0/type` and `input_keys` `[]`, an input whose `/body/blocks/0/type`
   holds the granted value is `OutsideInput`.
3. `an_absent_input_keys_leaves_the_input_open`, `an_empty_input_keys_admits_only_the_target`
   and `the_second_grant_covers_when_the_first_is_closed`: absent admits `/body/mrkdwn`; `[]`
   refuses `/body/text` and admits `{body: {channel}}`; with two grants in the file for the
   same channel, the first with `input_keys` `[]` and the second with `["/body/text"]`,
   `covering` for `{body: {channel, text}}` returns the second grant alone.
4. `input_keys_are_validated_at_parse`: `Malformed` for 17 pointers in `input_keys` (with one
   target member), a pointer of 257 bytes, a pointer without a leading `/`, a pointer holding
   `~2`, the same pointer twice, a pointer equal to a target pointer, `/body` beside target
   `/body/channel`, `/body/text` beside `/body/text/x`; a file with 16 pointers parses, and a
   file with `/body/text` beside `/body/text2` parses.
5. `served_input_keys_close_the_input` (`crates/connectors-host/tests/http_standing_grant.rs`):
   under one grant file with `input_keys`, a write inside the list is dispatched once and names
   the grant id, and a write with one member outside the list is refused by
   `assert_refused_before_anything` (:302) with the code that
   `a_write_to_another_target_is_refused_with_no_request` (:451) asserts.
6. A file without `input_keys` behaves as before: `git diff -U0 $(git merge-base origin/main
   HEAD) -- crates/connectors-host/tests/http_standing_grant.rs
   crates/connectors-host/tests/http_standing_grant_adversary.rs
   crates/connectors-host/tests/http_standing_grant_review.rs | rg '^-[^-]'` prints nothing
   (lines are only added), `cargo nextest run -p connectors-host --test http_standing_grant
   --test http_standing_grant_adversary --test http_standing_grant_review` exits 0, and `cargo
   nextest run -p connectors-host -E 'test(standing::)'` exits 0 and runs the 8 tests of the
   merge base and the tests of lines 1-4.
7. `rg -n 'input_keys' contracts/service/compatibility.md contracts/operations/v1alpha1/semantics.md
   docs/running-services.md ess/domains/standing_approvals.yaml` prints at least one line per
   file, and compatibility.md states that an older host refuses a file with the member.
8. `CHANGELOG.md` has an `### Added` line under `## Unreleased` naming `input_keys`.
9. `git diff --name-only $(git merge-base origin/main HEAD) | rg '\.py$'` prints nothing.

Not checked by a line of this story: the generated files. This story changes `ess/domains`,
which `inputs.json` digests, so the generated checks are red after it. They are regenerated
and checked once by acceptance 31 of
story:local-invoke-admits-a-keyed-write-under-a-standing-grant, in the same pull request.

## Order and shared files

- One implementor builds this story first and
  story:local-invoke-admits-a-keyed-write-under-a-standing-grant after it, in the same checkout
  and the same pull request. Both change `host/standing.rs`,
  `ess/domains/standing_approvals.yaml`, `contracts/operations/v1alpha1/semantics.md`,
  `contracts/service/compatibility.md`, `docs/running-services.md` and `CHANGELOG.md`.
- `crates/connectors-build/generated/metadata-emissions/inputs.json` digests `ess/domains` and
  the Entity Runtime pin. The generated files are regenerated once, at the end of the other
  story, after the rebase on a `main` that holds release 0.76.1 if it is out by then.
- `CHANGELOG.md` is also changed by story:settled-read-uses-leave-the-store,
  story:connections-list-answers-through-the-owner, story:windows-mcp-tool-call-answers-timeout
  and releases 0.76.0 and 0.76.1; this story's line goes in after the rebase.
- story:standing-grant-slot-released-before-dispatch (draft, no scope) cites
  `crates/connectors-host/src/server/v1alpha2.rs:654-664`, which this story does not change.

## Out of scope

- Any other change of the served host. A closed input for per-input approvals. A byte bound on a
  member's value. A canonical comparison of target values. A pointer syntax that reaches into
  arrays.
