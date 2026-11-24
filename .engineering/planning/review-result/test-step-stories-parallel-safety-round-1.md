---
format: aep.planning-md/3
id: review-result:test-step-stories-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Recorded by the session that ran the panel, 2026-11-24. The critic read the first draft of twelve
stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge. The set was revised once after this
report (eighteen stories); the report below is the critic's text about the first draft, with
the path of the worktree shortened to `<tree>`.

verdict: needs-revision

needs-revision

**Collisions among the twelve.** Lines marked "stated" are named, with regions, in both Scope sections. Lines marked "unstated" are not named in at least one Scope section. No `depends_on` edge exists between any two of the twelve. `aep plan artifact show` lists only decomposes, informed_by and serves for each.

1. `.github/workflows/slow.yml` — inferred. Eight stories each add a step and a path trigger: four-host, slow-owner (the seven-cli connect-wait story), eleven, seven-mcp, five-tests, catalog-provider, eight, eighteen. That is 28 pairs. They write the same two `paths:` lists (pull_request and push) and append to the same job. Stated: each body lists the other seven.
2. `crates/connectors-host/src/local/owner/transport.rs` — cited.
   - four-idle: test helper `idle_exit` and two tests.
   - slow-owner: `START_BOUND` and `Client::connect`.
   - seven-cli-shorten: the 5 s drain in `accept`.
   - Stated for those three, regions disjoint.
   - **eleven also lands here and does not say so.** It is cited neither in its Scope nor in the other three. The admission bound is checked at `transport.rs:2817` (`deadline_ms > connectors_sdk::now_ms() + 30_000`). More 30_000 uses sit at `:2884`, `:3000` and `:3038`.
3. `apps/connectors/tests/local_cli.rs` — cited. four-idle (`a_connected_client_keeps_an_owner_past_its_idle_bound`, line 1959), seven-cli-shorten (lines 325 and 1769), eighteen (line 1691). Stated for all three, different tests.
4. `apps/connectors/tests/owner_slow_start.rs` — slow-owner and eighteen. Stated. Shared helpers `slow()` (line 64), `serving_slowly()` (line 226) and `three_together` (line 235) sit between the tests each story edits.
5. `apps/connectors/tests/security_mcp_deadline.rs` — seven-mcp and eighteen. Stated.
6. `apps/connectors/tests/support/mounted.rs` — eleven (knobs) and eighteen (`Container::new`). Stated.
7. `.config/nextest.toml` — eighteen (cited, setup script) and eight (item 5, `ess generate` "by a setup step"). eight's body says it is inferred and its recorded scope omits the file. eighteen's body does not name eight. Unstated on one side.
8. `crates/connectors-catalog/src/definition.rs`, reader beside producer — catalog-provider's acceptance 3 puts a digest on the lowering sources, and `lower` is `definition.rs:4227`. catalog-crate edits `definition.rs` (`lower` and the ingest). Neither body names the other for this, and there is no edge. Whichever lands second makes the committed digest stale. This is 1 acceptance read traced to a producer in the set, 0 recorded by an edge.

**Files a story will touch that its scope lacks.**
- `crates/connectors-host/src/local/owner/transport.rs`, the owner spawn (about lines 1665-1695). It clears the environment and forwards only a fixed list of variables. A CLI-started owner never sees a new owner-side variable unless it is added there. eleven's clock offset and admission bound both need this, because its tests run through a CLI-started owner. four-idle and seven-cli-shorten avoid it only because their tests start the owner directly (`local_cli.rs:1003`, `owner_idle_exit_adversary.rs:81`, `owner_tether_adversary.rs:87`).
- `crates/connectors-sdk/src/lib.rs:356` (`now_ms`, about 191 callers) and `Registry::with_system_clock`. There are 20+ call sites in `connectors-host`/`apps`, for example `owner.rs`, `connections.rs` and `registry.rs`. eleven's clock offset has no single seam inside its listed files, and its fixture computes `valid_until_ms` from `now_ms()` at `mounted.rs:201`.
- `crates/connectors-host/src/local/owner/refresh/tests.rs`, where `Connected` and `Connected::with` live (lines 276-304). It is five-host's file. eighteen's per-trial `Connected::with` change lands there and eighteen lists only `refresh/tests/issue.rs`.
- `crates/connectors-host/src/local/approval_policy/tests.rs` (`fixture`, line 17). It is four-host's file. eighteen's body names `store()` of the approval-policy tests as paying the same cost; it was not opened.
- `crates/connectors-build/src/ignored.rs`, `classify()` (line 144), and `docs/development.md` lines 278-300. The ignored-test lane refuses any `#[ignore]` test it does not classify by name. Every story that moves a test out of the gate hits this if it uses `#[ignore]`. The bodies say how a test leaves the gate "is not stated in the plan". The other possible mechanism is a filter in `.config/nextest.toml`, which would add another shared file.
- The release-build check ("a build without `debug_assertions` ignores the override") appears in five stories: four-idle, slow-owner, eleven, five-tests and seven-cli-shorten. No CI job builds tests in release (`rust-gate.yml` and `gate.rs` have none). The job each would need is in no scope.
- The home of the library test that four-host moves out of `local_foundation.rs`. Unnamed.
- eighteen's setup-script file. No path yet.
- catalog-provider's new fixture directory, fast check and generator. No path yet.
- `clock.rs`: seven-cli-shorten adds an environment variable to `query_timeout()` there. five-host's quorum test calls `set_test_query_timeout` in the same area. Different files; a semantic neighbour only.

**Unassessed surfaces.**
- Cited 12, inferred-only 0, unplaceable 0. Every story has cited files.
- Inferred entries inside the scopes:
  - `slow.yml` on eight stories.
  - eleven's `apps/connectors/src/local` (a whole directory, no file). Too wide, and it is not where the seam must go (see above).
  - seven-cli-shorten's `crates/connectors-host/src/local/clock.rs`.
- Unnamed in the recorded scope: the file paths in the previous section marked "unnamed" or "no path yet", the `ignored.rs` classification table, and the release-test job.

**Proposed waves.** Strict means no shared file, hidden ones included. `slow.yml` is excluded from the grouping; see the closing paragraph.
- W1: four-idle, four-host, five-host, five-tests, seven-mcp, eight, catalog-crate. They share no file apart from `slow.yml`. Of these, four-host, five-tests, seven-mcp and eight share `slow.yml` with each other.
- W2: slow-owner and catalog-provider. slow-owner shares `transport.rs` with four-idle and seven-cli-shorten, so it waits for four-idle. catalog-provider waits for catalog-crate, because the digest covers `definition.rs`.
- W3: seven-cli-shorten. It shares `transport.rs` with slow-owner and four-idle, and `local_cli.rs` with four-idle.
- W4: eleven. It shares `transport.rs` (unstated) with three others and `mounted.rs` with eighteen.
- W5: eighteen. It shares a file with eight of the other eleven: four-idle, seven-cli-shorten, slow-owner, seven-mcp, eleven, eight, plus five-host and four-host through the unlisted files above. Only five-tests, catalog-crate and catalog-provider are file-disjoint from it. They could run beside it instead of in W1/W2.
- If the `transport.rs` regions are accepted as merge-clean (they are disjoint hunks), four-idle, slow-owner and seven-cli-shorten could share one wave. eleven could not, until it names its `transport.rs` and spawn-list edits.
- On `slow.yml`, a person decides. The remedies are an ordering edge or splitting the surface, for example one step block per story in a file each story owns. Do not leave it as eight concurrent appends to the same two lists.

**What I read.** 12 stories via `aep plan artifact show`. I ran `aep plan artifact waves --kind story --status draft` (its recorded-scope collisions, filtered to the twelve). I also read `slow.yml`, `.config/nextest.toml`, `transport.rs`, `maintenance.rs`, `clock.rs`, `ignored.rs`, `AGENTS.md`, `docs/development.md` and `refresh/tests.rs`.

**What I could not establish.**
- Which of `.config/nextest.toml` or `ignored.rs` carries the "leaves the gate" mechanism.
- Whether eleven's offset can avoid `now_ms()` and `Registry`.
- Where the release-test job lives.

Whether the plan's split is right, and whether the acceptances are checkable, is out of my lane.

```findings
- file: .engineering/planning/story/eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.md
  line: 133
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Scope does not list crates/connectors-host/src/local/owner/transport.rs (the owner spawn's cleared environment list and the 30 s check at :2817), which three other stories in the set also land on, and its seam for the clock sits in crates/connectors-sdk/src/lib.rs:356 and Registry::with_system_clock, also unlisted; the surface rests on the inferred directory apps/connectors/src/local"
- file: .engineering/planning/story/the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition.md
  line: 91
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 reads a digest of the lowering sources, which includes crates/connectors-catalog/src/definition.rs that story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process edits, and no depends_on edge runs from this story to it"
- file: .engineering/planning/story/eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.md
  line: 138
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: ".config/nextest.toml is inferred for the setup step of point 5 and absent from the recorded scope, while story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run records it as cited; neither body names the other for this file"
- file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md
  line: 180
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Scope omits crates/connectors-host/src/local/owner/refresh/tests.rs (Connected::with, five-host's file) and crates/connectors-host/src/local/approval_policy/tests.rs (store/fixture, four-host's file), both of which the body says pay the same per-test cost"
- file: .engineering/planning/story/four-host-tests-no-longer-wait-out-the-30-s-lock-wait.md
  line: 91
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the eight stories that leave the gate for the lane slow name no mechanism, and under the ignored-test lane an #[ignore] test must be classified by name in crates/connectors-build/src/ignored.rs classify() (and docs/development.md), a file in no recorded scope; the library test module that receives the moved test is also unnamed"
```
