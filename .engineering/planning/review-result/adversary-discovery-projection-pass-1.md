---
format: aep.planning-md/3
id: review-result:adversary-discovery-projection-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the Discovery projection
relations:
- reviews: story:catalog-discovery-projection
revision: 1
---
unit: story:catalog-discovery-projection, commit 34a954c5c on impl/catalog-discovery-projection (tree wave0930c-projection), plus one added test file
verdict: NEEDS-CHANGE
cases: executed 169→175, red 6
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 files under /dev/shm/wave0930c-projection-scratch/adv1/, plus build output in the assigned /dev/shm/wave0930c-projection-target
needs-coordinator: (a) does the story allow the new "differs only in template names" exclusion, which drops a pinned Gmail write operation; (b) should the media-upload fix list the keys in `ignored_keys` (which means changing the `media_upload/record.json` golden) or change the module's "nothing dropped" claim

**1. Diff.** `git --no-pager diff --stat` is empty because the only change is a new, untracked file. `git status --short`:
```
?? crates/connectors-catalog/tests/discovery_adversary_pass1.rs
```
It is a test file (261 lines). I created a probe, `tests/zz_adv1_probe.rs`, used it to dump projections and check float parsing, then deleted it. I touched no implementation file.

**2. Cases added** (`crates/connectors-catalog/tests/discovery_adversary_pass1.rs`). All 6 were red the first time they ran alone: `cargo test -p connectors-catalog --test discovery_adversary_pass1`, log at `adv1/red.log`.

| case | red output (verbatim) |
|---|---|
| `adversary_media_upload_keys_are_named_not_dropped` :73 | `32 Discovery keys read and dropped without a record entry:` `drive: /resources/files/methods/create/mediaUpload/accept` … `gmail: /resources/users/resources/messages/methods/send/mediaUpload/protocols/simple/multipart` |
| `adversary_number_bound_keeps_its_value` :122 | `left: "\"minimum\": 123456789.1234568,"` `right: "\"minimum\": 123456789.12345679,"` |
| `adversary_second_get_on_a_renamed_template_is_refused` :157 | `two GET methods on one OpenAPI path were accepted; excluded: [ExcludedMethod { id: "fixture.things.get", reason: "path `/things/{thingId}` differs from the projected `/things/{otherId}` only in parameter names, …" }]` |
| `adversary_root_url_without_a_host_is_refused` :182 | `accepted as server URLs: ["https://:/ -> \"https://:/fixture/v1\"", "https://../ -> …", "https://fixture.googleapis.com:port/ -> …", "https://fixture.googleapis.com::/ -> …"]` |
| `adversary_base_path_is_one_slash_and_the_service_path` :214 | `basePath values accepted: ["fixture/v1/", "//fixture/v1/", "///fixture/v1/"]` |
| `adversary_integer_values_fit_their_format` :236 | `out-of-format integers projected: ["int32 maximum=4294967296", "int32 default=-2147483649", "uint32 minimum=-1", "uint32 default=4294967296"]` |

**3. Suite.** Run after the cases existed: `cargo test -p connectors-catalog --no-fail-fast`, EXIT=101, log at `adv1/suite.log`. Every target passed except `discovery_adversary_pass1: test result: FAILED. 0 passed; 6 failed`. Total executed was 175. The "before" figure of 169 is that same run minus my 6 cases (`discovery.rs` ran 35, green).

**4. Findings.** `src/discovery.rs` does not exist at base b3a287d02 (`git cat-file` returned fatal), so every finding is introduced.

| # | file:line | what was measured | what reaches it | verdict / severity |
|---|---|---|---|---|
| F1 | crates/connectors-catalog/src/discovery.rs:1169 | `mediaUpload.accept`, `maxSize` and `protocols.*.multipart` are checked, then neither written to the output nor listed in `ignored_keys`. This breaks the module's promise at :7-10 that nothing is dropped silently. The unit's own golden `tests/discovery/media_upload/record.json` records `ignored_keys: []`, so it bakes the drop in. | 8 pinned methods, 32 pointers: Drive files.create/update and 6 Gmail methods | NEEDS-CHANGE / warning |
| F2 | crates/connectors-catalog/src/discovery.rs:313 | `canonical()` re-reads its own output with `serde_json::from_slice`, and the result depends on which serde_json features the build enables. In this crate's test build (no `float_roundtrip`), `123456789.12345679` comes back as `…1234568`. The `connectors-build` binary (`arbitrary_precision` and `float_roundtrip` on, per `cargo tree -e features`) writes `123456789.12345679`; I measured that with `connectors-build --root adv1/root discovery`. So the same bytes give different projections depending on the build. | No `number` bound or default in the four pinned docs; I built the input by hand | INFEASIBLE / warning |
| F3 | crates/connectors-catalog/src/discovery.rs:962 | A second GET on `things/{otherId}` is excluded rather than refused. The same collision with an identical template name is refused, which is what :50 requires. | The pinned Gmail name-only collision uses different verbs; this case was built by hand | INFEASIBLE / warning |
| F4 | crates/connectors-catalog/src/discovery.rs:786 | The authority filter accepts an empty host, `..`, a non-numeric port and `::` as `https://<host>/` | Built by hand | INFEASIBLE / note |
| F5 | crates/connectors-catalog/src/discovery.rs:822 | `trim_start_matches('/')` accepts a `basePath` with zero or several leading slashes | Built by hand | INFEASIBLE / note |
| F6 | crates/connectors-catalog/src/discovery.rs:455 | Integer values are not range-checked against `int32`/`uint32`, so the schema contradicts its own `format` | Built by hand | INFEASIBLE / note |
| F7 | crates/connectors-catalog/src/discovery.rs:963 | Judgement (no test). The name-only exclusion is a class the story's rule table does not list. It removes `gmail.users.settings.cse.identities.patch` from the pinned Gmail projection (`gmail.record.json`: 79 methods, 78 projected). | Pinned Gmail | CONFIRMED / note |
| F8 | crates/connectors-catalog/src/pipeline.rs:207 | Judgement (no test). `from_file` records only the file name, and nothing checks `from_sha256` against `<api>-source-hashes.json`. Any edited copy named `drive-api.json` inside the root is recorded as derived from `drive-api.json`. The digest itself stays accurate. | Nothing found that uses an unpinned copy | CONFIRMED / note |

Fixes, in the same order (named here, not applied):
- **F1:** add the three pointers to `walk.ignored` and update the golden.
- **F2:** pretty-print the sorted Value directly, without the re-parse.
- **F3:** refuse when an excluded candidate's verb is already on the chosen spelling.
- **F4:** require non-empty host labels and an optional `:digits` port.
- **F5:** compare `path == format!("/{service}")`.
- **F6:** check the value fits its format's range.

**5. Attacked and could not break:**
- **Credentials:** a method parameter with any of the 12 reserved names is refused (:1049). The only `"key"` in any output is a Calendar schema property.
- **Counts:** 186 Discovery methods = 185 projected + 1 excluded.
- **Keys:** every key present in the four pinned docs, for documents, methods, parameters and schemas, maps to a rule.
- **References:** `$ref` resolution works, and unresolved names are refused at the right pointer.
- **Pointers:** JSON-pointer escaping is correct for scope URLs, and the refusal pointers I checked name the correct key.
- **Determinism within one build:** no `preserve_order`, and `select` does not depend on declaration order.
- **`{+x}`:** the one Slides rewrite is listed, and its pattern `^[^/]+$` makes the narrowing harmless.
- **`--derived-from`:** a mismatched source is refused before the directory is touched.

**6. Paths written outside the worktree:**
- /dev/shm/wave0930c-projection-scratch/adv1/red.log
- /dev/shm/wave0930c-projection-scratch/adv1/suite.log
- /dev/shm/wave0930c-projection-scratch/adv1/{calendar,drive,gmail,slides}.openapi.json
- /dev/shm/wave0930c-projection-scratch/adv1/{calendar,drive,gmail,slides}.record.json
- /dev/shm/wave0930c-projection-scratch/adv1/root/fixture-api.json
- /dev/shm/wave0930c-projection-scratch/adv1/root/out/fixture-openapi.json
- /dev/shm/wave0930c-projection-scratch/adv1/root/out/fixture-openapi.projection.json
- Build output in /dev/shm/wave0930c-projection-target (the assigned directory, shared with the implementer)

Lease `wave0930c-projection-adv1` acquired and released.

```findings
- file: crates/connectors-catalog/src/discovery.rs
  line: 1169
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "mediaUpload accept, maxSize and protocols.*.multipart are read and neither projected nor listed in ignored_keys, dropping 32 pointers across 8 pinned Drive and Gmail methods despite the module's nothing-dropped-silently promise, and the media_upload golden encodes the drop"
- file: crates/connectors-catalog/src/discovery.rs
  line: 313
  category: property
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "canonical() re-parses its output through serde_json, whose float parsing depends on crate-graph features, so a number bound 123456789.12345679 projects as 123456789.1234568 in the library graph and exactly in the connectors-build graph"
- file: crates/connectors-catalog/src/discovery.rs
  line: 962
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "a second GET on the same OpenAPI path is excluded instead of refused when its template name differs, contradicting the documented refusal of two methods on one path and one HTTP method"
- file: crates/connectors-catalog/src/discovery.rs
  line: 786
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "rootUrl authority filter accepts https://:/, https://../, a non-numeric port and :: as an absolute https://<host>/ server"
- file: crates/connectors-catalog/src/discovery.rs
  line: 822
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "basePath check strips every leading slash, so fixture/v1/ and //fixture/v1/ are accepted where the table requires exactly / + servicePath"
- file: crates/connectors-catalog/src/discovery.rs
  line: 455
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "integer minimum, maximum and default are not range-checked against int32/uint32, so the projected schema contradicts its own format"
- file: crates/connectors-catalog/src/discovery.rs
  line: 963
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the name-only-collision exclusion is a class absent from the story's rule table and removes gmail.users.settings.cse.identities.patch from the pinned Gmail projection"
- file: crates/connectors-catalog/src/pipeline.rs
  line: 207
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the derivation records only the file name and never checks from_sha256 against the pinned source-hashes manifest, so an edited copy named drive-api.json anywhere under the root is recorded as derived from drive-api.json"
```
