---
format: aep.planning-md/3
id: review-result:standing-grant-local-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 310
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The comment fix at ess/domains/transport.yaml:18 is not in the source or the record and belongs to story:windows-mcp-tool-call-answers-timeout. Delete section 'Follows another story', and drop ess/domains/transport.yaml (frontmatter line 79) from scope."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 61
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Scope lists crates/connectors-host/src/local/owner/transport.rs (inferred) but Design step 3 (line 169) says there is no transport change and line 277 says 'unless transport.rs moves'. Remove the path, or state the transport change in Design."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 181
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The served host learning input_keys, with acceptance 20 (line 258), is a served-placement change. The source asks only for the local call, and decisions 1 and 4 list no closed input. Out of scope (line 285) says the served host's behaviour is excluded. Either keep Grant local (a local wrapper) and drop acceptance 20, or make input_keys, acceptance 6, 18 and 20 a first story that this one depends on. That is the cut I would make: 26 lines then split cleanly into a closed-input story and the commands, file, admission and answer story."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 189
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "revoke-standing writing the id into standing-grants.sqlite3 (acceptance 13) is a second revocation authority that no decision names. Decision 5 reads only the file. Put it to the operator as a lettered question with a default, or drop it with acceptance 13 and state the put-back-of-an-older-file limit. Do not keep it under 'Decided in drafting'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 146
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Source: grant-standing takes 'required not_after and max_per_hour'. Design lists the members but no refusal of a grant without them, and no acceptance line tests it. Add 'grant-standing refuses a missing or zero not_after_unix_ms or max_per_hour as invalid_input' and a test line."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 234
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Source: the consumer wants the exit status for not_attempted and 'no axis'. Only acceptance 9 asserts exit 1 (for unknown). Add to acceptances 2, 10 and 11: exit 1, stderr, '.error.data.mutation.classification' not_attempted, and no 'axis' member in '.error.data'."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 208
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Docs that state a local write needs a proof are missing from scope and Design item 2: README.md:51, docs/catalog-slack.md:310 and :784, docs/local-approvals.md (intro), docs/local-approval-keys.md (the key now signs a second payload), website/docs/connectors/slack.md (generated from docs/catalog-slack.md), website/docs/guides/guarded-write.md:11 is in scope. Add these paths. adapters/catalog/tests/local_runtime/slack_standing.rs is new: write 'new' beside it."
- file: .engineering/planning/story/standing-write-approval-for-unattended-agents.md
  line: 65
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Only the depends_on edge changed, so both stories still claim one outcome. Narrow the draft: Outcome 43-45 and summary to the container mount; Decided line 56 (ledger grant id, now answer and uses store); Design first 65-74 (ADR accepted, ESS exists, contract text moved); Shape 87-88 and 93-94 (commands, verification); Review 98-100 and Acceptance 104-116 (adversary, ESS, four scenarios that no story now owns, assign them); tests 121-132 reduce to those the jail changes (changed byte, absent file, replaced or removed file); line 123-124 drops thread_ts (O4); 134-139 duplicates acceptance 21; scope 14-38 drops CHANGELOG, cli.yaml, approvals.rs, approval_issuance.rs, semantics.md, approval_issuers.yaml and standing_approvals.yaml."
```

Out of my lane: the unrecorded approval-clock prerequisite (Design 179, not in the source) and the ordering of `Uses::take` against the preflight.
