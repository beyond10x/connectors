# 20. Bounded Git protocol v2

Status: accepted for implementation on 2026-09-05, under `story:git-protocol-v2`.

## Decision

Extend [design 19](19-read-only-git-fetch-sessions.md)'s internal byte plane with the
[Git protocol v2](https://git-scm.com/docs/protocol-v2) command grammar. The existing
`b10x.connector-git-fetch.v1` control payload and legacy clients remain compatible. An absent
`Git-Protocol` selects legacy transport; exactly one `Git-Protocol: version=2` selects v2.
Other values, duplicate protocol or source-authority headers, and protocol changes within one
source-capability generation are refused. Substrate releases the v2-only consumer after this
additive proxy release; Workspace and Devcenter then advance their consumer pins.

V2 capability discovery must receive an actual `version 2` response from the provider. Parse a
bounded advertisement and require `ls-refs`, `fetch` with `shallow`, and SHA-1 object format
(explicit or the protocol default). Advertise only `ls-refs`, `fetch=shallow`, `object-format=sha1`
and a proxy agent string. Client agent metadata is consumed at the proxy, never forwarded.
Unsupported upstream capabilities do not cross the boundary. V2 commands require completed
discovery; missing negotiation never silently falls back to legacy.

## Reference and pack requests

The proxy accepts one strictly framed `ls-refs` or `fetch` command per bounded request. Command
capabilities admit only one each of a printable agent string and `object-format=sha1`.
Unknown or duplicate capabilities, delimiters, terminators, trailing packets and expanding
arguments are refused before provider egress.

`ls-refs` accepts `symrefs`, `peel` and bounded reference prefixes. The proxy replaces all prefixes
with `HEAD` and the admitted `refs/heads/<branch>`, always requesting upstream symrefs. Prefixes
are an optimization, not an access boundary: parse every returned packet, retain only those two
references, and verify their exact admitted commit and HEAD symref target. Duplicate admitted rows,
moved tips, malformed/truncated framing and unexpected attributes fail closed. Hold only the two
selected rows and a bounded packet buffer; disclose nothing until the complete response validates.
Honor the client's `symrefs` preference when emitting the verified HEAD row.

`fetch` admits only the initial shallow-clone subset: one exact `want`, one positive `deepen`
within the session's maximum of 50, `done`, and optional `thin-pack`, `ofs-delta`, and `no-progress`.
It refuses incremental negotiation (`have`, `shallow`, relative/time/exclusion deepening), tags,
alternate wants, `want-ref`, filters, server options, sideband-all and packfile URLs. Canonical
upstream requests carry only the admitted arguments and `Git-Protocol: version=2`.

The pack response is validated and streamed with bounded memory: optional `shallow-info` containing
SHA-1 shallow boundaries, then `packfile` with normal sideband data and a final flush. Unrequested
sections, unshallow rows, fatal sideband errors, malformed packets and incomplete final framing
fail closed. Substrate still verifies the resulting pack, exact commit and checkout durability.

## Authority, budgets and lifetime

Every exchange retains current Connection/Grant and project/default-tip admission, credential
custody, redirect refusal, hard session expiry, transport deadlines, 32-request and one-GiB aggregate
upstream byte limits. Reference commands leave the final fetch available. Only a complete pack
response spends the one fetch; interrupted or refused pack transfers invalidate that source
capability and require the existing idempotent control retry, which rotates authority. Dropped
streams cannot leave an in-flight fetch reusable. A new capability generation resets negotiation.

## Connection reuse

The server additionally reuses provider HTTP connections in a cache owned by one immutable egress
policy, keyed by Connection/Connect Session identity, scheme, hostname, port and the complete
currently admitted DNS address set. Every request still resolves and validates addresses before
cache lookup. Changed or mixed/private-refused answers cannot select an old client. Bound the
cache to 64 entries, expire unused entries after 60 seconds, and retain at most two idle sockets
per origin in each client. Eviction drops only the cache reference; in-flight responses retain
their client. Credentials remain request headers, timeouts remain request-specific, and redirects,
ambient proxies and cookies stay disabled. This requires no permission or DNS-result cache.
The independent project and branch admission reads run concurrently on each exchange; both
must still validate, with project-first refusal precedence preserved.

## Verification

Exercise a real Git client through the HTTP proxy against Git's HTTP backend, assert v2 on both
legs, depth 50 and exact checkout, and retain legacy coverage. Add malformed/expanding request,
reference filtering, changed tip, protocol binding, revocation, expiry and interrupted-stream
tests. Compare upstream discovery bytes on one repository with many references: v2 must transfer
only targeted rows, while the legacy provider advertises its full reference set. Report measured
bytes separately from elapsed clone time; v2 adds a command round trip and is not a universal
latency guarantee.
