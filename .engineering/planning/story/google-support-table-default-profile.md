---
format: aep.planning-md/3
id: story:google-support-table-default-profile
kind: story
status: draft
title: The Google support table names the default read-only profile
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 11

11. The Google support table omits the default profile. It describes the valid write-capable google.oauth profile for Gmail, Drive and Calendar. The no-profile quick start selects google.oauth.readonly. An operator reading the table can therefore expect operations their new instance does not offer. The individual Gmail guide already states both choices.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/README.md#L75
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/README.md#L254
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/CHANGELOG.md#L227
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/website/docs/connectors/gmail.md#L52

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- The table names `google.oauth.readonly` as the no-profile default beside `google.oauth`, as the Gmail guide does.
