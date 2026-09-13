# Dedicated GitLab sandbox acceptance — 2026-09-13

First runtime evidence for the GitLab local binding collected against a dedicated
live GitLab, not fixtures. It clears the live-provider evidence that
`credential-blocker:gitlab-runtime-sandbox` withheld.

## Sandbox

| item | identity |
|---|---|
| GitLab | 19.3.2, revision `34042bf7d00` |
| image | `gitlab/gitlab-ce@sha256:05453dd1d9aba27c2c487613141596868409b4d03247647f7d66cb0b36f321b8` |
| runner image | `gitlab/gitlab-runner@sha256:23b81db80313e536171bd0d6e8eb63ddead60eeda0ec6664593cd2415a160c06` |
| API base | `https://localhost:8929/api/v4` |
| project | `root/connectors-sandbox`, id 1 |
| default branch head | `2ec9f02aa53b6345d4abd91abb7b8b72d00987fb` |
| merge request | iid 1, source `feature/sandbox-change`, head `fe85abe636373338976efd986e4e5833ac22689d` |
| pipelines | 1 on the main head, 2 on the MR head; both `success` |
| jobs | 1 and 2, stage `test`, name `say-hello`, both `success` |
| issue | iid 1 |

The runner uses the `shell` executor and is registered to the project with a
project runner authentication token. Registration tokens are not used; GitLab 19
removed them.

## Credential identity

The operations under test ran as `sandbox-dev`, user id 2, a **non-administrator**
with project role Developer and a personal access token scoped `read_api` only.

The `root` administrator token exists in the sandbox and was used exclusively for
sandbox administration — creating the project, the branch, the merge request, the
issue, the runner and the delegated user. No operation under test used it, except
the deliberate identity-mismatch refusal below.

Both credential documents are owner-only `0600` files outside the repository. No
secret value appears in this record.

## Built artifacts

| artifact | sha256 |
|---|---|
| `target/release/connectors` | `2ce6b1afbd36b8d6a031743f48406ff07504d9ec1cf8f6f904cc590888253c0f` |
| `target/release/connectors-gitlab` | `3525732e777b93a0557a13a7fbb76a463a6eae9fcb6c409159a605b294569082` |
| sandbox CA (`ca_file`) | `5579b0e895350c282164654593e91372146bce7ba7dd32065c47f280fc0c9385` |

Repository head `882904a66b2d85452ac7374ebf3bc2afbd82f0ce`, `rustc 1.98.1 (48a229cea 2026-09-01)`.
Adapter `configuration_revision` `b9f15c7a042c88971bba31bcdea74f6fd1b279953b25a84894660757d26ae247`.

## The TLS defect this sandbox found

The first sandbox certificate was a single self-signed certificate carrying
`basicConstraints critical, CA:TRUE`, used as the server's end-entity certificate.
`curl` and OpenSSL accept that; the adapter's rustls client rejects it, because an
end-entity certificate must not be a CA.

The adapter therefore opened **no TCP connection at all**. The refusal surfaced as
`outcome_unknown` at stage `publication`, because `crates/connectors-host/src/local/owner/transport.rs`
rewrites `Unavailable` and `Timeout` into `OutcomeUnknown`, and because both the owner
(`owner/transport.rs`) and the adapter child (`runtime/process.rs`) are spawned with
`Stdio::null()` on every descriptor. `adapters/gitlab/src/main.rs` writes the named
failure to a stderr that goes nowhere.

Nothing in the product is wrong here — the refusal is correct and the silence is
deliberate. It is recorded because the cost of diagnosing it was the whole first half
of the session, and the next person deserves the shortcut.

Resolution: a two-certificate chain. `ca.crt` is `CA:TRUE` with `keyCertSign`; it signs
`server.crt` with `CA:FALSE`, `extendedKeyUsage=serverAuth` and
`subjectAltName=DNS:localhost,IP:127.0.0.1`. GitLab serves `fullchain.crt`; the adapter's
`ca_file` names `ca.crt`.

## Persistent journey

`story:persistent-gitlab-journey`. From a fresh private configuration and state directory:

1. `setup init` → `created`
2. `setup check` → all five prerequisites `ready`, including `persistent_custody_qualification`
3. `connections connect --adapter forge --profile gitlab.pat --credential-file <owner-only file>`
   → `ok`, connection `conn_c8fad8ced37b44a8a6ec048905cce3fe_7e4ce7d4ce6b4854bd2ba439311e9f35`,
   external identity `gitlab.user` subject `2`
4. `operations invoke … project.get` → `ok`
5. `adapters stop` with the exact `--expected-revision`, `--host-incarnation` and
   `--child-incarnation` from `adapters status` → `suppressed`, adapter process gone
6. `operations invoke … project.get` again → `ok`, adapter resumed
7. owner process killed
8. `operations invoke … project.get` again → `ok`

Steps 6 and 8 reused the retained credential version. No protected entry was repeated.

Custody is a private Secret Service: a task-owned `dbus-daemon --session` and
`gnome-keyring-daemon --foreground --components=secrets`, selected through the
configuration's `secret_service_socket`. The daemon binary's SHA-256 matches
`DAEMON_SHA256` in `crates/connectors-host/src/local/keyring/custody/gnome.rs` exactly.
The desktop keyring cannot satisfy this binding on this host: its default collection
alias resolves to `Standard_2dSchl_c3_bcsselbund`, and that file pins
`/org/freedesktop/secrets/collection/login`.

## Operations

All eleven permitted operations returned `ok: true` with provenance.

| operation | observed |
|---|---|
| `project.get` | `root/connectors-sandbox` |
| `issues.list` | 1 item, `complete: true` |
| `file.get` | `README.md` with `blob_id`, `commit_id`, `content_sha256` |
| `pipelines.list` | pipeline 1 for the exact main SHA |
| `pipeline.get` | pipeline 1, `success` |
| `pipeline.jobs` | job 1 `say-hello` `success`, `complete: true` |
| `job.get` | job 1, pipeline 1, exact SHA verified |
| `job.trace` | 2448 bytes, `complete: true` |
| `merge_request.get` | iid 1 |
| `merge_requests.list` | bounded window, `complete: true` |
| `merge_request.validate` | see the matrix below |

Raw results are in `invocations.jsonl`.

### Pipeline observed to a terminal status

`story:gitlab-ci-runtime` requires the selected pipeline to be watched from
pending/running to terminal on one exact SHA, and the failed job's trace read with
explicit bounded completeness.

A second commit to `main`, `b3324ac24b3cfdf361b7da0e31e5bb56508be460`, added a job
that exits 7. Pipeline 4 was selected by `pipelines.list` on that exact SHA, then
polled with `pipeline.get` pinned to the same SHA:

| time (UTC) | `pipeline.get` status |
|---|---|
| 07:18:23 | `pending` |
| 07:18:54 | `running` |
| 07:18:57 | `failed` |

`pipeline.jobs` returned job 5 `always-fails` `failed` and job 4 `say-hello` `success`.
`job.get` on job 5 confirmed `allow_failure: false` against pipeline 4 and the exact SHA.

`job.trace` on job 5 with `max_bytes` 4096 returned 2346 bytes, `complete: true`,
containing `ERROR: Job failed: exit status 7`. The same request with `max_bytes` 64
returned 64 bytes and `complete: false`.

### Continuations

`merge_requests.list` over an inclusive window with `limit` 2 returned iids `[1, 2]`,
`complete: false` and a cursor. Following that cursor returned iid `[3]`,
`complete: true`, `next_cursor: null`.

Reusing that cursor after changing `state` from `all` to `opened` was refused:
`service_failure` at stage `dispatch` with `service_code: stale_cursor`. A cursor does
not carry across a changed selection partition.

### Validation blockers

`merge_request.validate` against MR iid 1, whose head is `fe85abe6…` with head pipeline 2:

| expected sha | expected pipeline | checks_passed | blockers |
|---|---|---|---|
| `fe85abe6…` | 2 | true | none |
| `fe85abe6…` | 1 | false | `pipeline_changed` |
| `000000…` | 2 | false | `head_changed`, `pipeline_head_mismatch` |
| `000000…` | 1 | false | `head_changed`, `pipeline_changed`, `pipeline_head_mismatch` |

`merge_performed` was `false` in every case.

The rows above pin a SHA the MR never had. `story:gitlab-mr-validation` asks for the
MR head to actually move, so a further commit was pushed to `feature/sandbox-change`,
taking its head from `fe85abe6…` to `f06a8841c820b7c7ce88c1ed92a0df62f2a00b16`. The
**same pinned request** — `sha` `fe85abe6…`, `pipeline_id` 2 — then returned
`checks_passed: false` with `head_changed`, `merge_checks_pending`, `pipeline_changed`,
`pipeline_head_mismatch`, `pipeline_not_successful`, and `merge_performed: false`.

Between the two calls the credential evidence lapsed and the read refused with
`not_granted` at stage `admission`, `next_action: repair_connection`. An explicit
`connections revalidate` on the exact connection revision restored it without any
protected entry. Evidence lifetime is 60 seconds.

## Refusals

| case | result |
|---|---|
| project outside `allowed_projects` | `forbidden` at stage `admission`, before provider work |
| `sha` that is not a full 40/64-char hex | `invalid_input` at stage `arguments` |
| `updated_after` later than `updated_before` | `invalid_input` at stage `arguments` |
| `limit` 101, above the maximum of 100 | `invalid_input` at stage `arguments` |
| `connections repair` with a different GitLab identity | `identity_mismatch` at stage `admission` |

After the refused repair, `connections describe` still reported `state: ready` with
identity subject `2`. The failed repair preserved the valid existing credential.

## Guarded merge

`story:guarded-gitlab-merge`. Run through a second private configuration:
`format = "connectors-local/2"`, `private_protocol = "connectors-private/2"`,
`merge_request.merge` added to the adapter's permitted operations, a
`[approval_clock]` table naming roughtime.se `192.36.143.134:2002`, and a separate
`api`-scope token for the same delegated user. Approval issuer initialized with
`approvals key-init`; policy `{"operations":["merge_request.merge"]}` published at
revision 1; each attempt prepared with `approvals prepare` and issued to an
owner-only proof file with `approvals issue --approve-subject --proof-output`.

`approvals clock-check` returned a bounded observation 2.07 s wide.

### What GitLab refused first

The first merge PUT was answered **401**, while the preflight GET on the same
connection returned 200. GitLab's own log shows the request authenticated:
`username: sandbox-dev, user_id: 2`. GitLab answers 401, not 403, when an identified
user may not merge into a protected branch, and `main` allowed merges by Maintainers
only while `sandbox-dev` is a Developer.

This is a sandbox configuration fact, not an adapter defect, and it is only visible
because the operations ran as a delegated user. An administrator token would have
merged and proved nothing. `main` was then reprotected with
`merge_access_level=30`.

### Three attempts

| MR | what happened | classification | PUTs to the merge endpoint | merged |
|---|---|---|---|---|
| 5 | response delivered | `applied` | 2 — the 401 above, then the merge | `29be68d0ffeb67ac01854e9a537cf53bf3a3ea19` |
| 6 | owner killed on the PUT | `unknown`, cause `unavailable` at stage `response` | 1 | `88bfb9b61b90a2560a0888d535145254f45fb3cc` |
| 7 | owner killed before the PUT | `unknown`, cause `unavailable` at stage `response` | 0 | not merged |

### The replays

**MR 6 — the response was lost and the merge had happened.** Invoking the same
idempotency key after the owner had died, with no owner running, returned
`ok: true`, `classification: applied`, `replayed: true`, carrying the original
attempt and `original_request_id`. The merge endpoint still shows **exactly one
PUT**. No second native merge was issued.

One honest nuance: the replay reported the attempt as `applied`, not as still
uncertain. Terminal replay is documented as passive, so the ledger already held the
outcome — the response was lost between the owner and the CLI, not between GitLab
and the owner. The attempt that the caller saw as uncertain is the attempt the
replay returned, and it returned it without merging again.

**MR 7 — the outcome was never recorded and no merge happened.** The same key
returned `approval_required`, `classification: not_attempted`, `replayed: false`.
The abandoned preparation was fenced rather than resumed, and a fresh proof is
required. The merge endpoint shows **zero PUTs**; MR 7 is still open.

## Repository gate

`cargo run --locked -p connectors-build -- gate --msrv`, run twice: once before the
guarded merge work and once after. The second run's full log was retained:
**35 steps, every one exit 0, 85 test targets**, `gate: all checks passed`, process
exit 0. `plan artifact validate` read 343 artifacts in `.engineering/planning` and
reported `valid`.

The first run's output was truncated to its last 40 lines when captured, so only its
verdict is known. The counts above are the second run's.

## What this evidence does not cover

- **No merge whose response was lost before the outcome reached the ledger.** The
  MR 6 crash lost the response between the owner and the CLI; the ledger already held
  `applied`. A crash between GitLab's acknowledgement and the ledger write would leave
  a genuinely unreconciled attempt, and the kill could not be timed into that window.
- No key rotation, revocation or retirement during a live write; no policy revocation
  mid-preflight; no approval replay after the proof expired.
- **No open MR with a deleted source branch.** Deleting the source branch of MR 2
  closed it, so what was read is `state: closed`, `detailed_merge_status: not_open`,
  `merge_commit_sha: null`. GitLab does not leave such an MR open, so this shape of the
  `story:gitlab-mr-reads` acceptance may not be reachable as written. MR 3 gives a
  second closed MR.
- **Unknown merge status was not produced.** MR 4 was created and read through the CLI
  as fast as the commands allow, to catch `checking` or `preparing` before the
  mergeability check finished. The first read already returned `mergeable`. On a local
  instance with one small project the check settles faster than a CLI invocation
  starts, so this case needs either a contrived slow instance or a different
  construction.
- The runner's own CA copy must be updated whenever the sandbox certificate changes.
  It was not, after the chain was reissued, and four pipelines sat `pending` with the
  runner reporting `online` — the failure is silent from the API's side.
- `connections revoke` was not exercised; the connection is left live.
- One GitLab instance, one project, one runner, one executor. No concurrency, no
  restart of GitLab under load, no credential expiry.
- The sandbox is local and disposable. It is not a claim about `gitlab.com` or any
  hosted instance.
