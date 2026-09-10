# GitLab CI read profiles/v1alpha1

Selected runtime work for C09. Runtime support reports must distinguish local
fixture verification from dedicated GitLab sandbox acceptance; both are required
before C09 is complete. The existing native project allowlist,
saved PAT profile, current connection evidence and operation policy govern every
call. All operations are reads. No trigger, retry, cancel, merge or implicit
credential revalidation is admitted by this profile.

## Exact targets and observations

| Operation | Required selection | Result |
|---|---|---|
| `pipelines.list` | configured project, exact full `sha`, limit 1..100, optional cursor | native pipeline summary page, descending numeric ID |
| `pipeline.get` | configured project, positive pipeline ID, exact `sha` | that pipeline summary with native status |
| `pipeline.jobs` | configured project, positive pipeline ID, exact `sha`, limit 1..100, optional cursor | current job attempts in that pipeline |
| `job.get` | configured project, positive job and pipeline IDs, exact `sha` | that job summary |
| `job.trace` | configured project, positive job ID, max_bytes 1..512000 | bounded UTF-8 trace prefix with completeness and byte count |

Full SHA means exactly 40 or 64 lowercase hexadecimal characters. Branch/tag
names, abbreviated SHAs and a successful pipeline for another SHA cannot satisfy
selection. Pipeline list sends the exact SHA filter and validates every returned
summary against it. Pipeline get validates ID and SHA. Jobs validate their native
pipeline ID and SHA; job get additionally validates job ID. An inconsistent
provider response fails `upstream_protocol`, never silently filters a mismatch.
Provider IDs must fit positive signed 64-bit integers. Project and text bounds
remain explicit in the authored descriptor and native codecs.

Summaries retain provider IDs, SHA, ref, status and project ID for pipelines;
job summaries retain job ID, pipeline ID, SHA, name, stage, status and allow_failure.
Native status is a bounded string, including previously unknown statuses; only
literal `success` is pipeline success. Pending/running observations remain such.
No mutable branch-head inference, implicit latest-success fallback or aggregate
merge-ready claim is supplied. A caller polls the same pipeline with fresh admitted
reads within its own deadline; a timeout does not change its provider status.

Pages are one bounded provider page per invocation. Cursors bind the exact
instance/descriptor, saved-connection partition, operation, project, SHA, pipeline
when selected, and limit. Native numeric continuations must advance. Missing
continuation on a full page requires another page rather than false completeness.
Both list operations retain descending numeric IDs and refuse duplicate or
non-descending IDs within a page; mutable offset pages are not a snapshot.
`pipeline.jobs` selects current attempts; retried and trigger-job histories are
not silently included or represented as current build jobs. An empty page reports
only that selected collection observation, not a successful CI check.

Trace selection is the project/job coordinate. It makes no SHA assertion of its
own: C09 first selects the job from the verified exact-pipeline job observation.
The private HTTP prefix capability retains at most 512000 bytes from one trace GET,
with no provider range/offset dependency or retry. Native interpretation narrows
to max_bytes; any omitted bytes make `complete:false`. EOF is authoritative only
for this response, not a promise that a running job's trace will never grow. A cut
UTF-8 suffix is dropped only on an incomplete prefix; invalid interior UTF-8 or an
invalid complete response fails. JSON escapes control characters; raw trace bytes
never become diagnostics. Permission errors and missing/erased traces remain
distinct failures, not empty success.

## Source and executable obligations

The existing unmodified OpenAPI source at GitLab revision
`2ff8d865e5016b14b724d1c2ce745f8300696192` supplies selected GET mappings for project
pipelines, one pipeline, pipeline jobs, one job and a job trace. Its digest remains
`f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`.
The source describes list responses as single entities and the trace as JSON job
data; these discrepancies remain explicit handwritten finish obligations. The
source's optional trace byte parameters are not needed by this binding.

Current documentation was checked on 2026-09-10:
[pipeline endpoints](https://docs.gitlab.com/api/pipelines/#list-project-pipelines)
and [jobs and trace endpoints](https://docs.gitlab.com/api/jobs/#retrieve-a-log-file-for-a-job).
The exact-revision jobs implementation URL returned 404 during inspection; it is
not implementation evidence for optional trace parameters. Native semantics live
here and in the adapter-owned CI ESS values; generic transport knows no GitLab IDs,
statuses, grants or byte encoding.

Prove exact-SHA filtering and mismatch refusal; pending/running/failure polling;
multiple job pages; failed-trace selection; complete, capped and split-UTF-8 traces;
original deadlines/cancellation; schema/project/policy refusal before provider
work; malformed response and paging refusal. Run actual generated-parser CLI
journeys through the persistent owner and disposable HTTPS/keyring fixtures, then
the dedicated GitLab sandbox when access is supplied. Fixtures cannot clear that
sandbox blocker. Keep existing three-read compatibility, generator refusals,
library boundaries, Rust 1.88 and the repository/website gates.
