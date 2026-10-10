---
format: aep.planning-md/3
id: review-result:adversary-parity-gitlab-ci-project-writes-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: parity-gitlab-ci-writes and parity-gitlab-project-writes'
relations:
- reviews: story:parity-gitlab-ci-writes
- reviews: story:parity-gitlab-project-writes
revision: 1
---
unit: story:parity-gitlab-ci-writes and story:parity-gitlab-project-writes, c31f52e1a and f3ce5e694 on wave/20261010d
verdict: 1 warning (fixed in e5e3d822b), no other finding
cases: executed 13→14, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none

## Findings

1. warning, acceptance, `adapters/catalog/providers/gitlab/operations.json` (`file.update`): the
   write was proven by reading `commits/{branch}`, which GitLab resolves like `git rev-parse`, a
   tag of the same name before the branch. A write that landed on a branch moved between the check
   and the write was reported applied instead of unknown when such a tag pointed at a commit whose
   parent is the pinned sha. Case `adversary_file_update_proof_is_not_satisfied_by_a_same_named_tag`
   in `adapters/catalog/tests/gitlab_ci_project_writes_adversary.rs`. Fixed in e5e3d822b: the read
   after the write is the branch itself, checking `/commit/parent_ids/0`.

## Judgement notes, no test

- `commit.create` and `file.update` accept `author_email` and `author_name`, so a caller sets the
  commit author; `project.create` accepts any `visibility`, `public` included. Both are GitLab's
  declared fields and stay admitted.
- `pipeline.cancel` and `pipeline.retry` on a pipeline with nothing to cancel or retry are reported
  applied; the descriptions say so.

## Attacked without a finding

- Branch, file and namespace names with `/` are sent as one encoded segment; `.` and `..` refused.
- Every value a guard reads is required; a missing one sends nothing.
- A sha race on `commit.create`, `pipeline.create` and `branch.create`, and a tag where
  `commit.create` or `file.update` needs a branch, are refused or reported unknown.
- `branch.delete`: only a 404 on the read after counts as deleted.
- A namespace id/path mismatch on `project.create` is refused.
- Closed bodies: `force`, `start_*` and `import_url` are refused; nested `actions` and `variables`
  hold only fields GitLab declares.
- `correct_media_type`: a second change, a `to` other than `application/json`, a `from` the source
  does not declare, and an operation declaring none or several types are refused.
- `environments.list`: `per_page` is bounded to 1..=100.
