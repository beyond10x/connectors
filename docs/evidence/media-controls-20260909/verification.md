# Media-control classification audit

This is a specification-only audit for E23 and E24. It does not execute a
session, media transport, provider, authority ledger, or runtime codec.

## Failing baseline at `8e1836c`

| Case | Expected classification | Baseline observation | Result |
|---|---|---|---|
| MC-01 | `sip.dial` and `sip.sessions.list` are ordinary operations; `close`, `signal`, `interrupt`, offer, accept and reject are duplex protocol messages unless separately admitted | `adapters/sip/design.md` placed every name in one `## Operation map` | fail |
| MC-02 | One-shot RTVBP establishment redemption belongs to `inbound-verifier`; an auth-evidence column contains only names declared by `auth.evidence` | The native RTVBP documents named `inbound-verifier`, but `contracts/README.md` put `redemption ledger` in the evidence column | fail |
| MC-03 | Media dependency rows include every contract used by native declarations | SIP declared `sip.sessions.list` and `static_entry`, while the root media row omitted `datasource.records` and `auth.acquisition` | fail |

The baseline was inspected with read-only `rg` over the SIP and RTVBP designs,
their native contracts, the shared session/media/auth documents, and the root
contract index. The corrected audit and coordinator-owned index patch are
recorded below after the semantic edits.

## Corrected textual audit

| Case | Corrected specification result | Status |
|---|---|---|
| MC-01 | SIP exposes only `sip.dial` and `sip.sessions.list` as ordinary operations. Session lifecycle and media controls are separately classified as duplex protocol messages in both native bindings and the composition. | pass |
| MC-02 | RTVBP selects no `auth.evidence` check. Atomic `(iss,jti)` redemption is named as `inbound-verifier` capability state; the shared index correction is supplied as a coordinator patch. | pass after coordinator patch |
| MC-03 | SIP now declares `datasource.records` for `sip.sessions.list` and `auth.acquisition` for `static_entry`; the shared index correction is supplied as a coordinator patch. | pass after coordinator patch |

The shared media contract now gives `hold` and `transfer` an explicit
reserved/refused disposition. Unsupported controls cannot be advertised,
negotiated or accepted. The historical interrupt source is the complete
`../connectors/docs/design/05-native-sip-and-rtvbp.md:315-318` range at
`81459ac4`; SIP response evidence uses
`../connectors/providers/b10x.toml:741-771`.

The SIP configuration JSON is explicitly illustrative. Shared PCM, queue,
revocation and teardown numbers are separately identified as selected semantic
requirements; the composition does not turn illustrative deployment values into
defaults. The selected protected-entry path uses versioned custody and a managed
connection while retaining the operator-admitted trunk target as configuration.

This is a textual specification audit. It does not claim a native codec,
provider runtime, timed-cutoff fixture or live PBX conformance run.
