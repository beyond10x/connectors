# MP-B-C01 — Known session termination is mislabeled as session loss

Severity: P2. Sole owner: story:contracts-mutation-classification. Status: open in this frozen recheck packet.

The new SIP outcome table, docs/adapters/media-session.md:85, selects session_lost for every terminal observed after readiness and before result encoding. classification-verification.md MC18 repeats the unconditional mapping. The trigger includes confirmed local close, remote hangup, revocation and lease expiry, not only loss of continuity.

Sessions defines lost as continuity loss (contracts/sessions/v1alpha1/semantics.md:47), retains explicit first terminal reasons, and distinguishes confirmed closed/released resources from loss (section 4.1). Conflating these states at the new mutation delivery boundary loses a known cause despite preserving applied effect knowledge.

Correction: retain applied and withhold the usable handle; select the applicable existing session error for the known fact (for example revoked, lease_expired or session_not_ready), and select session_lost only for actual continuity loss. Keep the original first terminal fact and the ready receipt unchanged. Update the textual MC18 expectation accordingly. This is a bounded error mapping correction, not a new code or runtime implementation requirement.
