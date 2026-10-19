---
format: aep.planning-md/3
id: review-result:wave-20261019-night-b4-adversary
kind: review-result
status: active
title: Adversary pass on the Slack one-user reads and file delete
relations:
- reviews: story:slack-single-user-lookups
- reviews: story:parity-slack-file-delete
revision: 1
---
**Verdict: two CONFIRMED findings; the introduced one fixed in `6b4aa2516`.**

Adversary (opus) on `3d1b1a354`, wave 20261019-night. 11 cases in
`adapters/catalog/tests/slack_adversary.rs`, 9 pass: the delete is sent only when `files.info`
names the same file and answers ok:true; file ids and e-mail addresses reach Slack as themselves;
answers outside the envelope are never data; the token travels only in the Authorization header.
Red 1 (introduced, warning): `docs/catalog-slack.md:450` said the one-user reads were selected on
neither connection; fixed. Red 2 (pre-existing, note): a required parameter given as `""` is
sent (`adapters/catalog/src/lib.rs:1259`); `#[ignore]`, story:required-parameter-refuses-empty.
