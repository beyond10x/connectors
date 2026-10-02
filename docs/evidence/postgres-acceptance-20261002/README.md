# PostgreSQL acceptance — 2026-10-02

Six explicitly selected live cases passed against the owned PostgreSQL 17.6
loopback fixture, including the strengthened existing restart journey. The final
run completed at 15:55:46 UTC in 58.17 seconds. The ordinary SQL package separately
ran 19 cases successfully and left six live cases ignored. Ignored cases are not
provider acceptance. Cross-review and local integration are complete; the combined
repository gate and release remain separate steps.

| Observation | Retained result |
| --- | --- |
| Live database: five CLI cases and one native invocation-drop case | [6 passed, 0 failed](final-live.log) |
| SQL package | [19 passed, 6 ignored](final-package.log) |
| Missing prerequisite control | [1 selected case refused, exit 101](missing-prerequisite.log) |
| Exact SQLSTATE regression before correction | [0A000 returned Unavailable instead of Unsupported, exit 101](sqlstate-existing-regression-red.log) |
| Classification diagnosis | [Reader-role prepare and healthy follow-up](sqlstate-diagnostic.log) |
| Clippy / formatting | [Clippy](clippy.log), [formatting](fmt.log), both exit 0 |
| Fixture cleanup | [No acceptance schemas or marked reader backends](fixture-cleanup.log) |

The live cases cover exact joins/grouping and UTC boundaries, quoted parameters,
empty-result metadata, truncation, capacity and timeout refusals, read-only escape
attempts with independently unchanged rows, native invocation-drop cancellation,
failed repair, revocation and busy child stop. The cancellation control remained
active for 5,045 ms; the dropped invocation's marked backend stopped executing
44 ms after the drop. These are fixture observations, not a universal remote
cancellation deadline. Busy stop took 414 ms.

The production correction maps exact PostgreSQL SQLSTATE `0A000` to the existing
`Unsupported` outcome. The complete wire matrix was observed red before the
one-line change, then green. Other mappings, sanitization and upstream-answer
semantics remain covered. The temporary diagnostic was removed; its retained
[patch](sqlstate-diagnostic.patch) is evidence, not shipped implementation.

The worker source base is `f3fb222b7edc7fc29520dd30effdb58bfdec5274`, with frozen
input patch SHA-256 `c583e40f88483081519eb9c192c159807341b0030f38c227b3ef027f585a9665`.
Final source and binary identities are retained in [source hashes](final-source.sha256)
and [binary hashes](final-identities.sha256). The exact container/image is recorded
in [fixture identity](fixture-identity.txt); [toolchain](toolchain.txt) records the
compiler inputs. This evidence does not claim a workspace/MSRV gate, TLS for the
plaintext fixture, MySQL, the sustained store-cost milestone or MCP delivery.

Published copies replace the local home prefix with literal `$HOME`; raw evidence
is retained privately. [Original input hashes](original-inputs.sha256) describe
the original bytes, before that path redaction. They are provenance records, not
checksums of the redacted copies. No credentials were used in command arguments.

The subsequent [adversary pass](review.md) added one protocol case distinguishing
exact `0A000` from custom neighboring codes. Its [focused run](review-focused.log)
passed, followed by [20 ordinary package cases](review-package.log), formatting
and Clippy. It found no defect and reused the six live-case results because their
production and fixture inputs were unchanged. The original source manifest above
identifies the author handoff, before this added protocol test. Reviewed source
commit: `c3e20e1df82ccd4bc33b25c2943d3059ae0b51e1`.
