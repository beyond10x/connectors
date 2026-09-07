---
format: aep.planning-md/1
id: review-result:domain-language-independent-review-round-one
kind: review-result
status: active
title: 'Domain language: independent review, round 1'
relations:
- reviews: story:reconcile-connectors-domain-language
revision: 1
---
Independent review of the Connectors domain-language proposal

Verdict: **revise before acceptance**. The central vocabulary is a substantial, source-grounded proposal, but the event model and incoming-session ownership still need explicit decisions. The assertion at lines 349–358 that these are complete recommendations is therefore too strong. This review is analysis evidence, not operator approval or implementation authorization.

Reviewed document: `/home/timo/.local/state/worktree/trees/b10x/connectors/wt-87a5e72befe7/docs/design/23-domain-language-review.md`.

Verified document SHA256: `5132420037a9f6e5b9d0533f07e075ab2cf0432b17178ac509e0c5d80fa1b82e`.

Source baseline: `62b1014551cd2b75f89df946b63e62b41a084199`, read from `/home/timo/.cache/connectors-naming-analysis-20260907/source`. All baseline paths and line numbers below refer to that commit, not the worktree's interrupted implementation.

1. **NIR-001 — High — Define consumer stream membership after cross-receiver deduplication.** Document lines **269–271**, also 214–215 and 326.

   The proposal allows subscriptions to select receivers, but merges an event received through Socket Mode and a webhook into one retained occurrence attributed to the first accepted receiver. Consider two receivers for the same Slack Endpoint, with one consumer selecting each receiver. If the webhook wins the race, what does the socket consumer receive? First-receiver provenance alone cannot determine both consumers' stream membership. Under the existing per-channel interpretation, that consumer misses the occurrence; under a new endpoint-wide interpretation, its receiver selection has a different, unspecified meaning. The same ambiguity applies to retained-event reads after stopping the winning receiver. This is a contradiction between proposed selection and deduplication rules, not a demand to implement a deduplicator.

   Baseline evidence: `crates/protocol/src/event.rs:54–59` selects a channel for receive, and `:87–95` attributes a DataEvent to one channel. `crates/connectors-runtime/src/kubernetes_endpoint_event_store.rs:183–189` deduplicates within a channel; `:201–215` records that channel; `:240–265` filters reads by it. These support the current behavior only; the proposal deliberately introduces broader deduplication.

   **Correction:** separate ingestion provenance from logical stream membership in the relationship table and D4. Recommend retaining one occurrence while explicitly making it readable through each applicable admitted receiver selection, with one delivery per consumer selection. Alternatively, make consumer selection independent of transport receivers and explain the resulting read semantics. The document needs a conceptual rule and a two-consumer walkthrough, not a storage algorithm or new generic entity.

2. **NIR-002 — Medium — Assign control ownership for an incoming offered Session.** Document line **301**, against 217 and 303.

   The relationship table requires every admitted Session to have exactly one controlling principal. D7 says incoming admission already produces an offered Session, then an authorized application or CLI accepts or rejects it. It never assigns the controlling principal before acceptance or says whether acceptance changes that principal. Endpoint ownership, an eligible accepting application, and the media destination are deliberately distinct elsewhere in the document, so none can be silently substituted here. For a tenant-shared SIP Endpoint with two eligible applications, it is unclear who may inspect, reject, or control an offer and whether one application's rejection can end another application's offer. Preventing duplicate sessions does not answer the ownership question.

   Baseline evidence: `crates/driver-sip/src/lib.rs:77–82` exposes outbound establishment; the reviewed baseline does not supply the proposed incoming admission rule. Existing controls are guarded by the configured principal in `crates/integration-sip/src/backend/mod.rs:283–293` and `:604–609`, and operate on the established session registry in `crates/integration-sip/src/backend/sessions.rs:36–70`. These establish the importance of control ownership, not an incoming implementation to preserve.

   **Correction / explicit decision:** choose either a configured controlling principal assigned at offer admission, with any delegated acceptance stated, or an offered Session with zero controllers until one authorized accepter claims it. In the latter case, amend the table's cardinality by state and distinguish offer-admission authority from established-session control. Name where that policy is configured, for example the Endpoint's incoming-session configuration. No SessionReceiver, Handler registry, auto-answer policy, or signaling API implementation is required to resolve this.

3. **NIR-003 — Medium — Preserve the separate operational-event vocabulary.** Document line **159**, also 109, 153–162 and the completeness claims at 13 and 366.

   The proposed Event/EventType definitions cover provider occurrences with native or polled provenance, flowing from an EventReceiver. They omit the baseline's separate operational-event family. A credential-degraded, receiver-down, or delivery-failing fact is generated by Connectors and does not naturally have a provider EventReceiverSpec or native/polled provenance. An inventory row naming the event domain does not resolve what happens to that existing distinction. Applying the new glossary literally would either force these facts into the provider data stream or leave their names outside the purported complete vocabulary.

   Baseline evidence: `docs/design/01-domain-model.md:366–376` declares disjoint data and operational event families with separate subscriptions. `ess/system/domains/event.yaml:44–52` explicitly models EventFamily and marks operational wire support unmapped. `crates/protocol/src/event.rs:1–4` explicitly excludes operational events from the current data-event protocol. Examples also exist in `ess/system/domains/connection.yaml:840–865`.

   **Correction:** explicitly retain DataEvent and OperationalEvent, or retain Event as an umbrella with a qualified family and family-specific type/provenance rules. State that EventReceiver describes external data intake, and that operational publication/consumption remains declared-only where no runtime exists. This is accounting for an existing concept; it does not require building an operational-event stream.

The parts I would retain are Endpoint as an access context, Integration as provider enablement, separate credential purposes and ownership, one immediate mediated-route parent, distinct discovery provenance, and independent references for two Slack identities. The source supports the need for these distinctions. Datasource plus Endpoint plus typed scope preserves the concrete Jira/GitLab/Kubernetes selection responsibilities. EventReceiverSpec versus IncomingSessionSpec, and Listener versus Handler, separate declarations, active intake, sockets, and code sensibly. The neutral Session identity and preservation of native protocol IDs and signed legacy bytes are also appropriate. I found no reason to demand another public account/instance selector or to rename upstream resource fields.

The opening CLI selection table is useful for beginners. After the findings are resolved, one short example showing a receiver, retained event, and two consumer selections would clarify the hardest remaining distinction. D7 should likewise say plainly who owns an incoming offer. Exact reference encodings and wire version allocation can remain deferred as the document says.

Scope is generally controlled: the document explicitly distinguishes proposals from baseline behavior and does not claim naming acceptance authorizes generic intake, incoming SIP, migrations, consumer changes, or releases. Cross-transport deduplication and detailed incoming-offer behavior are behavioral recommendations within the document; their unresolved parts should remain marked as such rather than being implied by approval of the nouns. No implementation or live tests are prerequisites for this review.

Checks and limitations: I read the full proposal and repository AGENTS.md; verified the proposal hash; verified the baseline commit and seven key extracted files against Git object IDs; checked the nine declared ESS domains, the counts of 44 crate manifests and 65 provider TOMLs, and the five concrete provider intake declarations. I inspected representative baseline definitions and paths for target selection, credentials, routes, Kubernetes discovery/context restoration, event declaration/production/retention/reads, Slack identity and append-before-ACK behavior, datasource bindings, SIP controls/destinations, generated-service identity, driver vocabulary, Git fetch, and SessionAuthority claims. The counts match, but counts do not prove semantic completeness, as NIR-003 demonstrates. I did not independently verify the predecessor checkout, every vendor schema or every crate line, live/provider readiness, released binaries, consumers, AEP validation results, or the claimed preservation of the interrupted diff. Those are not claimed as established by this review. No repository or planning files, integrations, or implementation were changed; only this task-local report was written.

```findings
- file: docs/design/23-domain-language-review.md
  line: 271
  category: event-identity-and-selection
  severity: blocker
  message: >-
    NIR-001 (High): Cross-receiver deduplication retains first-receiver provenance while subscriptions
    may select another receiver. Define logical stream membership independently of
    ingestion provenance, or change receiver-selection semantics explicitly, so intake
    races cannot silently determine consumer visibility.
- file: docs/design/23-domain-language-review.md
  line: 301
  category: session-ownership
  severity: warning
  message: >-
    NIR-002 (Medium): Incoming admission creates an offered Session before an accepter is selected, but
    the relationship table requires exactly one controlling principal for every admitted
    Session. Assign that principal at admission or specify state-dependent ownership
    and an authorized claim transition; identify the configuration that owns the policy.
- file: docs/design/23-domain-language-review.md
  line: 159
  category: domain-coverage
  severity: warning
  message: >-
    NIR-003 (Medium): The Event glossary omits the baseline's separate operational-event family and defines
    only provider-native/polled intake. Preserve DataEvent versus OperationalEvent, or
    an explicitly qualified Event family, and mark operational runtime support as
    declared-only without requiring implementation.
```
