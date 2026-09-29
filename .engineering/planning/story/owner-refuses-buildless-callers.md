---
format: aep.planning-md/3
id: story:owner-refuses-buildless-callers
kind: story
status: draft
title: An owner refuses work from a CLI that never named its build
relations:
- serves: vision:independent-contract-adapters
- informed_by: review-result:adversary-owner-build-handshake-pass-2
revision: 1
---
## Problem

An owner serves work requests on a connection greeted without `build`, so an older CLI (a real 0.15.1 CLI was measured, exit 0) runs against a newer owner after a rollback, the reverse of the owner-build-handshake incident. `contracts/cli/v1alpha1/owner.md` keeps build-less callers compatible on purpose. Found by review-result:adversary-owner-build-handshake-pass-2 (finding 1, pre-existing, crates/connectors-host/src/local/owner/transport.rs:827).

## Acceptance

- Decide whether the owner refuses every request except the build probe and shutdown on a connection greeted without `build`; record the decision in owner.md.
- If it refuses: `adversary2_an_owner_refuses_work_from_a_caller_that_never_named_its_build` (apps/connectors/tests/local_cli.rs) asserts the refusal instead of today's service.
