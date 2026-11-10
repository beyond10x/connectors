---
format: aep.planning-md/3
id: review-result:wave-a-websearch-deadline-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on websearch-contract-states-host-deadline (wave A)
relations:
- reviews: story:websearch-contract-states-host-deadline
revision: 1
---
```
unit: story:websearch-contract-states-host-deadline (cecb05675, worktree cz-websearch + 1 untracked test file)
verdict: green
cases: executed 10→11, red 1
origin: introduced 2, pre-existing 2, undecided 0
wrote-outside-worktree: <scratch>/units/websearch-contract-states-host-deadline/scratch/{red.log,suite.log,suite2.log,docs.log}
needs-coordinator: yes
```

1. `git diff --stat` empty; one new untracked test file `adapters/tavily/tests/adversary_deadline.rs`.
2. Case `the_tool_profile_bounds_no_call_past_the_cli_deadline` (every `bounds.timeout_ms.max` in `spec/tool-profiles.json` at most 30 000): RED alone, `websearch.crawl: tool profile promises timeout_ms.max 160000, the contract says a call ends at 30 s`, `0 passed; 1 failed`, exit 101.
3. Suite: lib 2 passed, adversary_deadline 0/1, protocol 8 passed (exit 101). `connectors-docs generate --check` current. `ess validate --path adapters/tavily/spec/ess`: valid.
4. Findings (all three Acceptance bullets hold read literally):
   - `adapters/tavily/spec/tool-profiles.json:16,35,55` (and generated `tool-contract.yaml:218,262,323`) still set `timeout_ms.max: 160000`, contradicting `semantics.md:66`. Warning, CONFIRMED, pre-existing. Fix: 30000, regenerate, update `contracts/tools/v1alpha1/scenarios/tool-contract-published.yaml:29`.
   - `adapters/tavily/src/lib.rs:25-27`: the fixed 12 s rests on the false premise that the adapter cannot read the caller's deadline; the runtime passes `deadline: Instant` into `invoke_explained`/`invoke_bound` (`crates/connectors-host/src/local/runtime/server.rs:66-92`, :398-405; catalog uses it at `adapters/catalog/src/local.rs:2953`). Warning, CONFIRMED, introduced.
   - `semantics.md:66` states a 20 s / 15 s service bound Tavily does not have (local host only, no provider budget in `src/local.rs:59-74`). Note, CONFIRMED, introduced.
   - `adapters/tavily/src/lib.rs:331-336`: a crawl cut short at its timeout with fewer than `limit` pages is reported `complete: true`, against `contracts/datasources/websearch/v1alpha1/semantics.md:68`; a shorter timeout makes it likelier; Tavily's answer when it cuts a crawl short is unobserved. Note, INFEASIBLE, pre-existing.
5. Attacked, not broken: reverting to 150 turns the protocol test red; 12 is inside Tavily's 10–150 range; the 160 s `READ_POST_TIMEOUT` is never effective (runtime caps at 120 s, CLI at 30 s); search and extract send no timeout; no stale 150/160 claim in prose; ESS invariant validates; docs current.

```findings
[
 {"file": "adapters/tavily/spec/tool-profiles.json", "line": 16, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "the published tool profile (and generated tool-contract.yaml) promises timeout_ms.max 160000 for every websearch operation while semantics.md now says a call ends at 30 s and the 160 s ceiling never applies"},
 {"file": "adapters/tavily/src/lib.rs", "line": 25, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the fixed 12 s rests on the false premise that the adapter cannot read the caller's deadline; runtime::Adapter::invoke_explained/invoke_bound receive it, so 'below the remaining deadline' holds only while at least 12 s remain"},
 {"file": "adapters/tavily/contracts/websearch/v1alpha1/semantics.md", "line": 66, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the contract states a 20 s / 15 s service bound for Tavily, which ships only on the local host and whose composition sets no provider budget"},
 {"file": "adapters/tavily/src/lib.rs", "line": 331, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "a crawl Tavily ends at its timeout with fewer than limit pages is reported complete: true, against the family rule; the 12 s timeout makes that far more likely, and Tavily's answer when it cuts a crawl short is unobserved"}
]
```
