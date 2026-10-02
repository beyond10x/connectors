---
format: aep.planning-md/3
id: verification-report:registry-clock-mitigation-decision-20261002
kind: verification-report
status: accepted
title: Reject the measured clock CAS suppression candidate
relations:
- verifies: story:registry-clock-outside-shared-batches
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-10-02T15:17:51Z", actor: "human:timo", revision: 2}
---
## Decision

Reject the single registry-clock CAS suppression candidate. The bounded experiment
is complete; no candidate production code is adopted. This is the
registry-clock-mitigation-decision acceptance record for
story:registry-clock-outside-shared-batches.

## Measured evidence

Source base 26929027f85b4653d81bd9cb290a7f472cfe84e8; Entity Runtime 0.25.1;
release profile, Rust 1.98.1, two Cargo jobs. Full public evidence and exact logs:
docs/evidence/clock-experiment-20261002/README.md. Original frozen worker report
SHA256 99fe25a55f592f7381bd40e6fb430383ccad596de57cb41f1fd82e14e6b132f7.

Baseline invariants executed9/passed9. Candidate executed9/passed8/failed1:
prepared_same_millisecond_observation_refuses_stale_registry_state returned
MetadataUnavailable instead of required ConcurrentRevision, process exit101.
This is a lost definite revision conflict; no stale successful response is claimed.
Other floor/race/corruption/postcommit cases passed. Both cost probes completed
all requested rows and exited0 on each profile. All three warmups and fifteen
measured invokes per profile succeeded; no caught panic is hidden by libtest.

Actual event counts55/601/1203. Baseline wall medians545/13805/31349ms;
candidate356/4410/9227ms. CPU medians baseline474/13559/30918ms;
candidate316/4174/9225ms. Candidate largest/smallest ratio25.9185 exceeds2;
baseline57.5211. Full batch diagnostic numbers and immutable fixture identities
are in the public report. Original saved snapshots were generated once by baseline
and reused byte-identically. Retained creation/copy manifests document the real
sequence; no nonexistent pre-generation full manifest is claimed.

## Restoration and limits

The sole condition change was reversed. Tracked status/diff are empty; er.rs
hash847e76669449d6df66c33ec8284d7a620f7b52c78a6a9a160155aa262f926415 matches
baseline. Cargo.lock and all other assigned source hashes match. Formatting and
diff checks exited0. No full package/Clippy rerun is claimed after exact restoration.
Verified v0.25.0 bridge evidence is reused for identical inputs. The unchanged
30-second bound and blob verification remain in force. No second candidate.

Unrelated host load prevents a controlled attribution of timing improvement; the
actual invariant failure and adoption-threshold failure both select rejection.
Entity Runtime issue51 was independently read through GitHub and remains OPEN.
The M1 600/6000-event and roughly700-invoke sustained acceptance remains pending.
This decision neither closes it nor proves every alternative design impossible.
