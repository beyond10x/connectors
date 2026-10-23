<!--
  generated from connectors v1
  model digest 0f1b92b6b86a69778081be2aca23de79337294bbb219d2bd7f19aad1f6523d26
  contract digest b9b32b755ebf5ba2e2d690e05122218781ab050a55aa96d08f5ab2d6164aff5e
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — connectors v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

42 capabilities: **41 generated**, **1 obligations**, **0 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

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
| command behaviour | `connectors.sessions.AcceptOffer` |
| command contract | `connectors.sessions.BeginClose` |
| command behaviour | `connectors.sessions.BeginClose` |
| command contract | `connectors.sessions.EstablishReady` |
| command behaviour | `connectors.sessions.EstablishReady` |
| command contract | `connectors.sessions.FinishTeardown` |
| command behaviour | `connectors.sessions.FinishTeardown` |
| command contract | `connectors.sessions.LoseContinuity` |
| command behaviour | `connectors.sessions.LoseContinuity` |
| command contract | `connectors.sessions.OfferSession` |
| command contract | `connectors.sessions.PermitData` |
| command behaviour | `connectors.sessions.PermitData` |
| command contract | `connectors.sessions.RenewDataLease` |
| command behaviour | `connectors.sessions.RenewDataLease` |
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
| view query | `connectors.sessions.SessionStates` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `connectors.sessions.OfferSession` | kept an obligation by `creates:` leaving the required field `binding` of `connectors.sessions.Session` undetermined, in `offered` | given `connectors.sessions.OfferSession` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `offered` otherwise, creates `connectors.sessions.Session`, emits `connectors.sessions.SessionOffered` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
