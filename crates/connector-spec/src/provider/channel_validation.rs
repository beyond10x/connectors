use super::*;

/// Checks the declared verification operation — a host's "Test connection".
pub(super) fn validate_verify(connector: &Connector, problems: &mut Vec<String>) {
    let Some(verify) = &connector.verify else {
        return;
    };
    match connector.operation(verify) {
        None => problems.push(format!(
            "`verify` names operation {verify:?}, which no `[[operations]]` block declares"
        )),
        // A "Test connection" button that could change vendor state is a button nobody dares press.
        // Direction is connector truth; neither method nor risk may stand in for it.
        Some(operation) if operation.direction == OperationDirection::Write => {
            problems.push(format!(
                "`verify` names operation {verify:?}, which declares `direction = \"write\"`. A \
                 connection test runs unattended whenever someone opens a settings page, so it must \
                 be a read a user would not mind being repeated"
            ));
        }
        Some(_) => {}
    }
}

/// Checks that discovery is a closed interpretation of one bounded read, never an authority or a
/// caller-selected proxy surface.
pub(super) fn validate_discoveries(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen_ids: Vec<&str> = Vec::new();

    for discovery in &connector.discoveries {
        let id = discovery.id.as_str();
        if let Err(reason) = crate::address::validate_member_name(id) {
            problems.push(format!("discovery {id:?} has an invalid `id`: {reason}"));
        } else if seen_ids.contains(&id) {
            problems.push(format!("discovery {id:?} is declared more than once"));
        }
        seen_ids.push(id);

        validate_member_service(connector, "discovery", id, &discovery.service, problems);

        match connector.operation(&discovery.operation) {
            None => problems.push(format!(
                "discovery {id:?} names operation {:?}, which no `[[operations]]` block declares",
                discovery.operation
            )),
            Some(operation) => {
                if operation.service != discovery.service {
                    problems.push(format!(
                        "discovery {id:?} belongs to service {:?} but operation {:?} belongs to {:?}",
                        discovery.service, discovery.operation, operation.service
                    ));
                }
                if operation.direction != OperationDirection::Read
                    || operation.interaction_shape != InteractionShape::Unary
                    || !matches!(operation.request, OperationRequest::HttpV1 { .. })
                    || !operation.effects.contains(&HostEffect::Read)
                    || !operation.effects.contains(&HostEffect::Network)
                {
                    problems.push(format!(
                        "discovery {id:?} operation {:?} must be a unary HTTP read with explicit read and network effects",
                        discovery.operation
                    ));
                }
            }
        }

        if discovery.mappings.is_empty() {
            problems.push(format!(
                "discovery {id:?} has no mappings; an observation parser with no closed target Provider mapping cannot produce a usable candidate"
            ));
        }

        let mut observed_types: Vec<&str> = Vec::new();
        for mapping in &discovery.mappings {
            let observed_type = mapping.observed_type.as_str();
            if !valid_discovery_token(observed_type) {
                problems.push(format!(
                    "discovery {id:?} has invalid `observed_type` {observed_type:?}; use 1..128 lowercase ASCII letters, digits, `.`, `_`, or `-`"
                ));
            } else if observed_types.contains(&observed_type) {
                problems.push(format!(
                    "discovery {id:?} maps observed type {observed_type:?} more than once"
                ));
            }
            observed_types.push(observed_type);

            if !valid_discovery_token(&mapping.target_provider) {
                problems.push(format!(
                    "discovery {id:?} has invalid target Provider {:?}",
                    mapping.target_provider
                ));
            }

            if !matches!(
                (discovery.driver, mapping.route_adapter),
                (
                    DiscoveryDriver::GrafanaDatasourceV1,
                    RouteAdapter::GrafanaDatasourceProxyV1
                )
            ) {
                problems.push(format!(
                    "discovery {id:?} driver {:?} is incompatible with route adapter {:?}",
                    discovery.driver.word(),
                    mapping.route_adapter.word()
                ));
            }
        }
    }
}

fn valid_discovery_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

/// Checks the inbound half of a service's members.
///
/// Name spelling and service membership only — an event declares no behaviour of its own, so there is
/// nothing else here to be wrong. What *uses* an event is a [`ChannelBinding`], and the
/// cross-references are checked there.
pub(super) fn validate_events(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for event in &connector.events {
        let name = event.name.as_str();
        if name.trim().is_empty() {
            problems.push("an event has an empty `name`".to_owned());
            continue;
        }
        if seen.contains(&name) {
            problems.push(format!(
                "event {name:?} is declared more than once; the event name is the trigger label a \
                 program matches on, so it must denote one event"
            ));
        }
        seen.push(name);

        if let Err(reason) = crate::address::validate_member_name(name) {
            problems.push(format!("event {name:?} has an invalid `name`: {reason}"));
        }
        validate_member_service(connector, "event", name, &event.service, problems);
        validate_requirements(
            connector,
            connector.effective_event_auth(event),
            &format!("event {name:?}"),
            problems,
        );
    }
}

/// Checks that a member's service is one this provider has.
///
/// The operation-side equivalent is [`validate_operation_service`], which stays separate because its
/// error text names the multi-service trap specifically; this is the shorter form the other two
/// kinds need.
pub(super) fn validate_member_service(
    connector: &Connector,
    kind: &str,
    name: &str,
    service: &str,
    problems: &mut Vec<String>,
) {
    let available = connector.service_names();
    if available.contains(&service) {
        return;
    }
    problems.push(format!(
        "{kind} {name:?} names service {service:?}, which no `[[services]]` entry declares. This \
         provider declares: {}",
        available.join(", ")
    ));
}

/// Checks every channel binding: its transport's own rules, and every reference it makes.
///
/// **Every rule here is a refusal, never a degradation.** A binding is a promise that an event can
/// reach a flow and that a reply can go back; a binding that half-holds is the plausible-but-wrong
/// artifact `AGENTS.md` requires the pipeline to refuse rather than emit.
pub(super) fn validate_channels(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for channel in &connector.channels {
        let name = channel.name.as_str();
        if name.trim().is_empty() {
            problems.push("a channel binding has an empty `name`".to_owned());
            continue;
        }
        if seen.contains(&name) {
            problems.push(format!(
                "channel binding {name:?} is declared more than once; the binding name is what an \
                 operator's `channel` declaration selects, so it must denote one surface"
            ));
        }
        seen.push(name);

        if let Err(reason) = crate::address::validate_member_name(name) {
            problems.push(format!(
                "channel binding {name:?} has an invalid `name`: {reason}"
            ));
        }
        validate_member_service(
            connector,
            "channel binding",
            name,
            &channel.service,
            problems,
        );

        validate_channel_events(connector, channel, problems);
        validate_channel_verification(connector, channel, problems);
        validate_channel_auth(connector, channel, problems);
        validate_socket_connect(connector, channel, problems);
        validate_channel_payload(channel, problems);
        validate_channel_reply(connector, channel, problems);
        validate_channel_transport(connector, channel, problems);
        validate_channel_setup(connector, channel, problems);
        validate_session_binding(channel, problems);

        for (label, selector) in [
            ("discriminator", &channel.discriminator),
            ("delivery_id", &channel.delivery_id),
        ] {
            if let Some(selector) = selector {
                validate_selector(name, label, selector, problems);
            }
        }
    }
}

fn validate_channel_auth(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    validate_requirements(
        connector,
        &channel.auth,
        &format!("channel binding {:?}", channel.name),
        problems,
    );
    if channel.transport == Transport::Webhook && !channel.auth.is_empty() {
        problems.push(format!(
            "channel binding {:?} declares `auth` on a webhook. Inbound webhook trust belongs in \
             `verification`; channel `auth` is credential custody for a host-established transport",
            channel.name
        ));
    }
}

/// Validate the declarative RFC 6455 handshake without resolving a host or reading credentials.
fn validate_socket_connect(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();
    let Some(connect) = &channel.connect else {
        return;
    };

    if channel.transport != Transport::Socket {
        problems.push(format!(
            "channel binding {name:?} declares `connect`, which only the `socket` transport uses"
        ));
    }

    let path = connect.path.as_str();
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("://")
        || path.contains(['?', '#'])
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
    {
        problems.push(format!(
            "channel binding {name:?} declares socket path {path:?}, which is not a relative \
             WebSocket path rooted at the service `base_url`"
        ));
    }

    const HANDSHAKE_HEADERS: &[&str] = &[
        "host",
        "connection",
        "upgrade",
        "sec-websocket-key",
        "sec-websocket-version",
        "sec-websocket-protocol",
        "authorization",
    ];
    for (header, value) in &connect.headers {
        if HANDSHAKE_HEADERS
            .iter()
            .any(|reserved| header.eq_ignore_ascii_case(reserved))
        {
            problems.push(format!(
                "channel binding {name:?} fixes handshake-owned header {header:?}; the guarded \
                 host owns upgrade, subprotocol and authentication headers"
            ));
        }
        if header.is_empty()
            || !header
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            problems.push(format!(
                "channel binding {name:?} declares invalid fixed header name {header:?}"
            ));
        }
        if value.chars().any(|c| !c.is_ascii() || c.is_ascii_control())
            || value.contains(['{', '}'])
        {
            problems.push(format!(
                "channel binding {name:?} declares fixed header {header:?} with an invalid or \
                 templated value; fixed headers are public literals"
            ));
        }
    }

    let mut seen_protocols: Vec<&str> = Vec::new();
    for protocol in &connect.subprotocols {
        let valid = !protocol.is_empty()
            && protocol
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte));
        if !valid {
            problems.push(format!(
                "channel binding {name:?} declares invalid WebSocket subprotocol {protocol:?}"
            ));
        }
        if seen_protocols.contains(&protocol.as_str()) {
            problems.push(format!(
                "channel binding {name:?} declares WebSocket subprotocol {protocol:?} twice"
            ));
        }
        seen_protocols.push(protocol);
    }

    for (parameter, value) in &connect.query {
        if parameter.is_empty()
            || parameter
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || "&=?#".contains(c))
        {
            problems.push(format!(
                "channel binding {name:?} declares invalid socket query parameter {parameter:?}"
            ));
        }
        for variable in template_variables(value) {
            let declared = connector.config.iter().any(|field| {
                field.service == channel.service
                    && field.name == variable
                    && matches!(
                        field.binding(),
                        Some(Binding::ChannelQuery { channel: owner, parameter: target })
                            if owner == name && target == parameter
                    )
            });
            if !declared {
                problems.push(format!(
                    "channel binding {name:?} query parameter {parameter:?} needs configuration \
                     {{{variable}}}, but no `[[config]]` field binds \
                     `channel.{name}.query.{parameter}` under that name"
                ));
            }
        }
    }

    validate_requirements(
        connector,
        &connect.auth,
        &format!("channel binding {name:?}"),
        problems,
    );
}

/// Every event a binding carries must exist **in the binding's own service**.
fn validate_channel_events(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();

    // A push binding that carries no events delivers nothing: the transport would connect, hold, and
    // route every arrival to a label no trigger can name. A poll binding is different — its cursor
    // operation is what it carries.
    if channel.events.is_empty()
        && !matches!(channel.transport, Transport::Poll | Transport::Session)
    {
        problems.push(format!(
            "channel binding {name:?} lists no `events`, so nothing it receives could reach a \
             trigger. A binding names the events it carries; only a `poll` binding may omit them, \
             because its `cursor` operation is what it carries"
        ));
    }

    let mut seen_events: Vec<&str> = Vec::new();
    let mut seen_wire_values: Vec<&str> = Vec::new();
    for event in &channel.events {
        if seen_events.contains(&event.as_str()) {
            problems.push(format!(
                "channel binding {name:?} carries event {event:?} twice"
            ));
        }
        seen_events.push(event);
        match connector.event(event) {
            None => problems.push(format!(
                "channel binding {name:?} carries event {event:?}, which no `[[events]]` block \
                 declares"
            )),
            Some(declared) if declared.service != channel.service => problems.push(format!(
                "channel binding {name:?} is in service {:?} but carries event {event:?}, which is \
                 in service {:?}. A binding carries the events of its own service — the two version \
                 and address independently",
                channel.service, declared.service
            )),
            Some(declared) => {
                let wire = declared.wire_value.as_deref().unwrap_or(&declared.name);
                if wire.trim().is_empty() {
                    problems.push(format!(
                        "event {event:?} carried by channel {name:?} declares an empty `wire_value`"
                    ));
                }
                if seen_wire_values.contains(&wire) {
                    problems.push(format!(
                        "channel binding {name:?} maps more than one event to wire value {wire:?}; \
                         a discriminator value must select exactly one declared event"
                    ));
                }
                seen_wire_values.push(wire);
            }
        }
    }
}

/// The tri-state on [`ChannelBinding::verification`], and the HMAC parameters when there are any.
fn validate_channel_verification(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();

    match (&channel.verification, channel.transport) {
        // Silence on an open endpoint is how an unverified event gets presented as a trusted one.
        // The author must say something, even if what they say is "this vendor publishes nothing".
        (None, Transport::Webhook) => problems.push(format!(
            "channel binding {name:?} uses the `webhook` transport and states no `verification`. An \
             endpoint anyone can POST to must say how it proves the caller is the vendor — write a \
             `[[channels]].verification.hmac` table, or `verification = \"none\"` to state \
             deliberately that the vendor publishes no signature"
        )),
        (Some(scheme), transport) if transport != Transport::Webhook => {
            let _ = scheme;
            problems.push(format!(
                "channel binding {name:?} states `verification`, which only the `webhook` transport \
                 uses. A `{}` binding is authenticated by the credential that opens the connection",
                transport_word(transport)
            ));
        }
        _ => {}
    }

    if let Some(VerificationScheme::Hmac(hmac)) = &channel.verification {
        validate_hmac(connector, name, hmac, problems);
    }
}

/// A direct-byte session is not an event channel with some fields omitted. Its closed driver and
/// host-authority axes are required together, while every event-delivery-only field is refused.
fn validate_session_binding(channel: &ChannelBinding, problems: &mut Vec<String>) {
    let name = channel.name.as_str();
    match (channel.transport, channel.session.as_ref()) {
        (Transport::Session, None) => problems.push(format!(
            "channel binding {name:?} uses the `session` transport but declares no `session` \
             driver/capability facts"
        )),
        (Transport::Webhook | Transport::Socket | Transport::Poll, Some(_)) => {
            problems.push(format!(
            "channel binding {name:?} declares `session` facts, which only the `session` transport \
             uses"
        ))
        }
        (Transport::Session, Some(session)) => {
            if session.interaction_shape != InteractionShape::SessionEstablishment {
                problems.push(format!(
                    "channel binding {name:?} declares session interaction shape {:?}; the \
                     `session` transport requires `session_establishment`",
                    session.interaction_shape
                ));
            }
            if session.protocol_driver != ProtocolDriver::SipV1 {
                problems.push(format!(
                    "channel binding {name:?} selects {:?} for an inbound session; the first closed \
                     inbound session driver is `sip_v1`",
                    session.protocol_driver
                ));
            }
            if session.required_capabilities.is_empty() {
                problems.push(format!(
                    "channel binding {name:?} declares no session `required_capabilities`; absence \
                     cannot prove the endpoint may listen"
                ));
            }
            for pair in session.required_capabilities.windows(2) {
                if pair[0] >= pair[1] {
                    problems.push(format!(
                        "channel binding {name:?} has unsorted or duplicate session \
                         `required_capabilities`"
                    ));
                    break;
                }
            }

            for (field, present) in [
                ("connect", channel.connect.is_some()),
                ("auth", !channel.auth.is_empty()),
                ("events", !channel.events.is_empty()),
                ("verification", channel.verification.is_some()),
                ("discriminator", channel.discriminator.is_some()),
                ("delivery_id", channel.delivery_id.is_some()),
                ("payload", !channel.payload.is_empty()),
                ("payload_root", channel.payload_root),
                ("reply", channel.reply.is_some()),
                ("cursor", channel.cursor.is_some()),
                ("subscription", channel.subscription.is_some()),
                ("setup", channel.setup.is_some()),
                ("interval", channel.interval.is_some()),
            ] {
                if present {
                    problems.push(format!(
                        "channel binding {name:?} declares event field `{field}` on a `session` \
                         transport"
                    ));
                }
            }
        }
        (_, None) => {}
    }
}

/// [`SIGNED_PLACEHOLDERS`] as an author reads them, so the refusal lists what it will accept rather
/// than only what it rejected. Derived from the list rather than restated beside it — a hand-written
/// copy is how an error message comes to name a vocabulary that has since moved.
fn fillable_placeholders() -> String {
    let names: Vec<String> = SIGNED_PLACEHOLDERS
        .iter()
        .map(|name| format!("{{{name}}}"))
        .collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}

/// The HMAC matrix's own consistency: a fillable template, a bounded replay window, and a secret that
/// resolves to a credential declared for exactly this purpose.
fn validate_hmac(
    connector: &Connector,
    channel: &str,
    hmac: &HmacSpec,
    problems: &mut Vec<String>,
) {
    if hmac.header.trim().is_empty() {
        problems.push(format!(
            "channel binding {channel:?} declares an HMAC scheme with an empty `header`; it names \
             the header carrying the signature"
        ));
    }

    let placeholders = signed_placeholders(&hmac.signed);
    for placeholder in &placeholders {
        if !SIGNED_PLACEHOLDERS.contains(&placeholder.as_str()) {
            problems.push(format!(
                "channel binding {channel:?} has `signed = {:?}`, which interpolates \
                 {{{placeholder}}}; the host can fill only {}",
                hmac.signed,
                fillable_placeholders()
            ));
        }
    }

    // **The rule this whole struct rests on.** A template that puts no payload into the signed
    // string signs something the request never enters, so a signature captured from one delivery
    // verifies *any* forged payload — bounded only by the tolerance, and by nothing at all without
    // one. It is the same defect as the unterminated brace `signed_placeholders` reports, except
    // that reaching it needs no typo: `signed = "{timestamp}"` is well formed, and every other check
    // here passes on it. Refusing an empty template is not enough, because the hole is not emptiness.
    //
    // The test is `PAYLOAD_PLACEHOLDERS`, not the literal `{body}`, and C-188 is why: `{url}` is a
    // per-endpoint constant, so `signed = "{url}"` is this exact hole under a placeholder that
    // *looks* request-specific — and a URL-signing vendor carries no timestamp, so there is not even
    // a window bounding it.
    if !placeholders
        .iter()
        .any(|p| PAYLOAD_PLACEHOLDERS.contains(&p.as_str()))
    {
        problems.push(format!(
            "channel binding {channel:?} has `signed = {:?}`, which never interpolates {{body}} or \
             {{sorted_form}}. The signed string must cover the request payload, or a signature \
             captured from one delivery verifies every forged payload that follows it — the \
             signature would prove only that somebody, once, held the secret",
            hmac.signed
        ));
    }

    let timestamped = placeholders.iter().any(|p| p == "timestamp");

    match (&hmac.timestamp, timestamped) {
        (None, true) => problems.push(format!(
            "channel binding {channel:?} signs over {{timestamp}} but declares no `timestamp` \
             selector. The template says the value is signed; it cannot say where the value is \
             read from, and a host left to guess would fall back to its own clock — which verifies \
             nothing"
        )),
        (Some(_), false) => problems.push(format!(
            "channel binding {channel:?} declares a `timestamp` selector, but its `signed` template \
             does not interpolate {{timestamp}} — the value would be read and never used"
        )),
        (Some(selector), true) => {
            validate_selector(channel, "verification timestamp", selector, problems);
            // Reading the timestamp out of the body inverts the order that makes verification mean
            // anything: the body would have to be parsed to find the value that decides whether the
            // body is trustworthy, which exposes a parser to any anonymous caller. flux refuses it
            // in its own request path; refusing it here puts the failure in a build instead.
            if selector.source == FieldSource::Body {
                problems.push(format!(
                    "channel binding {channel:?} reads its verification timestamp from the body \
                     ({:?}). A body-sourced timestamp has to be parsed *before* the bytes carrying \
                     it are verified, which inverts the order verification depends on; a signed \
                     timestamp is read from a header",
                    selector.name
                ));
            }
        }
        (None, false) => {}
    }

    // A timestamped scheme with no window is a signature that replays forever — strictly worse than
    // not timestamping at all, because it reads as though replay had been handled.
    match (&hmac.tolerance, timestamped) {
        (None, true) => problems.push(format!(
            "channel binding {channel:?} signs over {{timestamp}} but declares no `tolerance`. A \
             timestamped signature with no window replays forever; state how old a request may be, \
             as in `tolerance = \"5m\"`"
        )),
        (Some(_), false) => problems.push(format!(
            "channel binding {channel:?} declares a `tolerance`, but its `signed` template does not \
             interpolate {{timestamp}} — there is no timestamp to bound"
        )),
        // Requiring a window is not the same as having one. An unparseable spelling leaves the real
        // window to whatever each host decides at runtime, while reading exactly as though replay
        // had been handled.
        (Some(tolerance), true) => {
            if let Err(reason) = parse_tolerance(tolerance) {
                problems.push(format!(
                    "channel binding {channel:?} declares `tolerance = {tolerance:?}`, which is not \
                     a window a host can apply: {reason}"
                ));
            }
        }
        (None, false) => {}
    }

    // The spelling of a value nothing reads describes nothing — the same objection as an unused
    // selector or an unused window.
    if !timestamped && hmac.timestamp_format.is_some() {
        problems.push(format!(
            "channel binding {channel:?} declares a `timestamp_format`, but its `signed` template \
             does not interpolate {{timestamp}} — there is no timestamp to spell"
        ));
    }

    match connector.auth_method(&hmac.secret) {
        None => problems.push(format!(
            "channel binding {channel:?} names webhook secret {:?}, which no `[[auth]]` block \
             declares. An inbound secret is a credential like any other, so that the manifest names \
             every credential this connector requires",
            hmac.secret
        )),
        Some(method) if method.scheme != AuthScheme::Signing => problems.push(format!(
            "channel binding {channel:?} names webhook secret {:?}, which is declared with the \
             `{}` scheme. A verification secret is never placed in an outgoing request, so it is \
             declared `scheme = \"signing\"` — using an outbound credential here would spend the \
             same value in both directions",
            hmac.secret,
            scheme_word(&method.scheme)
        )),
        Some(_) => {}
    }
}

/// A payload map binds Flux symbols to dotted paths, and both halves have to be spellable.
fn validate_channel_payload(channel: &ChannelBinding, problems: &mut Vec<String>) {
    let name = channel.name.as_str();
    if channel.payload_root && !channel.payload.is_empty() {
        problems.push(format!(
            "channel binding {name:?} declares `payload_root = true` and a `payload` projection. \
             A delivery is either the complete JSON event or one projected object, never both"
        ));
    }
    for (symbol, path) in &channel.payload {
        if let Err(reason) = validate_symbol(symbol) {
            problems.push(format!("channel binding {name:?}: {reason}"));
        }
        if let Err(reason) = validate_path(path) {
            problems.push(format!(
                "channel binding {name:?} maps {symbol:?} to an invalid source path: {reason}"
            ));
        }
    }
}

/// The reply must resolve, and it must be **completely** bound.
///
/// The completeness rule is the one that earns its keep. A reply missing a required parameter builds,
/// ships, passes every artifact check, and then fails on the first real delivery — at which point the
/// failure is in an operator's production channel rather than in a build they were reading.
fn validate_channel_reply(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();
    let Some(Reply {
        operation,
        result,
        bind,
    }) = &channel.reply
    else {
        return;
    };

    let Some(target) = connector.operation(operation) else {
        problems.push(format!(
            "channel binding {name:?} replies with operation {operation:?}, which no \
             `[[operations]]` block declares. A binding's reply is an ordinary operation of this \
             same connector — that is what makes it a composition rather than a second code path"
        ));
        return;
    };

    if target.service != channel.service {
        problems.push(format!(
            "channel binding {name:?} is in service {:?} but replies with operation {operation:?}, \
             which is in service {:?}",
            channel.service, target.service
        ));
    }

    for (param, symbol) in bind {
        if !target.params.iter().any(|p| &p.name == param) {
            problems.push(format!(
                "channel binding {name:?} binds reply parameter {param:?}, which operation \
                 {operation:?} does not declare"
            ));
        }
        if !channel.payload.contains_key(symbol) {
            problems.push(format!(
                "channel binding {name:?} binds reply parameter {param:?} to {symbol:?}, which its \
                 `payload` map does not declare. A reply is filled from the inbound payload, so \
                 every bound value has to be something the payload produced"
            ));
        }
    }

    if let Some(result) = result {
        if !target.params.iter().any(|p| &p.name == result) {
            problems.push(format!(
                "channel binding {name:?} sends its journey result to reply parameter {result:?}, \
                 which operation {operation:?} does not declare"
            ));
        }
        if bind.contains_key(result) {
            problems.push(format!(
                "channel binding {name:?} both binds reply parameter {result:?} from the payload \
                 and sends the journey result to it. One parameter carries one value — decide which"
            ));
        }
    }

    for param in target.params.iter().filter(|p| p.required) {
        let covered =
            bind.contains_key(&param.name) || result.as_deref() == Some(param.name.as_str());
        if !covered {
            problems.push(format!(
                "channel binding {name:?} replies with operation {operation:?} but leaves its \
                 required parameter {:?} unbound. Bind it from the `payload` map, or name it as \
                 `result` if it carries the journey's own output — every required parameter is \
                 settled at build time, or the reply fails on the first delivery instead of in this \
                 diff",
                param.name
            ));
        }
    }
}

/// `cursor` and `interval` belong to `poll`, and `poll` cannot do without a cursor.
///
/// See [`crate::inbound`] for the reasoning: flux's cron drops ticks across a restart and replays
/// none of them, so a poll that cannot resume from a recorded position loses events silently.
fn validate_channel_transport(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();

    if channel.transport == Transport::Poll {
        match &channel.cursor {
            None => problems.push(format!(
                "channel binding {name:?} uses the `poll` transport and declares no `cursor`. flux's \
                 schedule channel is best-effort — a restart drops ticks and replays none of them — \
                 so the cursor operation, not the interval, is what makes a poll correct. Name the \
                 operation that reads forward from a recorded position"
            )),
            Some(cursor) => match connector.operation(cursor) {
                None => problems.push(format!(
                    "channel binding {name:?} names cursor operation {cursor:?}, which no \
                     `[[operations]]` block declares"
                )),
                Some(target) if target.service != channel.service => problems.push(format!(
                    "channel binding {name:?} is in service {:?} but names cursor operation \
                     {cursor:?}, which is in service {:?}",
                    channel.service, target.service
                )),
                Some(_) => {}
            },
        }
    } else {
        for (field, present) in [
            ("cursor", channel.cursor.is_some()),
            ("interval", channel.interval.is_some()),
        ] {
            if present {
                problems.push(format!(
                    "channel binding {name:?} declares `{field}`, which only the `poll` transport \
                     uses. A `{}` binding is woken by the vendor, not by a schedule",
                    transport_word(channel.transport)
                ));
            }
        }
    }
}

/// How a binding gets registered — and the rule that a webhook must say.
///
/// A product that knows a callback URL and nothing about what to do with it cannot finish an
/// installation. That is the same shape as the verification rule: an open endpoint has to state
/// something, and silence is not one of the options.
fn validate_channel_setup(
    connector: &Connector,
    channel: &ChannelBinding,
    problems: &mut Vec<String>,
) {
    let name = channel.name.as_str();

    if channel.transport == Transport::Webhook
        && channel.subscription.is_none()
        && channel.setup.is_none()
    {
        problems.push(format!(
            "channel binding {name:?} uses the `webhook` transport and says neither how to register \
             it nor what a human must do. A product can show a callback URL, but with no \
             `[channels.subscription]` naming the operation that registers it and no \
             `[channels.setup]` steps to follow, nobody can finish connecting it"
        ));
    }

    // API registration names where the vendor should deliver to our webhook. An outbound socket
    // has no callback endpoint, but may still need manual vendor-side mode, scopes, subscriptions,
    // and installation. Poll and direct-session setup belongs to their Integration/Connection.
    if channel.subscription.is_some() && channel.transport != Transport::Webhook {
        problems.push(format!(
            "channel binding {name:?} declares `subscription`, which only the `webhook` transport \
             uses. A `{}` binding has no endpoint for the vendor to register",
            transport_word(channel.transport)
        ));
    }
    if channel.setup.is_some()
        && !matches!(channel.transport, Transport::Webhook | Transport::Socket)
    {
        problems.push(format!(
            "channel binding {name:?} declares `setup`, which only `webhook` and `socket` \
             transports use. A `{}` binding is configured through its own runtime policy",
            transport_word(channel.transport)
        ));
    }

    if let Some(Subscription {
        subscribe,
        unsubscribe,
        list,
        callback_param,
    }) = &channel.subscription
    {
        for (label, id) in [
            ("subscribe", Some(subscribe)),
            ("unsubscribe", unsubscribe.as_ref()),
            ("list", list.as_ref()),
        ] {
            let Some(id) = id else { continue };
            match connector.operation(id) {
                None => problems.push(format!(
                    "channel binding {name:?} names `{label}` operation {id:?}, which no \
                     `[[operations]]` block declares. Registering a webhook is an ordinary \
                     authorized write, so it is an ordinary operation"
                )),
                Some(target) if target.service != channel.service => problems.push(format!(
                    "channel binding {name:?} is in service {:?} but names `{label}` operation \
                     {id:?}, which is in service {:?}",
                    channel.service, target.service
                )),
                Some(_) => {}
            }
        }

        // The callback URL is the product's, and this names where to put it. A parameter that does
        // not exist means the URL would be assembled into a request that drops it.
        if let Some(target) = connector.operation(subscribe) {
            if !target.params.iter().any(|p| &p.name == callback_param) {
                problems.push(format!(
                    "channel binding {name:?} sends its callback URL to parameter \
                     {callback_param:?}, which operation {subscribe:?} does not declare"
                ));
            }
        }
    }

    if let Some(ManualSetup { steps, .. }) = &channel.setup {
        if steps.is_empty() {
            problems.push(format!(
                "channel binding {name:?} declares `[channels.setup]` with no `steps`. An empty \
                 instruction list is the same as no instructions, stated more confidently"
            ));
        }
        for step in steps {
            if step.trim().is_empty() {
                problems.push(format!("channel binding {name:?} has an empty setup step"));
            }
        }
    }
}

/// A selector reads one named value off an inbound request; a body selector addresses it by path.
fn validate_selector(channel: &str, label: &str, selector: &Selector, problems: &mut Vec<String>) {
    if selector.name.trim().is_empty() {
        problems.push(format!(
            "channel binding {channel:?} has a `{label}` with an empty `name`"
        ));
        return;
    }
    if selector.source == crate::inbound::FieldSource::Body {
        if let Err(reason) = validate_path(&selector.name) {
            problems.push(format!(
                "channel binding {channel:?} has a `{label}` reading an invalid body path: {reason}"
            ));
        }
    }
}
