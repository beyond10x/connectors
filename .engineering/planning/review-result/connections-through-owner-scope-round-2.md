---
format: aep.planning-md/3
id: review-result:connections-through-owner-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: approve

All seven round-1 findings are resolved. I have no new findings.

1. **Resolved.** Outcome lines 25-27 say the story removes the CLI's own open and read and does not promise the consumer's time falls. They also say acceptance 6 reports the owner's time without a bar.
2. **Resolved.** Acceptance 6 (lines 157-162) is "A report, no bar", run at `CONNECTORS_COLD_START_STORE_MB=133`. Out of scope (lines 209-211) says a story for the owner's time is drafted from that report if the median with an owner is more than half of the median without. A failed half-bar no longer blocks closure and has a named next step.
3. **Resolved.** Source (lines 31-33) quotes wanted item 3 verbatim: "**No CLI-side full replay** for `connections list|describe|status` while an owner of this build runs…". Acceptance 1 covers all five forms of those three commands, so the `describe` and `status` arms trace to that sentence.
4. **Resolved.** Design (lines 111-115) sends a refusal by the owner to the direct read, citing `owner.md:257-261`. Acceptance 3 now expects the direct read's own code, and acceptance 7 adds `observe` to the `owner.md` sentence about a failed exchange.
5. **Resolved.** Design (lines 126-128) and Out of scope (line 218) move the ESS model to `story:owner-observe-request-is-modelled-in-ess`. That story has `depends_on` this story, and its Source says it was split out by the round-1 design and scope critics. Acceptance 10 requires no change under `ess/`, and the shared-files clash on `inputs.json` moved with it.
6. **Resolved.** The epic body now names issue #267 and lists five stories under it, this one included. It says outright that the review did not list those five.
7. **Resolved.** Out of scope (lines 212-216) gives a reason per group. The write and credential commands are followed by a transaction the CLI makes itself. `setup check` verifies the store on purpose. Item 3 names none of them.

Promise count. I extracted 5 promises from the parent and the quoted ask:
- Item 3's "No CLI-side full replay".
- Answering `list`, `describe` and `status` from the owner.
- "as `invoke` already does", meaning only to an owner of this build.
- The behaviour with no owner.
- The epic's "carried to the repository that owns it" for what is not fixed here.

I traced all 5 to the story: Outcome, Design, acceptance 1-5, and Out of scope. The follow-up story is a draft that gets its own panel later, so I did not judge it. Nothing in the set claims more than item 3 asks for.

Out of my lane, and not setting the verdict:
- Whether the acceptance 2 comparison "after `observed_at_ms` and `valid_until_ms` are removed" is checkable.
- Whether the custody observation the CLI sends holds up against the owner's cleared environment.

```findings
[]
```
