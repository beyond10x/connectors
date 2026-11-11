---
format: aep.planning-md/3
id: review-result:owner-invoke-effects-adversary-r1-20261111
kind: review-result
status: active
title: 'Adversary pass 1: owner-invoke-frame-effects-boundary'
relations:
- reviews: story:owner-invoke-frame-effects-boundary
revision: 1
---
# Adversary pass 1: owner-invoke-frame-effects-boundary

Reviewed commit 02b0bb2f7c on impl/owner-invoke-frame-effects-boundary (base 485ba7999b). Four
cases written in a scratch test file, all green; module run `local::owner::governed::tests` 41
passed, exit 0, one test thread through the build slot.

Held: no MCP-path caller reaches the plain Invoke frame (tools/call uses the bounded projected
read only); approvals and write frames never send Invoke; the owner reads the projection policy
only for governed jobs; the bounded and projected reads refuse declared effects before any
provider request; the commit changes no behaviour.

Not held: the commit's test could not fail (the declaration reached only an in-memory policy and
its base was a not_found refusal); local-mcp-stdio.md said approval governs a write on the invoke
frame; owner.md did not say that a GovernedRead without projection_revision skips the effects
rule; "grant alone" misdescribed what admit_invoke enforces, and a blank line was missing.

All four are fixed in the follow-up commit: the test declares effects in the protected curation
on a connected fixture, and the two documents are corrected.

```findings
[
{"file":"crates/connectors-host/src/local/owner/governed/tests/bounded_reads.rs","line":248,"category":"test-gap","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the declaration only reaches an in-memory Policy that admit_invoke never sees, and base is a NotFound refusal, so the test passes under an effects-holding admit_invoke and never shows a declared-effects read being served on invoke"},
{"file":"docs/local-mcp-stdio.md","line":72,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"says approval governs a write on the plain invoke frame, but that frame has no approval field and refuses every non-read requirement unsupported, contradicting owner.md"},
{"file":"contracts/cli/v1alpha1/owner.md","line":131,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a GovernedRead frame with projection_revision omitted skips both policy admission and the effects rule and dispatches what the projected read refuses, yet the paragraph names only invoke as a non-boundary"},
{"file":"contracts/cli/v1alpha1/owner.md","line":134,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"'grant alone' lists non-grant refusals and omits the connection, input and custody admission admit_invoke also enforces, and the paragraph has no trailing blank line"}
]
```
