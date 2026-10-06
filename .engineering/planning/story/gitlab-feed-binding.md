---
format: aep.planning-md/3
id: story:gitlab-feed-binding
kind: story
status: active
title: GitLab binds the feed family
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
- depends_on: story:feed-bindings-discoverable
- depends_on: story:catalog-feed-engine
- depends_on: story:feed-profile-capabilities
scope:
- confidence: cited
  path: adapters/catalog/contracts/feed/v1alpha1/gitlab.md
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: cited
  path: adapters/catalog/spec/ess/domains/feed.yaml
- confidence: cited
  path: adapters/catalog/src/feed.rs
- confidence: cited
  path: adapters/catalog/tests/feed/gitlab
- confidence: cited
  path: adapters/catalog/tests/feed_engine.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_feed.rs
- confidence: inferred
  path: adapters/catalog/tests/local_runtime.rs
- confidence: inferred
  path: adapters/catalog/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/oauth2_refresh.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/catalog_feed_conformance.rs
- confidence: cited
  path: crates/connectors-build/src/gitlab_feed_conformance.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T19:55:10Z", actor: "agent:claude", revision: 3}
---
## Outcome

The GitLab adapter binds `datasource.feed/v1alpha1`: its containers and items are readable through the family, with the binding's own profile, under `adapters/catalog/contracts/feed/v1alpha1/`.

## Acceptance

Revised 2026-10-07 for the first feed binding a consumer reads (cortex `story:feed-source` waits on it).

1. Spec first: the catalog feed model (`adapters/catalog/spec/ess/domains/feed.yaml`) and `contracts/catalog/v1alpha1/semantics.md` §3.3 gain a last-key cursor (continue after the key of the last record of a full page; GitLab's `id_after`) and a fixed container kind (`kind_word`, exactly one of `kind` and `kind_word`); the engine (`adapters/catalog/src/feed.rs`) realizes them. These two items moved here from `story:catalog-feed-engine-extensions`. The model's `kind` comment is corrected to the pointer the engine reads.
2. `adapters/catalog/providers/gitlab/operations.json` declares the feed: projects the token's user is a member of as containers, merge requests as items, `url` null, every container `private` (no visibility rule), as decided 2026-10-06. The profile declares its capabilities (`story:feed-profile-capabilities`): deletions not observed, fixed kind word, update-time revision, all-private visibility.
3. The family's suite passes against the binding with recorded provider fixtures, every scenario its declared capabilities select, and the harness names the ones left out. `adapters/catalog/contracts/feed/v1alpha1/gitlab.md` states what a container is, how the watermark is formed, how far a first read reaches, that deletions are not observed and why every project is listed `private`.
4. A CLI journey through a saved GitLab connection against the stand-in provider (`adapters/catalog/tests/local_runtime.rs`, new `/projects` and `/projects/:id/merge_requests` routes): `operations list --family datasource.feed/v1alpha1` names `feed.containers` and `feed.items`, `operations invoke` lists containers, reads a first items page, and resumes from the returned watermark.
5. The configuration revision of a saved GitLab connection moves (the effective configuration gains `feed`); the CHANGELOG states the migration (`connections upgrade`), and the revision pins in `gateway_prefix.rs`, `oauth2_refresh.rs` and `gitlab_commit_reads_adversary.rs` move with it.
6. Where the GitLab sandbox of `docs/evidence/gitlab-sandbox-20260913/` is running, a recorded real-CLI transcript (containers, a first items page, a resumed page) is kept as evidence; where it is not, that is stated and the release says the binding is checked against recorded fixtures only.

The 2026-10-06 attempt is the archive `connectors-w4-gitlab` (`dirty.patch`, applies to main with `git apply --3way`); it maps `public` and `internal` to `public`, which item 2 reverses.

## Depends on

`story:feed-contract`, `story:feed-bindings-discoverable`, `story:catalog-feed-engine`, `story:feed-profile-capabilities` (revised 2026-10-07: no longer `story:catalog-feed-engine-extensions`, whose two items this binding needs moved here).

## Decided 2026-10-06 (wave 20261006d)

## Decided 2026-10-06 (wave 20261006d)

- Items are merge requests only: one connection carries one binding with the fixed ids, so one profile yields one item kind.
- Visibility: every project maps to `private` until visibility can be read from two fields (a public project with members-only merge requests must not be listed public).
- `url` stays null: a project rename changes `web_url` without changing the merge request revision.
- The unit's binding and engine patch are kept in the worktree archive `connectors-w4-gitlab`.

## Scope

Derived 2026-10-07 by `aep:story-scoper`. **Cited** = read from the story, the tree or the archived patch; **inferred** = a reading that could be wrong.

- **Primary surface:** `adapters/catalog/providers/gitlab/operations.json` (the `feed` block) — cited
- **Binding files (new):** `adapters/catalog/contracts/feed/v1alpha1/gitlab.md`, `adapters/catalog/tests/gitlab_feed.rs`, `adapters/catalog/tests/feed/gitlab/{project,merge_request}.json`, `crates/connectors-build/src/gitlab_feed_conformance.rs`; mod line in `crates/connectors-build/src/main.rs:21-24` — cited (patch)
- **Shipped feeds checked against the model:** `crates/connectors-build/src/catalog_feed_conformance.rs:506-545` — cited (patch)
- **Revision pins that move:** `adapters/catalog/tests/gateway_prefix.rs:126-134`, `adapters/catalog/tests/local_runtime/oauth2_refresh.rs:1141-1200`, `adapters/catalog/tests/gitlab_commit_reads_adversary.rs:309-370` — cited (patch)
- **Engine (last-key cursor):** `CursorPaging` `adapters/catalog/src/feed.rs:85-91`, `Feed::new` `:326`, `Feed::containers` `:587-598`; model `adapters/catalog/spec/ess/domains/feed.yaml:42-48`; §3.3 `contracts/catalog/v1alpha1/semantics.md:139-140,157,161,173` — cited
- **Engine (`kind_word`):** `feed.rs:115,329,583`; model `feed.yaml:74-75`; §3.3 `:176`; refusal cases `adapters/catalog/tests/feed_engine.rs:186-260` — cited
- **Not needed by GitLab:** offset paging, next-URL cursor, JQL/CQL, `±hhmm` (GitLab writes `.081Z`, accepted by `instant` `feed.rs:1043-1118`), minute precision, relative URLs (`url` null) — cited
- **CLI journey:** template `gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart` (`adapters/catalog/tests/local_runtime/cli_journey.rs:323`, `#[ignore]`: needs the built CLI and a disposable Secret Service); helpers `Cli::operation_result` `:189-214`, `configure` `:301-321` (permissions at `:306` gain `feed.containers`, `feed.items`); stand-in `Provider` `adapters/catalog/tests/local_runtime.rs:68-445` loads the shipped `operations_file` (`:423`) — cited; the two new routes — inferred
- **Invoke path:** owner `admit_operation` (`crates/connectors-host/src/local/owner.rs:321-334,400-405`) → child `invoke_explained` (`adapters/catalog/src/local.rs:1329-1346`) → `Engine::read` feed dispatch (`adapters/catalog/src/lib.rs:661-671`); `feed` read only from `operations_file` (`local.rs:728-744`) — cited, walked not run
- **Sandbox:** evidence only, no provisioning in the repository: `docs/evidence/gitlab-sandbox-20260913/README.md:8-20` (container `connectors-gitlab-20260912`, `https://localhost:8929/api/v4`) — cited; whether it still runs — not checked
- **Confidence:** high for the binding files, medium for the engine lines, low for the journey routes.
- **Would collide with:** `story:feed-profile-capabilities` (conformance harness, `feed.rs`, `feed.yaml`, §3.3) — lands after it; `story:catalog-feed-engine-extensions` (`feed.rs`, §3.3); any unit editing `providers/gitlab/operations.json` or `adapters/catalog/tests/local_runtime.rs`.
- **Safety fact:** `feed` enters the effective configuration only when declared (`local.rs:858-861`), so only GitLab's revision moves; a connection saved under the old revision is refused `Conflict` until upgraded — cited, unproven at runtime.
