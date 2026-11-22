---
format: aep.planning-md/3
id: review-result:wave-b-custody-keyring-security-1
kind: review-result
status: active
title: Security review 1 on custody-keeps-out-of-the-session-keyring (single defect wave)
relations:
- reviews: story:custody-keeps-out-of-the-session-keyring
revision: 1
---
unit: story:custody-keeps-out-of-the-session-keyring
verdict: green
cases: executed 34→37, red 0
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/security/{tests-addition.rs,run-cases.sh,run-suite.sh,cases.out,suite.out,rr-sec-1.json,rr-sec-1.md}; <session scratch>/sr1 (short TMPDIR, socket paths; empty, removed)
needs-coordinator: no

Covers commit d164855d2 on impl/custody-keeps-out-of-the-session-keyring plus my uncommitted test additions.

1. diff --stat: `crates/connectors-custody/src/tests.rs | 321 +` (test file only, uncommitted).

2. Cases added, module `tests::session_keyring_review`. Each was run alone, then rustfmt was applied (whitespace only):
- `every_entry_point_refuses_the_session_bus_before_connecting`: open, open_at, items_at, remove_authority_at and orphans_at with no socket all panic with the refusal. Green. The unit's tests check only `session_bus_refusal`, so this case is what goes red if the guard call is dropped from any call site.
- `moved_alias_writes_behind_it_reads_login_and_removal_stays_inside_the_authority` (live fixture): makes a second collection, moves `default` onto it, writes there, still reads the `login` item, then removes the authority. Both items go. These stay: the id in uppercase, a longer id it prefixes, a Connectors item with no authority, and another schema's items that carry the same `connectors.authority`. Green. The unit's fixture tests all run with the alias on `login`, so this case is what goes red if the code goes back to the hard-coded login path.
- `a_daemon_without_login_receives_the_write_behind_the_alias` (live): `login` is deleted and the write still lands behind the alias. Green. This checks Acceptance 3 word for word.
  Each alone: `test result: ok. 1 passed; 0 failed; ... 36 filtered out`, exit 0 (x3).

3. Suite: `cargo-unit.sh <tree> test --locked -p connectors-custody --lib -- --include-ignored` gave `test result: ok. 37 passed; 0 failed`, exit 0. The same run with `--skip session_keyring_review` gave `34 passed; ... 3 filtered out`. clippy --all-targets -D warnings exit 0; fmt --check exit 0.

4. Findings:
- W1 mod.rs:557 (CONFIRMED, introduced). The guard exists only under cfg(test) or test-support. The CLI that the documented `CONNECTORS_TEST_CLI` workflow builds (docs/local-secret-service.md:138-139, `cargo build -p connectors`) has a no-op guard. What reaches it: any local_runtime or host journey run that way with no `secret_service_socket`. Every current journey was found to set the socket, and `connectors-build ignored` builds with `cargo test --workspace --no-run`, which turns the feature on. Acceptance holds for every test found.
- W2 inventory.rs:109 (CONFIRMED, introduced). If `synchronize` fails after the deletes, the whole Removal is dropped. The CLI then answers custody_unavailable/unlock_keyring and names nothing removed or failed. What reaches it: the keyring daemon exits, or the collection file is replaced, during a remove. This is rare, and I could not inject it through public API.
- N1 gnome.rs:182 (introduced). After a delete in legacy `login`, the barrier fsyncs `<alias>.keyring`, not `login.keyring`.
- N2 docs/local-secret-service.md:160 (introduced). The threat model still says writes go to `login`.
- N3 docs/local-secret-service.md:49 (introduced). The docs say test builds "refuse to open" the session bus, but inspect_at and available_at still open it read-only (`setup check`, local.rs:697). Nothing is written.

5. Checked and not faulted:
- Removal matches the authority exactly, and only within the two Connectors schemas (live case above).
- The store's own authority is refused under any spelling (compared as a Uuid).
- `--mode list` passes remove=false.
- An alias that is unset or points at a non-plain collection name fails closed (Unavailable).
- The production build activates only connectors-custody "default" (`cargo tree -e normal,features` for connectors and catalog/service), so a real user's CLI is never refused.
- No secret in the output: listing rows and contract types carry the item path and attributes only, `failed` carries `Failure` Debug, and nothing reads a value.
- No daemons leaked (ps check, the TMPDIR was empty).

6. Paths written outside the worktree: the files listed in the header under `.../custody-keeps-out-of-the-session-keyring/scratch/security/`, plus `<session scratch>/sr1`. I created sr1 because the assigned scratch path is too long for unix sockets. It was used as TMPDIR, ended empty, and was removed.

7. Findings JSON is in rr-sec-1.json (same five rows).

```findings
[
{"file":"crates/connectors-custody/src/secret_service/mod.rs","line":557,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"The session-bus guard is compiled in only with cfg(test) or feature test-support. The production-built CLI that the documented CONNECTORS_TEST_CLI workflow spawns (docs/local-secret-service.md:138-139, cargo build -p connectors) has a no-op guard, so a journey that leaves out secret_service_socket writes the real keyring without failing. Every current journey was found to set the socket."},
{"file":"crates/connectors-custody/src/secret_service/inventory.rs","line":109,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"remove() discards the Removal record when the closing synchronize fails after deletes have run. The CLI (apps/connectors/src/local/custody_orphans.rs:53,65) then answers custody_unavailable/unlock_keyring and names neither the deleted items nor the failed ones."},
{"file":"crates/connectors-custody/src/secret_service/gnome.rs","line":182,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The durability barrier after deleting a legacy login item fsyncs the alias collection's file (self.file) and never login.keyring, so a delete in login is acknowledged without its own file being synchronized."},
{"file":"docs/local-secret-service.md","line":160,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The threat model still says Connectors writes into the encrypted login collection, but writes now go to the collection behind the default alias."},
{"file":"docs/local-secret-service.md","line":49,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The docs say test builds refuse to open the session bus Secret Service, but inspect_at and available_at (setup check at apps/connectors/src/local.rs:697) still open it read-only with no socket; nothing is written."}
]
```

