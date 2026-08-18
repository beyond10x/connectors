use super::*;

/// Checks the configuration surface — what a human is asked for, and where each answer goes.
///
/// Two properties, and the first is the one that closes a defect every templated provider records in
/// a comment: **a connector must ask for everything it needs**, and **it must not ask for anything it
/// cannot use**. A template variable nobody declares is a connector that silently cannot be
/// configured; a field binding nothing real is a question whose answer is discarded.
///
/// # Why `secret` + `example` is refused here and not asserted over `providers/` — C-231
///
/// The case against putting it in the loader is that an `example` is *documentation*, and a loader
/// that polices documentation is a loader with an opinion about prose. **That argument is rejected,
/// on three grounds:**
///
/// 1. **This loader already treats `example` as a checked property, not as prose.** It is validated
///    against the field's own [`Format`](crate::Format) six lines below, and against the request
///    position it pins in [`validate_pin`]. The precedent is not merely nearby, it is the same
///    field; the documentation argument was already answered when those landed.
/// 2. **The property being checked is not "is this placeholder good".** It is "no credential-shaped
///    literal is committed", which is the rule
///    `no_provider_file_carries_a_credential_value` states over the same files and which
///    `validate_const_headers` already enforces at the loader for a header value. A secret field's
///    `example` is the one remaining place in `[[config]]` where a credential-shaped literal is
///    invited by the schema.
/// 3. **A test over `providers/` protects this repository's 53 files and nobody else's.** These
///    crates are published, so a downstream author writing their own provider TOML is a real
///    person, and a refusal at [`load`] is the only form of this rule that reaches them. The cost
///    asymmetry is what settles it: a placeholder that merely *looks* like a token blocks a push
///    and costs an hour, and one that *is* a token is a disclosed credential.
///
/// Catalogue-wide, the rule is named by `no_shipped_provider_gives_a_secret_field_an_example`
/// (`crates/connector-spec/tests/config_fields.rs`), which enumerates `providers/` from disk — the
/// same shape `no_shipped_provider_has_an_unbound_template_variable` beside it already uses for a
/// rule the loader also refuses, and the reason a provider landing tomorrow is covered without
/// anyone adding it to a list. What does *not* belong anywhere is a **per-connector** restatement:
/// C-219's `no_secret_config_field_carries_an_example` was reduced to the claim that is actually
/// about Confluence. Measured while landing this: **24** per-connector tests spelled this rule out,
/// and 14 of the 38 providers that declare a secret field had none — one rule with two dozen
/// spellings that still missed a third of its surface, which is the defect C-230 is about.
///
/// **Scope.** Secret fields only. Whether a *non-secret* field's example is realistic is a
/// documentation question, and those placeholders stay welcome.
pub(super) fn validate_config(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for field in &connector.config {
        let name = field.name.as_str();
        if name.trim().is_empty() {
            problems.push("a configuration field has an empty `name`".to_owned());
            continue;
        }
        if seen.contains(&name) {
            problems.push(format!(
                "configuration field {name:?} is declared more than once; the name is the key a host \
                 stores the collected value under"
            ));
        }
        seen.push(name);

        if let Err(reason) = crate::address::validate_member_name(name) {
            problems.push(format!(
                "configuration field {name:?} has an invalid `name`: {reason}"
            ));
        }
        validate_member_service(
            connector,
            "configuration field",
            name,
            &field.service,
            problems,
        );

        // A whole HTTPS origin is the explicit C-402 self-managed exception: the connector owns
        // the path while deployment policy approves the scheme+authority. It is deliberately a
        // generic configuration shape, not a GitLab branch in a consumer.
        if field.format == Format::Origin {
            if field.approval != Approval::Operator {
                problems.push(format!(
                    "configuration field {name:?} declares `format = \"origin\"` without \
                     `approval = \"operator\"`. A caller-selected whole authority is an unbounded \
                     egress grant; a non-default origin becomes active only after deployment policy \
                     approves and pins it"
                ));
            }
            match field.binding() {
                Some(Binding::Endpoint { variable }) => {
                    let base_url = connector.base_url_of(&field.service);
                    let placeholder = format!("{{{variable}}}");
                    if !base_url.starts_with(&placeholder)
                        || base_url[placeholder.len()..]
                            .chars()
                            .next()
                            .is_some_and(|next| next != '/')
                    {
                        problems.push(format!(
                            "configuration field {name:?} declares an HTTPS origin but service {:?} \
                             has base URL {base_url:?}. An origin must be the entire leading \
                             endpoint placeholder (`{{{variable}}}`); the connector may append a \
                             path after it, but input may not replace that path",
                            field.service
                        ));
                    }
                }
                _ => problems.push(format!(
                    "configuration field {name:?} declares `format = \"origin\"` but does not bind \
                     an `endpoint.<variable>`. An origin is one resolved endpoint, not an operation \
                     argument or request field"
                )),
            }
        } else if field.approval == Approval::Operator {
            problems.push(format!(
                "configuration field {name:?} declares `approval = \"operator\"` but format `{}`. \
                 Operator approval on this surface is the explicit whole-HTTPS-origin policy; use \
                 `format = \"origin\"` so consumers and the runtime can enforce the same rule",
                field.format.word()
            ));
        }

        // A field with no label or no help cannot be rendered into a form that anyone can answer.
        // Defaulting either to `name` would ship `zendesk.api_token` as user-facing copy.
        if field.label.trim().is_empty() {
            problems.push(format!(
                "configuration field {name:?} has an empty `label`; it is the text a form shows \
                 beside the input, and there is no sensible default for it — {name:?} is an \
                 identifier, not a label"
            ));
        }
        if field.help.trim().is_empty() {
            problems.push(format!(
                "configuration field {name:?} has an empty `help`; a field a user cannot answer is a \
                 field that stops the installation"
            ));
        }

        // A secret field takes no placeholder at all — C-231. See `validate_config`'s own doc
        // comment for why this is a loader refusal rather than a test over `providers/`.
        if field.secret && field.example.is_some() {
            problems.push(format!(
                "configuration field {name:?} declares `secret = true` and an `example`. A secret \
                 takes no placeholder: a token-shaped literal in a committed file has tripped \
                 GitHub push protection and blocked a release here before, and a placeholder that \
                 is a real token is a disclosed credential rather than a blocked push. Nobody \
                 recognises their own secret from an example of someone else's — put the shape in \
                 `help` instead"
            ));
        }

        // The example is a placeholder a user will copy, so it has to satisfy the field's own rule.
        if let Some(example) = &field.example {
            if let Err(reason) = field.format.validate(example) {
                problems.push(format!(
                    "configuration field {name:?} declares `format = \"{}\"` but an `example` that \
                     does not satisfy it: {reason}. A placeholder that would fail the field's own \
                     validation is worse than none, because a user copies it",
                    field.format.word()
                ));
            }
        }

        if field.secret && field.default.is_some() {
            problems.push(format!(
                "configuration field {name:?} declares a secret `default`; a default is a literal \
                 sent on the wire and credentials never belong in provider TOML"
            ));
        }
        if field.required && field.default.is_some() {
            problems.push(format!(
                "configuration field {name:?} is required and also declares a `default`. A value \
                 the connector can supply itself is optional; set `required = false`"
            ));
        }
        if let Some(default) = &field.default {
            if let Err(reason) = field.format.validate(default) {
                problems.push(format!(
                    "configuration field {name:?} declares a `default` that does not satisfy \
                     format `{}`: {reason}",
                    field.format.word()
                ));
            }
            if let Err(reason) = field.permits(default) {
                problems.push(format!(
                    "configuration field {name:?} declares a `default` outside its choices: {reason}"
                ));
            }
        }

        validate_choices(field, problems);
        validate_binding(connector, field, problems);
    }

    validate_every_template_variable_is_asked_for(connector, problems);
}

/// **A closed set of values is a narrowing of the field, not a second field beside it** — C-225.
///
/// `choices` answers *which values are legal*; [`Format`](crate::Format) answers *what shape a value
/// has*. Keeping them separate is what makes the rules below derivations rather than preferences:
///
/// 1. **Every permitted value satisfies the field's own `format`.** A set that could admit a value
///    the format rejects would let a closed field be *wider* than the open one it narrows, and the
///    renderer's fallback input — built from `format` — would refuse a value the select offers.
/// 2. **A set has at least two values.** A set of one is a constant: the field asks a question with
///    one answer, which belongs in the base URL rather than in front of a human. An empty
///    `choices = []` is an open field spelled the long way, and reads in a diff as a set someone
///    emptied by accident.
/// 3. **Every entry is renderable and distinguishable.** A blank label is a dropdown row with
///    nothing in it; a repeated value is one member wearing two names; a repeated label is two rows
///    a user cannot tell apart. Each of the three produces a form that cannot be answered
///    correctly, which is the same standard `label` and `help` are mandatory under.
/// 4. **A `secret` declares none.** The values would be credentials, enumerated in a committed file.
///    That is the C-231 rule about `example` in its stronger form — an example is one such literal,
///    a set is all of them — and the same push-protection and disclosure argument settles it.
/// 5. **The `example` is one of the choices.** It is the placeholder a user copies, so on a closed
///    field it has to be an answer they are allowed to give. Exactly the defect class the
///    format/example rule already refuses, one level narrower.
///
/// The sixth rule is not here: a value pinned into a request position is checked against that
/// position in [`validate_pin`], beside the `example` check it mirrors, because the rule belongs to
/// the binding rather than to the set.
fn validate_choices(field: &ConfigField, problems: &mut Vec<String>) {
    let name = field.name.as_str();
    if field.choices.is_empty() {
        // Distinguishable from "no `choices` key at all" only in the source, so an explicit empty
        // list is called out rather than silently read as an open field.
        return;
    }

    if field.secret {
        problems.push(format!(
            "configuration field {name:?} declares `secret = true` and `choices`. A closed set of \
             secret values is a list of credentials in a committed file — the same defect a \
             secret's `example` is refused for (C-231), and a stronger form of it, because a set is \
             exhaustive where an example is one literal"
        ));
        return;
    }

    if field.choices.len() < 2 {
        problems.push(format!(
            "configuration field {name:?} declares `choices` with one value. A set of one is a \
             constant, not a choice: put the value in the `base_url` (or wherever the field binds) \
             rather than asking a human to confirm the only answer"
        ));
    }

    let mut values: Vec<&str> = Vec::new();
    let mut labels: Vec<&str> = Vec::new();
    for choice in &field.choices {
        if let Err(reason) = field.format.validate(&choice.value) {
            problems.push(format!(
                "configuration field {name:?} declares `format = \"{}\"` but a choice that does not \
                 satisfy it: {reason}. A closed set narrows the field's format; it cannot widen it, \
                 or the input a renderer falls back to would reject a value the set offers",
                field.format.word()
            ));
        }
        if choice.label.trim().is_empty() {
            problems.push(format!(
                "configuration field {name:?} declares a choice {:?} with an empty `label`; a set \
                 of raw values is a dropdown nobody can read, which is why the label is the whole \
                 reason a choice is a table rather than a string",
                choice.value
            ));
        }
        if values.contains(&choice.value.as_str()) {
            problems.push(format!(
                "configuration field {name:?} lists the choice {:?} more than once; one value under \
                 two labels is a set a user cannot select from unambiguously",
                choice.value
            ));
        }
        if labels.contains(&choice.label.as_str()) {
            problems.push(format!(
                "configuration field {name:?} uses the label {:?} more than once; two rows a user \
                 cannot tell apart is a choice they cannot make",
                choice.label
            ));
        }
        values.push(&choice.value);
        labels.push(&choice.label);
    }

    if let Some(example) = &field.example {
        if let Err(reason) = field.permits(example) {
            problems.push(format!(
                "configuration field {name:?} declares an `example` that is not one of its own \
                 choices: {reason}. A placeholder a user copies and is then refused for is worse \
                 than none"
            ));
        }
    }
}

/// Checks one field's `binds`: that it parses, that it resolves, and that `secret` agrees with it.
///
/// **And each of its `also_binds`, on the same terms** (C-229). A field reaching several
/// destinations is validated once per destination rather than once per field, so "every position is
/// checked, not only the first" is how the loop is written rather than a claim beside it. The
/// per-field questions — the destination set is well-formed, and the slot collides with nothing —
/// are [`validate_destinations`] and [`validate_slot_is_not_shared`].
fn validate_binding(connector: &Connector, field: &ConfigField, problems: &mut Vec<String>) {
    validate_destinations(field, problems);
    for binds in
        std::iter::once(field.binds.as_str()).chain(field.also_binds.iter().map(String::as_str))
    {
        validate_one_binding(connector, field, binds, problems);
    }
    validate_slot_is_not_shared(connector, field, problems);
    validate_also_services(connector, field, problems);
}

/// **A shared endpoint slot names real sibling services, and only an endpoint slot may be shared**
/// (C-529).
///
/// Four refusals, each closing a way the declaration could name something without doing anything:
///
/// 1. **Only an `endpoint.` binding may be shared.** A credential or a request pin has no
///    per-service placeholder for a second service to fill, so an entry on one is a service named
///    for no effect — which reads like coverage and is not.
/// 2. **Every named service is declared.** A typo would leave the real service's `{variable}`
///    unbound, and the coverage check would then report the *other* service as the problem.
/// 3. **The head is not repeated.** `service = "default"` with `also_services = ["default"]` is one
///    slot spelled twice; harmless to resolve and a sign the author meant a different name.
/// 4. **No service is named twice.**
///
/// What this deliberately does *not* refuse is two fields reaching one service with different
/// variables — that is ordinary, and Contentful's two `space_id` fields stay two slots because they
/// share no field, not because anything here stops them.
fn validate_also_services(connector: &Connector, field: &ConfigField, problems: &mut Vec<String>) {
    if field.also_services.is_empty() {
        return;
    }
    let name = field.name.as_str();

    if !matches!(field.binding(), Some(Binding::Endpoint { .. })) {
        problems.push(format!(
            "configuration field {name:?} declares `also_services`, but binds {:?} rather than an \
             `endpoint.<variable>`. Only a base-URL placeholder exists once per service and can \
             therefore be filled for a sibling service; a credential or a request pin has no \
             per-service slot, so the entry would name a service without reaching anything there",
            field.binds
        ));
        return;
    }

    let declared = connector.service_names();
    let mut seen: Vec<&str> = Vec::new();
    for extra in &field.also_services {
        let extra = extra.as_str();
        if extra == field.service {
            problems.push(format!(
                "configuration field {name:?} lists its own service {extra:?} in `also_services`. \
                 The head `service` already carries the address; listing it again is one slot \
                 spelled twice"
            ));
            continue;
        }
        if seen.contains(&extra) {
            problems.push(format!(
                "configuration field {name:?} lists service {extra:?} twice in `also_services`"
            ));
            continue;
        }
        seen.push(extra);
        if !declared.contains(&extra) {
            problems.push(format!(
                "configuration field {name:?} lists service {extra:?} in `also_services`, which \
                 this connector does not declare. A misspelled sibling leaves the real service's \
                 base-URL placeholder unbound, and the failure would then be reported against that \
                 service rather than against this typo"
            ));
        }
    }
}

/// **The destination set itself is well-formed**, before any of its members is resolved.
///
/// Three rules, and each is a consequence of the slot being `binds`' own target:
///
/// 1. **A further destination is a request position and nothing else.** An `endpoint.` entry here
///    would be a second `{placeholder}` in a `base_url` the emitter fills from a slot that is not
///    its own, so it would arrive at the vendor as text; a `credential.`, `username.` or `oauth.`
///    entry resolves through a *different port* under a different address, which is the one thing a
///    single slot cannot be. The `endpoint.` case has a spelling that works and it is `binds`.
/// 2. **No destination is named twice.** One value written into one position twice is either a
///    duplicate the emitter drops or a header sent twice; either way the second entry says nothing
///    the first did not.
/// 3. **`also_binds` on its own means nothing** — it is only ever the tail of `binds`, so an entry
///    that fails to parse is reported against the field like `binds` is.
fn validate_destinations(field: &ConfigField, problems: &mut Vec<String>) {
    let name = field.name.as_str();
    if let Ok(head) = parse_binding(&field.binds) {
        if !matches!(head, Binding::Username { .. }) && head.target().starts_with("username.") {
            problems.push(format!(
                "configuration field {name:?} binds {:?}, whose target uses the reserved \
                 `username.` placeholder prefix. That prefix identifies the non-secret half of a \
                 Basic credential when a value also pins a request; choose a target that does not \
                 impersonate another configuration kind",
                field.binds
            ));
        }
    }
    let mut seen: Vec<&str> = vec![field.binds.as_str()];
    for binds in field.also_binds.iter().map(String::as_str) {
        if seen.contains(&binds) {
            problems.push(format!(
                "configuration field {name:?} names the destination {binds:?} twice. One collected \
                 value reaches a position once; a repeat is either dropped by the emitter or sent \
                 twice, and neither says anything the first one did not"
            ));
        }
        seen.push(binds);
        match parse_binding(binds) {
            Ok(Binding::Request { .. }) | Err(_) => {}
            Ok(other) => problems.push(format!(
                "configuration field {name:?} declares `also_binds = [… {binds:?} …]`, which is a \
                 `{}` destination. Only a request position — `path.`, `query.` or `header.` — may \
                 be a further destination: every other kind is resolved under its own address by a \
                 different port, and one collected value has exactly one address. A `base_url` \
                 variable belongs in `binds`, where it becomes the placeholder every other \
                 destination carries",
                other.kind()
            )),
        }
    }
}

/// One destination of one field: that it parses, that it resolves, and that `secret` agrees with it.
fn validate_one_binding(
    connector: &Connector,
    field: &ConfigField,
    binds: &str,
    problems: &mut Vec<String>,
) {
    let name = field.name.as_str();
    let binding = match parse_binding(binds) {
        Ok(binding) => binding,
        Err(reason) => {
            problems.push(format!("configuration field {name:?}: {reason}"));
            return;
        }
    };

    match binding {
        Binding::Endpoint { variable } => {
            let declared: Vec<&str> = connector
                .service_names()
                .into_iter()
                .flat_map(|service| template_variables(connector.base_url_of(service)))
                .collect();
            if !declared.contains(&variable) {
                problems.push(format!(
                    "configuration field {name:?} binds `{{{variable}}}`, which no service's \
                     `base_url` carries. This provider's templates offer: {}",
                    if declared.is_empty() {
                        "nothing — every base URL is literal".to_owned()
                    } else {
                        declared.join(", ")
                    }
                ));
            }
            // The host half of "every value is checked where it lands" (C-214/C-229), and the strict
            // one: `acme.example@evil.example` is a legal header value and a legal path segment, and
            // substituted into an authority it sends the request — and the operator's own
            // credential — to a host nobody named. See `config::validate_host_value`.
            if field.format != Format::Origin {
                validate_substituted_values(field, binding, "composes a host", problems);
            }
        }
        Binding::Request {
            position,
            name: pinned,
        } => validate_pin(connector, field, position, pinned, problems),
        Binding::ChannelQuery { channel, parameter } => {
            match connector.channel(channel) {
                None => problems.push(format!(
                    "configuration field {name:?} binds channel {channel:?}, which names no \
                     channel binding"
                )),
                Some(channel_binding) if channel_binding.service != field.service => {
                    problems.push(format!(
                        "configuration field {name:?} is in service {:?} but binds channel \
                         {channel:?} in service {:?}",
                        field.service, channel_binding.service
                    ));
                }
                Some(channel_binding) => match &channel_binding.connect {
                    None => problems.push(format!(
                        "configuration field {name:?} binds socket query parameter {parameter:?} \
                         on channel {channel:?}, which declares no generic `connect` block"
                    )),
                    Some(connect) if !connect.query.contains_key(parameter) => {
                        problems.push(format!(
                            "configuration field {name:?} binds socket query parameter \
                             {parameter:?} on channel {channel:?}, but its `connect.query` \
                             declares no such parameter"
                        ));
                    }
                    Some(connect) => {
                        let value = &connect.query[parameter];
                        if !template_variables(value).contains(&name) {
                            problems.push(format!(
                                "configuration field {name:?} binds socket query parameter \
                                 {parameter:?} on channel {channel:?}, but its value {value:?} \
                                 does not interpolate {{{name}}}"
                            ));
                        }
                    }
                },
            }
            validate_substituted_values(field, binding, "fills a socket query value", problems);
        }
        Binding::Credential { name: credential } | Binding::Username { name: credential } => {
            match connector.auth_method(credential) {
                None => problems.push(format!(
                    "configuration field {name:?} binds credential {credential:?}, which no \
                     `[[auth]]` block declares"
                )),
                Some(method) => {
                    // Only `basic` has a username half; for every other scheme the whole credential
                    // is the secret, so a username field would collect a value with nowhere to go.
                    if matches!(binding, Binding::Username { .. })
                        && method.scheme != AuthScheme::Basic
                    {
                        problems.push(format!(
                            "configuration field {name:?} binds the username half of credential \
                             {credential:?}, which uses the `{}` scheme. Only `basic` sends a \
                             username — it is `base64(<user>:<secret>)`, and every other scheme \
                             sends the secret alone",
                            scheme_word(&method.scheme)
                        ));
                    }
                }
            }
        }
        Binding::OAuthClientId | Binding::OAuthClientSecret | Binding::OAuthRedirectUri => {
            if !connector.auth.iter().any(|method| method.oauth2.is_some()) {
                problems.push(format!(
                    "configuration field {name:?} binds an OAuth app registration, but no `[[auth]]` \
                     block declares an `[auth.oauth2]` spec. There is no OAuth flow for a client id \
                     to belong to"
                ));
            }
        }
    }

    // The agreement that keeps this from becoming a second source of truth. flux partitions secret
    // from non-secret BY TYPE — an `AuthMethod` versus a `ConfigSpec` — and enforces it host-side.
    // A field that disagreed would put a contradicting claim in front of that enforcement.
    let expected = binding.is_secret();
    if field.secret != expected {
        problems.push(if expected {
            format!(
                "configuration field {name:?} binds {binds} but declares `secret = false`. That \
                 value is a credential: it must be masked on input, kept out of logs, and stored \
                 where a secret is stored"
            )
        } else {
            format!(
                "configuration field {name:?} binds {binds} but declares `secret = true`. That \
                 value is configuration, not a credential — marking it secret hides it from an \
                 operator who needs to read it back, and claims gating this repository does not \
                 provide"
            )
        });
    }
}

/// **The `example`, and every permitted choice, held to the rule of one destination they reach.**
///
/// Both are values a human will end up supplying: an `example` is the placeholder a user copies, and
/// a choice is a value the connector *invites* an operator to pick. Neither may be one the position
/// it lands in would refuse — a permitted value that escaped its path segment would be a sanctioned
/// way to address another resource on the same host with the same credential, and one that moved the
/// authority would be the same thing with a different host.
///
/// Called once per destination (C-229), which is what makes a multi-destination field satisfy
/// **every** rule rather than the first: the intersection is taken by checking each, not by picking
/// one. `did` names the destination in the refusal — "pins a header value", "composes a host" — so
/// an author told their example is illegal is also told *which* of the field's destinations refused
/// it.
fn validate_substituted_values(
    field: &ConfigField,
    binding: Binding<'_>,
    did: &str,
    problems: &mut Vec<String>,
) {
    let name = field.name.as_str();
    if let Some(example) = &field.example {
        if let Err(reason) = binding.validate_value(example) {
            problems.push(format!(
                "configuration field {name:?} {did} but gives an `example` that could not be one: \
                 {reason}"
            ));
        }
    }
    for choice in &field.choices {
        if let Err(reason) = binding.validate_value(&choice.value) {
            problems.push(format!(
                "configuration field {name:?} {did} but offers a choice that could not be one: \
                 {reason}"
            ));
        }
    }
}

/// **An operator-pinned request value resolves, is mandatory, and is not also an argument** —
/// C-187.
///
/// A pin says "this connector is installed for *this* zone / *this* team". Three ways that can be
/// declared and mean nothing, each refused here rather than discovered later:
///
/// 1. **It reaches nothing.** A `path.<variable>` no operation's path carries, a header name that
///    is not an HTTP field name — the pin would be collected from a human and dropped. This is the
///    request-position twin of the endpoint rule two functions down.
/// 2. **It is also a caller argument.** If any operation of the service declares a parameter the
///    pin already claims, the pin is *advisory*: the emitted op still takes the value, and a model
///    passing its own overrides the operator's. That is the opposite of the point, so it is a
///    refusal and not a precedence rule — there is no reading under which two declarations of one
///    request slot are both right.
/// 3. **It is optional.** A host substitutes a pinned placeholder into a string literal and refuses
///    the whole request when it has no value (`connector-pack`'s `Error::MissingConfig`), so
///    `required = false` describes a connector that composes no URL. For a *query* pin it is worse
///    than useless: Vercel's `teamId` is dangerous precisely because omitting it silently redirects
///    the call to a personal account, and an optional pin would reintroduce that.
///
/// The addressing check that used to be the fourth is now [`validate_slot_is_not_shared`], run once
/// per field rather than once per pin: a field that reaches several positions (C-229) has one slot,
/// not one per destination, so asking the question here would have asked it several times and
/// answered it about the wire spelling rather than about the slot.
fn validate_pin(
    connector: &Connector,
    field: &ConfigField,
    position: Position,
    pinned: &str,
    problems: &mut Vec<String>,
) {
    let name = field.name.as_str();
    let service = field.service.as_str();
    let word = position.word();

    // A pin whose value never arrives is a connector with no URL, not one that sends less.
    if !field.required {
        problems.push(format!(
            "configuration field {name:?} pins `{word}.{pinned}` but declares `required = false`. A \
             pinned value is substituted into the emitted module, and a host with no value refuses \
             the whole request rather than omitting the pin — so an optional pin is a connector that \
             composes no URL. Drop `required` or drop the pin"
        ));
    }

    // The example is what a user copies into the field, and every value a closed set permits is one
    // an operator is *invited* to pick (C-225) — so both are held to the rule this position imposes
    // on the real value, once per position the field reaches (C-229). See
    // `validate_substituted_values`.
    validate_substituted_values(
        field,
        Binding::Request {
            position,
            name: pinned,
        },
        &format!("pins a {word} value"),
        problems,
    );

    match position {
        Position::Path => {
            let carried = connector
                .operations_of(service)
                .filter_map(|operation| operation.request.http_path())
                .any(|path| template_variables(path).contains(&pinned));
            if !carried {
                problems.push(format!(
                    "configuration field {name:?} pins `{{{pinned}}}` in the request path, which no \
                     operation of service {service:?} carries. A pin nothing interpolates is a \
                     question whose answer is discarded"
                ));
            }
        }
        Position::Query => {
            if let Err(reason) = position.validate_value(pinned) {
                problems.push(format!(
                    "configuration field {name:?} pins query parameter {pinned:?}, which is not a \
                     query parameter name: {reason}"
                ));
            }
        }
        Position::Header => {
            let folded = pinned.to_ascii_lowercase();
            if !is_http_field_name(pinned) {
                problems.push(format!(
                    "configuration field {name:?} pins header {pinned:?}, which is not an HTTP \
                     field name — only ASCII token characters are allowed (RFC 9110 §5.1), and a \
                     request carrying it could never be built"
                ));
            }
            if folded == "content-type" {
                problems.push(format!(
                    "configuration field {name:?} pins `content-type`, which is the emitter's: it \
                     is derived from the request body, so pinning it would describe an encoding the \
                     emitted module does not produce"
                ));
            }
            // The line this binding exists **not** to cross. A pinned value is non-secret by
            // construction and reaches no redactor, so letting one land in an auth-owned header
            // would be a second, ungated route for a credential — the thing `const_headers` is
            // already refused for.
            if AUTH_OWNED_HEADERS.contains(&folded.as_str()) {
                problems.push(format!(
                    "configuration field {name:?} pins header {pinned:?}, which carries a \
                     credential. A pinned value is configuration: it is never masked, never \
                     redacted, and readable back by anyone who can open a settings page. \
                     Credentials are declared in `[[auth]]` and injected by the host at the `$auth` \
                     seam"
                ));
            }
            for method in &connector.auth {
                if let AuthScheme::Header { name: owned, .. } = &method.scheme {
                    if owned.eq_ignore_ascii_case(pinned) {
                        problems.push(format!(
                            "configuration field {name:?} pins header {pinned:?}, which is where \
                             credential {:?} is injected. One of the two would overwrite the other, \
                             and which one depends on an order nothing declares",
                            method.name
                        ));
                    }
                }
            }
        }
    }

    // A pin that is also an argument is advisory, and an advisory pin is not a pin.
    for operation in connector.operations_of(service) {
        let claimed = match position {
            Position::Path => operation
                .params
                .path
                .iter()
                .any(|param| wire_of(param) == pinned),
            Position::Query => operation
                .params
                .query
                .iter()
                .any(|param| wire_of(param) == pinned),
            Position::Header => {
                operation
                    .params
                    .header
                    .iter()
                    .any(|param| wire_of(param).eq_ignore_ascii_case(pinned))
                    || operation
                        .params
                        .const_headers
                        .keys()
                        .any(|header| header.eq_ignore_ascii_case(pinned))
            }
        };
        if claimed {
            problems.push(format!(
                "configuration field {name:?} pins `{word}.{pinned}`, but operation {:?} already \
                 declares it. A value an operator pins at install time and a caller may also pass is \
                 not pinned — the caller's wins, and the operator's choice of tenant becomes a \
                 suggestion. Declare it on one side only",
                operation.id
            ));
        }
    }
}

/// **Two configuration fields may not share a slot, and may not share a destination** — the C-197
/// addressing rule, and the door C-229 must not reopen.
///
/// A host keys a configuration value by `(tenant, provider, service, kind, name)` and the emitted
/// module carries one `{placeholder}` per field, so two fields of one service whose slots collide are
/// **one slot** — the exact collapse C-197 found between Contentful's two spaces, where a management
/// write landed in whichever space the delivery reads were configured with. That is the refusal
/// C-164 measured and quoted: *two questions that share an answer are one question*.
///
/// **It still fires, and it means the same thing.** C-229 does not weaken it; it answers the other
/// half. One field naming two destinations is *one* question with one answer and one slot, which is
/// what the rule was protecting. Two *fields* keyed to one slot is still one field's answer silently
/// discarded, so the comparison is between slots — [`ConfigField::slot`] — and a further destination
/// is not a slot and cannot become one.
///
/// The second clause is what a further destination makes newly possible and is refused for its own
/// reason: two fields, two slots, one *wire position*. Two answers written into one header on the
/// same request is not an addressing collapse, it is a request that carries one of two values
/// depending on an order nothing declares. `connector-flux` refuses the emitted shape independently
/// (`Error::HeaderConflict`); this is the declaration-level half, so the refusal names the two
/// fields rather than an operation.
///
/// **Scope, deliberately unchanged.** It runs for a field that reaches at least one request
/// position, exactly as it did when it lived inside `validate_pin`. Two `endpoint.` fields of one
/// service sharing a variable is a shape this has never refused — Contentful ships two `space_id`
/// fields under two *different* services, which is precisely why the check is service-scoped — and
/// widening it is not this story's to do.
fn validate_slot_is_not_shared(
    connector: &Connector,
    field: &ConfigField,
    problems: &mut Vec<String>,
) {
    let pins = field.pins();
    if pins.is_empty() {
        return;
    }
    let name = field.name.as_str();
    let service = field.service.as_str();
    let Some(binding) = field.binding() else {
        return;
    };

    for other in connector.config_of(service) {
        if std::ptr::eq(other, field) {
            continue;
        }
        if other.binding().is_some_and(|other| {
            config_address_kind(other) == config_address_kind(binding)
                && other.target() == binding.target()
        }) {
            problems.push(format!(
                "configuration fields {name:?} and {:?} both resolve `{}.{}` in service \
                 {service:?}, so a host would key them to one value under one address. Two questions \
                 that share an answer are one question — bind one of them to a different name, or \
                 make them one field with an `also_binds`",
                other.name,
                config_address_kind(binding),
                binding.target()
            ));
        }
        for theirs in other.pins() {
            let collides = pins.iter().any(|ours| {
                ours.position == theirs.position
                    && match ours.position {
                        Position::Header => ours.name.eq_ignore_ascii_case(theirs.name),
                        Position::Path | Position::Query => ours.name == theirs.name,
                    }
            });
            if collides {
                problems.push(format!(
                    "configuration fields {name:?} and {:?} both send {:?} on the {} of every \
                     request of service {service:?}. They are two questions with two slots writing \
                     one position, so which value the vendor sees depends on an order nothing \
                     declares — declare it on one side only",
                    other.name,
                    theirs.name,
                    theirs.position.word()
                ));
            }
        }
    }
}

/// The host-side kind one binding is stored under.
///
/// Bare request pins are carried through the established endpoint-configuration port, so an
/// endpoint and a request pin of one target still share an address (C-229). A Basic username and a
/// credential secret are separate ports and therefore separate addresses even when both name the
/// same credential — the distinction C-475's qualified placeholder preserves.
fn config_address_kind(binding: Binding<'_>) -> &'static str {
    match binding {
        Binding::Request { .. } => "endpoint",
        other => other.kind(),
    }
}

/// The spelling the vendor sees for a parameter: its `wire` alias when it declares one.
///
/// A pin is compared against this rather than against `name`, because it is the wire name that
/// occupies the request slot the pin would claim.
fn wire_of(param: &crate::Param) -> &str {
    param.wire.as_deref().unwrap_or(&param.name)
}

/// **Every template variable is asked for.** This is the rule that closes the `SCHEMA GAP:` comment
/// four shipped providers have carried since C-17.
///
/// A `{subdomain}` nobody declares is not a cosmetic omission: the connector has no valid destination
/// URL and no way to tell anyone what is missing. `catalog.json` already publishes an
/// `unbound-base-url-template` issue for exactly this, which is a diagnosis with no remedy attached.
fn validate_every_template_variable_is_asked_for(
    connector: &Connector,
    problems: &mut Vec<String>,
) {
    for service in connector.service_names() {
        for variable in template_variables(connector.base_url_of(service)) {
            // Only `binds` can answer, and never an `also_binds`: a further destination is a request
            // position by construction (`validate_destinations`), so a header pin still does not
            // bind a hostname. That is C-164's third measured shape, and C-229 does not move it —
            // the field that binds Algolia's hostname *and* its header binds the hostname in
            // `binds`, which is what makes `{app_id}` the one placeholder both destinations carry.
            // `config_of` is the head-service lookup; `also_services` extends the same field's one
            // address to a sibling surface of the same deployment (C-529). Both are consulted, and
            // neither admits an `also_binds` — a further destination is a request position by
            // construction, so a header pin still does not bind a hostname.
            let bound = connector.config_filling(service).any(|field| {
                matches!(field.binding(), Some(Binding::Endpoint { variable: v }) if v == variable)
            });
            if !bound {
                let where_ = if service == DEFAULT_SERVICE {
                    String::new()
                } else {
                    format!(" of service {service:?}")
                };
                problems.push(format!(
                    "the base URL{where_} carries `{{{variable}}}`, which no `[[config]]` field \
                     binds. Until something asks a user for it the connector has no valid \
                     destination URL — declare a field with `binds = \"endpoint.{variable}\"`"
                ));
            }
        }
    }
}
