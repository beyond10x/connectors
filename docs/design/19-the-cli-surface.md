# 19 — The command-line surface

Status: accepted 2026-09-04. First consumer of the ESS command-line construct (`ess` 0.16.0). It
declares the `connectors` binary and its groups in `ess/system/components.yaml`, commits the clap
tree projected from that declaration, and puts both under the gate.

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
`crates/catalog-build/tests/main/ess_claim_fence.rs:156`). A `cli:` block over the whole accepted
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
