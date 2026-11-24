---
format: aep.planning-md/3
id: review-result:local-upload-adversary-round-1
kind: review-result
status: active
title: Adversary review, round 1, story:a-file-upload-through-the-local-cli-is-sent-with-its-ports
relations:
- reviews: story:a-file-upload-through-the-local-cli-is-sent-with-its-ports
revision: 1
---
verdict: holds with findings

The fix does what the story asks: no upload outcome is classified `applied` or `refused` wrongly, and nothing is sent twice. One sentence of the guide was false on the local path. No blocker.

- unit: story:a-file-upload-through-the-local-cli-is-sent-with-its-ports, `git diff 25995f564..95fb4f974 -- adapters/catalog docs`, attacked in the working tree at `16c1c018d`
- cases: 83 before, 89 after, red 2 (the reviewer's own); origin: introduced 3, pre-existing 2
- suite: `cargo nextest run --locked -p connectors-catalog-provider --test main -E 'test(/^slack/) | test(/^upload_child_adversary::/) | test(/^created_credential::/) | test(/^declared::/)' --no-fail-fast`: `89 tests run: 87 passed, 2 failed, 1133 skipped`; the same filter without the reviewer's module: `83 tests run: 83 passed`
- the four green cases were not run against a mutated copy; that the suite missed those lines rests on grep
- needs-coordinator: finding 1, whether the guide's sentence or the commit's behaviour moves

Attacked and held:

- a deadline during the share is `Unknown` and sent once; the owner settles what the child reports and never sends again; the client has no retry and no redirect
- custody and upload in one selection are refused at load three ways (`check_binary`, `check_custody`, `check_upload`); checked by reading
- a `401` on the slot or the share evicts a cached token, a `403` does not; checked by reading
- the byte port is looked up by exact canonical origin, and the credential is attached only when `credential` is true
- wrong bytes fail on the SHA-256 assertion; a reverted fix fails (`0f0dd58ae` records it)
- the guide's limits match the code; the 196,243 and 196,475 byte figures were not measured again by the reviewer

```findings
- file: docs/catalog-slack.md
  line: 283
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a commit deadline ending during the slot or byte request is outcome_unknown through the provider child, where the guide and the engine's doc comment say refused, never outcome_unknown
- file: adapters/catalog/src/lib.rs
  line: 3074
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a 401 from an upload host that was sent no credential is answered invalid_credential and would evict a cached OAuth token; constructed, no shipped upload selection has a token cache
- file: adapters/catalog/src/lib.rs
  line: 2879
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: acceptance 3's refusal of a prepared upload sent without its ports had no test and the fix leaves the guard unreached
- file: adapters/catalog/src/local.rs
  line: 3076
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: no test asserted that an upload host admitted without credential receives no bearer header, on the child or the served path
- file: adapters/catalog/tests/slack.rs
  line: 3696
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the new test checks the slot's filename and length by substring, so a longer value with the same prefix passes
```
