---
format: aep.planning-md/1
id: review-result:git-v2-preamble-pass-1
kind: review-result
status: active
title: Independent review of optional Git v2 Smart HTTP preamble
relations:
- reviews: story:git-v2-smart-http-preamble
revision: 1
---
# Independent review: Git v2 smart HTTP preamble

Recommendation: approve the source change, subject to the repository gate and deployment verification owned by the integrating agent. No actionable correctness, security, or regression findings were identified in the reviewed diff.

Scope: the uncommitted changes to `crates/integration-gitlab/src/git_fetch_v2.rs` and `crates/integration-gitlab/tests/support/git_fetch_http.rs`, against base commit `a61e0a748be58ad03cf1907a48f9fefbb4423434`. Planning records were outside this review. The reviewed source/test diff matches the implementor's captured `final.patch` byte for byte; the comparison and `git diff --check` both returned exit 0.

The production parser at `crates/integration-gitlab/src/git_fetch_v2.rs:154` admits one additional framing form: exactly one packet containing `# service=git-upload-pack\n`, immediately followed by a flush, before the existing v2 advertisement. It still requires the v2 version packet. Wrong services, altered preamble payloads, extra preambles, missing separators, and non-flush separators cannot reach capability acceptance. The new branch rejoins the existing capability parser and normalized response builder, preserving the shallow-fetch requirement, SHA-1 restriction, duplicate-capability rejection, and capability filtering. Provider preamble bytes are not forwarded into the normalized client response.

Security and resource limits remain intact. The preamble uses the existing packet reader at `crates/integration-gitlab/src/git_fetch_v2.rs:449`; packet and buffered-byte limits still apply. It adds only one fixed-size packet and one flush, rather than an unbounded prefix-skipping loop. The capability byte bound is unchanged, and provider bytes continue through the session-budgeted stream in `crates/integration-gitlab/src/git_fetch.rs:485`. Complete-response validation still precedes setting `v2_negotiated` or exposing capabilities. This patch does not modify credential presentation, destination selection, admission authority, branch filtering, fetch arguments, or pack parsing.

The regression coverage is meaningful. `crates/integration-gitlab/src/git_fetch_v2.rs:698` checks equal normalized output for bare and wrapped v2 discovery across nine chunk sizes. The negative cases cover wrong services and payload shapes, missing or wrong flushes, duplicate framing, every truncated prefix, and invalid trailing frames. Existing invalid version, missing capability, unsupported object format, duplicate capability, and oversized capability cases are also tested with the wrapper. The existing optional single response-end handling remains unchanged.

The HTTP fixture at `crates/integration-gitlab/tests/support/git_fetch_http.rs:247` wraps a genuine `git http-backend` v2 discovery response at its HTTP boundary. It does not replace the production parser or synthesize a successful fetch response. The surrounding test at `crates/integration-gitlab/tests/support/git_fetch_http.rs:367` exercises a real Git client through the production HTTPS router and broker, then checks the expected commit, file contents, depth 50, shallow state, reference filtering, and absence of credential persistence in Git configuration.

I inspected the implementor's red/green evidence. With the changed fixture and original production parser, the real Git fetch failed with HTTP 502, exit 101. The new positive parser case independently failed with `Refused`, exit 101. With the production fix, the supplied GitLab test log reports 42 passed and 0 failed, including the HTTP interoperability test; the discovery comparison reports 280511 legacy bytes versus 388 v2 capability/reference bytes and three provider exchanges. Supplied formatting and all-target Clippy results passed with exit 0. The captured red patch contains test changes rather than the production acceptance branch.

Limitations: this review independently inspected source, tests, captured patches, logs, and exit files; it did not rebuild or rerun tests. The supplied passing log establishes real Git interoperability, but does not establish execution of the fixture's optional Substrate/gix branch. The wrapper models the observed provider framing; this review did not make a live provider request. Full repository CI and authenticated deployed file-tree verification remain necessary before declaring the deployed issue resolved.
