---
format: aep.planning-md/3
id: story:guarded-write-credential-refusal-says-repair
kind: story
status: draft
title: A guarded write refused for its stored credential reports repair_connection
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Source

Adversary pass 2 on the OAuth refresh profile (review-result:adversary-oauth2-refresh-profile-pass-2,
F1): a committed write refused with 401, or a preflight refused with `invalid_grant`, returns
`InvalidCredential`, which the host maps to `Origin::Host`
(`crates/connectors-host/src/local/owner.rs:163-168`); `mutation::Failure::next_action`
(`mutation.rs:60-79`) then says `retry_status`, and a preflight reports stage `admission`. Reads
report `repair_connection` since story:catalog-oauth2-refresh-profile; writes were deliberately left
at `retry_status` and the contract says so.

## Acceptance

- `InvalidCredential` from a write's preflight or commit carries the provider origin, and
  `next_action` is `repair_connection` for an unauthorized refusal classified `NotAttempted` or
  `Refused`.
- `adversary2_write_credential_refusal_names_repair_like_a_read` in
  `adapters/catalog/tests/local_runtime/oauth2_refresh_adversary_pass2.rs` asserts `repair_connection`
  (flipped from today's `retry_status`).
- `contracts/cli/v1alpha1/semantics.md` drops the write exception.
