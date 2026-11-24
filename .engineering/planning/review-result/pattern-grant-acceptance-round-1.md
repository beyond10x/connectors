---
format: aep.planning-md/3
id: review-result:pattern-grant-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


Findings are in the block below. There are 13: 1 blocker, 12 warnings.

Cited lines I checked at 43bda33b08d:
- `S:131-136`, `155-156`, `184-196`, `239-246`, `264-283`, `323`, `329`, `363-373`, `106`, `122-128` say what the story says.
- `S:184-196` does refuse an `input_keys` pointer above a target today, because `other.starts_with(one)` holds. `S:323` resolves array indexes.
- `K:56` backs the claim that an older host answers every write `unavailable`.
- `C:895-899` backs "no second confirmation".
- `inventory.rs:182` and `201-225` match the catalog rule. That rule is `^` first, an unescaped `$` last, and a wrap as `\A(?:p)\z`.
- One citation is wrong: `inventory.rs:204` is an unbounded `HashMap` behind a `OnceLock`, not the "bounded cache" the Design names (line 166).

The main problems:
- **Acceptance 9 has no test that can pass it.** The only CLI test runs with no keyring. `C:~960` says `grant-standing` needs the keyring to sign, so that test can only see a refusal, not `result.grant`.
- **Acceptance 3's CLI refusal depends on an order the contract doesn't fix.** The contract doesn't say whether input validation or the keyring check comes first.
- **Two tests have no acceptance and no bound.** `a_file_full_of_patterns_parses_in_bounded_time` and the plain adversary `#[test]` name no number and no acceptance statement.
- **Design promises with no acceptance:**
  - the record revision;
  - the new overlap-rule text;
  - the docs, model and CHANGELOG;
  - the regenerated `inputs.json` (the sibling story admits this goes red);
  - the test commands with the 60 s and 120 s bounds.
- **The consumer's own admission is never checked.** Acceptance 7 lists refusals and a second-paragraph admission, but never that a real transition or comment input is admitted.
- **Several statements join two or more outcomes.** Acceptances 3, 4, 5, 7 and 9 do. Acceptances 4 and 6 also use open ranges ("1 to 3,600", "nothing else") where an injected clock gives an exact value.

What I read: 4 artifacts: the story, the record's text through the story, and the two 0.77.0 stories (acceptance sections). I read the cited `S:`, `C:` and `K:` lines at 43bda33b08d and the `ess` yaml lines. I ran `git show`, `git grep`, `awk` and `sed`.

What I could not establish:
- Whether 0.77.0 local code answers `approval_required` for a file that verifies but is malformed. `C:1062-1066` says only "does not verify", and the local path isn't at the commit.
- Whether `ess/domains/cli.yaml:87-91` and `120-128` are unchanged in the working tree; I read them at the commit.
- Design coupling and scope, which are out of my lane.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 195
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 9 (grant-standing prints result.grant) cannot be checked by any listed test: grant_standing_refuses_a_bad_pattern runs one CLI process with no keyring (line 216) and grant-standing signs, so it refuses without the keyring (C:~960); name an in-process test in local::standing::tests that asserts the grant the 'grant' call returns and the list view of one member per form, or say how a CLI process gets a keyring, and drop (9) from the keyringless test"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 178
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 joins four independent refusals and a served half in one statement, and its CLI half assumes input validation runs before the keyring check, which C:957-962 does not order; split it into one statement per refusal family, say the validation precedes the keyring (and add that to C), and make the served half 'a grant file holding such a member makes the file unavailable' rather than 'the same document'"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 180
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'over 256 bytes or the size limit' names a limit the Design calls a proposal and not measured (line 73), so no witness exists; measure it, then name one pattern under 256 bytes that is refused and one just under the limit that is admitted, and list the 'two' case with a null value plus a pattern as a pair that is refused"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 174
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptances 1 and 2 name no witness for the whole-string rule the Design promises (lines 68-70); add refused inputs 'DEV-12x', 'xDEV-12', 'DEV-12\\n' and a number or null under a pattern, plus an admitted 'DEV-12', and a top-level '|' pattern that still matches only whole values"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 211
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "test a_file_full_of_patterns_parses_in_bounded_time maps to no acceptance and names no bound, though the Design promises a linear engine, a 256-byte cap and a size_limit; add an acceptance that names the file (256 grants, 16 patterns each, 256 bytes) and a time under the 60 s test bound, or drop the test"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 213
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the plain #[test] in http_standing_grant_adversary.rs has no name and no acceptance number, so nothing says what it asserts; name it and tie it to acceptance 3's served half and to a served write refused with the code a_write_to_another_target_is_refused_with_no_request asserts"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 183
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 holds two outcomes (the 501st write, the replay) and gives retry_after_seconds as the range 1 to 3,600, which any value in range passes; split the replay into its own statement and, with the test's injected clock, name the exact retry_after_seconds for a first use at t0 and the 501st at t0+N, as the 0.77.0 story's test 7 does"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 186
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 5 joins 'verifies and covers as before' with 'writes the same bytes', and 'same bytes from the same grants' is unchecked because grant mints a fresh UUID; split it, name the fixture's signing key and the document that is re-signed, and state that the fixture is signed by the 0.77.0 binary and is committed after 0.77.0 merges (the dependency in line 221)"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 188
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 ends 'and nothing else', an unbounded claim; list the refused inputs (member absent, 0, \"\", false, \"null\") and the admitted one, and say that {value:null} with a pattern or one_of is a member with two forms and is refused at parse"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 189
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 never states that the consumer's real inputs are admitted (a transition {issueIdOrKey DEV-12, current_status, target_status 10234, body.transition.id} under the transition grant, a one-paragraph 'Release review of ...' comment under the comment grant), which is the Outcome; add both admissions as a control, and name /body/update beside /body/fields since the Design keeps both out (line 137)"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 192
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 relaxes one overlap case and names no refusal for the others the rule still holds (two target pointers with one above the other under input_keys, two input_keys pointers with one above the other, a target above a key), and says 'malformed' without the surface; list those as still Malformed at parse and invalid_input at grant-standing"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 197
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 10 checks only the older-binary sentence in two contracts, and the Design's other file changes (record Decision point 1, ess standing_approvals.yaml and cli.yaml, the overlap-rule text at K:56 and C:982-984, running-services.md, local-approvals.md, CHANGELOG, regenerated inputs.json, Cargo.lock) have none; add rg-checkable statements as the sibling story does (its lines 7-9), a statement that the generated checks are green after regeneration, the test commands that exit 0 within 120 s with each test under 60 s, and that no .py file is added"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 166
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design promises 'a bounded cache ... as inventory.rs:204 does', but inventory.rs:204 is an unbounded HashMap in a OnceLock, so the bound has no model and no acceptance; state the bound (entries) and add an acceptance or test that a file of 256 distinct patterns evicts or fails closed, or say the cache is unbounded and capped by the 256-grant file"
```
