# Independent Roughtime fixture

`roughtime-se-request.bin` and `roughtime-se-response.bin` are the exact UDP
datagrams observed from `roughtime.se` (`192.36.143.134:2002`) on 2026-09-10.
The public long-term Ed25519 key is
`S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=`; the service publishes its key at
<https://roughtime.se/>. These packets contain public time and a random request
nonce, no credentials or private signing material.

The independent roughenough-client 2.0.0 Rust 1.88 probe verified MIDP=1789076475
and RADI=1 second, with an observed 86 ms exchange. The request/response bytes were
captured by its transport before parsing. The production Connectors verifier
tests these same original bytes. Full probe sources, lockfile, logs and hashes
are retained with the local-clock evidence receipt. A historic packet validates
the codec; it cannot supply a fresh clock capability.

`server.rs` is a separate test-only Rust encoder/signing fixture using ring.
It supplies selected synthetic timestamps for deterministic failures and CLI
tests. It calls no production framing or verification helpers. Its two constant
seeds are public test material, never production credentials or actual UTC proof.
