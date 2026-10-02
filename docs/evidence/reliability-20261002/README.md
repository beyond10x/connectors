# Reliability batch verification

The bridge and SQL unit reports establish their targeted acceptance and retain
independent adversarial cases. The ignored-suite runner, final integrated gate,
MSRV checks and source release are pending. No published release is claimed here.

- [Bridge ownership results](bridge/README.md)
- [SQL cancellation and 50-run results](sql/README.md)

## Publication redaction

Raw reports remain private. Publication copies replace the operator home prefix
with the literal `$HOME` marker; no test result, assertion, timing or verdict is
changed. Commands remain readable with the portable path. The aggregate SQL log
has one terminal blank line removed. Reports whose text says “verbatim” are
verbatim apart from this declared personal-path redaction. Gates refused the raw
copies before any commit or publication; its rules were not relaxed.

| Report | Raw SHA256 | Publication SHA256 |
| --- | --- | --- |
| bridge implementor | `9e842d81ceb24b425a6dc52e4b87505c9864856e47b652eb8bb715f7af40273e` | `befcde5ced459ed46bd3a68b60ff0e9a8aec7b51f16a63579aa09e55aef91522` |
| bridge adversary | `e4d7d3bb5ebb8bfb868674c4ffb5b7dd08443b550d6c82a137e4f70a6913d032` | `0b44df9b1977f9159b6c281c9a75186303f3bc248860ef112a8f6f4ce55262e4` |
| sql implementor | `ca7f21abf5c403108fda02c3d270452914ece7bc16f10ce4a413b997a6c0e9e0` | `63a6525afa335e33de5f29b1f6ff0c4155fa375e04c1b4514319c65967a2f9d7` |
| sql adversary | `58dc27b410a464a6298d7d7a72ae8fdce0a41a6a9b3cf3177aa512565442d25b` | `dd06ced2a0c62ffc8b37f243fc49af8e3da45ee9a983428cf613d017ba8c23a1` |
