---
format: aep.planning-md/3
id: story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
kind: story
status: draft
title: 'Twelve tests take a template store or generated schemas built once per run: 115.0 s become an estimated 55.9 s'
summary: A store from nothing and a run of ess generate are made per test; a setup step builds each once per run under target/, keyed by a digest of its sources
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .config/nextest.toml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: adapters/sql/tests/ess_model.rs
- confidence: cited
  path: crates/connectors-build/Cargo.toml
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-build/src/test_fixtures.rs
- confidence: cited
  path: crates/connectors-host/src/local/mod.rs
- confidence: cited
  path: crates/connectors-host/src/local/mutations/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/governed/tests/bounded_realization_security.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/refresh/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/refresh/tests/issue.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/expiry_batch_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/template_store.rs
revision: 13
---
## Defect

Twelve tests pay for a product per test that could be made once per run: eleven build a metadata
store from nothing, some of them many times, and one runs `ess generate`. Together they take
115.0 s of the test step. This is the mistake the folder rule "Tests stay fast" names: an expensive
product made per test.

This story was drafted as "eighteen tests take a template store or jail built once per run". It is
now the first of three. Its id keeps the old name; an id cannot be changed. The jail went to
story:six-cli-tests-take-jail-binaries-built-once-per-run, the stress test to
story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate. The test of `ess generate` came from
story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (eleven
rows of group K, one row of group N). The plan read the sources at 0.77.2. The function names below
were read in the tree of 0.78.1. The last figure of each row is the plan's estimate after.

Unit tests of binary `connectors-host`, under `crates/connectors-host/src/local/`:

- 17.5 s, `owner/governed/tests/bounded_realization_security.rs`,
  `bounded_owner_serves_exactly_the_realizations_the_projection_serves`: a loop over about 13
  cases, each builds a `Target` and an owner read (the plan, inferred). After: 8.7 s.
- 16.6 s, `owner/refresh/tests/issue.rs`,
  `adversary_concurrent_issues_within_the_lead_send_one_exchange`: per trial a `Server::start` and
  a `Connected::with`, so a store per trial. After: 8.3 s.
- 9.6 s, `registry/tests.rs`,
  `a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity`: a fixture store plus 21
  `repair_prepare` transactions. After: 4.8 s.
- 9.3 s, `owner/mutation/tests.rs`, `final_audit_recovery_preserves_every_live_business_result`: 35
  store fixtures (5 × 7). After: 4.7 s.
- 9.1 s, `registry/expiry_batch_tests.rs`,
  `due_expiries_past_the_former_32_member_bound_join_the_callers_batch`:
  `store_with_due_read_uses(40)`. After: 4.6 s.
- 8.9 s, `registry/tests.rs`,
  `concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence`: 8 rounds, each a fresh
  fixture store. After: 4.4 s.
- 8.7 s, `registry/tests.rs`, `a_read_invoke_against_a_grown_store_replays_it_at_most_once`:
  `replay_fixture(150)`. After: 4.3 s.
- 7.2 s, `mutations/tests.rs`,
  `abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt`: 24 rounds on two
  threads. After: 3.6 s.
- 6.9 s, `registry/store_cost_tests.rs`,
  `a_reopen_after_its_own_write_refuses_every_blob_altered_since`: one reopen per blob row. After:
  3.5 s.

Integration tests of `connectors-host`, under `crates/connectors-host/tests/`:

- 7.1 s, binary `connectors-host::http_standing_grant_adversary`,
  `adversary_a_concurrent_duplicate_under_a_cap_of_one_is_a_replay_not_rate_limited`. After: 3.5 s.
- 7.1 s, binary `connectors-host::http_standing_grant`,
  `replays_waiting_on_an_original_hold_no_slot_and_at_most_four_wait_per_key`. After: 3.5 s.

Generator work per test:

- 7.0 s, binary `connectors-sql::ess_model`, `adapters/sql/tests/ess_model.rs`,
  `every_page_is_one_the_model_admits`: the helper `schema` (line 24) runs the pinned
  `ess generate` into a fresh temporary directory per test. After: 2.0 s (the plan: the 12 other
  tests of `ess_model` under 1 s average 0.3 s).

Sums, computed from the rows: before 108.0 + 7.0 = 115.0 s; after 53.9 + 2.0 = 55.9 s. For the
eleven store tests the plan's "after" is 50 % of before. That is an assumption of the plan: it did
not measure `Metadata::initialize`.

Read at 0.78.1, which of the eleven the template reaches:

- A call of `Metadata::initialize` on an empty directory is on the path of eight tests: `fixture()`
  in `registry/tests.rs` (lines 858 to 868; three tests, and `store_with_due_read_uses` in
  `expiry_batch_tests.rs` line 40 calls it), `owner/mutation/tests.rs` line 177,
  `mutations/tests.rs` line 35, `registry/store_cost_tests.rs` line 847, and `Connected::build` in
  `owner/refresh/tests.rs` (lines 314 to 323), which `issue.rs` reaches through `Connected::with`
  at line 309.
- No such call was found in the file of three tests: `bounded_realization_security.rs` (its
  `Target::new` is in `owner/governed/tests.rs` line 59) and the two `http_standing_grant` files.
  Whether a template store reaches them is not established. Their "after" is the plan's 50 %.

Read at 0.78.2, what `Metadata::initialize` (`crates/connectors-host/src/local/metadata.rs` lines
372 to 471) creates the store through:

- the module `filesystem` (`filesystem.rs` and the directory `filesystem/`): the file imports it
  as `fs` (line 3) and `initialize` calls `fs::directory`, `fs::publish_new`, `fs::sync_directory`
  and `fs::uid`, and through `lifecycle_lock` `fs::private_file_at`;
- `metadata.rs` itself (`open_connection`, `validate`, `adopt_er`, `migrate`,
  `enable_checkpoints_now`) and the files of `metadata/`: `er.rs`, `er/ledger.rs`, `deadline.rs`,
  the `.sql` files it takes in with `include_str!`, and `entity-runtime-definitions.json`;
- `metadata/er.rs` names types of the modules `runtime`, `approvals`, `audit` and `registry`.
  Whether a call of `initialize` on an empty directory runs code of those four was not followed.

## Change

The second accepted form of the folder rule: one fixture built once per test run by a setup step
before the tests, kept on disk under `target/` and keyed by the digest of its sources. An
in-process global is not this form: nextest runs each test in its own process.

1. New: a setup step. `.config/nextest.toml` gets one setup script entry that runs
   `cargo run --locked --offline --bin connectors-build -- test-fixtures` (`--bin` as the gate job
   starts the tool, `.github/workflows/rust-gate.yml` line 108). The command is a new
   subcommand in a new file, `crates/connectors-build/src/test_fixtures.rs`, entered in
   `crates/connectors-build/src/main.rs`. It is Rust, not Python and not a shell script. It follows
   `baseline_suite` in `crates/connectors-build/src/metadata_emissions.rs` (lines 127 to 151): read
   the product from `<target>/<name>/<digest>/` when it is there, else build it beside its place
   and rename it in.
   The entry carries a filter: `package(connectors-host) | package(connectors-sql)`. The script
   runs only for a run that selects tests of these two packages. They are the packages whose tests
   read the two products (points 6 and 7). The fourth step of the lane `slow`, a run of one
   `connectors-sdk` test (story:the-lane-slow-runs-the-ignored-timing-family), and a run of
   `-p connectors-build` then do not run the script.
2. Product one, the template store: `Metadata::initialize` on an empty directory, kept in
   `target/test-fixtures/template-store/<digest>/`. Digest inputs: every file under
   `crates/connectors-host/src/local/`, and `Cargo.lock` (inferred: the store's format follows the
   pinned Entity Runtime crates). The whole directory is the input because of the four modules
   named above whose reach was not followed: a list of modules could miss one. The cost: nearly
   every change of the host builds the template once more, which is one `Metadata::initialize`.
   `connectors-build` gets `connectors-host` as a direct dependency
   (`crates/connectors-build/Cargo.toml`). It has none today: its entry in `Cargo.lock` (lines 612
   to 641) lists no `connectors-host`, it takes `connectors-catalog-provider` with
   `default-features = false` (line 12 of its manifest), and that package depends on
   `connectors-host` only under its feature `service` (`adapters/catalog/Cargo.toml` lines 11 and
   12). Both manifests name `rust-version = "1.91"`. `Cargo.lock` changes by that one line.
3. Product two, the generated schemas of `adapters/sql/spec/ess`: the pinned `ess generate`, kept
   in `target/test-fixtures/sql-ess-schemas/<digest>/`. Digest inputs: every file under
   `adapters/sql/spec/ess/` and `crates/connectors-spec/toolchain.json` (the pin of `ess`).
4. The step prints one line per product: `test-fixtures: <product> built in <ms> ms, digest <hex>`
   or `test-fixtures: <product> reused, digest <hex>`. It appends the same line to the file
   `target/test-fixtures/last-run.txt`. It hands the two directories to the tests
   through the file nextest names in `NEXTEST_ENV`, as `CONNECTORS_TEST_TEMPLATE_STORE` and
   `CONNECTORS_TEST_SQL_ESS_SCHEMAS`.
5. The gate prints what the step did. `test_step` in `crates/connectors-build/src/gate.rs` (lines
   521 to 585) removes `target/test-fixtures/last-run.txt` before the tests, where it removes the
   JUnit report today (lines 559 to 563). After the tests it prints each line of the file as
   `gate: test-fixtures: <line>`, or `gate: no test-fixtures record at <path>` when the file is
   absent, where it prints the ten slowest tests today (lines 568 to 576). The gate's own output
   is in the job's log whether or not nextest shows the output of a setup script.
6. New: `crates/connectors-host/src/local/template_store.rs` (`#[cfg(test)]`, entered in
   `crates/connectors-host/src/local/mod.rs`): one function that copies the template into a test's
   own directory when the variable is set and calls `Metadata::initialize` when it is not. The
   call sites named above use it. A test never writes into the template.
7. `schema` in `ess_model.rs` reads from the directory of the variable when it is set and
   generates as today when it is not.
8. A run without the setup step (plain `cargo test`, `connectors-build ignored`) sets no variable
   and behaves as today.

`crates/connectors-host/src/local/approval_policy/tests.rs` has a `fixture()` (line 17) that calls
`Metadata::initialize` too. None of the twelve tests uses it. This story does not change that file;
it is in the scope of story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait.

That story edits test-only helpers in `metadata.rs`, a digest input. The template is then built
once more under a new digest. No edge is needed.

Bound rule of the three stories split from the first draft: bound = estimate × 1.5, rounded up to
a whole second.

## Acceptance

1. Built once: in the log of the forge's "repository gate" job of this story's pull request, each
   of the two lines `gate: test-fixtures: template-store …` and
   `gate: test-fixtures: sql-ess-schemas …` appears once. The gate prints them (point 5). The line
   of the template store names the time of one `Metadata::initialize` on the runner, which the
   plan did not measure.
2. Reused: a second run of the setup step with unchanged sources prints `reused` for both
   products and writes no file under the two product directories. A unit test of
   `test_fixtures.rs` holds this, and holds that one changed byte of a digest input gives a new
   digest and `built`.
3. The twelve tests pass, and their durations in the test step of that job sum to at most 84 s.
   Before: 115.0 s. Estimate: 55.9 s. Bound: 1.5 × 55.9 = 83.85, rounded up to 84 s.
4. Kept runs: each of the twelve tests keeps its number of cases, rounds and trials (13 cases, 21
   transactions, 35 fixtures, 40 read uses, 8 rounds, 150 events, 24 rounds, 6 rounds of 8
   invokes, 8 originals, 7 cap and limit cases). The pull request changes no loop count in them.
5. With the variable unset, the tests of `registry::tests` pass under plain `cargo test`.

## Scope

- `.config/nextest.toml`: the setup script entry. No other story of this set writes this file.
- `crates/connectors-build/src/test_fixtures.rs` (new), `crates/connectors-build/src/main.rs`
  (the subcommand), `crates/connectors-build/Cargo.toml` (the dependency) and `Cargo.lock` (its
  one line).
- `crates/connectors-build/src/gate.rs`: `test_step` (point 5). No other story of this set has
  this file in its scope. story:the-gates-test-step-ends-within-120-s-on-the-forge changes the
  same function after the stories of this set, when it makes the step fail over 120 s.
- `crates/connectors-host/src/local/template_store.rs` (new) and
  `crates/connectors-host/src/local/mod.rs` (one line).
- Under `crates/connectors-host/src/local/`: `registry/tests.rs` (`fixture`),
  `registry/expiry_batch_tests.rs`, `registry/store_cost_tests.rs`, `owner/mutation/tests.rs`,
  `mutations/tests.rs`, `owner/refresh/tests/issue.rs`,
  `owner/governed/tests/bounded_realization_security.rs`.
- `crates/connectors-host/src/local/owner/refresh/tests.rs`: `Connected::build` (lines 314 to
  323). story:five-host-unit-tests-shorten-the-waits-they-chose touches another region of this
  file: `LIFETIME_MS` and one test near line 995.
- `adapters/sql/tests/ess_model.rs`: `schema`.

The plan lists `crates/connectors-host/tests/http_standing_grant.rs` and
`crates/connectors-host/tests/http_standing_grant_adversary.rs`. No change is named for them
above, and they are not in this story's scope.

story:store-tests-end-within-60s-under-load, a draft outside this set, names eight of these tests
by function, two tests of the `http_standing_grant` binaries without a name, and the same form (a
saved store under `target/` keyed by the digest of its sources). This story owns the change for
the twelve tests it names. The older story keeps its acceptance (60 s at host load) and is measured
after this story lands; it has a `depends_on` edge to this story.

## Not in this story

- The jail binaries: story:six-cli-tests-take-jail-binaries-built-once-per-run. That story no
  longer adds a product to the setup step and reads nothing of this story.
- The 100-run stress test: story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate.
- The template store for every other test that builds a store: the 406 tests of 1 s to under 5 s
  (813.8 s) and the tests under 1 s. The plan names this as a candidate that was not measured.
- `fixture()` of `approval_policy/tests.rs` and `audit::tests::fixture`, which the plan names as
  places that pay the same cost.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: whether cargo-nextest 0.9.143 (the pin in `.github/workflows/rust-gate.yml`
  line 91) runs a setup script without an experimental switch, and whether it passes variables
  from `NEXTEST_ENV` as described; whether that version supports a filter on a setup script
  entry and leaves the script out of a run that selects no test of the two packages; whether a
  store made by one build of the library is read the same by the other (the setup step makes the
  template with the library built without `cfg(test)`, the tests read it in the `cfg(test)` build,
  and `metadata.rs` and `metadata/er.rs` have `cfg(test)` and `cfg(not(test))` branches, the
  latter at `metadata.rs` line 281 and `metadata/er.rs` lines 701, 3157 and 3186; acceptance 3
  would show a difference as red tests); whether a store copied from a template opens as its own
  store
  in every test; whether a test needs two stores to differ where copies of one template are
  equal (`initialize` writes one random authority id into the store, `metadata.rs` line 447, and
  every copy carries it); what the dependency on `connectors-host` adds to a build of
  `connectors-build` alone, as `slow.yml` (line 59) and `pages.yml` (lines 76 and 78) start it
  with `-p`; whether the
  template reaches the three tests named above; every "after" figure on the forge.
  The gate's test step has a setup step of its own already (`crates/connectors-build/src/gate.rs`
  lines 550 to 558, `baseline_suite`); the revision placed this one in `.config/nextest.toml`.
