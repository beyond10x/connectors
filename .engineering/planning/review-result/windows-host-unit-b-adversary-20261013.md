---
format: aep.planning-md/3
id: review-result:windows-host-unit-b-adversary-20261013
kind: review-result
status: active
title: Adversary review of Windows capture, jobs and consumer launch
relations:
- reviews: story:host-runs-on-windows
revision: 1
---
## Adversary review of Windows capture, jobs and consumer launch

Reviewed on 2026-10-13: unit B of `story:host-runs-on-windows` (capture, job objects, consumer launch and supervision, 6fe154a42..1987e4f9e), from source; no case ran on Windows during the review. The fixes are in 80b284abc (contracts) and 5b14a27ca (code); the cases are in `runtime/launch/consumer_tests/adversary_b.rs`, `#[cfg(windows)]`, and first run on `windows-latest` in the wave pull request.

| # | finding | outcome | case |
|---|---|---|---|
| 1 | An owner a consumer starts is created inside the consumer's kill-on-close job and ends with it. | fixed: a consumer's job permits breakaway on request | `adversary_b_a_consumer_can_start_a_process_that_leaves_its_job` |
| 2 | Processes a consumer starts end when the consumer exits normally, unlike Linux. | kept and stated: the consumer-launch contract and the changelog name it | `adversary_b_a_grandchild_ends_with_a_consumer_that_exited` |
| 3 | Image copies are left in the temporary directory on early returns, with no cleanup. | fixed: the removal guard exists before any fallible step; the owner sweeps unheld copies at start | `adversary_b_a_nul_argument_leaves_no_copy_behind`, `adversary_b_a_failed_creation_leaves_no_copy_behind`, `a_sweep_removes_left_copies_that_no_process_holds` |
| 4 | Children inherit the launcher's current directory, so DLLs are searched there. | fixed: the copy's owner-only directory is the current directory; executables are self-contained | `adversary_b_the_copy_directory_cannot_be_renamed_while_the_copy_is_held` |
| 5 | A consumer gets no `SystemRoot`, so it cannot open a socket. | fixed | `adversary_b_a_consumer_with_an_empty_environment_can_open_a_socket` |
| 6 | `pass_env` matches names case-sensitively. | fixed: any letter case | `adversary_b_pass_env_matches_a_name_in_any_letter_case` |
| 7 | An inheritable handle exists during one creation. | kept as a limitation: the host and the CLI make no concurrent inheriting creation | none |
| 8 | A suspended child is outside any job until it is assigned. | fixed: `PROC_THREAD_ATTRIBUTE_JOB_LIST`; leftover `-wal`/`-shm` files of a crashed elevated session kept as a limitation | existing job cases |
