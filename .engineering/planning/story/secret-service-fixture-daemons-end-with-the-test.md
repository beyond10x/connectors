---
format: aep.planning-md/3
id: story:secret-service-fixture-daemons-end-with-the-test
kind: story
status: draft
title: The disposable Secret Service fixture's daemons end with the test process that started them
summary: A test that exits without unwinding leaves the fixture's dbus-daemon and keyring daemon running under pid 1
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

Found while implementing story:custody-keeps-out-of-the-session-keyring on 2026-11-10. The defect
was already there before that work.

The disposable Secret Service fixture starts a `dbus-daemon`
(`crates/connectors-custody/src/secret_service/fixture.rs:59`) and a `gnome-keyring-daemon`
(`:120`). It stops them only in `OwnedChild::drop` (`:15`).

A test that ends without unwinding never runs that drop. For example,
`crates/connectors-host/src/local/mutations/tests.rs:889` calls `process::exit(23)` in a `*_child`
test. The daemons then keep running, re-parented to pid 1.

One `--include-ignored` run of `connectors-host --lib` left 12 such `dbus-daemon` processes. They
had inherited the build lock's descriptor and held that lock for about 9 minutes, until they were
killed by hand.

## Acceptance

- The fixture's daemons end when the test process that started them ends, by any path: drop, panic,
  `process::exit` or a kill. For example, set `PR_SET_PDEATHSIG` on both spawns on Linux.
- A test starts the fixture in a child process that exits without unwinding. It asserts that no
  daemon started under that fixture's directory is still running a bounded time later.
- The fixture's daemons inherit no descriptor beyond their own standard streams.
