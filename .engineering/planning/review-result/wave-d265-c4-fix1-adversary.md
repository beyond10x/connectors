---
format: aep.planning-md/3
id: review-result:wave-d265-c4-fix1-adversary
kind: review-result
status: active
title: Adversary pass on the nested name-check walk
summary: 1 note, closed by an added case
relations:
- reviews: story:catalog-name-check-reads-nested-sources
revision: 1
---
# Adversary pass on the nested name-check walk

```findings
[
{"file":"crates/connectors-build/src/catalog_feed_conformance.rs","line":1212,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"no test asserted the suite adapter is in the name-check source list, so dropping it stayed green; closed by the added case adversary_the_name_check_still_reads_the_suite_adapter_and_both_levels"}
]
```
