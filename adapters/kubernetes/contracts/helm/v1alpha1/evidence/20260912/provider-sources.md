# Helm release storage sources — 2026-09-12

[provider-source-hashes.json](provider-source-hashes.json) retains the exact URL,
uncompressed SHA-256 and byte length of fourteen Helm source files: seven from
Helm `v4.3.0`, commit `bec5b06ed841fe5269972d864d5177944fd5970f`, and the seven
corresponding files from Helm `v3.22.0`, commit
`144ca65f8501953fa8b41cd1d37c7223051c85b7`. Each `vendor/*.go.gz` is a `gzip -n`
archive whose decompressed bytes were compared against the recorded digest and
length. Both commits were read from the tag list of `helm/helm` on 2026-09-12.

Two lines are pinned because a cluster's release Secrets may have been written by
either major version and the read must be true of both. Every fact below was
checked in both, and the two agree on every field this binding reads.

## Object name, type and data key

| Fact | Helm v4.3.0 | Helm v3.22.0 |
|---|---|---|
| storage-type prefix `sh.helm.release.v1` | `storage.go:38` | `storage.go:35` |
| object name `<prefix>.<release>.v<revision>` | `storage.go:327-328` | `storage.go:251-252` |
| Secret `type` is `helm.sh/release.v1` | `driver/secrets.go:284` | `driver/secrets.go:254` |
| Secret `metadata.name` is that key | `driver/secrets.go:281` | `driver/secrets.go:251` |
| payload lives under data key `release` | `driver/secrets.go:285`, read at `:79` | `driver/secrets.go:255`, read at `:75` |

## Labels

| Label | Helm v4.3.0 | Helm v3.22.0 |
|---|---|---|
| `name` — the release name | `driver/secrets.go:263` | `driver/secrets.go:233` |
| `owner` — constant `helm` | `driver/secrets.go:247,264` | `driver/secrets.go:217,234` |
| `status` — the release status string | `driver/secrets.go:265` | `driver/secrets.go:235` |
| `version` — decimal revision | `driver/secrets.go:266` | `driver/secrets.go:236` |
| `createdAt` — Unix seconds, set on create | `driver/secrets.go:174` | `driver/secrets.go:155` |
| `modifiedAt` — Unix seconds, set on update | `driver/secrets.go:205` | `driver/secrets.go:181` |
| the closed system-label set | `driver/util.go:34` | `driver/util.go:33` |

`createdAt` and `modifiedAt` are each written by exactly one code path, so a
revision Secret carries one of them or the other, never necessarily both.

## Selection Helm itself performs

| Selection | Helm v4.3.0 | Helm v3.22.0 |
|---|---|---|
| revision history is `{name, owner=helm}` | `storage.go:212-215` | `storage.go:153-156` |
| deployed revisions are `{name, owner=helm, status=deployed}` | `storage.go:193-200` | `storage.go:134-141` |
| more than one revision can carry `status=deployed` | `storage.go:171-175` | `storage.go:125-129` |
| whole-namespace listing is `owner=helm` alone | `driver/secrets.go:91` | `driver/secrets.go:84` |

The "Take the latest" comment states in Helm's own words that concurrent writes
corrupt the store into several deployed rows; Helm resolves it by sorting. That
resolution is Helm's, not a provider guarantee, which is why the native contract
reports every deployed row rather than silently choosing one.

## Encoding of the stored payload

| Step | Helm v4.3.0 | Helm v3.22.0 |
|---|---|---|
| release is JSON-marshalled | `driver/util.go:39` | `driver/util.go:38` |
| then gzip at best compression | `driver/util.go:44` | `driver/util.go:43` |
| then standard base64 | `driver/util.go:30,56` | `driver/util.go:29,52` |
| decode is base64 first | `driver/util.go:64` | `driver/util.go:60` |
| gzip is applied only when the magic `1f 8b 08` is present | `driver/util.go:32,72` | `driver/util.go:31,68` |

The Kubernetes API serialises `Secret.data` values as base64 of the stored bytes,
and the stored bytes are already Helm's base64 text, so a reader going through the
API base64-decodes twice before testing the gzip magic. That second decode is a
Kubernetes fact, not a Helm one; Helm's own driver receives the raw `[]byte`.

## Release fields this binding reads

| JSON field | Helm v4.3.0 | Helm v3.22.0 |
|---|---|---|
| `name` | `release.go:34` | `release.go:24` |
| `info` | `release.go:36` | `release.go:26` |
| `config` — the recorded values | `release.go:41` | `release.go:31` |
| `manifest` — the rendered manifest text | `release.go:43` | `release.go:33` |
| `version` — the revision number | `release.go:47` | `release.go:37` |
| `namespace` | `release.go:49` | `release.go:39` |
| `info.first_deployed` | `info.go:30` | `info.go:27` |
| `info.last_deployed` | `info.go:32` | `info.go:29` |
| `info.description` | `info.go:36` | `info.go:33` |
| `info.status` | `info.go:38` | `info.go:35` |

`labels` is explicitly excluded from the JSON body and lives only in the object's
metadata (`v4 release.go:52`, `v3 release.go:42`). `chart` and `hooks` exist
(`v4 release.go:38,45`; `v3 release.go:28,35`) and are deliberately not read.
`apply_method` (`v4 release.go:55`) and `info.rollback_revision` (`v4 info.go:40`)
exist only in the v4 line and are not read. `info.notes` (`v4 info.go:42`,
`v3 info.go:37`) and `info.resources` (`v4 info.go:44`, `v3 info.go:39`) are not
read; notes are rendered template output and resources are populated by a status
action rather than by the storage driver.

## Status values

The status vocabulary is a closed set of nine strings, identical in both lines:
`unknown`, `deployed`, `uninstalled`, `superseded`, `failed`, `uninstalling`,
`pending-install`, `pending-upgrade`, `pending-rollback`
(`v4 common/status.go:25-41`, `v3 status.go:25-41`).

## Release-name rule

A release name must match
`^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$` and be at most
53 bytes (`v4 chart/v2/util/validate_name.go:35,60`;
`v3 chartutil/validate_name.go:36,61`). The 53 is Helm's own reservation of ten
characters inside the 63-byte Kubernetes limit, explained at
`v4 validate_name.go:53-60`.

## What this evidence is not

This is research provenance for a native read binding. It is not upstream source
or license adoption, not a vendored Helm dependency, and not evidence that any
particular cluster stores releases in Secrets: the driver is selectable and
ConfigMap and SQL drivers exist beside it (`v4 driver/cfgmaps.go`,
`v4 driver/sql.go`, listed in the pinned tree but deliberately unpinned because
this binding reads neither). Nothing here establishes that a given credential may
read those Secrets.
