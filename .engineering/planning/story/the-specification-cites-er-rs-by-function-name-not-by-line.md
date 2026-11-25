---
format: aep.planning-md/3
id: story:the-specification-cites-er-rs-by-function-name-not-by-line
kind: story
status: draft
title: The specification cites er.rs by function name, so moved lines no longer break a test or leave a stale citation
summary: Two comments under ess/domains cite er.rs by line and a connectors-build test holds one of them within a window of three lines; the citations name a function, the test finds it and holds the arm, and the emissions record is regenerated
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
revision: 7
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
  `ConnectionListCursor` in them. Line 5095 is the arm, line 5096 the command
  `"connectors.cli.ExpireConnectionListCursor"` and line 5097 the closing brace. So the test
  stays green when 1 or 2 lines are added above line 5095 or 1 is removed, and goes red when 3
  or more are added or 2 or more are removed (the test's rule applied to the shifted window, by
  reading). Within that tolerance it passes on lines the comment no longer cites. It runs in
  the gate's test step.
- Five commits moved the numbers after such changes: 63e2d15e3 and 5a43fbafc (2026-10-08) and
  e54fc6b4f (2026-10-14) (`git log`, each changes `cli.yaml` and the test), then c97cb5660
  (2026-11-23) and df1f2c5b1 (2026-11-24).
- Since then `cli.yaml` is an input of the committed metadata emissions:
  `crates/connectors-build/generated/metadata-emissions/inputs.json` records a sha256 per input
  path, 27 of them under `ess/domains/` (`auth_bindings.yaml` at line 52, `cli.yaml` at line
  56), and a digest over them, and `the_committed_metadata_emissions_are_current` compares it.
  Moving the numbers now also means a regeneration of that record: the last two commits each
  changed one line of `cli.yaml` and 4 lines of `inputs.json`, and no other file under
  `generated/metadata-emissions/` (`git show --stat`).
- `ess/domains/auth_bindings.yaml` line 521 cites `er.rs:4245-4258` for "maps no Revoked row
  change to this command". No test holds it, and it is wrong today: those lines are in the
  check of read uses in `persist_batch`. The mapping it means is in `update_command` (line
  4910; `connectors.auth_bindings.PublishBinding` at line 4934). The citation was written on
  2026-10-17 (commit f962c2689).

Who needs this first:
story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process changes
`er.rs` near lines 992, 1169 and 4328, all above line 5095. By how many lines it moves line 5095
is not known before it is written. At 3 lines or more its pull request is red in
`connectors-build` without this story, or it writes `cli.yaml` and the emissions record itself;
under that, the test passes while the comment points at the wrong lines.

No test time is claimed: the test takes milliseconds before and after (it reads two files).

## Change

1. `ess/domains/cli.yaml`, the comment at line 2218: the citation names the function and no
   line numbers, in the form the specification already uses elsewhere, the file name followed
   by the function name in backticks (`ess/domains/execution_audit.yaml` lines 282 and 378,
   `ess/domains/credentials.yaml` lines 70 to 71): here `er.rs` and `removal_command`.
2. `ess/domains/auth_bindings.yaml`, the comment at line 521: the same form with
   `update_command`. Line 487 of that file already cites `update_command` by name. The registry
   story does not need this point: no test reads the citation. It is here because the check of
   point 3 fails on every `er.rs:<digits>` under `ess/`, and this is the only other one.
3. `crates/connectors-build/tests/cli_spec_mapping_adversary.rs`: the test holds two claims,
   one per comment. For each it requires that the file still holds the citation (the function
   name in backticks after `er.rs`), finds the one function of that name in `er.rs` (a line
   that starts with `fn <name>(`; exactly one), takes its text up to the next line that is `}`
   in the first column, and requires the arm the comment speaks of, as text:
   - `removal_command`, for "only expires Active cursors": exactly one line of the body holds
     `"connectors.cli.ConnectionListCursor"`; trimmed, it is
     `("connectors.cli.ConnectionListCursor", "Active") => {`; and the next line holds
     `"connectors.cli.ExpireConnectionListCursor"`.
   - `update_command`, for "maps no Revoked row change to this command": exactly one line of
     the body holds `"connectors.auth_bindings.PublishBinding"`, and the line before it,
     trimmed, is `("connectors.auth_bindings.Connection", ("Live" | "Stale", "Live")) => {`.
   An arm that is widened (`"Active" | "Expired"`, a `"Revoked"` state) or a second arm for the
   same command fails the test, and whoever changes the arm reads the comment again.
   The test also fails when a file under `ess/` holds a citation `er.rs:<digits>`, and names
   the file and the line. It means the file `er.rs` itself: the pattern does not match
   `owner.rs:` or `server.rs:`. It does not check the other citations of `er.rs` by name under
   `ess/domains/` (`local_approval_issuers`, `command_arguments`, `fingerprint_value`,
   `generation_row`, and `update_command` at line 487): they have no claim in it.
   The function that finds a body takes the source as text, so one test calls it on two texts
   that hold the same function at different lines.
4. `crates/connectors-build/generated/metadata-emissions/inputs.json` is regenerated, because
   two of its inputs changed. How, from `docs/development.md` lines 92 to 97: the whole run
   does not fit one pool job (a part took 8 to 27 minutes, a job may name 30 at most), so it is
   one job per part, `--part component`, `--part connectors.oauth2` and
   `--part connectors.approval_issuers`, and each part also rewrites `inputs.json`.
   - This story runs one part, `--part component`: one pool job on the build host that names
     30 minutes and brings the directory back with
     `--fetch crates/connectors-build/generated/metadata-emissions`. `connectors-build` is built
     in a job before it, so the build is not in the 30 minutes.
   - Why one part is enough: `inputs.json` is the same whichever part writes it, and a comment
     changes no emitted manifest (the two commits of 2026-11-23 and 2026-11-24 above). The
     diff under `generated/metadata-emissions/` is that one file. If the job changes another
     file there, the pull request is not pushed; the other two parts are run, one job each,
     and the pull request says which file changed and why.
   - What holds the two parts that were not run: the forge's `slow` job of the pull request
     regenerates all three with `--check` and compares every byte (acceptance 5).

The rule for later changes stands in the test's doc comment: a comment under `ess/` cites
`er.rs` by function name, never by line.

## Acceptance

1. The test no longer depends on line numbers. Shown red first, once, in the pull request's
   body: at the base commit, with 10 empty lines inserted at line 1 of `er.rs` and nothing else
   changed, `the_er_rs_citation_lands_on_the_cursor_expiry_path` fails (command and failing
   assertion quoted; 3 lines are the least that make it fail). With this change and the same 10
   inserted lines, the test passes. The inserted lines are not committed. Run on the build
   host:
   `cargo nextest run -p connectors-build -E 'test(/^cli_spec_mapping_adversary::/)'` (the file
   is a module of the crate's one test binary, `main`).
2. The test of point 3 that calls the body finder on two texts passes: both texts hold a
   second function with another body before the one it looks for, and the finder returns the
   same body from both. That the test can fail is shown once in the pull request's body: with
   the finder changed to return the first function of the text, the test fails (assertion
   quoted). The changed finder is not committed.
3. The claims hold the arms. Shown once each in the pull request's body, not committed: with
   `"Active"` changed to `"Active" | "Expired"` in the arm of `removal_command` the test fails;
   with `"Revoked"` added to the state pattern of the `PublishBinding` arm of `update_command`
   the test fails.
4. A citation by line cannot come back unnoticed: with `er.rs:5095-5097` put back into
   `cli.yaml` the test fails and names the file and the line of the comment. Shown once in the
   pull request's body; not committed.
5. The emissions record is current.
   `cargo nextest run -p connectors-build -E 'test(the_committed_metadata_emissions_are_current)'`
   passes on the build host, and the step "Regenerate the metadata emissions and compare them
   byte for byte" of the forge's `slow` job of the pull request passes. `git diff --stat` of
   the pull request names, outside the planning store, the four files of the scope and no
   other.
6. The gate's test step of the pull request passes; no test of it is new except the one of
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
- By the path in a story's text, nine more drafts say they edit `ess/domains/cli.yaml` and
  record no scope: `a-clock-refusal-of-issue-and-invoke-names-its-sources`,
  `custody-credential-file-delivery`, `operations-list-text-match`, `operations-usage-report`,
  `refusals-name-the-field`, `setup-check-reports-weakened-guarantees`, `tool-contract-serving`,
  `upgrade-moves-catalog-adapters` and `ess-format-paging-views`. The tool reads only recorded
  scope, so it does not separate them from this story. They are unassessed. The same rule
  holds for them: a draft that writes `cli.yaml` starts from a head that holds this change.
- `inputs.json` holds a sha256 of the 27 files under `ess/domains/`, so every story that changes
  one of them regenerates it: the thirteen above and `spec-models-operation-admission`
  (`ess/domains/declarations.yaml`). `owner-observe-request-is-modelled-in-ess` records the
  directory `crates/connectors-build/generated/metadata-emissions`. Whoever merges second
  regenerates the record on a head that holds the first; it cannot be merged by hand. Each
  such merge pays at least one part's job; a change of the model itself (not a comment) pays
  all three.
- `inputs.json` holds five more paths: `crates/connectors-build/src/metadata_emissions.rs`
  and `metadata_entities.rs`, `crates/connectors-spec/toolchain.json`, `ess/components.yaml`
  and `ess/system.yaml`, and the digest of the ESS crates in `Cargo.lock`. Four drafts record
  the directory `crates/connectors-build` or `crates/connectors-build/src`
  (`mcp-outbound-stdio-runtime` and the three slack drafts below). Whether they write one of
  the two sources is unassessed.
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
  `er.rs:785-797`; `git grep -nE '(^|[^a-z_])er\.rs:[0-9]+'` outside the planning store finds
  this one and the two under `ess/`). No test reads it and no record hashes it. It is not
  changed here.
- Line citations of other source files under `ess/domains/` (`gate.rs:184` in
  `declarations.yaml` line 584): no test holds them. Not changed here.
- Line citations of `er.rs` in the texts of stories in the planning store: they are records of
  what was read at a date.
- Line citations of other files in the specification (`semantics.md`): the test
  `the_corrected_semantics_citations_land_on_their_rules` holds two of them by line. That file
  changes rarely and no story of the set writes it.
- Whether the decision the two comments record is right.
- The time of the regeneration (8 to 27 minutes a part, `docs/development.md` line 92). This
  story pays one part and changes nothing about it.
- The sentence of `docs/development.md` that the parts ran "on this workstation": the jobs of
  this story run on the build host, where every pool job of the workstation runs today. The
  document is not changed here.
