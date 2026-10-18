---
format: aep.planning-md/3
id: review-result:invoke-definition-reuse-security-round-1
kind: review-result
status: active
title: Security review, round 1, on reusing a stored credential
summary: 'needs-change: a token endpoint on another host was not compared (blocker); target not required to be a definition instance (note); refused dispatch not cancelled (note)'
relations:
- reviews: story:invoke-definition-reuse-credential
revision: 1
---
# Security review, round 1, on reusing a stored credential

Security review, round 1, of the green change for story:invoke-definition-reuse-credential on
branch unit/invoke-definition-reuse-credential at 4a9d3cb01c, run 2026-10-18. Verdict:
needs-change, one introduced blocker and two introduced notes.

Case added: adapters/catalog/tests/security_reuse_credential.rs
`security_reuse_a_token_endpoint_on_another_host_is_not_the_same_authentication`, red at
4a9d3cb01c: two catalog bootstraps at the same API base with the shipped client-credentials
profile, one exchanging at the API's origin and one at another host, have the same provider
authority and the same profile authentication, so the second is a reuse candidate for the first's
client secret.

1. blocker, introduced: reuse compared only the provider authority (API base) and the profile
   revision, which leaves out the token endpoint, token CA, CA, admitted hosts and request prefix
   (D12 point 1). A definition naming a client-credentials token endpoint on another host would
   have the stored client secret sent there by the copy's own validation, before any identity
   comparison.
2. note, introduced: the owner did not require the reuse target to be an invoked definition's
   instance; a local request could copy into a configured instance.
3. note, introduced: a refused `dispatch_read` left the source's read use to lapse instead of
   cancelling it.

Held: no material in replies, errors or the question; `same_connector` excludes legacy entries and
self; the adapter id is compared; the configuration is re-read three times; the target binding is
the owner's own; identity is compared before `prepare`; revoked, Stale, expired and moved sources
are excluded; the source is written only through its read use; the copy has its own claim and
scope; the acquisition is failed on every error; OAuth acquisition and read-only custody are
refused on both sides.

```findings
[
{"file":"crates/connectors-host/src/local/registry/reuse.rs","line":726,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"reuse compares only the API base and the profile revision, so a definition whose client-credentials token_url is on another host is offered, and its validation sends the source's client secret to that host"},
{"file":"crates/connectors-host/src/local/owner/transport.rs","line":446,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the owner does not check that the reuse target is a definition instance, so it also copies into a configured instance, which the CLI contract does not describe"},
{"file":"crates/connectors-host/src/local/owner/transport.rs","line":527,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a failed dispatch_read leaves the source's read use to lapse on its own instead of cancelling it"}
]
```
