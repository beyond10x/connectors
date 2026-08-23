---
id: S-043
title: "Admission proofs become unforgeable"
pillar: Platform
status: ready
priority: 1
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [domain, service, integrations]
---

# Admission proofs become unforgeable

## Goal

Remove `AdmittedOperation::from_grant_decision` — a public constructor over caller-supplied
strings — and make `AdmittedOperation` reachable only through two honestly named, sealed paths:
a `GrantDecision` proof (hosted; constructor lands with S-044, a module-private placeholder seals
the type now) and a local-owner admission for personal placements speaking over the owner's own
socket. Every current caller moves to the local-owner path explicitly; behavior does not change.

## Acceptance

- `from_grant_decision` no longer exists; grepping for it finds only history.
- `AdmittedOperation` has no public constructor; construction sites outside `crates/domain` use
  the named local-owner path, and its rustdoc states what it asserts and where it must not be used
  (hosted request handling).
- The workspace gate is green with no test weakened.
