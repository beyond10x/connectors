# Local approval preparation and issuance

The CLI on current `main` can publish local write policy, prepare an exact approval
subject and issue a protected proof. Preparing a write or issuing its proof requires an explicitly configured
`connectors-local/2` adapter using `connectors-private/2`, admitted cached write
metadata, a saved connection, a local signing issuer and a configured bounded
clock. Policy status can inspect any configured adapter alias. GitLab's public
and private-version-one projections retain eleven reads; its explicitly selected
private-version-two projection adds the [guarded merge](local-gitlab-merge.md).
Issuance itself performs no provider write.

After [key initialization](local-approval-keys.md) and
[clock configuration](local-clock.md), inspect the retained policy:

```sh
connectors approvals policy-status --adapter forge --output json
```

Status is passive. Before the first publication, the optional policy member is
absent. Publish a closed JSON file containing `{"operations":["<write-operation>"]}`
with `approvals policy-set --adapter forge --input-file <file>`. Each operation
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
connectors approvals prepare --adapter forge \
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
connectors approvals issue --adapter forge \
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

The implementation is exercised through the generated production CLI with a
disposable qualified Secret Service and independent synthetic clock responder,
including keyring restart and locked custody. Deterministic tests also cover
policy changes during clock acquisition and failure after proof publication.
These issuance fixtures establish no provider effect or dedicated sandbox result.
The separate guarded-merge CLI fixture joins proof consumption, final
audit/attempt/connection gates and native dispatch. Its broader crash/failure
matrix and dedicated GitLab sandbox acceptance remain open under the active story.
