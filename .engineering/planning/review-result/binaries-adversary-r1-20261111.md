---
format: aep.planning-md/3
id: review-result:binaries-adversary-r1-20261111
kind: review-result
status: active
title: 'Adversary pass 1: test-binary-per-package'
relations:
- reviews: story:test-binary-per-package
revision: 1
---
# Adversary pass 1: test-binary-per-package

Reviewed commit 30499141064 on impl/test-binary-per-package (base 4265b2ec0b5). Two cases in
adapters/catalog/tests/adversary_one_binary.rs: the re-run names check is green (11 call sites,
every name listed by the binary); the installation-leak case is green alone and red beside
`derived_instance_adversary::` under default threads ("another suite's CONNECTORS_INSTALLATION
reached this one"). 1208 tests listed in 3 executables.

Held: all 118 top-level files and the 38 under tests/local_runtime are reached; inner `#![cfg]`
attributes apply as before; no `crate::` path resolves differently; every `--exact` re-run name is
correct; ignored.rs maps each catalog entry one to one with the same family and prerequisites,
including the 11 deferred journeys; no other process-wide state (ports, cwd, subscribers, umask,
rlimits, signal handlers, waitpid(-1)) is shared.

Not held: derived_instance_adversary sets CONNECTORS_INSTALLATION process-wide and its SAFETY
comment is false now that ~1170 suites share the process; the declared-module guard misses a
tests/<dir>/main.rs and an empty read; docs/local-catalog-provider.md names a test by its old
name; the before/after counts are not recorded in the story.

Findings 1-3 go back to the implementor; finding 4 is recorded by the coordinator.

```findings
[
{"file":"adapters/catalog/tests/derived_instance_adversary.rs","line":57,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the SAFETY comment claims every reader of CONNECTORS_INSTALLATION in the binary holds ENV, but set_var now leaks the value into every suite sharing the main process (adversary_one_binary::no_suite_sees_the_installation_another_suite_sets red beside it) and Config::load readers in local_runtime do not take the lock"},
{"file":"adapters/catalog/tests/main.rs","line":149,"category":"test-gap","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the declared-module guard only checks for absence among tests/*.rs, so a tests/<dir>/main.rs suite that autotests=false now skips, or a cfg-disabled mod line, passes unnoticed"},
{"file":"docs/local-catalog-provider.md","line":375,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"names the ignored fixture by its old libtest name, cli_journey::serve_the_raw_file_fixture, instead of local_runtime::cli_journey::serve_the_raw_file_fixture in target main"},
{"file":".engineering/planning/story/test-binary-per-package.md","line":1,"category":"acceptance","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the acceptance asks for before/after test binary counts and build CPU time from cargo build --tests --timings to be recorded in the story, and none are recorded"}
]
```
