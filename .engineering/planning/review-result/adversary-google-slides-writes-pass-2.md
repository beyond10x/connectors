---
format: aep.planning-md/3
id: review-result:adversary-google-slides-writes-pass-2
kind: review-result
status: active
title: Adversary pass 2 on Google Slides writes
relations:
- reviews: story:catalog-google-slides-writes
revision: 1
---
unit: story:catalog-google-slides-writes, impl/catalog-google-slides-writes at 57737bb2a plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 100→103, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: The story's Acceptance still says the guide "names `connections repair` as the way to add a write scope". The correction now asserts the opposite (google_slides.rs, `!flat.contains("with `connections repair`")`). Someone who can write to the store has to amend the story.

**1. Diff proof.** `git --no-pager diff --stat` is empty: I changed no tracked file. `git status --short` shows one new path, and it is a test file:
```
?? adapters/catalog/tests/google_slides_writes_adversary_pass2.rs
```

**2. Cases added** (`adapters/catalog/tests/google_slides_writes_adversary_pass2.rs`; rustfmt and `clippy -D warnings` are clean). I ran the file alone first, and all three cases were red:
```
panicked at adapters/catalog/tests/google_slides_writes_adversary_pass2.rs:155:5:
assertion `left == right` failed: the write instance's authorize_url differs from the auth_uri of Google's client file, so `connections connect` with that file is refused before consent
  left: String("https://accounts.google.com/o/oauth2/v2/auth")
 right: "https://accounts.google.com/o/oauth2/auth"
panicked at ...pass2.rs:189:5:
neither the batchUpdate description nor the guide says an image URL is saved into the deck as Image.sourceUrl, readable by every viewer, while getThumbnail's description forbids sharing that URL
panicked at ...pass2.rs:230:5:
a 200 whose commentUpdateState is ALL_FAILED_UNKNOWN_REASON is reported applied, while the guide says Google applies the requests all or none and never names commentUpdateState
test result: FAILED. 0 passed; 3 failed
```
- **`the_documented_write_instance_can_connect_from_the_google_client_file`**: takes the guide's read block, applies the guide's write values to it, and asserts that its `authorize_url` equals the `auth_uri` in Google's client file.
- **`an_image_url_published_by_batch_update_is_named_where_the_thumbnail_warning_is`**: sends a `createImage` whose `url` is a thumbnail `contentUrl`. The engine forwards the URL unchanged and reports `Applied`. The case then asserts that the description or the guide names `sourceUrl`.
- **`a_batch_whose_comment_updates_failed_is_not_reported_as_all_applied`**: sends `insertComment` in a batch, and Google answers 200 with `commentUpdateState: ALL_FAILED_UNKNOWN_REASON`. The case asserts that the outcome is not `Applied`, or that the guide documents that field.

**3. Suite run** (after the cases existed): `cargo test --locked -p connectors-catalog-provider --no-fail-fast`, EXIT=101. Every binary passes except mine (`google_slides` 12, `google_slides_writes_adversary` 2, `local_runtime` 30 with 6 ignored). My binary gives `FAILED. 0 passed; 3 failed`. The count of 100 is the same run without my binary.

**4. Findings** (these cover 57737bb2a)

| # | file:line | what was measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| F1 | docs/catalog-google-slides.md:187 | The guide tells the reader to connect the write instance "with the Google client file". The write instance copies `authorize_url` `…/o/oauth2/v2/auth` from the read block (:211). The client-file flow compares that URL byte for byte with the file's `auth_uri`, which is `…/o/oauth2/auth` (`crates/connectors-host/src/local/oauth.rs:160` on `impl/cli-oauth-loopback-acquisition` 839cb135c). If they differ it refuses with `protected_entry_unavailable` before consent. That branch already changed `docs/local-catalog-provider.md` to `/o/oauth2/auth`. `google_slides.rs:420` pins v2, and that pin was already there at the base. | This story `depends_on` the loopback story. Once that story merges, the guide's only documented way to connect a write instance is refused. In this tree, without the loopback story, a client file is not supported at all. The fix is to use `https://accounts.google.com/o/oauth2/auth` at :211 and :420. The Drive guide on the integration branch has the same defect. | NEEDS-CHANGE / introduced |
| F2 | adapters/catalog/providers/google-slides/operations.json:14 | `getThumbnail`'s description says `contentUrl` is a credential and must not be shared. The pinned document says `createImage`, `replaceImage` and `replaceAllShapesWithImage` store their URL in the deck as `Image.sourceUrl`. The engine forwards that URL verbatim. Neither the description nor the guide says so. The approval presentation shows only the instance, operation, connection, revision and input digest (`approval_issuance.rs:290-295`). | An agent that copies another deck's page into this deck with `getThumbnail` then `createImage` publishes the credential to every viewer. All the operations it needs are in the shipped set and the write instance. This is also the one cross-file path open under the documented scope (see part 5). | NEEDS-CHANGE / introduced |
| F3 | docs/catalog-google-slides.md:135 | The guide says "applies them all or none", and the description says "atomically". The pinned document's `commentUpdateState` can be `ALL_FAILED_UNKNOWN_REASON` on a 200, and the engine reports `Applied`. | Only the comment requests (`insertComment`, `deleteComment`, `addCommentReply`, `updateCommentPost`) trigger it. The pinned document marks them Developer Preview, and I found nothing showing an enrolled project. The fix is one doc line. | INFEASIBLE / introduced |

**5. Attacked and not broken**
- **Write-instance config:** it loads through `--print-local-bootstrap`. Instance ids only have to be unique (`config.rs:267`), and I found no identity-uniqueness rule across instances in `registry/lifecycle.rs`.
- **Sheets charts and Drive videos:** `createSheetsChart`, `replaceAllShapesWithSheetsChart`, `refreshSheetsChart` and a Drive `createVideo` need spreadsheets or drive scopes (pinned document). The documented write instance requests only `openid` and `presentations`, so these cannot read another file. That comes from the pinned document; I have not observed it live.
- **Preflight query:** the preflight sends no `fields`, and a missing `revisionId` refuses before the POST (`lib.rs:492-499`).

**6. Paths written outside the worktree**
- `~/.cache/w0930ws/adv2/` (scratch directory, mode 700)
- `~/.cache/w0930ws/adv2/suite.log`

Build output went only to `<worktree>/target`.

**7. Findings block**
```findings
- file: docs/catalog-google-slides.md
  line: 187
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The guide connects the write instance with the Google client file, but its authorize_url is the v2 endpoint while the client-file flow this story depends on requires byte equality with the file's auth_uri https://accounts.google.com/o/oauth2/auth, so the documented connect is refused before consent."
- file: adapters/catalog/providers/google-slides/operations.json
  line: 14
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "batchUpdate image requests persist their URL as Image.sourceUrl for every viewer, so a getThumbnail contentUrl the set calls a credential is published by the natural copy-a-page workflow, and neither the description, the guide nor the approval presentation shows it."
- file: docs/catalog-google-slides.md
  line: 135
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The guide says batchUpdate applies all or none, but the pinned document's commentUpdateState can report ALL_FAILED_UNKNOWN_REASON on a 200 for Developer Preview comment requests, which the engine reports as applied."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
