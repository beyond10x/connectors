# Documentation-index dependency audit

This evidence prepares E22 after the media semantics settle. Root indexes and
`docs/design.md` are coordinator-owned; this worker records the source matrix
and supplies an unapplied patch rather than editing those files.

## Baseline omissions at `8e1836c`

| Index row | Dependencies declared by the adapter documents but absent from `contracts/README.md` |
|---|---|
| Kubernetes | `auth.connection`, `auth.custody` |
| Docker | `auth.connection`, `auth.custody` |
| Grafana composition | `auth.custody` |
| Media composition | `datasource.records` for `sip.sessions.list`; `auth.acquisition` for SIP `static_entry` |

## Independently sourced dependency matrix

| Root index row | Missing dependency | Native declaration that requires it | Coordinator correction |
|---|---|---|---|
| Kubernetes | `auth.connection`, `auth.custody` | `adapters/kubernetes/design.md` §3 declares configured cluster connection and read-only token/certificate custody | add both families |
| Docker | `auth.connection`, `auth.custody` | `adapters/docker/design.md` §3 declares configured daemon connection and read-only certificate/key custody | add both families |
| Grafana/monitoring | `auth.custody` | `adapters/grafana/design.md` declares read-only/versioned credential custody | add the family |
| Media | `datasource.records` | `adapters/sip/design.md` declares `sip.sessions.list` as a bounded records read | add the family |
| Media | `auth.acquisition` | `adapters/sip/design.md` declares `static_entry` for protected trunk credential entry | add the family |

RTVBP one-shot redemption is independently sourced from
`adapters/rtvbp/design.md` and its native session semantics as
`inbound-verifier` capability state. It is not an eighth `auth.evidence` name.

## Corrected textual audit

The owned native and composition documents now agree on operation, records,
duplex-message, capability and evidence classification. The exact shared-index
changes remain in the coordinator patch because `contracts/README.md` is a
reserved file. The final repository-wide index result therefore depends on that
patch being applied after the media semantics dependency closes.

This is a documentation dependency audit. It does not claim implementation or
runtime coverage.
