---
format: aep.planning-md/3
id: story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold
kind: story
status: draft
title: 'The MySQL test fixture sets TCP_NODELAY: 7.0 s become an estimated 0.6 s if the cause holds'
summary: One SQL test spends 91 % of its time in epoll_wait on its own fixture server, which writes each packet in two parts without TCP_NODELAY; the first acceptance statement confirms or refutes that cause
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- supersedes: story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
scope:
- confidence: cited
  path: adapters/sql/tests/mysql/fixture.rs
revision: 2
---
## Defect

`every_page_is_one_the_model_admits` (`adapters/sql/tests/ess_model.rs` line 1302, binary
`connectors-sql::ess_model`) waits for its own fixture server for 91 % of its time.

Durations:

- Forge: 7.0 s, as nextest printed it in the forge's gate job 138270416028 of pull request 291
  (2026-11-24), as the story `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`
  lists it.
- Build host, measurement of 2026-11-25, head d5cf22480, the test alone: 9.293 s (solo run) and
  9.245 s (run 6, under `strace`), both as nextest printed them.

What run 6 measured (`strace -f -tt -T -C -w` around the test binary; `perf` is refused on the
build host):

- Wall time of the process: 9.229 s.
- `epoll_wait`: 8.437 s in 616 calls. That is 91.4 % of the wall time (arithmetic).
- The child `ess generate`: 0.568 s (`wait4`), 6.2 % (arithmetic).
- CPU of the process and its children: 0.555 s (user 0.163 s, system 0.392 s).
- 28 `connect` calls, 7 `listen` calls, 2,584 `sendto`, 1,032 `recvfrom`, 35 `setsockopt`.
- The run listed the calls over 50 ms: 23 `epoll_wait` calls of 56.9 to 59.6 ms. They sum to
  about 1.35 s (arithmetic). The other 593 calls hold 7.08 s, 11.9 ms each on average
  (arithmetic). Their distribution was not listed and the trace file no longer exists.

The test (counted in the source): 7 pairs of cap and limit, each with its own fixture server and
a loop of `profile.read` invokes; 28 connections are 4 per pair (arithmetic).

The cause is a reading of the source, not a measurement:

- `Wire::write` in `adapters/sql/tests/mysql/fixture.rs` (lines 239 to 245) sends every packet
  with two `write_all` calls, the 4 bytes of the header and then the body, and a `flush`.
- The fixture never sets `TCP_NODELAY`: the socket comes from `listener.accept()` in `start`
  (line 211) and goes into `session` as it is; the word `nodelay` does not occur in the file
  (`grep`).
- With Nagle's algorithm on, the kernel can hold the second small write until the peer has
  acknowledged the first, and the peer may delay that acknowledgement.
- The client does not hold its side: `mysql_async` 0.37.1 has `tcp_nodelay: true` as its default
  (`src/opts/mod.rs` line 1201) and applies it on connect (`src/conn/mod.rs` line 514); the
  adapter's options (`adapters/sql/src/mysql.rs` lines 376 to 387) do not change it. The 35
  `setsockopt` calls fit one per client connection and one per listener (28 + 7, arithmetic);
  which options they set was not read from the trace.

What the reading does not explain: the 23 listed calls hold 1.35 s of the 8.437 s. The reading
holds only when many of the 593 unlisted calls are such holds too. That is not established.
Acceptance statement 1 is the run that decides it.

## Change

Test code only. No product file changes; `adapters/sql/src/mysql.rs` stays as it is.

1. `start` in `adapters/sql/tests/mysql/fixture.rs` (lines 203 to 220) calls
   `set_nodelay(true)` on the accepted `TcpStream` before it is boxed and given to `session`.
   The TLS path wraps the same socket (`acceptor.accept(wire.io)`, lines 416 to 428), so the
   one call covers it. The bytes on the wire and their order do not change; only the moment the
   kernel sends them does.

Established by reading: nine test binaries of `connectors-sql` include the fixture through
`#[path = "mysql/fixture.rs"]`: `mysql_adversary` (2 test functions), `mysql_local` (6),
`mysql_adversary_2` (3), `query_provenance_adversary` (2), `catalogue_adversary` (9),
`ess_model` (17), `mysql_protocol` (23), `profiles_adversary` (4), `profiles` (17). That is 83
test functions (`grep` for the test attributes, summed). The change reaches all of them.

Not established by reading: whether one of the 83 relies on the hold. Statement 2 shows it.

## Acceptance

1. Confirm or refute the cause, before anything is pushed. On the build host, the test alone,
   three runs at the base commit and three runs with point 1, in turns, durations as nextest
   prints them; and one run of each kind under `strace -f -T -e trace=epoll_wait` with the
   durations counted in steps of 10 ms. The wrapper script only measures, so it starts without
   the `slot` prefix; `cargo` inside it takes its slot through the shim.
   - Confirmed: each of the three runs with point 1 ends under 3.0 s. Outside `epoll_wait` the
     test took 0.79 s in run 6 (9.229 − 8.437, arithmetic), 0.568 s of it in `ess generate`,
     which took 0.10 to 0.69 s in four runs on the build host.
   - Refuted: one run with point 1 takes 3.0 s or more. Then the second probe replaces point 1:
     `Wire::write` sends header and body with one `write_all`; the same three runs decide.
   - Refuted twice: nothing is merged. The story goes back to draft with the two counts of
     `epoll_wait` durations, and open question 10 takes the test over.
2. Behaviour: the 83 test functions of the nine binaries pass, and the pull request changes none
   of them. The run: the test step of the forge's "repository gate" job of the pull request,
   and before the push those nine binaries on the build host, each test under the 60 s bound.
3. Kept runs: the test keeps its 7 pairs of cap and limit.
4. Time: the duration of the test in the test step of the forge's "repository gate" job of the
   pull request is at most 2.0 s. Before: 7.0 s. Estimate: 0.6 s.

Derivation of statement 4:

- Estimate: 7.0 s × (1 − 0.914) = 0.6 s (arithmetic). It assumes the forge has the share of the
  build host, which is not measured, and that the whole wait is the hold, which statement 1
  decides.
- The old rule of the set, estimate times 1.5, gives 0.9 s. That is under "before" but narrower
  than `ess generate` alone varies (0.10 to 0.69 s on the build host).
- Proposed bound: 2.0 s. It is 5.0 s under "before". It cannot be met without the change: the
  wait alone was 8.4 s on the build host, and the two runs there differ by 0.05 s (9.245 and
  9.293 s).
- A job over the bound is not run again until it passes: the duration goes into the pull
  request and the story stays open.

No time window of a test changes. No test is over 60 s before or after.

## Scope

- `adapters/sql/tests/mysql/fixture.rs`: `start` sets `TCP_NODELAY` on the accepted socket (or,
  after a first refutation, `Wire::write` sends one buffer per packet).

No other file is written. No story of the set (those that decompose
`the-gates-test-step-ends-within-120-s-on-the-forge`) names this file. Outside the set
`incident-call-lookup` and `released-install-reads-mysql-and-loki-per-environment` name it;
both are implemented. `adapters/sql/tests/ess_model.rs` is not written.

## Not in this story

An "open question" with a number is that item of
story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves.

- `ess generate` per test: 16 of the 17 tests of `ess_model` run it (0.10 to 0.69 s per call on
  the build host). Open question 9.
- The other 82 test functions that use the fixture: their durations were not measured and the
  story claims nothing for them.
- The MySQL adapter of the product: it already runs with `TCP_NODELAY` and is not changed.
- The fixtures of the other engines under `adapters/sql/tests/`: not read.
- The eleven host tests: the story of the registry built once per process.
