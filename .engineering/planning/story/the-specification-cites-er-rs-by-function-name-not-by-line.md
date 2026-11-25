---
format: aep.planning-md/3
id: story:the-specification-cites-er-rs-by-function-name-not-by-line
kind: story
status: draft
title: The specification cites er.rs by function name, so a change above line 5095 no longer breaks a test
summary: Two comments under ess/domains cite er.rs by line and a connectors-build test holds one of them; the citations name a function and the test finds it, with the emissions record regenerated
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-build/generated/metadata-emissions/inputs.json
- confidence: cited
  path: crates/connectors-build/tests/cli_spec_mapping_adversary.rs
- confidence: cited
  path: ess/domains/auth_bindings.yaml
- confidence: cited
  path: ess/domains/cli.yaml
revision: 5
---
## Defect

Two comments of the specification cite `crates/connectors-host/src/local/metadata/er.rs` by line
number, and a test of another crate fails when those lines move.

- `ess/domains/cli.yaml` line 2218: "the host removal path (er.rs:5095-5097) only expires Active
  cursors". At head f9f263a2 lines 5095 to 5097 are the arm of `removal_command` (line 5093) for
  `connectors.cli.ConnectionListCursor` in state `Active`.
- The test `the_er_rs_citation_lands_on_the_cursor_expiry_path`
  (`crates/connectors-build/tests/cli_spec_mapping_adversary.rs` lines 181 to 199) reads every
  `er.rs:<a>-<b>` in `cli.yaml`, takes those lines of `er.rs` and requires the text
  `ConnectionListCursor` in them. A change that adds or removes one line above line 5095 makes
  it red. It runs in the gate's test step.
- Three commits moved the numbers after such changes: 63e2d15e3 and 5a43fbafc (2026-10-08) and
  e54fc6b4f (2026-10-14) (`git log`, each changes `cli.yaml` and the test).
- Since then `cli.yaml` is an input of the committed metadata emissions:
  `crates/connectors-build/generated/metadata-emissions/inputs.json` records a sha256 per input
  path, 26 of them under `ess/domains/` (`auth_bindings.yaml` at line 52, `cli.yaml` at line
  56), and a digest over them, and `the_committed_metadata_emissions_are_current` compares it.
  Moving the numbers now also means a regeneration of that record.
- `ess/domains/auth_bindings.yaml` line 521 cites `er.rs:4245-4258` for "maps no Revoked row
  change to this command". No test holds it, and it is wrong today: those lines are in the
  check of read uses in `persist_batch`. The mapping it means is in `update_command` (line
  4910; `connectors.auth_bindings.PublishBinding` at line 4934). The citation was written on
  2026-10-17 (commit f962c2689).

Who needs this first:
story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process changes
`er.rs` near lines 992, 1169 and 4328, all above line 5095. Without this story its pull request
is red in `connectors-build`, or it writes `cli.yaml` and the emissions record itself.

No test time is claimed: the test takes milliseconds before and after (it reads two files).

## Change

1. `ess/domains/cli.yaml`, the comment at line 2218: the citation names the function, in the
   form `er.rs: removal_command`, and no line numbers.
2. `ess/domains/auth_bindings.yaml`, the comment at line 521: the citation names
   `er.rs: update_command`, and no line numbers.
3. `crates/connectors-build/tests/cli_spec_mapping_adversary.rs`: the test reads every citation
   of the form `er.rs: <function name>` in the files under `ess/domains/`, finds the one
   function of that name in `er.rs` (a line that starts with `fn <name>(`; exactly one), takes
   its text up to the next line that is `}` in the first column, and requires what the comment
   claims:
   - for `removal_command`: an arm with both `"connectors.cli.ConnectionListCursor"` and
     `"Active"`, and the command `"connectors.cli.ExpireConnectionListCursor"`;
   - for `update_command`: the command `"connectors.auth_bindings.PublishBinding"`.
   It fails when a file under `ess/domains/` holds a citation `er.rs:<digits>`, and when a
   citation names a function the test has no claim for. The function that finds a body takes
   the source as text, so one test calls it on two texts that hold the same function at
   different lines.
4. `crates/connectors-build/generated/metadata-emissions/inputs.json` is regenerated with
   `connectors-build metadata-conformance emissions`, because two of its inputs changed. A
   comment changes no emitted manifest: the diff under `generated/metadata-emissions/` is that
   one file. If another file there changes, the pull request says which and why before it is
   pushed.

The rule for later changes stands in the test's doc comment: a comment under `ess/` cites code
by function name.

## Acceptance

1. The test no longer depends on line numbers. Shown red first, once, in the pull request's
   body: at the base commit, with one empty line inserted at line 1 of `er.rs` and nothing else
   changed, `the_er_rs_citation_lands_on_the_cursor_expiry_path` fails (command and failing
   assertion quoted). With this change and the same inserted line, the test passes. The inserted
   line is not committed. Run on the build host:
   `cargo nextest run -p connectors-build -E 'test(/^cli_spec_mapping_adversary::/)'` (the file
   is a module of the crate's one test binary, `main`).
2. The test of point 3 that calls the body finder on two texts passes, and fails when the
   finder returns the lines of another function (shown by the test's own second text, which
   holds a second function with a different body).
3. A citation by line cannot come back unnoticed: with `er.rs:5095-5097` put back into
   `cli.yaml` the test fails and names the file and the line of the comment. Shown once in the
   pull request's body; not committed.
4. `the_committed_metadata_emissions_are_current` passes, and the step "Regenerate the metadata
   emissions and compare them byte for byte" of the forge's `slow` job of the pull request
   passes. `git diff --stat` of the pull request names, outside the planning store, the four
   files of the scope and no other.
5. The gate's test step of the pull request passes; no test of it is new except the one of
   statement 2, and none is over 60 s.

## Scope

- `ess/domains/cli.yaml`: one comment line.
- `ess/domains/auth_bindings.yaml`: one comment line.
- `crates/connectors-build/tests/cli_spec_mapping_adversary.rs`: the citation test and the test
  of its body finder.
- `crates/connectors-build/generated/metadata-emissions/inputs.json`: regenerated.

Shared files, from the `scope` of each story's frontmatter (2026-11-25):

- `ess/domains/cli.yaml` is in the recorded scope of three drafts outside the set:
  `owner-observe-request-is-modelled-in-ess`, `definition-files-in-user-config` and
  `timeout-refusal-says-how-long-it-waited`. A fourth, `admission-checks-token-scopes`, records
  the directory `ess`. No story with a recorded path under `ess/` is active. This story changes
  one comment line of the file and goes first; no edge records that, because none of the four
  needs this change. A draft that writes `cli.yaml` later starts from a head that holds it.
- `inputs.json` holds a sha256 of 26 files under `ess/domains/`, so every story that changes
  one of them regenerates it: the four above and `spec-models-operation-admission`
  (`ess/domains/declarations.yaml`). `owner-observe-request-is-modelled-in-ess` records the
  directory `crates/connectors-build/generated/metadata-emissions`. Whoever merges second
  regenerates the record on a head that holds the first; it cannot be merged by hand.
- `ess/domains/auth_bindings.yaml` and `cli_spec_mapping_adversary.rs` are named by no recorded
  scope of a story that is not finished. Three drafts record the directory
  `crates/connectors-build` (`slack-four-profile-definition-generated`,
  `slack-method-token-scope-table`, `slack-sdk-upstream-import`), and
  story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
  writes `crates/connectors-build/tests/main.rs`, the list of modules, which this story does
  not change.
- `er.rs` is read, not written. The test reads the text of `removal_command` and
  `update_command`; story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
  changes neither function.

## Not in this story

- Any change of `er.rs`:
  story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process.
- The one line citation of `er.rs` in code outside `ess/`: a doc comment of a test
  (`adapters/catalog/tests/local_runtime/oauth2_refresh_adversary_pass2.rs` line 629 cites
  `er.rs:785-797`; `git grep -nE 'er\.rs:[0-9]+'` outside the planning store finds this one and
  the two under `ess/`). No test reads it and no record hashes it. It is not changed here.
- Line citations of `er.rs` in the texts of stories in the planning store: they are records of
  what was read at a date.
- Line citations of other files in the specification (`semantics.md`): the test
  `the_corrected_semantics_citations_land_on_their_rules` holds two of them by line. That file
  changes rarely and no story of the set writes it.
- Whether the decision the two comments record is right.
- The time of the regeneration: the module comment of
  `crates/connectors-build/src/metadata_emissions.rs` (line 5) says it "takes minutes".
  It runs on the build host as a pool job that names its time (30 minutes at most).
