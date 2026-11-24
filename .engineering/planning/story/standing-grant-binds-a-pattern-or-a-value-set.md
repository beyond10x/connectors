---
format: aep.planning-md/3
id: story:standing-grant-binds-a-pattern-or-a-value-set
kind: story
status: active
title: A standing grant binds a target member to a pattern or a value set
relations:
- depends_on: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- informed_by: architecture-decision-record:standing-scoped-approval
- serves: vision:independent-contract-adapters
- decomposes: epic:connectors-with-claude
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/local-mutations.md
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions
- confidence: cited
  path: crates/connectors-catalog/tests/main.rs
- confidence: cited
  path: crates/connectors-catalog/tests/whole_value_patterns.rs
- confidence: cited
  path: crates/connectors-host/Cargo.toml
- confidence: cited
  path: crates/connectors-host/src/local/standing.rs
- confidence: cited
  path: crates/connectors-host/tests/fixtures/whole_value_patterns.json
- confidence: cited
  path: crates/connectors-host/tests/http_standing_grant_adversary.rs
- confidence: cited
  path: docs/local-approvals.md
- confidence: cited
  path: docs/running-services.md
- confidence: cited
  path: ess/domains/standing_approvals.yaml
revision: 32
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T14:44:21Z", actor: "human:timo", revision: 31, decided_on: {"recorded":{"review_outcome":14}}}
- {from: "proposed", to: "active", at: "2026-11-23T14:44:22Z", actor: "human:timo", revision: 32, decided_on: {"recorded":{"review_outcome":14}}}
---
## Outcome

A standing grant may bind a target member to a pattern or to a list of values, so one
long-lived grant per operation covers a consumer whose target changes on every call. For
release 0.78.0, after the exact-value grants of 0.77.0.

## Decision

D11, Timo's pick of 2026-11-23 about 13:25 CET, relayed by the operator: "A now, C next
(Recommended)". A: 0.77.0 keeps exact-value grants. C: a target member may be a pattern
(`/issueIdOrKey` matching `^DEV-[0-9]+$`) plus a value set (`/target_status`), one long-lived
grant per operation with `max_per_hour` and `not_after`, changing the accepted record
architecture-decision-record:standing-scoped-approval. The operator: "Timo's yes for the record
change is this pick."

The record today (Decision, point 1): "every target member the grant names (a JSON pointer into
the operation's input and its exact value, such as `/owner` and `/repo`) is present in the
input with that value". This story revises that point in both placements (served and local).

## Consumer

The Jira step of release-review (dev-operator), facts from its session on 2026-11-23, read from
`bindings/tools/release-stage.sh:790-870` of its own repository and
`~/beyond10x/connectors-definitions/connectors/jira.yaml:54-68`; not run, no route yet:

- Writes per DEV ticket of a revision: `issue.transition.run` `{issueIdOrKey, current_status,
  target_status, body.transition.id}`, one call per hop, 1 to 5 hops per ticket; and
  `issue.comment.add` `{issueIdOrKey, body: ADF document}`, one per ticket.
- No input member is the same between calls. The ticket set differs every week.
- `issueIdOrKey` matches `^DEV-[0-9]+$`.
- `target_status` and `current_status` carry status ids. The connector checks them
  (`jira.yaml:54-68`): `guard.before` `/fields/status/id == current_status`; `guard.also`
  the transition named by `body.transition.id` leads to `target_status` and is available;
  `guard.after` the issue read back has `/fields/status/id == target_status`.
- Status ids in project DEV, read by that session about 13:50 CET from 100 DEV issues of the
  last 90 days: 3 In Progress, 10234 Review, 10223 QA, 10001 Done, 10000 To Do, 10235
  Preparation; also seen and never a target: 10002 Backlog, 6 Abandoned. Paths pass through
  To Do and Preparation, so the target set is 10000, 10235, 3, 10234, 10223, 10001.
- Writes per run, from 64 prod-eu `release.json` files: tickets min 0, median 23, max 80; at
  most 6 writes per ticket; worst case 480, median case 138. `max_per_hour` 500 covers it.
- Every comment starts with `Release review of ` (`release-stage.sh:815`); the consumer wants
  that prefix bound. The text sits inside an ADF document, so the pointer goes into nested
  arrays.

Under 0.77.0 the step runs with one exact-value grant per ticket and operation, issued before
the run by a person or the operator session (up to 160 grants at 80 tickets; the file holds
256).

## Design

Planned 2026-11-23 from the tree at 43bda33b08d (read, nothing built or run), revised the same
day after critic round 1. `S` is `crates/connectors-host/src/local/standing.rs`, `C` is
`contracts/cli/v1alpha1/semantics.md`, `K` is `contracts/service/compatibility.md`, all at
43bda33b08d. The 0.77.0 work moves these lines (`TargetMember` is at `S:140` in its working
tree); the implementation reads them again at the merged 0.77.0 head.

### Settled

1. Pattern language: the syntax of the `regex` crate (1.13.1, `Cargo.lock:3027-3028`), matched
   against the whole string value. The rule is the one `connectors-catalog` has
   (`crates/connectors-catalog/src/inventory.rs:182`, `:201-225`): the pattern starts with `^`,
   ends with an unescaped `$`, is matched as `\A(?:pattern)\z` and is at most 256 bytes. Only a
   string value can match. Matching is case-sensitive, and `\d`, `\w`, `\s`, `\b` are
   Unicode-aware, so a grant writes ASCII classes such as `[0-9]`; the contract says so.
2. The host has its own function for that rule, in `standing.rs`; `connectors-catalog` is not
   changed and `connectors-host` gains `regex = "1"` (in `Cargo.lock` already). It is a copy
   of the catalog's rule, and the story says so in a comment. One committed vector file,
   `crates/connectors-host/tests/fixtures/whole_value_patterns.json` (pattern, value, expected
   match; and patterns the form check refuses), is read by a test in each crate, so the two
   copies cannot differ on those vectors unseen. The host compiles with
   `RegexBuilder::size_limit` and `dfa_size_limit` both at 64 KiB (a proposal; the
   implementation measures the two consumer patterns and lowers or raises the figure once,
   before the contract text is written); the catalog keeps its own limits and its cache.
3. No cache in the host. A member keeps the pattern's text and its compiled form in one type
   whose equality compares the text, so `Grant` keeps `PartialEq` (`S:106`). A file holds at
   most 64 pattern members, counted like the 256-grant bound: at `grant-standing` after the
   expired grants are dropped (`C:974-975`), and at every parse. One parse therefore compiles
   at most 64 patterns and holds at most 64 times the two limits, 8 MiB at 64 KiB. The local
   path parses the file three times per write (`C:1075-1076`), the served path once per write.
4. A pattern or value-set member may point through an array index: the target lookup
   `input.pointer(..)` (`S:323`) resolves indexes already. Arrays stay leaves of the closed
   input walk (`S:264-283`). To close such a grant, its `input_keys` lists the enclosing array.
   One relaxation of the overlap rule (`S:184-196`): an `input_keys` pointer may lie above a
   `target` pointer when the token of the `target` pointer that follows the `input_keys` prefix
   is a canonical decimal array index (`0`, or digits without a leading `0`). It applies to
   every member form. It opens no input: such a grant admits what the same grant without that
   target member admits, less what the member refuses (`S:322-336`). An array-only rule can be
   widened later; a wider rule signed into files cannot be narrowed.
5. A pattern or value-set member is a `target` member: checked in the target loop, covered in
   the walk through `Grant::pointers` (`S:239-246`), required present, never listed in
   `input_keys`. No new `Uncovered` variant (`S:363-373`).
6. No second confirmation (`C:895-899`; D13 B: the operator session issues the grant). The
   result of `grant-standing` does not change; `list-standing` shows what a grant binds, as
   point O6 of story:local-invoke-admits-a-keyed-write-under-a-standing-grant settled.
7. The order of checks of `grant-standing` stays as 0.77.0 has it (`C:955-962`: the key and
   the keyring first). The document refusals are checked in-process on 0.77.0's `grant`, which
   takes the file's path, the state directory, a signer, the instance and a clock and is
   tested with no keyring (its test `a_grant_input_is_validated`); the CLI maps its error to
   `invalid_input` naming `input_file`, as in 0.77.0.

### Grant file

A target member holds exactly one of `value`, `pattern`, `one_of`:

```json
{"pointer": "/owner", "value": "acme"}
{"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"}
{"pointer": "/target_status", "one_of": ["10000", "10235"]}
```

- `one_of` holds 1 to 64 distinct JSON values, each compared as `value` is (`S:329`).
- `value` may be `null` and then matches `null`. `pattern` and `one_of` may not be `null`:
  a member with `"pattern": null` or `"one_of": null` is malformed. `value` gets the treatment
  `input_keys` got (`S:122-128`), so present-and-null differs from absent.
- The `typ` `b10x.connectors-standing-grants.v1+jws` and `connectors.standing-grants/1` stay,
  as `input_keys` was added without a new version (`K:56`).
- A 0.77.0 binary reads a 0.78.0 file as `Malformed` when the file holds a pattern or `one_of`
  member (`TargetMember` is `deny_unknown_fields` with a required `value`, `S:131-136`,
  `S:155-156`), or a grant whose `input_keys` pointer lies above a `target` pointer
  (`S:184-196`). Served, every write is then `unavailable` (`K:56`); local, `invoke` answers
  `approval_required` and the commands answer `invalid_configuration` and leave the file
  (`C:987-988`, `:1014-1016`; `C:1062-1066` says "does not verify", and whether a file that
  verifies but is malformed is the same refusal is read at the merged 0.77.0 head).
- 0.78.0 writes an exact member without the new keys, so a file with neither of those two
  things still reads on 0.77.0.

### The consumer's two grants

No new flag: `grant-standing` takes a document (`C:964-968`).

```
connectors approvals grant-standing --connector jira --instance <instance> --input-file transition.json
connectors approvals grant-standing --connector jira --instance <instance> --input-file comment.json
```

```json
{"operation": "issue.transition.run",
 "target": [{"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
            {"pointer": "/target_status", "one_of": ["10000","10235","3","10234","10223","10001"]}],
 "input_keys": ["/current_status", "/body/transition/id"],
 "not_after_unix_ms": "<expiry>", "max_per_hour": 500}
```

```json
{"operation": "issue.comment.add",
 "target": [{"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
            {"pointer": "/body/body/content/0/content/0/text", "pattern": "^Release review of (?s:.*)$"}],
 "input_keys": ["/body/body/content", "/body/body/type", "/body/body/version"],
 "not_after_unix_ms": "<expiry>", "max_per_hour": 100}
```

- `<expiry>` is the Unix time in milliseconds the operator picks; 0.77.0 refuses a missing or
  zero one. How far ahead it may lie has no bound in 0.77.0 and gets none here (Out of scope).
- The text pointer is read from `adapters/catalog/providers/jira/operations.json:56-58` and
  `adapters/catalog/tests/jira_issue_writes.rs:98-100`, `:444-447`.
- Status ids are strings in `adapters/catalog/tests/jira_transition_run.rs:39-41`; the declared
  type was not read.
- The closed transition grant keeps out `body.fields` and `body.update`
  (`operations.json:33`).
- The comment grant bounds the first text node only: what follows it (more paragraphs, links,
  mentions, media) is free. The docs page says so. 100 per hour is a figure for 80 tickets with
  one comment each, not a measurement.

### Files

- `crates/connectors-host/Cargo.toml` (`regex = "1"`);
  `crates/connectors-host/tests/fixtures/whole_value_patterns.json` (new);
  `crates/connectors-catalog/tests/whole_value_patterns.rs` (new, a test only) and its `mod`
  line in `crates/connectors-catalog/tests/main.rs`, the registry of that crate's one test
  binary; no production file of `connectors-catalog` or `connectors-core` changes. The test
  calls the public `connectors_catalog::inventory::pattern_matches`
  (`crates/connectors-catalog/src/inventory.rs:177`).
- `crates/connectors-host/src/local/standing.rs`: `TargetMember` (`S:131-136`), `parse` and
  the overlap rule (`S:155-196`), `covering` (`S:322-336`), the limits (`S:56-57`).
- Model: `ess/domains/standing_approvals.yaml:35-42`, `:53-56`, `:330-340`.
- Served contract: `K:56`; `contracts/operations/v1alpha1/semantics.md:152`.
- Local contract: `C:880`, `:955-986`, `:1004-1007`;
  `contracts/operations/v1alpha1/semantics.md:163`;
  `contracts/service/local-mutations.md:336-346`.
- Record: Decision point 1 of architecture-decision-record:standing-scoped-approval
  (`:43-44`), and its `decides` relation to this story, through `aep`.
- Docs: `docs/running-services.md:251` (served), `docs/local-approvals.md` (local).
- `CHANGELOG.md`, `Cargo.lock` (one new edge, `connectors-host` to `regex`).
- Generated: `crates/connectors-build/generated/metadata-emissions/inputs.json`, which digests
  the files under `ess/domains` (story:standing-grant-closes-the-input-to-listed-members
  regenerates it for the same reason).
- Tests: `crates/connectors-host/tests/http_standing_grant_adversary.rs`. Not changed:
  `apps/connectors/tests/standing_grant.rs`, which 0.77.0 creates (absent at 43bda33b08d).
- Not changed, to confirm at the merged 0.77.0 head: `apps/connectors/spec/cli.yaml` and
  `ess/domains/cli.yaml` (no command, flag or result member is added).

## Acceptance

Matching, in `covering`, which the served and the local placement both call
(`crates/connectors-host/src/local/service.rs:466`). "Refused" below is the `Uncovered`
variant `OtherTarget`, and `MissingMember` where the member is absent (`S:363-373`). What a
caller sees for either is in statement 5a.

1. Under `{"pointer": "/k", "pattern": "^DEV-[0-9]+$"}` the input value `DEV-12` is covered;
   `DEV-12x`, `xDEV-12`, `dev-12` and `DEV-12` followed by a newline are refused.
2. Under the same member the number 12, `null`, `true` and an array are refused.
3. Under `{"pointer": "/k", "pattern": "^a$|^b$"}` the values `a` and `b` are covered; `ab` and
   `a`, newline, `b` are refused.
4. Under `{"pointer": "/k", "one_of": ["10000", "10235"]}` the string `10000` is covered; the
   string `10001` and the number 10000 are refused.
5. Under `{"pointer": "/k", "value": null}` an input whose `/k` is `null` is covered; an input
   without `/k`, and the values 0, `""`, `false` and `"null"`, are refused.

5a. Through the local `admit`, a write whose `/issueIdOrKey` is `OPS-12` under the transition
    grant of this story is `approval_required`, and the use store holds no row afterwards.
    Through the served host, the same write is refused with the code that
    `a_write_to_another_target_is_refused_with_no_request` asserts, and the provider double
    saw no request.

The grant document. Each refusal below is the error of 0.77.0's in-process `grant` that the
CLI maps to `invalid_input` naming `input_file` (its test `a_grant_input_is_validated`), with
a test signer and a temporary state directory, and the grant file's bytes are unchanged.

6. A pattern is refused when it does not compile (`^(a$`), lacks the leading `^`
   (`DEV-[0-9]+$`), lacks the trailing `$` (`^DEV-[0-9]+`), is 257 bytes long, or compiles to
   more than the size limit (the test names a pattern of at most 256 bytes that does, and one
   that compiles to less and is accepted; both are found red-first by the implementation).
7. A `one_of` is refused when it is empty, holds 65 values, or holds one value twice.
8. A member is refused when it holds two of `value`, `pattern`, `one_of` (including
   `"value": null` beside a `pattern`), none of them, `"pattern": null`, or `"one_of": null`.
9. On a file that holds 63 pattern members in unexpired grants, a grant with one pattern
   member is accepted and a grant with two is refused. On a file that holds 64 pattern members
   of which 4 are in an expired grant, a grant with 4 pattern members is accepted and the
   expired grant is gone.
10. A grant whose `input_keys` holds `/a/list` and whose `target` holds `/a/list/0/text` is
    accepted. Refused, as in 0.77.0: `input_keys` `/a` with `target` `/a/b`; `input_keys` `/a`
    with `target` `/a/01`; an `input_keys` pointer equal to a `target` pointer; a `target`
    pointer above an `input_keys` pointer; two `input_keys` pointers of which one lies above
    the other; and, on a grant with `input_keys`, two `target` pointers of which one lies
    above the other.
10a. `list` returns a grant's pattern member with its `pattern` key and its value-set member
    with its `one_of` key, as written.
11. A served grant file that holds any member or grant refused in 6 to 10 makes the file
    unreadable: every keyed write is `unavailable`.

The cap, the expiry and revocation.

12. With `max_per_hour` 500 on the transition grant, 500 writes to 500 different tickets
    within one hour are admitted and the next write, to any ticket, is `rate_limited` with the
    `retry_after_seconds` an exact-value grant gives for the same use times (the control in the
    same test, with the test's clock).
13. A replay of an admitted write under a pattern grant takes no use: after it, the count of
    uses is what it was.
14. A write the pattern grant covers is `approval_required` once the clock's upper bound has
    passed `not_after`, and once `revoke-standing <grant id>` has removed the grant.

Older files and older binaries.

15. A grant file signed by the 0.77.0 binary with exact members only (a fixture committed
    after 0.77.0 is merged, with the public key it verifies under) verifies under 0.78.0, and
    the input that file's 0.77.0 test covers is covered.
16. 0.78.0 serializes the grants of that fixture to the payload bytes the fixture holds.
17. `K` and `C` each say that a file with a pattern member, a `one_of` member, or an
    `input_keys` pointer above a `target` pointer is unreadable to 0.77.0, and name the refusal
    of their placement.

The consumer.

18. Under the transition grant of this story, `{issueIdOrKey: "DEV-12", current_status: "3",
    target_status: "10234", body: {transition: {id: "31"}}}` is covered. Refused: the same
    with `issueIdOrKey` `OPS-12`; with `target_status` `10002`; with `/body/fields` present;
    with `/body/update` present.
19. Under the comment grant of this story, a comment on `DEV-12` whose document has one
    paragraph with the text `Release review of 2.46.8` is covered, and so is one with a second
    paragraph. Refused: a first text of `Note: Release review of 2.46.8`; `issueIdOrKey`
    `OPS-12`; `/body/visibility` present; an empty `content` array.

The record, the texts and the gate.

20. Decision point 1 of architecture-decision-record:standing-scoped-approval names the three
    member forms, and the record has the relation `decides:
    story:standing-grant-binds-a-pattern-or-a-value-set`; `aep plan artifact validate` prints
    `valid`.
21. `rg -n 'one_of' ess/domains/standing_approvals.yaml contracts/service/compatibility.md
    contracts/cli/v1alpha1/semantics.md contracts/service/local-mutations.md
    docs/running-services.md docs/local-approvals.md CHANGELOG.md` finds a line in each file;
    `C`, `K` and `ess/domains/standing_approvals.yaml` each hold: whole-value, case-sensitive
    matching with Unicode-aware classes; the array-index relaxation; and the three bounds (a
    pattern of at most 256 bytes, the compiled-size limit with its figure, at most 64 pattern
    members per file, counted after expired grants are dropped), in `K` in the file-bounds
    sentence of line 56.
21a. `git diff --name-only <merged 0.77.0 head>` lists neither `ess/domains/cli.yaml` nor
    `apps/connectors/spec/cli.yaml`, and no file under `crates/connectors-core` or
    `crates/connectors-catalog/src`.
22. `docs/local-approvals.md` says the comment grant bounds the first text node only.
23. With `CONNECTORS_ESS` set to the pinned `ess`, the metadata emissions are regenerated and
    `cargo test --locked -p connectors-build --bin connectors-build
    the_committed_metadata_emissions_are_current` exits 0.
24. `cargo nextest run -p connectors-host -E 'test(standing)'`, `cargo nextest run -p
    connectors-catalog -E 'test(whole_value_patterns::)'` and `cargo nextest run -p connectors
    -E 'binary(standing_grant)'` each exit 0 within 120 s with no test over 60 s; `cargo clippy
    --workspace --all-targets -- -D warnings` exits 0; the change adds no `.py` file.
25. A file of 16 grants with 4 pattern members each, 64 patterns of which each compiles to
    just under the size limit (the pattern is found by the implementation and named in the
    test), parses in under 1 s in a debug build; the contract states the memory ceiling of one
    parse as 64 times the two limits.
26. Every vector of `whole_value_patterns.json` gives the same answer in the host's function
    and in the catalog's `pattern_matches`.

## Tests

In-process in `local::standing::tests` unless named; each estimated under 1 s; none starts a
server; none needs a keyring except where said.

- `a_pattern_member_matches_the_whole_string` (1, 2, 3)
- `a_value_set_member_admits_listed_values_only` (4)
- `a_null_value_member_matches_null_only` (5)
- `a_grant_document_with_a_bad_member_is_refused` (6, 7, 8), on `grant`
- `a_file_holds_at_most_64_pattern_members` (9), on `grant` against a file in a temporary
  state directory
- `input_keys_may_lie_above_a_target_through_an_array_index_only` (10)
- `list_shows_the_three_member_forms` (10a)
- `a_write_outside_a_pattern_grant_counts_no_use` (5a, local half)
- `one_cap_counts_every_ticket_a_pattern_matches` (12, 13)
- `a_pattern_grant_ends_at_not_after_and_at_revocation` (14)
- `the_jira_grants_cover_the_consumers_inputs` (18, 19)
- `a_file_of_64_patterns_parses_within_a_second` (25)
- `signed_tests::a_0_77_file_verifies_and_is_reproduced` (15, 16)
- `the_shared_pattern_vectors_hold` in `local::standing::tests`, and
  `whole_value_patterns::the_shared_pattern_vectors_hold` in the `main` test binary of
  `connectors-catalog` (file `crates/connectors-catalog/tests/whole_value_patterns.rs`)
  (26; the vectors include those of 1, 3 and the form refusals of 6)
- in `crates/connectors-host/tests/http_standing_grant_adversary.rs`, two plain `#[test]`s:
  `a_served_file_with_a_bad_member_is_unavailable` (11) and
  `a_served_write_outside_a_pattern_grant_reaches_no_provider` (5a, served half)
- no new test in `apps/connectors/tests/standing_grant.rs`: the mapping of `grant`'s error to
  `invalid_input` and exit 2 is 0.77.0's test and does not change

## Order and shared files

- Starts when 0.77.0 is merged. It shares with the two 0.77.0 stories: `standing.rs`,
  `ess/domains/standing_approvals.yaml`, `C`, `K`,
  `contracts/operations/v1alpha1/semantics.md`, `contracts/service/local-mutations.md`,
  `docs/running-services.md`, `docs/local-approvals.md`,
  `crates/connectors-host/tests/http_standing_grant_adversary.rs`,
  `crates/connectors-host/Cargo.toml` and `CHANGELOG.md`.
- It shares `Cargo.lock` and the generated `inputs.json` with the Entity Runtime 0.33.1 pin
  (branch `er-0331-pin`). Whichever of the two lands second rebases onto `main`, then
  regenerates `Cargo.lock` and the emissions last, and adds its `CHANGELOG.md` line after the
  rebase.
- It shares no file with the branch `story/read-uses-in-memory`.

## Out of scope

- A pattern or value set on a member that is not a `target` member.
- A second confirmation at `grant-standing`, and a change of its result.
- A bound on how far ahead `not_after_unix_ms` may lie (out of scope in 0.77.0 too).
- An `input_keys` pointer above a `target` pointer through an object member.
- Who may run `grant-standing`: any process of the install's user can; a key outside that
  user's reach is a change of architecture and Timo's.

Estimate for 0.78.0, not a measurement: 2026-11-24 about 18:00 CET.
