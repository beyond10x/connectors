---
format: aep.planning-md/3
id: review-result:claude-epic-acceptance-r3-20261022
kind: review-result
status: active
title: 'Plan-critic acceptance, round 3: connectors with Claude'
summary: needs-revision, 2 blockers; addressed
relations:
- reviews: story:mcp-stdio-runtime-branch
- reviews: story:standing-write-approval-for-unattended-agents
- reviews: story:claude-connects-to-the-mcp-server
- reviews: story:mounted-custody-kind-secret-volume
revision: 1
---
# Plan-critic, acceptance lens, round 3: connectors with Claude

**Verdict: needs-revision. 8 findings, 2 blockers. All are addressed in the revisions of 2026-10-22 after round 3.**

| Story | Finding | Disposition |
|---|---|---|
| mcp-stdio-runtime-branch (blocker) | acceptance named no test or scenario | eight named scenarios in `apps/connectors/tests/local_mcp_server.rs`, run in `cargo test` under the `mounted_secret` jail; the Secret Service tests move to the jail or get an `ignored.rs` entry |
| standing-write (blocker) | no invalid-signature scenario | `a_grant_with_an_invalid_signature_is_refused_with_no_request` |
| claude-connects | cited `mcp-stdio-runtime-branch.md:44-45`, now scope lines | cites the runtime story's named scenarios |
| claude-connects | the "recorded fixture server" does not exist on the branch | a loopback Jira listener with `jira-search.json`, as `revalidate_transient.rs` serves its provider, under the `mounted_secret` jail, not ignored |
| standing-write | the input that moves the target had no scenario | `an_input_that_moves_the_target_is_refused_with_no_request` |
| standing-write | `no_standing_grant_exists_unless_the_operator_issued_one` was vacuous; no test files | `a_send_with_no_grant_file_is_refused_approval_required_with_no_request`; `apps/connectors/tests/standing_approval.rs`; the five per-input suite files named |
| standing-write | the design steps had no acceptance | ADR record `accepted`; the ESS domain validates in `task check` |
| mounted-custody-kind-secret-volume | two outcomes in one line; no file or job | two lines: the test in `mounted_custody.rs` passes in `kind-custody.yml`; the guide records each fact as observed in that run |
