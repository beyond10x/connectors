# Read refresh retry: pre-correction textual cases

Story: contracts-read-refresh-retry. Findings: F15 and E05. Baseline: 8e1836cad8ae1b2127ce9ae306c6d8131960db4c. This record was written before changing the scoped contracts. It is a manual textual audit, not an executed runtime test suite.

The source findings are retained in `.engineering/planning/review-result/contract-semantics-20260908.md` (F15) and `contract-docs-external-20260908.md` (row 5/E05). Capability §4 now conditions read redispatch on a separately selected supported binding, but that binding is absent; acquisition §4.1 explicitly leaves the read decision unsettled. The existing service still forbids automatic retries. All three prerequisite stories are implemented in this baseline.

| Case | Concrete observation to establish | Baseline textual result |
|---|---|---|
| RR01 | Legacy provider 401 yields its documented error after one business call, with no refresh/redispatch caused by this invocation | Preserved service rule; capability's relationship to legacy needs explicit disposition |
| RR02 | An explicitly selected extended unary read receives 401, refreshes once, then succeeds on one redispatch | Missing selected binding and exact advertisement/selection rule |
| RR03 | Redispatched read returns 401 again | Missing terminal rule; generic 'at most one' prose is not a complete sequence |
| RR04 | First response is 403, 429, 5xx, timeout, malformed or oversized | Existing no-retry baseline; no explicit new-binding trigger boundary |
| RR05 | Mutation, anonymous, static configuration, mediated or streaming operation attempts to select retry | Partial exclusions exist; no closed eligibility rule for the new binding |
| RR06 | Initial work consumes most of the original deadline or byte/call allowance | Missing concrete shared deadline and count/byte bounds for the proposed sequence |
| RR07 | Permission check for target X consumed its slot before the first read; refresh invalidates that evidence | Existing F08 limits must survive; no retry-binding rule defines the resulting refusal |
| RR08 | Refresh narrows grants, changes identity, loses custody, becomes uncertain or requires reauthorization | Private outcomes exist; no complete retry-invocation public outcome mapping |
| RR09 | Two invocations receive 401 for the same generation | Existing refresh ledger prevents a second authorized exchange; waiter/redispatch composition missing |
| RR10 | Publication, revocation or policy withdrawal races the fresh dispatch | Existing generation-admission fences apply; retry invocation does not yet bind them explicitly |
| RR11 | Caller cancellation or timeout precedes refresh publication | Missing terminal invocation rule; existing refresh recovery must not revive business work |
| RR12 | Publication from the source generation already happened, or its successor is superseded again | Missing bounded one-generation-step selection and no-chain rule |
| RR13 | Successful redispatch returns 404 or loses its final audit acknowledgement | Existing outcome/audit rules apply; no explicit retry composition |
| RR14 | A client sees unchanged native profile metadata or an unsupported retry profile | No concrete new-profile identity rule; old behavior must not change silently |

Result: the new binding's complete sequence cannot be derived from the baseline. These are specification gaps, not fourteen measured runtime failures. The revision must supply a textual observation/refusal for all fourteen cases while retaining the existing refresh and evidence model contracts.
