use super::*;

/// Checks every operation, and the auth it names.
pub(super) fn validate_operations(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for operation in &connector.operations {
        let id = operation.id.as_str();
        if id.trim().is_empty() {
            problems.push("an operation has an empty `id`".to_owned());
        } else if seen.contains(&id) {
            problems.push(format!(
                "operation {id:?} is declared more than once; the op id is the public name callers \
                 and models use, so it must be unique"
            ));
        }
        seen.push(id);

        validate_operation_service(connector, operation, problems);

        if operation.description.trim().is_empty() {
            problems.push(format!(
                "operation {id:?} has an empty `description`; every published catalog operation \
                 needs a source-grounded one-line description, including operations not exposed \
                 to a model"
            ));
        }

        if operation.effects.is_empty() {
            problems.push(format!(
                "operation {id:?} declares no `effects`; host-resource consequences are required \
                 facts and are never inferred from direction, method, host, risk, or driver"
            ));
        }
        let directional = match operation.direction {
            OperationDirection::Read => HostEffect::Read,
            OperationDirection::Write => HostEffect::Write,
        };
        let opposite = match operation.direction {
            OperationDirection::Read => HostEffect::Write,
            OperationDirection::Write => HostEffect::Read,
        };
        if !operation.effects.contains(&directional) || operation.effects.contains(&opposite) {
            problems.push(format!(
                "operation {id:?} declares direction {:?} but effects {:?}; the matching host \
                 effect is explicit and the opposite one is refused",
                operation.direction.word(),
                operation.effects
            ));
        }
        for pair in operation.effects.windows(2) {
            if pair[0] >= pair[1] {
                problems.push(format!(
                    "operation {id:?} has unsorted or duplicate `effects`; declare the closed \
                     vocabulary once each in stable order"
                ));
                break;
            }
        }
        if operation.required_capabilities.is_empty() {
            problems.push(format!(
                "operation {id:?} declares no `required_capabilities`; absence cannot prove that \
                 a deployment can serve it"
            ));
        }
        for pair in operation.required_capabilities.windows(2) {
            if pair[0] >= pair[1] {
                problems.push(format!(
                    "operation {id:?} has unsorted or duplicate `required_capabilities`; declare \
                     the closed vocabulary once each in stable order"
                ));
                break;
            }
        }

        match &operation.request {
            OperationRequest::HttpV1 { path, .. } => {
                if path.trim().is_empty() {
                    problems.push(format!("operation {id:?} has an empty `path`"));
                } else if !path.starts_with('/') {
                    problems.push(format!(
                        "operation {id:?} has path {path:?}, which must start with `/` — it is \
                         joined onto the connector's `base_url`"
                    ));
                }
                if operation.interaction_shape == InteractionShape::SessionEstablishment {
                    problems.push(format!(
                        "operation {id:?} selects `http_v1` with `session_establishment`; the HTTP \
                         request shape cannot establish a direct-byte session"
                    ));
                }
            }
            OperationRequest::SipV1 => {
                if operation.interaction_shape != InteractionShape::SessionEstablishment {
                    problems.push(format!(
                        "operation {id:?} selects `sip_v1` with interaction shape {:?}; SIP v1 is \
                         admitted only as `session_establishment`",
                        operation.interaction_shape
                    ));
                }
                if !operation.params.path.is_empty()
                    || !operation.params.query.is_empty()
                    || !operation.params.header.is_empty()
                    || !operation.params.const_headers.is_empty()
                {
                    problems.push(format!(
                        "operation {id:?} selects `sip_v1` but declares HTTP path, query, or header \
                         parameters; SIP call inputs must use the driver payload"
                    ));
                }
            }
            OperationRequest::AudioV1 => {
                // A local device answers one bounded request and returns; it establishes no
                // direct-byte session and leases nothing. Admitting `audio_v1` under any other
                // shape would promise a lifecycle the driver does not implement.
                if operation.interaction_shape != InteractionShape::Unary {
                    problems.push(format!(
                        "operation {id:?} selects `audio_v1` with interaction shape {:?}; audio v1 \
                         is admitted only as `unary`",
                        operation.interaction_shape
                    ));
                }
                if !operation.params.path.is_empty()
                    || !operation.params.query.is_empty()
                    || !operation.params.header.is_empty()
                    || !operation.params.const_headers.is_empty()
                {
                    problems.push(format!(
                        "operation {id:?} selects `audio_v1` but declares HTTP path, query, or \
                         header parameters; local-device inputs must use the driver payload"
                    ));
                }
            }
            OperationRequest::CdpV1 => {
                // A browser is a resource held across calls: one operation opens the session,
                // several observe or navigate inside it, one releases it. That is `leased_session`
                // and nothing else. `unary` would deny that the profile, process and page survive
                // between calls; `session_establishment` would promise a direct-byte plane and a
                // short-lived endpoint authority, and there is none — every browser result returns
                // through the ordinary bounded operation path.
                if operation.interaction_shape != InteractionShape::LeasedSession {
                    problems.push(format!(
                        "operation {id:?} selects `cdp_v1` with interaction shape {:?}; CDP v1 is \
                         admitted only as `leased_session`",
                        operation.interaction_shape
                    ));
                }
                if !operation.params.path.is_empty()
                    || !operation.params.query.is_empty()
                    || !operation.params.header.is_empty()
                    || !operation.params.const_headers.is_empty()
                {
                    problems.push(format!(
                        "operation {id:?} selects `cdp_v1` but declares HTTP path, query, or \
                         header parameters; the browser's own HTTP traffic belongs to the driver, \
                         not to this operation's request template"
                    ));
                }
            }
            OperationRequest::SqlV1 => {
                // A database answers one bounded request and returns: no direct-byte session is
                // established and nothing is leased across calls — the driver may pool
                // connections, but that is an implementation fact below this seam, not a
                // lifecycle a caller can observe. Admitting `sql_v1` under any other shape would
                // promise a lifecycle the driver does not implement.
                if operation.interaction_shape != InteractionShape::Unary {
                    problems.push(format!(
                        "operation {id:?} selects `sql_v1` with interaction shape {:?}; SQL v1 is \
                         admitted only as `unary`",
                        operation.interaction_shape
                    ));
                }
                if !operation.params.path.is_empty()
                    || !operation.params.query.is_empty()
                    || !operation.params.header.is_empty()
                    || !operation.params.const_headers.is_empty()
                {
                    problems.push(format!(
                        "operation {id:?} selects `sql_v1` but declares HTTP path, query, or \
                         header parameters; database inputs must use the driver payload"
                    ));
                }
            }
        }

        for param in operation.params.iter() {
            if param.name.trim().is_empty() {
                problems.push(format!(
                    "operation {id:?} has a parameter with an empty `name`"
                ));
            }
        }

        // Two answers to one question, refused rather than merged. "The body is these named fields"
        // and "the body *is* this schema" cannot both hold, and nothing states how they would
        // combine — so an operation declaring both has no derivable request body and no derivable
        // `input_schema` (C-125). `connector-flux` refuses it again at emission, which is the
        // narrower gate: this one also covers a definition nobody has emitted yet.
        if operation.params.body_schema.is_some() && !operation.params.body.is_empty() {
            problems.push(format!(
                "operation {id:?} declares both named `params.body` fields and a free-form \
                 `params.body_schema`. Those are two answers to one question — what the request \
                 body is — and there is no rule for merging them, so declare either the fields or \
                 the schema"
            ));
        }
        for param in &operation.params.path {
            // The placeholder is written in the vendor's spelling, so a parameter that declares a
            // `wire` alias is looked up under that — matching on the caller-facing name would
            // reject `{requester_id}` for a parameter a caller knows as `req_id`.
            let wire = param.wire.as_deref().unwrap_or(&param.name);
            let placeholder = format!("{{{wire}}}");
            let path = operation.request.http_path().unwrap_or_default();
            if !path.contains(&placeholder) {
                problems.push(format!(
                    "operation {id:?} declares path parameter {:?}, but its path {:?} has no \
                     `{placeholder}` to interpolate it into",
                    param.name, path
                ));
            }
        }

        validate_repeatability_condition(operation, problems);
        validate_semantic_effects(operation, problems);
        // The two credential declarations are checked as a pair before either is checked alone:
        // when both are present the operation is incoherent at the root, and the rules downstream
        // of each would render two contradicting instructions for one fact. See
        // `validate_one_credential_disposition`, which returns whether it took the decision.
        if !validate_one_credential_disposition(operation, problems) {
            validate_credential_response(operation, problems);
            validate_produces_credential(connector, operation, problems);
        }

        if let Some(alternatives) = &operation.auth {
            validate_requirements(
                connector,
                alternatives,
                &format!("operation {id:?}"),
                problems,
            );
        }
    }
}

/// Semantic effects are a closed, policy-bearing set and must agree with the metadata Flux gates.
fn validate_semantic_effects(operation: &Operation, problems: &mut Vec<String>) {
    let id = operation.id.as_str();

    for pair in operation.semantic_effects.windows(2) {
        if pair[0] == pair[1] {
            problems.push(format!(
                "operation {id:?} declares semantic effect {:?} more than once; semantic effects \
                 are a set, so remove the duplicate rather than relying on a consumer to dedupe it",
                pair[0].tag()
            ));
        }
    }

    if operation.semantic_effects.contains(&SemanticEffect::Pure) {
        problems.push(format!(
            "operation {id:?} declares semantic effect `pure`, but every connector operation makes \
             an external HTTP call. `pure` means deterministic and side-effect free, so it cannot \
             describe a connector operation"
        ));
    }

    for effect in &operation.semantic_effects {
        if matches!(effect, SemanticEffect::Money | SemanticEffect::Delete)
            && operation.risk != Risk::Destructive
        {
            problems.push(format!(
                "operation {id:?} declares semantic effect {:?} but risk {:?}; Flux requires \
                 `money` and `delete` to be `destructive` so policy and the approval preview cannot \
                 understate them",
                effect.tag(),
                risk_word(operation.risk)
            ));
        } else if effect.is_consequential() && operation.risk == Risk::Low {
            problems.push(format!(
                "operation {id:?} declares consequential semantic effect {:?} but risk `low`; Flux \
                 does not permit a consequence that outlives the call to use its harmless tier",
                effect.tag()
            ));
        }
    }

    if operation
        .semantic_effects
        .iter()
        .any(|effect| effect.is_consequential())
        && operation.idempotency == Idempotency::Idempotent
    {
        problems.push(format!(
            "operation {id:?} declares a consequential semantic effect but `idempotency = \
             \"idempotent\"`; that value licenses Flux to skip execution in favour of a cached \
             result, so a consequence-bearing operation must be `conditional` or `non_idempotent`"
        ));
    }
}

/// **A `conditional` write must state its condition, and the condition must mean something.**
///
/// flux's I3 (`flux_spec::coherence`) names [`Idempotency::Conditional`] as the escape hatch for a
/// mutation that is genuinely replay-safe — "safe to repeat under **stated** conditions". This is
/// what makes "stated" true. Before C-186 nothing did: six operations declared `conditional` with
/// the condition written in no field and no artifact, so a host learned that a condition existed
/// and nothing about what it was.
///
/// Four refusals, each a different author mistake:
///
/// - **an authored `conditional` write with no condition** — the claim without the thing that makes it
///   checkable, and the reason this validator exists;
/// - **a condition on an authored read** — there is no repeat hazard to condition, so the
///   field would spread as cargo-culted decoration until no reviewer read any of them;
/// - **a condition on an operation not declaring `conditional`** — prose asserting what its own
///   field denies, which is precisely the drift this story removes, arriving from the other side;
/// - **a condition that says nothing** — `"yes"` unlocks the claim while telling a reviewer no more
///   than silence did.
///
/// `connector-flux` refuses all four again on the IR rather than on the file. That overlap is
/// deliberate and each layer is pinned on its own: this is the loud, early refusal an author sees,
/// and `check_write_metadata` is the one an IR assembled in memory cannot walk past.
fn validate_repeatability_condition(operation: &Operation, problems: &mut Vec<String>) {
    let id = operation.id.as_str();
    let mutating = operation.direction == OperationDirection::Write;

    if !operation.states_repeatability_condition() {
        if mutating && operation.idempotency == Idempotency::Conditional {
            problems.push(format!(
                "operation {id:?} declares `idempotency = \"conditional\"` but no \
                 `repeatable_because`. `conditional` is flux's escape hatch for a write that is \
                 genuinely safe to repeat *under a stated condition* (`flux_spec::coherence`, I3), \
                 and a condition stated nowhere leaves a host knowing only that one exists — say \
                 what makes repeating this call safe"
            ));
        }
        return;
    }

    if !mutating {
        problems.push(format!(
            "operation {id:?} is an authored {} and declares `repeatable_because`, but a read has \
             no repeat hazard to put a condition on. The field exists only to state the \
             condition behind `idempotency = \"conditional\"` on a write; remove it",
            operation.direction.word()
        ));
        return;
    }

    if operation.idempotency != Idempotency::Conditional {
        problems.push(format!(
            "operation {id:?} declares `repeatable_because` but `idempotency = {:?}`. The condition \
             describes a repeat that is safe while the field says otherwise — one of the two is \
             wrong, and shipping both is the contradiction C-186 exists to end",
            idempotency_word(operation.idempotency)
        ));
        return;
    }

    if operation.repeatability_condition().is_none() {
        problems.push(format!(
            "operation {id:?} declares `repeatable_because` = {:?}, which is shorter than \
             {MIN_REPEATABILITY_CONDITION} characters once trimmed and states no vendor behaviour. \
             This is what a reviewer reads beside a retry-safety claim on a write; say what \
             repeating the call actually does, as `cloudflare-cache-purge` and \
             `launchdarkly-flag-toggle` do",
            operation.repeatable_because.as_deref().unwrap_or_default()
        ));
    }
}

/// The `risk` value as an author spells it in a provider file. Exhaustive for the reason
/// [`idempotency_word`] is: a fifth variant must be a compile error here, not a refusal quoting a
/// word the file cannot contain.
pub(super) fn risk_word(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "low",
        Risk::Medium => "medium",
        Risk::High => "high",
        Risk::Destructive => "destructive",
    }
}

/// A boolean as an author spells it, so `expose` reads the same way in a refusal as in the file.
pub(super) fn bool_word(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// **One fact, one disposition** — C-432's reconciliation of C-430 and C-136.
///
/// [`Operation::credential_response`] and [`Operation::produces_credential`] state the same fact —
/// a credential arrives in this operation's response — and prescribe opposite outcomes. C-430's
/// field withholds the operation; C-136's ships it and returns a handle. Declared together they ask
/// for both, and before this story the loader obliged: `credential_response`'s stock refusal told
/// the author to withhold an operation the sibling declaration says ships, with nothing stating
/// which governs. Two rules in one repository is the thing C-432 exists to remove.
///
/// **The discriminator is purpose, not shape.** Both fields point at a credential in a response; no
/// inspection of the pointer, the schema or the field name can tell them apart, because the
/// difference is what the operation is *for*:
///
/// - the credential **is** the answer — a token exchange, a login — so diverting it into the store
///   and returning the handle costs the caller nothing. That is `produces_credential`.
/// - the credential arrives **incidentally**, beside the meeting or the server the operation exists
///   to deliver. Diverting the whole result would delete the answer, so the operation is withheld
///   until the value can be redacted where it sits — `credential_response`, and C-79.
///
/// That sentence is what the refusal has to carry, because it is the one thing an author cannot
/// re-derive by reading either field's own documentation.
///
/// Returns `true` when it refused, and the pair-wise check is then the *only* thing said about this
/// operation's credential declarations: the per-field rules downstream are all conditioned on a
/// disposition this operation has not yet chosen, so running them would bury the choice under
/// consequences of both branches.
fn validate_one_credential_disposition(operation: &Operation, problems: &mut Vec<String>) -> bool {
    if operation.credential_response.is_empty() || operation.produces_credential.is_none() {
        return false;
    }
    let id = operation.id.as_str();
    problems.push(format!(
        "operation {id:?} declares both `credential_response` (at {}) and `produces_credential`, \
         which state one fact — a credential arrives in this response — and prescribe opposite \
         dispositions. Exactly one governs, and which one is a question about the operation's \
         **purpose**, not about the shape of the value: if the credential *is* the answer, as a \
         token exchange's is, declare only `produces_credential` and the value is diverted into the \
         bound `CredentialStore` with the caller receiving the handle. If the credential arrives \
         **incidentally**, beside the result the operation exists to deliver, declare only \
         `credential_response` — diverting the whole result would delete the answer, so the \
         operation is withheld until the value can be redacted where it sits (C-79)",
        quoted(&operation.credential_response)
    ));
    true
}

/// **No operation returns a secret** (C-430) — the gate, reading the declaration that says one does.
///
/// `AGENTS.md` § Authentication contract states the rule this enforces, and states it once: an
/// operation whose declared response carries a token is withheld until C-136's diversion lands,
/// because the host's redactor holds only values the host itself resolved and cannot know a secret
/// minted by the very call returning it. Four operations shipped in v0.9.0 against it — postmark's
/// server pair returning `ApiTokens` in plaintext, zoom's meeting pair returning a `start_url` with
/// the host's ZAK token embedded — every one of them accurately describing the hazard in its own
/// `response_schema` and returning the field anyway. Describing a credential is not withholding it.
///
/// # It reads a declaration, and that is the design rather than a shortcut
///
/// A catalogue-wide scan for token-shaped property names found 31 candidates and **28 of them were
/// correct as they stood**, each documented as harmless by its own connector. A regex over field
/// names would refuse all 28, and a gate that is wrong nine times in ten is one authors learn to
/// spell around — so the only thing that trips this is [`Operation::credential_response`], which
/// nothing but a connector can write. The cost is stated rather than hidden: this does not catch an
/// author who never declares. `crates/connector-spec/tests/credential_response.rs` carries the other
/// half — the four withheld operations, named, so reinstating one is a red build.
///
/// # Three refusals, and the first two are what keep the third honest
///
/// - **A location with no `response_schema` to resolve against**, which is a claim about a shape
///   nothing states.
/// - **A location that matches nothing**, which is the shape a vendor rename takes: a declaration
///   that quietly stopped applying reads as protection while being none. C-79 names this one
///   explicitly, and it is the reason the walk descends into arrays — `ApiTokens` sits under
///   `Servers[]`, and a resolver stopping at the root would call the true declaration a typo.
/// - **The declaration itself**, which is the withholding.
fn validate_credential_response(operation: &Operation, problems: &mut Vec<String>) {
    if operation.credential_response.is_empty() {
        return;
    }
    let id = operation.id.as_str();

    match &operation.response_schema {
        None => problems.push(format!(
            "operation {id:?} declares `credential_response` but no `response_schema`, so there is \
             nothing for {} to resolve against. A location naming a shape the operation does not \
             declare cannot be checked by anything",
            quoted(&operation.credential_response)
        )),
        Some(schema) => {
            for location in &operation.credential_response {
                if !response_location_exists(schema, location) {
                    problems.push(format!(
                        "operation {id:?} declares a credential response location {location:?} \
                         that matches nothing in its `response_schema`. A location resolving to \
                         nothing protects nothing, and this is the shape a vendor rename takes — \
                         spell each segment as the response spells it, and `*` for every element \
                         of an array"
                    ));
                }
            }
        }
    }

    problems.push(format!(
        "operation {id:?} declares that its own response carries a credential at {}, so it cannot \
         ship. `AGENTS.md` § Authentication contract: an operation whose declared response carries \
         a token is withheld until C-136's diversion lands, because the host's redactor holds only \
         values the host itself resolved and cannot know a secret minted by the very call returning \
         it. Withhold the operation and name it as an exclusion carrying that reason — `expose = \
         false` is not the mechanism, since `connector_pack::resolve` admits any named operation \
         whatever its exposure (C-413)",
        quoted(&operation.credential_response)
    ));
}

/// **A credential-producing operation returns a handle, or it does not load** — C-136's refusals.
///
/// [`Operation::produces_credential`] is the declaration that makes a login shippable: the secret
/// travels from the vendor's response into the host's bound `CredentialStore` and the caller
/// receives `{ "credential": "tenants/…" }`. Every rule below exists because the guarantee is
/// *structural* — it comes from the declared shape rather than from a filter — and a declaration the
/// loader accepted while the shape did not hold would be the worst of both: an operation documented
/// as safe to call, shipping the token.
///
/// # The three the story names, and the three that make them possible
///
/// - **The declared output still exposes the secret.** A `response_schema` beside this declaration
///   documents the vendor's wire body; if the secret's own location resolves in it, the operation is
///   describing an output it does not have and one that carries a credential. Refused, and this is
///   C-430's mechanism read from the other side — that story established that *deleting* the
///   location from the schema removes the disclosure and leaves the exposure, so the schema is
///   cross-checked rather than silently rewritten.
/// - **No secret field is named.** The extractor would not know what to divert, and an operation
///   that diverts nothing returns the vendor's body — which is the unsafe operation wearing the safe
///   operation's declaration.
/// - **`idempotency = "idempotent"`.** Minting a token is a write, and some vendors invalidate the
///   previous one; `Idempotent` additionally licenses flux's op cache to serve a stored result
///   *instead of executing*, which for a login means handing back an address whose value was
///   replaced.
///
/// The other three are the ones without which the first three cannot be enforced at all: a
/// credential the connector does not declare has no leaf and therefore no address; a connector with
/// no `authority` has no second path segment, so nothing composes; and two operations minting one
/// credential leave "which call put the value there" unanswerable, which is the same ambiguity
/// C-406 refuses for two connections of one vendor.
fn validate_produces_credential(
    connector: &Connector,
    operation: &Operation,
    problems: &mut Vec<String>,
) {
    let Some(produced) = &operation.produces_credential else {
        return;
    };
    let id = operation.id.as_str();

    // **Refusal 1 — names no secret field.** A pointer must start with `/`, exactly as
    // `credential_response` does: one spelling of "a location in a response", not two.
    if produced.secret.trim().is_empty() || !produced.secret.starts_with('/') {
        problems.push(format!(
            "operation {id:?} declares `produces_credential` with `secret = {:?}`, which names no \
             field of the vendor's response. The extractor would not know what to divert, so the \
             operation would return the vendor's body — state a location as `credential_response` \
             spells one: a JSON Pointer into the response body, `/access_token`",
            produced.secret
        ));
    }

    // **And it names exactly one value.** `credential_response`'s vocabulary admits `*` for every
    // element of an array, because that field describes *where credentials appear* and an array of
    // them is a real shape — postmark's `Servers[].ApiTokens` is the case that forced it. A **mint**
    // is the other question: one call, one value, one address. `*` here would name several secrets
    // for one credential with nothing to say which is stored, so it is refused at load rather than
    // left to fail at every call. Refusing is also what keeps the runtime honest — the diversion
    // resolves the location with `serde_json::Value::pointer`, which has no wildcard, so a `*` this
    // validator admitted would be a documented behaviour the code does not have.
    if produced.secret.contains('*') {
        problems.push(format!(
            "operation {id:?} declares `produces_credential` at {:?}, which uses `*`. A `*` names \
             every element of an array and a mint stores exactly one value at exactly one address, \
             so there would be nothing to say which element is the credential. That spelling \
             belongs to `credential_response`, which describes where credentials *appear*; name a \
             single location here",
            produced.secret
        ));
    }

    // **Refusal 2 — the declared output still exposes the secret.** Read against what the author
    // wrote, because `Operation::effective_response_schema` already answers the handle here; the
    // question is whether the connector's own description of the wire body promises a caller the
    // value.
    if let Some(schema) = &operation.response_schema {
        if response_location_exists(schema, &produced.secret) {
            problems.push(format!(
                "operation {id:?} declares `produces_credential` at {:?} and its `response_schema` \
                 still describes that location, so its published contract offers a caller the \
                 secret the diversion exists to withhold. A `response_schema` here documents the \
                 vendor's wire body and must not carry the minted value — note that deleting the \
                 location is not enough on its own (C-430): what makes the operation safe is that \
                 the value never reaches the result, which is what `produces_credential` does",
                produced.secret
            ));
        }
    }

    // **Refusal 3 — a write declared safe to repeat.**
    if operation.idempotency == Idempotency::Idempotent {
        problems.push(format!(
            "operation {id:?} declares `produces_credential` and `idempotency = \"idempotent\"`. \
             Minting a credential is a write — some vendors invalidate the previous token — and \
             `idempotent` additionally licenses flux's op cache to serve a stored result instead of \
             executing, which would hand back an address whose value has since been replaced. \
             Declare `non_idempotent`"
        ));
    }

    // The credential must be one the connector declares, or there is no leaf to address it by.
    if !connector
        .auth
        .iter()
        .any(|method| method.name == produced.credential)
    {
        problems.push(format!(
            "operation {id:?} declares `produces_credential` storing {:?}, which this connector \
             declares no `[[auth]]` credential for. The value would have nowhere to be put: the \
             address is composed from the connector's `authority` and the credential's own leaf, \
             and neither exists for a name nothing declares",
            produced.credential
        ));
    }

    // And the connector must have an authority, or the second segment of the address does not
    // exist. `connector-pack` refuses the same arrangement at resolve time with
    // `Error::NoCredentialAddress`; refusing here makes it a build failure instead of a first-call
    // one.
    if connector.authority.is_none() {
        problems.push(format!(
            "operation {id:?} declares `produces_credential` but this connector declares no \
             `authority`, so `tenants/<tenant>/<authority>/…` has no second segment and the minted \
             value has no address to be stored at"
        ));
    }

    // One producer per credential. Two would leave a reader with no way to say which call put the
    // value there, and the catalogue's own record of the mint names exactly one operation.
    for other in &connector.operations {
        if other.id == operation.id {
            break;
        }
        if other
            .produces_credential
            .as_ref()
            .is_some_and(|earlier| earlier.credential == produced.credential)
        {
            problems.push(format!(
                "operations {:?} and {id:?} both declare `produces_credential` storing {:?}. Two \
                 calls minting into one address leave \"which one put the value there\" \
                 unanswerable, and a downstream operation naming the credential cannot say which \
                 login it needs — give each grant its own credential",
                other.id, produced.credential
            ));
        }
    }
}

/// Locations as a refusal lists them: `"/a", "/b"`. One spelling, so two refusals about the same
/// operation read alike.
fn quoted(locations: &[String]) -> String {
    locations
        .iter()
        .map(|location| format!("{location:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `idempotency` value as an author spells it in a provider file. Exhaustive so a fourth variant
/// is a compile error here rather than a refusal quoting the wrong word.
pub(super) fn idempotency_word(idempotency: Idempotency) -> &'static str {
    match idempotency {
        Idempotency::Idempotent => "idempotent",
        Idempotency::NonIdempotent => "non_idempotent",
        Idempotency::Conditional => "conditional",
    }
}

/// The method as an author spells it in a provider file.
pub(super) fn method_word(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

/// Checks one alternatives list — the OR of mechanisms an operation or the connector default names.
///
/// Two rules, and the first is the one that keeps the encoding unambiguous.
pub(super) fn validate_requirements(
    connector: &Connector,
    alternatives: &[AuthRequirement],
    context: &str,
    problems: &mut Vec<String>,
) {
    for (index, mechanism) in alternatives.iter().enumerate() {
        // An empty mechanism inside a non-empty list is a *second spelling of "no auth"*, and the
        // IR already has one: an empty alternatives list. Two encodings of one meaning is how
        // ambiguity gets baked in — and here the two would not even be equivalent downstream, since
        // C-10 picks "the first satisfiable mechanism" and an empty mechanism is trivially
        // satisfiable, so it would silently disable auth for the whole operation.
        if mechanism.is_empty() {
            problems.push(format!(
                "{context}: auth mechanism {index} names no credentials. \"No auth\" is written as \
                 an empty alternatives list (`auth = []`), never as a list holding an empty \
                 mechanism — an empty mechanism is always satisfiable, so it would disable auth for \
                 every alternative beside it"
            ));
            continue;
        }
        for credential in mechanism {
            match connector.auth_method(credential) {
                None => problems.push(format!(
                    "{context}: auth mechanism {index} names credential {credential:?}, which no \
                     `[[auth]]` block declares"
                )),
                // The complement of the rule in `validate_hmac`: a signing secret establishes
                // transport provenance and never represents a vendor capability grant. Operations,
                // events and channel establishment therefore cannot use it as ordinary auth.
                Some(method) if method.scheme == AuthScheme::Signing => problems.push(format!(
                    "{context}: auth mechanism {index} names credential {credential:?}, which is \
                     declared `scheme = \"signing\"`. A signing secret verifies an inbound request \
                     and is never placed in an outgoing one or treated as capability evidence"
                )),
                Some(_) => {}
            }
        }
        for (credential, alternatives) in mechanism.scopes() {
            if !mechanism.contains(credential) {
                problems.push(format!(
                    "{context}: auth mechanism {index} attaches scopes to credential \
                     {credential:?}, but that mechanism does not name it in `credentials`. Scope \
                     evidence is credential-local and cannot authorize a different credential"
                ));
            }
            if alternatives.is_empty() {
                problems.push(format!(
                    "{context}: auth mechanism {index} gives credential {credential:?} an empty \
                     scope-alternatives list. Omit the `scopes` entry when presence alone is \
                     sufficient; an explicit scope expression must name at least one alternative"
                ));
                continue;
            }
            for (alternative, scopes) in alternatives.iter().enumerate() {
                if scopes.is_empty() {
                    problems.push(format!(
                        "{context}: auth mechanism {index} gives credential {credential:?} an \
                         empty scope set at alternative {alternative}. That alternative is always \
                         true and would silently bypass every scoped alternative beside it"
                    ));
                }
                for scope in scopes {
                    if scope.is_empty()
                        || scope.len() > 256
                        || scope
                            .bytes()
                            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
                    {
                        problems.push(format!(
                            "{context}: auth mechanism {index} declares invalid scope {scope:?} \
                             for credential {credential:?}. A scope is 1..=256 non-whitespace, \
                             non-control bytes"
                        ));
                    }
                }
            }
        }
    }
}
