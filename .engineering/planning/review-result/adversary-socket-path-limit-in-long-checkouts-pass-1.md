---
format: aep.planning-md/3
id: review-result:adversary-socket-path-limit-in-long-checkouts-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: socket-path-limit-in-long-checkouts'
relations:
- reviews: story:socket-path-limit-in-long-checkouts
revision: 1
---
unit: story:socket-path-limit-in-long-checkouts, working tree wave1001b-socket at 93d553c95 (base b19027874)
verdict: NEEDS-CHANGE
cases: executed 413→415, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (wave-20261001b/socket/scratch/adv-suite.log)
needs-coordinator: none

Cases (apps/connectors/tests/socket_path_adversary.rs), both green: a socket far past SUN_LEN is bound and reached
through the directory (direct bind refused; same inode at the absolute path); 32 threads × 50 bind/connect/drop
cycles never cross sockets through a reused fd. Suite with gate-length TMPDIR (`RUSTC_WRAPPER=`):
415 passed, 0 failed, 27 ignored, EXIT=0; no SUN_LEN in the log.

Not broken: no assertion dropped or weakened in converted tests; Directory kept alive in both relay threads
(local_cli.rs:546-560, :699-724) and the transport idle-sweep thread; helpers drop the descriptor only after
bind/connect returns; no absolute bind/connect left that the gate runs (remaining dbus fixtures are all #[ignore];
gate runs without --ignored, gate.rs:80); the Secret Service docs sentence is accurate.

Finding: started a separate sccache server (port 4917) under the gate-length TMPDIR: `sccache: error: failed to start
server process` / `caused by: path must be shorter than SUN_LEN`, exit 2. `~/.cargo/config.toml` sets
`rustc-wrapper = "/usr/bin/sccache"`; gate.rs:16 passes TMPDIR to cargo. Seen before at
docs/waves/auth-hardening-20260908/integration/full-gate-attempt-1.log:11.

```findings
- file: docs/development.md
  line: 19
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the unit removed the short-checkout advice and says long checkout paths no longer hit SUN_LEN, but the machine-wide sccache wrapper started under the gate's TMPDIR from a managed worktree still fails with "path must be shorter than SUN_LEN" whenever no sccache server is already running
```
