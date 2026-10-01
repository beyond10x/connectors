---
format: aep.planning-md/3
id: review-result:adversary-discovery-projection-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the Discovery projection
relations:
- reviews: story:catalog-discovery-projection
revision: 1
---
unit: story:catalog-discovery-projection, branch impl/catalog-discovery-projection at 628638db1 (tree wave0930c-projection), plus one added test file
verdict: NEEDS-CHANGE
cases: executed 177→181, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths under /dev/shm/wave0930c-projection-scratch/adv2/ (listed in part 6), plus build output in the assigned /dev/shm/wave0930c-projection-target
needs-coordinator: none

**1. Diff.** `git --no-pager diff --stat` is empty because the only change is a new, untracked file. `git status --short`:
```
?? crates/connectors-catalog/tests/discovery_adversary_pass2.rs
```
I touched no implementation file.

**2. Cases added** (`crates/connectors-catalog/tests/discovery_adversary_pass2.rs`). All 4 are red now, and all 4 were red the first time they ran alone. Command: `cargo test -p connectors-catalog --test discovery_adversary_pass2`, EXIT=101. Log: `adv2/red.log`.

| case | red output (verbatim) |
|---|---|
| `adversary2_same_verb_off_the_kept_spelling_is_refused` | `two PUT methods on one OpenAPI path were accepted: ["one unkept spelling: excluded [\"fixture.things.renameA\", \"fixture.things.renameB\"]", "two unkept spellings: excluded [\"fixture.things.renameA\", \"fixture.things.renameB\"]"]` |
| `adversary2_string_format_defaults_fit_their_format` | `defaults that contradict their format projected: ["google-datetime default=yesterday", "date-time default=yesterday", "date default=soon", "byte default=!!not base64!!"]` |
| `adversary2_upload_paths_belong_to_projected_methods` | `upload paths listed for methods that were not projected: ["fixture.things.update"]` |
| `adversary2_media_upload_without_protocols_leaves_a_trace` | `supportsMediaUpload: true with no protocols left no trace; ignored_keys: []` |

**3. Suite.** I ran it after the cases existed: `cargo test -p connectors-catalog --no-fail-fast`, EXIT=101. Log: `adv2/suite.log`.
- Every target passed except `discovery_adversary_pass2: test result: FAILED. 0 passed; 4 failed`.
- 181 cases ran. The 177 "before" figure is the same run without my 4 cases.
- `discovery.rs` ran 37 and `discovery_adversary_pass1` ran 6, all green.

**4. Findings.** `src/discovery.rs` does not exist at base b3a287d02, so every finding is introduced by this unit. All four are cases I built by hand.

| # | file:line | what was measured | what reaches it | verdict / severity |
|---|---|---|---|---|
| G1 | crates/connectors-catalog/src/discovery.rs:1036 | The F3 fix only checks a method on another spelling against the spelling that was kept. Two PUT methods that both sit on other spellings (one other spelling or two) are never compared with each other, so both are excluded. The module table at :50-51 says this case is "refused". | Built by hand. Pinned docs: only the Gmail different-verb case | INFEASIBLE / warning |
| G2 | crates/connectors-catalog/src/discovery.rs:474 | The new doc comment at :468-469 says "no schema contradicts its own `format`". In fact only integer formats and `float` are checked, so a `default` on a `date-time`, `date` or `byte` string is projected whatever it holds. | Built by hand. My grep of the four pinned docs found no string-format default | INFEASIBLE / note |
| G3 | crates/connectors-catalog/src/discovery.rs:1262 | Upload paths are recorded before `select` runs, so a method later excluded for a name-only collision still gets an `excluded_upload_paths` entry. That breaks the promise at :267 ("its method's metadata path is projected"). Its `{+x}` rewrite, by contrast, is dropped (:1028). | Built by hand. The one pinned exclusion, `gmail.users.settings.cse.identities.patch`, has no upload | INFEASIBLE / note |
| G4 | crates/connectors-catalog/src/discovery.rs:1251 | `supportsMediaUpload: true` with `protocols: {}` is read and leaves nothing in the output or the record. `supportsMediaUpload: false` is listed (:1282), so this is a silent drop of the kind the module rules out at :7-10. | Built by hand | INFEASIBLE / note |

Suggested fixes (not applied):
- **G1:** in the `elsewhere` loop, remember each (kept path, HTTP method) pair used by an excluded method too, and refuse the second one.
- **G2:** either validate `date`, `date-time` and `byte` defaults, or narrow the doc comment to the formats that are checked.
- **G3:** keep upload paths only for methods that are projected, and move them into `Candidate` the same way `rewritten` already is.
- **G4:** refuse an empty `protocols`, or list its pointer in `ignored_keys`.

**5. Attacked and could not break:**
- **F1 fix:** all four media-upload pointers are listed. The updated golden matches.
- **F2 fix:** I ran `connectors-build discovery` (the `arbitrary_precision`/`float_roundtrip` build) on the pass-1 fixture and it writes `"minimum": 123456789.12345679`. The pass-1 case, which uses the library build, is now green. No numeric value from the input is copied straight to the output; every number goes through `typed`.
- **F3 fix:** a same-method collision with the kept spelling is refused at the colliding method's `/path` pointer.
- **F4 fix:** checked `authority()`: IPv6, `@`, `%`, an empty label, a leading or trailing hyphen, a trailing dot and an empty port are all refused. A port above 65535 (`:99999`) is accepted, which matches the documented `[:<digits>]`, so I did not raise it.
- **F5 fix:** `basePath` is `/`+servicePath, or empty when servicePath is empty.
- **F6 fix:** the i32/u32/i64/u64 and f32 limits are checked. Rust's number parser does accept a leading `+`, so an int64 string `"+5"` passes through as written; I did not raise it.
- **Determinism:** no build of `connectors-catalog`, `connectors-build` or `connectors` enables `preserve_order` (checked with `cargo tree -e features -i serde_json`), so the order keys are walked in, and so which pointer a refusal names, is the same in every build. The 128-level nesting limit applies, because `read_json` never disables it.
- **Pass-1 test file:** matches the pass-1 report (261 lines), and its asserts are unchanged.

**6. Paths written outside the worktree:**
- /dev/shm/wave0930c-projection-scratch/adv2/red.log
- /dev/shm/wave0930c-projection-scratch/adv2/suite.log
- /dev/shm/wave0930c-projection-scratch/adv2/root/fixture-api.json (copied from adv1)
- /dev/shm/wave0930c-projection-scratch/adv2/root/out/fixture-openapi.json
- /dev/shm/wave0930c-projection-scratch/adv2/root/out/fixture-openapi.projection.json
- Build output in the assigned /dev/shm/wave0930c-projection-target, including a `cargo build -p connectors-build`

Lease `wave0930c-projection-adv2` was acquired and released. I ran read-only git commands only (`log`, `diff`, `status`).

```findings
- file: crates/connectors-catalog/src/discovery.rs
  line: 1036
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "select compares a method on another spelling only against the kept spelling, so two same-verb methods that both sit off the kept spelling are both excluded instead of refused, contradicting the table row at :50-51"
- file: crates/connectors-catalog/src/discovery.rs
  line: 474
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "typed's new doc claims no schema contradicts its own format, but date, date-time, google-datetime and byte string defaults are projected unchecked"
- file: crates/connectors-catalog/src/discovery.rs
  line: 1262
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "upload paths are recorded before select, so a method excluded for a name-only collision still appears in excluded_upload_paths, contrary to the UploadPath doc that its metadata path is projected"
- file: crates/connectors-catalog/src/discovery.rs
  line: 1251
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "supportsMediaUpload true with an empty protocols object is read and leaves no trace in the output or the record, a silent drop the module rules out"
```
