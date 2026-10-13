---
format: aep.planning-md/3
id: decision-blocker:platform-custody-limits
kind: decision-blocker
status: cleared
title: Nobody has decided how Windows stores values over 2,560 bytes or how macOS items survive an upgrade
relations:
- blocks: story:platform-credential-custody
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-12T23:09:29Z", actor: "human:timo", revision: 4}
---
## Question

Two design points of `story:platform-credential-custody` that the first build showed do not hold.

### 1. Windows: the 2,560-byte bound refuses almost every shipped profile

A Credential Manager blob holds at most 2,560 bytes. Measured in
`crates/connectors-custody/src/tests.rs` (every entry field at its declared maximum): token
profiles 8,204 bytes, basic 9,241, OAuth refresh 24,630, OAuth client credentials 16,419, SQL
password 8,207; only Tavily's API key (526) and the approval signing key (32) fit. Typical real
tokens are smaller than the declared maximum, so a refusal would depend on the token's length.

| option | what | cost |
|---|---|---|
| A | Split a value over numbered Credential Manager entries of one version, written first, then one header entry naming their count and digest; a read needs the header and every part, a delete removes the header first. Values stay in the Credential Manager; the 64 KiB bound holds. | One more write path and its crash tests (parts without a header are orphans, reclaimed like any unacknowledged version). |
| B | Keep one entry and refuse over 2,560 bytes. | No work. Whether a Windows user can save a connection depends on the token's length. |
| C | Store values over 2,560 bytes as a DPAPI-protected (user scope) owner-only file. | Not plain text, but a second store and a file, which the contract excludes today. |

Recommended: A.

### 2. macOS: a new binary may lose access to existing items

An item created through the default keychain access is trusted for the application that created it.
With user interaction disallowed, a different (upgraded, unsigned) `connectors` binary is expected
to get `Unavailable` on read instead of a prompt: inferred from the keychain's access model, not
yet observed on a runner.

| option | what | cost |
|---|---|---|
| A | Create items with an access that admits any application of the user, as the Secret Service does on Linux; a locked keychain is still refused. | Any process of the user can read an item while the keychain is unlocked, the same exposure as on Linux. |
| B | Keep per-application trust; after an upgrade every connection needs its credential entered again. | Credential re-entry after every upgrade. |
| C | Keep per-application trust and allow the system prompt on the first read after an upgrade. | Breaks "never prompts", and blocks an unattended caller. |

Recommended: A, with a CI test on the runner that a second binary reads an item the first created.

Default on silence: 1A and 2A.

## Decided

Part 1, option A, 2026-10-12: a Windows value over 2,560 bytes is split over numbered Credential Manager entries of its version, written first, then one header entry naming their count and digest, written last. A read needs the header and every part; a delete removes the header first. The 64 KiB bound holds on every platform. Crash tests cover parts without a header, which are orphans reclaimed like any unacknowledged version.

Part 2, option A, 2026-10-13: macOS keychain items are created with an access that admits any application of the user, the same exposure as the Secret Service on Linux; a locked keychain is still refused without a prompt. The CI runner test in which a second binary reads an item the first created stays, and fails when the reader is refused.
