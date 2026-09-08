# SIP dial/v1alpha1

**Status:** proposed adapter-native binding, not implemented. Shared
[operations](../../../../../contracts/operations/v1alpha1/semantics.md),
[sessions](../../../../../contracts/sessions/v1alpha1/semantics.md) and
[media](../../../../../contracts/media/v1alpha1/semantics.md) supply common obligations.

The closed native dial selector uses an opaque configured trunk alias (never
URI/host/port) and digits-only destination number. `sip.dial` is risk high,
idempotency:none, approval:required, with effects external_write, network,
send_external, session_establishment and semantic_effect human_visible.
The old public established result becomes a ready receipt only under the complete
shared session/result binding. Live dialogs never migrate or reattach.

### 4.1 Dial effect and ready-session result

This profile selects **confirmed establishment of the exact admitted SIP leg** as definitive dial-effect success. The adapter must provide trusted protocol evidence for that exact trunk, destination and attempt. A local port, open socket, provisional ringing, generic HTTP/SIP transport success or caller assertion is insufficient. The binding must specify how its SIP implementation proves establishment and proves a no-effect refusal before it may advertise this operation; the old runtime's sequencing is evidence of separate phases, not that proof. This is a selected semantic boundary, not a claim that the predecessor implemented mutation outcome metadata.

The public successful operation result additionally requires the authenticated application binding and every declared stream to be ready. It is the full ready receipt, not an early SIP port. These two facts use [operations §4.1](../../../../../contracts/operations/v1alpha1/semantics.md#41-session-establishing-mutation-outcomes) and the existing applied-plus-error response rule:

| Observed sequence | Mutation classification / ordinary result | Ready handle |
|---|---|---|
| Input, grant, approval or preflight refuses; dispatch gate definitely did not open | `not_attempted` / specific safe refusal | none |
| Dispatch may have occurred; no definitive establishment or complete no-effect evidence | `unknown` / `outcome_unknown` | none |
| Definitive binding evidence proves the entire dial was refused without business effects, including no provisional outreach | `refused` / safe provider refusal | none |
| Target may have rung, including known ringing, but establishment is unproved; final rejection, cancellation or application failure follows | `unknown` / `outcome_unknown`; known partial outreach must not be erased | none |
| Exact SIP establishment confirmed, then application connection, authority redemption or media readiness fails | `applied` / safe error (`unavailable`, `offer_rejected` or `session_not_ready` as applicable) | none |
| Exact SIP establishment and all streams/application ready; ready decision wins before a terminal and result remains safely deliverable | `applied` / success with `state: ready` | one ready receipt |
| SIP established; terminal wins before full readiness | `applied` / `offer_rejected` or other applicable safe session error | none |
| Full ready decision wins; terminal follows before result delivery | preserve `applied`; if terminal is observed before result encoding, return the applicable safe session error: `revoked`, `lease_expired`, `session_not_ready` for confirmed close/hangup, or `session_lost` only for continuity loss | never present a terminated or revoked session as currently usable |
| Host knows establishment, but outcome persistence fails | live observer preserves `applied` and the result/delivery error, reporting storage failure separately; recovery without the evidence remains `unknown` | only if full readiness and safe delivery are independently proved |
| Response is lost, or owner restarts without effect evidence | caller/recovery sees `unknown`; a stored terminal outcome is not rewritten | no fabricated or reattached handle |
| Ready result delivered; session later closes, expires or is lost | original `applied` result remains settled; separate session terminal fact governs ongoing use | original reference grants no renewed authority |

Known ringing establishes a narrower partial effect, not the selected SIP-establishment success; `unknown` in that row means the attempt lacks a definitive success/no-effect outcome, not that every fact about the call is unknown. Conversely, confirmed SIP establishment remains applied even if no ready-session receipt can ever be delivered. A final provider error or successful CANCEL/BYE alone cannot prove that no dial effect occurred. Teardown controls remaining resources; it cannot undo outreach or retroactively convert an applied/unknown attempt into a refusal.

Only safe closed error codes and sanitized causes cross the service boundary; provider signaling, private call identifiers and credentials do not enter messages. Current observation authority precedes all mutation/result disclosure. `sip.dial` remains `idempotency: none`: a deliberate new invocation needs fresh admission and required approval and may place another call; there is no automatic resend, redial, refresh-triggered retry, reattach or approval restoration. Readiness/terminal serialization, the 2 s cutoff and 5 s local teardown bounds are unchanged. Detailed protocol evidence and enforcement tests are future binding prerequisites, not implemented by the ESS values.


## Native media and transport obligations

Only this adapter opens its admitted SIP/RTP sockets. The native user agent is
not a proxy, registrar, PBX, TURN server or IVR. Caller ID is untrusted.
The selected narrow media binding is PCM s16le, 8 kHz mono, 20 ms/320-byte frames
without transcoding at the seam. DTMF uses RFC 4733 telephone-events, refusing when
negotiation declines it. The shared interruption/loss/overload and terminal rules
remain obligations of this binding.

`sip.dial` is the ordinary mutation. `close` is a session control message, while
`signal { kind: dtmf, ... }` and `interrupt { track }` are media control messages;
offer, accept, reject and cancel are session-establishment messages. None is an
ordinary operation id unless a later profile specifies independent admission,
effects and a terminal result. `hold` and `transfer` are reserved/refused and are
absent from this binding's advertised and negotiated capability set.

Loopback is the default aperture. Explicitly authorized non-loopback configuration
must pre-admit every address and port that SDP can cause the stack to use; the old
sipx binding may transmit to learned peers before validation. Socket ownership,
peer validation and transmit-buffer cutoff need native enforcement fixtures.
No historical stack behavior is claimed conforming merely by copying the rule.

A configured SIP credential lease supplies exactly username then password for one
bounded establishment; the native adapter does not import the old service-layer
AdmittedSipPlan. Authentication/custody and current authority remain shared ports.
