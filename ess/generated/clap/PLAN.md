<!--
  generated from connectors v2
  model digest b46e08fdaea05ec1169f3d75212c80578b59fd21a94fe1d523f21714bddb663c
  contract digest d01ebb2ef8f96762b184433c1df72babf3565039aef3600d49dc1cd99b931c9b
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — connectors v2

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

216 capabilities: **200 generated**, **16 obligations**, **0 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `connectors.catalog.Audience` |
| domain type | `connectors.catalog.Authority` |
| domain type | `connectors.catalog.Catalog.State` |
| domain type | `connectors.catalog.CatalogRef` |
| domain type | `connectors.catalog.ConditionalRateLimit` |
| domain type | `connectors.catalog.FixedRateLimit` |
| domain type | `connectors.catalog.OAuthClientAuthentication` |
| domain type | `connectors.catalog.OAuthEndpoint` |
| domain type | `connectors.catalog.OAuthRedirectShape` |
| domain type | `connectors.catalog.OAuthRefreshPolicy` |
| domain type | `connectors.catalog.OAuthRegistrationUse` |
| domain type | `connectors.catalog.OAuthScopeEncoding` |
| domain type | `connectors.catalog.OAuthTokenEvidence` |
| domain type | `connectors.catalog.Operation.State` |
| domain type | `connectors.catalog.OperationRef` |
| domain type | `connectors.catalog.PersonalOAuthAdmission` |
| domain type | `connectors.catalog.PersonalOAuthFlow` |
| domain type | `connectors.catalog.Provider.State` |
| domain type | `connectors.catalog.ProviderRef` |
| domain type | `connectors.catalog.PublishedRate` |
| domain type | `connectors.catalog.RateLimitBasis` |
| domain type | `connectors.catalog.RequestParameterSemantics` |
| domain type | `connectors.catalog.RequestSemantics` |
| domain type | `connectors.catalog.RiskToken` |
| domain type | `connectors.catalog.ServiceName` |
| domain type | `connectors.catalog.SetupProfileActor` |
| domain type | `connectors.catalog.SetupProfileSummary` |
| domain type | `connectors.deployment.Credential.State` |
| domain type | `connectors.deployment.CredentialRef` |
| domain type | `connectors.deployment.CredentialSubject` |
| domain type | `connectors.deployment.Integration.State` |
| domain type | `connectors.deployment.IntegrationRef` |
| domain type | `connectors.deployment.LocalCredentialSelection` |
| domain type | `connectors.deployment.OAuthBrowserPlacement` |
| domain type | `connectors.deployment.OAuthCredentialEvidence` |
| domain type | `connectors.deployment.OAuthRefreshAttempt` |
| domain type | `connectors.deployment.PersonalOAuthCustody` |
| domain type | `connectors.deployment.PersonalOAuthRegistration` |
| domain type | `connectors.deployment.ProviderRef` |
| domain type | `connectors.deployment.Scope` |
| domain type | `connectors.endpoint.AuthProfile` |
| domain type | `connectors.endpoint.Binding` |
| domain type | `connectors.endpoint.BindingRef` |
| domain type | `connectors.endpoint.BoundRemediationStatus` |
| domain type | `connectors.endpoint.CandidateRef` |
| domain type | `connectors.endpoint.CredentialKind` |
| domain type | `connectors.endpoint.CredentialReference` |
| domain type | `connectors.endpoint.DiscoveryMetadata` |
| domain type | `connectors.endpoint.DiscoveryObservation.State` |
| domain type | `connectors.endpoint.DiscoveryRef` |
| domain type | `connectors.endpoint.Endpoint.State` |
| domain type | `connectors.endpoint.EndpointActor` |
| domain type | `connectors.endpoint.EndpointCandidate.State` |
| domain type | `connectors.endpoint.EndpointInitiator` |
| domain type | `connectors.endpoint.EndpointRef` |
| domain type | `connectors.endpoint.EndpointScope` |
| domain type | `connectors.endpoint.EventName` |
| domain type | `connectors.endpoint.EventReceiver.State` |
| domain type | `connectors.endpoint.EventReceiverRef` |
| domain type | `connectors.endpoint.EvidenceDigest` |
| domain type | `connectors.endpoint.IntegrationRef` |
| domain type | `connectors.endpoint.Label` |
| domain type | `connectors.endpoint.OAuthCompletionRefusal` |
| domain type | `connectors.endpoint.OAuthInstructionKind` |
| domain type | `connectors.endpoint.ObservationRef` |
| domain type | `connectors.endpoint.PersonalOAuthSessionPolicy` |
| domain type | `connectors.endpoint.ProviderRef` |
| domain type | `connectors.endpoint.Readiness` |
| domain type | `connectors.endpoint.RemediationAcknowledgement` |
| domain type | `connectors.endpoint.RemediationBinding` |
| domain type | `connectors.endpoint.RemediationNextAction` |
| domain type | `connectors.endpoint.RemediationResumeState` |
| domain type | `connectors.endpoint.RouteAdapter` |
| domain type | `connectors.endpoint.Scheme` |
| domain type | `connectors.endpoint.SetupSession.State` |
| domain type | `connectors.endpoint.SetupSessionRef` |
| domain type | `connectors.endpoint.Title` |
| domain type | `connectors.endpoint.Tls` |
| domain type | `connectors.endpoint.Transport` |
| domain type | `connectors.event.Delivery.State` |
| domain type | `connectors.event.DeliveryRef` |
| domain type | `connectors.event.EndpointRef` |
| domain type | `connectors.event.Event.State` |
| domain type | `connectors.event.EventFamily` |
| domain type | `connectors.event.EventProvenance` |
| domain type | `connectors.event.EventReceiverRef` |
| domain type | `connectors.event.EventRef` |
| domain type | `connectors.event.IntegrationRef` |
| domain type | `connectors.event.Subscription.State` |
| domain type | `connectors.event.SubscriptionRef` |
| domain type | `connectors.event.Webhook.State` |
| domain type | `connectors.event.WebhookRef` |
| domain type | `connectors.gitlab.ScheduleCreateFields` |
| domain type | `connectors.gitlab.ScheduleCreateRequest` |
| domain type | `connectors.gitlab.ScheduleDeleteRequest` |
| domain type | `connectors.gitlab.ScheduleListRequest` |
| domain type | `connectors.gitlab.ScheduleScope` |
| domain type | `connectors.gitlab.ScheduleSnapshot` |
| domain type | `connectors.gitlab.ScheduleUpdateFields` |
| domain type | `connectors.gitlab.ScheduleUpdateRequest` |
| domain type | `connectors.inventory.ContainerImage` |
| domain type | `connectors.inventory.DeploymentSummary` |
| domain type | `connectors.inventory.NamespaceInventory` |
| domain type | `connectors.inventory.WorkloadInventory` |
| domain type | `connectors.runtime.ApprovalPosture` |
| domain type | `connectors.runtime.Audit.State` |
| domain type | `connectors.runtime.AuditRef` |
| domain type | `connectors.runtime.AuthenticationAttemptState` |
| domain type | `connectors.runtime.AuthenticationNeed` |
| domain type | `connectors.runtime.AuthenticationNextAction` |
| domain type | `connectors.runtime.AuthenticationRequired` |
| domain type | `connectors.runtime.CallRef` |
| domain type | `connectors.runtime.ConditionalRateAdvice` |
| domain type | `connectors.runtime.CredentialReadiness` |
| domain type | `connectors.runtime.EffectClass` |
| domain type | `connectors.runtime.EndpointRef` |
| domain type | `connectors.runtime.ExecutionRef` |
| domain type | `connectors.runtime.Grant.State` |
| domain type | `connectors.runtime.GrantRef` |
| domain type | `connectors.runtime.Invocation.State` |
| domain type | `connectors.runtime.OperationRateAdvice` |
| domain type | `connectors.runtime.OperationRef` |
| domain type | `connectors.runtime.Proxy.State` |
| domain type | `connectors.runtime.ProxyRef` |
| domain type | `connectors.runtime.RateLimitObservation` |
| domain type | `connectors.runtime.Session.State` |
| domain type | `connectors.runtime.SessionTermination` |
| domain type | `connectors.runtime.SipDial.State` |
| domain type | `connectors.target.Target` |
| entity lifecycle | `connectors.catalog.Catalog` |
| entity lifecycle | `connectors.catalog.Operation` |
| entity lifecycle | `connectors.catalog.Provider` |
| entity lifecycle | `connectors.deployment.Credential` |
| entity lifecycle | `connectors.deployment.Integration` |
| entity lifecycle | `connectors.endpoint.DiscoveryObservation` |
| entity lifecycle | `connectors.endpoint.Endpoint` |
| entity lifecycle | `connectors.endpoint.EndpointCandidate` |
| entity lifecycle | `connectors.endpoint.EventReceiver` |
| entity lifecycle | `connectors.endpoint.SetupSession` |
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
| command contract | `connectors.endpoint.ActivateCandidate` |
| command contract | `connectors.endpoint.AuthorizeEndpoint` |
| command contract | `connectors.endpoint.ConnectEventReceiver` |
| command contract | `connectors.endpoint.CreateSetupSession` |
| command contract | `connectors.endpoint.FinishSetupSession` |
| command contract | `connectors.endpoint.MaterializeObservation` |
| command contract | `connectors.endpoint.ReauthorizeEndpoint` |
| command contract | `connectors.endpoint.ReconnectEventReceiver` |
| command contract | `connectors.endpoint.RefreshObservation` |
| command contract | `connectors.endpoint.RevokeEndpoint` |
| command contract | `connectors.endpoint.StopEventReceiver` |
| command contract | `connectors.endpoint.SuperviseEventReceiver` |
| command contract | `connectors.endpoint.VerifyEndpoint` |
| command contract | `connectors.runtime.InvokeOperation` |
| command contract | `connectors.runtime.SettleSession` |
| command contract | `connectors.runtime.TerminateSession` |
| event type | `connectors.endpoint.CandidateActivated` |
| event type | `connectors.endpoint.EndpointAuthorized` |
| event type | `connectors.endpoint.EndpointBecameCallable` |
| event type | `connectors.endpoint.EndpointDegraded` |
| event type | `connectors.endpoint.EndpointReauthorized` |
| event type | `connectors.endpoint.EndpointRevoked` |
| event type | `connectors.endpoint.EventReceiverConnected` |
| event type | `connectors.endpoint.EventReceiverReconnecting` |
| event type | `connectors.endpoint.EventReceiverStarting` |
| event type | `connectors.endpoint.EventReceiverStopped` |
| event type | `connectors.endpoint.ObservationMaterialized` |
| event type | `connectors.endpoint.ObservationReobserved` |
| event type | `connectors.endpoint.ObservationWithdrawn` |
| event type | `connectors.endpoint.SetupSessionCompleted` |
| event type | `connectors.endpoint.SetupSessionCreated` |
| event type | `connectors.endpoint.SetupSessionExpired` |
| event type | `connectors.endpoint.SetupSessionFailed` |
| event type | `connectors.runtime.SessionEstablished` |
| event type | `connectors.runtime.SessionTerminated` |
| event type | `connectors.runtime.SessionTerminating` |
| error type | `connectors.endpoint.CandidateNotFound` |
| error type | `connectors.endpoint.MaterializationNotGranted` |
| error type | `connectors.endpoint.MaterializationUnavailable` |
| error type | `connectors.endpoint.ObservationNoLongerCurrent` |
| error type | `connectors.endpoint.ObservationNotFound` |
| error type | `connectors.endpoint.SetupSessionCapacity` |
| error type | `connectors.endpoint.SetupSessionNotPending` |
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
| command behaviour | `connectors.endpoint.ActivateCandidate` | decided outside the system: no candidate is stored under this reference | given `connectors.endpoint.ActivateCandidate` input, decide and enact exactly one outcome — `activated` otherwise, takes `activate` of `connectors.endpoint.EndpointCandidate`, emits `connectors.endpoint.CandidateActivated`; `not-found` externally decided (no candidate is stored under this reference), error `connectors.endpoint.CandidateNotFound` |
| command behaviour | `connectors.endpoint.AuthorizeEndpoint` | the contract is declared; the algorithm is not | given `connectors.endpoint.AuthorizeEndpoint` input, decide and enact exactly one outcome — `authorized` otherwise, takes `authorize` of `connectors.endpoint.Endpoint`, emits `connectors.endpoint.EndpointAuthorized` |
| command behaviour | `connectors.endpoint.ConnectEventReceiver` | the contract is declared; the algorithm is not | given `connectors.endpoint.ConnectEventReceiver` input, decide and enact exactly one outcome — `connected` otherwise, takes `attach` of `connectors.endpoint.EventReceiver`, emits `connectors.endpoint.EventReceiverConnected` |
| command behaviour | `connectors.endpoint.CreateSetupSession` | decided outside the system: the bounded pending set is already at `maximum_pending` | given `connectors.endpoint.CreateSetupSession` input, decide and enact exactly one outcome — `pending` otherwise, creates `connectors.endpoint.SetupSession`, emits `connectors.endpoint.SetupSessionCreated`; `at-capacity` externally decided (the bounded pending set is already at `maximum_pending`), error `connectors.endpoint.SetupSessionCapacity` |
| command behaviour | `connectors.endpoint.FinishSetupSession` | decided outside the system: the session outlived its `expires_at_unix_ms` | given `connectors.endpoint.FinishSetupSession` input, decide and enact exactly one outcome — `completed` otherwise, takes `complete` of `connectors.endpoint.SetupSession`, emits `connectors.endpoint.SetupSessionCompleted`; `expired` externally decided (the session outlived its `expires_at_unix_ms`), takes `expire` of `connectors.endpoint.SetupSession`, emits `connectors.endpoint.SetupSessionExpired`; `failed` externally decided (the acquisition attempt failed, or the process shut down with the session still pending), takes `fail` of `connectors.endpoint.SetupSession`, emits `connectors.endpoint.SetupSessionFailed`; `not-pending` from a state no declared move starts in, error `connectors.endpoint.SetupSessionNotPending` |
| command behaviour | `connectors.endpoint.MaterializeObservation` | decided outside the system: no observation is stored under this reference | given `connectors.endpoint.MaterializeObservation` input, decide and enact exactly one outcome — `materialized` otherwise, takes `materialize` of `connectors.endpoint.DiscoveryObservation`, emits `connectors.endpoint.ObservationMaterialized`; `not-found` externally decided (no observation is stored under this reference), error `connectors.endpoint.ObservationNotFound`; `no-longer-current` externally decided (the observation was deactivated by a later refresh pass), error `connectors.endpoint.ObservationNoLongerCurrent`; `not-granted` externally decided (the observed type has no target Provider contract, or that Provider has no independent Connector Grant), error `connectors.endpoint.MaterializationNotGranted`; `unavailable` externally decided (durable inventory storage refused the new child projection), error `connectors.endpoint.MaterializationUnavailable` |
| command behaviour | `connectors.endpoint.ReauthorizeEndpoint` | the contract is declared; the algorithm is not | given `connectors.endpoint.ReauthorizeEndpoint` input, decide and enact exactly one outcome — `reauthorized` otherwise, takes `reauthorize` of `connectors.endpoint.Endpoint`, emits `connectors.endpoint.EndpointReauthorized` |
| command behaviour | `connectors.endpoint.ReconnectEventReceiver` | the contract is declared; the algorithm is not | given `connectors.endpoint.ReconnectEventReceiver` input, decide and enact exactly one outcome — `reconnecting` otherwise, takes `supervise` of `connectors.endpoint.EventReceiver`, emits `connectors.endpoint.EventReceiverReconnecting` |
| command behaviour | `connectors.endpoint.RefreshObservation` | decided outside the system: the re-seen observation still carries the `endpoint_ref` a materialization set | given `connectors.endpoint.RefreshObservation` input, decide and enact exactly one outcome — `reobserved` otherwise, takes `reobserve` of `connectors.endpoint.DiscoveryObservation`, emits `connectors.endpoint.ObservationReobserved`; `rematerialized` externally decided (the re-seen observation still carries the `endpoint_ref` a materialization set), takes `rematerialize` of `connectors.endpoint.DiscoveryObservation`, emits `connectors.endpoint.ObservationReobserved`; `withdrawn` externally decided (the refresh pass no longer sees the resource this observation was read from), takes `withdraw` of `connectors.endpoint.DiscoveryObservation`, emits `connectors.endpoint.ObservationWithdrawn` |
| command behaviour | `connectors.endpoint.RevokeEndpoint` | the contract is declared; the algorithm is not | given `connectors.endpoint.RevokeEndpoint` input, decide and enact exactly one outcome — `revoked` otherwise, takes `revoke` of `connectors.endpoint.Endpoint`, emits `connectors.endpoint.EndpointRevoked` |
| command behaviour | `connectors.endpoint.StopEventReceiver` | the contract is declared; the algorithm is not | given `connectors.endpoint.StopEventReceiver` input, decide and enact exactly one outcome — `stopped` otherwise, takes `stop` of `connectors.endpoint.EventReceiver`, emits `connectors.endpoint.EventReceiverStopped` |
| command behaviour | `connectors.endpoint.SuperviseEventReceiver` | the contract is declared; the algorithm is not | given `connectors.endpoint.SuperviseEventReceiver` input, decide and enact exactly one outcome — `starting` otherwise, creates `connectors.endpoint.EventReceiver`, emits `connectors.endpoint.EventReceiverStarting` |
| command behaviour | `connectors.endpoint.VerifyEndpoint` | decided outside the system: the provider reports the credential expired or was revoked upstream | given `connectors.endpoint.VerifyEndpoint` input, decide and enact exactly one outcome — `callable` otherwise, takes `verify` of `connectors.endpoint.Endpoint`, emits `connectors.endpoint.EndpointBecameCallable`; `degraded` externally decided (the provider reports the credential expired or was revoked upstream), takes `degrade` of `connectors.endpoint.Endpoint`, emits `connectors.endpoint.EndpointDegraded` |
| command behaviour | `connectors.runtime.InvokeOperation` | decided outside the system: the grant, the approval gate or the provider refuses the call | given `connectors.runtime.InvokeOperation` input, decide and enact exactly one outcome — `established` otherwise, takes `establish` of `connectors.runtime.Session`, emits `connectors.runtime.SessionEstablished`; `refused` externally decided (the grant, the approval gate or the provider refuses the call), error `connectors.runtime.OperationRefused` |
| command behaviour | `connectors.runtime.SettleSession` | the contract is declared; the algorithm is not | given `connectors.runtime.SettleSession` input, decide and enact exactly one outcome — `terminated` otherwise, takes `settle` of `connectors.runtime.Session`, emits `connectors.runtime.SessionTerminated` |
| command behaviour | `connectors.runtime.TerminateSession` | decided outside the system: no session is held under this execution reference | given `connectors.runtime.TerminateSession` input, decide and enact exactly one outcome — `terminating` otherwise, takes `terminate` of `connectors.runtime.Session`, emits `connectors.runtime.SessionTerminating`; `not-found` externally decided (no session is held under this execution reference), error `connectors.runtime.SessionNotFound`; `unavailable` externally decided (the audit journal could not record the termination request), error `connectors.runtime.RuntimeUnavailable` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
