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
revision: 5
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
