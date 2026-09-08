# Kubernetes log source supplement — 2026-09-09

[provider-source-hashes.json](provider-source-hashes.json) retains the exact URL,
uncompressed SHA-256 and byte length of core/v1/types.go from Kubernetes v1.35.0,
commit `66452049f3d692768c39c797b21b793dce80314e`. The new `gzip -n` archive was
decompressed and compared byte-for-byte with the read-only research cache; its
original manifest hash was independently checked.

PodLogOptions at source lines 7169–7222 supplies the native input evidence:
relative sinceSeconds (7186), timestamps (7196), tailLines and combined-stream
restriction (7199–7201), approximate/split-line limitBytes (7202–7206), and the
backend TLS bypass option the selected binding refuses (7207–7215). These are
provider declarations, not evidence that a particular cluster implements them.

The [native contract](../../semantics.md) deliberately omits limitBytes, enforces
its own exact finite text cutoff and preserves provider order/null timestamps.
This is research provenance, not completed upstream source/license adoption or
an implemented pod-log reader. The original review evidence remains immutable.
