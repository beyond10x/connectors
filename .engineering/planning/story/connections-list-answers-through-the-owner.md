---
format: aep.planning-md/3
id: story:connections-list-answers-through-the-owner
kind: story
status: active
title: connections list, describe and status answer through a running owner
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: apps/connectors/tests/cold_owner_start.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/owner.md
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/admission_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: inferred
  path: docs/local-er-metadata.md
revision: 29
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:57:21Z", actor: "human:timo", revision: 26, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-11-23T08:57:22Z", actor: "human:timo", revision: 27, decided_on: {"recorded":{"review_outcome":8}}}
---
## Outcome

While an owner of the same build runs for the state directory, `connections list`,
`connections describe` and `connections status` answer through that owner, and the CLI process
makes no fresh open and no complete read of the metadata store for them. With no such owner the
three commands answer as today and start none.

This story removes the CLI's own open and read. It does not promise that the consumer's time
falls: the owner's own time for these reads on a grown store is not measured, and acceptance
line 6 reports it without a bar.

## Source

Issue #267, 2026-11-16, connectors 0.73.0, wanted item 3: "**No CLI-side full replay** for
`connections list|describe|status` while an owner of this build runs: answer them from the
owner's held handle, as `invoke` already does." Measured there: `connections list --connector
slack --instance <name>` took 0.26 s on a fresh store and 35 s real, 26 s user on a 133 MB copy,
all in the CLI process. Consumer: dev-operator (session bf-runbook), whose Jira store held
650 MB on 2026-11-23 (no time measured at that size).

The contract already says it: "A command run while an owner of its own build runs reads the
store through that owner … and does not open it" (`contracts/cli/v1alpha1/semantics.md:1378-1380`;
`docs/local-er-metadata.md:269-271`). For these three commands that is false today.

## What the tree holds today

Read by the scoper and the design critic on 2026-11-23 at 5983a6b9502 (connectors 0.75.0):

- The three arms (`apps/connectors/src/local/connections.rs:97-167`) never call
  `owner::Client::connect`. Each builds `Registry::with_system_clock(&paths.state)` (`:15`) and
  calls `Registry::list` (`:107-119`), `Registry::describe` (`:127-144`) or
  `Registry::acquisition_status` (`:145-164`).
- `connections-describe` is in `SELECTING` (`:402-408`). `connections::select` (`:417-450`) runs
  before any handler (`apps/connectors/src/local.rs:233`): without `--connection` it calls
  `Registry::held` in the CLI process (`:434`) and writes the one held connection into the
  input. With none held it refuses `not_found`, stage `admission`, `create_connection`; with
  more than one `invalid_input`, stage `arguments`.
- A fresh process finds no pooled handle, so `er::open` starts the provider and reads a complete
  snapshot (`crates/connectors-host/src/local/metadata/er.rs:2396-2440`, `:2997-3006`). `list` and
  `status --acquisition` hold `metadata.lock` across that open (`metadata.rs:694`, `:1082-1083`).
- The printed documents are built in the CLI crate: list `{"connections":[summary…], "source",
  "stale", "observed_at_ms", "valid_until_ms"}` plus `next_cursor` (`connections.rs:120-125`,
  `summary` `:472-479`), `description` (`:480-495`), the acquisition value (`:154-163`). The host
  crate cannot call them. The host's own `connection_value`
  (`crates/connectors-host/src/local/owner.rs:1285-1303`) is a different document: no
  `pending_reason`, `external_identity` `null` when absent, `binding` added.
- Custody is judged in the CLI process (`connections.rs:106`, `:132`) and passed to the
  registry. An owner starts with a cleared environment (`owner/transport.rs:1964-1966`), so its
  view of the keyring can differ.
- The owner protocol is `connectors-owner/1` to `/3` (`owner.rs:33`). One request per stream
  after `hello`. Request kinds: `owner.rs:1307-1445`, hand-written serde. The `connections`
  request is the precedent (enum `owner.rs:1411-1413`, alias match `owner/transport.rs:2782-2793`,
  `action` arm `:3120-3157`, client `:1061-1093` with a 30 s span, wrapper `owner.rs:683-692`,
  contract `contracts/cli/v1alpha1/owner.md:224-261`). It is sent only after `same_build()`
  (`transport.rs:1062`, `:1135-1141`), so a new kind needs no protocol version.
- `Client::connections` returns one `Err` for a refusal and for a failed exchange
  (`transport.rs:1127-1133`), and `owner.md:257-261` says so: "A `cached` or `connections`
  exchange that fails after the greeting, an owner's refusal but `description_unavailable`
  included, is answered by that direct read, so a refusal carries the direct read's own code."
- The `connections` answer is too small for these commands: `connection, state, current,
  binding?`, revoked connections dropped, custody judged in the owner (`transport.rs:3140-3149`).
- `Client::connect` reads the authority row first (`transport.rs:278`, `metadata.rs:501-505`):
  the lifecycle lock and a read-only SQLite open in the CLI process, no Entity Runtime open.
  It waits at most 10 s for that lock.
- The owner holds a verified handle in its process pool, not a ready list: its `Registry::list`
  re-opens SQLite under `metadata.lock`, takes the pooled handle, catches up and builds the
  projection (`er.rs:2404-2411`, `:3045-3047`). In the owner `list` and `status --acquisition`
  take `Hold::Locked`, `describe` and `held` take `Hold::Relocks` (`metadata.rs:1080-1088`);
  `take_or_wait` does not wait for a handle a relocking user holds (`er.rs:571-607`), so with a
  second observation or an admission in flight the owner makes a fresh complete open itself.
- Pinned and kept: these commands start no host (`semantics.md:73`, `:92-93`, `:99`;
  `contracts/cli/v1alpha1/scenarios.md:65`, L01 "Host starts 0"); "Without a reachable owner of
  the same build a command reads the store directly" (`owner.md:243-244`); beside an owner of
  another build `connections list` still answers (`docs/catalog-slack.md:934-935`).
- The owner's 10-minute idle interval restarts when it accepts a connection
  (`transport.rs:1912-1914`), before the build is compared (`owner.md:42-43`, `:56`).
- `apps/connectors/tests/cold_owner_start.rs` (Linux, jailed) counts fresh opens and complete
  reads through `CONNECTORS_TEST_VERIFICATION_LOG` (`er.rs:2890-2922`, `:2997-3001`; a started
  owner inherits it, `transport.rs:1681-1694`). A log line is `<pid> open` or `<pid> snapshot`;
  `passes()` (`cold_owner_start.rs:104-126`) drains the log and counts all pids together. The
  existing warm-read check asserts no line at all (`:215-220`).

## Design

- One new owner request kind, `observe`, in Rust beside `Request::Connections`, with a
  selector: `list` (`limit`, `cursor`), `describe` (a connection, or none for the one the
  instance holds) and `acquisition` (an acquisition id). Each names the adapter alias and
  carries the custody observation the CLI made, so `state` is the same value on both paths.
  `connections` and what `invoke` reads from it stay as they are.
- The owner answers with what the registry returned, as data: the fields of the page, of the
  described connection or of the acquisition status that the CLI's document code reads. The CLI
  builds the printed document from that answer with the same `summary`, `description` and
  acquisition code it uses on the direct read. `connection_value` is not used and not changed.
- Sent only to an owner of the same build, with the 30 s span of `connections`. No owner, an
  owner of another build, a failed exchange, an expired span or a refusal by the owner: the
  direct read of today, as `owner.md:257-261` says for `connections`. A refusal therefore
  carries the direct read's own code and stage, at the cost of one open in the CLI process on
  that path. The command starts no owner.
- `connections-describe` leaves `SELECTING`. Without `--connection` its arm first asks the owner
  with the selector's connection left out; the owner resolves the held connection
  (`Registry::held`) and describes it. When that gives no answer the arm runs what `select` does
  today and then the direct describe, so the two refusals of `select` keep their code and stage.
  `select` is unchanged for revalidate, revoke, launch and token.
- The CLI-process guarantee covers the Entity Runtime open and the complete read. The authority
  read of `Client::connect` stays; when its 10 s wait for the lifecycle lock ends, the command
  falls back to the direct read.
- The owner may itself make a fresh open when another observation or an admission holds its
  pooled handle. That is the owner's cost and is not changed here.
- No ESS model of the request in this story: story:owner-observe-request-is-modelled-in-ess
  carries it and depends on this one, as c58fdb5f82f followed the `connections` request.
  `owner.md` describes `observe` in prose, as it does most request kinds.

## Acceptance

1. In `apps/connectors/tests/cold_owner_start.rs` (no new test binary),
   `connections_commands_answer_through_a_running_owner_without_verifying_the_store`: an owner
   is started by one cold read and the log is drained with `passes()`. Then `connections list`,
   `connections describe --connection <id>`, `connections describe` without `--connection`,
   `connections status --connection <id>` and `connections status --acquisition <id>` each exit
   0, and after each the drained log holds no `open` and no `snapshot` line at all. The
   acquisition is a failed one: a successful connect names no acquisition, so the test takes
   the id from a connect the fixture refuses (knob `refuse=1`). A pending acquisition needs a
   live owner and cannot exist in the no-owner phase of line 2.
2. Same test: before the owner is started (after `setup owner-stop`), the five commands of
   line 1 are run and their answers kept. Each answer of line 1 equals its kept answer as JSON
   after `observed_at_ms` and `valid_until_ms` are removed at every depth of `result`. Two more
   tests make the same comparison on other fixtures:
   `a_pending_connection_answers_through_a_running_owner_as_directly` (`pending_reason`
   `configuration_changed`) and `a_bound_connection_answers_through_a_running_owner_as_directly`
   (an instance that declares a binding). No fixture holds a connection without an external
   identity: publication always sets it (`crates/connectors-host/src/local/registry/
   lifecycle.rs:223`).
3. `a_refused_connections_command_keeps_its_refusal_beside_a_running_owner`: with an owner
   running, `connections describe --connection <unknown>`, `connections status --acquisition
   <unknown>`, `connections list --cursor <junk>` and `connections describe` without
   `--connection` on an instance with no connection are each refused with the code, stage and
   next action they have with no owner running. An instance with two connections cannot be
   built through the CLI: a connect is refused while the instance holds one, and
   `Registry::begin_unchecked` is `#[cfg(test)] pub(crate)` (`registry/lifecycle.rs:26-36`,
   `registry/cardinality_tests.rs:250-262`). The owner's `invalid_input` for none held and for
   two held is asserted by the host test
   `owner_observe_answers_the_registry_s_observation_and_refuses_none_or_two_held`. No test
   runs the CLI against an instance with two.
4. `connections_commands_with_no_owner_running_start_none`: with no owner running, the three commands answer and no owner process exists
   afterwards (the L01 rule).
5. A host test in `connectors-host`: the wrapper of the new request returns `None` for an owner
   of another build (the fixture of `admission_tests.rs:140-183`) and for a scripted owner that
   answers a refusal or drops the stream (`answer_once`, `admission_tests.rs:191`).
6. A report, no bar, in the pull request and on issue #267: the existing ignored test
   `a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds` is extended to time
   `connections list` with no owner and with a running owner, three runs each, the median of
   each printed, and one more owner run in which the test spawns a read invoke and
   `connections list` together and prints both times. Run on a release build with
   `CONNECTORS_COLD_START_STORE_MB=133`; the report pastes the `uptime` line. A release build
   logs no verification (`cold_owner_start.rs:320-323`), so a fallback has no signal there: the
   same ignored test is run once more on a test build with `CONNECTORS_COLD_START_STORE_MB=10`,
   where it prints the count of `open` lines the owner runs left in the log, and the report
   gives that count.
7. `contracts/cli/v1alpha1/owner.md` describes `observe` beside `connections` and names it in
   the sentence at `:257-261` about an exchange that fails.
8. `contracts/cli/v1alpha1/semantics.md:1378-1380` and `docs/local-er-metadata.md:269-271` name
   the three commands beside `cached`. The edit adds no net line to `semantics.md` (2,334 lines at 5983a6b9502), because
   `ess/domains/cli.yaml` cites `semantics.md:1508` and `:1512` and this story edits no ESS file.
9. `CHANGELOG.md` has a Changed entry: the three commands read through a running owner of the
   same build, and each of them now restarts the idle interval of any running owner, also one
   of another build.
10. The drift checks of `AGENTS.md:149-153` exit 0, and `git diff --stat` against the base shows
    no change: `git diff --stat 5983a6b9502 -- ess apps/connectors/spec
    crates/connectors-build/generated crates/connectors-core/generated 'adapters/*/generated'`
    prints nothing.
11. `cargo nextest run -p connectors-host -p connectors -E 'binary(cold_owner_start) |
    test(owner_observe) | (test(owner::) -
    test(bounded_owner_serves_exactly_the_realizations_the_projection_serves) -
    test(adversary_concurrent_issues_within_the_lead_send_one_exchange))'` exits 0 with no test
    over 60 s (the nextest `slow-timeout`), and the run step, without the build, takes at most
    120 s. Each new test's time is in the report. The two tests left out exceed 60 s without
    this change: on a lib test binary of b3eab7f8575 (no file of this story), run alone on
    2026-11-23, `bounded_owner_serves_exactly_the_realizations_the_projection_serves` took
    181.90 s (load 39 to 61) and `adversary_concurrent_issues_within_the_lead_send_one_exchange`
    544.69 s (load 61 to 128). They are added to story:store-tests-end-within-60s-under-load.

Notes for the implementor: the build of the two packages' test binaries is one pool job with
`SLOT_TIMEOUT=30m`; the timed runs of line 6 start without the `slot` prefix.

## Shares files with

- `CHANGELOG.md`: story:entity-runtime-0330-records-name-definition-by-digest (34 lines under
  the unreleased heading on branch `story/er-0330-pin`), story:timeout-refusal-says-how-long-it-waited,
  story:guarded-write-credential-refusal-says-repair. The second to merge rebases its entry.
- `apps/connectors/tests/cold_owner_start.rs`: the pin story lists it in its scope (inferred).
  The pin runs it (`binary(cold_owner_start)` in its acceptance 8) and its branch diff does not
  edit it; this story edits it for acceptance 1 to 4 and 6.
- `crates/connectors-host/src/local/owner/admission_tests.rs`:
  story:host-gate-load-sensitivity-w4 (draft, no scope) cites it at `:84`, `:105`, `:133`,
  `:248` and `:293` in its body, so its region is unassessed. This story's region: a new test
  beside the fixture at `:140-183` and `answer_once` at `:191`.
- `crates/connectors-host/src/local/owner.rs`, `contracts/cli/v1alpha1/semantics.md`:
  story:guarded-write-credential-refusal-says-repair (cited there; its regions are not stated).
  This story's regions in `owner.rs`: the request enum near `:1411-1413` and a wrapper beside
  `:683-692`.
- `contracts/cli/v1alpha1/semantics.md`: story:definition-files-in-user-config and
  story:timeout-refusal-says-how-long-it-waited cite it by line; this story edits `:1378-1380`.
- `crates/connectors-host/src/local/owner/transport.rs`: story:owner-refuses-buildless-callers,
  story:owner-counters-are-panic-safe, story:host-gate-load-sensitivity-w4,
  story:write-reply-waits-out-the-owners-delivery-grace,
  story:served-provider-memory-growth-under-repeated-syncs and
  story:test-children-die-with-their-parent name no scope, so their regions are unassessed.
  This story's regions: a client method beside `:1061-1093`, an alias match beside `:2782-2793`
  and an `action` arm beside `:3120-3157`.
- `apps/connectors/tests`: story:test-binary-per-package (active, no scope) has not reached
  this package; when it does, `cold_owner_start.rs` becomes a module of `tests/main.rs`.
- `CHANGELOG.md`, `contracts/cli/v1alpha1/owner.md`, `contracts/cli/v1alpha1/semantics.md`,
  `docs/local-er-metadata.md`, `website/docs/reference/contracts/cli.md`:
  story:settled-read-uses-leave-the-store (branch `story/read-uses-in-memory`) edits them too.
  The second to merge rebases its entries.
- No file under `ess/` and no generated file: the clash with the pin story on
  `crates/connectors-build/generated/metadata-emissions/inputs.json` moved to
  story:owner-observe-request-is-modelled-in-ess.

## Out of scope

- Store size itself: issue #267 items 1 and 2 (the pin story, and
  story:settled-read-uses-leave-the-store with question D5).
- The owner's own time for these reads on a large store, and its fresh open when its handle is
  in use: line 6 reports the time. If the median with an owner is more than half of the median
  without, a story for the owner's time is drafted from that report.
- `connections revalidate`, `revoke`, `launch` and `token`: they write or hand out a
  credential, and their `select` read is followed by a transaction the CLI makes itself. Item 3
  names none of them.
- `setup check`, the `definition` commands and `endpoint`: item 3 names none of them, and
  `setup check` is the command that verifies the store on purpose.
- A cached list in the owner's memory.
- The ESS model of the request (story:owner-observe-request-is-modelled-in-ess).
