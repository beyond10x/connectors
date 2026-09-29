---
format: aep.planning-md/3
id: release-plan:connectors-0-13-0
kind: release-plan
status: implemented
title: 'Connectors 0.13.0: ess/13 and the newest ESS and AEP'
revision: 2
transitions:
- {from: "draft", to: "implemented", at: "2026-09-29T00:53:05Z", actor: "human:timo", revision: 2}
---
## Scope

Release 0.13.0 of `beyond10x/connectors`. The specification and the planning tools move to their
newest releases; no connector behaviour changes.

- Every ESS system (`ess/`, `adapters/*/spec/ess`) declares source format `ess/13`; 123 emitted
  payload fields in `ess/domains` gain explicit sources (103 from command input, 20 generated).
- ESS crates and toolchain 0.31.0 -> 0.35.0 (`a86579e1`); AEP 0.59.2 -> 0.60.0 (`99ad0b51`) for the
  toolchain, CI and the store's protocols pin, with the findings patch rebased.
- The regenerated Entity Runtime definitions keep all 55 commands.

## Evidence

- `ess specify validate` exits 0 for all 7 systems; synthesis 279 scenarios, 43 authored.
- `connectors-build gate --msrv` (2026-09-27, two runs): every step exits 0 except the isolated
  `connectors-host` test `final_audit_recovery_preserves_every_live_business_result`.
  - That test fails identically on the unchanged 0.12.0 base: its recovery is capped at 250 ms
    (`crates/connectors-host/src/local/audit.rs:212`), and nine injected faults exceed it on a
    loaded machine. Run alone at load about 7 it passed 4 of 5.
  - One run also failed `concurrent_duplicate_prepare_has_one_live_receipt_and_original_correlation`
    with `MetadataUnavailable`; alone it passed 5 of 5 on both this tree and the 0.12.0 base, and
    the second gate run passed the whole workspace test step.
- `aep plan artifact validate` with AEP 0.60.0: valid.

## Known limitation carried

The recovery-bound test above is timing-dependent on a shared machine at this release and at 0.12.0.
