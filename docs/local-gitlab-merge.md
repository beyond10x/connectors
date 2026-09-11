# Guarded GitLab merge in the development checkout

The local CLI can call `merge_request.merge` through the approval, audit and
mutation coordinator. This development work extends the v0.2.0 source release;
it is not a completed GitLab provider batch. Dedicated sandbox acceptance,
owner-crash failure cases and the wider write failure matrix remain open.

Start with the [saved GitLab connection guide](local-gitlab-cli.md). Deliberately
select `format = "connectors-local/2"` in the host configuration and
`private_protocol = "connectors-private/2"` on each configured adapter. Select the
exact new executable hash, permit `merge_request.merge` in the adapter's
operation permissions and retain `gitlab.pat` in its profile permissions. The
saved PAT must grant GitLab `api` scope. Use a fresh explicit local selection and
an admitted connect to collect its descriptor before discovering the new metadata.
The native configuration format stays `connectors-gitlab-local/1`.

Version one and the public service still expose eleven reads. There is no
automatic configuration rewrite, credential migration or fallback write path.
Keep the previous executable available until the new selection is usable.

Configure a [bounded authenticated clock](local-clock.md), initialize
[approval-signing keys](local-approval-keys.md) and use the
[approval commands](local-approvals.md) to publish a policy file containing:

```json
{"operations":["merge_request.merge"]}
```

Discover the exact `schema` and descriptor `revision` using
`connectors operations describe --adapter forge --operation merge_request.merge`.
Put the intended business input in `merge.json`, using a project from the native
allowlist, a project-local MR IID, its exact lowercase source SHA and the expected
successful head pipeline ID:

```json
{"project":"group/project","iid":17,"sha":"0123456789abcdef0123456789abcdef01234567","pipeline_id":123}
```

Use `approvals prepare` with this input and those exact selectors. Inspect its
subject and pass that subject's digest to `approvals issue --approve-subject`.
Issue to a new absolute `--proof-output` path in a private directory. Ordinary
output contains metadata; the proof stays in the owner-private file.

Invoke the same target and input:

```sh
connectors operations invoke --adapter forge \
  --connection "$connection" --operation merge_request.merge \
  --schema "$schema" --revision "$revision" --input-file merge.json \
  --approval-file "$proof_file" --idempotency-key "$business_key" --output json
```

The optional key is an opaque string of 1–256 bytes. Reuse the exact key for
observation of this invocation. A conflicting input refuses without disclosing
the original result. The optional proof-file argument supports exact replay
without another proof; a new attempt always requires a valid proof. Reads refuse
both write-only options. The original 20-second write budget includes argument
processing, document/proof acquisition, startup, preflight and commit.

Fresh preflight checks the same MR/head/pipeline conditions as
`merge_request.validate`. Commit sends one PUT with `sha`, `auto_merge:false`
and `should_remove_source_branch:false`. It omits squash and follows the project's
configuration. GitLab checks the source SHA atomically; expected pipeline ID and
status are preflight observations, not an atomic merge condition.

Success retains the usual JSON-text `result` and adds `request_id`, `mutation`
and `source_audit`. Failure carries those observations inside `error.data` when
available. Inspect `mutation.classification`: `not_attempted`, `refused`,
`applied` or `unknown`. Applied can accompany a result-delivery error; incomplete
audit cannot undo the known effect. A lost response is unknown and never triggers
another PUT. Repeating an exact retained key under current result-access policy
observes the original attempt without provider, credential, proof or clock access.
Every delivery has fresh request/audit correlation. A pending original remains
unknown; observing it does not authorize takeover or resend.

Disposable production CLI fixtures prove applied/refused/lost-response behavior,
same-key observation after owner shutdown and no duplicate PUT. They do not
establish real GitLab sandbox behavior or full crash-recovery qualification.
