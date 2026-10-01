---
format: aep.planning-md/3
id: review-result:adversary-cli-oauth-acquisition-pass-1
kind: review-result
status: active
title: Adversary pass 1 on OAuth loopback acquisition
relations:
- reviews: story:cli-oauth-loopback-acquisition
revision: 1
---
```
unit: story:cli-oauth-loopback-acquisition at f01fdde18 (impl/cli-oauth-loopback-acquisition), working tree plus 2 adversary test paths
verdict: NEEDS-CHANGE
cases: executed 265→268, red 2 (plus 1 known flake that isn't mine)
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (~/.cache/w0930aq/adv1)
needs-coordinator: none
```

**1. Diff stat**

```
 crates/connectors-host/src/local/mod.rs | 2 ++
 ?? crates/connectors-host/src/local/oauth_adversary_tests.rs
```

`mod.rs` is not a test file. The two lines I added there only wire in my new file, `#[cfg(test)] mod oauth_adversary_tests;`, as the brief allowed. There are no implementation edits.

**2. Cases added** (in `crates/connectors-host/src/local/oauth_adversary_tests.rs`)

Each case runs the public `oauth::entry` in its own process, which is the function the CLI calls at `session.rs:195`. It reads the consent address from stderr and plays the browser. The listener's answer is written before the code exchange, so it shows whether the redirect was accepted.

| Case | What it asserts | Now |
|---|---|---|
| `:173 an_idle_connection_first_does_not_hide_the_redirect` | An idle connection is accepted first, then a valid redirect arrives. The redirect gets a 200 within 5 s. | red |
| `:193 a_queued_connection_without_a_request_is_not_a_second_request` | A connection that is open but has sent no bytes when the redirect's head completes does not cause a refusal. | red |
| `:216 chrome_following_the_redirect_is_accepted` (ignored, needs Chrome) | Headless Chrome 150.0.7871.46 navigating to the redirect gets "Consent received." | red 7 of 15 runs |
| `:248 chrome_opens_one_connection_per_navigation` (ignored, needs Chrome) | Chrome opens exactly one connection per navigation. | red 3 of 3 runs |
| `:74 adversary_flow_fixture` | Re-entry point for the cases above; does nothing unless its env var is set. | green |

The two red cases, each run alone before the suite:
```
a queued connection that carried no request refused the consent redirect; answer: "HTTP/1.1 400 Bad Request\r\n...Consent was not completed. Return to the terminal."
the consent redirect was not accepted within 5.167736799s of a 10000 ms flow while an idle connection was held open; answer: ""
test result: FAILED. 1 passed; 2 failed; 1 ignored; ... finished in 6.10s
```

The Chrome connection probe, verbatim (accepted at, bytes received, per connection):
```
[(545.642222ms, 0), (550.726139ms, 0), (555.80392ms, 0), (806.991172ms, 675)]
[(526.236927ms, 0), (531.32176ms, 0), (845.042413ms, 675)]
[(1.413461642s, 675), (1.433906941s, 0)]
```

**3. Suite runs** (after the cases existed)

- `cargo test --locked -p connectors-host --lib`: `FAILED. 265 passed; 3 failed; 23 ignored`, EXIT=101. The three failures are my two cases and `local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` (`registry/tests.rs:1639`), which the store already tracks as `story:grown-store-replay-test-flake`.
- The same command with `-- --skip oauth_adversary_tests`: `FAILED. 264 passed; 1 failed; 21 ignored; 5 filtered out`. That run gives the before count of 265.
- `CONNECTORS_TEST_CLI=<tree>/target/debug/connectors cargo test --locked -p connectors-catalog-provider --test local_runtime -- --ignored cli_journey`: `ok. 7 passed`. This includes `oauth_connect_journey`, `oauth_repair_journey` and the two non-Google file-connect journeys.

**4. Findings** (tree f01fdde18 plus my test files)

| # | file:line | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|---|
| F1 | `crates/connectors-host/src/local/oauth.rs:246` | After reading the first head, any connection waiting in the queue counts as a "second request" and the flow is refused. That includes a connection that has sent no bytes. Case `:193` is red. | Chrome. It opens 1 to 3 extra connections that send nothing, and a real Chrome navigation was refused in 7 of 15 runs. Headless only; I did not measure desktop Chrome after a real Google redirect. The unit's own `second_request_refused` (`oauth.rs:808`) asserts this refusal, and the guide (`docs/catalog-google-oauth.md:65-66`) describes it as "a second request". | NEEDS-CHANGE / introduced, blocker |
| F2 | `oauth.rs:243` (read loop at `:260-290`) | Connections are handled one at a time, and a connection gets no read timeout of its own. One idle connection holds the listener until the flow's deadline, and a real redirect behind it gets no answer. Case `:173` is red. | Any local process can do this, which is no worse than the designed "first request decides" rule. In the Chrome probe, idle connections were accepted before the one carrying the request (2 of 3 runs), but none of the 15 Chrome flow runs hung. Nothing benign was shown to reach it. | INFEASIBLE / introduced, note |
| F3 | `oauth.rs:62` with `oauth.rs:388-399` | The CLI's code exchange uses the platform trust roots. `runtime::Acquisition` (`runtime.rs:255-259`) has no way to carry the profile's `token_ca_file`, which the documentation describes (`docs/local-catalog-provider.md:159`) and the provider uses (`adapters/catalog/src/local.rs:621-627`). Both OAuth journeys pass only because the debug hook replaces the trust roots (`cli_journey.rs:165`). A release build would fail the exchange after consent. | A profile with `token_ca_file`, which is a documented field; the journey fixture uses one. Google itself needs no custom trust root. I did not run a release build. | NEEDS-CHANGE / introduced, warning |
| F4 | `oauth.rs:366-368` | With no `/dev/tty`, the consent address (including `state`, `client_id` and `code_challenge`) goes to stderr. With `--output json` it sits in front of the JSON error object (`docs/cli-migration-v1-to-v2.md:71`). The journeys never test this path because the hook replaces `present`. | Any CLI run without a terminal, such as an agent or CI. `state` is only useful before the listener closes. | CONFIRMED / introduced, note |

The fix I would suggest for F1 and F2, not applied: treat only a connection that sends a request as a request. Give each connection a short read deadline of its own, and do not refuse because another connection is waiting in the queue.

**5. Attacked and could not break**
- `state`: a 32-byte CSPRNG value, compared in constant time (`:323-325`). A missing, empty or duplicated `state`, or an `error=` parameter, is refused. It is single-use because the listener closes after one answer.
- PKCE verifier: 43 characters from ring `SystemRandom`, base64url (unreserved characters only), and the RFC 7636 test vector passes.
- Bind address is `Ipv4Addr::LOCALHOST` only (`:161`). Oversized requests are capped at 8 KiB. Malformed request lines and non-UTF-8 input are refused. The answer is a fixed body that never echoes the code.
- The redirect URI is the same string in the authorize URL and the exchange, and the path must be exactly `/`.
- The `auth_uri`/`token_uri` check is exact string equality (`:152-153`), so a trailing slash or a case change is refused, not accepted. The URLs actually used come from the profile.
- Timeout and cancellation both return before `self.captured` is set (`session.rs:203-211`). Cancellation is polled in the accept loop, in the read loop, and before the exchange.
- Secrets: no Debug on `Installed`, `Exchange` or `Secret`. Errors carry only a code. No argv, env, `unwrap` or `panic` in non-test code.
- `CONNECTORS_TEST_OAUTH_FOLLOW` is compiled only under `cfg(debug_assertions)`, and `[profile.release]` does not turn that on.
- A non-Google `--credential-file` goes through unchanged, and `repair` re-runs the flow (journeys green).
- The guide's claims about 240 s, Ctrl-C, `identity_mismatch` (`registry/lifecycle.rs:163`) and `repair_connection` match the code. The exception is "second request" (F1).

**6. Paths written outside the worktree**
- `~/.cache/w0930aq/adv1`, used as `TMPDIR`. It holds Chrome's leftover `com.google.Chrome.*` temp directories. No Chrome process of mine is still running.

**7. Findings block**

```findings
- file: crates/connectors-host/src/local/oauth.rs
  line: 246
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A queued connection that has sent no request counts as a second request, so headless Chrome 150's own consent redirect was refused in 7 of 15 runs."
- file: crates/connectors-host/src/local/oauth.rs
  line: 243
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "One idle connection accepted first holds the one-at-a-time listener until the flow deadline and the real redirect behind it is never answered, but no benign client was shown to trigger it."
- file: crates/connectors-host/src/local/oauth.rs
  line: 62
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The CLI code exchange ignores the profile's documented token_ca_file because Acquisition cannot carry it, and both OAuth journeys pass only because the debug hook replaces the trust roots."
- file: crates/connectors-host/src/local/oauth.rs
  line: 367
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Without a tty the consent address with state and client_id is written to stderr, ahead of the JSON error object in --output json mode, a path no journey exercises."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
