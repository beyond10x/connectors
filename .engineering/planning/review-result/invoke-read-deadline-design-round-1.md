---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

Everything below was read in the checkout. Nothing was run, so test behaviour is inferred.

1. **"One cutoff" is not what the code can do as written.** (read) There are two clocks. `prepare` (invoke.rs:617-640) starts the owner with `Instant::now() + READ_DEADLINE` (invoke.rs:1029). It then returns rewritten args to the generated runner. The runner sets `session.deadline = now_ms() + 30_000` later, in the input phase after `operation_snapshot` (session.rs:600), so no state is shared between the two. A walk re-enters per page (`walking.run` → `run_selected`, invoke.rs:638, ~746), so "taken when the arguments are parsed" contradicts "each page's deadline". Fix: say where the one value lives. Take it in `prepare` and in each walk page, as absolute epoch ms. Have session.rs:600 read it instead of `now_ms()+30_000`, and derive the owner-start `Instant` from it.

2. **The "within n seconds plus grace, not 2n" claim is wrong for the input phase.** (read) `operation_snapshot` → `bound_described` runs owner exchanges with fixed 40 s (transport.rs:1037) and 30 s (transport.rs:1052, 1068) timeouts. These run before `session.deadline` exists. A deadline above 30 s opens no hole here, because they stay at 30/40 s. A deadline below 30 s is not honoured by them, so `--deadline-seconds 2` can still take about 30 s. The acceptance only covers an owner that never greets. The story must name this. Smallest fix: narrow the outcome to "the owner start and the read share one cutoff" and list these exchanges under Unchanged.

3. **"Reads only" is decidable at argument-parse time only as a flag combination.** (read) The code's notion of a read is no key, no approval file and not `--describe` (invoke.rs:1018-1031). A write operation named without a key is known only after `admit_operation` (`requirement.effect`, invoke.rs:~1067). The story is silent on `--describe`, on `--definition` (that path never calls `start_for_read`, invoke.rs:1000-1010, but still gets `session.deadline`), and on a write operation. Fix: refuse the flag beside `--describe` (field `deadline_seconds`), state the `--definition` behaviour, and refuse it at the effect check for a write operation.

4. **Real hole on the executable-changed path; the story must name and fix it.** (read) `follow_executable` caps the capture lease at `deadline.min(now+30_000)` (owner.rs:1135-1141). The registry requires the lease to be at most 30 s (revalidation.rs `capture`, `expires > deadline(now,30_000)`). `dispatch_revalidation` and `finish_revalidation` return `Expired` once `now >= expires` (revalidation.rs:163). Validation retries run to `read_deadline` (supervisor.rs:1516-1546, `revalidate_with_retries` supervisor.rs:168-210). Today the deadline equals the lease bound, so there is no gap. With a deadline above 30 s, a slow validation can succeed and then be refused `Expired`. Fix: pass `min(read_deadline, lease expiry)` to `validate` and the retries. Add supervisor.rs to scope (it is not listed). Add one test on that path with a 60 s deadline; the retry helper takes an injectable pause.

5. **`lapses_early` widens but is not a correctness hole.** (read) The window is `min(lifetime/2, read_deadline - now)` (supervisor.rs:1498-1503). A longer deadline lets more evidence count as lapsing, so more reads run a recollection probe first. Recollection stays clamped to 30 s (supervisor.rs:1584, kept by the story). It may be left, but add one line under Unchanged or Observed. The real-instance measurement will show the cost.

6. **`waited_ms` is a contract change with real collisions.**
   - It needs a new `Optional<Integer>` on the shared closed `connectors.cli.Failure` (ess/domains/cli.yaml:791-), with an ESS-LIMIT presence rule like `retry_after_seconds` (cli.yaml:812-815), plus regenerated outputs.
   - The two named pinned tests look safe. The mapping adversary compares the owner's `Request`/`Reply` field lists with spec structs (cli_spec_mapping_adversary.rs:1-60), and the owner wire is unchanged. cli_surface.rs:1086 uses a generic `fixture("Failure")`. (Inferred; not run.)
   - Collision: draft story:refusals-name-the-field says it edits `Failure` and "collides with every cli.yaml unit". Its `field` already exists (cli.yaml:877), so it looks stale, but it still shares the surface.
   - Two emitters exist: the start timeout in `prepare` (`owner_failure`) and the admission timeout through `dispatch`. The story names neither.
   - Fix: say `waited_ms` appears only on `timeout` at `stage = admission` or at the start (not at `dispatch`, where the time is the provider's). Say it is measured from the same instant as the cutoff, or the "2000 or more" acceptance can fail.

7. **Overlap with story:invoke-exits-promptly-on-sigterm is inferred, not declared.** That story has no scope. Both stories sit in the CLI's wait on the owner (`session.signals()`, session.rs:~583, and the owner exchanges). A 120 s deadline lengthens the window its test targets. It is not a shared-ownership defect. Smallest fix: add a relation edge with the SIGTERM story, and run its blocked-admission test with `--deadline-seconds 120`.

8. **Split `waited_ms` out.** The flag and the single cutoff are one unit: the flag means nothing unless the start and read share it, and findings 1 and 4 hang on them. `waited_ms` is diagnostics. It touches the shared spec, the generated contract and the pinned tests (finding 6), and the flag's behaviour does not depend on it. The reported refusals (14 of 16) are cured by the flag alone. Move `waited_ms` to its own story, with a `depends_on` edge on this one if wanted.

Not my lane: whether the final acceptance line (real-instance measurement) is checkable, and whether parallel work can run on cli.yaml.

Edges walked: `decomposes` to epic:review-20261123-findings, plus the cited siblings. I did not run `graph` or `validate`.
