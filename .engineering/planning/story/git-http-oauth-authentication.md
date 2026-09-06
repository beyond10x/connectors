---
format: aep.planning-md/1
id: story:git-http-oauth-authentication
kind: story
status: active
title: Authenticate GitLab Smart HTTP with the documented OAuth token scheme
relations:
- informed_by: story:git-protocol-v2
- derived_from: story:git-protocol-v2
scope:
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/integration-gitlab
revision: 7
---
## Outcome

A coding workspace can fetch an admitted private GitLab repository through the existing Smart HTTP proxy because upstream Git requests use GitLab's documented OAuth token authentication for Git-over-HTTP.

## Failure and evidence

The deployed proxy uses bearer_headers for both REST admission and Git discovery/upload-pack. A read-only check against an actual GitLab instance on 2026-09-06 used the same existing credential and repository: REST Bearer returned 200, Git discovery Bearer returned 401, and Git discovery with Basic authentication using username oauth2 returned 200 with protocol-v2 advertisement. No credential was printed or persisted. A post-quota coding attempt separately records workspace.git-fetch-failed after admission; the earlier quota refusal is no longer its failure.

GitLab's official OAuth documentation distinguishes API Bearer usage from Git-over-HTTP token authentication: https://docs.gitlab.com/api/oauth2/ . The affected implementation is crates/integration-gitlab/src/git_fetch.rs, where the streaming Git request currently calls bearer_headers; REST admission uses the same helper in separate calls.

## Acceptance

- Git discovery and upload-pack encode the admitted OAuth access token as the password of Basic authentication with username oauth2, solely at the existing connection-bound egress boundary and without URL credentials.
- REST admission continues to use its documented Bearer scheme. Subject, grant, connection, exact project/default-branch/commit, protocol-v2, depth, request and byte-limit checks remain enforced on every exchange.
- A deciding regression fixture refuses API Bearer authentication at the Git endpoint, accepts the provider's documented Git scheme, and proves complete bounded fetch through the proxy; the unmodified production behavior fails that fixture.
- Credential bytes remain absent from logs, errors, downstream replies, Git configuration and persisted session state. Existing revocation, cross-origin, protocol and stream-budget tests remain green.
- Required repository checks and independent review precede immutable consumer publication. The composed Devcenter Connectors image is rebuilt from the reviewed source; release/deployment verification and headless source-file acceptance are coordinated by the existing Atlas coding-workspace story.

## Scope

Cited source scope: crates/integration-gitlab/src/git_fetch.rs and crates/integration-gitlab/src/transport.rs; the affected HTTP fixture lives in crates/integration-gitlab/tests/support/git_fetch_http.rs. The integration manifest and runtime lock may change for a standard encoding dependency. No new domain entity, connector catalogue operation or identity model is introduced.

## Focused verification

The strengthened real HTTP fixture refused the predecessor's Bearer authentication at the upstream Git boundary, causing the complete clone regression to fail. With the Git-only Basic authentication correction, the same regression completes a real protocol-v2 fetch through the production HTTP router and broker, with three upstream exchanges, the exact admitted commit and a depth-50 shallow history. Its controlled many-ref fixture observed 280511 legacy discovery bytes against 354 bytes for v2 capabilities plus ls-refs; this is fixture evidence, not deployed end-to-end timing.

All 40 integration-gitlab tests pass, with no ignored tests. Formatting and strict all-target Clippy pass. Offline metadata succeeds for all twelve declared workspaces; only the runtime and CLI locks require the already-resolved base64 0.22 dependency edge. The production change is confined to the Git transport header helper and call site; REST remains Bearer and the existing authority, byte, request, depth and protocol limits are unchanged.

The full sharded repository gate, immutable consumer publication, deployment and authenticated headless file/Agent acceptance are still pending. This story remains active until its required delivery evidence is recorded.

## Full source and deployed-client compatibility evidence

Source CI34002384974 completed successfully at fe3541a6d866e84855dfdc19ec4d22a7e779b1e5: all twelve workspace shards, final catalog/documentation/ESS checks, secret scan, local-identity refusal and all four Unix release-build targets passed. The reviewed repair is on the default branch through PR15. No standalone CLI release tag was created for this source-consumption repair.

The existing optional interoperability lane also executed the production Git materialization code from the exact deployed Substrate 0.7.5 source, 64ae2ed5a888663b036cbe06515cbfd277369d58. A release-profile host test executable built with its pinned Rust toolchain passed git::materialization_tests::external_connectors_proxy_v2_fixture through the repaired Connectors router/broker: the admitted commit, 50 shallow-history entries, absent tags, no transient authority in stored Git configuration and spent broker session were verified. Both inner and outer tests actually ran and passed. The enabled branch asserts six v2 provider exchanges across native Git and gix; its unchanged final log label describes the original three-exchange native-only lane.

This fixture uses synthetic credentials, loopback TLS and real Git HTTP bytes. It establishes client compatibility, not hosted end-to-end timing, quota lifecycle or authenticated browser acceptance. Devcenter PR50 owns the composed image and delivery. Immutable publication, downstream rollout and authenticated file/Agent checks remain required, so the story stays active. Temporary build outputs and the managed read-only Substrate checkout were retired after retaining commands, hashes and logs.

## Downstream delivery evidence

Devcenter PR50 merged the reviewed source consumption at a15597c0d08527a553afe9562c8b3eaa255b2f01 after its complete gate and affected OCI builds passed in CI34003341500. The signed Connectors 0.8.26 publication passed both architecture builds and composed-candidate validation in CI34004333609. Its immutable image digest is sha256:d499c64f3a30bc2a86d3ee2b5bc887a6bbd2fb1433cd1d1f97d1b85ef7232a0a and its exact integration source remains fe3541a6d866e84855dfdc19ec4d22a7e779b1e5.

The downstream validation, atomic deployment and running-image verification jobs all succeeded. Direct observation confirmed a ready Connectors pod with the published image ID and zero restarts. No additional cloud resources were created by this authentication repair. The post-deployment headless browser reached the public sign-in entry without JavaScript or application-asset failures, but it had no authenticated user session. Hosted file-tree and Agent acceptance remain open under the Devcenter recovery story, so this story remains active. Source test evidence is retained outside the repository; task-local build output and scratch copies were removed after byte-for-byte archive verification.
