---
format: aep.planning-md/3
id: review-result:adversary-catalog-binary-responses-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: catalog-binary-responses, parity-slack-file-reads, parity-jira-attachment-download'
relations:
- reviews: story:catalog-binary-responses
- reviews: story:parity-slack-file-reads
- reviews: story:parity-jira-attachment-download
revision: 1
---
unit: story:catalog-binary-responses with story:parity-slack-file-reads and story:parity-jira-attachment-download, ecee52705 on wave/20261010e
verdict: 2 warnings and 3 notes, fixed in 7f448f4f84; 1 note on documentation, fixed by the wave's docs commit
cases: executed 573→584, red 5
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: none

## Findings

1. warning, boundary, `adapters/catalog/src/lib.rs` (`read_download`): a download segment decoding to `../../api/auth.test` passed the raw-path prefix check and was sent with the credential as `..%2F..%2Fapi%2Fauth.test`. Case `a_download_segment_with_an_encoded_slash_is_refused`. Fixed: a decoded segment holding `/` or `\` is refused as forbidden with nothing sent, for redirect targets too.
2. warning, acceptance, `read_binary`: a body with `Content-Encoding: gzip` was answered as the Content-Type's media type. Case `a_content_coded_body_is_not_reported_as_the_media_type_it_encodes`. Fixed: a coded body is answered as `application/octet-stream` (the case `the_bound_holds_on_the_wire_whatever_the_framing` requires a coded body to succeed).
3. note, boundary: a repeated Content-Type joined by the transport was answered as one media type. Fixed: refused.
4. note, contract drift: `binary.hosts: []` loaded though the model requires at least one. Fixed: refused at load. The connection-level `hosts: []` still means the same as no `hosts` (nothing followed), as `the_configuration_revision_follows_hosts` asserts.
5. note, boundary: `origin()` accepted `https://..`, `https://a..b`, `https://.`, `https://-`. Fixed: real hostnames or IP literals only.
6. note, contract drift: the owner document and the Slack guide did not name the binary members at ecee52705. Fixed by the docs commit.

## Attacked without a finding

- Redirect origin comparison (case, default port, trailing dot, userinfo, IDN, `..`, relative and protocol-relative Location, scheme downgrade, IP literals); the redirect bound is exactly three.
- The size bound on the wire for close-delimited, chunked and oversized bodies; a long Content-Length is an error.
- No credential or cookie on a redirect hop; the configuration revision follows `hosts` and its credential flag.
