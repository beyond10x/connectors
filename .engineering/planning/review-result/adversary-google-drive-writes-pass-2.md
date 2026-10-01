---
format: aep.planning-md/3
id: review-result:adversary-google-drive-writes-pass-2
kind: review-result
status: active
title: Adversary pass 2 on Google Drive writes
relations:
- reviews: story:catalog-google-drive-writes
revision: 1
---
unit: story:catalog-google-drive-writes at 5eff0fb86 (impl/catalog-google-drive-writes), plus one untracked test file I added
verdict: NEEDS-CHANGE
cases: executed 114→117, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: none

All three findings are gaps in the descriptions and the guide. The version guard held under every numeric edge case I tried.

**1. Diff stat**

`git --no-pager diff --stat` is empty. The only change is one untracked file: `?? adapters/catalog/tests/google_drive_writes_adversary_pass2.rs`. No implementation file was touched. I formatted it with `rustfmt` on that file alone, not `cargo fmt --package`, so no other file could be rewritten.

**2. Cases added** (all three are red now; red run of this file alone, captured before the suite ran)

| case | asserts | red output |
|---|---|---|
| `files_copy_says_where_a_copy_without_parents_lands` (:145) | The engine sends a copy with no `body.parents` (passes). The description or Writes section says where that copy lands (fails). | `a copy without body.parents lands in the source file's folder (pinned Discovery, File.parents), and neither the files.copy description ("Copy one file; the body names the copy and its parents; …") nor the guide's Writes section says so` |
| `drive_writes_say_a_name_is_not_unique_in_a_folder` (:183) | The Writes section says a create or copy can make a second file with the same name. | `Drive keeps two files of one name in one folder (pinned Discovery, File.name), and the guide's Writes section does not say a create or copy can make one` |
| `drive_writes_name_the_lock_download_and_comment_effects_they_forward` (:219) | The engine forwards `body.contentRestrictions`, `body.downloadRestrictions` and the copy query `copyComments` unchanged (passes). The descriptions or guide name them (fails). | `…names them: ["files.update: contentRestrictions", "files.update: downloadRestrictions", "files.create: contentRestrictions", "files.create: downloadRestrictions", "files.copy: contentRestrictions", "files.copy: downloadRestrictions", "files.copy: copyComments"]` |

A fourth case, a pin above `u64::MAX` compared with a different number, came out **green**. The reason is that serde_json's `arbitrary_precision` feature is enabled (`cargo tree -e features -i serde_json`), so numbers are compared exactly. I dropped that theory and turned the case into a temporary probe (part 5).

**3. Suite run** (after the cases existed)

Command: `cargo test -p connectors-catalog-provider --locked --no-fail-fast`, with `CARGO_TARGET_DIR=<worktree>/target`.
- Every other target passed: 1+11+4+8+5+21+3+5+2+7+6+4+30+5+2 = 114 passed, 6 ignored.
- `google_drive_writes_adversary_pass2`: `0 passed; 3 failed`.
- `error: 1 target failed`, `EXIT=101`.
- `<before>` = 114 comes from this run with my target excluded.
- Checks are clean: `cargo fmt --package connectors-catalog-provider -- --check` (exit 0) and `cargo clippy -p connectors-catalog-provider --locked --tests -- -D warnings` (exit 0).

**4. Findings** (covering 5eff0fb86 plus the untracked test file)

| # | file:line | measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| 1 | `operations.json:29`, `docs/catalog-google-drive.md:139` | A copy with no `body.parents` is sent as supplied. The pinned Discovery (`File.parents`) says that copy "inherits any discoverable parent of the source file". The description says only "the body names the copy and its parents", and the guide says `body.parents` "places the copy". For `files.create`, leaving out parents means My Drive. | An agent reading `operations describe` who leaves out `parents` expecting My Drive, as a create gives. The copy lands in the source's folder, possibly shared, and takes that folder's access. The approval presentation shows no folder. Fix: say in the description and table where a copy without parents lands. | NEEDS-CHANGE / introduced |
| 2 | `docs/catalog-google-drive.md:152` | The guide says a repeated create "makes another file". It never says a name does not identify a file (Discovery `File.name`: "isn't necessarily unique within a folder"). | An agent that creates or copies under an existing name gets two files with that name. A later `files.list` by name finds both. | CONFIRMED / introduced |
| 3 | `operations.json:22`, `docs/catalog-google-drive.md:137-139` | The corrected list "beyond naming and describing a file" reads as complete, but misses three things the engine forwards. `contentRestrictions` with `readOnly` locks the file: no new revision, no comments, no title change (`ContentRestriction`). With `ownerRestricted`, only the owner can lift that lock. `downloadRestrictions` restricts downloads. `copyComments` copies other users' open comments into the copy. | An issuer who reads the input for the listed members and finds none approves a lock or a download restriction. Fix: name them in the description and table, or say the list is not exhaustive. | NEEDS-CHANGE / introduced |

**5. Tried and could not break**
- **Version guard boundaries** (temporary probe, removed):
  - Pin `9223372036854775807`, as a number or a string, matches `"9223372036854775807"`.
  - These are all refused before any write: `9223372036854775808` against the int64 max, `-7` against `"7"`, `7e0`, `18446744073709551617` against the number `18446744073709551616`, and a 23-digit value against a different one.
  - `null` and `[7]` are `invalid_input`.
  - `-0` matches `"0"`, which is the same integer.
- **What the issuer checks:** `approvals issue` rebuilds the subject from `--input-file` (`docs/local-approvals.md:55-62`). The guide's "read the input file before approving" is therefore tied to the digest being approved.
- **Guide's approval claims:** `input_sha256` is `connectors_core::digest`, which is SHA-256 of the canonical input (`crates/connectors-core/src/lib.rs:339`). The presentation is instance, operation, connection and descriptor revision (`approval_issuance.rs:290-295`). The guide matches both.
- **Pinned Discovery wording:** matches for the quoted `drive.file` text, `writersCanShare`, `copyRequiresWriterPermission` and `ignoreDefaultVisibility`.
- **`version` on the PATCH:** it is never sent; `parameter_values` only forwards declared parameters (`lib.rs:381-392`).
- **Description length:** found no cap. The longest description (`files.update`) is 532 characters.

**6. Paths written outside the worktree**
- `~/.cache/w0930wd/adv2/` (created, mode 700)
- `~/.cache/w0930wd/adv2/red-only.log`
- `~/.cache/w0930wd/adv2/suite.log`
- `pass2-kept.rs` and `pass2-probe.rs` in the same directory were created and have been deleted.

Lease `wave0930c-drive-writes-adv2` is released.

**7. Findings block**

```findings
- file: adapters/catalog/providers/google-drive/operations.json
  line: 29
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "files.copy without body.parents lands in the source file's discoverable parent per the pinned Discovery, unlike create's My Drive default, and neither the description nor the guide says so, so an agent can write a copy into a shared folder it never named"
- file: docs/catalog-google-drive.md
  line: 152
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the guide never says a Drive name is not unique within a folder, so a create or copy under an existing name silently makes a second file of that name"
- file: adapters/catalog/providers/google-drive/operations.json
  line: 22
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the corrected effect lists omit body.contentRestrictions (readOnly lock, ownerRestricted), body.downloadRestrictions and the copy query copyComments, all forwarded unchanged, so a list read as complete lets an issuer approve a file lock or download restriction unknowingly"
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
