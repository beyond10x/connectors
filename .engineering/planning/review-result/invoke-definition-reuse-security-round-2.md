---
format: aep.planning-md/3
id: review-result:invoke-definition-reuse-security-round-2
kind: review-result
status: active
title: Security review, round 2, on reusing a stored credential
summary: 'needs-change: target rule ignored where the source sends a client secret (warning); source CA vs platform roots (note)'
relations:
- reviews: story:invoke-definition-reuse-credential
revision: 1
---
# Security review, round 2, on reusing a stored credential

Security review, round 2, of the fixes for story:invoke-definition-reuse-credential at 9cc91a24f
(4a9d3cb01c..9cc91a24f), run 2026-10-18. Verdict: needs-change, one introduced warning and one
introduced note. Round-1 F2 (definition target) and F3 (refused dispatch cancels) confirmed fixed;
the ReuseGuard checks fence, generation, material, Stale and revocation inside the publishing
commit.

Case added: adapters/catalog/tests/security_reuse_source_token_origin.rs
`security_reuse_a_source_secret_is_never_sent_to_an_origin_its_connection_does_not_send_it_to`,
red at 9cc91a24f: a pinned client-credentials source exchanging its secret only at an identity
provider off its API origin was offered to a definition exchanging it at its API origin.

1. warning, introduced: the target rule compared the definition's URLs with its own API origin,
   never with where the source sends the credential, so the copy's validation could post a client
   secret to a host its source never sends it to. Not reachable with a shipped connector.
2. note, introduced: a copy of a source pinned to its own CA file is validated under the platform
   roots, a different trust set than the source's.

Held: the origin parser (userinfo, port, scheme, case); no redirects; the credential only in a
header; probe paths relative; request prefix on the same host; NATIVE_KEYS excludes hosts and CA
files.

```findings
[
{"file":"crates/connectors-host/src/local/definitions.rs","line":221,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the reuse target rule checks only the definition's URLs against its own API origin, so a client secret the source sends only to an off-origin identity provider is posted to the API host by the copy's validation"},
{"file":"crates/connectors-host/src/local/definitions.rs","line":237,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a source pinned to its own ca_file is copied to a definition validated under platform roots, a wider trust set than the source's"}
]
```
