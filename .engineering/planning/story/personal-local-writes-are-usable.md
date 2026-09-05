---
format: aep.planning-md/1
id: story:personal-local-writes-are-usable
kind: story
status: active
title: Local write fixes reach ordinary CLI sessions
summary: Deliver the existing mixed-grant discovery and approval-metadata fix, then verify Slack writes through the ordinary local CLI and daemon.
tags:
- priority-first
- ready
- wave-cli
refs:
- provider: git
  reference: 5285a3bb47a47274b4b049f71a881bffd4b56146
relations:
- derived_from: epic:local-product
- informed_by: story:explicit-target-never-implicit
- informed_by: story:one-placement-several-credentials
scope:
- confidence: inferred
  path: crates/connectors-runtime/tests/local_catalog_writes.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: docs/architecture/deployment.md
- confidence: cited
  path: docs/guides/connect-slack.md
revision: 30
---

## Objective

O1: governed reach. An individual who has explicitly enabled a Slack connection for writes must be able to discover and perform an authorized post through the ordinary local CLI workflow.

## Confirmed incident

The operator's 2026-09-05 session was reviewed on 2026-09-06. Before invocation, searching for Slack writes failed with `protocol: operation protocol identity or framing is invalid`; describing `slack-chat-post-message` failed with `not_granted` and `this Connection's grant admits reads only; connect with --allow writes to raise it`, despite a later configured Slack connection already allowing writes.

The session isolated two defects in the catalog adapter: description chose the first provider connection instead of an admitting connection, and mutating operations declared approval metadata that the protocol rejected. Commit `5285a3bb47a47274b4b049f71a881bffd4b56146` fixes both and includes the existing local AEP record `story:catalog-write-discovery`. Its recorded validation includes all 35 focused tests and all repository gate workspaces. Using a separately built binary and a dedicated local daemon, the session subsequently received Slack `ok: true` for the operator-authorized brief on 2026-09-05 at 17:51 UTC. This was a successful historical send; no new message was sent during this investigation.

## Remaining gap

The fix is committed in the primary local checkout but is absent from remote main as inspected on 2026-09-06. The ordinary installed CLI reports version 0.6.0; its usual local daemon still returns the same protocol error for `operation search --query slack-chat-post-message --limit 25`, with explicit local config and state-root selection. The config already contains a writable Slack connection alongside read-only connections. The historical dedicated-daemon success therefore does not establish that an ordinary local session has the fix.

This is a delivery and ordinary-workflow verification follow-up to the existing implementation. Read-only enrollment by default does not explain this incident, and enabling writes globally is not its remedy.

## Acceptance

With the existing fix published and installed through the normal delivery path, an ordinary local CLI session can search and describe a Slack post across mixed read-only and writable connections and complete an explicitly authorized fixture post through its selected writable connection, while selection of a read-only connection remains refused with actionable guidance.

## Original scope requirements

- Review and reuse the existing catalog fix and its AEP record; reconcile its implementation evidence and delivery status when that commit is integrated.
- Verify the selected CLI executable and serving daemon both contain the fix, and provide the supported update/restart path for an already-running local daemon.
- Exercise search, description and invocation against a local fake provider through the normal daemon path, with the read-only connection before and after the writable one.
- Preserve selected-connection grants and truthful required-approval metadata; document the actual supported local authorization workflow.
- Confirm missing grants, incompatible or stale runtime and provider refusals remain distinguishable enough for a CLI user to take the correct next step.
- Do not repeat a live Slack post as a regression probe; any later live verification needs a specifically authorized destination and message.

## Readiness

Ready for implementation at the operator's request on 2026-09-06. Highest CLI priority, ahead of the ten previously selected wave-cli stories; their original ordering is preserved in their Readiness sections. This is additional work, not a replacement for one of those ten. The proposed lifecycle state and ready tag record scheduling intent, not delivery completion. Scope paths are conservative inferred follow-up surfaces and must be narrowed after the existing fix is reviewed.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/connectors-runtime/tests/local_catalog_writes.rs` — inferred.
- `docs/guides/connect-slack.md` — cited.
- `docs/architecture/deployment.md` — cited.

High confidence. Existing catalog fix 5285a3bb is an ancestor. Add real CatalogBackend/BackendRegistry/local-socket regression with fake egress, both connection orders and explicit read-only refusal. Local owner authorization plus selected connection grant remains the boundary; Required metadata does not justify fabricated hosted approval proof. Coordinator owns source installation, selected daemon restart and read-only installed-binary verification; no new live Slack send.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 1 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## Delivery evidence in progress

The coordinator installed the source at opening commit 4769ce32a331bb8daddaf066b69459c4148fb749 with the supported cargo install path (debug profile, locked dependencies). Install exit 0; installed connectors reports 0.6.5. Installed binary and workspace build have identical SHA256 483879ea4d9d3a8522c9e163b4559ca520f4d99afb134e522aebfd59e1e7edef.

The normal local daemon still failed write search before restart (protocol, exit 1), proving client installation alone did not fix it. Its graceful restart initially refused a home-directory cwd because the state root must be outside cwd; restart with the original cwd succeeded. An immediate query raced readiness and was unreachable; after ready, write search and describe both exited 0. The separate brain daemon reproduced the same protocol failure before its graceful replacement; afterwards search and describe both exited 0. Both show the admitted writable Slack connection and required approval posture. No live provider invocation or Slack message was sent.

Long-lived daemon logs were moved outside disposable wave scratch into each daemon's existing state root as daemon-wave-20260906.log; their processes retain the open files. Final integration must reinstall the reviewed bytes before closing the delivery story. Socket fixture invocation and adversarial review subsequently passed, and the complete integration gate is recorded in the wave runbook.

The fixture found actionable guidance missing when all bindings are read-only. Coordinator reviewed and applied the implementor's exact two-string catalog-guidance.patch, extending scope to integration-catalog/src/lib.rs. It names an intentional allow_writes policy update and daemon restart; the existing enrollment command only sets the flag on a new instance. No grant policy was changed by this wave.
