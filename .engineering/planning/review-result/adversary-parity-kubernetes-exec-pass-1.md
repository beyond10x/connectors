---
format: aep.planning-md/3
id: review-result:adversary-parity-kubernetes-exec-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: parity-kubernetes-exec'
relations:
- reviews: story:parity-kubernetes-exec
revision: 1
---
unit: story:parity-kubernetes-exec, ecbcfdfdb on wave/20261010e
verdict: 2 warnings, fixed in 0ff5668be
cases: executed 521→535, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none

## Findings

1. warning, contract drift, `adapters/kubernetes/src/exec.rs` (`Retained::text`): an output cut after an earlier invalid byte replaced the incomplete trailing character with U+FFFD instead of withholding it (mutations contract section 4.2). Case `exec_cut_after_an_invalid_byte_still_withholds_the_incomplete_trailing_scalar`. Fixed in 0ff5668be.
2. warning, contract drift, `settle`: the host stream error was discarded, so a deadline or byte bound ended as unknown with code unavailable instead of timeout or capacity. Case `exec_host_deadline_and_byte_bound_keep_their_codes_in_the_unknown_outcome`. Fixed in 0ff5668be: the code is kept, the outcome stays unknown and is never resent.

## Judgement notes, no test

- The contract requires each argument to be 1 to 4096 bytes, so an empty argument is refused; that rule stands.

## Attacked without a finding

- Arguments with `;`, spaces, newlines, `&command=`, `#`, `%2F` reach the API as separate unchanged `command` terms; `tty` and `stdin` cannot be overridden.
- Pod and container names with `/`, `?`, `#` or encoded `..` stay one segment.
- The deadline ends a silent server and a slow drip; byte bounds hold across fragmented and multiple messages.
- Malformed 101 answers end as lost with upstream_protocol; a refused connect or a non-token subprotocol is not sent.
- 3xx, 409, 429 and 5xx answers are unknown; a split status settles; the consumable bound beats a later status.
