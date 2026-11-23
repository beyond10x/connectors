---
format: aep.planning-md/3
id: review-result:pattern-grant-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


**Round 1 findings**
1. **Consumer inputs and the comment prefix.** Resolved. Acceptance 18 and 19 state the admitted inputs and the refusals, including `OPS-12`, status `10002` and a first text of `Note: Release review…`.
2. **Record revision.** Resolved. Acceptance 20 checks Decision point 1 and the `decides` relation.
3. **`not_after` and "long-lived".** Resolved. Acceptance 14 covers expiry and revocation for a pattern grant. The examples carry `<expiry>`, and the bound on how far ahead `not_after` may lie is in Out of scope (line 358).
4. **`result.grant`.** Resolved. It is dropped in Design point 6 and Out of scope (line 357), and the Files section no longer names `cli.yaml`.
5. **Downgrade claim.** Resolved. Lines 154-162 and Acceptance 17 name the `input_keys`-above-`target` case. The relaxation is also narrowed to array indexes (Design point 4, Acceptance 10), which is tighter than I asked.

**New items checked against the parents**
- **Shared pattern function in `connectors-core`.** Not traceable to the pick, the record or the consumer. It edits `connectors-catalog` production code and adds `regex` to `connectors-core`. Finding 1.
- **Document check before the key and the keyring.** Not traceable either. It reverses the order the 0.77.0 contract states today (`contracts/cli/v1alpha1/semantics.md:960-962`: the commands "need the instance's approval key and refuse an instance without one"). The only reason given is a cheaper test. Finding 2.
- **Cap of 64 pattern members per file.** I did not flag it. It traces to the consumer's pattern use, since the consumer needs 3 patterns, and to bounding parse cost. Acceptance 9 and 25 claim it. Out of my lane: Acceptance 21 does not require `C` and `K` to state the cap.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 109
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design point 2 moves the pattern rule into connectors-core and edits connectors-catalog inventory.rs:182-225, which no parent asks for (the pick asks for a pattern member on a grant). Fix: keep the function in connectors-host and leave connectors-catalog unchanged, or put the shared function to the operator as a lettered question with a default and move it out of Files and Acceptance 24 until answered."
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 134
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design point 7 changes the 0.77.0 order of checks of grant-standing (the contract says it needs the approval key and refuses without one) so a bad document is invalid_input on an instance without a key; no parent asks for it and the stated gain is a cheaper test. Fix: drop point 7 and its contract sentence in Acceptance 21, and test the refusals in 6 to 9 in-process on the request check that 0.77.0 splits from grant, and in the CLI test with the key fixture 0.77.0 uses; or put the reorder to the operator as a lettered question with a default."
```
