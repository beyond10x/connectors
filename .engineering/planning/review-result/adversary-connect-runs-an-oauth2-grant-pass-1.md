---
format: aep.planning-md/3
id: review-result:adversary-connect-runs-an-oauth2-grant-pass-1
kind: review-result
status: active
title: 'U3 adversary pass 1: connect-runs-an-oauth2-grant'
relations:
- reviews: story:connect-runs-an-oauth2-grant
revision: 1
---
unit: story:connect-runs-an-oauth2-grant, 5ee623faa plus uncommitted adversary tests
verdict: red
cases: executed 105→108, red 3
origin: introduced 4, pre-existing 0, undecided 0
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: adapters/catalog/src/local.rs
  line: 684
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: GrantEntry reads id_token and access_token through the 8192-byte token rule, so the child refuses with InvalidInput an entry the CLI (ENTRY_LIMIT 64 KiB) and the contract row admit, after the person has already consented
- file: apps/connectors/src/local/session.rs
  line: 285
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a no-source connect on a permitted non-oauth2_grant profile under a platform binding runs custody admission and owner begin_grant instead of refusing locally, answering custody_unavailable/unlock_keyring where contracts/cli/v1alpha1/semantics.md:995-997 requires protected_entry_unavailable/select_protected_source
- file: adapters/catalog/src/local.rs
  line: 1098
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: usable_declaration relies on ServerDeclaration::validate, whose PastedCode branch only parses redirect_uri, so an http:// pasted-code redirect is accepted although the story refuses a non-HTTPS endpoint
- file: crates/connectors-host/src/local/oauth2_grant.rs
  line: 167
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the device grant takes issued_at_ms after the successful poll response, breaking the documented no-later-than-issue rule and pushing expires_at_ms past the real expiry by up to one poll round trip; the code grant takes the time before the request
```

Ignored lane: 3 of the 10 failures the implementor reported fail identically on base c6d9e7900 (cli_journey basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code, lifecycle catalog_cli_failed_repair_and_busy_stop_preserve_authority, revision_recovery a_changed_evidence_lifetime_recovers_with_a_new_connection_in_two_commands): pre-existing.
