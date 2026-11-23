---
format: aep.planning-md/3
id: story:thalamus-invoke-on-0-74
kind: story
status: draft
title: 'Thalamus source reads use invoke <connector>/<instance> <operation> (consumer: beyond10x/thalamus)'
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 5

5. Thalamus source reads use a command that 0.74.1 refuses. Thalamus builds operations invoke and defaults to a Connectors binary on PATH. The release rejected that command form at argument parsing, exit 2. Its supported form is invoke <connector>/<instance> <operation>. The Thalamus test double still accepts the old spelling. Update the caller and exercise the released command in its compatibility check.
https://github.com/beyond10x/thalamus/blob/8bbdef6f9c2fac30673ec89eea1137c32e8fa328/crates/thalamus-sources/src/connectors.rs#L288
https://github.com/beyond10x/thalamus/blob/8bbdef6f9c2fac30673ec89eea1137c32e8fa328/crates/thalamus-sources/src/bin/connectors-double.rs#L77
https://github.com/beyond10x/thalamus/blob/8bbdef6f9c2fac30673ec89eea1137c32e8fa328/website/docs/getting-started.md#L73
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/CHANGELOG.md#L1011

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- In beyond10x/thalamus, source reads call the 0.74 form, and the test double refuses the removed `operations invoke` spelling.
- Owned by the thalamus repository; this story tracks it until a change there lands.
