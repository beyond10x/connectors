# Evidence records

This directory held the dated evidence records of this repository, written between
2026-09-08 and 2026-10-02: review records and independent review reports, gate and test
logs, live acceptance results, specification snapshots and their verification notes, and
provider-source hash manifests (`provider-source-hashes.json`). It held 1,364 files.

The records are no longer in the working tree. Their last state is the branch
`history/evidence-2026-10` of this repository, which points at commit `6498a881c`.

Read a record without checking the branch out:

```sh
git fetch origin history/evidence-2026-10:history/evidence-2026-10
git show history/evidence-2026-10:docs/evidence/<path>
git ls-tree -r --name-only history/evidence-2026-10 -- docs/evidence/<directory>
```

On GitHub the same files are under
<https://github.com/beyond10x/connectors/tree/history/evidence-2026-10/docs/evidence>.

A reference to `docs/evidence/...` in the planning store, the changelog or an older commit
names a path on that branch.
