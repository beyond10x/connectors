---
format: aep.planning-md/3
id: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
kind: story
status: active
title: A keyed write through the local CLI is admitted under a standing grant
summary: 'invoke with --idempotency-key and no --approval-file runs under a signed grant file: one instance, one operation, exact target, closed input, expiry, hourly cap'
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:standing-grant-closes-the-input-to-listed-members
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/slack_standing.rs
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/approvals.rs
- confidence: cited
  path: apps/connectors/src/local/invoke.rs
- confidence: inferred
  path: apps/connectors/src/local/migrate.rs
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: inferred
  path: apps/connectors/src/local/select.rs
- confidence: cited
  path: apps/connectors/tests/standing_grant.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/private-mutations.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/local-mutations.md
- confidence: inferred
  path: crates/connectors-build/generated/metadata-emissions
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: inferred
  path: crates/connectors-host/src/local/approval_keys.rs
- confidence: inferred
  path: crates/connectors-host/src/local/approvals.rs
- confidence: inferred
  path: crates/connectors-host/src/local/approvals/proof.rs
- confidence: inferred
  path: crates/connectors-host/src/local/approvals/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/execution.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/standing.rs
- confidence: cited
  path: docs/catalog-slack.md
- confidence: cited
  path: docs/local-approval-keys.md
- confidence: cited
  path: docs/local-approvals.md
- confidence: cited
  path: docs/running-services.md
- confidence: cited
  path: ess/domains/approval_issuers.yaml
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/local_approval_policy.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
- confidence: cited
  path: ess/domains/standing_approvals.yaml
- confidence: inferred
  path: website/docs/connectors/slack.md
- confidence: cited
  path: website/docs/guides/guarded-write.md
- confidence: inferred
  path: website/docs/reference
revision: 62
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T12:03:26Z", actor: "human:timo", revision: 61, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-11-23T12:03:27Z", actor: "human:timo", revision: 62, decided_on: {"recorded":{"review_outcome":8}}}
---
## Outcome

A keyed write through the local CLI is admitted under a standing grant its operator signed once:
`connectors --output json invoke <connector>/<instance> <operation> --input-stdin
--idempotency-key <key>`, with no `--approval-file`. The answer names the grant. A call the grant
does not cover makes no provider request and records no attempt. First consumer: the `inbox`
bridge on the operator's workstation, posting `chat.postMessage` into one Slack channel as a bot.

## Source

- inbox's ask: its bridge sends unattended; nobody is there to issue an approval per message.
- D8 = B (the operator, 2026-11-23): the record stays as it is; a grant is a signed grant file,
  issued with `connectors approvals grant-standing`, ended with `connectors approvals
  revoke-standing <grant id>`, with required `not_after` and `max_per_hour`; grants do not go
  into the `approvals policy-set` document.
- D10 = A (the operator, 2026-11-23): the attempt is recorded as the served host records it,
  `approval_mode` `not_required`, the grant id bound in `standing-grants.sqlite3` and named in
  the answer; no ledger schema change.
- inbox, 2026-11-23: a refusal is `not_attempted` with `approval_required` and no axis; over the
  cap `rate_limited` with `retry_after_seconds`; `--idempotency-key` keeps its meaning; the
  answer names the grant id; no byte bound; it reads the JSON paths and the exit status.
- `architecture-decision-record:standing-scoped-approval` (accepted), decisions 1-3, 5 and 6.
- story:standing-grant-closes-the-input-to-listed-members supplies `input_keys`; this story
  depends on it. story:standing-write-approval-for-unattended-agents keeps the container part
  and depends on this story, which is the owner path on a plain state directory.

## What the tree holds today

Read at 5983a6b9502 (0.75.0). `host/` is `crates/connectors-host/src/local/`, `cli/` is
`apps/connectors/src/local/`. Lines marked (critic) were read by a critic of round 1 or 2, not
by the drafting session.

- The CLI gate refuses a write without `--approval-file`: `cli/invoke.rs:1155-1160`
  (`approval_required`, stage `admission`, `next_action` `none`, `approval_steps`); bytes pinned
  by `apps/connectors/tests/invoke_short.rs:319-342`. The gate tests `approval_file.is_none()`
  and does not read the file.
- The CLI's write path, `cli/operations.rs`: keyed lookup `observe_original` :223, in the CLI
  process, before any file is opened or any owner started; a settled key returns there :224-228;
  a pending key sends no proof :233-234; the approval file is read at :237 with
  `host/protected.rs` `file_until` (it accepts an empty file, (critic) :286-308); a read error
  looks the key up once more :244-249; `owner::WriteClient::connect` :251 starts the owner.
  (critic) `WriteClient` has this one user.
- The owner reads an empty proof frame as no proof: `host/owner/transport.rs:2640` (critic).
- Owner executor, `host/owner/mutation/execution.rs`: `execute` :142 takes `Invocation{request,
  key, proof, until}` :17-22 and a live `runtime::Child`; `resolve` :156, keyed replay :158,
  proof required :167, clock :169 (a `BoundedClock`, (critic)), proof verified :172 (`verify`
  :97-119: policy, key, subject), audit anchor :274, native preflight :277 (a provider request;
  Slack `auth.test`). The proof is verified again in the closure at :366-375 before
  `prepare_approved` :386, and in the closure at :420-441 around the spend :424-432; the `keys`
  lease is held from :331 (critic). The gate is `open_approved_dispatch` :469. After the gate
  won, :500-511 checks the deadline, verifies the proof once more and consumes the winner; a
  failure there cancels the native commit and settles `unknown`; the comment at :498-499 allows
  no network, custody lookup or retry between the gate and the commit. An `unknown` outcome is
  not sent again :474-490. One caller: `host/owner/transport.rs:4624-4631`. No test calls
  `execute` in process.
- `resolve` parses the input with `runtime::channel::depth` (64 levels, `host/runtime/
  channel.rs:376-378`) and `connectors_core::read_json`, which refuses duplicate member names:
  `host/owner/approval_issuance.rs:221-223`. `resolve` and the replay lookup both require a
  published approval policy that lists the operation: `approval_issuance.rs:257-264`,
  `host/owner/mutation.rs:681-686`.
- The grant model serves the served host only: `host/standing.rs` `Uncovered` :67 with `rank`
  :243 (an exhaustive match), `Grant` :97 (it names its instance), `read` :122, `parse` :130,
  `covering` :191 (returns the covering grants in file order), file custody `admitted` :265,
  `Uses` :400 (`standing-grants.sqlite3` :50; `open` :412 always creates the store; `take` :473
  answers `AtCap{retry_after_seconds}`; `bind` :527, `release` :543, `grant_of` :554; only rows
  that are not bound are pruned); caller `host/service.rs:456-470`.
- Ledger: `host/mutations.rs:123` (`prepare`), :267 (`open_dispatch`, requires
  `approval_mode='not_required'` :300-301); `host/metadata/mutations.sql:12`, :21 (three modes;
  `approval_ref` is barred with `not_required`); the mode is also a member of the Entity Runtime
  definitions (`host/metadata/er.rs:307`). (critic) The mode is dispatched on only at
  `mutations.rs:300` and :552; the candidate's default is `not_required` (:382) and
  `execution.rs:376` sets `Required`, so a local ledger holds no `not_required` row for a write
  whose policy requires approval; the ledger keeps 10,000 attempts per instance
  (`host/mutations/types.rs:189`).
- Approval commands run in the CLI process and start no owner
  (`apps/connectors/src/local.rs:293-301`); declared in `apps/connectors/spec/cli.yaml:184-338`.
  `approvals issue` takes `--approve-subject`; `approvals policy-set` takes no confirmation.
- Signing: one Ed25519 approval key per instance (`host/approval_keys.rs` `acquire_key` :486,
  `with_signer` :641, which needs the keyring; lock `Lease::acquire` :668 on the file
  `approval-issuer.lock` :23); compact JWS in `host/approvals/proof.rs`: `typ` :13, canonical
  check :69, key window (critic) :104-122, `verify` :171, `Signer` :185, `issue` :197;
  `ConfiguredApprovalKey` `host/approvals/types.rs:76`. Tests sign in process with a test key:
  `host/approvals/tests.rs` `signer` :109 and `fixture` :150, both private to that module.
- Configuration is strict (`deny_unknown_fields`, `host/config.rs:53`, :341, :476, :486);
  `Adapter::selection()` :405-414.
- Answer: success on stdout under `.result` (`cli/operations.rs:309`, :336-338), exit 0; failure
  on stderr under `.error.data`, `.error.code` `failure` (:313-338), exit 1; usage exit 2
  (`cli/session.rs:505-506`). `MutationObservation` is closed (`host/owner/mutation.rs:156-163`);
  `ApprovalCode` :19-24 has no `rate_limited`; `ess/domains/cli.yaml:715` lists `not_found`,
  `invalid_input`, `invalid_configuration`, `unavailable`, `approval_required`,
  `idempotency_conflict` and no `rate_limited`.
- Slack `chat.postMessage`: body keys channel, text, thread_ts, mrkdwn, unfurl_links
  (`adapters/catalog/providers/slack/operations.json:56-65`).
- The one end-to-end test of a local approved write is
  `adapters/catalog/tests/local_runtime/slack_approval.rs:246` (`#[ignore]`, production CLI,
  disposable Secret Service, loopback Slack listener and clock). Its helpers are private to the
  file: `Slack` :23 (`posts()` :181, the list `requests` :29), `clock()` :200 (its closure sets
  the time of each answer), `carries_token` :223, `refused` :233. The suite links `rusqlite`
  (`adapters/catalog/Cargo.toml:60`).
- `crates/connectors-build/src/ignored.rs:542` classifies that test `Deferred`
  (story:main-failing-ci-journeys, draft), so the CI job `ignored CLI journeys`
  (`.github/workflows/rust-gate.yml:110-186`) does not run it. The one recorded failure: CI run
  43055896285 of 2026-11-11, `CLI failed: {"error":{"code":"cli_parse",...}}` at
  `slack_approval.rs:434`, after 2.78 s. Whether it fails at 5983a6b9502 is not known. The
  runner `connectors-build ignored` has no name filter (acceptance critic, round 1).
- MCP admits only reads (`apps/connectors/src/mcp/projection.rs:272-287`, :755).

## Design

- Boundary. The grant file, the signing key and the state directory belong to the operator's
  UID. A process of that UID can run `grant-standing` itself; so can it run `approvals issue`
  today (`contracts/cli/v1alpha1/semantics.md:117`, the same-user boundary). The two differ in
  size: `approvals issue` yields one write of one input; `grant-standing` yields up to 10,000
  writes an hour until a `not_after` that has no upper bound. A grant bounds what a caller that
  only invokes can write; it is no boundary against the operator's own UID. The contract text
  says all of this. The container part (the other story) is where the caller cannot sign.
- One file per instance. A grant file is signed by one instance's approval key and holds that
  instance's grants only: issuer and audience are per instance (`host/approval_keys.rs:543`,
  :593-594, critic), so a second instance pointed at the same path fails verification. The
  state directory (`paths.state`) is one per host (`host/config.rs:1209-1212`, critic); the
  uses store and the lock below are in it and serve every instance.
- Commands, in the CLI process, on the pattern of `approvals policy-set` (`cli/approvals.rs`),
  each with the instance selectors:
  - `approvals grant-standing --input-file <grant>` reads `{operation, target: [{pointer,
    value}], input_keys, not_after_unix_ms, max_per_hour}`, sets `id` (a new UUID) and
    `instance`, adds the grant to the instance's grant file, signs, replaces the file and
    prints `{grant_id, path}`. It refuses with `invalid_input`: a member the list above does not
    name, a missing or zero `not_after_unix_ms` or `max_per_hour`, a `not_after_unix_ms` at or
    before the upper bound of the clock reading, and whatever `parse` refuses. Before it adds,
    it drops every grant whose `not_after_unix_ms` is at or before the lower bound of the clock
    reading; with 256 grants left (`GRANTS_LIMIT`) it refuses with `invalid_input`. It also
    refuses with `invalid_input`, and writes nothing, when the signed file would be larger
    than `FILE_LIMIT` (64 KiB, `host/standing.rs:40`): a reader refuses such a file as
    `Oversized` (:336), which would end every grant in it. A dropped grant changes no replay:
    a replay reads the uses store by attempt id.
  - `approvals revoke-standing <grant id>` removes the grant from the file and signs again. An
    id the file does not hold: `not_found`.
  - `approvals list-standing` prints `{grants: [{grant_id, operation, target, input_keys,
    not_after_unix_ms, max_per_hour}]}` of the verified file, expired grants included.
  - No `standing_grants` member on the instance: all three refuse with `invalid_configuration`.
  - An existing file that does not verify (another key, a rotated or revoked key, a changed
    byte, another `typ`): all three refuse with `invalid_configuration` and write nothing. The
    operator removes the file and grants again; the documents say so.
  - An absent file: `grant-standing` creates it, `list-standing` prints no grants,
    `revoke-standing` is `not_found`.
  - The two that write hold one exclusive `flock` from read to rename on the file
    `standing-grants.lock` in the state directory and replace the grant file with
    `publish_replace` (`host/filesystem.rs:516`), mode 0600. The lock is `Lease::acquire`
    (`host/approval_keys.rs:668`, private today), which gains the file name as an argument
    and becomes `pub(crate)`. Lock order: a command takes `standing-grants.lock` first and
    the key lease (`acquire_key` :486, shared) second; the executor holds the shared key
    lease and never takes `standing-grants.lock`, so the two cannot wait on each other.
- Grant file and signature: compact JWS, Ed25519, `kid` of the instance's approval key, a `typ`
  of its own (working name `b10x.connectors-standing-grants.v1+jws`), header and claims
  `deny_unknown_fields`; payload `{iss, aud, document}`, `document` in
  `connectors.standing-grants/1`. One decoder checks `typ`, the canonical form, `iss`, `aud` and
  the key window with the checks of `proof.rs:104-122`; a rotated or revoked key ends every
  grant it signed. The local path reads the file only through this decoder: a bare unsigned
  grant document at the path admits nothing. A grant file is not a proof and a proof is not a
  grant file. A grant binds instance, operation and target, as the record says, and neither the
  connection nor the configuration revision: a connection authorised again keeps its grants.
- Configuration member: optional `standing_grants = "<absolute path>"` on `[adapters.<name>]`
  and `[connectors.<c>.instances.<i>]`; a relative path is refused as the other paths are. It is
  outside `selection()`: every call reads and verifies the file again, so a changed path needs
  no new policy or key. The file and its directory pass `admitted` (`host/standing.rs:265`); a
  symlink is `unavailable`. No format bump; an older binary refuses a configuration that
  carries the member (`contracts/cli/v1alpha1/semantics.md:416-417`). The rewriters
  (`cli/migrate.rs`, `cli/select.rs`) keep it.
- Which path. One function, `standing::route(proof, key, member)`, decides in the owner from
  what the owner received: a proof, so the per-input path, in every byte as today, and its
  refusal is final (no fall back to a grant); no proof and no key, or no `standing_grants`
  member: `approval_required` as today; no proof, a key and the member: the grant path.
  - The CLI gate (`cli/invoke.rs:1155`) lets a write without `--approval-file` go on only with
    `--idempotency-key` on an instance with the member.
  - On an instance with the member, an `--approval-file` whose content is empty is refused by
    the CLI in `cli/operations.rs`, right after `file_until` returned (:237) and so after the
    keyed lookup (:223, a settled key still replays): the reply of the gate, byte for byte
    (`approval_required`, stage `admission`, `next_action` `none`, `approval_steps`), and no
    owner is started. Changed for such an instance: today the owner answers it, with the same
    code. Reason: the owner reads an empty frame as no proof (`transport.rs:2640`, unchanged),
    so without this an empty approval file would take the grant path. On an instance without
    the member nothing changes: the owner answers as today (`execution.rs:167`).
  - Stated, not changed: with a pending key the CLI sends no proof (:233-234) even when
    `--approval-file` was given. If the pending attempt is gone when the owner looks, the owner
    routes by what it received and may take the grant path. The write is then one the grant
    covers; a caller can always leave the flag out.
- The input the checks read. `covering` and the closed-input walk get the `Value` that
  `connectors_core::read_json` builds from `request.input`, as `resolve` builds it
  (`approval_issuance.rs:222-223`), never a second parse with another parser: `read_json`
  refuses duplicate member names, and the bytes of `request.input` are the bytes
  `execution.rs:282` sends.
- Admission order on the grant path.
  1. The CLI's keyed lookup (`cli/operations.rs:223`) and the owner's `resolve` :156 and replay
     :158 stay first: a settled key replays, and a key with other input conflicts, before any
     grant check, also after expiry or revocation and at the cap (ADR decision 3).
  2. In the executor's first closure the clock (:169), `resolve` again and `unchanged`
     (:170-171) stay where they are. In place of the proof check (:167, :172) the closure makes
     one call to `standing::admit` with the grant file's path, the instance's
     `ConfiguredApprovalKey`, the instance, the operation, the input `Value`, a reading of
     `clock.now()` and the uses store. `admit` reads and verifies the file, runs `covering`
     and takes a use on the first covering grant with room. All before the audit anchor :274
     and the preflight :277.
  3. A refusal in 2 takes the closure's error arm: winner recheck, `not_attempted`, no audit
     anchor, no attempt, no provider request.
  4. Then audit anchor and preflight as today; `prepare` (`host/mutations.rs:123`) in place of
     `prepare_approved`; `uses.bind` to the attempt; no spend; `open_dispatch` (:267) as the
     gate; commit and settle unchanged.
- Rechecks, by one function `standing::still_admitted`. Where the per-input path verifies the
  proof again before `prepare` (:375) and before the gate (:440), the grant path reads
  `clock.now()` fresh, reads and verifies the file again with the held `keys` lease, and checks
  that the grant taken is still in it, still covers the input and is not expired. It takes no
  second use. A `clock.now()` that fails (the reading is refused once 30 s have passed since
  its sample, `host/clock.rs:816-818`, critic) is answered `unavailable`, as the per-input
  path answers it (`execution.rs:64`), never `approval_required`. After the gate (:500-511)
  the grant path keeps the deadline check and the winner's `consume` and reads no file: the
  recheck before the gate is the last one. From it through `dispatch_read` (:443-447) and the
  `open_dispatch` transaction neither a revocation nor a `not_after` that passes stops that
  write; the per-input path checks `exp` once more at :502. The contract text says so, and
  says that a refusal at the recheck before the gate leaves an aborted attempt that names
  the grant, a case decision 1 of the record ("no attempt") does not describe. The spend
  site (:424-432) has no counterpart.
- The use. `admit` returns a guard, a new local type that wraps the `Use` of
  `Taken::Slot(Use)`; `Uses::take`, `Taken`, `Use` (`host/standing.rs:374-387`) and the served
  host's `release_grant` stay as they are. Dropped before `bind` the guard releases the use,
  on every exit: a refusal, a failed preflight (:311, :345, :389, critic), a failed `prepare`,
  a failed `bind`. After `bind` the use stays counted, whatever the outcome, also when the
  recheck before the gate refuses: that answer is `not_attempted`, has an attempt and names
  the grant. A replay takes no use. An in-grant caller can hold a use for the length of a
  preflight; that is inherent to one cap shared by all callers of a grant.
- Clock. `not_after` and the hour window compare against the upper bound of a fresh
  `clock.now()`, as a proof is refused when the upper bound reaches `exp` (`proof.rs:113`,
  critic). The served host keeps its system clock
  (`ess/domains/standing_approvals.yaml:176-185`); the ESS text gains the local case.
- Ledger record (D10 = A). The attempt records `approval_mode='not_required'`, as the served
  host's attempts under a grant do, and the grant id is bound to the attempt in
  `<state>/standing-grants.sqlite3` (`Uses::open(&paths.state)`; ADR decisions 2 and 6). The
  contract text states three things: the ledger alone cannot tell a write under a grant from a
  write that needs no approval; a local ledger now holds `not_required` for a write whose
  policy requires approval; the ledger's gate does not enforce the grant, the executor's order
  does. Bound rows are never pruned; the ledger holds at most 10,000 attempts per instance
  (whether it refuses or prunes there was not read). Not taken (D10, option B): a third mode
  `standing`.
- Answer member: `MutationObservation` gains optional `standing: {grant_id}`, added by one
  function `standing::named(observation, attempt id, state directory)` that reads `grant_of`:
  `.result.mutation.standing.grant_id` on a fresh write and on its replay,
  `.error.data.mutation.standing.grant_id` on a failure that has an attempt. Absent on
  per-input writes and on a refusal before `prepare`. The CLI's replay reads the store through
  a new constructor `Uses::existing(directory)`, which opens read-only, returns `None` when
  the store file is absent and never creates it; with no store or no row the member is absent.
- Refusals (stderr, exit 1, stdout empty, `.error.data.mutation.classification`
  `not_attempted`, no `standing` member):
  - Outside the grant (instance, operation, target, closed input, expired, revoked, a file
    that does not verify, an absent file): `.error.data.code` `approval_required`; no member
    named `axis`; the reason goes to the owner's log.
  - At the cap: `.error.data.code` `rate_limited` with `.error.data.retry_after_seconds` (1 to
    3600); a new variant in `ApprovalCode` (`host/owner/mutation.rs:19-24`), the member on
    `Failure` and in `project` (`cli/operations.rs:282`).
  - Same key, other input: `idempotency_conflict`, unchanged.
  - A grant file that is group- or world-writable, foreign-owned, a symlink or not a regular
    file: `unavailable` (`host/standing.rs:809`).
- A key, `[approval_clock]` and a policy listing the operation stay prerequisites (O1). MCP
  stays read-only.
- The test seam. In `host/standing.rs`, each a function of plain arguments (the grant file's
  path, the state directory, `&Signer` or `&ConfiguredApprovalKey`, the instance, a clock
  reading) with no keyring, owner or child: `sign`, `verify`, `grant`, `revoke`, `list`,
  `route`, `admit`, `still_admitted`, `named`. `execute` and the three commands call them and
  add nothing of their own to a decision. `resolve` and `unchanged` are not behind the seam;
  they stay in `execute`. The test key: `mod tests` in `host/approvals.rs:11` and, in
  `host/approvals/tests.rs`, `signer` :109 and the constant `PUBLIC` :17 become `pub(crate)`
  (the module is compiled under `cfg(test)` only); `fixture` is not used (`bind` takes any
  UUID).

## Design first, in this story

1. ESS, through `ess:specifying`, in `ess/domains/`: `standing_approvals.yaml` (the signed
   file, the local actor beside `ServiceHost` :151-156, the local order and clock);
   `mutations.yaml:497-515`; `cli.yaml` (:715 `rate_limited`, three command inputs beside :46,
   the answer member); `approval_issuers.yaml:1-3`; `local_approval_policy.yaml:2-4`.
2. Contract text: `contracts/operations/v1alpha1/semantics.md` (:103, :106, beside §4.2
   :148-156); `contracts/cli/v1alpha1/` `semantics.md` (:815-820, configuration keys, the
   three commands, the same-user boundary and the size of a grant against one approval, the
   three ledger statements, the restore limit of O5), `owner.md:147-149`,
   `private-mutations.md:33-35`; `contracts/service/` `local-mutations.md` (:126-131, :193),
   `compatibility.md` §2.1.
3. Documents that say a local write needs a proof per write (scope critic, round 1):
   `README.md:51`, `docs/catalog-slack.md:310` and :784, `docs/local-approvals.md`,
   `docs/local-approval-keys.md`, `docs/running-services.md:251-275`,
   `website/docs/guides/guarded-write.md:11`; `website/docs/connectors/slack.md` is generated
   from `docs/catalog-slack.md`.
4. Four authored traces in `contracts/operations/v1alpha1/scenarios/`:
   `standing-approval-scope-match.yaml`, `standing-approval-expiry.yaml`,
   `standing-approval-revocation.yaml`, `standing-approval-rate.yaml` (no scenario directory
   holds a standing trace today).
5. Steps 1 and 2 settle the `typ`, the payload, the command arguments and the refusal codes of
   the three commands. Implementation commits follow their review in the same pull request.

## Acceptance

Every test runs 60 s at most and each test command below ends within 120 s. No test calls a
model or an address other than loopback. "Control" means: the same test first shows that an
in-grant write is admitted under the same file, key and clock, so the refusal is the grant's.

In process, `crates/connectors-host`, on a temporary state directory with the test key
(`signer`). Tests 1-13 are in the module `local::standing::tests` (`host/standing.rs:595`),
test 14 in a new test module `standing_tests` of `host/config.rs` (it has no `mod tests`),
test 15 in `local::owner::mutation::tests`. The output of `cargo nextest list -p
connectors-host -E 'test(standing)'` holds the 15 names below (their paths or names hold
`standing`), and `cargo nextest run -p connectors-host -E 'test(standing)'` exits 0:

1. `a_signed_grant_file_admits_the_granted_write_and_counts_one_use` (`admit`).
2. `a_grant_file_that_does_not_verify_admits_nothing`: after the control, each of: one changed
   byte; signed by another key; a per-input proof document at the path (other `typ`); a bare
   unsigned grant document; another `iss`; another `aud`; signed before the key was rotated;
   signed by a revoked key. Each: `approval_required`, no use counted.
3. `a_grant_file_is_not_a_proof`: the grant file's bytes given to the proof decoder are refused.
4. `an_unsafe_grant_file_is_unavailable`: after the control (the same file at mode 0600 is
   admitted), mode 0620, mode 0602, a symlink to it, a directory at the path. A foreign owner
   is covered by `admitted`'s own test, not here.
5. `a_write_outside_the_grant_is_refused_and_counts_no_use`: after the control, another
   instance, another operation, another channel value, the channel missing, `/body/mrkdwn`
   added, no file; each names its `Uncovered` variant.
6. `expiry_reads_the_upper_bound_of_the_clock`: a reading that straddles `not_after` is
   refused; one that ends before it is admitted.
7. `a_fresh_key_at_the_cap_is_rate_limited`: `max_per_hour` 1; the second `admit` answers
   `retry_after_seconds` in 1..=3600; with the clock 3600 s later it is still at the cap, and
   3600 s and 1 ms later it is admitted (`Uses::take` counts a use while `admitted_at_ms >=
   now - WINDOW_MS`, `host/standing.rs:498`).
8. `a_use_not_bound_is_released_and_a_bound_use_stays_counted`: `max_per_hour` 1; `admit` and
   drop the guard, the next `admit` is admitted; `admit`, bind and drop, the next is at the cap.
9. `a_recheck_refuses_a_grant_that_left_the_file_and_takes_no_use` (`still_admitted`): after
   the control (the recheck passes on the unchanged file and the use count stays 1), each of:
   the file replaced without the grant, the file removed, the clock past `not_after`, the key
   rotated, the input given one more member.
10. `a_present_proof_never_falls_back_to_a_grant` (`route`): after the control (no proof, a key
    and the member give the grant path), a proof with a key and the member gives the per-input
    path; no key gives `approval_required`; no member gives `approval_required`.
11. `grant_revoke_and_list_keep_the_file`: `grant` returns a UUID and the file has mode 0600; a
    second grant keeps the first; `list` shows both; `revoke` removes one; `revoke` of an id the
    file does not hold is `not_found`; on a file signed by a rotated key `list`, `grant` and
    `revoke` are each `invalid_configuration` and the file's bytes are unchanged; with 256
    grants of which one is expired `grant` succeeds and the expired one is gone; with 256
    unexpired grants `grant` is `invalid_input`; a grant that would make the signed file
    larger than 64 KiB is `invalid_input` and the file's bytes are unchanged.
12. `eight_concurrent_grants_keep_all_eight` (eight threads, one file).
13. `a_grant_input_is_validated`: `invalid_input` for a missing `not_after_unix_ms`, a zero
    one, one before the clock, a missing `max_per_hour`, a zero one, 10,001, an `id` member, an
    `instance` member.
14. `standing_grants_round_trips_and_leaves_the_selection` (`host/config.rs`): present and
    absent parse; `selection()` is equal with and without it; a relative path is refused.
15. `standing_is_named_for_an_attempt_and_absent_without_a_store` (`named` and the
    serialisation of the observation and of `Failure`): `standing.grant_id` for a bound
    attempt, on the success shape and on the failure shape; absent for an attempt with no row;
    with no store file the member is absent and the directory holds no
    `standing-grants.sqlite3` afterwards; `rate_limited` carries `retry_after_seconds`.

CLI without a keyring, new file `apps/connectors/tests/standing_grant.rs` on the pattern of
`approval_field_adversary.rs` (:60, :229); run by `cargo nextest run -p connectors --test
standing_grant --test invoke_short`:

16. `standing_commands_refuse_without_the_configuration_member`: the three commands exit 1 with
    `.error.data.code` `invalid_configuration`.
17. `a_write_without_a_key_or_without_the_member_changes_no_byte`: no `--approval-file` in
    either case. With `standing_grants` set and no `--idempotency-key`, and with
    `--idempotency-key k` on an instance without the member: exit 1 and `.error.data` equals
    the value `invoke_short.rs:337-338` pins. The existing test
    `a_write_without_an_approval_file_is_approval_required_naming_the_steps` passes unchanged.
18. `an_empty_approval_file_is_answered_as_no_approval_file`: with the member, `--idempotency-key
    k` and an `--approval-file` of 0 bytes: exit 1, `.error.data` equals the value
    `invoke_short.rs:337-338` pins, and the state directory holds neither `owner.lock` nor
    `owner.sock` afterwards. The line fails before the change: today the owner is started and
    answers. The same call on an instance without the member does not get that value (it
    takes today's path to the owner).
19. The rewriters keep the member: `migrate_keeps_standing_grants` (unit, `cli/migrate.rs`) and
    `select_keeps_standing_grants` (unit, `cli/select.rs`); `cargo nextest run -p connectors -E
    'test(keeps_standing_grants)'` runs two tests and exits 0.

Authored traces:

20. The four files of design step 4 exist. `cargo test --locked -p connectors-build --bin
    connectors-build the_registered_authored_scenarios_execute_against_the_model --
    --nocapture` (`crates/connectors-build/src/gate.rs:716-724`) passes and prints `gate:
    executed N authored scenarios, P passed` (`gate.rs:632`) with N and P each at least 4
    higher than the same command prints at the merge base; `AUTHORED_KNOWN_FAILING`
    (`gate.rs:646`) stays empty. The test runs `ess verify conform synthesize`: the report
    gives its wall time at the merge base and after, and a time over 60 s after is fixed in
    this pull request.

One journey, new file `adapters/catalog/tests/local_runtime/slack_standing.rs`, one test
`a_slack_bot_post_is_admitted_under_a_standing_grant_and_refused_outside_it`. The file holds
its own copies of `Slack`, `clock`, `carries_token` and `refused` (they are private to
`slack_approval.rs`), with three additions: `Slack::requests()` (the count of all requests,
`auth.test` included), two switches on `Slack` (answer the next `auth.test` with an error;
answer the next `chat.postMessage` with HTTP 500), and an offset in seconds that the test adds
to the clock's answers. `attempts()` counts the rows of the table `mutation_attempts`
(`host/metadata/mutations.sql:6`) in `<state>/metadata.sqlite3` with `rusqlite`. "No request": `requests()` does not move. "No
attempt": `attempts()` does not move.

21. First step, before any code of the journey: the sibling
    `slack_approval::a_slack_bot_post_is_issued_the_approval_its_prepare_named_and_invoked_with_it`
    runs alone on the workstation at the merge base, from the built suite executable with the
    arguments and environment `connectors-build ignored` gives it
    (`<suite> --ignored --exact <name> --nocapture --test-threads=1`, `CONNECTORS_TEST_CLI`,
    `CONNECTORS_TEST_CATALOG_PROVIDER`, `TMPDIR`). The report gives the command, the wall time
    and the result. This story changes neither that test nor its `Deferred` entry
    (`ignored.rs:542`); both belong to story:main-failing-ci-journeys, and the result is
    written into the report for it.
22. Steps of the new journey, in order, with `max_per_hour` 3, `input_keys` `/body/text` and
    `/body/thread_ts`, and `not_after_unix_ms` 3600 s after the fixture clock's midpoint:
    a. a keyed send before any grant file exists: exit 1, `approval_required`, `not_attempted`,
       no request, no attempt;
    b. `grant-standing` prints `{grant_id, path}`; `list-standing` prints one grant and
       `.grants[0].grant_id` equals the id;
    c. a keyed send to the granted channel: exit 0, `.result.mutation.classification`
       `applied`, `.result.mutation.standing.grant_id` equals the id, one `chat.postMessage`;
    d. the same key and input: the same answer, no second post;
    e. the same key with another text: exit 1, `idempotency_conflict`, `not_attempted`, no
       request;
    f. another channel, then `/body/mrkdwn` added, each with a fresh key: exit 1, stdout
       empty, `.error.data.code` `approval_required`, `.error.data.mutation.classification`
       `not_attempted`, no `.error.data.mutation.standing`, no member named `axis` anywhere in
       `.error.data`, no request, no attempt;
    g. an in-grant input with a fresh key and an `--approval-file` holding a proof with one
       changed byte: exit 1, `.error.data.code` `approval_refused` (`execution.rs:62`), no `standing`, no
       `chat.postMessage`, and the use count is unchanged (step i shows it);
    h. an in-grant send with a fresh key while `auth.test` is switched to an error: exit 1,
       `not_attempted`, no `chat.postMessage`, no `standing` (the use is released);
    i. an in-grant send with a fresh key while `chat.postMessage` is switched to HTTP 500:
       exit 1, a classification other than `not_attempted`, and
       `.error.data.mutation.standing.grant_id` equals the id (the use stays counted: 2);
    j. one more in-grant send with a fresh key: admitted (3). Then a further fresh key: exit 1,
       `.error.data.code` `rate_limited`, `.error.data.retry_after_seconds` in 1..=3600,
       `not_attempted`, no request. Had g or h kept a use, the first send of this step would
       be `rate_limited`. The key of step c still replays with the grant id;
    k. `revoke-standing <grant id>`; `list-standing` prints no grant; a fresh key:
       `approval_required`, no request; the key of step c still replays with the grant id;
    l. `grant-standing` again with `not_after_unix_ms` 600 s after the clock: a new id; one
       send admitted that names the new id. The clock's offset is set to 3600 s: a fresh key
       is `approval_required` with no request; the key of this step's send replays with the
       new id, and the key of step c replays with the first id.
23. Classification of the new test in `crates/connectors-build/src/ignored.rs`: the one of
    `custody_journey::a_write_after_an_executable_only_change_is_validated_once_and_sent` (the
    arm at :249), so the CI job `ignored CLI journeys` runs it. `cargo test -p
    connectors-build --tests` passes. In the pull request that job passes, and in its artifact
    `ignored-suites.json` the entry of `.result.outcomes[]` whose `name` ends in the new
    test's name has `selected` true and `status` `passed`.
    - A failure of the new test in that job whose cause is the new test's or this change's
      is fixed in this pull request, and the fix includes what makes the workstation's run
      show it.
    - A failure whose CI log shows the cause the sibling fails with in that job (the entry of
      line 21's test in the same artifact): the new test is classified `Deferred`,
      story:main-failing-ci-journeys names it as its twelfth, and the report names the CI log
      and says that no CI job proves the list below.
24. The report gives the new journey's command, wall time (under 60 s) and result on the
    workstation.

What lines 1-20 do not prove, and only the journey does (in CI unless line 23's last case
holds, on the workstation otherwise): that `execute` calls `admit` before the audit anchor
and the preflight (no request and no attempt on a refusal); the refusal codes as the owner and
the CLI print them; replay before the grant check; release of the use on a failed preflight;
the rechecks' place in `execute`; `open_dispatch` with `not_required`; the grant id bound to
the attempt and named on a fresh write, a replay and a failure; the CLI gate admitting a keyed
write; the CLI wiring of the three commands (arguments, `{grant_id, path}`, signing with the
key from the keyring). Proven by no test: release of the use on a failed `prepare` or a failed
`bind` (it rests on the guard's drop, line 8); the recheck before the gate refusing after
`bind`; two callers racing for the last use across two owners.

Unchanged, generated and the rest:

25. Per-input approvals: `git diff --quiet $(git merge-base origin/main HEAD) --
    crates/connectors-host/src/local/approval_policy/tests.rs
    adapters/catalog/tests/local_runtime/slack_approval.rs
    apps/connectors/tests/approval_field_adversary.rs
    apps/connectors/tests/approval_field_adversary_2.rs
    apps/connectors/tests/invoke_short.rs` exits 0. `git diff -U0 $(git merge-base origin/main
    HEAD) -- crates/connectors-host/src/local/approvals.rs
    crates/connectors-host/src/local/approvals/tests.rs` shows three changed lines, each one
    adding `pub(crate)` (`mod tests`, `signer`, `PUBLIC`). `cargo nextest run -p connectors-host
    -E 'test(approvals::) | test(approval_policy::)'` and `cargo nextest run -p connectors
    --test approval_field_adversary --test approval_field_adversary_2` exit 0.
26. `cargo nextest run -p connectors-host -E 'test(owner::mutation::)'` exits 0.
27. `rg -l 'grant-standing' README.md docs/catalog-slack.md docs/local-approvals.md
    docs/local-approval-keys.md docs/running-services.md website/docs/guides/guarded-write.md
    contracts/cli/v1alpha1/semantics.md` prints seven paths; `rg -n 'same-user'
    contracts/cli/v1alpha1/semantics.md` prints a line in the section of the three commands;
    `rg -n 'not_required' contracts/cli/v1alpha1/semantics.md` prints a line in that section.
28. The report lists each new test's time; none is over 5 s without being named with its time.
29. `git diff --name-only $(git merge-base origin/main HEAD) | rg '\.py$'` prints nothing.
30. Before merge, an `aep:adversary` `review-result` and an `aep:security-reviewer`
    `review-result` each `reviews:` this story; every finding has an outcome.
31. Generated files, once, after the rebase named below (`AGENTS.md:149-152`), for this story
    and for story:standing-grant-closes-the-input-to-listed-members together: `slot cargo run
    --locked -p connectors-build --` with each of `metadata-entities --check`, `ess-boundary`,
    `cli --check`, `mcp-bindings --check`, `service-metadata --check`, and `slot cargo run
    --locked -p connectors-docs -- generate --check`, exit 0; `cargo test -p connectors-build
    --tests` passes; the repository gate passes on the pull request.
32. `CHANGELOG.md` has an `### Added` entry under `## Unreleased`: the three commands,
    `standing_grants`, the answer member, `rate_limited`, the empty approval file, and that an
    older binary refuses a configuration with the member.

## Order and shared files

- One implementor, one checkout, one pull request, in this order: line 21; then
  story:standing-grant-closes-the-input-to-listed-members; then the signed file, the three
  commands and the configuration member (acceptance 1-4, 11-14, 16, 19); then the owner path,
  the answer, the traces and the journey (the rest). `host/standing.rs`,
  `host/approvals/proof.rs`, `host/config.rs` and `apps/connectors/spec/cli.yaml` are shared by
  the parts, and the journey reads the file the commands write.
- Generated files: `crates/connectors-build/generated/metadata-emissions/inputs.json` digests
  `ess/domains` and the Entity Runtime pin. Regenerate once, after the rebase on a `main` that
  holds release 0.76.1 and story:windows-mcp-tool-call-answers-timeout, never before. Until
  then the generated checks of both stories are red inside the pull request.
- In work now: `CHANGELOG.md` is shared with story:windows-mcp-tool-call-answers-timeout,
  story:settled-read-uses-leave-the-store, story:connections-list-answers-through-the-owner
  and releases 0.76.0 and 0.76.1 (these also `inputs.json`); `contracts/cli/v1alpha1/owner.md`,
  `semantics.md` and the generated `website/docs/reference/contracts/cli.md` are shared with
  the second and third (other lines; rebase and regenerate). `host/owner.rs` gains 358 lines
  in the third; inferred: the owner reaches the grants path there.
  `host/owner/transport.rs` is read here (:2640, :4624) and not changed; the Windows story
  changes its `watch_cancel` (about :2479-2536).
- `crates/connectors-build/src/ignored.rs` is also named by story:main-failing-ci-journeys
  (draft, not in work).
- Drafts, not in work: story:guarded-write-credential-refusal-says-repair (`execution.rs`,
  `host/owner/mutation.rs`); story:timeout-refusal-says-how-long-it-waited
  (`ess/domains/cli.yaml`); story:definition-files-in-user-config and
  story:openrouter-inference-key-profile (`host/config.rs`, the second also `cli/migrate.rs`).
  story:invoke-admission-queues-under-load shipped in 0.75.0, the base of this tree.

## Out of scope

- The container mount under `mounted_secret` custody and its jail tests (the other story).
- The served host. Writes through MCP. A byte bound on `body.text`. Grants in the `approvals
  policy-set` document. A bound on how far ahead `not_after` may lie. A third ledger mode
  `standing`. The comment at `ess/domains/transport.yaml:18`, which belongs to
  story:windows-mcp-tool-call-answers-timeout.

## Decided in drafting, 2026-11-23

Taken by the session that drafted the story, D10 excepted. Each keeps what the tree or the
accepted record does today.

- O1. Code requires a published policy listing the operation for any keyed write or replay
  (`approval_issuance.rs:257-264`, `mutation.rs:681-686`). Kept: `approvals policy-set` lists
  the operation once, and the grant is not in that document.
- O2. No `standing_grants` member: today's bytes (acceptance 17). Member set and the file
  absent: the owner's shape, `not_attempted` (acceptance 22a).
- O3. `rate_limited` and `retry_after_seconds` are added to the local write failure, as the
  served host answers them.
- O4. `/body/thread_ts` is a caller's key and is admitted with any value; the channel stays
  the granted one. How Slack answers a `thread_ts` of another channel is not established.
- O5 (rounds 1 and 2). The file is the one revocation authority, as decision 5 says:
  `revoke-standing` removes the grant and nothing is written to the uses store. Limit, stated
  in the contract text: the file carries no sequence, so an older signed file put back at the
  path admits again until its `not_after` or until the approval key is rotated. No other UID
  can put it back (`admitted`, `host/standing.rs:265`). The operator's UID can, and so can a
  backup, a file sync or a snapshot restore running as that UID, with no caller involved.
  Rotating the approval key ends every grant for certain.
- O6 (round 1). `grant-standing` has no second confirmation step like `--approve-subject`:
  `policy-set`, the nearest command, has none, and a second step is no boundary against the
  operator's own UID. `list-standing` is added so that an operator can see what is granted.
- O7 (round 2). The three commands refuse a grant file that does not verify and leave it in
  place; removing it is the operator's act, so the operator sees that every grant in it ended
  before a new file is written.
- D10 = A (the operator, 2026-11-23): the attempt records `not_required` and the grant id
  lives in the uses store, as on the served host.
