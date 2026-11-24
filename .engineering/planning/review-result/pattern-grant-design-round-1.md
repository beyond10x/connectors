---
format: aep.planning-md/3
id: review-result:pattern-grant-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-revision


Nothing blocks the design. Four findings (all warnings) would change what is drafted. Verdicts on your five points:

- **(a) Member form and `typ`: sound.**
  - The exactly-one-of form and the unchanged `typ` and version fail closed on 0.77.0 (`standing.rs:131-136`, via `git show 43bda33b08d`).
  - A file of exact members only still reads on 0.77.0. This matches the `input_keys` precedent (`contracts/service/compatibility.md:56`).
  - One gap: the story treats `value: null` only. `"pattern": null` or `"one_of": null` would read as absent under a plain `Option`.
- **(b) The relaxation does not open an input.**
  - A grant with `input_keys` above a target admits the same inputs as the grant with the target removed. The target only adds a check (`standing.rs:322-336`).
  - Grants signed under 0.77.0 are unaffected. A target into an array with `input_keys` elsewhere was already accepted and never matched (`compatibility.md:56`). A target under an `input_keys` pointer was refused at `standing.rs:184-196`, so no such file exists.
  - It is broader than the consumer needs, and a signed file can't be narrowed later. Restrict it to the case where the token after the `input_keys` prefix is a decimal index.
  - The story also doesn't say that `input_keys` above `input_keys` and target above target stay malformed.
- **(c) Regex.**
  - Trailing newline and multi-line values are safe. The `\A(?:…)\z` wrap holds even with `(?m)`, and the crate's `$` is end-of-text. A top-level `|` matches less than it reads, which is the safe direction. Matching is case-sensitive unless the author writes `(?i)`.
  - `\d`, `\w`, `\s`, `\b` and `(?i)` are Unicode-aware, so `^DEV-\d+$` admits other digit scripts. The Jira grants use `[0-9]` and are not affected, but the contract is silent on it.
  - The cache has no bound (finding 3).
- **(d) Comment grant.** It binds only `content[0].content[0].text`. Acceptance 7 and the Design already say so. Everything after the first node is free, including links, mentions and media. I raise no finding, but the docs must say it plainly.
- **(e) Surfaces and coupling.**
  - Served and local both call `covering`, so there is one rule on both surfaces.
  - There is no cycle. The graph is this story → `local-invoke…` → `standing-grant-closes…`. Acceptance 7 and 8 read the closed-input rule transitively through the recorded edge.
  - The coupling defect is the copied regex rule (finding 1).

What I read: the story, its two 0.77.0 stories, the record, `compatibility.md:56`, `local-mutations.md:296-360`, `semantics.md:960-990`, `standing_approvals.yaml`, `standing.rs` and `inventory.rs:175-226` at 43bda33b08d, and the host and catalog Cargo.toml files. I ran `aep plan artifact graph`, `relations` and `validate` (valid).

Not established:
- The declared type of Jira status ids (the story says it did not read it).
- Real compile time and memory for patterns near the 1 MiB limit.
- The numbers in finding 3 (`TARGET_LIMIT` is at `standing.rs:57`).
- Out of my lane: the acceptance for the over-limit pattern needs a concrete example.

```findings
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 68
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the whole-value rule is copied into connectors-host, which has no edge to connectors-catalog, and the copy differs (size_limit 1 MiB against the catalog's default Regex::new at inventory.rs:220-222), so the two crates will drift; put the compile-and-match function in connectors-core (a dependency of both, crates/connectors-catalog/Cargo.toml:11 and crates/connectors-host/Cargo.toml:20) and have both call it, or state the copy and add one shared vector test run in both crates, resting on crates/connectors-catalog/src/inventory.rs:182-225"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 165
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the compiled-form cache is 'bounded' with no bound: a file may hold 256 grants of up to 16 members (standing.rs:56-57) and the local path parses it three times per write, so a cache below the distinct-pattern count recompiles on every write, and the model it copies compiles under its mutex and never evicts (inventory.rs:213-225); state the entry cap and its worst-case memory (entries times the 1 MiB size_limit), compile outside the lock, and give a number to the 'bounded time' test, resting on crates/connectors-host/src/local/standing.rs:56-57"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 74
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the relaxation lets an input_keys pointer lie above a target through any token, though only the array case is needed (the comment grant's /body/body/content then /0); an object-form relaxation is signed into files and cannot be narrowed later, while an array-only one can be widened; allow it only where the token after the input_keys prefix is a canonical decimal index, and add to acceptance 8 that input_keys above input_keys and target above target stay malformed, resting on crates/connectors-host/src/local/standing.rs:184-196"
- file: .engineering/planning/story/standing-grant-binds-a-pattern-or-a-value-set.md
  line: 162
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "only value:null gets the listed-style treatment; with plain Option fields, {pointer, value:'a', pattern:null} reads as an exact member and {pointer, pattern:null} as 'none of', unlike input_keys which is never null (standing.rs:122-128); refuse null for pattern and one_of at parse, add it to acceptance 3, and add one contract sentence that the pattern is matched whole, case-sensitively, with Unicode-aware \\d \\w \\s \\b, so ASCII classes like [0-9] are the written form, resting on crates/connectors-host/src/local/standing.rs:122-128"
```
