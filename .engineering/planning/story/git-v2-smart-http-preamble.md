---
format: aep.planning-md/1
id: story:git-v2-smart-http-preamble
kind: story
status: implemented
title: Accept GitLab Smart HTTP discovery framing before Git v2 capabilities
relations:
- derived_from: story:git-protocol-v2
scope:
- confidence: cited
  path: crates/integration-gitlab/src/git_fetch_v2.rs
- confidence: cited
  path: crates/integration-gitlab/tests/support/git_fetch_http.rs
revision: 7
---
## Outcome

An authenticated coding workspace discovers its admitted GitLab Git v2 source when the provider prefixes the capability advertisement with the Smart HTTP service announcement and a flush packet. This preserves O1 governed reach; this component store has no local vision artifacts, so the relation is to its existing Git v2 story.

## Evidence

Authenticated headless verification after the OAuth-header repair still failed at Substrate Git fetch. A normal Identity-derived broker session reproduced HTTP 502 on initial v2 discovery. A bounded direct read returned HTTP 200 and the expected advertisement content type, with an exact `# service=git-upload-pack` packet, flush, then `version 2` and capabilities. The current capabilities parser instead requires version 2 as its first packet. The real-Git fixture uses git-http-backend without that provider preamble.

## Acceptance

Both bare v2 advertisements and the exact optional upload-pack service announcement plus flush succeed. Wrong services, repeated preambles, missing flush, legacy advertisements, truncated or oversized frames and trailing garbage stay refused. The real Git HTTP fixture includes the observed framing, fails with the original production parser, and completes bounded v2 materialization after repair. Focused negative cases preserve fail-closed behavior.

Current authorization, exact project/branch/commit binding, depth, byte/request limits and single-use source authority remain enforced. Public source, tests and planning contain no credentials, deployment coordinates or captured private provider payloads.

Full repository CI and independent review precede the composed Connectors-only image update. Devcenter's existing Projects recovery story coordinates immutable deployment and authenticated headless file and Agent acceptance. This is an existing-contract framing correction, with no new entity, permission, operation or wire version.

## Scope

Cited: crates/integration-gitlab/src/git_fetch_v2.rs and crates/integration-gitlab/tests/support/git_fetch_http.rs.

## Source delivery evidence

Connectors PR16 merged the reviewed source 53ba51fb744e223e220523ff49f313e1d23d8673 in bot-authored merge 1daa209012ec3d8f9df8b559aa3900cbd7af969a. The merge tree is byte-identical to the tested source. CI34020120274 completed successfully: all twelve workspace gates, final repository checks and all four platform CLI builds passed. Its publish job was correctly skipped for a branch validation run.

The focused regression first failed the real Git HTTP fixture with HTTP 502 and the positive parser case with Refused under the original parser. The correction then passed all 42 GitLab cases, formatting and Clippy. The independent review is retained unchanged. Devcenter PR51 consumes this exact source in composed Connectors 0.8.27; its full gate and affected image build passed in CI34020408981, and publication CI34021522883 is in progress. Hosted browser acceptance is owned by the downstream Projects recovery story and is not claimed here.
