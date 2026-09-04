---
format: aep.planning-md/1
id: story:the-binary-says-what-it-carries
kind: story
status: draft
title: The binary says what it carries
relations:
- derived_from: epic:cli-surface
- depends_on: story:cli-first-level-groups
revision: 1
---
# Story: the binary says what it carries

## Defect

A `connectors` binary carries four independently versioned things and reports one of them. `--version`
prints the workspace version (`crates/connectors-cli/src/lib.rs:36`). The other three are readable
only by failing:

| what | where the version lives | how a person finds out today |
|---|---|---|
| the embedded catalog pack | `crates/catalog-reader/catalog.pack`, embedded at `crates/catalog-reader/src/lib.rs:73`; schema version recorded at `connectors.lock:6-9` | not at all |
| the credential file-store format | `crates/connector-secrets/src/file.rs:88-92` (`VERSION = "1"`, v2 at `:44-47`) | a refusal at `file.rs:822-829` after a write is attempted |
| hosted session metadata | `crates/connectors-client/src/identity.rs:36` (`METADATA_VERSION: u32 = 1`), checked at `:884` | `IdentityError::State`, which names no version |

The credential store migrates v1 to v2 on the first prepared-transaction write, and a 0.19.1 reader
then refuses v2 (`crates/connector-secrets/src/file.rs:30-35`). That makes downgrade-after-upgrade a
data hazard a person cannot see coming, because nothing prints which format is on disk.

## Shape

One leaf: `connectors inspect upgrade`. It reads constants already compiled into the binary and
prints them beside the install command the repository already has (`Taskfile.yaml:5,54-69`).

`inspect` is the group whose declared summary is reading what is configured, what is connected and
what cannot work (`ess/system/components.yaml:201-202`). The command writes nothing, opens no
socket, and downloads nothing.

**It is an enumerated exception, not a declared command.** `connectors-cli` owns
`connectors.target` alone, which declares no command, and `ESS-COMPONENT-004` refuses placing
another domain's command there (`ess/system/components.yaml:101-106,120-127`). So the path arrives
as one `unspecified-path: inspect upgrade — read` line in `ess/system/components.yaml` and one
matching `UNSPECIFIED_PATHS` entry, which `crates/connectors-cli/tests/cli_surface.rs:98,429,459`
holds equal in both directions, kind included.

No new dependency. Nothing in `CLI_DEPENDENCIES`
(`crates/catalog-build/tests/main/architecture_fence.rs:304-330`) changes, which is what keeps this
story one parser arm and one console function rather than a crate.

## Not in this story

**No download and no version comparison.** Resolving the newest published release and replacing this
binary is `connectors setup upgrade`, and it is blocked rather than deferred: `SHA256SUMS` is
produced by the same job that uploads the archive (`.github/workflows/release.yml:276-282,300-313`),
so it establishes integrity and not authenticity, and `crates/catalog-reader/README.md:41-49` — a
public file of this repository — states that b10x has no supported distribution channel and that the
first must follow ADR 0019 with a signed bundle manifest. A `--check-remote` flag would also be
reach into the outside world with no catalogued operation, no bound credential and no grant, which
is what O1 forbids (`AGENTS.md:15`).

**No configured catalog pack.** `[catalog] pack` and its digest are `epic:deployment-packs`.

**No correction to design 02.** `docs/design/02-architecture.md:390` states "Pre-v1 there are no
release artifacts; the repo is the product". Sixteen releases are published, the latest `v0.5.11`.
The sentence is false and amending it is a separate change, named here so it is not lost.

## Acceptance

- `connectors inspect upgrade` exits 0 and prints four version facts: the CLI version, the embedded
  pack's schema version and digest, the credential-store format version, and the session-metadata
  version.
- Each printed value is read from the constant that governs it, not restated — a test changing
  `connector-secrets`' `VERSION` changes the output.
- The command opens no socket and writes no file. It runs with the state root absent.
- `ess/system/components.yaml` and `UNSPECIFIED_PATHS` agree, and `ess/generated/clap` regenerates
  byte-identically.
- `connectors --help` still lists 8 commands.
- `bash scripts/gate.sh` exits 0.

## Depends on

`story:cli-first-level-groups`, which is what makes `inspect` a group.
