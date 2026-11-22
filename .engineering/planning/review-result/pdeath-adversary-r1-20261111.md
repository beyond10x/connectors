---
format: aep.planning-md/3
id: review-result:pdeath-adversary-r1-20261111
kind: review-result
status: active
title: 'Adversary pass 1: test-children-die-with-their-parent'
relations:
- reviews: story:test-children-die-with-their-parent
revision: 1
---
# Adversary pass 1: test-children-die-with-their-parent

Reviewed commit e39537f8000 on impl/test-children-die-with-their-parent (base 4265b2ec0b5). Five
cases in apps/connectors/tests/parent_death_adversary.rs, each re-running the test binary as a
harness, SIGKILLing it and requiring every process it published gone within 2 s: 3 passed, 2
failed. The commit's 3 named lib cases re-ran green.

Held: `print` spawns, waits and stops on one thread, so PR_SET_PDEATHSIG cannot fire early; no
production child meant to outlive its caller is bound (the owner start is untouched); the getppid
check runs after prctl against a pid taken before fork; pre_exec calls only prctl and getppid;
non-Linux code is unchanged and nothing becomes unused there (read, not built); the provider test
is red without `bind`.

Not held: the usual owner fixture (apps/connectors/tests/support/owner.rs) neither ties nor binds
the owner, so a killed test leaves it serving until its 600 s idle bound, while the tied control
meets 2 s; the adapter test passes without the death signal because the closed fd-3 channel ends
the adapter; the new harnesses are unbound and unreaped; "leaves nothing behind" is false for a
provider's own children.

All four go back to the implementor in this wave.

```findings
[
{"file":"apps/connectors/tests/support/owner.rs","line":51,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"The usual owner fixture neither ties nor binds the owner, so a SIGKILLed test leaves it serving past 2 s (red: a_killed_test_leaves_no_owner_from_the_usual_fixture); acceptance 1 was tested with a stand-in instead of an owner."},
{"file":"crates/connectors-host/src/local/runtime/process.rs","line":2231,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a_killed_owner_leaves_no_adapter_running passes without PR_SET_PDEATHSIG because the harness's death closes the adapter's fd-3 channel, which ends it on its own (process.rs:2182-2206)."},
{"file":"crates/connectors-host/src/local/runtime/process.rs","line":2246,"category":"test-gap","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The new harnesses (here and parent_death.rs:170) are unbound and never reaped, so a killed outer test or a failed pid_in or running assertion leaves the 300 s stand-in and its adapter running."},
{"file":"crates/connectors-host/src/local/parent_death.rs","line":6,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"'leaves nothing behind' (and config.rs:2208) is false for a provider's own children in its process group, which survive a killed loader (red: a_killed_load_leaves_no_provider_descendant)."}
]
```
