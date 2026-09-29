---
format: aep.planning-md/3
id: review-result:adversary-provider-refusal-stage-pass-1
kind: review-result
status: active
title: Adversary pass 1 on provider refusal stage
relations:
- reviews: story:provider-refusal-reports-dispatch-stage
revision: 1
---
unit: story:provider-refusal-reports-dispatch-stage, uncommitted tree wave0929b-stage on 6902cfef3
verdict: NEEDS-CHANGE
cases: executed 26→29, red 3
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary (empty)
needs-coordinator: none

Cases in adapters/kubernetes/tests/stage_origin_adversary.rs (all red, left Provider right Host): namespace outside
the configured scope (lib.rs:307), resource kind outside the configured scope (lib.rs:459), hosts.discover switched
off (lib.rs:572). Each refusal happens before any request and now reads dispatch/request_permission.
Could not break: owner/CLI build skew (origin defaults to Host), host admission refusals, host-side from_service,
transport rewrites keep origin, connection probe 403 reads dispatch.

Coordinator routing: blocker to the implementor with the rule that only a refusal carrying an upstream answer is
Provider; the doc drift fixed with it.

```findings
[
  {"file": "adapters/kubernetes/src/lib.rs", "line": 307, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Kubernetes configured-scope refusals (namespace :307, kind :459, host discovery :572) raised in the adapter child before any provider request become Origin::Provider and read stage=dispatch/request_permission instead of admission"},
  {"file": "contracts/cli/v1alpha1/semantics.md", "line": 477, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the new text says a connection probe's upstream 404 keeps service_code=not_found with next_action=none, but catalog probe_failure and kubernetes auth map 404 to Protocol, which the CLI reports as upstream_protocol with retry_explicitly"}
]
```
