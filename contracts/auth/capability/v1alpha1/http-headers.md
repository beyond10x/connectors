# Receiver-owned HTTP headers

**Status:** proposed configuration binding; current HttpConfig does not implement
`http.extra_headers`. One slice is implemented: a trusted composition may fix **one**
nonsecret header on a native port, `ScopedHttp::with_fixed_header`
(`crates/connectors-host/src/http.rs`), whose value it derives from the admitted configuration
(first use: Grafana IRM's `X-Grafana-URL`, `adapters/grafana/src/local.rs`). This supplements
[auth.capability](semantics.md).

## Implemented slice: one composition-fixed header

- The composition names the header and its value once, when it builds the port; the business
  adapter holds only `AuthenticatedHttp` and can neither add, change nor remove it. No caller
  input, provider response or adapter-built request reaches the value.
- It cannot take the port's credential header name, or any name in these classes, each
  compared case-insensitively; a trailing `*` is every name with that prefix:
  - credential and cookie: `authorization`, `proxy-authorization`, `cookie`;
  - routing and forwarding: `host`, `forwarded`, `x-forwarded-*`, `x-real-ip`;
  - method override: `x-http-method-override`, `x-http-method`, `x-method-override`;
  - identification: `user-agent`;
  - framing and negotiation: `transfer-encoding`, `connection`, `proxy-connection`,
    `keep-alive`, `te`, `trailer`, `expect`, `accept`, `accept-encoding`;
  - content: `content-*`;
  - upgrade: `upgrade`, `sec-websocket-*`.

  Its value is 1–1,024 bytes of visible ASCII and spaces. A second fixed header on the same
  port is refused. Each refusal is `invalid_input` at composition, before any request.
- Every request of the port carries it once, beside the credential: reads, reads sent as a
  POST, the probe capability and writes derived from the port. The credential-free form
  exchange (`post_form`) carries neither.
- The value is part of the admitted configuration, so a change is a new configuration
  revision.

## Implemented: the client's `User-Agent`

Every request a port makes, the credential-free form and assertion exchanges included, carries
`User-Agent: connectors/<version>`, set by the transport's HTTP client; some providers refuse a
request without one (GitHub's REST API). Neither a port's credential header nor a fixed header
may take that name, each refused as `invalid_input` at composition, so no configuration or
definition replaces it.

What stays proposed: operator-declared `http.extra_headers`, more than one header,
case-variant duplicate detection across several declared headers, and the mediated-parent
placement below.

An admitted direct connection may declare fixed extra request headers only when
its selected strict configuration schema and transport capability support them.
The adapter binding owns the allowed native header names and meaning. The receiver
owns their values and admitted configuration revision.

Query input, caller headers, provider response data and adapter-built request
headers cannot choose, append or override configured values. The host installs
exactly the admitted values at dispatch. Configuration validation rejects duplicate
names ignoring ASCII case and conflicts with credential placement or
transport-owned headers. Unsupported headers/configuration are refused rather than
ignored. Changes activate a new admitted configuration revision; cached results
and continuations cannot cross that revision.

A provider tenancy header is distinct from the caller's verified application
tenant/realm and supplies no application authority. Diagnostics must not expose
credential-bearing or private values.

For a mediated connection, the admitted parent route owns downstream header
placement. The child consumes the shared mediated capability and supplies no direct
`extra_headers` override. A requested binding guarantee that the selected route
cannot establish is refused without a direct fallback. Concrete parent/child
pairings belong to compositions, not to the shared contract or child implementation.

Verification must cover conflicting case variants, caller and adapter overrides,
configuration revision changes, credential/transport conflicts and missing mediated
guarantees. These are binding obligations, not results from executed HTTP tests.
