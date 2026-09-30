---
format: aep.planning-md/3
id: review-result:adversary-google-drive-writes-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Google Drive writes
relations:
- reviews: story:catalog-google-drive-writes
revision: 1
---
unit: story:catalog-google-drive-writes at 44e9aac03 on impl/catalog-google-drive-writes, plus my one untracked test file
verdict: NEEDS-CHANGE
cases: executed 111→114, red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (listed in part 6)
needs-coordinator: the fixes for findings 1 and 3 need changes in `adapters/catalog/src/lib.rs`, which the story forbids editing and wave 20260930b is also editing

No approval bypass and no stale-file write found. Two of the three red cases are about the update's inputs (one wrong preflight, one wrong schema) and one is about what the docs say `files.update` can do.

**1. Diff stat**

`git --no-pager diff --stat` shows nothing. The only change is one untracked test file: `?? adapters/catalog/tests/google_drive_writes_adversary.rs`. I touched no implementation file.

**2. Cases added** (in `adapters/catalog/tests/google_drive_writes_adversary.rs`; each was red on its first run, alone)

| case | asserts | red output |
|---|---|---|
| `files_update_preflight_reads_under_the_shared_drive_mode_of_the_patch` | if the input has `supportsAllDrives: true`, the `files.get` preflight carries it too, as the PATCH does | `the PATCH carries supportsAllDrives=true but its preflight read of the same file does not: [("fields", "id,version")]` |
| `files_update_says_it_moves_trashes_and_changes_access` | the engine sends `addParents`, `removeParents`, `body.trashed` and `body.inheritedPermissionsDisabled` to Drive unchanged (passes); the description or the guide's Writes section names them (fails) | `...neither its description ("Change one file's metadata while its version is the pinned version; ...") nor the guide's Writes section names ["addParents", "removeParents", "trashed", "inheritedPermissionsDisabled"]` |
| `files_update_declares_fields_required` | the declared input schema requires `fields`, which the runtime refuses without | `the runtime refuses an update without `fields`, and the declared input schema requires only ["body", "fileId", "version"]` |

**3. Suite run** (after the cases existed)

Command: `cargo test -p connectors-catalog-provider --locked --no-fail-fast`, with CARGO_TARGET_DIR set to the worktree's own `target/`.
- Every other target passed. Totals: 1+11+4+8+5+21+5+2+7+6+4+30+5+2 = 111 passed, 6 ignored.
- `google_drive_writes_adversary`: `0 passed; 3 failed`.
- `error: 1 target failed`, `EXIT=101`.
- fmt and `cargo clippy -p connectors-catalog-provider --tests -- -D warnings` are clean.

**4. Findings**

| # | file:line | measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| 1 | `adapters/catalog/providers/google-drive/operations.json:26` | The guard passes only `fileId` and `fields` to the preflight. `supportsAllDrives` goes on the PATCH but not on the preflight. | Any update of a shared-drive file. Probable effect: Drive answers the preflight 404, which `lib.rs:486-492` turns into "guard target was not found", so every shared-drive update is refused. That effect is inferred, not verified against live Drive. It refuses the write, so it cannot cause a stale one. Fix needs optional preflight values; `lib.rs:475-480` refuses any absent value. | CONFIRMED / introduced |
| 2 | `docs/catalog-google-drive.md:115,131` and `operations.json:22` | One `files.update` can move a file (`addParents`/`removeParents`), trash it (`trashed`) and remove inherited access (`inheritedPermissionsDisabled`). The guide and description call it "metadata". The guard pins only `version`. | Anyone issuing an approval. What `approval_issuance.rs:288-293` shows the issuer is only instance, operation, connection and revision; the input is only in the digest. The same gap exists for `writersCanShare`, `copyRequiresWriterPermission`, and `ignoreDefaultVisibility` on create/copy. | NEEDS-CHANGE / introduced |
| 3 | `adapters/catalog/src/lib.rs:715` | `declare()` marks a guard reference required only when no parameter already covers it, so `fields` stays optional in the schema. | An agent using `operations describe` builds an input without `fields` and gets `invalid_input`. It is the same gap as the pinned `about.get` case (`story:catalog-selection-required-parameters`). | CONFIRMED / introduced |
| 4 | `adapters/catalog/tests/google_drive.rs:1425,1469` | The approval test builds its own subject from `connectors_core::digest(input)`. It tests the approvals library, not the Drive owner path. It would stay green if the owner hashed only part of the input. The guide's Limits section says so. | nothing Drive-specific | CONFIRMED / introduced |
| 5 | `docs/catalog-google-drive.md:199-202` | "Drive refuses a write to a file outside drive.file" has not been checked against live Drive. I don't know whether a token with `drive.readonly` + `drive.file` can copy any readable file, or create a file in a folder the app did not create. | No live Drive file has been written. | INFEASIBLE / introduced |

Suggested fixes (I did not apply them):
- **Finding 2:** name these fields in the guide's Writes section and in the `files.update` description.
- **Findings 1 and 3:** both need engine changes in `lib.rs`.

**5. Tried and could not break** (temporary green checks, removed afterwards)
- **Version type coercion:** input `7` matches Drive's `"7"`, since both are the same version. `7.0`, `true`, `"07"` and `" 7"` are all refused before a write.
- **Answer without `version`:** returns `UpstreamProtocol` and no write is sent.
- **Preflight errors:** a 401, 403, 429, 500 or 302 on the preflight sends no write.
- **Upload parameters:** `uploadType`, `upload_protocol`, `alt` and `media` are each refused as `invalid_input` with no request sent. Upload paths are excluded from the projection.
- **Empty or dot-segment `fileId`:** refused before any request (`template.rs:623-628`, not new in this unit).
- **Duplicate JSON keys:** refused by `read_json`, not new in this unit.
- **Approval binding:** the digest covers the whole input (`approval_issuance.rs:277`), so nothing in the input is unbound.

**6. Paths written outside the worktree**
- `~/.cache/w0930wd/adv1/` (I created it, and its parent `~/.cache/w0930wd`, both mode 700, because they did not exist)
- `~/.cache/w0930wd/adv1/suite.log`
- `~/.cache/w0930wd/adv1/red-only.rs` was created and has been deleted.

Lease `wave0930c-drive-writes-adv1` is released.

**7. Findings block**

```findings
- file: adapters/catalog/providers/google-drive/operations.json
  line: 26
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the files.update preflight drops the input's supportsAllDrives while the PATCH carries it, so a shared-drive update's preflight reads the file under a different shared-drive mode than the write, and optional preflight values need an engine change"
- file: docs/catalog-google-drive.md
  line: 115
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "files.update is described as a metadata change while it also moves (addParents/removeParents), trashes (trashed) and removes inherited access (inheritedPermissionsDisabled) under a guard that pins only version, and the approval presentation shows none of the input"
- file: adapters/catalog/src/lib.rs
  line: 715
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the files.update input schema leaves fields optional although the runtime refuses an update without it as invalid_input"
- file: adapters/catalog/tests/google_drive.rs
  line: 1469
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the Drive approval test computes the subject digest itself and exercises only the approvals library, so it stays green if the owner path binds less than the whole input"
- file: docs/catalog-google-drive.md
  line: 199
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "the claim that Drive refuses any write outside drive.file is unverified against live Drive, including copying a readonly-visible file and creating a file in a folder the app did not create"
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
