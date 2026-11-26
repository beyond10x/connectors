---
format: aep.planning-md/3
id: review-result:wave4-drafts-design-round-4
kind: review-result
status: active
title: Design critic, round 4, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at a2a515f39. No build, no test and no `ess` call was run. Statements marked "inferred"
come from reading code.

- Provider budget that ends in an unanswered TLS handshake: needs-revision, one warning.
- Sixteen ess_model tests: needs-revision, one warning. The approval of round 3 does not hold
  for the text as revised.

## The findings of round 3 on the provider budget

1. The blocker on the two tests of `http_budget.rs`: resolved. The task-local is set only with
   at least 500 ms left; the tests with 200 ms and 150 ms left stay under it, and acceptance 6
   lists the file as unchanged and passing.
2. The read POST: resolved by taking it out of the story. `with_read_posts` is called in
   production by `adapters/grafana/src/local.rs` (30 s) and `adapters/tavily/src/local.rs`
   (160 s) alone; `with_provider_budget` by `adapters/catalog/src/local.rs` alone.

## The provider budget as it stands

No test was found that the change turns red.

- All 8 tests of `http_budget.rs` use a plain loopback listener, whose connect completes at
  once.
- `adapters/catalog/tests/local_runtime/bounded_reads.rs` uses a TLS fixture that is idle
  when the request comes, so the handshake completes at once; where about 900 ms are left the
  cut is marked, and the test accepts it.
- `provider_timeout_adversary.rs` and the unit tests of `http.rs` set no budget.
- Not established: the CLI tests under `apps` that start the real catalog provider were not
  read. Inferred: `with_provider_budget` appears nowhere under `apps`.

The threshold is coherent at its edge: with 500 ms left the connect gets 250 ms, and the
request's own timer ends 250 ms later. The statement on the second connector call of
hyper-util 0.1.21 is right: the retry runs only for a request that came back unsent, inside
the same `send`, so the same bound applies. `provider_error`, `connector_layer`, the two
`tower` entries of `Cargo.lock` and the `libc` precedent are as the draft says.

Finding (warning): the sentence "A budget that ends before the TCP connection opens is cut by
the same bound and is unmarked" names no floor. Item 3 leaves a call with under 500 ms left as
today, and there a hung TCP connect is ended by the request's own timer and is marked. The
sentence takes "with at least 500 ms left at `send`".

Open until a compile: the generic layer and service against reqwest's private connector
types with a `Send + 'static` future; and that a `connector_layer` moves every client of
`ScopedHttp` to the layered connector, where reading found no difference in behaviour.

## The sixteen ess_model tests

The helper today starts `ess generate` with the working directory at the crate's manifest
directory and the relative path `spec/ess`. Every other `ess generate` call that was read
does the same. Whether `ess` takes an absolute `--path` is not shown.

Finding (warning): item 4 gives `schemas_in(parent, sources)` a source directory and says it
"does items 1 and 2", which use `sources` for the key alone. An implementation that keeps the
manifest directory and `spec/ess` hashes the copy and generates from the original. Acceptance
3 cannot see that: its byte is in a comment line, so the output is the same. The item says
that `schemas_in` starts `ess` with the working directory at the parent of `sources` and
`--path` at its file name.

Outside this lane: item 4 and acceptance 6 say `schemas_in` is called "from the two new
tests", and the test of acceptance 1 starts children alone.
