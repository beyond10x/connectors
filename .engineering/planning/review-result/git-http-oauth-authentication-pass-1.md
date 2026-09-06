---
format: aep.planning-md/1
id: review-result:git-http-oauth-authentication-pass-1
kind: review-result
status: active
title: Independent Git Smart HTTP authentication review
relations:
- reviews: story:git-http-oauth-authentication
revision: 1
---
# Independent Git Smart HTTP authentication review

## Findings

No blocking findings. Severity: none; no concrete defect identified in the scoped patch. This is a bounded source and regression review, not deployment or authenticated browser acceptance.

Reviewed 2026-09-06 against base `caf9e36de3dd0a6f7508eee5b67abf7c19872a90`. The seven-file binary diff SHA256 is `3b96482dba1bc938b667ad7243cb772f9526d2f11a77e770f000a3be14044d03`. Scope: `crates/integration-gitlab/Cargo.toml`, `src/git_fetch.rs`, `src/transport.rs`, `tests/support/git_fetch.rs`, `tests/support/git_fetch_http.rs` within that crate, and the two `crates/connectors-runtime/Cargo.lock` and `crates/connectors-cli/Cargo.lock` files. Coordinator-owned planning and future publication changes are outside this hash.

## Authentication and authority

`crates/integration-gitlab/src/transport.rs:41` correctly produces standard Base64 HTTP Basic authentication for username `oauth2` and the admitted token as password. The temporary combined cleartext credential uses `Zeroizing`; the result enters only the existing outbound authorization header. This matches [GitLab's documented Git-over-HTTPS token convention](https://docs.gitlab.com/api/oauth2/#access-git-over-https-with-access-token). `bearer_headers` remains unchanged and still serves REST admission at `crates/integration-gitlab/src/git_fetch.rs:643`, consistent with [GitLab REST OAuth authentication](https://docs.gitlab.com/api/rest/authentication/#oauth-20-tokens).

The only production call-site replacement is the shared Git discovery/upload-pack exchange at `crates/integration-gitlab/src/git_fetch.rs:432`. Current principal/connection/grant checks, revalidation of numeric project identity, provider default branch and exact commit (`:577`), generation/source-authority checks, v2 negotiation, exact wants/depth, request budgets and streamed-byte accounting remain intact. No extra provider operation, broader ref selection, credential scope or runtime authority is introduced.

The upstream URL still derives from the admitted provider origin and project path, without credentials. Production egress retains HTTPS, destination authorization, disabled redirects, response bounds and fixed error classes (`crates/server/src/egress.rs:406`; `crates/service/src/egress.rs`). The broker forwards only its selected protocol/content headers and the upstream credential; it does not forward the downstream source-authorization header. Upstream refusal bodies and authentication challenges are not exposed as errors. No new credential-bearing log, persisted configuration or control response was added.

## Observed verification

I independently inspected all seven diffs, the surrounding authority/egress code, official documentation, the complete deciding test and retained logs. My local `git diff --check` passed. I did not modify source or planning, compile another tree, or rerun the implementor's tests.

- `deciding-red.log`: the strengthened real-HTTP fixture with unchanged production source fails its legacy discovery probe when the provider rejects Bearer; the broker returns its existing 404 refusal. SHA256 `b573e42fd23222388d8f9b7d3faf1d5ac9f55333869e47d287037406909603b9`.
- `deciding-green.log`: the same fixture passes after the repair. Both Git endpoints explicitly reject Bearer, REST assertions require Bearer, and a real native Git v2 checkout passes through the production router/broker into `git http-backend`. It verifies the expected commit, depth 50, absent private refs/tags, spent session and credential-free Git configuration. Three v2 provider exchanges transfer 354 discovery bytes versus 280511 in the controlled legacy comparison. SHA256 `5a54a651a253dcb031cec801f8c934ae8f07abbefd98af3308c5a0b8509ec25c`.
- `package-tests.log`: 40 passed, zero failed or ignored; zero doctests. Existing tests cover foreign authority, current-grant/default-tip refusal, protocol framing, revocation, byte budgets and stream expiry. SHA256 `e35dbb83c2c217732c2e6b36d9ead0f8eb3c365b2a45c8bd374bfb884de9ad0d`.
- Formatting passed; all-target package Clippy with `-D warnings` completed successfully and the implementor confirmed exit 0. Clippy log SHA256 `9f0a5350461391e495945ca3e1fee52f3b6f1a8435d3822c2adfeb8efcfb973e`. Offline metadata reports exit 0 for all twelve workspaces. Only the two existing lockfiles gain the direct dependency edge to already-locked Base64 0.22.1.

These log names are relative to `.scratch/git-http-auth/` in the reviewed checkout. Test and Clippy executions are implementor evidence read by this reviewer, not executions attributed to this review.

## Limits

The fixture uses a synthetic provider and `LoopbackEgress`; REST admission responses are controlled and Git bytes come from real `git http-backend`. It does not exercise the complete production egress implementation against hosted GitLab. The optional external Substrate binary lane was not run. The complete repository gate, immutable publication, consumer rollout and authenticated headless file/Agent acceptance remain coordinator-owned and unproven by these fixtures. No fixture duration is claimed as hosted startup performance. Source was confirmed idle before finalization.

```findings
[]
```

Final immutable report; do not amend after delivery.
