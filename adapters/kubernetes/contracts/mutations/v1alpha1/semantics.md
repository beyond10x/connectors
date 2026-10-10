# kubernetes mutation binding/v1alpha1

**Status:** proposed native binding, not implemented. Implements the shared
[mutation contract](../../../../../contracts/operations/v1alpha1/semantics.md).
Exact native intent, preparation, dispatch and outcome interpretation are owned here.

### 4.1 Rollout restart intent and replay

`deployment.rollout_restart` selects **receiver-keyed idempotency** under [operations §§5.1–5.2](../../../../../contracts/operations/v1alpha1/semantics.md#51-key-namespace-fingerprint-and-replay-admission), retaining Kubernetes optimistic concurrency as an additional provider guard. Its closed input requires namespace, name, uid and resource_version, all nonempty exact strings within the ordinary request bound. Namespace/name must be valid single Kubernetes path components; UID is the Deployment identity, not Invocation.request_id or the host key. The admitted instance/connection fixes the configured cluster authority. No input can choose an API origin, refresh a credential identity, or substitute a new object under the same name.

This profile selects the built-in apps/v1 Deployment conditional-update binding evidenced at Kubernetes v1.35.0. Its resource_version input must be the canonical ASCII decimal representation of an integer in **1..18446744073709551615**: 1–20 digits, first digit 1–9, remaining digits 0–9, within that unsigned 64-bit bound. Zero, every all-zero alias, leading zeros on positive values, signs, whitespace, radix prefixes, non-ASCII digits and overflow are invalid_input at admitted operation-input validation, before key inspection or business dispatch. The host rejects these spellings rather than normalizing them. This rule is specific to the selected conditional Deployment binding, not a universal lexical rule for all Kubernetes resource versions or APIs. A different provider interpretation requires a separately established binding before advertisement.

The canonical approval/fingerprint binds that exact input and semantic connection/configuration/operation revisions. Mutation preflight checks the exact permission target `(patch, apps, v1, deployments, namespace, name, "")` under auth.evidence's mutation evidence rules. It does not fetch a new Deployment or refresh UID/resourceVersion. Accepted resourceVersion bytes are copied unchanged. The pinned provider compares parsed unsigned values; canonical positive admission makes that comparison identify the same admitted version without its zero/unconditional-update branch or alternate spellings. Numeric range validation does not choose a newer version or rewrite intent. No arithmetic, trimming, refetch or rebase may substitute a version. This is the chosen precondition contract, not a claim that current Kubernetes forbids all scoped version ordering.

For a candidate new attempt only, provider preparation fixes one timestamp marker from the host's current nonnegative epoch milliseconds, serialized as a canonical decimal string, and one exact patch body. This preserves the predecessor's decimal timestamp annotation, while making generation/fixity explicit. The marker is generated once for that candidate and associated with its immutable prepared intent; it never changes after the attempt is anchored. Approval authorizes the declared single timestamp-patch operation, including this receiver-controlled generation rule; the marker is not caller input, an authority token, a deduplication key or a unique-rollout promise. A generation-rule change is an operation-semantic revision. A concurrent loser discards its prepared candidate; matching host replay/wait never generates another marker or body. Recovery never sends an anchored request again.

Send one strategic merge PATCH to `/apis/apps/v1/namespaces/{namespace}/deployments/{name}` with `Content-Type: application/strategic-merge-patch+json`, metadata.uid and metadata.resourceVersion copied exactly, and only `spec.template.metadata.annotations["kubectl.kubernetes.io/restartedAt"]` set to the fixed marker. No server-side apply/force, alternate object, automatic version refetch/rebase or follow-up rollout polling is implicit. Permission checks and PATCH share the original 15 s provider / 20 s service budget and 4 MiB response bound.

A definitive successful API acknowledgement must contain the exact Deployment namespace/name/UID, a nonempty resulting resourceVersion and the requested annotation value. Return `{namespace, name, uid, resource_version, patch_accepted: true}` with applied. This proves the accepted patch intent only, not rollout convergence, healthy Pods, or even a fresh template value if an equal marker was already present. A timestamp is not guaranteed unique. Unexpected identity/value, malformed/oversized response, transport loss or generic provider error after possible dispatch is unknown/outcome_unknown. For a binding-verified precondition refusal that proves this PATCH was not applied, use refused with a safe invalid_input error; preserve safe not_found/forbidden for their verified refusal cases. Do not copy the predecessor's broad 409/422 mapping as universal no-effect proof. A code alone or an unrelated later conflict does not settle an earlier attempt.

| Request history | Required meaning |
|---|---|
| Same live host key and same full fingerprint | observe original pending/terminal/quarantined reservation under current result admission; no new marker, approval spend or PATCH |
| Same live key, changed namespace/name/UID/version or semantic binding | idempotency_conflict; no stored-input/axis disclosure or new PATCH |
| Original PATCH succeeded and advanced the version; a deliberate new key uses original UID/version | API precondition can refuse this later PATCH; that refusal alone cannot determine whether the original or another actor advanced the version |
| Original reply lost; current version still matches original | an independently admitted new key could apply a PATCH; do not assume all repeats must conflict or must cause another rollout |
| Same name now identifies another UID | exact UID guard prevents silently restarting the replacement; do not remove the guard or refetch a successor |
| Caller intentionally reads a new version, forms new canonical input and obtains fresh admission/approval/key | new mutation intent, not a replay or reconciliation of the earlier unknown attempt |
| Later read matches marker or reports healthy Pods | present state is not proof of which exact earlier request executed; no automatic settlement/retry |
| Known-result key expires, or an unknown reservation remains quarantined | §5.1 retention and fresh-admission rules apply; expiry does not unspend approvals or reopen unknown reservations |

The retained provider evidence (`restart-visibility-20260908`, `provider-evidence.md`) pins the old implementation at 81459ac4, official Kubernetes conditional update documentation, immutable-UID validation, unsigned version parsing, Deployment's unconditional-update strategy and the strategic PATCH/store path. The evidence supports the selected positive-version precondition mechanism; it does not execute a provider race or prove all server/admission-plugin behavior. The concrete binding must establish its exact success/no-effect evidence before advertisement. Existing AttemptRecord/KeyReservation lifecycles and the rule against reclassifying a settled Indeterminate attempt remain unchanged.

### 4.2 Pod exec (`pods.exec`)

**Status:** specified and implemented in the adapter library and the local
composition, against recorded streams: with `pod_exec: true` the local runtime
lists it on the `connectors-private/2` write exchange and runs it over the host's
upgraded-stream write capability (below). The federated service never lists it.

`pods.exec` runs one command in one named container of one pod, under the
execution-family rules selected on 2026-10-03 for bounded process execution:
it needs an approval the record names, produces one attempt, carries the command as an explicit argument vector
with no shell added, is bounded in output bytes and in time, and reports an
unknown outcome as unknown. Its types are `PodExecRequest`, `ExecChannel`,
`PodExecResult` and `PodExecOutcome` in the
[`mutations` ESS domain](../../../spec/ess/domains/mutations.yaml).

Declaration: contract `operations/v1alpha1`, profile `mutation`, effects
`[external_write, network, process]`, `idempotency: none` (running a command twice
runs it twice; there is no receiver key and no replay), `approval: required`. It is
advertised only when the configuration sets `pod_exec: true` and only on the
`connectors-private/2` write exchange; the read exchange never lists it, and the
read path refuses it (`forbidden`, zero requests) even when it is configured.

**Input.** Closed: `namespace` (configured, checked before any I/O, `forbidden`
otherwise), `pod` (DNS-1123 subdomain), `container` (DNS-1123 label, required:
no default-container fallback), `command` (1–64 elements, each 1–4096 bytes, at
most 16384 in all; element one is the executable), `timeout_seconds` (1–60,
default 30) and `max_output_bytes` (1–1048576, default 65536, applied to stdout
and stderr each). Approval and the fingerprint bind that exact input.

**Dispatch.** After the host has spent the approval and anchored the attempt, one
GET to `/api/v1/namespaces/{namespace}/pods/{pod}/exec` with `container`, one
`command` term per element in order, `stdin=false`, `stdout=true`,
`stderr=true`, `tty=false`, upgraded to a WebSocket offering
`v5.channel.k8s.io` then `v4.channel.k8s.io`. Both frame each binary message
with a channel byte (1 stdout, 2 stderr, 3 status) and end with a v1 `Status` on
channel 3 for success too. Older subprotocols, which report errors as text, are
not offered. The upgrade request is never resent.

**Outcome.**

| Observation | Outcome |
|---|---|
| Refused before sending | `refused` with that error |
| Upgrade answered 400/422, 401, 403 or 404 with no stream | `refused` (`invalid_input`, `unauthorized`, `forbidden`, `not_found`): no process was started |
| Upgrade answered 5xx or another status | `unknown` |
| Request sent, answer lost | `unknown` |
| Stream accepted with an unoffered subprotocol | `unknown` |
| Status `Success` on channel 3 | `applied`, `exit_code: 0` |
| Status `Failure`, reason `NonZeroExitCode`, one `ExitCode` cause 1–255 | `applied` with that `exit_code`: a completed attempt, not a refusal |
| Any other status, a close or loss before a status, a message on channel 0, 4 or another channel, a status over 64 KiB, more than 16 MiB consumed, the deadline | `unknown` |

`stdout` and `stderr` keep the first `max_output_bytes` bytes of their channel;
the rest is read and discarded so the status is still observed, and
`stdout_truncated`/`stderr_truncated` say so. A cut never splits a UTF-8 scalar
(an incomplete trailing scalar is withheld); other invalid bytes are replaced
with U+FFFD. The result is `{namespace, pod, container, exit_code, stdout,
stderr, stdout_truncated, stderr_truncated, provenance}`. An `unknown` attempt is
never retried or settled later by this binding; a caller who wants the command
again obtains a fresh approval.

**Transport.** The guard is the one every implemented mutation uses: the
composition implements `prepare_write` for the `connectors-private/2` exchange
and returns a prepared write that consumes the host's one-use write capability,
built for one upgraded stream (`ScopedHttp::into_upgrade_write`). Preparation
refuses before any I/O (namespace, input bounds, protected entry); the upgrade is
sent only when the host commits the write it admitted, so the approval is spent
and the attempt recorded first, and a cancelled preparation opens nothing. The
upgrade (`connectors_sdk::AuthenticatedWrite::upgrade`, modelled as
`connectors.transport.UpgradeOutcome` and `UpgradedStreamEnd` in the shared
`ess/domains/transport.yaml`) goes through the host's admitted HTTP path: the
configured API server only, the captured TLS roots and credential, no proxy, no
redirect followed. Only the WebSocket framing is added (`tokio-tungstenite`, with
no TLS or connect feature). The host answers ping, reassembles fragments and
enforces the bounds the composition fixed: 1 MiB per message, 17 MiB in all
(above this binding's 16 MiB consumable bound, which it reports itself) and the
request's `timeout_seconds`. A bound reached closes the stream and is reported as
truncation (`capacity`), the deadline as `timeout`; both are `unknown` here. The
composition hands the host's message stream to `exec::settle` as an
`ExecStream`. A read capability has no upgrade.
