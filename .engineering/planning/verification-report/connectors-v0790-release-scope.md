---
format: aep.planning-md/3
id: verification-report:connectors-v0790-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.79.0
summary: What 0.79.0 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-11-26T22:19:18Z", actor: "human:operator", revision: 2}
---
## Scope of connectors 0.79.0

Cut on 2026-11-26 after pull request 315 merged at 23:18 CET as `5ab5047b3`, on the operator's
standing order to cut a release when merged work a user sees is not in the last release.
`main` holds one merge after `v0.78.3`: pull request 315.

- A new release archive, `connectors-<version>-x86_64-unknown-linux-musl.tar.gz`: the same
  binaries, statically linked, checked in the release workflow for no program interpreter and
  no shared library and for `--version` in a plain `alpine:3` container. It is the reason for a
  minor version: a platform is added.
- story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout:
  a call whose provider budget ends before its connection opened answers `Timeout` without the
  provider's-timeout mark; a connect may use the whole budget.
- The `close_range` and `renameat2` system calls go through `syscall`, so the workspace builds
  for musl; the calls, flags and errors are unchanged.
- Test-step and gate work that changes no installed binary:
  story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run,
  story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs, ten clock tests that
  no longer race a write deadline, and a time bound on the gate's package steps.
- No command, configuration member, answer member or error code is added, removed or changed.

## What a consumer reads

- On Linux without glibc (Alpine and others), the musl archive runs as it is.
- A provider call cut by its budget during the connect or the TLS handshake answers `Timeout`
  with `upstream_answer` false where 0.78.3 answered it true. A caller that retries only an
  unmarked timeout now retries this case: no request reached the provider.

## Known at the release

- The forge's test step is still over its budget of 120 s (467.2 s on pull request 313).
- A connection whose TCP and TLS are up and whose HTTP/2 handshake is pending when the budget
  ends is still marked as the provider's timeout. `upgrade()` does not watch its connect.
- The musl binaries use musl's own allocator; no allocator measurement has run.
- Not changed from 0.78.3: a write refused at commit answers a code without the engine's reason
  (story:a-refused-local-write-names-its-reason, `draft`), and the release archive holds no
  `connectors-sql` (story:released-install-reads-mysql-and-loki-per-environment).
