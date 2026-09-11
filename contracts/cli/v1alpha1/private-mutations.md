# Private mutation extension

This selects the guarded GitLab merge binding and extends
[private adapter ownership](private-adapter.md) and
[local mutation coordination](../../service/local-mutations.md). The explicit
configuration selection and private adapter transport below are implemented.
Production GitLab still uses the read-only projection; CLI approval issuance,
owner mutation IPC and the complete write coordinator remain implementation
work. The transport port alone does not admit a provider write.

The host configuration format `connectors-local/2` adds an adapter
`private_protocol` selection, exactly `connectors-private/1` or
`connectors-private/2`; it is required in this format. The existing
`connectors-local/1` reader retains its fields and implicit private version one.
It refuses the new field. Public service `protocol = "v1alpha1"` keeps its
existing meaning in both formats. No automatic configuration rewrite occurs.
The private selection participates in the executable selection digest. Unsupported
selection fails before startup; there is no negotiation fallback or silent
downgrade. Native configuration and public service formats do not change here.

Version two uses the same owned fd-3 Unix channel, owner/parent credentials,
nonce and child-incarnation challenge, bounded frame lengths and absolute
deadline plus original monotonic budget. Hello and Ready both carry the exact
selected private version. A version-two child may serve an explicitly requested
version-one session using its read-only descriptor projection; it does not send
new fields or write operations on that session. An old child rejects version two
before receiving credentials. A successful readiness response is checked against
the selected instance/configuration and exact executable as before.

Version-two bootstrap uses the existing closed bootstrap fields, including an
operation requirement's effect. This profile admits `Read` and required-approval
`Write`; `Unknown` remains unsupported. Every advertised Write requires the
host's full approval/audit/attempt binding, with no opt-out field. The full
private descriptor includes writes and has its own content-derived revision.
Version-one and public descriptors expose the read-only projection. An operation
missing from the selected projection is unknown, not a compatibility write path.

The additional closed control variants are:

| Request | Fields and following frames | Reply |
|---|---|---|
| `prepare_write` | `id`, `operation`, `revision`, `partition`, `deadline_ms`; then exactly one bounded secret frame and one input-document frame, as for existing invoke | `prepared_write` with the same `id` and a fresh canonical non-nil `preparation_id` UUID, or existing safe `failed` |
| `commit_write` | `id`, `preparation_id`; no secret or input frame | `write_result` with `id`, effect `applied`, `refused` or `unknown`, then one bounded result document; or channel loss/invalid reply, which the host treats as unknown |
| `cancel_write` | `id`, `preparation_id`; no extra frames | `cancelled_write` with matching `id` and `preparation_id`, only after pending state has been destroyed |

The existing `failed` reply retains `request_id` and its closed safe `code`, even
when answering `prepare_write`; it has no document. A `write_result` has exactly
one closed JSON document: `{"kind":"success","value":...}` for an applied safe
result, or `{"kind":"failure","code":...}` using the same closed failure codes.
Only `applied` permits a success document. The effect and safe result are separate:
an applied native effect can carry a failure to project, validate or bound the
result. No native error text crosses this boundary. Malformed framing, control,
document envelopes or correlation yields unknown effect. A well-formed, correlated
applied reply whose success value violates its declared schema retains applied
effect knowledge, discards that value and terminates the child.

The preparation ID names only the same request ID, child incarnation, operation,
revision, partition, canonical input digest and original deadline captured in
the same live exchange. The server retains the immutable typed native request
and credential; it never returns them as control metadata. A single pending
preparation occupies the child until exact commit/cancel or deadline/EOF. Other
requests, substituted IDs and a second commit are refused without a write and
close the exchange. Cancel acknowledgement proves destruction of an uncommitted
preparation, never rollback. Prepare failure has no business effect; credential
checks and declared native reads may already have happened.

The host sends commit only after definite current policy/key, audit, attempt,
approval-spend, credential dispatch and attempt-gate acknowledgements. It checks
the original remaining budget at every boundary. The child consumes pending
state before invoking its one-use write capability. It cannot accept an extended
deadline or replacement material. Deadline, cancellation or response loss after
the host gate remains possible-write uncertainty, including before a commit was
observed by the child. The existing private `invoke` always requires Read, even
on version two. No generic adapter method gains an ungated write.

The owner/CLI IPC gets a separately selected version-two envelope for write
inputs and observations; existing version-one requests stay closed. Its write
request carries adapter/connection/operation/schema/revision/deadline and the
optional bounded opaque idempotency key, followed by separately bounded business
input and protected approval-document frames. Proof bytes never enter ordinary
parser JSON or provider input. Reply mutation/audit fields use the already modeled
MutationObservation and SourceAudit types. Reads reject the write-only fields.
Version-one readers reject the version-two envelope; compatibility does not retry
an invocation using another protocol version.

Required executable cases include old peers in both directions, mismatched
readiness/version/descriptor, read-only legacy projections, missing approval,
substituted preparation/correlation, duplicate commit, cancelled or expired
preparation, EOF and exact-child cleanup, lost post-gate responses and restart
with a quarantined attempt and no second native write.
