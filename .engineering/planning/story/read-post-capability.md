---
format: aep.planning-md/3
id: story:read-post-capability
kind: story
status: active
title: An adapter sends a read as a POST without a write approval
relations:
- decomposes: epic:generic-websearch
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T02:15:49Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-05T02:15:49Z", actor: "agent:claude", revision: 3}
---
## Outcome

`AuthenticatedHttp::post_json(segments, query, body)` sends one POST with a JSON body as a read.
Its default answers `unavailable` without I/O, as `get_prefix` does. `ScopedHttp` implements it with
the authority, deadline, redirect refusal and response bound of `get`, and caps the request body at
64 KiB. `contracts/auth/capability/v1alpha1/semantics.md` records the addition.

## Acceptance

- An adapter reaches `post_json` only for an operation its descriptor declares a read sent as a POST;
  a test drives an undeclared operation and observes the refusal with no request sent.
- A body over 64 KiB is refused with no request sent.
- A redirect answer is not followed; the response bound of `get` holds.
- The credential never appears in an error, a log line or a debug rendering.
