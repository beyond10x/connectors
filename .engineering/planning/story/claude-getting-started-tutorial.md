---
format: aep.planning-md/3
id: story:claude-getting-started-tutorial
kind: story
status: draft
title: A getting-started tutorial from a download to Claude using Slack and Jira, with no cargo command
summary: Install from the release archive or the plugin installer; sections for connectors in general, Slack, Jira, Claude and a small Development section
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:docs-first-run-path
- depends_on: story:instances-resolve-through-locked-index
- depends_on: story:claude-connects-to-the-mcp-server
- depends_on: story:mcp-stdio-runtime-branch
- depends_on: story:definition-reader-feed-extensions
scope:
- confidence: cited
  path: .github/workflows/release-binaries.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/grafana/README.md
- confidence: cited
  path: adapters/loki/README.md
- confidence: cited
  path: adapters/prometheus/README.md
- confidence: cited
  path: apps/connectors/tests/release_journey.rs
- confidence: cited
  path: contracts/datasources/feed/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-docs/src/crates.rs
- confidence: cited
  path: crates/connectors-docs/src/development.rs
- confidence: cited
  path: crates/connectors-docs/src/getting_started.rs
- confidence: cited
  path: crates/connectors-docs/src/main.rs
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-google-calendar.md
- confidence: cited
  path: docs/catalog-google-drive.md
- confidence: cited
  path: docs/catalog-google-gmail.md
- confidence: cited
  path: docs/catalog-google-slides.md
- confidence: cited
  path: docs/catalog-hubspot.md
- confidence: cited
  path: docs/catalog-jira.md
- confidence: cited
  path: docs/catalog-openrouter.md
- confidence: cited
  path: docs/catalog-opsgenie.md
- confidence: cited
  path: docs/catalog-runpod.md
- confidence: cited
  path: docs/catalog-slack.md
- confidence: cited
  path: docs/catalog-zendesk.md
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/live-e2e.md
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: cited
  path: docs/local-kubernetes-cli.md
- confidence: cited
  path: docs/local-mcp-stdio.md
- confidence: cited
  path: docs/local-mysql-cli.md
- confidence: cited
  path: docs/local-postgres-cli.md
- confidence: cited
  path: docs/local-runtime-foundation.md
- confidence: cited
  path: docs/local-secret-service.md
- confidence: cited
  path: docs/local-tavily.md
- confidence: cited
  path: docs/verification.md
- confidence: cited
  path: website/data/runs/getting-started
- confidence: cited
  path: website/docs/connectors/grafana.md
- confidence: cited
  path: website/docs/connectors/kubernetes.md
- confidence: cited
  path: website/docs/connectors/loki.md
- confidence: cited
  path: website/docs/connectors/mysql.md
- confidence: cited
  path: website/docs/connectors/postgres.md
- confidence: cited
  path: website/docs/connectors/prometheus.md
- confidence: cited
  path: website/docs/connectors/tavily.md
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/author-a-definition.md
- confidence: cited
  path: website/docs/guides/build-a-catalog-bundle.md
- confidence: cited
  path: website/docs/guides/invoke-a-definition-file.md
- confidence: cited
  path: website/docs/guides/set-up-the-local-cli.md
- confidence: cited
  path: website/docs/reference/contracts/data/feed.md
- confidence: cited
  path: website/docs/reference/crates.md
revision: 4
---
## Outcome

The documentation site has one getting-started tutorial that takes a person from a download to
Claude working with their Slack and Jira. Nothing is compiled along the way. Its sections:

1. **Install.** Two ways in.
   - Download the release archive for the platform, check it against `SHA256SUMS`, and put
     `connectors`, `connectors-catalog-provider` and the service binaries on `PATH`.
   - Or let the Connectors plugin install it (`b10x init connectors`, prebuilt archive;
     agentplugins `plugins/connectors/skills/init/SKILL.md:13-25`).
2. **Connectors in general.** Configure an instance, `connect`, `operations list`, one read, and
   one write under approval.
3. **Slack.** The `slack.bot` and `slack.user` instances, their scopes, one read
   (`conversations.history`), and one approved `chat.postMessage`.
4. **Jira.** The `atlassian.basic` profile, one `issues.search`, and one approved
   `issue.comment.add`.
5. **With Claude.**
   - The Connectors plugin skill `connectors:integrating` and the commands it runs.
   - A link to `website/docs/connectors/mcp.md`, which story:claude-connects-to-the-mcp-server
     fills before this story lands.
6. **Development (small).** Building from source, for contributors only.

The `cargo` rule ("we don't want to have a cargo something command", across all documentation)
covers every page under `website/docs/`, `README.md`, `docs/*.md` and every `adapters/*/README.md`
a user reads to install a service. A `cargo` command appears only under a heading named
`Development`. `docs/development.md`, `docs/verification.md` and `docs/live-e2e.md` are developer
pages: they carry the heading once at the top and keep their commands. The upstream snapshots
`adapters/openrouter/upstream/README.md` and `adapters/zendesk/upstream/README.md` are vendored
inputs, not documentation, and stay as they are.

### No compilation for the native services

The release archive carries only `connectors` and `connectors-catalog-provider` today
(`.github/workflows/release-binaries.yml:107`, `BINARIES`). Seven connector pages tell the user to
`cargo build` a service (grafana, kubernetes, loki, mysql, postgres, prometheus, tavily). This
story adds the six service packages to `BINARIES`: `connectors-kubernetes`, `connectors-sql`
(mysql and postgres), `connectors-grafana`, `connectors-loki`, `connectors-prometheus` and
`connectors-tavily` (each `adapters/<name>/Cargo.toml:2,14`, package and binary of the same name).
The seven pages then install from the archive.

## Order and ownership

- Depends on story:docs-first-run-path, which lands first. Both change these pages:
  - `website/docs/getting-started.md`;
  - `guides/set-up-the-local-cli.md`;
  - `guides/author-a-definition.md`;
  - `guides/build-a-catalog-bundle.md`;
  - `guides/invoke-a-definition-file.md`;
  - the README install text, which that story moves to the site.

  This story then removes the `cargo` lines that remain on them.
- Depends on story:mcp-stdio-runtime-branch. That branch changes `README.md`,
  `docs/development.md`, `docs/local-runtime-foundation.md`, adds workspace packages that
  regenerate `website/docs/reference/crates.md`, and adds `docs/local-mcp-stdio.md` with a
  `cargo test` line (`:128`). This story's sweep and check run after it and cover that page.
- Depends on story:claude-connects-to-the-mcp-server, which fills `mcp.md` and owns the README
  support-table row and `onboarding.rs`. This story edits neither.
- Depends on story:definition-reader-feed-extensions, which edits `docs/catalog-jira.md` and
  `docs/catalog-confluence.md`. This story's sweep then changes their `cargo` lines
  (`docs/catalog-jira.md:25`, `docs/catalog-confluence.md:35,121`).
- Depends on story:instances-resolve-through-locked-index for the Slack section. Slack is held out
  of the by-name served set until that story lands.
- `website/docs/connectors/jira.md` and `slack.md` are not edited here.

## Scope

- `website/docs/getting-started.md`: rewritten as the tutorial. Today it starts with
  `cargo build --locked -p connectors -p connectors-kubernetes` (lines 17-23) and needs the
  source-only Kubernetes service (lines 34-40).
- `.github/workflows/release-binaries.yml:107`: the six service packages join `BINARIES`.
- The `cargo` sweep:
  - hand-written site pages with a `cargo` line: grafana, kubernetes, loki, mysql, postgres,
    prometheus and tavily under `connectors/`, and the four guides above;
  - `adapters/grafana/README.md:20`, `adapters/loki/README.md:15`,
    `adapters/prometheus/README.md:14`;
  - generated pages, changed at their sources:
    - `website/docs/reference/crates.md` at `crates/connectors-docs/src/crates.rs:148`;
    - `website/docs/reference/contracts/data/feed.md:246` at
      `contracts/datasources/feed/v1alpha1/semantics.md:231`, a wording change to a shared
      contract with no semantic change;
  - `README.md` (1 line), the 19 `docs/*.md` files besides the 3 developer pages at `9b7676614`,
    and `docs/local-mcp-stdio.md`.
- `crates/connectors-docs/src/development.rs` (new): the check.
- `crates/connectors-docs/src/getting_started.rs` (new): the page checks.
- `crates/connectors-docs/src/main.rs`: a `record-run` subcommand that runs one command block
  against a given `connectors` binary and writes its record.
- `website/data/runs/getting-started/*.json` (new): one recorded run per command block:
  - fields `command`, `stdout`, `exit` and `connectors_version`;
  - written by `connectors-docs record-run` against the released binary.
- `apps/connectors/tests/release_journey.rs` (new): the release journey below.
- `CHANGELOG.md`.

## Acceptance (named conformance scenarios)

`crates/connectors-docs/src/development.rs` (`cargo test -p connectors-docs`):
- `no_cargo_command_outside_development`:
  - every page in the trees above passes;
  - a fixture page with `cargo build` under `## Install` is refused, naming file and line;
  - the same line under `## Development` is accepted.
- `the_release_archive_carries_every_service_a_page_installs`: every binary a connector page
  tells the user to put on `PATH` is named in `BINARIES` of `release-binaries.yml`; fails at
  `9b7676614` for the six services.

`crates/connectors-docs/src/getting_started.rs`:
- `getting_started_installs_from_a_release_archive`: the Install section names the release
  archive, `sha256sum -c SHA256SUMS` and `b10x init connectors`.
- `getting_started_has_the_six_sections_in_order`: Install, Connectors in general, Slack, Jira,
  With Claude, Development.
- `getting_started_sections_show_their_operations`:
  - the Slack section shows `conversations.history` and `chat.postMessage` with an approval;
  - the Jira section shows `issues.search` and `issue.comment.add` with an approval;
  - With Claude names `connectors:integrating` and links `connectors/mcp.md`.
- `tutorial_commands_match_recorded_runs`:
  - every command block has a record under `website/data/runs/getting-started/`;
  - the page's output block equals the record's `stdout` after `connectors_version` and every
    RFC 3339 timestamp are replaced by placeholders.

Release journey, run once per release:
- `apps/connectors/tests/release_journey.rs::tutorial_from_download_to_first_reads`, `#[ignore]`d,
  run with `CONNECTORS_RELEASE_ARCHIVE` naming the downloaded Linux archive:
  - the test checks the archive against the release's `SHA256SUMS`, extracts it into its own
    directory and runs only those binaries, never a workspace build;
  - it serves recorded Jira and Slack answers from an in-test listener and points each instance's
    `base_url` at it, as `apps/connectors/tests/revalidate_transient.rs:54` does;
  - `issues.search` and `conversations.history` each exit 0 with the recorded records;
  - `chat.postMessage` is refused with `approval_required` until an approval is given.

  Its output is recorded as `test_result` evidence on this story.
