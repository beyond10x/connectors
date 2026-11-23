---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
verdict: needs-revision

1. Outcome line "a `timeout` refusal says how long the call waited" replaces the earlier "or its refusal carries a retry hint with a delay". Nothing in the story or elsewhere claims the retry delay, and `waited_ms` is not one. Smallest fix: add to Out of scope "A retry delay on a timeout refusal: dropped by this story, because the caller picks its own deadline. Carried by <issue or story>". If you want to keep it, state it plainly as not delivered.

2. Observed line "`issues.search` answered `metadata_unavailable` at `observation`". This refusal is in "Observed", but no Design or Acceptance line and no Out of scope line mentions it again. I found no artifact for it. The only `metadata_unavailable` story I found is `story:missing-state-database-says-initialize`, which is about a missing database, not about load. Smallest fix: add to Out of scope "`metadata_unavailable` at `observation` under load: cause not shown; assumed to be the 650 MB store (issue #267); re-check after #267". Better, open an issue and name it.

3. Outcome line "instead of being refused at 30 s" together with Design line "Without it a read behaves as today (30 s)". A consumer that does not pass the flag, such as the dev-operator session on its next run, is still refused at 30 s. The Outcome reads as if the queueing is fixed for everyone, and it is not. Smallest fix: reword the Outcome to "A caller that passes `--deadline-seconds` waits up to that long. A caller that does not is refused at 30 s as before." If the default is meant to stay 30, put the reason in Design (the contract at `semantics.md:1493` says 30 s).

4. Outcome "parallel reads on one instance then wait their turn … instead of being refused". The queue already existed. The first Observed figure (5 of 16 answered, 11 refused at about 34 s) is the same story at the old deadline. The Acceptance rows use 3 reads at 1.5 s, which does not touch the 16-read burst. Only the last Acceptance line does, and it allows "or the ones that do not are refused `capacity`". Smallest fix: keep that line, and say that a refusal `capacity` there is expected from the queue of 16 and is not a failure of the story.

5. Measured fact: "One read took 2.5 s at a 3 MB store and 9.9 s at 14 MB". 16 serialized reads at 9.9 s need about 158 s, which is over the 120 s cap. At 12 MB the measured 16 reads took 22 to 46 s for the 5 that answered, so the tail would not finish in 120 s either. The story never says what the consumer gets then. Smallest fix: add to Design "At the 120 s cutoff reads still waiting are refused `timeout` at `admission` with `waited_ms` about 120000. This is expected until #267 and entity-runtime#94 shrink the store. The caller splits the burst (4 at a time answered in 7 to 11 s)." The last Acceptance line should say the real instance may still refuse some reads at 120 s, and that the count is recorded either way.

6. Out of scope line "More than one worker per instance, and a longer queue than 16." No named artifact. My grep of `.engineering/planning/story` and `epic`, and of open connectors issues, found none for it. Those are the only levers besides the store size that make a 16-read burst complete. `owner-cancel-frees-queue-slot` (draft) is related but is not this. Smallest fix: open one issue or draft story "Owner serves reads of one instance in parallel, or queues more than 16" and name it in the line, or write "dropped, no owner" so it is not read as tracked.

7. Out of scope line "The store size and per-read cost: issue #267 and beyond10x/entity-runtime#94." This one is carried. Both issues are open (#267 and entity-runtime#94 checked with gh). Add "D3 of 2026-11-23" there so the pick is findable. No change otherwise.

8. Out of scope line "A deadline flag for writes, `connections` commands and the MCP projection." It has no artifact. Nobody asked for it. The consumer only ran reads. This one can stay dropped, but say "not requested". `story:cli-connect-admission-timeout-under-load` (implemented) and `story:joined-refresh-honours-the-caller-deadline` (draft) cover nearby ground and could be named.

9. Neither the parent epic nor the consumer asked for these Design lines: "`--deadline-seconds` flag … 1 to 120", "With `--walk`, the flag is each page's deadline", and "One cutoff … not 2n". The parent epic is generic ("every finding fixed, shown already fixed, or carried"), so the flag itself is a choice this story made. The flag fits the consumer's goal, and the one-cutoff fix is a real bug, but the Acceptance lines "A call with `--deadline-seconds 2` against an owner that never greets exits within 5 s" and the `--walk` line are extras. Smallest fix: mark the one-cutoff and `--walk` lines "needed because the flag would otherwise double the wait". If you cannot justify that, cut the `--walk` line and say `--walk` stays at 30 s.

10. The earlier Acceptance line "Measured once against 0.74.x with a fresh store before any change, to separate #267 from admission" is gone. The 2026-11-23 numbers in Observed partly replace it, but they came from a 12 MB store and not from a fresh one. Smallest fix: add "Cause separation: measured on a 12 MB store and a 3 MB store (see Observed). A fresh-store run is not needed because the one-read timings already separate the two causes."
