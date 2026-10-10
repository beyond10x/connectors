---
format: aep.planning-md/3
id: review-result:adversary-parity-gitlab-tags-releases-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: parity-gitlab-tags-releases'
relations:
- reviews: story:parity-gitlab-tags-releases
revision: 1
---
unit: story:parity-gitlab-tags-releases, uncommitted on wave/20261010c at f9bfb15e9
verdict: 1 warning (fixed in the description and guide), 1 note (unreachable), no failing test
cases: executed 539→539 (provider) + 4 (catalog_guard_model), red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none

## Findings

1. warning, contract drift, `adapters/catalog/providers/gitlab/operations.json` (`tag.delete`):
   a tag deleted and recreated at another commit between the guard's read and the DELETE is
   deleted and still reported applied; a check after the delete cannot see it and GitLab has no
   conditional delete. Unlike `tag.create`, the description and guide did not name the race.
   Fixed: both now say it.
2. note, `adapters/catalog/src/lib.rs` (`postflight.absent`): the read after a delete accepts any
   404 without reading its body, so a "404 Project Not Found" would also count as gone. No path
   reaches it: the same token just received the 204, and redirects are off in the host transport.
   Left as is.

## Attacked without a finding

- `absent` loading: `false`, `null`, without `read`, beside `checks` or `any_of` are refused at
  load; the model's rules match the engine's.
- After the delete only 404 counts; 2xx, 401, 403, 410, 500, 3xx and transport errors are
  unknown. The suite covered only 500; cases for 403 and 410 were added to
  `adapters/catalog/tests/gitlab_tags_releases.rs` after this pass.
- A 404 on the read before the write is refused before anything is sent.
- Tag names: `/` is sent as `%2F`, `%` as `%25`, bare `.`/`..` segments are refused; the reads
  and the write build the same path from the same input.
- `tag.create`: a branch moved before the create is unknown; a short or uppercase sha is refused.
- `release.create`/`release.update`: `ref`, `tag_message` and `tag_name` in the update body are
  refused; a moved tag is unknown because the commit is checked after the write.
- Existing guards behave as before (`absent: None` takes the old path).
