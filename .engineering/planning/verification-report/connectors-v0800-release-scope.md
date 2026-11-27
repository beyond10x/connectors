---
format: aep.planning-md/3
id: verification-report:connectors-v0800-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.80.0
summary: What 0.80.0 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-27T08:25:16Z", actor: "human:operator", revision: 3}
---
## Scope of connectors 0.80.0

Cut on 2026-11-27 after pull request 317 merged as `f58b2520f`, on the operator's standing order
to cut a release when merged work a user sees is not in the last release. `main` holds one merge
after `v0.79.0`: pull request 317.

- story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice:
  `connectors invoke --retry-not-attempted <n>` (1 to 10, needs `--idempotency-key`). The owner
  tries a keyed write again only while the provider answered the preflight with its own timeout
  or capacity answer and nothing was sent, inside the write's 20 s deadline, with the last try's
  duration plus 3 s left. It is the reason for a minor version: a command gains an option.
- The answer member `mutation.tries` is added; it is absent without the option.
- A write classified `unknown` exits 3 under the option; without it the exit code is 1 as before.
- The write protocol stays `connectors-owner/2`; an owner of another build is refused with
  `owner_build_mismatch` before any write request.

## What a consumer reads

- Without the option, every command answers as in 0.79.0.
- With the option, `mutation.tries` counts the tries made; 0 on a same-key replay.
- A caller that maps exit codes reads 3 as "the write's outcome is unknown, do not resend".

## Known at the release

- Each try costs about 450 ms in the existing write path
  (story:a-write-try-costs-less-than-450-ms-in-the-owner, `draft`).
- Not covered by the option: not-attempted causes found on the host side
  (story:invoke-retries-host-side-not-attempted-causes, `draft`), and settling an unknown write
  by a declared read-back (story:an-unknown-write-is-settled-by-a-declared-read-back, `draft`).
- The forge's test step is still over its budget of 120 s (467.2 s on pull request 313).
- Not changed from 0.79.0: the HTTP/2 handshake case of the provider budget, the musl
  allocator measurement, and the release archive holds no `connectors-sql`.
