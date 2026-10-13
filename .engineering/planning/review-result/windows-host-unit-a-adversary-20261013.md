---
format: aep.planning-md/3
id: review-result:windows-host-unit-a-adversary-20261013
kind: review-result
status: active
title: Adversary review of the Windows owner transport and handle passing
relations:
- reviews: story:host-runs-on-windows
revision: 1
---
## Adversary review of the Windows owner transport and handle passing

Reviewed on 2026-10-13: unit A of `story:host-runs-on-windows` (named-pipe owner transport, handle passing, a7e49760c), from source; no case ran on Windows during the review. The fixes are in 80b284abc (contracts) and 5b14a27ca (code); the cases are `#[cfg(windows)]` and first run on `windows-latest` in the wave pull request.

| # | finding | outcome | case |
|---|---|---|---|
| 1 | The owner starts with only `SystemRoot`, so `temp_dir()` falls back to the Windows directory and a sealed credential is refused or lands there. | fixed: scratch lives in the state directory's owner-only `run` directory, and the owner gets `TMP` and `TEMP` pointing at it | `a_credential_is_sealed_in_the_environment_the_owner_runs_with` |
| 2 | The peer check compares the user SID only: a medium owner drops an elevated CLI without a reply, and an elevated CLI admits a lower-integrity owner. | fixed: integrity level and token restriction are compared on both sides; a mismatch is `lifecycle_conflict` | `a_medium_integrity_owner_refuses_an_elevated_client_of_its_user_by_name` |
| 3 | The pipe name derives from public inputs, so another local user can take it first. | fixed: a random salt kept in an owner-only file of the state directory | `the_pipe_name_derives_from_a_random_salt_kept_in_the_state_directory` |
| 4 | A frame naming one handle value twice is taken over twice. | fixed: refused | `a_receive_refuses_one_handle_value_named_twice` |
| 5 | A wrong-count frame leaves the duplicated handles open. | fixed: every handle that arrived is closed | (with 4) |
| 6 | The scratch guard drops before its directory handle, leaving an empty directory. | fixed: the guard owns the handle | none |
| 7 | `clients` is decremented before the request's pipe instance closes. | fixed: the stream closes first | none |
