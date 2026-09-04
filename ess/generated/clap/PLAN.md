<!--
  generated from connectors v1
  model digest 5849d9d17106bb6c8f9f7331f9c1fcf8d8addd0506851273b950db073eacb81b
  contract digest ec905bc6d63ad79ca30a539c0a27a4d8c83bd41b4d0909594faa9ae6f34ab91e
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — connectors v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

158 capabilities: **142 generated**, **16 obligations**, **0 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `connectors.catalog.Audience` |
| domain type | `connectors.catalog.Authority` |
| domain type | `connectors.catalog.Catalog.State` |
| domain type | `connectors.catalog.CatalogRef` |
| domain type | `connectors.catalog.Operation.State` |
| domain type | `connectors.catalog.OperationRef` |
| domain type | `connectors.catalog.Provider.State` |
| domain type | `connectors.catalog.ProviderRef` |
| domain type | `connectors.catalog.RiskToken` |
| domain type | `connectors.catalog.ServiceName` |
| domain type | `connectors.catalog.SetupProfileActor` |
| domain type | `connectors.catalog.SetupProfileSummary` |
| domain type | `connectors.connection.AuthProfile` |
| domain type | `connectors.connection.BindingRef` |
| domain type | `connectors.connection.CandidateRef` |
| domain type | `connectors.connection.Channel.State` |
| domain type | `connectors.connection.ChannelRef` |
| domain type | `connectors.connection.ConnectSession.State` |
| domain type | `connectors.connection.ConnectSessionRef` |
| domain type | `connectors.connection.Connection.State` |
| domain type | `connectors.connection.ConnectionActor` |
| domain type | `connectors.connection.ConnectionCandidate.State` |
| domain type | `connectors.connection.ConnectionInitiator` |
| domain type | `connectors.connection.ConnectionRef` |
| domain type | `connectors.connection.ConnectionScope` |
| domain type | `connectors.connection.DiscoveryObservation.State` |
| domain type | `connectors.connection.DiscoveryRef` |
| domain type | `connectors.connection.EventName` |
| domain type | `connectors.connection.EvidenceDigest` |
| domain type | `connectors.connection.IntegrationRef` |
| domain type | `connectors.connection.Label` |
| domain type | `connectors.connection.ObservationRef` |
| domain type | `connectors.connection.ProviderRef` |
| domain type | `connectors.connection.RouteAdapter` |
| domain type | `connectors.connection.Title` |
| domain type | `connectors.deployment.Credential.State` |
| domain type | `connectors.deployment.CredentialRef` |
| domain type | `connectors.deployment.CredentialSubject` |
| domain type | `connectors.deployment.Integration.State` |
| domain type | `connectors.deployment.IntegrationRef` |
| domain type | `connectors.deployment.ProviderRef` |
| domain type | `connectors.deployment.Scope` |
| domain type | `connectors.event.ChannelRef` |
| domain type | `connectors.event.ConnectionRef` |
| domain type | `connectors.event.Delivery.State` |
| domain type | `connectors.event.DeliveryRef` |
| domain type | `connectors.event.Event.State` |
| domain type | `connectors.event.EventFamily` |
| domain type | `connectors.event.EventProvenance` |
| domain type | `connectors.event.EventRef` |
| domain type | `connectors.event.IntegrationRef` |
| domain type | `connectors.event.Subscription.State` |
| domain type | `connectors.event.SubscriptionRef` |
| domain type | `connectors.event.Webhook.State` |
| domain type | `connectors.event.WebhookRef` |
| domain type | `connectors.runtime.ApprovalPosture` |
| domain type | `connectors.runtime.Audit.State` |
| domain type | `connectors.runtime.AuditRef` |
| domain type | `connectors.runtime.CallRef` |
| domain type | `connectors.runtime.ConnectionRef` |
| domain type | `connectors.runtime.EffectClass` |
| domain type | `connectors.runtime.ExecutionRef` |
| domain type | `connectors.runtime.Grant.State` |
| domain type | `connectors.runtime.GrantRef` |
| domain type | `connectors.runtime.Invocation.State` |
| domain type | `connectors.runtime.OperationRef` |
| domain type | `connectors.runtime.Proxy.State` |
| domain type | `connectors.runtime.ProxyRef` |
| domain type | `connectors.runtime.Session.State` |
| domain type | `connectors.runtime.SessionTermination` |
| domain type | `connectors.runtime.SipDial.State` |
| domain type | `connectors.target.Target` |
| entity lifecycle | `connectors.catalog.Catalog` |
| entity lifecycle | `connectors.catalog.Operation` |
| entity lifecycle | `connectors.catalog.Provider` |
| entity lifecycle | `connectors.connection.Channel` |
| entity lifecycle | `connectors.connection.ConnectSession` |
| entity lifecycle | `connectors.connection.Connection` |
| entity lifecycle | `connectors.connection.ConnectionCandidate` |
| entity lifecycle | `connectors.connection.DiscoveryObservation` |
| entity lifecycle | `connectors.deployment.Credential` |
| entity lifecycle | `connectors.deployment.Integration` |
| entity lifecycle | `connectors.event.Delivery` |
| entity lifecycle | `connectors.event.Event` |
| entity lifecycle | `connectors.event.Subscription` |
| entity lifecycle | `connectors.event.Webhook` |
| entity lifecycle | `connectors.runtime.Audit` |
| entity lifecycle | `connectors.runtime.Grant` |
| entity lifecycle | `connectors.runtime.Invocation` |
| entity lifecycle | `connectors.runtime.Proxy` |
| entity lifecycle | `connectors.runtime.Session` |
| entity lifecycle | `connectors.runtime.SipDial` |
| command contract | `connectors.connection.ActivateCandidate` |
| command contract | `connectors.connection.AuthorizeConnection` |
| command contract | `connectors.connection.ConnectChannel` |
| command contract | `connectors.connection.CreateConnectSession` |
| command contract | `connectors.connection.FinishConnectSession` |
| command contract | `connectors.connection.MaterializeObservation` |
| command contract | `connectors.connection.ReauthorizeConnection` |
| command contract | `connectors.connection.ReconnectChannel` |
| command contract | `connectors.connection.RefreshObservation` |
| command contract | `connectors.connection.RevokeConnection` |
| command contract | `connectors.connection.StopChannel` |
| command contract | `connectors.connection.SuperviseChannel` |
| command contract | `connectors.connection.VerifyConnection` |
| command contract | `connectors.runtime.InvokeOperation` |
| command contract | `connectors.runtime.SettleSession` |
| command contract | `connectors.runtime.TerminateSession` |
| event type | `connectors.connection.CandidateActivated` |
| event type | `connectors.connection.ChannelConnected` |
| event type | `connectors.connection.ChannelReconnecting` |
| event type | `connectors.connection.ChannelStarting` |
| event type | `connectors.connection.ChannelStopped` |
| event type | `connectors.connection.ConnectSessionCompleted` |
| event type | `connectors.connection.ConnectSessionCreated` |
| event type | `connectors.connection.ConnectSessionExpired` |
| event type | `connectors.connection.ConnectSessionFailed` |
| event type | `connectors.connection.ConnectionAuthorized` |
| event type | `connectors.connection.ConnectionBecameCallable` |
| event type | `connectors.connection.ConnectionDegraded` |
| event type | `connectors.connection.ConnectionReauthorized` |
| event type | `connectors.connection.ConnectionRevoked` |
| event type | `connectors.connection.ObservationMaterialized` |
| event type | `connectors.connection.ObservationReobserved` |
| event type | `connectors.connection.ObservationWithdrawn` |
| event type | `connectors.runtime.SessionEstablished` |
| event type | `connectors.runtime.SessionTerminated` |
| event type | `connectors.runtime.SessionTerminating` |
| error type | `connectors.connection.CandidateNotFound` |
| error type | `connectors.connection.ConnectSessionCapacity` |
| error type | `connectors.connection.ConnectSessionNotPending` |
| error type | `connectors.connection.MaterializationNotGranted` |
| error type | `connectors.connection.ObservationNoLongerCurrent` |
| error type | `connectors.connection.ObservationNotFound` |
| error type | `connectors.runtime.OperationRefused` |
| error type | `connectors.runtime.RuntimeUnavailable` |
| error type | `connectors.runtime.SessionNotFound` |
| error type | `connectors.runtime.SessionOutcomeUnknown` |
| component port | `catalog-build` |
| component port | `connectors-cli` |
| component port | `connectors-service` |
| component transport | `connectors-service` |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `connectors.connection.ActivateCandidate` | decided outside the system: no candidate is stored under this reference | given `connectors.connection.ActivateCandidate` input, decide and enact exactly one outcome — `activated` otherwise, takes `activate` of `connectors.connection.ConnectionCandidate`, emits `connectors.connection.CandidateActivated`; `not-found` externally decided (no candidate is stored under this reference), error `connectors.connection.CandidateNotFound` |
| command behaviour | `connectors.connection.AuthorizeConnection` | the contract is declared; the algorithm is not | given `connectors.connection.AuthorizeConnection` input, decide and enact exactly one outcome — `authorized` otherwise, takes `authorize` of `connectors.connection.Connection`, emits `connectors.connection.ConnectionAuthorized` |
| command behaviour | `connectors.connection.ConnectChannel` | the contract is declared; the algorithm is not | given `connectors.connection.ConnectChannel` input, decide and enact exactly one outcome — `connected` otherwise, takes `attach` of `connectors.connection.Channel`, emits `connectors.connection.ChannelConnected` |
| command behaviour | `connectors.connection.CreateConnectSession` | decided outside the system: the bounded pending set is already at `maximum_pending` | given `connectors.connection.CreateConnectSession` input, decide and enact exactly one outcome — `pending` otherwise, creates `connectors.connection.ConnectSession`, emits `connectors.connection.ConnectSessionCreated`; `at-capacity` externally decided (the bounded pending set is already at `maximum_pending`), error `connectors.connection.ConnectSessionCapacity` |
| command behaviour | `connectors.connection.FinishConnectSession` | decided outside the system: the session outlived its `expires_at_unix_ms` | given `connectors.connection.FinishConnectSession` input, decide and enact exactly one outcome — `completed` otherwise, takes `complete` of `connectors.connection.ConnectSession`, emits `connectors.connection.ConnectSessionCompleted`; `expired` externally decided (the session outlived its `expires_at_unix_ms`), takes `expire` of `connectors.connection.ConnectSession`, emits `connectors.connection.ConnectSessionExpired`; `failed` externally decided (the acquisition attempt failed, or the process shut down with the session still pending), takes `fail` of `connectors.connection.ConnectSession`, emits `connectors.connection.ConnectSessionFailed`; `not-pending` from a state no declared move starts in, error `connectors.connection.ConnectSessionNotPending` |
| command behaviour | `connectors.connection.MaterializeObservation` | decided outside the system: no observation is stored under this reference | given `connectors.connection.MaterializeObservation` input, decide and enact exactly one outcome — `materialized` otherwise, takes `materialize` of `connectors.connection.DiscoveryObservation`, emits `connectors.connection.ObservationMaterialized`; `not-found` externally decided (no observation is stored under this reference), error `connectors.connection.ObservationNotFound`; `no-longer-current` externally decided (the observation was deactivated by a later refresh pass), error `connectors.connection.ObservationNoLongerCurrent`; `not-granted` externally decided (the observed type has no target Provider contract, or that Provider has no independent Connector Grant), error `connectors.connection.MaterializationNotGranted` |
| command behaviour | `connectors.connection.ReauthorizeConnection` | the contract is declared; the algorithm is not | given `connectors.connection.ReauthorizeConnection` input, decide and enact exactly one outcome — `reauthorized` otherwise, takes `reauthorize` of `connectors.connection.Connection`, emits `connectors.connection.ConnectionReauthorized` |
| command behaviour | `connectors.connection.ReconnectChannel` | the contract is declared; the algorithm is not | given `connectors.connection.ReconnectChannel` input, decide and enact exactly one outcome — `reconnecting` otherwise, takes `supervise` of `connectors.connection.Channel`, emits `connectors.connection.ChannelReconnecting` |
| command behaviour | `connectors.connection.RefreshObservation` | decided outside the system: the re-seen observation still carries the `connection_ref` a materialization set | given `connectors.connection.RefreshObservation` input, decide and enact exactly one outcome — `reobserved` otherwise, takes `reobserve` of `connectors.connection.DiscoveryObservation`, emits `connectors.connection.ObservationReobserved`; `rematerialized` externally decided (the re-seen observation still carries the `connection_ref` a materialization set), takes `rematerialize` of `connectors.connection.DiscoveryObservation`, emits `connectors.connection.ObservationReobserved`; `withdrawn` externally decided (the refresh pass no longer sees the resource this observation was read from), takes `withdraw` of `connectors.connection.DiscoveryObservation`, emits `connectors.connection.ObservationWithdrawn` |
| command behaviour | `connectors.connection.RevokeConnection` | the contract is declared; the algorithm is not | given `connectors.connection.RevokeConnection` input, decide and enact exactly one outcome — `revoked` otherwise, takes `revoke` of `connectors.connection.Connection`, emits `connectors.connection.ConnectionRevoked` |
| command behaviour | `connectors.connection.StopChannel` | the contract is declared; the algorithm is not | given `connectors.connection.StopChannel` input, decide and enact exactly one outcome — `stopped` otherwise, takes `stop` of `connectors.connection.Channel`, emits `connectors.connection.ChannelStopped` |
| command behaviour | `connectors.connection.SuperviseChannel` | the contract is declared; the algorithm is not | given `connectors.connection.SuperviseChannel` input, decide and enact exactly one outcome — `starting` otherwise, creates `connectors.connection.Channel`, emits `connectors.connection.ChannelStarting` |
| command behaviour | `connectors.connection.VerifyConnection` | decided outside the system: the provider reports the credential expired or was revoked upstream | given `connectors.connection.VerifyConnection` input, decide and enact exactly one outcome — `callable` otherwise, takes `verify` of `connectors.connection.Connection`, emits `connectors.connection.ConnectionBecameCallable`; `degraded` externally decided (the provider reports the credential expired or was revoked upstream), takes `degrade` of `connectors.connection.Connection`, emits `connectors.connection.ConnectionDegraded` |
| command behaviour | `connectors.runtime.InvokeOperation` | decided outside the system: the grant, the approval gate or the provider refuses the call | given `connectors.runtime.InvokeOperation` input, decide and enact exactly one outcome — `established` otherwise, takes `establish` of `connectors.runtime.Session`, emits `connectors.runtime.SessionEstablished`; `refused` externally decided (the grant, the approval gate or the provider refuses the call), error `connectors.runtime.OperationRefused` |
| command behaviour | `connectors.runtime.SettleSession` | the contract is declared; the algorithm is not | given `connectors.runtime.SettleSession` input, decide and enact exactly one outcome — `terminated` otherwise, takes `settle` of `connectors.runtime.Session`, emits `connectors.runtime.SessionTerminated` |
| command behaviour | `connectors.runtime.TerminateSession` | decided outside the system: no session is held under this execution reference | given `connectors.runtime.TerminateSession` input, decide and enact exactly one outcome — `terminating` otherwise, takes `terminate` of `connectors.runtime.Session`, emits `connectors.runtime.SessionTerminating`; `not-found` externally decided (no session is held under this execution reference), error `connectors.runtime.SessionNotFound`; `unavailable` externally decided (the audit journal could not record the termination request), error `connectors.runtime.RuntimeUnavailable` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
