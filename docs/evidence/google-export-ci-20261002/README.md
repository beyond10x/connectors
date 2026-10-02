# Google export test fixture correction, 2026-10-02

PR 79's repository gate failed on candidate
`b0b42f5a3a85dc3621a5a263b717c9d748bb2932`: an export of exactly 4 MiB
returned `Unavailable`. The unchanged boundary test reproduced locally after
26 successful repetitions, failing on repetition 27. This test file was unchanged
by the provider acceptance wave. CI run: [37051606475](https://github.com/beyond10x/connectors/actions/runs/37051606475).

The separate [bounded review](review.md) found no defect in the source or retained
evidence. It executed no tests and explicitly inherits the worker's results.

The HTTPS fixture dropped its TLS stream after `write_all`. In the locked
tokio-rustls 0.26.6 implementation, a successful plaintext write can leave encrypted
output buffered. The fixture now flushes after writing the response. Production
code, dependencies, response limits, deadlines and existing assertions are unchanged.

A one-variable flush-only probe passed 100 boundary repetitions. A new regression
then exercised the actual shared fixture writer through verified rustls client and
server connections over a 64-byte bounded duplex stream. The unflushed helper
failed with `UnexpectedEof`; restoring only its flush made the test pass. This is
real TLS over an in-memory transport, not TCP or live Google. The original
production-child/loopback HTTPS test separately retains exact-limit success and
limit-plus-one `Capacity` / `ProviderCapacity` assertions.

The first regression setup timed out with default TLS 1.3 ticket emission; that
failure is retained separately and is not the deciding response-delivery evidence.
Disabling tickets only in the new regression produced the deciding EOF. Both red
and green deciding runs use that same configuration. The original provider fixture
TLS configuration remains unchanged. Existing ignored write errors allow the
oversized-response client to close early; its capacity result remains asserted.

Final worker checks: 100/100 original boundary repetitions; all six tests in the
affected binary passed; scoped Clippy with warnings denied, package formatting and
diff whitespace checks passed. The broad formatting check reported only unchanged
generated CLI-contract files; the worker did not edit them. Integration gate and
remote CI results are recorded separately. Finite repetitions alone do not prove
absence of every flake; the direct regression establishes this completion defect.

Retained identities:

[Selected source inputs](source-inputs.sha256) identify the corrected test,
changelog and unchanged workspace version/lock inputs.

| Evidence | SHA256 |
| --- | --- |
| CI failure log | `894320c580b62b002e7295e984c9a479d4f03c01fada4c9bb5993d48d6cbcb66` |
| Original local boundary loop | `7fce34011d0bf245ae7018067338a572c10b6e6ff0bd8b1c49750de78ba1c0ec` |
| Original test source | `129eca0ac87f84e75a4c8a8af9cd16ac5d55e535802113fc37b4c64c8ed5081d` |
| Corrected test source | `5f81c2f93c5168a40209f1aefa7bc8006bef6c03b1ddda77d6e7f84fe4608b0a` |
| Worker handoff report | `9bb2030afa8daa0aedad20dee037bf33770f3dd1891fd8e65002c6f238dad87c` |
| Final worker test executable | `36c027503ba3e0915a16afa6cdc0f342e17ce0b1c4d9a7121c6178fe2dd38c2e` |

Private task evidence retains original logs, commands, exits, patches and binary
hashes, including the separate missing-private-TMPDIR setup failure. That setup
failure is not the reported CI symptom. No live Google requests were performed.

## Corrected candidate integration

`cargo run --locked -p connectors-build -- gate --msrv` exited 0 with ESS 0.45.0,
AEP 0.65.0, two Cargo jobs and sccache. The workspace reported **1,208 passed,
0 failed, 65 ignored**. Authored formatting, Clippy, source and generated-output
checks, independent adapter models, library boundaries, Rust 1.88 library checks
and Rust 1.91 workspace/all-targets checks passed. Full gate log SHA256:
`c2b649601465fba90f733edf0d153bc63406945be2eb906723ff56a7c6d8bd50`.

Contract synthesis emitted 498 scenarios (43 authored), retaining 21 refusals.
Metadata conformance executed 289 scenarios successfully but remains
**Inconclusive: coverage undeclared**. AEP validation passed with historical
warnings retained. None of these counts establishes full provider conformance.

Documentation reference generation, reference drift check and the Docusaurus
build/public audit passed; 482 public files had no private path markers. Build log
SHA256: `a75ce9dc973033866f949c273af9585aeaa89720b38181d18ef501b51d8d6973`.
The unchanged website presentation, dependencies and examples retain their prior
successful typecheck, 15 example tests, browser smoke and UI checks. This rerun used
the existing built documentation command followed by Docusaurus build; it did not
rerun unchanged example compilation as a prebuild hook.

Fresh required PR checks and exact tag/release verification remain separate from
these local results. The earlier failed CI run is preserved above.
