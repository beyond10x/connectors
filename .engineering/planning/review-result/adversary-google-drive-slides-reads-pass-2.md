---
format: aep.planning-md/3
id: review-result:adversary-google-drive-slides-reads-pass-2
kind: review-result
status: active
title: Adversary pass 2 on Google Drive and Slides reads
relations:
- reviews: story:catalog-google-drive-reads
- reviews: story:catalog-google-slides-reads
revision: 1
---
unit: story:catalog-google-drive-reads + story:catalog-google-slides-reads, impl/catalog-google-reads at 655e01a2a, working tree ~/.local/state/worktree/trees/b10x/connectors/wave0930c-reads
verdict: CONFIRMED
cases: executed 101→103, red 2
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 paths under ~/.cache/w0930rd/adv2 (details in part 6)
needs-coordinator: none

**1. What I touched**
- `git --no-pager diff --stat` prints nothing: no tracked file changed.
- `git status --short` shows one new file: `?? adapters/catalog/tests/google_reads_adversary_pass2.rs`. It is a test file.
- `cargo fmt --package connectors-catalog-provider -- --check` exits 0.
- I ran no implementation edits, no git writes and no `aep plan artifact` command.

**2. Cases added**, in `adapters/catalog/tests/google_reads_adversary_pass2.rs`. Both drive `Engine::read` against a scripted `AuthenticatedHttp`.

| case | asserts | now |
|---|---|---|
| `an_empty_text_export_is_the_empty_string` | a 200 `files.export` with no bytes (`text/csv`) has body `""` | red |
| `a_drive_usage_limit_answer_is_rate_limited_not_forbidden` | Drive's `403 userRateLimitExceeded` answer comes back as `RateLimited` | red |

Red output when each case was run alone (`cargo test --locked -p connectors-catalog-provider --test google_reads_adversary_pass2`):
```
panicked at adapters/catalog/tests/google_reads_adversary_pass2.rs:109:5:
assertion `left == right` failed: Drive's usage-limit 403 reached the caller as Forbidden
  left: Forbidden
 right: RateLimited
panicked at adapters/catalog/tests/google_reads_adversary_pass2.rs:77:5:
assertion `left == right` failed: an empty export was returned as null rather than the JSON string the guide promises
  left: Null
 right: String("")
test result: FAILED. 0 passed; 2 failed
```

**3. Suite run**, after the cases existed
- Command: `cargo test --locked -p connectors-catalog-provider --no-fail-fast`, with the environment from the invariants. EXIT=101.
- Passing targets: bundle_drift 1, confluence 11, confluence_reads_adversary 4, engine 8, gitlab_repository_reads_adversary 5, google_drive 13, google_reads_adversary 5, google_slides 7, jira 6, jira_cloud_reads_adversary 4, local_runtime 30 (6 ignored), selection_bounds_adversary 5, shipped 2.
- google_reads_adversary_pass2: 0 passed, 2 failed.
- The "before" count of 101 is the same run with my target left out.

**4. Findings**

1. **An empty text export comes back as `null`, not a string.**
   - Measured: `adapters/catalog/src/lib.rs:641-643` returns `Value::Null` for an empty body before it checks for text mode.
   - The Drive guide (`docs/catalog-google-drive.md:82`) says the body "is returned as a JSON string". The selection description (`providers/google-drive/operations.json:13`) says "returned as text".
   - Reaches: exporting an empty sheet as `text/csv`. The guide invites that when it says "Use a text `mimeType`". The output schema's `body: {}` lets `null` through.
   - Origin: the code is the same at base 5c95cd9e4 (`:641`), and GitLab's text selection (`gitlab/operations.json:31`) reaches it there. I did not run it at base, so the origin is **undecided**.
   - Fix to name: in text mode, return `""` for an empty body.

2. **Drive's rate-limit 403 comes back as `forbidden`.**
   - Measured: `lib.rs:633` maps 403 to Forbidden.
   - Google's published Drive limits say an exceeded quota answers `403 userRateLimitExceeded`, alongside 429. That comes from Google's docs; I have not seen it from live Drive.
   - Effect: an agent cannot tell a quota refusal from a permission denial, so it will not resume a `files.list` or `changes.list` walk. The guide's "rate-limited read is returned as a refusal" (`docs/catalog-google-drive.md:149-150`) only holds for 429.
   - Reaches: every Drive read under quota pressure. Drive is the first selected provider that signals quota with 403. **Introduced** (it exposes an existing mapping).
   - Fix to name: map a 403 whose `error.errors[].reason` is `userRateLimitExceeded` or `rateLimitExceeded` to RateLimited, or document the gap.

3. **Judgement: the paging end conditions are wrong when `fields` leaves out the tokens.**
   - `docs/catalog-google-drive.md:73-76,86-90` and the selection descriptions (`operations.json:8,18`) say a page without `nextPageToken` is the last, and the page with `newStartPageToken` ends the walk. Both are stated without conditions.
   - The guide also says `fields` is accepted (`:97-98`). Under Google's partial-response rules, a `fields` value without `nextPageToken`/`newStartPageToken` removes those tokens. An agent would then take page 1 as complete, or end the delta walk with no new baseline.
   - This is inferred from Google's partial-response behaviour, not measured. **Introduced**.

4. **Judgement: the thumbnail credential warning is not where the agent looks.**
   - `docs/catalog-google-slides.md:73-76` says to treat `contentUrl` as a credential.
   - The description that `operations describe` returns (`providers/google-slides/operations.json:10`, confirmed in the bootstrap descriptor) leaves that out. So the agent reading it gets no warning before it logs or passes on the URL.
   - **Introduced**.

**5. Attacked and could not break**
- **Guide commands:** run from the root of a `git archive HEAD` copy, both `discovery` commands and both `catalog --replace` commands reproduce every committed file under `adapters/google/generated` and `adapters/catalog/generated/bundles` byte for byte. Without `--replace` the result is "provider is already indexed; ask for replacement", which matches the guide.
- **Guide configs:** both JSON examples, with only `/absolute/path` substituted, load through `--print-local-bootstrap` (exit 0). The authorities are `https://www.googleapis.com/drive/v3/` and `https://slides.googleapis.com/`.
- **Slides `{+presentationId}` rewrite:** the pinned document sets `fullyEncodeReservedExpansion: true`, and the record lists the rewrite.
  - The template keeps no reserved byte (`template.rs:157`). The transport's `path_segments_mut().push` also escapes `%`, so `:`, `%`, `/` and dot-segments cannot move the path.
- **Slides scopes:** `presentations.readonly` is in the pinned scopes of all three methods.
- **Slides guide quote:** the `contentUrl` passage matches the pinned `Thumbnail.contentUrl` text.
- **`changes.list` paging:** `pageToken` is `required: true` in the pinned document. The `nextPageToken` and `newStartPageToken` semantics match the pinned descriptions.
- **Pass-1 corrections:** both absent-id tests now check that the bundle lacks the id and assert the exact absence message. I could find no mutant they miss.

**6. Paths written outside the worktree**
- ~/.cache/w0930rd/adv2 (my TMPDIR, mode 700)
- ~/.cache/w0930rd/adv2/suite.log (kept)
- ~/.cache/w0930rd/adv2/copy/ and ~/.cache/w0930rd/adv2/cfg/ (deleted)
- Lease `wave0930c-reads-adv2` is released.

```findings
- file: adapters/catalog/src/lib.rs
  line: 641
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "An empty 200 files.export body returns null, although the Drive guide and selection description promise the export as a JSON string."
- file: adapters/catalog/src/lib.rs
  line: 633
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Drive's documented usage-limit answer, 403 userRateLimitExceeded, reaches the caller as forbidden rather than rate_limited, so a paging walk cannot tell quota from permission."
- file: docs/catalog-google-drive.md
  line: 73
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The guide and selection descriptions state the paging end conditions unconditionally while accepting fields, which under Google partial response can drop nextPageToken or newStartPageToken and end a walk early."
- file: adapters/catalog/providers/google-slides/operations.json
  line: 10
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The getThumbnail description an agent reads through operations describe omits the guide's warning that contentUrl grants the requester's access to anyone holding it."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
