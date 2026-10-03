<!--
  generated from connectors v1
  model digest 0f1b92b6b86a69778081be2aca23de79337294bbb219d2bd7f19aad1f6523d26
  contract digest b9b32b755ebf5ba2e2d690e05122218781ab050a55aa96d08f5ab2d6164aff5e
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — connectors v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

42 capabilities: **33 generated**, **9 obligations**, **0 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `connectors.sessions.Binding` |
| domain type | `connectors.sessions.CleanupDecision` |
| domain type | `connectors.sessions.DataLease` |
| domain type | `connectors.sessions.GateDecision` |
| domain type | `connectors.sessions.PeerShutdown` |
| domain type | `connectors.sessions.Placement` |
| domain type | `connectors.sessions.Profile` |
| domain type | `connectors.sessions.Session.State` |
| domain type | `connectors.sessions.SessionId` |
| domain type | `connectors.sessions.TerminalActor` |
| domain type | `connectors.sessions.TerminalFact` |
| domain type | `connectors.sessions.TerminalReason` |
| entity lifecycle | `connectors.sessions.Session` |
| command contract | `connectors.sessions.AcceptOffer` |
| command contract | `connectors.sessions.BeginClose` |
| command contract | `connectors.sessions.EstablishReady` |
| command contract | `connectors.sessions.FinishTeardown` |
| command contract | `connectors.sessions.LoseContinuity` |
| command contract | `connectors.sessions.OfferSession` |
| command contract | `connectors.sessions.PermitData` |
| command contract | `connectors.sessions.RenewDataLease` |
| event type | `connectors.sessions.ClosingBegun` |
| event type | `connectors.sessions.ContinuityLost` |
| event type | `connectors.sessions.DataAuthorityTerminated` |
| event type | `connectors.sessions.DataLeaseRenewed` |
| event type | `connectors.sessions.DataPermitted` |
| event type | `connectors.sessions.LocalCleanupUnaccounted` |
| event type | `connectors.sessions.LocalResourcesReleased` |
| event type | `connectors.sessions.OfferAccepted` |
| event type | `connectors.sessions.SessionOffered` |
| event type | `connectors.sessions.SessionReady` |
| error type | `connectors.sessions.StateConflict` |
| view type | `connectors.sessions.SessionStates` |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `connectors.sessions.AcceptOffer` | the contract is declared; the algorithm is not | given `connectors.sessions.AcceptOffer` input, decide and enact exactly one outcome — `accepted` otherwise, takes `accept` of `connectors.sessions.Session`, emits `connectors.sessions.OfferAccepted`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.BeginClose` | the contract is declared; the algorithm is not | given `connectors.sessions.BeginClose` input, decide and enact exactly one outcome — `closing` otherwise, takes `begin_close` of `connectors.sessions.Session`, emits `connectors.sessions.ClosingBegun`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.EstablishReady` | the contract is declared; the algorithm is not | given `connectors.sessions.EstablishReady` input, decide and enact exactly one outcome — `ready` when `decision == allow`, takes `ready` of `connectors.sessions.Session`, emits `connectors.sessions.SessionReady`; `authority-terminated` otherwise, takes `deny_ready` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.FinishTeardown` | the contract is declared; the algorithm is not | given `connectors.sessions.FinishTeardown` input, decide and enact exactly one outcome — `closed` when `decision == released`, takes `close` of `connectors.sessions.Session`, emits `connectors.sessions.LocalResourcesReleased`; `lost` otherwise, takes `cleanup_lost` of `connectors.sessions.Session`, emits `connectors.sessions.LocalCleanupUnaccounted`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.LoseContinuity` | the contract is declared; the algorithm is not | given `connectors.sessions.LoseContinuity` input, decide and enact exactly one outcome — `lost` otherwise, takes `continuity_lost` of `connectors.sessions.Session`, emits `connectors.sessions.ContinuityLost`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.OfferSession` | the contract is declared; the algorithm is not | given `connectors.sessions.OfferSession` input, decide and enact exactly one outcome — `offered` otherwise, creates `connectors.sessions.Session`, emits `connectors.sessions.SessionOffered` |
| command behaviour | `connectors.sessions.PermitData` | the contract is declared; the algorithm is not | given `connectors.sessions.PermitData` input, decide and enact exactly one outcome — `permitted` when `decision == allow`, takes `permit_data` of `connectors.sessions.Session`, emits `connectors.sessions.DataPermitted`; `authority-terminated` otherwise, takes `deny_data` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `connectors.sessions.RenewDataLease` | the contract is declared; the algorithm is not | given `connectors.sessions.RenewDataLease` input, decide and enact exactly one outcome — `renewed` when `decision == allow`, takes `renew` of `connectors.sessions.Session`, emits `connectors.sessions.DataLeaseRenewed`; `authority-terminated` otherwise, takes `deny_renewal` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields |
| view query | `connectors.sessions.SessionStates` | how the projection is kept current is a storage decision | a query answering `connectors.sessions.SessionStates` with rows projected from `connectors.sessions.Session` at `read_your_writes` consistency |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
