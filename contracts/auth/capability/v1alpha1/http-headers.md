# Receiver-owned HTTP headers

**Status:** proposed configuration binding; current HttpConfig does not implement
`http.extra_headers`. This supplements [auth.capability](semantics.md).

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
