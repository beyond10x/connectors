# Supported-vocabulary disposition audit

This is a specification-only audit for E27. It distinguishes selected support
from names retained for historical translation or future work. It does not
execute an adapter, descriptor reader, provider request, or runtime refusal.

## Failing baseline at `8e1836c`

| Case | Required result | Baseline observation | Result |
|---|---|---|---|
| SV-01 | `http_signing` has an explicit supported, reserved/refused, or removed disposition | The name appeared in the general scheme/capability lists although only compatibility prose called it reserved | fail |
| SV-02 | `oauth2_password` cannot be selected or advertised | The profile prose called it reserved/refused, but there was no consolidated disposition beside the closed vocabulary | fail |
| SV-03 | `hold` and `transfer` cannot be negotiated or advertised by the selected media profile | They were called reserved/not promised without an explicit refusal rule | fail |
| SV-04 | `promql-instant` and `promql-labels` cannot be advertised as implemented profiles | The Prometheus binding called them reserved without an explicit descriptor/refusal rule | fail |
| SV-05 | Resource discovery has one explicit disposition for the `address` locator | `address` remained in `locator.kind` as a reserved value although `endpoint_discovery` owns address observations | fail |

The corrected audit records the evidence, disposition, selection behavior and
advertisement behavior for each name after the textual contract edits.

## Corrected textual audit

| Name(s) | Disposition and rationale | Selection rule | Status |
|---|---|---|---|
| `http_signing` | reserved/refused; historical Twilio, Slack and Stripe declarations do not supply a selected current canonicalization/profile binding | cannot be selected or advertised | pass after coordinator capability patch |
| `oauth2_password` | reserved/refused; the historical Babelforce-only value has no selected acquisition flow or current provider binding | cannot be selected or advertised | pass |
| `hold`, `transfer` | reserved/refused; no selected media/SIP state, admission, target or failure semantics exist | cannot be advertised, negotiated or accepted | pass |
| `promql-instant`, `promql-labels` | Prometheus-native reserved/refused values; only `promql-range` has a selected source operation and complete profile contract | cannot be selected or advertised | pass |
| `address` resource locator | removed/refused; `endpoint_discovery` owns address/port observations while resource discovery accepts opaque locators | cannot be emitted or selected as resource-discovery vocabulary | pass |

The refusal is explicit: a reserved or removed name never gains support by
appearing in an enum, historical provider declaration or incoming descriptor.
Unknown or unsupported selection attempts fail before provider use.

This audit checks documented vocabulary and refusal semantics. It does not claim
runtime descriptor filtering or provider conformance.
