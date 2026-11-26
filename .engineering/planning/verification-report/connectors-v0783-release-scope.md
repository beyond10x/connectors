---
format: aep.planning-md/3
id: verification-report:connectors-v0783-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.78.3
summary: What 0.78.3 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-11-26T06:37:31Z", actor: "human:operator", revision: 2}
---
## Scope of connectors 0.78.3

Cut on 2026-11-26 after pull request 313 merged at 07:35 CET as `72ae2ff07`, on the
operator's standing order to cut a release when merged work a user sees is not in the last
release. `main` holds 16 merges after `v0.78.2` (pull requests 296 to 313).

- story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
  (pull request 313): the metadata store builds the registry once per process. The one change
  an installed binary carries.
- story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
  (pull request 308): one helper for the test overrides; a release build reads none, shown by
  `release_build_ignores_a_debug_override` in the lane `slow`.
- story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts (pull request 313): the
  HTTP client's two timeouts through that helper; the SQL adapter's query deadline without its
  panic, reached only by the adapter's tests.
- Change 3 of story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step (pull
  request 296): the gate's `cargo run` calls name their program with `--bin`.
- Test-step work under story:the-gates-test-step-ends-within-120-s-on-the-forge: pull requests
  300, 302, 303, 304, 306, 307, 309, 311 and the MySQL fixture and citation stories of 313;
  planning pull requests 299, 301, 305, 310 and 312.
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed.

## What a consumer reads

- A local CLI or owner process opens and persists the metadata store without parsing the
  definition bundle on each call. Records, errors and their order are as in 0.78.2.
- Nothing else changes for an installed binary.

## Known at the release

- The forge's test step is still over its budget of 120 s; its stories stay open.
- Three story changes are committed on branches that are not pushed and are not in this
  release: the ess_model schemas generated once per run, the two MCP tests under a short
  request budget, and the provider budget of an unanswered TLS handshake. An adversary read of
  the last found that it refuses a call whose connection opens in the last 250 ms of its
  budget; it is not merged before that is decided.
- On pull request 313 two checks passed on their second attempt: the macOS host job, where two
  `connectors-host` tests with a 2 s write deadline timed out (a sibling test failed the same
  way on 2026-11-24), and the repository gate, whose `apt-get update` before the Windows lint
  hung 90 minutes on the runner's package mirror. Both go into the next wave.
- Not changed from 0.78.2: a write refused at commit answers a code without the engine's reason
  (story:a-refused-local-write-names-its-reason, `draft`), and the release archive holds no
  `connectors-sql` (story:released-install-reads-mysql-and-loki-per-environment).
