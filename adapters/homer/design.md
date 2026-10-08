# homer adapter design

**Status:** proposed; not implemented. Records model: [spec/ess](spec/ess/system.yaml),
domain `connectors_homer.capture`. No contract, upstream pin, fixture or runtime exists.

Homer is a SIP capture server (homer-ref:3) that serves searches over captured SIP
messages and RTCP reports (homer-ref:43,58). Connectors would only read Homer's API. This
is distinct from [SIP](../sip/design.md), which places calls and lists the sessions it owns.

Sources: the fluxplane-plugin skill reference `references/homer.md` (plugin homer 0.4.1,
cited `homer-ref:<line>`, not in this repository) and
[the parity page](../../docs/fluxplane-plugin-parity.md) `## homer` and unit U17. No Homer
API document was read or fetched.

## Operations (unit U17, wave W4)

| fluxplane operation | calls | reads | selection in `capture.yaml` |
|---|---:|---|---|
| `homer.call.list` | 13 (+1 undeclared) | `Call`: caller, callee; status, duration, route UNMAPPED | `CallListSelection` |
| `homer.call.show` | 10 | `SipMessage` flow of the requested calls | `CallFlowSelection` |
| `homer.test` | 4 | no record | none: `connections revalidate` covers `*.test` (parity:81-83) |
| `homer.call.qos` | 3 | `StreamQuality` per stream; loss, jitter, MOS UNMAPPED | `CallQualitySelection` |
| `homer.search` | 2 | `SipMessage` | `MessageSearchSelection` |

All five are read-only (homer-ref:36-64). Calls are from parity:369-373 and :730.
Connectors operation ids are chosen with the route.

## Route

The parity page leaves it open: a catalog provider if a Homer Swagger document can be
pinned, otherwise a native adapter (parity:792). What decides it:

| Decider | Catalog provider | Native adapter | Known? |
|---|---|---|---|
| A pinnable Homer API document | required | optional | not checked; nothing fetched |
| Username/password exchanged for an API JWT (homer-ref:14) | no such scheme: the engine places bearer, basic, signing, header or query credentials (`adapters/catalog/design.md:87-91`) | the adapter can own the exchange | both routes need a reviewed auth profile first |
| Calls grouped by Call-ID (homer-ref:38) | only if the API serves calls | adapter code if it serves messages only | UNMAPPED |
| E-model MOS from loss and jitter (homer-ref:43) | not a generic HTTP mapping | adapter code | computed by the reader, not stated as a Homer field |
| Ordered flow, SDP annotation, plain-text ladder (homer-ref:48) | raw records only | adapter code | the ladder is presentation and does not lower a verdict (parity:75-76) |

**Recommendation: a native adapter `connectors.homer`.** The auth exchange is new work on
either route. Beyond it, `call.list` and `call.qos` (17 of the 33 calls) need grouping or
computation that a generic HTTP mapping does not do; response interpretation is an
adapter obligation (`adapters/loki/design.md:79`). Revisit the catalog route only if a
document is pinned, the API serves calls and per-stream quality as records, and the
engine gains the login exchange.

## Authentication

| Proposed profile | Fields | Acquisition | Presented as | Evidence |
|---|---|---|---|---|
| `homer.login` | `username`, `password` (sensitive), both required (homer-ref:15-18) | `static_entry` of two fields, as `sip.trunk` does (`adapters/sip/design.md:21`) | an API JWT obtained with them (homer-ref:14) | `connections revalidate` re-running a reachability and login probe (homer-ref:63) |

Not expressible today: the `auth.profile` vocabulary has no credential-exchange flow
(`contracts/auth/profile/v1alpha1/semantics.md:65-66`), and `oauth2_password` is reserved
and refused (:75); nothing says Homer's login is OAuth.

UNMAPPED: the login endpoint and request shape. UNMAPPED: the JWT's lifetime and
renewal. UNMAPPED: the profile's purpose and subject; a web UI account, personal or
service, is not stated (homer-ref:14).

## Out of scope

- `homer.alias.list`, `homer.call.analyze`, `homer.pcap.export`: 0 calls, not planned
  until used (parity:841). Multi-leg correlation is composition (parity:375); PCAP is a
  binary export (parity:376).
- Any write, HEP ingestion or capture-agent configuration: no source operation does it.
- Any link between a `sip.sessions.list` session and a Homer call.
