---
format: aep.planning-md/3
id: review-result:adversary-cli-oauth-acquisition-pass-2
kind: review-result
status: active
title: Adversary pass 2 on OAuth loopback acquisition
relations:
- reviews: story:cli-oauth-loopback-acquisition
revision: 1
---
```
unit: story:cli-oauth-loopback-acquisition at 839cb135c (impl/cli-oauth-loopback-acquisition), working tree plus 2 adversary test paths
verdict: NEEDS-CHANGE
cases: executed 270→279, red 2 (plus the known registry flake, and 1 ignored Chrome case red)
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (~/.cache/w0930aq/adv2)
needs-coordinator: none
```

The rewritten listener fixes both pass-1 listener findings. Pass 1's two cases are now green, and headless Chrome was accepted 12 times in 12 runs (pass 1: red 7 of 15). One new blocking-class defect: a redirect head over 8 KiB ends the flow, and real Chrome sends one once it holds enough cookies for 127.0.0.1.

**1. Diff stat**
```
 crates/connectors-host/src/local/mod.rs | 2 ++
 ?? crates/connectors-host/src/local/oauth_adversary_pass2_tests.rs
```
The `mod.rs` lines only wire in the new file as `#[cfg(test)] mod oauth_adversary_pass2_tests;`, as the brief allowed. There are no implementation edits.

**2. Cases added** (all in `oauth_adversary_pass2_tests.rs`; each drives the public `oauth::entry` in its own process)

| Case | Asserts | Now |
|---|---|---|
| `:460` loopback cookies | a 8647-byte redirect head gets 200 | **red** |
| `:488` (ignored, Chrome) | Chrome holding six 1.4 KB cookies, set by a server on another 127.0.0.1 port, gets "Consent received." | **red** |
| `:325` 64 idle connections | the redirect is answered while 64 idle connections are pending | **red** |
| `:382` delay measurement | one-shot exhaustion: the redirect is eventually accepted | green |
| `:261`, `:293` connection flood | flood from 16 threads: the deadline still fires, and the drain thread's join still returns (also probed with 64 threads) | green |
| `:394` listener closed | nothing accepts on the port after `entry` returns. This is the assertion the unit removed from `second_request_refused` | green |
| `:414` idle limit | a head completed after 2.3 s gets no answer; one completed at 1.8 s gets 200 | green |
| `:442` stderr | stderr is exactly one `connectors: consent-url …` line, with no client secret and no `code_verifier` | green |

Red output, each case run alone before the suite:
```
with 64 idle connections pending the redirect was not answered (write: Ok(())); answer: ""; the same redirect sent again after they closed: None
a 8647 byte redirect head was not accepted; answer: Some("HTTP/1.1 400 Bad Request"); flow: Some((38.711181ms, "ProtectedEntryUnavailable"))
Chrome holding loopback cookies was not accepted; page: "...Consent was not completed. Return to the terminal...."; flow: Some((690.575787ms, "ProtectedEntryUnavailable"))
adversary-measure: held once (Some(1.727828847s), 17); re-opened (None, 79)
```

**3. Suite runs** (after the cases existed)
- `cargo test --locked -p connectors-host --lib`: `FAILED. 276 passed; 3 failed; 24 ignored`, EXIT=101. The failures are my two red cases and `registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once`, which the store already tracks as `story:grown-store-replay-test-flake`.
- The same command with `-- --skip oauth_adversary_pass2_tests`: `FAILED. 269 passed; 1 failed; 23 ignored; 10 filtered out`. That run gives the before count of 270.
- `cargo fmt --package connectors-host -- --check` exits 0. `cargo clippy --locked -p connectors-host --lib --tests -- -D warnings` is clean.
- Pass 1's `chrome_following_the_redirect_is_accepted`: 12 passed, 0 failed.

**4. Findings** (tree 839cb135c plus my test files)

| # | file:line | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|---|
| N1 | `oauth.rs:41` with `:306` and `:407` | A head reaching 8192 bytes counts as complete. `redirect` then refuses it for not ending in CRLFCRLF, and the flow ends `protected_entry_unavailable`. Cases `:460` and `:488` are red. | Real Chrome 150 sends every cookie it holds for 127.0.0.1, whatever the port (RFC 6265 §8.5). So a local dev server that set about 7.5 KB of cookies is enough. I did not measure how common that is. The limit was already there at f01fdde18; pass 1 listed it as unbroken. Suggested fix, not applied: read to the end of the head with a larger cap, and keep only the request line. | NEEDS-CHANGE / introduced, warning |
| N2 | `oauth.rs:260` | Once 64 connections are pending, the redirect is closed on accept with no answer. Accept runs before the read phase, so a slot freed by a client closing is not reused on the same poll. Measured: a one-shot hold turned the redirect away for about 2 s (17 retries). An attacker re-opening connections turned it away for the whole 8 s window (79 retries), so effectively until the deadline. The flow itself survives, but the browser does not retry. The guide (`docs/catalog-google-oauth.md:69-71`) does not mention the 64 limit. | Only a hostile local process. Any local process can already end the flow by sending `GET /` first, which is the designed "first request decides" rule. No benign client comes close: Chrome opens 1 to 3 connections. | INFEASIBLE / introduced, note |

**5. Attacked and could not break**
- Connection flood from 16 and 64 threads: the accept loop still reaches WouldBlock, the deadline fires on time, and the drain thread's join returns.
- Slow-loris: every connection is nonblocking with a 2 s limit counted from accept. Answers are under 200 bytes, so the write never blocks.
- The 2 s idle limit: the code reads before checking the time, so a head that completes on the poll at the limit is accepted.
- A right-state request on `/` arriving while the drain thread starts: pending connections move into `Drain` with the listener, so there is no gap. A duplicate gets 400 and its code is never exchanged.
- Drain shutdown: no drain thread exists on the timeout, cancel or refusal paths. On success it is dropped and joined on every return, including an early `cancelled()?` or a failed exchange. After return, reconnecting is refused.
- `connectors: consent-url <url>`: one line, and URL serialisation cannot contain a newline. It carries `state`, `client_id` and `code_challenge`, but not the secret or the verifier.
- Docs: the `authorize_url` `/o/oauth2/auth` matches Google's `auth_uri` and the exact-equality check at `oauth.rs:161`, consistent across both guides. The `token_ca_file` limitation is now documented, which closes pass-1 F3 as a doc statement.

**6. Paths written outside the worktree**
- `~/.cache/w0930aq/adv2`, used as TMPDIR (1.1 MB). It holds the 12 Chrome run logs, `suite.log`, `suite-before.log` and Chrome's temp directories.

**7. Findings block**
```findings
- file: crates/connectors-host/src/local/oauth.rs
  line: 41
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A redirect head over the 8 KiB REQUEST_LIMIT is cut, taken as complete and refused, so real Chrome carrying about 7.5 KB of 127.0.0.1 cookies from another local port ends the flow with protected_entry_unavailable."
- file: crates/connectors-host/src/local/oauth.rs
  line: 260
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "A local process holding 64 idle connections gets the browser's redirect closed on accept with no answer, for about 2 s one-shot or until the deadline when it re-opens them, which only a hostile local process does."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
