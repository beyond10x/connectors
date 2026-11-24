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
names one instance, one operation, what each target member must hold (an exact value, a
pattern or one of a set of values), the input
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
  with a value the member admits. A member holds exactly one of three keys: `value`, the
  exact value (it may be `null`); `pattern`, a regular expression the whole string must
  match; `one_of`, 1 to 64 distinct values of which the input holds one ("A grant for many
  targets", below). A number in a `value` or a `one_of` is an integer from
  -9223372036854775808 to 18446744073709551615: `grant-standing` refuses any other number
  (a fraction, an exponent, `-0`) as `invalid_input`, a grant file that holds one does not
  verify, and a write whose input holds one at a target pointer is covered by no grant.
- `input_keys`, at most 16 pointers, closes the input: a write that sets any other member
  is outside the grant. Without `input_keys` the rest of the input is open.
- `not_after_unix_ms` has no upper bound and `max_per_hour` is 1 to 10,000. Both are read
  against the trusted clock, not the system clock.
- `grant-standing` answers `{grant_id, path}`, creates the file with mode 0600 or replaces
  it, and drops the grants that expired. `list-standing` prints the grants of the file.
  `revoke-standing` removes one grant; nothing new is admitted under it afterwards.
- `grant-standing` and `revoke-standing` answer `outcome_unknown` when the host could not
  confirm that the new file is durably in place: the path holds the new file or the old
  one, and `list-standing` shows which.

### A grant for many targets

A caller whose target changes on every call, such as a step that moves each ticket of a
release, needs no grant per ticket. A `pattern` member admits every string that matches it
whole, and a `one_of` member admits each value it lists. One grant then covers all those
writes under one `max_per_hour`. These two grants cover the Jira writes of such a step, the
transitions and the comments of the tickets of project `DEV`:

```json
{
  "operation": "issue.transition.run",
  "target": [
    {"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
    {"pointer": "/target_status", "one_of": ["10000", "10235", "3", "10234", "10223", "10001"]}
  ],
  "input_keys": ["/current_status", "/body/transition/id"],
  "not_after_unix_ms": 1798761600000,
  "max_per_hour": 500
}
```

```json
{
  "operation": "issue.comment.add",
  "target": [
    {"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
    {"pointer": "/body/body/content/0/content/0/text", "pattern": "^Release review of (?s:.*)$"}
  ],
  "input_keys": ["/body/body/content", "/body/body/type", "/body/body/version"],
  "not_after_unix_ms": 1798761600000,
  "max_per_hour": 100
}
```

- A pattern is written in the syntax of the Rust `regex` crate, starts with `^` and ends
  with `$`, and must match the whole value: `^DEV-[0-9]+$` admits `DEV-12` and refuses
  `DEV-12x`, `OPS-12` and a number. Only a string can match. Matching is case-sensitive, and
  `\d`, `\w`, `\s` and `\b` are Unicode-aware, so write `[0-9]` for ASCII digits. `.`
  matches every character but U+000A, so it also matches a carriage return and the other
  line breaks of Unicode; `(?s:.*)` matches U+000A too.
- A pattern folds case only with Unicode matching off. Write the fold as a group around
  the letters only: `^(?i-u:sk)-[0-9]+$` folds `s` and `k` over the ASCII letters. Behind a
  bare `(?i-u)`, and inside the group, the pattern is in byte mode: `\d`, `\w` and `\s` are
  ASCII only, and `.`, a negated class such as `[^/]` and `\W` are refused, as the `regex`
  crate refuses them. Every class that can match a byte outside ASCII is refused there,
  also one written inside another class: `\D`, `\S`, a negated POSIX class such as
  `[[:^alpha:]]` and `[^\W]` are refused as `.`, a negated class and `\W` are, and a
  difference from an ASCII class such as `[\x00-\x7f--a]` is taken. So `^(?i-u)dev$` is
  taken, and `^(?i-u)Release review of (?s:.*)$` is refused: write
  `^(?i-u:release review of )(?s:.*)$`. A pattern that folds while
  Unicode matching is on, such as `^(?i)dev$`, `^(?i:dev)$` or `^(?i-u)de(?u)v$`, is
  refused: the Unicode case fold also admits look-alikes (U+017F for `s`, U+212A for `k`).
- A pattern is matched against a value of at most its `max_value_bytes`; a longer string
  is outside the grant, whatever the pattern would answer. The figure falls as the
  pattern's compiled program grows: `^DEV-[0-9]+$` is matched against 22,055 bytes,
  `^Release review of (?s:.*)$` against 8,192 and `^(?i-u:sk)-[0-9]+$` against 23,405.
  `list-standing` shows it for each pattern member. A pattern that needs more bytes than
  it is matched against, such as `^[a-z]{906}$`, covers nothing, and `grant-standing`
  refuses a pattern that is matched against under 64 bytes.
- The transition grant is closed: a transition that also sets `/body/fields` or
  `/body/update` is outside it. A ticket of another project and a status outside the list
  are outside it too.
- The comment grant bounds the first text node only: the first paragraph of the comment
  must start with `Release review of `. What follows that node (more text, more
  paragraphs, links, mentions, media) is free, because `/body/body/content` is listed
  whole. A comment whose first node is not that text, or that has no content, is outside
  the grant.
- A target pointer may point into an array (`/content/0/...`) on a closed grant only when
  `input_keys` lists the array: an `input_keys` pointer may lie above a `target` pointer
  when the next part of the target pointer is an array index. No other two pointers of a
  closed grant may lie one above the other.
- Bounds: a pattern is at most 256 bytes and its compiled program at most 65,536 bytes; a
  grant file holds at most 64 pattern members, counted after `grant-standing` dropped the
  expired grants. `grant-standing` refuses a document outside these bounds as
  `invalid_input`. A counted repetition of a Unicode class passes the compiled-size limit
  quickly: `^\w{2}$` and `^\pL{2}$` are refused, and `\d` is taken up to 12 repetitions.
  Write the ASCII class where ASCII is meant: `[A-Za-z0-9_]` is taken up to 680
  repetitions.
- The hourly cap counts every write under the grant, whichever ticket it matched; a replay
  of a key counts none. The grant ends at `not_after_unix_ms` and at `revoke-standing`, as
  any grant does.
- `list-standing` shows each member with the key it was written with, and a pattern
  member also with `max_value_bytes`.
- A binary older than 0.78.0 does not read a file with a `pattern` or `one_of` member, or
  with an `input_keys` pointer above a `target` pointer: its `invoke` answers
  `approval_required` and its three commands `invalid_configuration`. A file with `value`
  members only reads on both.

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
  count cannot be kept, the state directory holds no use store, or the clock gives no
  reading.

The same key with the same input replays the original answer with no grant check, also
after the grant expired, was revoked or reached its cap.

The size of a grant. The grant file, the key that signs it and the state directory
belong to the operator's UID, so a process of that UID can grant, as it can issue a
proof. One proof admits one write; one grant admits up to 10,000 writes an hour until
its expiry. A grant bounds what a caller that only invokes can write. Rotating or
revoking the approval key ends every grant it signed. A revoked grant comes back when an
older signed file is put back at the path (a backup, a file sync or a snapshot restore
running as the operator can do that); rotating the key ends it for certain.

The hourly count is the file `standing-grants.sqlite3` in the state directory.
`grant-standing` creates it and nothing else does. Removing it frees no cap: the owner
answers every write a grant covers `unavailable` until the key holder runs
`grant-standing` again, and the count then starts at zero. A process of the operator's
UID that replaces the file or edits its rows can free the cap; that is the same-user
boundary, which a grant does not close.

The implementation is exercised through the generated production CLI with a
disposable qualified Secret Service and independent synthetic clock responder,
including keyring restart and locked custody. Deterministic tests also cover
policy changes during clock acquisition and failure after proof publication.
These issuance fixtures establish no provider effect or dedicated sandbox result.
The separate guarded-merge CLI fixture joins proof consumption, final
audit/attempt/connection gates and native dispatch. Its broader crash/failure
matrix and dedicated GitLab sandbox acceptance remain open under the active story.
