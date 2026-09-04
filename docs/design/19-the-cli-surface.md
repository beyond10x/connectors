# 19 — The command-line surface

Status: accepted 2026-09-04. First consumer of the ESS command-line construct. It declares the
`connectors` binary and its groups in `ess/system/components.yaml`, commits the clap tree projected
from that declaration, and puts both under the gate. The toolchain is pinned in
`.github/workflows/release.yml` and is the version that regenerates the committed tree
byte-identically; the pin is a fact of that file rather than of this page, which is why no version
is written here.

## Decision

The first-level shape of the `connectors` binary is declared in the specification and checked
against the parser on every run. Nothing else in this repository said what the surface is: the
completion script was already generated from the parser (`crates/connectors-cli/src/lib.rs:463-470`),
which proved the parser could be a source, but the parser answered to nothing, so a command added
in the wrong place, a command dropped, or a group renamed was invisible.

Three things carry that now:

- `ess/system/components.yaml` — `connectors-cli` is `reached_by: command_line` and its `cli:`
  block names `binary: connectors` and five groups;
- `ess/generated/clap/` — the tree `ess generate synthesize --target clap` projects from it,
  committed;
- `crates/connectors-cli/tests/cli_surface.rs` — the parser held against the declaration, both
  directions, plus `scripts/gate.sh --final`, which regenerates and diffs.

Three things about the parser that only the generated tree used to answer for are read off the
parser itself, because an independent review changed each of them alone and nothing failed. A
declared group's `--help` line is the summary the `cli:` block carries
(`every_declared_group_help_line_is_the_summary_the_specification_declares`) — editing `Admin`'s doc
comment left the whole suite green while `connectors admin --help` printed a sentence the
specification does not have. No word answers to an alias
(`no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare`) — `alias = "conn"` gave
`connectors conn` to every user, and the completion script, which is generated from the same tree,
listed it zero times. And the exception list is the specification's own enumeration, below.

## Why the groups, and only the groups

ESS ties three things together, and the three of them decide this design between them.

| rule | where it is enforced |
|---|---|
| a component's command tree places every command it accepts, exactly once, and nothing else | `validate_command_placement` — five refusals |
| a component accepts only commands of domains it owns | `ESS-COMPONENT-004`, "§9 gives a command one handler, and it is the component owning its domain" |
| a command's typed word is its `naming.wire`, or the qualified name's last segment, verbatim and un-cased | the clap target's `word()` over `Naming::wire_or` |

`connectors-cli` owns `connectors.target` alone, and that domain declares no command. So its tree
places none, and the sixteen commands `connectors-service` accepts appear in
`ess/generated/clap/TARGET.md` as sixteen target refusals — "no component declaring
`reached_by: command_line` accepts this command, so no tree places it and there is no word to
type". That list is the difference between the accepted surface and the command-line surface,
written by the generator on every run rather than asserted once in a comment.

This is not a limitation worked around; it is the right answer, and the third rule is why. Eleven
of the sixteen carry no `naming.wire`, because in this specification a `naming.wire` is the
protocol `method` tag and nothing else (`ess/system/system.yaml`, enforced by
`every_declared_wire_name_is_a_method_the_protocol_accepts`,
`crates/catalog-build/tests/main/ess_claim_fence.rs:360`). A `cli:` block over the whole accepted
surface was generated to see what it produced, and it produced `SuperviseChannel`,
`ReconnectChannel`, `ConnectChannel`, `StopChannel`, `AuthorizeConnection`, `VerifyConnection`,
`ReauthorizeConnection`, `RevokeConnection`, `FinishConnectSession`, `RefreshObservation` and
`SettleSession` as words at a shell. Every one of those is an act the platform performs on its own —
a supervisor step, a settlement, a lifecycle move — and giving each a typeable verb would mean
inventing eleven `naming.wire` values, which the claim fence refuses because they name methods no
decoder in the tree accepts.

So the specification says the two surfaces differ by making them two components. `connectors-service`
is `reached_by: network` and accepts sixteen commands, which is what it handles. `connectors-cli` is
`reached_by: command_line` and accepts none, because a thin frontend forwards a command rather than
handling it. Neither can borrow the other's commands, and ESS refuses the attempt rather than
leaving it to review.

## What that leaves undeclared, and how it is bounded

The words a person really types under those groups — `list`, `search`, `activate`, `materialize`,
`invoke` — are still undeclared, and so are the nine leaf words at the top level. Two countdowns
bound that, both in `crates/connectors-cli/tests/cli_surface.rs`:

- `UNSPECIFIED_PATHS` names each **path** the specification cannot yet carry, with a kind and a
  reason. A path is what a person types after the binary, at whatever depth: `connection activate`,
  `admin credentials set`. It has **26 entries, 9 of them one word and 17 of them two or three**,
  which is the shape of the parser and not of the top of it — a list of first-level words would let
  `connectors connection harvest` and `connectors event invoke` through, and an earlier revision of
  this page described it as one. The kind is a checked `enum Unspecified` rather than prose, so the
  sentence above the list cannot claim something the list does not carry. Both directions are held:
  a path the parser gains that is neither a declared group nor on the list fails
  `every_path_of_the_parser_is_declared_or_a_named_exception`, and an entry the parser no longer
  carries fails `every_named_exception_is_still_a_path_of_the_parser`.

  **The list answers to the specification, not to itself.** An independent review added
  `connection prune` to the parser and absorbed it with one entry, a copy of that entry in the
  drift suite and two numbers on this page — twenty-five of twenty-five green, and the refusal
  that should have caught it named the exception list as the first thing to do about it. So the
  set now lives in `ess/system/components.yaml`, as one `unspecified-path: <path> — <kind>` line
  each under the `connectors-cli` note, and
  `the_exception_list_is_the_set_the_specification_enumerates` requires the two to be equal in both
  directions, kind included. Adding a word, or relabelling one, means editing the specification.

  **How much of the kind column rests on the tree, exactly.** A second adversary pass relabelled
  each of the nineteen non-`Lifecycle` entries `Lifecycle` in turn, with a sentence naming no
  command, and got two refusals — the two `Grouping` ones. The `Forwarded` and `Read` rules fired
  only when the entry's own reason volunteered a `connectors.<domain>.<Command>` token, and that
  reason is written by whoever wants the entry. Its cases are kept, at
  `crates/connectors-cli/tests/adversary_fence_probe.rs`. What holds now is three separate things,
  and it is worth being exact about which covers what:

  - **13 of the 26** kinds are derived from the tree, by `kinds_the_tree_derives`. A path is
    followed to the frames it puts on a socket — its own dispatch arm in
    `crates/connectors-cli/src/lib.rs`, and one hop into the `connectors-console` module that arm
    hands off to, whose `LocalClient` calls are read too. Several requests is a guided sequence of
    steps and not one declared command: `Flow`. One is settled by the `method` on that frame: an
    accepted command's `naming.wire` is `Forwarded`, a read verb `ess/system/components.yaml`
    enumerates as changing no entity is `Read`, and the variant that document names as neither is
    `Unmodelled`.
  - **2 more** are `Grouping`, decided by the parser: the path carries subcommands, and no other
    kind may.
  - **The remaining 11** — `doctor`, `providers`, `auth status`, `admin integrations status` and
    the seven `Lifecycle` steps — reach no protocol request, in their own arm or through the
    module it calls, and the tree says nothing about them. Their kind is a **claim**, recorded in
    `ess/system/components.yaml` so that changing it is an edit to the reviewed document, and
    nothing measures whether it is true. That is the honest size of it.

  **The callee is followed because stopping at the arm was measured and found wrong.** An
  adversary pass showed `connectors connect` reaching seven request variants through
  `connect::dispatch` and `LocalClient` — three of them the `naming.wire` of a command
  `connectors-service` accepts — while an earlier revision of this page said the tree was silent
  about it. One hop is enough here because `product_cli_is_a_thin_frontend` keeps it so; when it
  stops being enough, `kinds_the_tree_derives` fails its own count guard rather than deriving
  less. `connect` derives `Flow`, which is the kind the list already gave it, and it is now the
  measured one: seven requests through one word is a sequence of steps.

  **What the derivation still reads out of prose, and how that prose is held.** The `Read`
  half — eight of the thirteen — turns on the read-verb enumeration in a YAML comment in
  `ess/system/components.yaml`, which `ess specify validate` never opens. An adversary pass ran
  this unit's own edit to that sentence backwards and moved `connection candidates` from `Read`
  to `Unmodelled`. So `the_read_verb_enumeration_partitions_the_protocols_it_names` holds it to
  the enums it cites: the accepted commands, the read verbs and what the document names as
  neither have to be **disjoint and exhaustive** over every variant of those three request enums.
  A verb dropped from the read half is then a variant in no part, and refused. Which part a
  variant belongs in is still a judgement — `crates/protocol` does not say whether a call changes
  an entity — and this page does not claim otherwise.

  The reason column is held separately, by
  `an_exception_whose_kind_the_tree_contradicts_is_refused`: a `Forwarded` reason has to name a
  command some component's `accepts.commands` carries, and a `Read`, `Lifecycle`, `Flow` or
  `Unmodelled` reason that names a declared command is describing a `Forwarded`. None of this
  makes a genuinely new, genuinely unmodelled act impossible to except.

  **The enumeration is a YAML comment.** `ess specify validate` never reads it and no ESS rule
  constrains it; ESS has no construct for "a word this tree carries that this document cannot
  declare", which is the whole subject of this page. What holds it is the two cases named above,
  in this repository's own test suite. It lives in `ess/system/components.yaml` rather than in a
  file of its own because it is a fact about that component's surface and belongs beside the two
  `UNMAPPED:` markers that explain why the surface cannot be declared — and because a reviewer
  reading the specification to find out what `connectors` types should not have to open a test.
- `TARGET_EXCEPTIONS` is `connection`, `event` and `operation`: the groups whose commands do not say
  which deployment they reach. `the_target_countdown_is_exactly_what_the_parser_still_owes` asserts
  **nothing about the list's size**. It derives a pending set from the parser and the tree — the
  declared groups named after a module of `crates/protocol/src` that declares a request enum, minus
  those already carrying `--target` anywhere in their subtree, the group's own arguments included —
  and requires the list to equal it. So a group that gains the flag has to leave the list, a group
  that owes it and is missing from the list fails too, and when
  `story:explicit-target-never-implicit` has finished the pending set is empty and so is the list.
  The candidate modules are read from the directory rather than named: `crates/protocol` declares
  five request enums, not three, and a literal could only ever have shrunk.

Two things would let the first countdown run down. Reads need a `views:` declaration carrying a
consistency claim, and nothing in this tree states that claim for any of them
(`ess/system/components.yaml`, the note on `connectors-service`). Forwarded commands need a
construct that lets a client declare a tree over a command it does not handle; `ess specify compose`
generates such clients today and has no `cli:` block of its own.

## The generated tree is a parallel artifact, not a replacement

`crates/connectors-cli/src/lib.rs` still declares the parser by hand, and `ess/generated/clap/` sits
beside the specification rather than inside the crate. Two reasons, both measured:

- the architecture fence caps the thin frontend at `CLI_TOTAL_LINE_LIMIT` production lines and
  counts every `.rs` file under `crates/connectors-cli` whose path contains `src`
  (`crates/catalog-build/tests/main/architecture_fence.rs:357-363`, `crates/catalog-build/tests/main/architecture_fence.rs:599-613`). Generated source
  placed there would be measured as hand-written frontend code, which is the wrong thing to
  measure;
- the clap target emits a whole crate — `crates/<system>-cli/Cargo.toml` and three modules — so an
  `--out` inside the real crate puts a second `Cargo.toml` named `connectors-cli` under its own
  `src/`.

**A note on `path:line` citations, after four of them broke in one day.** Both this page and
`ess/system/components.yaml` cited `ess_claim_fence.rs:156` as the enforcement of the `naming.wire`
rule. The commit that strengthened that fence inserted about 192 lines above it, and 156 became a
line inside a string-normalising helper. Nothing failed:
`every_citation_this_unit_wrote_resolves` and `every_citation_of_the_specification_resolves` both
check that the cited line *exists*, and both say in their own comments that they do not check what
it means. A line number is the one kind of citation that is almost always resolvable and almost
never meaningful. **A citation into a file this repository edits should name a declaration, not a
line** — `every_declared_wire_name_is_a_method_the_protocol_accepts` survives every edit above it,
and a fence can resolve it by looking for the declaration. Both sentences now carry the name, and
the line beside it is held inside the named function's span by
`adversary_fence_probe.rs::the_wire_name_rule_citation_in_the_design_document_points_at_the_rule`
and its sibling for the specification. The number is kept only because those two cases read it;
if it is ever dropped, the name is the half that was doing the work.

Two things about the emitted files are the generator's and not this repository's, and are recorded
here because a reader has no other way to find out. Their headers say to regenerate with
`cargo xtask synth --target clap` (`tree.rs`, `handler.rs`, `main.rs`, `Cargo.toml`) or with
`ess synthesize` (`PLAN.md`, `TARGET.md`). There is no `xtask` in this repository and no `ess
synthesize` subcommand; the command that produces these bytes, and the one `scripts/gate.sh --final`
diffs against, is `ess generate synthesize --path ess/system --target clap --out ess/generated/clap`.
Correcting the headers here is not available: they are compared byte for byte against a fresh
generation, so an edit would fail the gate. `the_regeneration_command_the_documents_name_is_the_one_the_gate_runs`
holds this paragraph to being true *and* to still being needed — when the generator writes the real
command, that case fails and this paragraph goes.

`plan.json` and `target.json` are generated and deliberately not committed. `json-schemas.toml`
admits a tracked JSON document only as a registered schema, a document validated against one, or
vendored source; no schema exists for the ESS report shape, so either would be a JSON file this
repository cannot classify (`crates/catalog-build/tests/main/json_governance.rs`). The two Markdown
notes beside them carry the same content for a reader.

## The cutover cannot start yet, and this document said it could

An earlier revision of this page said the cutover "happens one group at a time, with the gate green
between each". That is not implementable against the tree as it stands, and the adversary pass of
`story:cli-surface-contract` demonstrated it:
`crates/connectors-cli/tests/cli_surface_drift.rs::cutting_the_admin_group_over_to_the_generated_tree_is_refused`
takes the emitted `admin` verbatim, puts it in the parser, and the contract refuses it twice.

The reason is the whole subject of this page. `connectors-cli` accepts no command, so every group
`ess generate synthesize` emits is `subcommand_required(true)` with nothing under it. Replacing a
hand-written group with the emitted one therefore deletes that group's commands, and two rules in
`crates/connectors-cli/tests/cli_surface.rs` refuse the result — correctly:

- a declared group whose parser word carries no subcommand is a word that can only refuse;
- an entry in `UNSPECIFIED_PATHS` that the parser no longer carries is an excuse for a command
  nobody can type.

Both rules stay. The cutover is what has to change, and one thing has to happen before any of it:
**the emitted tree has to carry commands**, which means `connectors-cli` has to be able to place a
command it forwards rather than handles. Until ESS grows that construct — see the two `UNMAPPED:`
markers on the `cli:` block in `ess/system/components.yaml` — `ess/generated/clap/` is a checked
artifact and nothing more, and no group moves onto it.

When that lands, the cutover is still one group at a time with the gate green between each. A
single cutover of every group is the version of this that fails at 2 a.m.
