# Loki source supplement — 2026-09-09

This append-only record fills the parser-source gap in the original datasource
intake. [provider-source-hashes.json](provider-source-hashes.json) records exact
original URLs, uncompressed SHA-256 values and byte lengths. Each `vendor/*.gz`
archive was created with `gzip -n`; decompression was compared byte-for-byte with
the read-only source cache, and its original manifest digest was checked.

Both files come from Loki commit
`3361de24b692875d77bd7433cd6baa7c68dc0ef9` (v3.7.0). In `logql-ast.go`, lines
72, 327 and 370 distinguish the LogSelectorExpr interface, MatchersExpr and
PipelineExpr; lines 2148/2182 and 2366/2409 show why literal/vector interface
membership is insufficient. In `logql-parser.go`, lines 74 and 261 establish the
general expression and log-selector entry points. These sources support the
selected root distinction in [§4.2](../../semantics.md#42-native-logql-scope).

The earlier HTTP/LogCLI source record remains in the immutable
[datasource evidence intake](../../../../../../../docs/evidence/datasource-semantics-20260908/provider-evidence.md).
Its HTTP interval/count evidence and LogCLI overlap refusal justify the bounded
observation selection; they do not supply an occurrence cursor.

These are retained research bytes, not an adopted upstream build dependency,
completed source/license packaging, local parser implementation or live-provider
conformance result. No source was refetched or modified in this supplement.
