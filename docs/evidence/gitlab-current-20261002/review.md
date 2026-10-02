unit: current GitLab read replay — retained 2026-10-02 evidence in cb26c-plan
verdict: nothing found in bounded evidence comparison
cases: 18 retained successful distinct reads checked; 0 new provider calls or test executions
origin: introduced 0 / pre-existing 0 / undecided 0 findings
wrote-outside-worktree: own worktree lease metadata only
needs-coordinator: retain raw evidence and stated process-observation limitations

No source, implementation, test or AEP file was changed by this review. Writes are the assigned review report and requested public copy of the earlier classifier report. Existing root changes are outside review ownership. No builds, live calls, process signals, credential/keyring reads or fixture mutations occurred.

The reviewed public `read-observations.json` SHA-256 is `99ea026aa6d30aebb3ae6e916530ce7bce59595720bde1d7695ec6e5057faea4`; README SHA-256 is `f9841b3b974042781512c50b1d14cd7b374ac6d4b7d66d683ed7660159b1daa2`. This report covers those bytes. The raw source is the assigned private `.local/gitlab-replay-20261002` directory. No approval or independence claim is made.

## Comparisons performed

Read-only jq comparisons checked every public row against its raw `operation.input.json` and decoded `operation.result.json`: operation identity, successful CLI envelope, status, complete provenance object, body type, array length, UTF-8 string byte length and object id. All 18 rows matched; all 18 corresponding `.exit` files contain 0. The public rows contain 18 unique names and exactly match the 18 `effect=read` operations in `adapters/catalog/providers/gitlab/operations.json`. The generated GitLab bundle's source digest is the source revision carried by all rows: `f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`.

The historical eleven are grounded in `docs/evidence/gitlab-retirement-20260914/README.md:25`: project.get, issues.list, file.get, branch.get, merge_requests.list, merge_request.get, pipelines.list, pipeline.get, pipeline.jobs, job.get and job.trace. The other seven are projects.list, tags.list, releases.list, project.events, commits.list, repository.compare and deployments.list. The seven are additional replay coverage of shipped reads, not seven newly implemented operations.

All rows report status 200. Tags, releases and deployments are actual empty arrays in the raw responses. They establish successful empty reads, not nonempty object semantics. Other array counts and the 2,383-byte UTF-8 job trace match the public summary. Pages are not complete exports. The actual MR IID 11, pipeline id 23 and job id 27 appear in their preceding list results; repository.compare's `to` equals the observed main branch head and `from` is an observed parent. These checks returned true without printing raw provider bodies.

All 22 hashes in `private-response-hashes.sha256` match the named private raw response files, including the four empty stdout files from refused calls. The manifest itself has SHA-256 `b460002c09f936223ed702fda89e285dd9726d9291337f1bc60260f3d35ebe35`. Private home prefixes in that manifest are publication substitutions, resolved to the corresponding assigned raw files for comparison.

## Admission, restart and failures

The first revalidation's stderr retains `unavailable`, stage `dispatch`, `ok:false`; its stdout and `first-revalidation-api-observation.jsonl` are zero bytes. The empty API observation cannot establish a cause or prove absence of an upstream request. The four first attempts for project.get, issues.list, merge_requests.list and pipelines.list each retain `not_granted`, stage `admission`, `ok:false`, and exit 1. They are not counted among the 18 passing rows. Later explicit revalidation responses are ready and retain the connection identity and revision. No diagnosis of the initial refusal follows from this replay.

Raw connect and post-restart revalidation agree on connection reference, revision and external identity. Before/after owner status agrees on configuration revision and differs in both host and child incarnation. The post-restart project.get response is status 200 with expected provenance. Thus the public restart booleans agree with the raw envelopes. This is owner restart reuse, not a custody-daemon restart or graceful owner-shutdown protocol test.

All 18 retained operations-describe envelopes are `source=cached`, `stale=true`. The invocation results are successful; the README does not claim fresh descriptor observations. An initial inspection predicate assuming authoritative fresh descriptions returned false; reading these envelopes resolved that incorrect review assumption. It was not a provider failure or a reproduced product defect.

## Binary and cleanup observations; missing evidence

The retained binary copies hash to `382bba2b86b2f191f5fdff0ad67982c2a56f4ef8ff89d923edcc866422758cf8` (CLI) and `46a2886b00e72846d8a15fb69f19ce761db34406e1002b3d7765629a19c63c6f` (catalog provider), matching the manifest. The final owner's recorded executable hash equals the CLI hash. Saved owner argv names the private config/state paths; saved process stat records contain start identities for distinct old/final PIDs. The final exact adapter-stop response is successful and suppressed. Cleanup logs record executable disappearance, no detached-owner wait status, and waited exit 0 for keyring and bus.

The coordinator reports interactive shell session 59018 exited 0 and identity/start-time checks immediately preceded TERM after exact child stop. No complete executable replay script or timestamped shell transcript is retained in the inspected file set, so this pass cannot independently reconstruct that ordering, the signalling syscall, or the shell's exit. The saved identity/status artifacts and cleanup log support the bounded narrative; detached owner wait status remains unavailable. No claim of a waited owner exit, universal process cleanup, fresh release, or independent rebuild is made. The private state and shared GitLab container/runner are intentionally retained.

The relevant production-source diff file is empty, as documented. This review did not rebuild binaries to establish reproducibility or re-execute the provider workload. It does not extend the evidence to mutation/recovery, unseen pages, all GitLab workflows, sustained store performance or MCP. The README states these limits and the initial failures explicitly.

## Findings and handoff

No concrete discrepancy was found between the reviewed public read summary and retained raw input/result evidence. The missing transcript and detached wait status above are explicit evidence limits, not invented successful checks. No additional probe or source change is requested by this pass.

The requested initial classifier publication copy is `.local/provider-wave-briefs/classifier-review/report-public.md`, SHA-256 `2613dd3ba0fe38953f73df533cfa57c3af5a9382485ed96a32ebf2b8edb59360`. It preserves the initial pending-execution finding verbatim except the local home prefix becomes literal `$HOME`, with a disclosure appended. Raw report SHA remains `abb05666526c8229718de50f8a9d46a01abc7fbe87ddec8951540c4ab22283d5`. The later resolved disposition remains separately recorded in `followup-public.md`.

Outside-worktree writes are limited to the worktree CLI's own lease updates in `$HOME/.local/state/worktree/registry.sqlite3` and SQLite-managed transaction files if used. Own lease `codex-gitlab-read-review` is released with handoff; root retains ownership of evidence and cleanup. This report uses `$HOME` for the private home path and contains no secret material.

```findings
[]
```
