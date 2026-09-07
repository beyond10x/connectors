---
format: aep.planning-md/1
id: story:preserve-operation-only-remediation
kind: story
status: active
title: Preserve ordinary generated service invocation through remediation preflight
relations:
- derived_from: epic:enforced-authority
scope:
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/src/remediation_tests.rs
- confidence: cited
  path: crates/connectors-runtime/src/service_bundle.rs
revision: 5
---
## Outcome

Generated services that own operations and bind reviewed Connection references continue through ordinary grant admission when they do not expose the separate Connection API or remediation interface.

## Observed regression

A real Devcenter k3d composition of Connectors 0.7 starts and describes generated operations, but invocation returns HTTP 403 under both v2 and v3. The registry's new remediation fallback selects only a backend owning both the Connection API and operation. Generated service wrappers intentionally own operations without a Connection API, so the optional unsupported path becomes a refusal before ordinary grant checks. Workbench coordination and cleanup consequently fail.

## Acceptance

Route unsupported remediation metadata to the unique ordinary operation owner only when there is no competing Connection owner. Preserve split-owner rejection, ambiguity refusal, and no fallback after a claimed remediation refusal. No credential is read, operation dispatched or grant admission bypassed by metadata routing. Exercise the real composed service wrapper and preserve adversarial routing cases. Rebuild and verify the actual downstream workbench after the runtime correction.

## Authorization

Continue the operator-authorized 0.7 adoption and local deployment repair. This bounded existing-interface regression proceeds through draft, proposed and active. It introduces no new entity or authorization surface.

## Verification before downstream rebuild

All 37 runtime library tests pass, including the actual service bundle wrapper in both registration orders and the expanded adversarial routing matrix. Split owners, ambiguous owners and claimed remediation refusals preserve their existing results. All-target runtime clippy and formatting pass. The correction changes predicate selection only; ordinary description, binding, grant and approval checks remain unchanged.

A separate read-only client review confirmed that current hosted backend RateLimited errors use validated HTTP 200 envelopes, so the BFF v3 rate mapping is reachable. Literal HTTP 429 support is not required to fix this routing failure; no client transport or authorization status behavior is changed here.
