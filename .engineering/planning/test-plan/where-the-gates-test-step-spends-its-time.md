---
format: aep.planning-md/3
id: test-plan:where-the-gates-test-step-spends-its-time
kind: test-plan
status: draft
title: Where the gate's test step spends its 621 s, and the stories that shorten it
revision: 1
---
## Where the 621 s of the test step go

Sources: the durations nextest printed in the gate job 138270416028 of pull request 291 (4,318 rows, 2,474.8 s summed; the summary line of the job says 4,317 tests), and the sources of the repository at commit e722692df (0.77.2). All seconds below are computed from those durations unless marked "estimate". Paths are relative to the repository root. Nothing was built or run for this analysis. Written 2026-11-24 for story:the-gates-test-step-ends-within-120-s-on-the-forge.

Totals computed from the durations:

| Band | Tests | Seconds |
|---|---|---|
| 5 s or more | 81 | 1,117.9 |
| 1 s to under 5 s | 406 | 813.8 |
| under 1 s | 3,831 | 543.1 |
| all | 4,318 | 2,474.8 |

2,474.8 s / 4 at a time = 618.7 s, against the 621 s measured: the step is throughput bound at 4-way, so wall = summed seconds / 4. The target 120 s is 480 s summed.

Marks: `read` = the cause is in source I read (file:line given); `inferred` = from names, neighbours or arithmetic, code not read. Constants quoted: `crates/connectors-host/src/local/metadata.rs:51` WAIT_BOUND 30 s; `crates/connectors-host/src/local/owner/transport.rs:142` START_BOUND 60 s, `:259` connect wait 10 s, `:1923` client drain 5 s, `:1964` idle exit 600 s; `crates/connectors-host/src/local/owner/maintenance.rs:48` recovery sweep 5 s; `apps/connectors/src/local/invoke.rs:57` READ_DEADLINE_MS 30,000, `:60` max 120 s; `apps/connectors/src/mcp/session.rs:16` LEASE_MS 1900; `crates/connectors-host/src/local/owner/supervisor.rs:142` EARLY_READ_MARGIN_MS 2,000; `crates/connectors-host/src/local/clock.rs:29` QUERY_TIMEOUT 2 s; `crates/connectors-host/src/http.rs:237-239` connect 5 s, request 15 s; `adapters/sql/src/lib.rs:23` QUERY_TIMEOUT 10 s; `crates/connectors-host/src/local/config.rs:1505` provider print 10 s; `.config/nextest.toml` slow-timeout 30 s, terminate-after 2.

## 1. The 81 tests of 5 s or more, by seconds

Column "grp" is the cause group of section 2. Column "after" is my estimate in seconds once the seam of the group is in place (rule per group in section 2).

| # | s | binary | file:line | grp | what the time is spent on | mark | after |
|---|---|---|---|---|---|---|---|
| 1 | 47.2 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:181 | B | sleeps 31 s (half of a 60 s evidence lifetime plus 1 s), then probe_ms 40 s knob; wall clock of the owner | read | 4 |
| 2 | 35.6 | connectors::owner_slow_start_adversary | apps/connectors/tests/owner_slow_start_adversary.rs:110 | A | two reads each with a 34 s owner start delay against the 30 s default deadline (invoke.rs:57) | read | 6 |
| 3 | 33.3 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:44 | C | probe_ms 16 s run alone, then two together (serialised 2 x 16 s) under the 30 s admission bound | read | 4 |
| 4 | 31.9 | connectors::local_cli | apps/connectors/tests/local_cli.rs:1691 | K | 100 CLI process runs, each with 32 held sockets and a random pause up to 300 ms | read | 3.2 |
| 5 | 31.3 | connectors-host | crates/connectors-host/src/local/approval_policy/tests.rs:377 | F | sleeps WAIT_BOUND + 1 s = 31 s (metadata.rs:51) to outlast the production wait | read | 1 |
| 6 | 30.9 | connectors::owner_idle_exit_adversary | apps/connectors/tests/owner_idle_exit_adversary.rs:125 | G | 4 owners with a 30 s idle bound; sweep interval 5 s (maintenance.rs:48); polls until all exit | read | 3 |
| 7 | 30.3 | connectors-host | crates/connectors-host/src/local/approval_policy/tests.rs:357 | F | child action blocked keeps production lock wait: waits 30 s for a lease the test holds (approval_policy/tests.rs:310) | read | 2 |
| 8 | 30.3 | connectors-host | crates/connectors-host/src/local/adversary_w2_tests.rs:84 | F | reruns the test above in a child with the wait variable set: the same 30 s again (adversary_w2_tests.rs:84) | read | 1 |
| 9 | 30.2 | connectors-host::local_foundation | crates/connectors-host/tests/local_foundation.rs:306 | F | Metadata::initialize waits WAIT_BOUND 30 s against a held flock; the per-thread override is cfg(test) of the library and not reachable from tests/ | read | 1.5 |
| 10 | 26.4 | connectors::connection_lease | apps/connectors/tests/connection_lease.rs:42 | B | loop of reads every 250 ms over 3 x 8 s + 1 s = 25 s of wall clock | read | 4 |
| 11 | 21.9 | connectors::security_mcp_deadline | apps/connectors/tests/security_mcp_deadline.rs:843 | D | BUDGET 20 s chosen by the test (security_mcp_deadline.rs:730), reply released at BUDGET + 0.6 s | read | 6 |
| 12 | 21.7 | connectors::security_mcp_deadline | apps/connectors/tests/security_mcp_deadline.rs:870 | D | same BUDGET 20 s, reader paused until BUDGET + 0.5 s | read | 6 |
| 13 | 20.5 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:128 | C | probe_ms 18 s while a read's recollection is in flight | read | 4 |
| 14 | 20.0 | connectors-catalog::main | crates/connectors-catalog/tests/bundle_adversary_2.rs:63 | L | race loop runs until a fixed 20 s deadline unless a link is followed (bundle_adversary_2.rs:80-110) | read | 2 |
| 15 | 19.1 | connectors::connection_lease | apps/connectors/tests/connection_lease.rs:261 | C | probe 18 s, two revalidations; asserts 2 x 18 s > 30 s bound (connection_lease.rs:263) | read | 4 |
| 16 | 18.5 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:305 | C | unavailable_after_ms 16 s under a 1 s lifetime | read | 4 |
| 17 | 17.5 | connectors-host | crates/connectors-host/src/local/owner/governed/tests/bounded_realization_security.rs:157 | K | loop over about 13 cases, each building a Target and an owner read; cost per case not measured | inferred | 8.7 |
| 18 | 16.9 | connectors::mcp_tool_call_shapes_adversary | apps/connectors/tests/mcp_tool_call_shapes_adversary.rs:417 | D | REQUEST_MS 20 s (mcp_tool_call_shapes_adversary.rs:43), pages with delay_ms 4000 until the budget ends | read | 6 |
| 19 | 16.6 | connectors-host | crates/connectors-host/src/local/owner/refresh/tests/issue.rs:300 | K | TRIALS x (Server::start + Connected::with) store and refresher built per trial; cost not measured | inferred | 8.3 |
| 20 | 16.5 | connectors::adversary_local_mcp_server | apps/connectors/tests/adversary_local_mcp_server.rs:881 | E | Served::new, then waits up to 5 s for the session to end after the store is replaced | read | 6 |
| 21 | 16.0 | connectors-host::http_budget | crates/connectors-host/tests/http_budget.rs:181 | H | fixture answers after 16 s to prove the 15 s client timeout (http.rs:239) does not clip a 30 s budget | read | 1 |
| 22 | 15.5 | connectors::owner_slow_start_adversary | apps/connectors/tests/owner_slow_start_adversary.rs:131 | A | owner start delay 14 s against the 10 s connect wait (transport.rs:259) | read | 5 |
| 23 | 15.4 | connectors-host | crates/connectors-host/src/local/owner/transport.rs:4915 | G | idle bound 15 s spanning three 5 s sweeps (transport.rs:4915) | read | 1.5 |
| 24 | 15.2 | connectors-host | crates/connectors-host/src/local/owner/transport.rs:4903 | G | idle bound 15 s spanning three 5 s sweeps (transport.rs:4903) | read | 1.5 |
| 25 | 15.1 | connectors::owner_slow_start_adversary | apps/connectors/tests/owner_slow_start_adversary.rs:168 | A | owner start delay 14 s; waits for the lock then stops it | read | 5 |
| 26 | 15.0 | connectors-host::provider_timeout_adversary | crates/connectors-host/tests/provider_timeout_adversary.rs:29 | H | server stalls the body; ends at the 15 s client timeout | read | 1 |
| 27 | 13.8 | connectors::owner_slow_start | apps/connectors/tests/owner_slow_start.rs:83 | A | SLOW_START 12 s (owner_slow_start.rs:28) against the 10 s connect wait | read | 5 |
| 28 | 13.7 | connectors::cold_owner_start | apps/connectors/tests/cold_owner_start.rs:170 | A | slow_start 12 s (cold_owner_start.rs:171) | read | 5 |
| 29 | 13.6 | connectors::owner_slow_start | apps/connectors/tests/owner_slow_start.rs:146 | A | CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS 12 s | read | 5 |
| 30 | 13.5 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:91 | B | sleeps 11 s (half of a 20 s lifetime plus 1 s) | read | 4 |
| 31 | 13.4 | connectors::owner_slow_start | apps/connectors/tests/owner_slow_start.rs:117 | A | SLOW_START 12 s | read | 5 |
| 32 | 13.4 | connectors-host | crates/connectors-host/src/local/runtime/process/write_tests.rs:691 | M | five modes; expiry and preflight modes sleep a 2 s budget, a 2.5 s stall, then a doubled 4 s budget (write_tests.rs:529-560, :98) | read | 6 |
| 33 | 12.5 | connectors::setup_migrate_records | apps/connectors/tests/setup_migrate_records.rs:372 | K | Jail install of the CLI and of the test binary (copy), sha256 of the adapter, several CLI runs; split not measured | inferred | 12.5 |
| 34 | 12.5 | connectors::connector_instance_selectors | apps/connectors/tests/connector_instance_selectors.rs:280 | K | 28 commands x 2 selectors, each run a new configured state plus owner start and owner-stop | read | 12.5 |
| 35 | 11.9 | connectors-catalog-provider::main | adapters/catalog/tests/github_definition_reads_adversary.rs:425 | J | 8 calls of spent_limit, each builds engine() = definition::lower of the released GitHub definition | read | 1.2 |
| 36 | 11.9 | connectors-host | crates/connectors-host/src/local/owner/refresh/tests.rs:994 | M | sleeps LIFETIME_MS 10 s + 1 s (refresh/tests.rs:995,1008); the lifetime is the test's own constant | read | 3 |
| 37 | 11.8 | connectors::connection_lease | apps/connectors/tests/connection_lease.rs:168 | B | probes_of_a_read_at(10_500): waits until 10.5 s of a 20 s lifetime, up to 6 attempts | read | 4 |
| 38 | 11.7 | connectors::adversary_d218_u1 | apps/connectors/tests/adversary_d218_u1.rs:683 | B | connected(10_000) 10 s evidence window, polls every 250 ms until state pending | read | 7 |
| 39 | 11.6 | connectors-kubernetes::exec_credential_security | adapters/kubernetes/tests/exec_credential_security.rs:468 | N | walks /etc, /var/lib, /usr/share, /usr/include to depth 3 for a file of another owner; host dependent | read | 1 |
| 40 | 11.5 | connectors::owner_cancel_adversary | apps/connectors/tests/owner_cancel_adversary.rs:771 | M | every later launch held 5 s (owner launch delay), then the adapter is ended | read | 8 |
| 41 | 10.9 | connectors::invoke_walks | apps/connectors/tests/invoke_walks.rs:784 | M | page_ms 3000 x 3 pages under --deadline-seconds 8 | read | 5 |
| 42 | 10.7 | connectors::connection_lease | apps/connectors/tests/connection_lease.rs:161 | B | probes_of_a_read_at(9_500) | read | 4 |
| 43 | 10.6 | connectors::adversary_local_mcp_server | apps/connectors/tests/adversary_local_mcp_server.rs:1117 | E | sleeps until 9 s after dispatch to see that the stalled provider read closed | read | 3 |
| 44 | 10.4 | connectors-host | crates/connectors-host/src/local/oauth_adversary_pass2_tests.rs:403 | L | two redirect_delay runs of up to 8 s; asserts only the first (oauth_adversary_pass2_tests.rs:404-407) | read | 5.2 |
| 45 | 10.2 | connectors::connection_lease_adversary | apps/connectors/tests/connection_lease_adversary.rs:259 | B | lifetime 8 s, sleeps until 1.6 s before valid_until, then invoke_ms 2.5 s | read | 5 |
| 46 | 10.2 | connectors::local_cli | apps/connectors/tests/local_cli.rs:1769 | M | the loop ends at its own deadline (asserted under 12 s); the bound was not located in source | read | 10.2 |
| 47 | 10.2 | connectors::catalog_provider_failure | apps/connectors/tests/catalog_provider_failure.rs:201 | M | hanging provider until the 10 s print deadline (config.rs:1505) | read | 1.5 |
| 48 | 10.1 | connectors-sql::mysql_protocol | adapters/sql/tests/mysql_protocol.rs:587 | I | query hangs until QUERY_TIMEOUT 10 s (sql/src/lib.rs:23, mysql.rs:415) | read | 1.2 |
| 49 | 10.1 | connectors-sql::protocol | adapters/sql/tests/protocol.rs:539 | I | query hangs until QUERY_TIMEOUT 10 s (sql/src/lib.rs:394) | read | 1.2 |
| 50 | 10.0 | connectors-catalog-provider::main | adapters/catalog/tests/local_runtime/guarded_merge.rs:143 | L | two poll(5000) timeouts in finish_filtered_owner (guarded_merge.rs:94,112) | read | 0.3 |
| 51 | 10.0 | connectors-catalog-provider::main | adapters/catalog/tests/adversary_one_binary.rs:159 | L | polls an env var for 10 s; under nextest no other suite shares the process, so it cannot fail | read | 0 |
| 52 | 9.6 | connectors-host | crates/connectors-host/src/local/registry/tests.rs:1463 | K | fixture store plus 21 repair_prepare transactions; split not measured | inferred | 4.8 |
| 53 | 9.3 | connectors-host | crates/connectors-host/src/local/owner/mutation/tests.rs:50 | K | 35 audit store fixtures (5 x 7), each anchor, confirm and recover | read | 4.7 |
| 54 | 9.1 | connectors-host | crates/connectors-host/src/local/registry/expiry_batch_tests.rs:136 | K | store_with_due_read_uses(40) builds 40 read uses | inferred | 4.6 |
| 55 | 9.0 | connectors-catalog-provider::main | adapters/catalog/tests/github_definition_reads_adversary.rs:368 | J | raw_read builds engine() per call, 1 MiB base64 bodies | read | 1.5 |
| 56 | 9.0 | connectors::local_cli | apps/connectors/tests/local_cli.rs:325 | M | source answering after 2.5 s and a silent source against the 2 s clock query timeout (clock.rs:29), several CLI rows | read | 4 |
| 57 | 8.9 | connectors-host | crates/connectors-host/src/local/registry/tests.rs:787 | K | 8 rounds, each a fresh fixture store, repair and three racing threads | read | 4.4 |
| 58 | 8.7 | connectors-host | crates/connectors-host/src/local/registry/tests.rs:2277 | K | replay_fixture(150): 150 recorded events | read | 4.3 |
| 59 | 8.3 | connectors-catalog-provider::main | adapters/catalog/tests/local_runtime.rs:1154 | M | provider Retry-After of 1 s taken twice plus a 3 s deadline case; Retry-After is whole seconds | read | 8.3 |
| 60 | 8.2 | connectors::renamed_commands_adversary | apps/connectors/tests/renamed_commands_adversary.rs:325 | K | container, connect, rotate, then a loop of CLI runs over every argument combination | read | 8.2 |
| 61 | 8.2 | connectors::adversary_local_mcp_server | apps/connectors/tests/adversary_local_mcp_server.rs:1264 | E | a release thread sleeps 7 s and the test joins it, though the assert is under 3 s | read | 4 |
| 62 | 7.5 | connectors-catalog-provider::main | adapters/catalog/tests/github_definition_reads.rs:388 | J | engine() per list id (lists() loop) | read | 1 |
| 63 | 7.4 | connectors-catalog::main | crates/connectors-catalog/tests/definition_published_features.rs:231 | L2 | four lower() calls of the released definition plus one variant each, about 1.5 s each | read | 2 |
| 64 | 7.4 | connectors-catalog-provider::main | adapters/catalog/tests/github_definition_reads.rs:362 | J | engine() per list id (lists() loop) | read | 1 |
| 65 | 7.2 | connectors-host | crates/connectors-host/src/local/mutations/tests.rs:453 | K | 24 rounds of prepare, open_dispatch against abort on two threads | read | 3.6 |
| 66 | 7.1 | connectors-host::http_standing_grant_adversary | crates/connectors-host/tests/http_standing_grant_adversary.rs:258 | K | 6 rounds x 8 concurrent invokes, each round rewrites the grant file | read | 3.5 |
| 67 | 7.1 | connectors-host::http_standing_grant | crates/connectors-host/tests/http_standing_grant.rs:949 | K | 8 originals held at a gate, then replays; polls until states are reached | inferred | 3.5 |
| 68 | 7.0 | connectors::owner_cancel_adversary | apps/connectors/tests/owner_cancel_adversary.rs:905 | M | sleeps 0.7 s and 1.5 s around a SLOW read of two sessions | read | 4 |
| 69 | 7.0 | connectors-sql::ess_model | adapters/sql/tests/ess_model.rs:1302 | N | schema() runs the pinned ess generate in a fresh temp dir per test (ess_model.rs:24-40), then 7 cap/limit cases | read | 2 |
| 70 | 6.9 | connectors-host | crates/connectors-host/src/local/registry/store_cost_tests.rs:583 | K | one reopen per blob row with simulate_process; per-row store verification | read | 3.5 |
| 71 | 6.7 | connectors-catalog-provider::main | adapters/catalog/tests/adversary_git_relay.rs:1053 | M | forge hold_push_ms 3000 plus a 3 s sleep (adversary_git_relay.rs:1066,1078) | read | 2 |
| 72 | 6.6 | connectors-host | crates/connectors-host/src/local/protected/tests.rs:200 | M | three modes; late answers 5.5 s after the prompt (protected/tests.rs:201-203) | read | 3 |
| 73 | 6.4 | connectors-catalog-provider::main | adapters/catalog/tests/security_git_relay.rs:757 | M | check sleeps 8 s with a 1 s timeout and a 3 s survivor probe (security_git_relay.rs:757-770) | read | 3 |
| 74 | 6.2 | connectors-host | crates/connectors-host/src/local/clock/tests.rs:441 | M | silent source waits the 2 s clock query timeout per row; set_test_query_timeout exists (clock.rs:49) and is not used here | read | 1 |
| 75 | 6.0 | connectors::adversary_local_mcp_server | apps/connectors/tests/adversary_local_mcp_server.rs:1171 | E | sleeps 4 s with the store held | read | 3 |
| 76 | 6.0 | connectors::owner_slow_start | apps/connectors/tests/owner_slow_start.rs:279 | K | three reads queued behind a serving_slowly fixture, deadline 20 s | read | 5.9 |
| 77 | 5.9 | connectors::local_cli | apps/connectors/tests/local_cli.rs:1959 | G | idle bound 1500 ms; asserts no exit within 4 s, then waits for the idle exit | read | 3.5 |
| 78 | 5.8 | connectors::owner_recovers_after_failed_acquisition | apps/connectors/tests/owner_recovers_after_failed_acquisition.rs:119 | K | failed first connect, then a retry loop every 500 ms inside WINDOW + 20 s | read | 5.8 |
| 79 | 5.7 | connectors::security_mcp_deadline | apps/connectors/tests/security_mcp_deadline.rs:678 | K | exposures_within 2000 ms budget with the store held; Served::new | read | 5.7 |
| 80 | 5.6 | connectors::owner_tether_adversary | apps/connectors/tests/owner_tether_adversary.rs:162 | M | waits the 5 s client drain bound (transport.rs:1923) plus 2 s | read | 2 |
| 81 | 5.0 | connectors-host::provider_timeout_adversary | crates/connectors-host/tests/provider_timeout_adversary.rs:72 | H | server never answers the ClientHello; ends at the 5 s connect timeout (http.rs:237) | read | 1 |

Group totals of section 1 (computed from the table): A 7 tests 120.8 s, B 7 tests 131.5 s, C 4 tests 91.4 s, D 3 tests 60.5 s, E 4 tests 41.3 s, F 4 tests 122.0 s, G 4 tests 67.4 s, H 3 tests 36.0 s, I 2 tests 20.2 s, J 4 tests 35.9 s, K 18 tests 190.6 s, L 4 tests 50.5 s, L2 1 test 7.4 s, M 14 tests 123.7 s, N 2 tests 18.6 s. Sum 1,117.9 s (81 tests). Sum of "after": 324.3 s.

## 2. Groups by cause, seam, seconds left

How "after" is estimated, in every group: where a wait is replaced by a shorter test-chosen value, after = the same binary's own fixture cost per test (measured from the durations: tests of the same file in the band 1 s to under 5 s) + the new wait + 0.5 s. Where the cost is fixture work I did not measure, after = 50 % of before, stated as an assumption. Nothing was re-timed on the forge.

| grp | cause | tests (section 1 rows) | before s | seam that removes the wait | after s (estimate and how) |
|---|---|---|---|---|---|
| A | CLI starts an owner whose start is delayed longer than the 10 s connect wait (`transport.rs:259`) or the 30 s read default; real sleep in the owner | 7: `owner_slow_start.rs:83,117,146`, `owner_slow_start_adversary.rs:110,131,168`, `cold_owner_start.rs:170` | 120.8 | Existing: debug-only `CONNECTORS_TEST_OWNER_START_DELAY_MS`, `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`, flag `--deadline-seconds` (1 to 120, `invoke.rs:57-72`). Missing: none for the 10 s connect wait and START_BOUND 60 s (`transport.rs:142,259`): needs a new debug-only env like `CONNECTORS_TEST_OWNER_IDLE_MS` (new seam in `transport.rs`) so a 1.5 s delay exceeds a 1 s wait | 36.0: fixture 3.0 s (`owner_slow_start` band: 8.9 s / 3 tests) + delay 1.5 s + 0.5 s = 5 s each; 6 s for the two-read case |
| B | Evidence lifetimes and half-life rules proved on the owner's wall clock: the test sleeps the lifetime | 7: `connection_lease.rs:42,161,168`, `connection_lease_adversary.rs:91,181,259`, `adversary_d218_u1.rs:683` | 131.5 | None: needs a new seam. The adapter fixture knobs `evidence_ms`, `probe_ms`, `invoke_ms` exist (`apps/connectors/tests/support/mounted.rs:160-226`) but the owner reads the system clock (`Registry::with_system_clock`, e.g. `apps/connectors/src/local/connections.rs:15`). A debug-only clock offset the test advances instead of sleeping, in `apps/connectors/src/local/` and `crates/connectors-host/src/local/owner/supervisor.rs`. A lifetime under about 3 s is not an option: `connection_lease.rs:43-45` says one read costs about 3 s | 32.0: about 4 s each (same binary band: 2.9 s per test + 1 s); `adversary_d218_u1` 7 s because its window must outlast a connect |
| C | Revalidation probes of 16 to 18 s proving behaviour under the 30 s admission bound | 4: `connection_lease.rs:261`, `connection_lease_adversary.rs:44,128,305` | 91.4 | None: needs a new seam. The bound is the contract's owner deadline of `connections revalidate` (`connection_lease.rs:14-16`); where production sets 30 s I did not find (candidates: `crates/connectors-host/src/local/access_token.rs:113`, `crates/connectors-host/src/local/owner.rs:1145`). Needs a debug-only override; `probe_ms` already exists | 16.0: 4 s each (same rule as B) |
| D | Request budget of 20 s chosen by the test | 3: `security_mcp_deadline.rs:843,870`, `mcp_tool_call_shapes_adversary.rs:417` | 60.5 | Existing: the MCP companion config `limits.request_milliseconds` (`security_mcp_deadline.rs:408-420`, `exposures_within`); the tests pass 20,000 (`:730`, `mcp_tool_call_shapes_adversary.rs:43`). Use 4,000. For `mcp_tool_call_shapes_adversary` that its config also feeds the budget is `inferred` | 18.0: 6 s each (fixture 3 s + budget 4 s - overlap, rounded up) |
| E | Test-chosen sleeps and holds in MCP session tests (store held 4 s, 7 s; sleep until 9 s after dispatch; wait up to 5 s) | 4: `adversary_local_mcp_server.rs:881,1117,1171,1264` | 41.3 | Existing: none needed; shorten the test's own values to multiples of LEASE_MS 1900 (`apps/connectors/src/mcp/session.rs:16`). `:1264` sleeps 7 s in a thread the test joins though its assertion is under 3 s (read) | 16.0: fixture 2.1 s (band median of the binary) + 1 to 3 s |
| F | Production lock wait WAIT_BOUND 30 s (`metadata.rs:51`) lived through in real time | 4: `approval_policy/tests.rs:357,377`, `adversary_w2_tests.rs:84`, `crates/connectors-host/tests/local_foundation.rs:306` | 122.0 | Existing: `child_lock_wait` (`metadata.rs:230-239`) for the `blocked` child (`approval_policy/tests.rs:310`), and `set_lifecycle_lock_wait` (`metadata.rs:272`) which is cfg(test) of the library, so `local_foundation.rs` (an integration test) cannot reach it: move the test into the library tests. New: a child action that reports its effective wait without waiting, for `:377` (sleeps WAIT_BOUND + 1 s) and `adversary_w2_tests.rs:84` (reruns `:357`, 30.3 s twice) | 5.5: 2 + 1 + 1 + 1.5 s |
| G | Idle exit bound spanning several 5 s recovery sweeps | 4: `owner_idle_exit_adversary.rs:125`, `transport.rs:4903,4915`, `local_cli.rs:1959` | 67.4 | Existing: `CONNECTORS_TEST_OWNER_IDLE_MS` (debug-only), the `bound` argument of `idle_exit` (`transport.rs:4790`). Missing: the sweep interval is a literal 5 s (`maintenance.rs:48`): new debug-only interval | 9.5: 3 + 1.5 + 1.5 + 3.5 s (`local_cli.rs:1959` keeps a 2 s no-exit window) |
| H | HTTP client timeouts 15 s and 5 s lived through | 3: `http_budget.rs:181`, `provider_timeout_adversary.rs:29,72` | 36.0 | Existing: `with_provider_budget` (`http.rs:135`, 1 ms to 30 s) shortens a budget, but the legacy clip 15 s and connect 5 s are literals (`http.rs:237-239`): new seam `ScopedHttp` timeouts for tests. Whether a short provider budget gives the same Timeout / `upstream_answer` result is `inferred` | 3.0: 1 s each |
| I | SQL query timeout 10 s lived through | 2: `adapters/sql/tests/mysql_protocol.rs:587`, `adapters/sql/tests/protocol.rs:539` | 20.2 | None: needs a new seam. `QUERY_TIMEOUT` is a constant (`adapters/sql/src/lib.rs:23`, used `:394`, `mysql.rs:415`); `stalled()` passes no deadline (`protocol.rs:515-536`) | 2.4: 1.2 s each |
| J | `engine()` per call: `definition::lower` of the released GitHub definition (gunzip, sha256, ingest, bundle write; `crates/connectors-catalog/src/definition.rs:4227-4340`), about 1.5 s per call; `engine()` is copied in 3 files | 4 of the 81 (`github_definition_reads_adversary.rs:425,368`, `github_definition_reads.rs:388,362`) and 62 band tests in the same 3 modules (`definition_published_features_adversary.rs` 32 tests 89.5 s, `github_definition_reads_adversary.rs` 17 tests 29.6 s, `github_definition_reads.rs` 13 tests 20.4 s) | 35.9 + 139.5 | Form 1 of the folder rule: commit the lowered output of `definitions-0.5.2/connectors/github.yaml` as a generated file with a digest of its sources, tests read it; the lowering itself is checked in lane `slow` by byte comparison. No seam needed in code | 4.7 + 8.7 = 13.4: 0.14 s per test, the mean of the 55 `slack` tests in the same binary (7.5 s / 55 tests, the durations) |
| L2 | The same lowering, with one appended operation per case, in the catalog crate: inputs differ per test, so a committed file does not apply | 1 of the 81 (`definition_published_features.rs:231`, 4 lowerings) and 10 band tests (`definition_published_features*.rs`, 24.2 s) | 7.4 + 24.2 | None: needs a new seam: ingest of the released definition made once per process and reused by `lower` (`definition.rs:4227-4340`). Which sub-step of `lower` dominates I did not measure | 2.0 + 3.0 = 5.0 (estimate, 0.3 s per test) |
| L | Waits that prove nothing more than a shorter one, or nothing under nextest | 4: `bundle_adversary_2.rs:63` (race loop until a fixed 20 s), `adversary_one_binary.rs:159` (polls an env var 10 s; each nextest test is its own process, so no other suite can write it), `oauth_adversary_pass2_tests.rs:403` (two 8 s measurements, only the first asserted), `guarded_merge.rs:143` (two `poll(5000)` in the helper at `:94,:112`) | 50.5 | Existing: none needed. Cap `bundle_adversary_2` by attempts (2 s); move `adversary_one_binary:159` to lane `slow` (it can only fail under `cargo test`); drop the unasserted `held` run; give `finish_filtered_owner` a poll timeout parameter | 7.5: 2 + 0 + 5.2 + 0.3 s |
| M | Test-chosen real waits and holds, each with its own bound (14 rows, below) | 14 | 123.7 | per row below | 61.0 |
| N | Host or generator work per test | 2: `exec_credential_security.rs:468` (walks `/etc`, `/var/lib`, `/usr/share`, `/usr/include` to depth 3 for a foreign-owned file), `adapters/sql/tests/ess_model.rs:1302` (`ess generate` in a fresh temp dir per test, `:24-40`) | 18.6 | `ess_model`: Form 2, one shared output built once by a setup step under `target/` keyed by a digest of `adapters/sql/spec/ess`; `exec_credential_security`: none: needs a new seam (try a short candidate list before the walk) | 3.0 (estimate: the 12 other `ess_model` tests under 1 s average 0.3 s) |
| K | Per-test fixture work: store from nothing, container jail with a copy of two binaries, loops of CLI runs, seeded stores | 18 | 190.6 | Form 2 (see section 3): a template store and a template jail built once by a setup step under `target/`, copied or linked per test | 107.7: 50 % of before. Assumption: I did not measure `Metadata::initialize`, `Container::new` or a CLI run separately |

Rows of group M and their seams:

| s | test | seam | after s |
|---|---|---|---|
| 13.4 | `write_tests.rs:691` (budgets 2 s, stall 2.5 s, doubled 4 s; `:529-560`, `:98`) | existing: the budget is the test's (`expiring`, `EXPIRING_CAP`); start at 1 s and stall 1.2 s | 6 |
| 11.9 | `refresh/tests.rs:994` (sleeps LIFETIME_MS 10 s + 1 s) | existing: the lifetime is the test's own constant; use 1,000 | 3 |
| 11.5 | `owner_cancel_adversary.rs:771` (5 s launch hold) | existing: `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`; use 1.5 s | 8 |
| 10.9 | `invoke_walks.rs:784` (`page_ms` 3000 x 3 pages) | existing: knob `page_ms`, `--deadline-seconds` minimum 1 | 5 |
| 10.2 | `local_cli.rs:1769` (loop ends at its own deadline, asserted under 12 s) | none: needs a new seam; the loop's bound was not located | 10.2 |
| 10.2 | `catalog_provider_failure.rs:201` (hang until the 10 s print deadline, `config.rs:1505`) | none: needs a new seam in `crates/connectors-host/src/local/config.rs` | 1.5 |
| 9.0 | `local_cli.rs:325` (source answering after 2.5 s, silent source, 2 s query timeout `clock.rs:29`) | `set_test_query_timeout` exists (`clock.rs:49`) but is cfg(test) thread-local: unit tests only; the CLI needs a debug-only env | 4 |
| 8.3 | `adapters/catalog/tests/local_runtime.rs:1154` (Retry-After of 1 s taken twice, 3 s deadline case) | none: Retry-After is whole seconds; stays | 8.3 |
| 7.0 | `owner_cancel_adversary.rs:905` (sleeps 0.7 s and 1.5 s) | existing: shorten the test's sleeps | 4 |
| 6.7 | `adversary_git_relay.rs:1053` (`hold_push_ms` 3000 + 3 s sleep) | existing: `hold_push_ms` | 2 |
| 6.6 | `protected/tests.rs:200` (`late` answers 5.5 s after the prompt) | existing: the deadline handed to the child | 3 |
| 6.4 | `security_git_relay.rs:757` (check sleeps 8 s, timeout 1 s, 3 s survivor probe) | existing: shorten the sleeps | 3 |
| 6.2 | `clock/tests.rs:441` (silent source costs the 2 s query timeout per row) | existing: `set_test_query_timeout` (`clock.rs:49`), not used by this test | 1 |
| 5.6 | `owner_tether_adversary.rs:162` (5 s client drain bound `transport.rs:1923` + 2 s) | none: needs a new debug-only override of the drain bound in `transport.rs` | 2 |

Group K rows: `local_cli.rs:1691` (100 CLI runs; keep 10 in the step, the 100 in lane `slow`: 3.2 s), `bounded_realization_security.rs:157`, `refresh/tests/issue.rs:300`, `setup_migrate_records.rs:372`, `connector_instance_selectors.rs:280`, `registry/tests.rs:1463,787,2277`, `owner/mutation/tests.rs:50`, `registry/expiry_batch_tests.rs:136`, `renamed_commands_adversary.rs:325`, `mutations/tests.rs:453`, `http_standing_grant_adversary.rs:258`, `http_standing_grant.rs:949`, `registry/store_cost_tests.rs:583`, `owner_slow_start.rs:279`, `owner_recovers_after_failed_acquisition.rs:119`, `security_mcp_deadline.rs:678`. Three of them (`renamed_commands_adversary.rs:325`, `connector_instance_selectors.rs:280`, `setup_migrate_records.rs:372`) are CLI process loops: 8.2 s, 12.5 s, 12.5 s; no seam is named for them, so I kept their seconds unchanged (after = before).

## 3. The band 1 s to under 5 s: 406 tests, 813.8 s

Per binary (the durations), the binaries with more than 20 s in the band:

| binary | band tests | band s | binary median (all tests) | band median |
|---|---|---|---|---|
| `connectors-host` (unit) | 103 | 189.5 | 0.203 s | 1.53 s |
| `connectors-catalog-provider::main` | 75 | 166.8 | 0.049 s | 1.61 s |
| `connectors-catalog::main` | 23 | 45.2 | 0.011 s | 1.55 s |
| `connectors::adversary_local_mcp_server` | 14 | 35.4 | 3.20 s | 2.09 s |
| `connectors::local_mcp_server` | 15 | 30.7 | 1.53 s | 1.68 s |

Not above 20 s but large in sum: all `connectors::*` binaries together 144 tests 302.1 s (median 1.78 s); `connectors-sql`, `connectors-kubernetes`, `connectors-build` together 35 tests 62.7 s (`connectors-build::bin/connectors-build` alone 19.5 s).

### `connectors-host` (unit): 103 tests, 189.5 s

| s | test | what it pays for | mark |
|---|---|---|---|
| 4.884 | `registry::upgrade_tests::a_new_connection_that_changes_authority_or_profile_needs_a_new_instance_id` | 5 cases, each `fixture()` (a store from nothing: `Metadata::initialize`, `registry/tests.rs:858-868`) plus begin, needs-new-instance (`upgrade_tests.rs:320-331`) | read |
| 4.688 | `runtime::process::write_tests::native_outcomes_and_lost_replies_never_repeat_a_send` | 9 modes, each a child process of the test binary; `timeout` and `peer-expired-eof` take the `expiring` budget of 2 s (`write_tests.rs:589-620`) | read |
| 4.287 | `owner::governed::tests::bounded_realization_security::worker_admission_refuses_...` | a `Target` store per case plus `Pool::shared_with_read_policy` per case (`bounded_realization_security.rs:281-303`) | read |
| 4.131 | `oauth_adversary_pass2_tests::a_head_completing_at_the_idle_limit` | sleeps 2.3 s past the 2 s idle limit, then waits up to 2 s for an answer (`:435-443`) | read |
| 4.036 | `owner::refresh::tests::a_read_recollection_during_the_exchange_does_not_burn_the_rotated_token` | `Server::delayed(.., 3 s)`: the exchange itself takes 3 s (`refresh/tests.rs:851-854`) | read |
| 1.508 | `registry::tests::cursors_expire_at_the_original_five_minute_deadline` | `fixture()` + 3 prepared and published connections (`registry/tests.rs:1413-1418`) | read |
| 1.526 | `governed::tests::bounded_reads::bounded_owner_refuses_..._outside_the_resource_profile` | 4 profiles, each a `Target::new()` store and `implemented_read` (`bounded_reads.rs:411-422`) | read |
| 1.532 | `registry::binding_scope_tests::adv2_launch_admits_only_a_current_usable_connection_of_the_instance` | `fixture()` + `adv2_connect` + 3 admits (`binding_scope_tests.rs:479-486`) | read |
| 1.564 | `metadata::er::tests::a_reconnect_candidate_on_a_stale_connection_is_recorded_and_it_stays_stale` | a temp dir and `Metadata::initialize` (`metadata/er.rs:8718-8723`) | read |
| 1.590 | `config::connector_instance_tests::connector_tables_are_refused_outside_format_five_and_when_malformed` | about 14 configuration texts parsed, no catalog provider beside the test binary (`config.rs:2879-2900`) | read, cost split inferred |

Paid per test that could be paid once per run: `Metadata::initialize` on an empty directory, in `fixture()` (`registry/tests.rs:858`), `audit::tests::fixture`, `Connected::new` of the refresh tests and `store()` of the approval-policy tests. Form: one template store built once by a setup step before the tests (a nextest setup script) into `target/`, keyed by a digest of the metadata sources (`crates/connectors-host/src/local/metadata.rs` and `metadata/`), copied into each test's temp directory. An in-process `OnceLock` does not qualify: nextest runs each test in its own process. How much of the 189.5 s (and of the 161.7 s in 659 tests under 1 s) is the initialisation I did not measure: the binary median is 0.203 s against 0.049 s for `connectors-catalog-provider::main`, which suggests up to about 0.15 s per test (`inferred`).

### `connectors-catalog-provider::main`: 75 tests, 166.8 s

| s | test | what it pays for | mark |
|---|---|---|---|
| 4.992 | `local_runtime::oauth2_refresh::oauth_cache_reuses_token_until_skew` | real sleep to 4.5 s so a token valid 63 s with a 60 s skew expires (`oauth2_refresh.rs:676-698`) | read |
| 4.209 | `definition_published_features_adversary::query_fixed_on_a_download_is_refused` | 2 to 3 `engine()` or `lower` of the released definition | inferred |
| 3.694 | `definition_published_features_adversary::embedded_text_with_zero_max_bytes_is_refused` | same | inferred |
| 3.338 | `local_runtime::github_app::adversary::an_exchange_answer_outside_its_shape_is_a_protocol_failure_with_no_operation_request` | a child adapter process plus several answers; split not read | inferred |
| 3.202 | `definition_published_features_adversary::malformed_default_rate_limit_headers_are_refused_beside_only_a_download` | `lower` of the released definition per case | inferred |
| 1.605 | `definition_published_features_adversary::default_rate_limit_headers_do_not_reach_a_download` | one `engine()` (`definition_published_features_adversary.rs:25-48`) | read |
| 1.607 | `definition_published_features_adversary::the_released_fixed_values_and_folds_shape_the_declared_input` | one `engine()` | read |
| 1.608 | `github_definition_reads::repos_list_sends_the_org_path_its_order_and_the_page` | one `engine()` (`github_definition_reads.rs:28-48`) | read |
| 1.617 | `github_definition_reads_adversary::review_requests_list_refuses_a_caller_q_sort_or_order_before_any_request` | one `engine()` | read |
| 1.646 | `github_definition_reads::user_me_reads_the_authenticated_user` | one `engine()` | read |

Paid per test that could be paid once per run: the lowering of the released GitHub definition, 1.5 s, once in each of 62 band tests and 4 tests of the 81 (section 2, group J). Form: committed generated file with a digest of its sources (form 1): the lowered `operations` and bundle of `definitions-0.5.2/connectors/github.yaml`, a fast test compares the digest of the YAML tree plus the lowering sources, lane `slow` regenerates and compares bytes. The 3 copies of `engine()` read the file instead.

### `connectors-catalog::main`: 23 tests, 45.2 s

Slowest 5: `definition_published_features::query_fixed_reaches_the_selection` 4.411, `an_operation_rate_limit_headers_replace_the_default` 3.103, `definition_auth::a_defaulted_host_is_the_default_endpoint_and_the_path_under_a_variable_host_is_api_path` 3.045, `definition_published_features::query_folds_reach_the_selection` 2.991, `definition_published_features_adversary::lowering_the_released_definition_twice_writes_the_same_bytes` 2.991: each lowers the released definition once to three times (`inferred` from the 1.583 s of `the_released_github_definition_lowers`, which is one lowering; the last one is two by its name). Nearest the median (1.54 to 1.89 s): `embedded_text_reaches_the_selection`, `embedded_text_members_reach_the_selection_as_written`, `defaults_reach_writes_and_reads_beside_an_embedded_text_read_as_documented`, `the_released_github_definition_lowers`, `definition_feed::a_record_field_that_is_not_a_pointer_is_refused`: one lowering each (`inferred`, same reason). Here the inputs differ per test, so a committed file does not apply; it needs the memoised ingest of group L2. Once per run: the ingest and gunzip of the released definition (`definition.rs:4227-4340`); form 2, a cache under `target/` keyed by the digest of the definition tree, is possible only if `lower` can start from it (`inferred`).

### `connectors::adversary_local_mcp_server`: 14 tests, 35.4 s; `connectors::local_mcp_server`: 15 tests, 30.7 s

Slowest 5 and median 5 of `adversary_local_mcp_server`: 4.7 to 3.3 s `adversary_a_read_longer_than_one_lease_is_answered_and_the_session_stays_ready`, `..._a_store_held_past_one_lease_leaves_the_session_ready`, `..._a_reply_drained_after_one_lease_still_arrives`, `..._a_request_past_its_deadline_answers_timeout_and_the_session_continues`, `..._stdin_close_discards_a_reply_the_lease_cannot_carry`: each holds the store or a read across LEASE_MS 1900 (`apps/connectors/src/mcp/session.rs:16`) one or two times, on top of the fixture (`inferred` from names and from `:1171-1190`, which sleeps 4 s with the store held). Median 5 (1.54 to 3.2 s) such as `adversary_control_a_prompt_reader_gets_the_reply_and_the_ping` (1.539 s): `Served::new` plus `Peer::start` and a ping, so almost only the fixture. For `local_mcp_server`, slowest 5 (4.1 to 1.9 s: `production_stdio_withdrawal_framing_and_cancellation`, `stdout_carries_only_json_rpc_frames`, `production_stdio_stopped_session_cannot_renew_after_expiry`, `a_call_without_meta_binds_to_the_advertised_revision_on_2025_11_25`, `a_call_without_a_revision_binds_to_the_advertised_one`) and median 5 (1.52 to 1.78 s, e.g. `an_approval_required_operation_is_refused_with_no_request`): no source read, `inferred` that the median five pay the fixture only. Paid per test that could be paid once: `Container::new`, which copies the CLI binary and the test binary (about 133 MB) into the jail and takes the sha256 of the adapter (`apps/connectors/tests/support/mounted.rs:270-284`). Form 2: a template jail root built once by a setup step under `target/`, keyed by the digest of the two binaries, with the files hard-linked or bind-mounted rather than copied per test (whether the jail allows a link or a bind mount I did not check).

## 4. Stories that can be worked at the same time

"Before" and "after" are seconds summed over the tests covered; after is the estimate of section 2. Files marked (*) are touched by another story too.

| story | tests covered | before s | after s | files touched | stays in lane `slow` |
|---|---|---|---|---|---|
| S1 host lock waits | group F, 4 | 122.0 | 5.5 | `crates/connectors-host/src/local/approval_policy/tests.rs`, `crates/connectors-host/src/local/adversary_w2_tests.rs`, `crates/connectors-host/src/local/metadata.rs`, `crates/connectors-host/tests/local_foundation.rs` (moved into the library tests) | one run at the real 30 s wait that proves WAIT_BOUND itself |
| S2 owner connect wait | group A, 7 | 120.8 | 36.0 | `crates/connectors-host/src/local/owner/transport.rs` (*) (`:142`, `:254-260`), `apps/connectors/tests/owner_slow_start.rs`, `owner_slow_start_adversary.rs`, `cold_owner_start.rs` | one case at the real 10 s |
| S3 lease clock and admission bound | groups B and C, 11 | 222.9 | 48.0 | `apps/connectors/tests/connection_lease.rs`, `connection_lease_adversary.rs`, `adversary_d218_u1.rs`, `apps/connectors/tests/support/mounted.rs` (*), a new debug-only clock/bound seam in `crates/connectors-host/src/local/owner/supervisor.rs` and `apps/connectors/src/local/` | one 60 s lifetime run, one 30 s admission run |
| S4 idle exit and sweep | group G, 4 | 67.4 | 9.5 | `crates/connectors-host/src/local/owner/maintenance.rs`, `crates/connectors-host/src/local/owner/transport.rs` (*) (tests near `:4790-4930`), `apps/connectors/tests/owner_idle_exit_adversary.rs`, `apps/connectors/tests/local_cli.rs` (*) (`:1959`) | the 600 s production idle exit is not tested here |
| S5 MCP budgets and sleeps | groups D and E, 7 | 101.8 | 34.0 | `apps/connectors/tests/security_mcp_deadline.rs` (*), `mcp_tool_call_shapes_adversary.rs`, `adversary_local_mcp_server.rs` | one 20 s budget case |
| S6 client timeouts | groups H and I, 5 | 56.2 | 5.4 | `crates/connectors-host/src/http.rs`, `crates/connectors-host/tests/http_budget.rs`, `provider_timeout_adversary.rs`, `adapters/sql/src/lib.rs`, `adapters/sql/src/mysql.rs`, `adapters/sql/tests/protocol.rs`, `mysql_protocol.rs` | one run each at the production 15 s and 10 s |
| S7a released definition once (provider side) | group J, 4 of the 81 + 62 band | 175.4 | 13.4 | `adapters/catalog/tests/github_definition_reads.rs`, `github_definition_reads_adversary.rs`, `definition_published_features_adversary.rs`, a new generated fixture directory beside them | full lowering and byte comparison with the committed file |
| S7b memoised ingest (catalog crate) | group L2, 1 of the 81 + 10 band | 31.6 | 5.0 | `crates/connectors-catalog/src/definition.rs` (`:4227-4340`), `crates/connectors-catalog/tests/definition_published_features.rs`, `definition_published_features_adversary.rs` | none |
| S8a waits that prove nothing, adapters and catalog crate | 3 of group L, group N, 3 rows of M: 8 | 80.0 | 18.6 | `crates/connectors-catalog/tests/bundle_adversary_2.rs`, `adapters/catalog/tests/adversary_one_binary.rs`, `local_runtime/guarded_merge.rs`, `local_runtime.rs`, `adversary_git_relay.rs`, `security_git_relay.rs`, `adapters/kubernetes/tests/exec_credential_security.rs`, `adapters/sql/tests/ess_model.rs` | `adversary_one_binary:159` under `cargo test` |
| S8b waits in the host unit tests | rows of M and L, 5 | 48.6 | 18.2 | `crates/connectors-host/src/local/runtime/process/write_tests.rs`, `owner/refresh/tests.rs`, `oauth_adversary_pass2_tests.rs`, `protected/tests.rs`, `clock/tests.rs` | none |
| S8c waits in the CLI tests | rows of M, 7 | 64.2 | 34.7 | `apps/connectors/tests/owner_cancel_adversary.rs`, `invoke_walks.rs`, `catalog_provider_failure.rs`, `owner_tether_adversary.rs`, `local_cli.rs` (*) (`:325`, `:1769`), `crates/connectors-host/src/local/config.rs` (`:1505`), `transport.rs` (*) (`:1923`) | none |
| S9 fixture templates | group K, 18 | 190.6 | 107.7 | `crates/connectors-host/src/local/registry/tests.rs` (fixture `:858`), `registry/expiry_batch_tests.rs`, `registry/store_cost_tests.rs`, `owner/refresh/tests/issue.rs`, `owner/mutation/tests.rs`, `mutations/tests.rs`, `owner/governed/tests/bounded_realization_security.rs`, `crates/connectors-host/tests/http_standing_grant.rs`, `http_standing_grant_adversary.rs`, `apps/connectors/tests/support/mounted.rs` (*), `setup_migrate_records.rs`, `connector_instance_selectors.rs`, `renamed_commands_adversary.rs`, `owner_slow_start.rs` (*), `owner_recovers_after_failed_acquisition.rs`, `security_mcp_deadline.rs` (*), `local_cli.rs` (*) (`:1691`), `.config/nextest.toml` (setup script) | 100-run stress of `local_cli.rs:1691` |

Overlaps: `transport.rs` (S2 at `:142,:254-260`, S4 at the tests `:4790-4930`, S8c at `:1923`: three stories, different regions); `mounted.rs` (S3 knobs, S9 jail install); `local_cli.rs` (S4 `:1959`, S8c `:325,:1769`, S9 `:1691`); `security_mcp_deadline.rs` (S5 `:843,:870`, S9 `:678`); `owner_slow_start.rs` (S2 `:83,:117,:146`, S9 `:279`). S1, S6, S7a, S7b, S8a, S8b touch no file of another story.

Check of the sums: the stories cover 1,117.9 s of the 81 plus 139.5 s (62 band tests of S7a) plus 24.2 s (10 band tests of S7b) = 1,281.6 s before, and 324.3 + 8.7 + 3.0 = 336.0 s after.

### Arithmetic

| | seconds summed |
|---|---|
| all 4,318 tests now | 2,474.8 |
| minus covered by stories (before) | 1,281.6 |
| not covered: band remainder 650.1 + under 1 s 543.1 | 1,193.2 |
| plus covered by stories (after) | 336.0 |
| summed after all stories | 1,529.2 |
| divided by 4 | 382.3 s |
| target | 120 s (480 s summed) |

The target is not reached: 382 s against 120 s. 1,049 s of summed time (1,529.2 - 480) is still to go, and it is not in the 81. What is in the 1,529.2 s:

- the 81 after the seams: 324.3 s (K 107.7, M 61.0, A 36.0, B 32.0, D 18.0, C 16.0, E 16.0 and the rest 37.6), 81 s of wall;
- the band remainder: 650.1 s, 162 s of wall: `connectors::*` 144 tests 302.1 s (median 1.78 s each: per-container fixture), `connectors-host` 103 tests 189.5 s, `connectors-catalog-provider::main` 13 tests 27.3 s, `connectors-catalog::main` 13 tests 21.0 s, `connectors-sql`/`-kubernetes`/`-build` 35 tests 62.7 s, the rest of the workspace about 47.5 s (`oauth2` 6.1 s and others);
- under 1 s: 543.1 s, 136 s of wall: `connectors-host` 659 tests 161.7 s, `connectors-catalog-provider::main` 1,038 tests 119.5 s, `connectors::*` 450 tests 95.3 s, `connectors-catalog::main` 413 tests 62.0 s, `connectors-sql`/`-kubernetes`/`-build` 577 tests 58.0 s, the rest about 46.6 s. Of the 3,831 tests under 1 s, 2,112 take under 0.05 s and 2,901 under 0.2 s (the durations).

What would close the gap, `inferred` and not measured: S9 applied to the whole band and not only to its 18 tests. If the per-container and per-store fixture cost fell by half in `connectors::*` (302.1 + 95.3 s) and `connectors-host` (189.5 + 161.7 s), the sum falls by about 374 s, to about 1,155 s (289 s of wall): still not 120 s. The `connectors::*` binaries alone hold 635 tests and 1,026.6 s (41 tests 629.2 s, 144 tests 302.1 s, 450 tests 95.3 s now); the folder rule's answer for a step that stays over budget after the seams is a story of its own, not a longer timeout. I have no measurement that says a per-container cost near 0.3 s or a per-store cost near 0.05 s is possible.

Not checked (needs a run or a forge): the cost of `Metadata::initialize`, `Container::new`, a CLI run and `definition::lower` taken apart; which sub-step of `lower` dominates; the build and link time (not in the test step); the 4,318 rows against the 4,317 of the summary line; the tests under 1 s (no source read); binaries below 20 s in the band, except by their sums; whether a jail allows a hard link or bind mount of the test binary; the sources of the 5 slowest of `connectors-catalog-provider::main` except `oauth_cache_reuses_token_until_skew`, and of `local_mcp_server`; where production sets the 30 s admission bound; whether `request_milliseconds` also feeds `mcp_tool_call_shapes_adversary`; every "after" figure on the forge.
