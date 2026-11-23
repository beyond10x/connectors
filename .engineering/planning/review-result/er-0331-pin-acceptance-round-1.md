---
format: aep.planning-md/3
id: review-result:er-0331-pin-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:entity-runtime-0331-pin
relations:
- reviews: story:entity-runtime-0331-pin
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 4
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Line 4 cannot end inside 120 s and cannot be decided: the host story measured its 76-test run at 228 to 239 s, exit 100, and I count about 249 #[test] in those modules. The fifth slow test, metadata_reopen_adversary, is an integration binary that test(local::registry::) | test(local::metadata::) does not select. A killed test has no time to report, and the line passes before the pin too. Fix: 'cargo nextest run --locked -p connectors-host -E (the filterset) & !test(<the four named tests>)' prints the selected count and exits 0 in at most 120 s, and the five tests run singly with slot --timeout 30m from the lib and metadata_reopen_adversary binaries, each time reported'."
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 5
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The 1.05 bound is below the noise: the predecessor's two runs of one build read 314.5 and 350.5 ms (11 percent), and 0.33.0 was 1.27 times 0.32.0. Fix: 'Each of the three binaries (v0.74.2, e535fc5, this branch) is built in release, its store grown once with CONNECTORS_STORE_COST_EVENTS=50 CONNECTORS_INVOKES=100 CONNECTORS_STORE_COST_STORES=<own dir>, then run three times, three at a time, in rotating order; met when the median of 0.33.1 kind er.execute_batch ms_per_invoke is at most 1.10 times the 0.32.0 median and below the 0.33.0 median'. The variable is CONNECTORS_STORE_COST_INVOKES. Move the 0.33.1-on-a-0.33.0-store run to its own line: exit 0 and blob_bytes_per_invoke reported."
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 7
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Line 7 contradicts line 8: the release commit of 0.76.0 (84824a81bc3) also changed docs/running-services.md and website/docs/getting-started.md. Add both to line 7's list and to the scope. Also add that git diff -U0 Cargo.lock changes only the five entity-* sources, their versions and the connectors version, so cargo update drags in nothing else."
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 8
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Line 8 holds two outcomes, and a tag is a publication that waits for host-operator. Split it: 'gh pr checks <n> shows every required check passing', and 'git ls-remote --tags origin v0.76.1 prints one line' (the repository's tags carry a v prefix)."
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 6
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "At e535fc5 CHANGELOG.md has no '## Unreleased' (0.76.0 is dated). Word it: 'adds ## Unreleased with a ### Changed line carrying the line-5 medians'. Make the docs half decidable: rg -c '0\\.33\\.0' docs/development.md prints nothing and rg -n 'Entity Runtime 0\\.33\\.1' docs/development.md prints line 107."
- file: .engineering/planning/story/entity-runtime-0331-pin.md
  line: 2
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "Each --check regenerates its part (8 to 27 minutes), so lines 2's three checks come on top of the regeneration jobs. Make the decider 'git diff --name-only -- crates/connectors-build/generated/metadata-emissions prints only inputs.json.' The three --check jobs stay optional, since the slow lane runs them. Line 1 should also require 'rg -c entity-runtime\\?tag=0\\.33\\.1# Cargo.lock' to print 5."
```

Lines 3 and 4 pass before the pin as well, so they are regression guards, not transition checks.

**Not checked by any line:**
- Real 0.33.0 stores. Released 0.76.0 binaries will have written them, and the only 0.33.0 store read is the line-5 measurement run, which has no fixture.
- A mixed 0.76.0 and 0.76.1 install on one store.
- Whether `blob_bytes_per_invoke` stays near 24,500.
- Whether the 0.33.1 crates are versioned 0.33.1. `inputs.json` records crate versions, so the digest only changes if they are. I could not check, because the tag is not in the local cargo git cache.

The `inputs.json` and `docs/development.md` facts hold, and `Cargo.lock` has five 0.33.0 sources, so line 1's third pattern works.

I read the one story artifact plus `story:store-tests-end-within-60s-under-load`.
