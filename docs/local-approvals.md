# Local approval preparation and issuance

The CLI on current `main` can publish local write policy, prepare an exact approval
subject and issue a protected proof. Preparing a write or issuing its proof requires an explicitly configured
instance served under `connectors-private/2` (every derived catalog instance is; a pinned one states `private_protocol = "connectors-private/2"`), admitted cached write
metadata, a saved connection, a local signing issuer and a configured bounded
clock. Policy status can inspect any configured instance. GitLab's public
and private-version-one projections retain eleven reads; its explicitly selected
private-version-two projection adds the [guarded merge](local-gitlab-merge.md).
Issuance itself performs no provider write.

Connector, instance and connection are the terms of the [vocabulary](../README.md#vocabulary).

After [key initialization](local-approval-keys.md) and
[clock configuration](local-clock.md), inspect the retained policy:

```sh
connectors approvals policy-status --connector gitlab --instance forge --output json
```

Status is passive. Before the first publication, the optional policy member is
absent. Publish a closed JSON file containing `{"operations":["<write-operation>"]}`
with `approvals policy-set --connector gitlab --instance forge --input-file <file>`. Each operation
must be an admitted required-approval write allowed by the configured operation
and credential-profile permissions. Reads and unknown operations are refused.
Only file input is supported for this management command. The generated binding
uses an ordinary path option because pinned `ess-cli/1` document sources require
all three carriers; the production handler supplies bounded file acquisition.

The first publication requires no expected revision. Later changes require
`--expected-revision <current-integer>`. An empty operation list revokes new uses.
Policy identity is stable and revisions increase; failed comparison preserves the
current policy. Changing configuration, cached descriptor, executable selection
or clock selection requires deliberate policy republication.

For an admitted write, use exact connection, operation, schema and descriptor
revision values to prepare the intended input:

```sh
connectors approvals prepare --connector gitlab --instance forge \
  --connection "$connection" --operation "$operation" \
  --schema "$schema" --revision "$revision" --input-file request.json \
  --output json
```

Preparation returns the canonical subject, its SHA-256 digest, presentation and
issuer/audience policy. It validates the original JSON and native write schema,
including format constraints. It never reads a credential, contacts the provider
or clock, starts a service, updates the registry clock, reserves a business key
or spends an approval. Expired validation evidence can still identify the exact
retained connection; preparation does not establish readiness to call it.

Canonical subjects retain every nullable coordinate as an explicit `null`. The
delegation model uses named nullable values so the schema requires those members;
ordinary optional metadata fields keep their existing omission-based projection.

Issuance reconstructs the subject independently. Use its exact approved digest
and a new absolute output path inside an existing private directory:

```sh
connectors approvals issue --connector gitlab --instance forge \
  --connection "$connection" --operation "$operation" \
  --schema "$schema" --revision "$revision" --input-file request.json \
  --approve-subject "$subject_sha256" --proof-output "$new_private_file" \
  --output json
```

The explicit owner command authorizes that exact subject under the current local
policy. This supports deliberate machine issuance and makes no claim about an
independent human review. A prepared document or presentation label supplies no
authority. Prepare and issue also accept one of `--input-json` or `--input-stdin`
for their nonsecret business input, sharing the original 20-second helper budget.
Preparation checks its 272 KiB request and 64 KiB result limits and independently
bounds the projected target at 256 KiB. Later invocation must check its own full
envelope, including proof, again.

Issuance acquires fresh authenticated time before policy/key leases, reconstructs
the current subject afterward, reads the qualified seed and signs the fixed
300-second proof profile. It holds current policy/key leases through publication
and acknowledgement. The new mode-0600 file contains only `{reference,evidence}`;
file and directory synchronization precede success. Ordinary output contains only
the safe reference, subject digest and `published` disposition. Existing files or
links are never overwritten. Locked custody or changed admission refuses issuance.
An uncertain acknowledgement can leave a protected file; the CLI neither deletes
it nor issues a replacement automatically.

## Standing grants

A proof admits one write of one input. A standing grant admits every keyed write it
covers, with no proof per write, until it expires. The operator signs it once. A grant
names one instance, one operation, the exact value of each target member, the input
members a caller may set beside them, an expiry and a cap of writes per hour. The rules
are in the [CLI contract](../contracts/cli/v1alpha1/semantics.md#standing-grants).

An instance takes grants only when its entry names its grant file by an absolute path.
The member is optional, changes no policy and no key, and a binary built before it
existed refuses a configuration that carries it:

```toml
[connectors.slack.instances.bot]
standing_grants = "/absolute/private/slack-bot.grants"
```

The key, the clock and a published policy that lists the operation stay prerequisites,
as for a proof. The grant is not part of the `policy-set` document. Write the grant to a
file and sign it:

```json
{
  "operation": "chat.postMessage",
  "target": [{"pointer": "/body/channel", "value": "C0000000001"}],
  "input_keys": ["/body/text", "/body/thread_ts"],
  "not_after_unix_ms": 1798761600000,
  "max_per_hour": 20
}
```

```sh
connectors approvals grant-standing --connector slack --instance bot --input-file grant.json --output json
connectors approvals list-standing --connector slack --instance bot --output json
connectors approvals revoke-standing --connector slack --instance bot "$grant_id" --output json
```

- `target` holds 1 to 16 members; a write is covered only when its input has each pointer
  with exactly that value.
- `input_keys`, at most 16 pointers, closes the input: a write that sets any other member
  is outside the grant. Without `input_keys` the rest of the input is open.
- `not_after_unix_ms` has no upper bound and `max_per_hour` is 1 to 10,000. Both are read
  against the trusted clock, not the system clock.
- `grant-standing` answers `{grant_id, path}`, creates the file with mode 0600 or replaces
  it, and drops the grants that expired. `list-standing` prints the grants of the file.
  `revoke-standing` removes one grant; nothing new is admitted under it afterwards.

A keyed write with no `--approval-file` then runs under the grant:

```sh
connectors --output json invoke slack/bot chat.postMessage --input-stdin --idempotency-key "$key" < post.json
```

The answer is that of any guarded write, and `mutation.standing.grant_id` names the
grant, also on a replay of the key and on a failure that has an attempt. A write with
`--approval-file` stays a per-input write: a refused proof is final and never falls back
to a grant. An `--approval-file` that holds no byte is answered as no `--approval-file`
is. Without `--idempotency-key`, or on an instance without `standing_grants`, a write
without a proof is `approval_required` as before.

What the grant refuses, each before any provider request and with no attempt recorded:

- `approval_required`: no grant covers the write. The answer does not say why; another
  operation, another target value, an input member outside `input_keys`, an expired or
  revoked grant, a file that does not verify and no file at all answer alike. The reason
  is in the owner's log.
- `rate_limited` with `retry_after_seconds` (1 to 3,600): every covering grant admitted
  `max_per_hour` writes in the last hour.
- `unavailable`: the grant file or its directory is not private to the operator, the use
  count cannot be kept, or the clock gives no reading.

The same key with the same input replays the original answer with no grant check, also
after the grant expired, was revoked or reached its cap.

The size of a grant. The grant file, the key that signs it and the state directory
belong to the operator's UID, so a process of that UID can grant, as it can issue a
proof. One proof admits one write; one grant admits up to 10,000 writes an hour until
its expiry. A grant bounds what a caller that only invokes can write. Rotating or
revoking the approval key ends every grant it signed. A revoked grant comes back when an
older signed file is put back at the path (a backup, a file sync or a snapshot restore
running as the operator can do that); rotating the key ends it for certain.

The implementation is exercised through the generated production CLI with a
disposable qualified Secret Service and independent synthetic clock responder,
including keyring restart and locked custody. Deterministic tests also cover
policy changes during clock acquisition and failure after proof publication.
These issuance fixtures establish no provider effect or dedicated sandbox result.
The separate guarded-merge CLI fixture joins proof consumption, final
audit/attempt/connection gates and native dispatch. Its broader crash/failure
matrix and dedicated GitLab sandbox acceptance remain open under the active story.
