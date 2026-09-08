# Discovery-profile and persistence review dispositions

The two initial immutable reviews contain eighteen individually owned findings. Reports and exact snapshots are retained in [review provenance](review-provenance.json). Each correction below has its own AEP outcome on its sole owner. Final review is separate; these are specification revisions, not runtime conformance.

## PP-A-01

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Receiver-owned adapter declaration map fixes contract/profile and source-selection rule; each invocation resolves one admitted source, and closed request input rejects profile/origin/namespace overrides. Aliases cannot change leaf meaning. Evidence: resources §4.5; PP-D01–05, [textual traces](traces.md) and [verification](verification.md).

## PP-A-02

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Argo whole exact name OR stable app.kubernetes.io/name label, ASCII lowercase and precedence are selected. Three broader old aliases are explicitly unselected for Argo. Nullable recognition/candidate and unrecognized rows are consistent; Argo never becomes callable. Evidence: resources §4.5 and Kubernetes/Grafana summaries; PP-D06–14, [textual traces](traces.md) and [verification](verification.md).

## PP-A-03

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Configured instance/connection/canonical HTTPS authority/trust-policy scope remains distinct from physical cluster attestation. Configuration changes require re-admission; known physical replacement invalidates continuity; unsupported global identity/deduplication is not claimed. Evidence: resources §4.5; PP-D15–18, [textual traces](traces.md) and [verification](verification.md).

## PP-A-04

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Design §31 assigns eighteen logical responsibilities once to named ports, with family links, state/decision ownership and failure handoffs. Deferred event/assignment/execution/deployment/telemetry state remains visible; no universal store is created. Evidence: design §31; PP-P01–25, [textual traces](traces.md) and [verification](verification.md).

## PP-A-05

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

The binding metadata group and mutation metadata group name the actual coupled compare/write/fence boundaries. Custody, audit, spend, transport nonce, provider and remote policy are distinct acknowledged handoffs; a disconnected backend cannot claim the profile. Evidence: design §31.1; PP-P03–05, PP-P08, PP-P12–21, [textual traces](traces.md) and [verification](verification.md).

## PP-A-06

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Separate leaf approval tombstones, transport nonce receipts, keyed reservations/attempts and per-hop audit retain their exact uniqueness, ordering and retention. Spend is not a dispatch gate, event receipt is not an event claim, and replay deletion cannot erase independent safety facts. Evidence: design §31–31.2; PP-P12–19, PP-P23, PP-P25, [textual traces](traces.md) and [verification](verification.md).

## PP-A-07

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

SessionRedemptionPort is separate from SessionSupervisorPort, data leases and execution approval. Restart loses live continuity; persistence cannot reattach devices or resurrect leases. Deferred checkpoint/job/assignment contracts do not become selected stores. Evidence: design §31; PP-P22–25, [textual traces](traces.md) and [verification](verification.md).

## PP-A-08

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-a-initial-20260908.

Inventory names the eleven actual ESS entities and distinguishes values/transient entities from missing persistent models. Custody no longer claims an invented CredentialSet lifecycle/Connection edge; connection status is derived from authoritative facts. No unproven owns edge or backend guarantee is introduced. Evidence: design §31.2; custody §9; connection §8; ess-entity-inventory.json; PP-P09–11, PP-P25, [textual traces](traces.md) and [verification](verification.md).

## PP-B-01

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Receiver-owned adapter declaration map fixes contract/profile and source-selection rule; each invocation resolves one admitted source, and closed request input rejects profile/origin/namespace overrides. Aliases cannot change leaf meaning. Evidence: resources §4.5; PP-D01–05, [textual traces](traces.md) and [verification](verification.md).

## PP-B-02

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Argo whole exact name OR stable app.kubernetes.io/name label, ASCII lowercase and precedence are selected. Three broader old aliases are explicitly unselected for Argo. Nullable recognition/candidate and unrecognized rows are consistent; Argo never becomes callable. Evidence: resources §4.5 and Kubernetes/Grafana summaries; PP-D06–14, [textual traces](traces.md) and [verification](verification.md).

## PP-B-03

Owner: story:contracts-discovery-profiles. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Configured instance/connection/canonical HTTPS authority/trust-policy scope remains distinct from physical cluster attestation. Configuration changes require re-admission; known physical replacement invalidates continuity; unsupported global identity/deduplication is not claimed. Evidence: resources §4.5; PP-D15–18, [textual traces](traces.md) and [verification](verification.md).

## PP-B-04

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Design §31 assigns eighteen logical responsibilities once to named ports, with family links, state/decision ownership and failure handoffs. Deferred event/assignment/execution/deployment/telemetry state remains visible; no universal store is created. Evidence: design §31; PP-P01–25, [textual traces](traces.md) and [verification](verification.md).

## PP-B-05

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

The binding metadata group and mutation metadata group name the actual coupled compare/write/fence boundaries. Custody, audit, spend, transport nonce, provider and remote policy are distinct acknowledged handoffs; a disconnected backend cannot claim the profile. Evidence: design §31.1; PP-P03–05, PP-P08, PP-P12–21, [textual traces](traces.md) and [verification](verification.md).

## PP-B-06

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Unknown writes/consume/spend/gate acknowledgements grant no send/entry/publication. Write-before-publish, audit-before-dispatch, nonce-before-entry and spend-before-gate are explicit; recovery reads the same owner and never fabricates certainty by retrying effects. Evidence: design §31.1; PP-P01–05, PP-P08, PP-P15–21, [textual traces](traces.md) and [verification](verification.md).

## PP-B-07

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Owner-specific retention remains distinct. Capacity refuses new work where safety facts cannot retire; refresh consumption, pending/unknown keys, approval tombstones and live nonces cannot vanish through GC/backup. Lost read caches do not supply authority/absence. Evidence: design §31.1–31.2; PP-P01–04, PP-P11, PP-P13–14, PP-P17–18, PP-P23, [textual traces](traces.md) and [verification](verification.md).

## PP-B-08

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Inventory names the eleven actual ESS entities and distinguishes values/transient entities from missing persistent models. Custody no longer claims an invented CredentialSet lifecycle/Connection edge; connection status is derived from authoritative facts. No unproven owns edge or backend guarantee is introduced. Evidence: design §31.2; custody §9; connection §8; ess-entity-inventory.json; PP-P09–11, PP-P25, [textual traces](traces.md) and [verification](verification.md).

## PP-B-09

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Public connection/F02/F03 semantic revision is distinct from private auth publication fence, material generation, discovery generation and route evidence revision. Same-identity refresh changes private fences/evidence without changing semantic fingerprint; real target/identity/meaning changes still invalidate it. Evidence: design §31.2; connection/acquisition/custody/operations prose and refresh ESS comment; PP-P06–07, [textual traces](traces.md) and [verification](verification.md).

## PP-B-10

Owner: story:contracts-persistence-ownership. Disposition: fixed. Source: review-result:profile-persistence-b-initial-20260908.

Custody errors/logs/metrics no longer expose private scope/version/generation refs. Internal access-controlled state stays separate from ordinary diagnostic export, which uses safe admitted correlation. Evidence: custody §4; design §31.2; PP-P24, [textual traces](traces.md) and [verification](verification.md).
