---
format: aep.planning-md/3
id: review-result:adversary-google-drive-slides-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Google Drive and Slides reads
relations:
- reviews: story:catalog-google-drive-reads
- reviews: story:catalog-google-slides-reads
revision: 1
---
unit: story:catalog-google-drive-reads (326d173c0) + story:catalog-google-slides-reads (92567a849), working tree of impl/catalog-google-reads at ~/.local/state/worktree/trees/b10x/connectors/wave0930c-reads
verdict: CONFIRMED
cases: executed 96→101, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (~/.cache/w0930rd/adv1/suite.log, and ~/.cache/w0930rd/adv1/bundles-copy/, which I have deleted)
needs-coordinator: yes. The brief said to run no git commands, so there is no `git diff --stat` below. I found which files are new by comparing sibling trees (wave0930c-integration, -projection and -refresh have no `adapters/google/generated` and no Google providers).

**1. What I touched (listed instead of `git diff --stat`)**
- I added one file: `adapters/catalog/tests/google_reads_adversary.rs`. It is a test file.
- I edited no implementation file and ran no `aep plan artifact` write.

**2. Cases added.** Each was run alone first: `cargo test --locked -p connectors-catalog-provider --test google_reads_adversary`, 3 passed, 2 failed.

| case | asserts | now |
|---|---|---|
| `the_drive_absent_selection_case_is_refused_for_its_absence` | loading `drive.files.download` is refused because the bundle has no such operation, which is what the existing test claims | red |
| `about_get_declares_fields_required` | `about.get`'s input schema lists `fields` as required | red |
| `drive_file_id_never_splits_or_leaks_into_the_query` | a `fileId` containing `/`, `?` or `#` is sent as one escaped segment (`files.get`, `files.export`) | green |
| `files_export_at_and_over_the_response_limit` | an export of exactly `RESPONSE_LIMIT` bytes comes back as text; one byte more is refused as capacity | green |
| `files_export_of_non_utf8_bytes_is_refused_as_upstream_protocol` | the guide's claim that non-UTF-8 bytes fail as `upstream_protocol` holds | green |

Red output, verbatim:
```
panicked at adapters/catalog/tests/google_reads_adversary.rs:67:5:
refused for another reason: selection `about.get` declares effect `Read` for method `post`
panicked at adapters/catalog/tests/google_reads_adversary.rs:89:5:
`about.get` input schema requires []; `{}` is admitted and Drive refuses it
test result: FAILED. 3 passed; 2 failed
```

**3. Suite run (after the cases existed)**
- Command: `cargo test --locked -p connectors-catalog-provider --no-fail-fast`, with the environment from the invariants. EXIT=101.
- Every target passed except mine: bundle_drift 1, confluence 11, confluence_reads_adversary 4, engine 8, gitlab_repository_reads_adversary 5, google_drive 13, google_slides 7, jira 6, jira_cloud_reads_adversary 4, local_runtime 30 (6 ignored), selection_bounds_adversary 5, shipped 2.
- google_reads_adversary: 3 passed, 2 failed.
- The "before" count of 96 is the same run with my target left out.

**4. Findings**

1. **The Drive "absent selection" test passes for the wrong reason.**
   - Measured: `adapters/catalog/tests/google_drive.rs:146-149` says "no method named for them exists". But `drive.files.download` is a POST in the pinned Discovery document, in `drive.openapi.json` and in the committed bundle (`/drive/v3/files/{fileId}/download`). `Engine::new` refuses it at `adapters/catalog/src/lib.rs:242-246` because of the effect/method mismatch, not at `:225` (absence).
   - Effect: if the absence check were removed, this test would stay green. The Slides counterpart (`slides.presentations.list`, which really is absent) still catches that.
   - Fix to name: substitute an id the bundle does not have, and assert the refusal message.
   - Reaches: gate only. CONFIRMED, note, introduced.

2. **`about.get` accepts `{}`, but Drive refuses any call without `fields`.**
   - Measured: the declared input schema's `required` list is `[]`. The projection marks `fields` optional, and `declare` (`lib.rs:687`) only copies `required` from the projection. `Selection` has no way to add a required parameter. So `{}` passes validation, a request is sent, and Drive's 400 comes back as a generic `invalid_input` "provider refused the request" (`lib.rs:631`).
   - Reaches: any agent that builds its input from `connectors operations describe`. The only warning is prose, in the selection description (`providers/google-drive/operations.json:6`) and the guide (`docs/catalog-google-drive.md`, `about.get` bullet).
   - Fix to name: a per-selection `required` override in the engine, or refuse `about.get` without `fields` before sending.
   - CONFIRMED, warning, introduced (the selection is new; the missing mechanism in the engine was already there).

3. **The documented regeneration command fails when re-run.**
   - Measured: `bundle::write` (`crates/connectors-catalog/src/bundle.rs:261-264`) refuses a provider that is already indexed unless `--replace` is given. Both guides' `catalog` commands (`docs/catalog-google-drive.md`, `docs/catalog-google-slides.md`, "Source and bundle") point at the committed directory without `--replace`. Anyone regenerating after a re-pin gets "provider is already indexed".
   - This comes from reading the code. I could not run it: the CLI refused my scratch directory ("declaration paths must be repository-relative"). The Jira, Confluence and GitLab guides follow the same pattern.
   - Reaches: a maintainer re-pinning Discovery. CONFIRMED, note, introduced (the new guide text).

**5. Attacked and could not break**
- The six Drive and three Slides selection ids equal their Discovery ids without the prefix, and all are GET. The engine refuses a read that maps to a non-GET method (`lib.rs:239-248`).
- `pageSize` bounds match Discovery (1–1000). Values `0`, `1001`, `"00"`, `-0`, `+1`, floats and non-ASCII digits are all refused before any request is sent (`check_bounds`).
- Both paging tests check the end conditions and the exact targets for real. Removing a bound or `response: text` fails the existing tests.
- `bundle_drift` really regenerates the projection, its record, the bundle and the index. The derivation's `from_sha256`/`from_bytes` equal `drive-source-hashes.json` and `slides-source-hashes.json`.
- Path values of `.` and `..` are refused (`template.rs:627`), and `/`, `?` and `#` are escaped.
- Guide claims hold: 64 and 5 methods, none excluded; the thumbnail enums; `commentsViewMode`; the 30-minute `contentUrl`; `alt` refused; capacity at 4 MiB + 1.
- The export always comes back as text or fails. It never returns a non-text body.

**6. Paths written outside the worktree**
- ~/.cache/w0930rd/adv1 (my assigned TMPDIR, mode 700)
- ~/.cache/w0930rd/adv1/suite.log
- ~/.cache/w0930rd/adv1/bundles-copy/ (deleted)

```findings
- file: adapters/catalog/tests/google_drive.rs
  line: 148
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The substituted id drive.files.download exists in the bundle as a POST, so the test is refused for its effect/method mismatch rather than the absence it is named for, and would stay green if the absence check were removed."
- file: adapters/catalog/providers/google-drive/operations.json
  line: 5
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "about.get's declared input schema does not require fields, so a describe-driven caller's {} passes validation and spends a request on a certain Drive 400, reported as a generic invalid_input."
- file: docs/catalog-google-drive.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The documented catalog regeneration commands in both Google guides omit --replace, which bundle::write requires for a provider the committed index already carries, so re-running them after a re-pin is refused."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
