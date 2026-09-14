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

## Raced merge-request update

`merge_request.update` was added after the operator accepted the C14 race boundary.
GitLab's update endpoint carries no source-SHA precondition, so the write is a
preflight read, an unguarded PUT and a postflight comparison.

Three runs against merge request 9, pinned to head `b6f6c959`:

| pinned sha | what happened | result | PUTs |
|---|---|---|---|
| current head | ordinary update | `applied`, title changed | 1 |
| `000000…` | head already differs | `forbidden` at stage `admission`, `not_attempted` | 0 |
| current head, branch moved mid-flight | see below | `applied` | 1 |

The refused run left the title untouched and issued no PUT at all: a head difference
seen in preflight is the one place it is definite.

### The postflight comparison is not detection

The third run is the important one. A commit moving `feature/conflicting` from
`b6f6c959` to `d7abb779` was pushed the moment the preflight GET appeared in GitLab's
access log, before the update PUT was sent.

- The PUT response reported `sha: b6f6c959`, the pinned head.
- The postflight comparison therefore passed, and the write was classified `applied`.
- A read taken immediately afterwards reported `sha: d7abb779`.

A merge request's recorded head is eventually consistent with its source branch. The
postflight read can confirm a head that did not move and can catch a move it happens
to see; it cannot prove that none occurred. The decision record said a postflight read
that finds the head changed reports the write as possibly applied — true, and
incomplete. It may not find it.

This does not reopen the decision, because a precondition the provider does not offer
cannot be built here. It narrows what may be claimed, and
`adapters/gitlab/contracts/raced-update.md` now says so.

### Create is not bound natively

`merge_request.create` was not added to the native adapter. The declarative-runtime
decision says GitLab is not expanded endpoint by endpoint, so create — and update
again — ran through the catalog provider instead, below.

## Through the catalog provider

`connectors-catalog-provider` bound to the GitLab bundle built from the pinned
`openapi_v3.yaml` (source SHA-256 `f9e830bd…`, 1,847 operations, 0 unsupported),
under a fresh private configuration. Executable SHA-256
`e48a84a5671d0162b5295c8fc90c210349d892f9efffd8e97048d67e31f99b2e`, configuration
revision `a938c824…`; the native configuration is [gitlab-catalog.json](gitlab-catalog.json),
the host configuration [catalog-config.toml](catalog-config.toml), the script
[catalog.sh](catalog.sh) and its log [catalog.log](catalog.log). Same delegated
user (`sandbox-dev`, id 2) with the `api` token; identity and scopes were read by
the declared probes, not by adapter code.

| attempt | operation | pin | result | requests | record |
|---|---|---|---|---|---|
| read MR 9 | `merge_request.get` | — | `ok`, status 200, head `d7abb779` | 1 GET | [catalog-get-9.json](catalog-get-9.json) |
| read branch | `branch.get` | — | `ok`, status 200 | 1 GET | [catalog-branch-applied.json](catalog-branch-applied.json) |
| read MR 999999 | `merge_request.get` | — | `service_failure`, `service_code: not_found` | 1 GET | [catalog-get-outside.json](catalog-get-outside.json) |
| create | `merge_request.create` | `b268f4f4`, the branch head | `applied`, **MR 10** opened at that head | 1 POST → 201 | [catalog-create-applied.json](catalog-create-applied.json) |
| create, stale pin | `merge_request.create` | `c64b5812`, main's head | `forbidden`, `not_attempted` | 0 | [catalog-create-stale.json](catalog-create-stale.json) |
| create again | `merge_request.create` | `b268f4f4` | `forbidden`, `refused` | 1 POST → 409 | [catalog-create-conflict.json](catalog-create-conflict.json) |
| update | `merge_request.update` | `b268f4f4` | `applied`, MR 10 retitled | 1 PUT → 200 | [catalog-update-applied.json](catalog-update-applied.json) |
| update, stale pin | `merge_request.update` | `c64b5812` | `forbidden`, `not_attempted` | 0 | [catalog-update-stale.json](catalog-update-stale.json) |
| create, branch moved mid-flight | `merge_request.create` | `84e7a49c` | `outcome_unknown`, `unknown`; **MR 11** exists at `dcadb91c` | 1 POST → 201 | [catalog-create-raced.json](catalog-create-raced.json) |

The request counts are from GitLab's access log: two POSTs answered 201, one
answered 409, one PUT to merge request 10 answered 200, and no request at all for
the two attempts the guard refused.

The raced create is the same race the native update saw, with the opposite
visibility: the branch moved between the preflight GET and the POST, GitLab
opened the merge request at the moved head, its 201 body carried that head, and
the postflight comparison saw the difference. The effect is real and the outcome
is reported uncertain, as the accepted boundary says; the merge request's IID is
in the retained response, not in the classification.

Every operation above came from the pinned source through the bundle. The
provider crate contains no GitLab code: the selection, the guard and the auth
probes are the configuration file.

### The whole native surface from the shipped selection set

Same day, a third private configuration (`cfg8`, [shipped-config.toml](shipped-config.toml))
bound the provider to the selection set the repository ships,
`adapters/catalog/providers/gitlab/operations.json`, through `operations_file` in
[gitlab-catalog-2.json](gitlab-catalog-2.json) (format `connectors-catalog-local/2`,
configuration revision `46f7e228…`, executable SHA-256 `46387422…`). All fourteen
selections were permitted; the approval policy named create, update and merge. The
connection was the same delegated `sandbox-dev` (identity subject `2`, `api` scope).
Script: [shipped.sh](shipped.sh); log: [shipped.log](shipped.log).

Every read the native adapter exposes answered `200` through the bundle:

| selection | input | answer | record |
|---|---|---|---|
| `project.get` | the project | object | [shipped-project.json](shipped-project.json) |
| `issues.list` | `per_page` 2 | 1 issue | [shipped-issues.json](shipped-issues.json) |
| `file.get` | `.gitlab-ci.yml` at `main` | object | [shipped-file.json](shipped-file.json) |
| `branch.get` | `feature/create-applied` | object | [shipped-branch.json](shipped-branch.json) |
| `merge_requests.list` | `opened`, `per_page` 3 | 3 requests | [shipped-mrs.json](shipped-mrs.json) |
| `merge_request.get` | iid 10 | head `b268f4f4…`, pipeline 19 `success`, `mergeable` | [shipped-mr-10.json](shipped-mr-10.json) |
| `pipelines.list` | `per_page` 2 | 2 pipelines | [shipped-pipelines.json](shipped-pipelines.json) |
| `pipeline.get` | 19 | object | [shipped-pipeline-19.json](shipped-pipeline-19.json) |
| `pipeline.jobs` | 19 | 1 job, id 23 | [shipped-pipeline-19-jobs.json](shipped-pipeline-19-jobs.json) |
| `job.get` | 23 | object | [shipped-job.json](shipped-job.json) |
| `job.trace` | 23 | the runner log as a string, under the selection's `"response": "text"` | [shipped-job-trace.json](shipped-job-trace.json) |

Then `merge_request.merge` under the five-check guard, against merge request 10
(source `feature/create-applied`, head `b268f4f4482cc79ecdb4803baf5db270824e31f6`,
head pipeline 19 `success`), counting PUTs to `…/merge_requests/10/merge` in the
sandbox's nginx access log:

| attempt | pinned `body.sha` | `pipeline_id` | classification | PUTs to the merge endpoint | record |
|---|---|---|---|---|---|
| stale head | `main`'s head `c64b5812…` | 19 | `not_attempted`, `forbidden` at stage `dispatch` | 0 | [shipped-merge-stale.json](shipped-merge-stale.json) |
| other pipeline | `b268f4f4…` | 18 (canceled, older head) | `not_attempted`, `forbidden` at stage `dispatch` | 0 | [shipped-merge-other-pipeline.json](shipped-merge-other-pipeline.json) |
| pinned head, pinned pipeline | `b268f4f4…` | 19 | `applied`; response `200`, `state: merged`, `sha` still `b268f4f4…` | 1 (`18:13:38`, `200`) | [shipped-merge-applied.json](shipped-merge-applied.json) |

Merge request 10 is merged with merge commit
`45fbac1b0127059aac91c45ee6cf25b1903230fb`. Afterwards `merge_request.update` on
the same request, at its still-correct head, was refused before dispatch by the
literal `/state` = `opened` check: `not_attempted`, and the access log shows no
further PUT to `…/merge_requests/10`
([shipped-update-merged.json](shipped-update-merged.json)).

The provider held every one of the native adapter's merge preconditions — open,
mergeable, pinned head, pinned pipeline, pipeline successful — as data in the
selection, with the same `not_attempted` / `applied` classifications the native
`merge_request.validate` plus `merge_request.merge` produced on 2026-09-13 above.
What the native path reported in its own vocabulary (`checks_passed`, named
blockers) the catalog path reports as a refusal at stage `dispatch` with no request
sent; the blocker's name is in the provider's refusal, not in the CLI's output.


## Nullable and unusual merge-request reads

The two cases `story:gitlab-mr-reads` names, both read through the CLI:

| case | how it was produced | observed |
|---|---|---|
| nullable source | a fork opened a merge request to the parent, then the fork project was destroyed | `source_project_id: null`, `state: closed`, `detailed_merge_status: not_open` |
| unusual merge status | a merge request whose change conflicts with `main` | `detailed_merge_status: conflict` |

An earlier attempt deleted the source *branch* and concluded the nullable case was
unreachable. That was wrong: GitLab closes a merge request whose branch goes away but
keeps `source_project_id`. It is the source **project** that has to go.

`sha` stayed non-null on the fork merge request. GitLab retains the recorded head SHA
after the source project is destroyed, so the schema's nullable `sha` was not
exercised.

## Repository gate

`cargo run --locked -p connectors-build -- gate --msrv`, run twice: once before the
guarded merge work and once after. The second run's full log was retained:
**35 steps, every one exit 0, 85 test targets**, `gate: all checks passed`, process
exit 0. `plan artifact validate` read 343 artifacts in `.engineering/planning` and
reported `valid`.

The first run's output was truncated to its last 40 lines when captured, so only its
verdict is known. The counts above are the second run's.

A third run, on the tree that adds the catalog provider, the native update and the
version 0.10.0 bump: `gate: all checks passed`, process exit 0, **90 test targets,
none failed**, every gate command exit 0, including the workspace build, test and
Clippy runs, the four adapter library boundary builds (`connectors-gitlab`,
`connectors-kubernetes`, `connectors-sql`, `connectors-catalog-provider`) and the
Rust 1.88 check of all targets. `plan artifact validate` read 345 artifacts and
reported `valid`. Website typecheck and build pass after two Kubernetes contract
pages stopped naming private evidence paths, which the public-output audit refuses.

A fourth run, on the tree that ships the GitLab selection set, the multi-check guard
and the text-response exception: `gate: all checks passed`, process exit 0, **91 test
targets, none failed** (the new `shipped` target among them), every gate command
exit 0. `plan artifact validate` reported `valid` over the two new planning
artifacts. Website typecheck and build exit 0.

## What this evidence does not cover

- **No merge whose response was lost before the outcome reached the ledger.** The
  MR 6 crash lost the response between the owner and the CLI; the ledger already held
  `applied`. A crash between GitLab's acknowledgement and the ledger write would leave
  a genuinely unreconciled attempt, and the kill could not be timed into that window.
- No key rotation, revocation or retirement during a live write; no policy revocation
  mid-preflight; no approval replay after the proof expired.
- **Nullable `sha` was not produced.** GitLab keeps a merge request's recorded head
  SHA after its source project is destroyed, so only `source_project_id` went null.
- **`merge_request.create` has no native binding.** It ran through the catalog
  provider, as did update; the native adapter was not extended.
- **Only GitLab has run through the catalog provider.** The bundle carries no
  request/response schemas, no header parameters and one token-header auth
  profile; a second provider through the same engine is still required.
- **A transient `checking` or `preparing` merge status was not read through the CLI.**
  It was observed through the API on merge request 8, but on a one-project local
  instance the mergeability check settles faster than a CLI process starts.
- The runner's own CA copy must be updated whenever the sandbox certificate changes.
  It was not, after the chain was reissued, and four pipelines sat `pending` with the
  runner reporting `online` — the failure is silent from the API's side.
- `connections revoke` was not exercised; the connection is left live.
- One GitLab instance, one project, one runner, one executor. No concurrency, no
  restart of GitLab under load, no credential expiry.
- The sandbox is local and disposable. It is not a claim about `gitlab.com` or any
  hosted instance.
