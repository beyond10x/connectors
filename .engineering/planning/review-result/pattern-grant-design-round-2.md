---
format: aep.planning-md/3
id: review-result:pattern-grant-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


**Round-1 findings**
1. Shared rule: resolved. The function now lives in `connectors-core`, with the size limit as its argument and a shared vector test (Design point 2, story line 109).
2. Cache: resolved. The cache is gone, and the member type compares by its text, so `Grant` keeps `PartialEq` (Design point 3). The cost this leaves is in new finding 1.
3. Relaxation: resolved. It is array-index only, and acceptance 10 lists the overlaps that stay refused. Clause 4 of acceptance 10 needs a small fix (finding 4 below).
4. Nulls and the contract sentence: resolved (Design point 1, acceptance 8, acceptance 21).

**The 64-pattern cap**
- The cap is coherent with the other limits. The consumer needs 3 patterns, and `GRANTS_LIMIT` times `TARGET_LIMIT` allows 4096 members (`standing.rs:56-57`).
- A served file that exceeds the cap becomes unreadable, which matches the existing 256-grant limit (acceptance 11).
- The cap is not yet stated where an operator will read it (finding 3).
- It does not bound the cost of a parse (finding 1).

**New findings**
1. Acceptance 25 times 64 typical patterns. The worst case is 64 patterns just under the 1 MiB size limit, compiled on each of the three local parses per write and on every served write. Run that case, state the memory ceiling per parse and per concurrent served write, and cut the limit or the cap until it fits. This rests on story line 314-315 and the unchanged 1 MiB (line 113).
2. Neither Design point 2 nor the Files list says whether the catalog keeps its pattern cache around the core call. If it does not, every `pattern_matches` call recompiles. If it does, the caching code stays in the catalog and the function in core must be pure. The passed limit is also unspecified: "the crate's default" is 10 MiB, so name the constant or use `Option`. This rests on `inventory.rs:201-225` and story line 109-114.
3. The new cap, the 256-byte pattern limit and the 1 MiB size limit are not in the file-bounds sentence of `K` (`compatibility.md:56`, which lists "64 KiB and 256 grants, each with 1 to 16 target members"), `C`, or the ess model. Acceptance 21 only greps for `one_of`. Add the three limits to those texts and to acceptance 21. Also say whether the 64 counts after `grant-standing` drops expired grants, as the 256-grant rule does (`semantics.md:974-975`).
4. Two items:
   - Design point 7 reorders the 0.77.0 CLI handler, but neither the scope nor the Files list names the CLI source, only the contract. Add the handler (`apps/connectors/src/cli/approvals.rs`, per the 0.77.0 story at line 229), or drop point 7 and test acceptance 6 to 9 in-process.
   - Acceptance 10 says "refused, as in 0.77.0" for two target pointers one above the other. 0.77.0 refuses that only on a grant with `input_keys` (`standing.rs:184-196`). Without `input_keys` it refuses only a repeated pointer. Say "on a grant with `input_keys`", or acceptance 15 and 16 will contradict it.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 314
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 25 times 64 typical patterns, but the bound that matters is 64 patterns just under the 1 MiB size_limit, compiled on each of the three local parses per write and on every served write, in memory per concurrent write; run the test on that worst-case pattern, state the memory ceiling per parse, and lower the host limit or the cap until it fits, resting on the story's line 113 and line 117-118"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 109
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story does not say whether the catalog keeps its compiled-pattern cache around the call to the shared function, nor which constant it passes as 'the crate's default' (10 MiB); say that the cache stays in connectors-catalog and the core function is pure, and name the constant or an Option, or every pattern_matches call recompiles, resting on crates/connectors-catalog/src/inventory.rs:201-225"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 299
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the 64-pattern file cap, the 256-byte pattern limit and the 1 MiB size limit are in no contract or model text (acceptance 21 greps only one_of), so a served operator meets the cap unannounced; add the three limits to the file-bounds sentence of K (contracts/service/compatibility.md:56), to C and to ess/domains/standing_approvals.yaml, add them to acceptance 21, and say whether the 64 counts after expired grants are dropped as the 256-grant rule does (contracts/cli/v1alpha1/semantics.md:974-975)"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 134
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design point 7 reorders the 0.77.0 grant-standing handler, but neither the scope nor Files names the CLI source (apps/connectors/src/cli/approvals.rs per the 0.77.0 story at line 229), and acceptance 10 says 'refused, as in 0.77.0' for two target pointers one above the other, which 0.77.0 refuses only on a grant with input_keys (standing.rs:184-196); add the handler to scope and Files or drop point 7 and test 6 to 9 in-process, and write 'on a grant with input_keys' in acceptance 10, resting on crates/connectors-host/src/local/standing.rs:184-196"
```
