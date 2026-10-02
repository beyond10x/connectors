# Ignored-suite runner verification

The new command's package passes 151 cases. Its first real disposable-family run
is intentionally reported as unsuccessful: 31 cases passed, one existing Chrome
connection-count assertion failed, and one historical pre-handshake binary was
missing. The report accounts for all 45 inventoried ignored cases. This is proof
of actual execution and failure reporting, not a claim of an all-green ignored suite.

- [Implementor report and original red/final green output](implementor.md)
- [Final independent review](adversary-2.md)
- [Every operator-run case and disposition](operator-disposable.json)
- [Unchanged browser assertion failure](browser-failure.log)

The first independent review found child descendants surviving both successful and
failed test parents. The final implementation retains the group leader until it
signals the group, then reaps the exact owned children with a bounded wait. Its
three retained adversarial tests pass. Deliberately detached sessions remain each
fixture's documented shutdown responsibility.

The final review's isolated SIGCHLD diagnostic is not a supported public invocation:
the real CLI rejects that inherited disposition at Cargo metadata, before fixture
dispatch. The diagnostic source and outputs are preserved privately with the raw
review. Only that unsupported acceptance assertion was removed from shipped tests,
by coordinator direction; no production fix or third adversary was requested.

Publication files have only the home-prefix redaction disclosed in the parent
receipt. Raw implementor SHA256:
`306f3c867cd9a9c8a2a69095b0e083eaa0f1ef649fab0307f5e40f6059649ed3`;
publication SHA256:
`f0dde6123153f3106eac5f91ba847e6b364b5038348f25c07c89c71e3e5d1c2b`.
Raw final review SHA256:
`cddf210df96da93780e566c2b129f99b899e0fab53ed792458c3e4b031632222`;
publication SHA256:
`6af642f3613c9d48e047a2b572eab0a73de11d5be5d7d52bc070bc6f1f4f55ff`.
