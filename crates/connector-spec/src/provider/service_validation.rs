use super::*;

/// The three member kinds share one namespace per service — see [`Connector::member_names_of`].
///
/// **Cross-kind collisions only.** A name repeated *within* one kind is already reported by that
/// kind's own pass, in its own vocabulary ("the op id is the public name callers and models use"),
/// and reporting it twice would make an author fix one problem and see two. What no single pass can
/// see is an operation and an event that happen to share a name — neither list has a duplicate, and
/// only the union does.
pub(super) fn validate_member_namespace(connector: &Connector, problems: &mut Vec<String>) {
    for service in connector.service_names() {
        // (name, kind), in the order `member_names_of` yields them.
        let mut seen: Vec<(&str, &str)> = Vec::new();
        let mut reported: Vec<&str> = Vec::new();

        let members = connector
            .operations_of(service)
            // The labels carry their own article, because they are interpolated into a sentence that
            // reads "names both {other} and {kind}" — "a operation" and "a event" otherwise.
            .map(|operation| (operation.id.as_str(), "an operation"))
            .chain(
                connector
                    .events_of(service)
                    .map(|event| (event.name.as_str(), "an event")),
            )
            .chain(
                connector
                    .channels_of(service)
                    .map(|channel| (channel.name.as_str(), "a channel binding")),
            )
            .chain(
                connector
                    .config_of(service)
                    .map(|field| (field.name.as_str(), "a configuration field")),
            )
            .chain(
                connector
                    .graphs_of(service)
                    .map(|graph| (graph.name.as_str(), "a graph")),
            );

        for (name, kind) in members {
            if let Some((_, other)) = seen
                .iter()
                .find(|(seen_name, seen_kind)| *seen_name == name && *seen_kind != kind)
            {
                if !reported.contains(&name) {
                    let where_ = if service == DEFAULT_SERVICE {
                        String::new()
                    } else {
                        format!(" of service {service:?}")
                    };
                    problems.push(format!(
                        "{name:?} names both {other} and {kind}{where_}. The three member kinds \
                         share one namespace: all of them render into the same address \
                         (`…#{name}`) and into flux's declaration namespace, so a name has to \
                         denote exactly one thing"
                    ));
                    reported.push(name);
                }
            }
            seen.push((name, kind));
        }
    }
}

/// The `kind` word for a transport, for error text.
pub(super) fn transport_word(transport: Transport) -> &'static str {
    match transport {
        Transport::Webhook => "webhook",
        Transport::Socket => "socket",
        Transport::Poll => "poll",
        Transport::Session => "session",
    }
}

/// The `scheme` word for a credential, for error text.
pub(super) fn scheme_word(scheme: &AuthScheme) -> &'static str {
    match scheme {
        AuthScheme::Bearer => "bearer",
        AuthScheme::Basic => "basic",
        AuthScheme::Header { .. } => "header",
        AuthScheme::Query { .. } => "query",
        AuthScheme::Signing => "signing",
    }
}

/// Checks the connector's address components and its `[[services]]` declarations — C-49.
///
/// The operation-side half of the rule (every operation belongs to a declared service) is in
/// [`validate_operations`], because that is where an operation is already being read.
///
/// # Why the grammar is enforced *here*
///
/// The [`address`](crate::address) module owns the spelling of an authority, a service name and an
/// API version, and this is the only place that can refuse a bad one while the author is still
/// looking at the file. Two things go wrong if it does not:
///
/// 1. **A service name reaches the output filesystem path.** It names the emitted
///    `<provider>-<service>.flux`, and a build creates that file's parent directories. A name
///    carrying `/` or `..` would therefore let a *content* field of a provider TOML decide where a
///    build writes — including outside the repository root. Before services existed, no content field
///    could influence an output path at all: paths came from the discovered file stem. That invariant
///    is worth keeping, and keeping it costs one call to a validator that already exists.
/// 2. **An unspellable component publishes a malformed address.** [`Connector::gid_of`] renders
///    whatever the loader accepted, and that string reaches every service manifest and
///    `catalog.json`. An authority of `com.acme/s3` renders `com.acme/s3:v2`, which *reparses* — as a
///    different address. That is exactly the "a typo in a segment cannot masquerade as a valid
///    address" property the address module claims, and only validation here makes the claim true.
pub(super) fn validate_services(connector: &Connector, problems: &mut Vec<String>) {
    if let Some(authority) = &connector.authority {
        if let Err(reason) = crate::address::validate_authority(authority) {
            problems.push(format!(
                "`authority` is not a valid reverse-DNS authority: {reason}. It is the leading \
                 component of every service address"
            ));
        }
    }
    if let Some(api_version) = &connector.api_version {
        if let Err(reason) = crate::address::validate_api_version(api_version) {
            problems.push(format!(
                "`api_version` cannot travel in an address: {reason}"
            ));
        }
    }

    let mut seen: Vec<&str> = Vec::new();

    for service in &connector.services {
        let name = service.name.as_str();
        if let Err(reason) = crate::address::validate_service_name(name) {
            problems.push(format!(
                "a `[[services]]` entry has an invalid `name`: {reason}"
            ));
            continue;
        }
        if service.legacy && name != DEFAULT_SERVICE {
            problems.push(format!(
                "service {name:?} sets `legacy = true`, but the marker belongs only to the reserved \
                 {DEFAULT_SERVICE:?} service whose already-published addresses must stay elided"
            ));
        }
        // The reserved name is normally the *implicit* service. C-458 also admits an explicitly
        // marked legacy default beside named siblings without changing its published addresses.
        // See `validate_default_service_entry`.
        if name == DEFAULT_SERVICE {
            validate_default_service_entry(connector, service, problems);
        }
        if seen.contains(&name) {
            problems.push(format!(
                "service {name:?} is declared more than once; an operation naming it could not say \
                 which declaration it meant"
            ));
        }
        seen.push(name);

        if let Some(base_url) = &service.base_url {
            if base_url.trim().is_empty() {
                problems.push(format!(
                    "service {name:?} declares an empty `base_url`; omit it to inherit the \
                     connector's"
                ));
            }
        }
        if let Some(api_version) = &service.api_version {
            if let Err(reason) = crate::address::validate_api_version(api_version) {
                problems.push(format!(
                    "service {name:?} declares an `api_version` that cannot travel in an address: \
                     {reason}. Omit it to inherit the connector's"
                ));
            }
        }

        validate_service_roles(connector, service, problems);
        validate_service_tags(service, problems);
        validate_service_audiences(service, problems);
    }
}

/// Checks that every tag a service declares is stated once — C-153.
///
/// There is no satisfaction check here and there deliberately cannot be one: a [`Tag`] carries no
/// required members, because no operation makes a service `storage`. The unknown-name case is not
/// here either — `serde` refuses it first at the parse, since [`Tag`] is a closed enum — so the only
/// thing left to refuse is a repeat.
fn validate_service_tags(service: &Service, problems: &mut Vec<String>) {
    let name = service.name.as_str();
    let mut seen: Vec<Tag> = Vec::new();

    for tag in &service.tags {
        let word = tag.word();
        if seen.contains(tag) {
            problems.push(format!(
                "service {name:?} declares tag {word:?} more than once. A tag is a label, and a set \
                 that tolerates repeats is a list pretending to be a set. Known tags: {}",
                Tag::known_set()
            ));
            continue;
        }
        seen.push(*tag);
    }
}

/// Checks that discovery audiences form a set on each service.
///
/// Unknown values are refused by serde because [`crate::Audience`] is closed. Repeats are refused
/// here so every projection can treat the field as a set without silently normalizing author input.
fn validate_service_audiences(service: &Service, problems: &mut Vec<String>) {
    let name = service.name.as_str();
    let mut seen: Vec<crate::Audience> = Vec::new();

    for audience in &service.audiences {
        let word = audience.word();
        if seen.contains(audience) {
            problems.push(format!(
                "service {name:?} declares audience {word:?} more than once. An audience is a \
                 discovery label, and a set that tolerates repeats is a list pretending to be a \
                 set. Known audiences: {}",
                crate::Audience::known_set()
            ));
            continue;
        }
        seen.push(*audience);
    }
}

/// Checks the one `[[services]]` entry that may name the reserved [`DEFAULT_SERVICE`] — C-120,
/// C-458.
///
/// C-49 refused the name outright, and the reason was sound: `default` is the service an operation
/// belongs to when it names none, so declaring it is a second definition of something that already
/// exists, and the two could disagree about a base URL or a version.
///
/// Roles and tags are the one thing that argument does not cover for **a provider with a single API
/// surface**, which has no other service to attach either to. C-458 adds a second, explicitly marked
/// shape for a published default service growing named siblings. The exceptions are scoped along
/// two axes:
///
/// 1. **What the entry may carry.** `roles`, `tags` and `audiences`, and nothing else. None has a
///    connector-level spelling, so neither has anything to contradict, while `base_url`,
///    `api_version` and `description` all do. `tags` joined the exception with C-153, which is what
///    makes the *forty-seven* single-surface providers taggable at all.
/// 2. **Whether the provider has any other service.** A `default` entry beside a named one remains
///    refused unless it sets `legacy = true`. That marker says the elided address already exists and
///    must not move. In that shape every service-bearing source table must state `service`, so the
///    declaration cannot silently catch omissions.
///
/// The reserved service stays address-elided in both admitted forms. A single-surface provider still
/// satisfies [`Connector::is_default_only`]; a mixed legacy provider does not, but artifact and
/// address rendering elide `default` by name and therefore keep its existing `<provider>.flux`.
fn validate_default_service_entry(
    connector: &Connector,
    service: &Service,
    problems: &mut Vec<String>,
) {
    // Scoped by "a service other than `default` is declared" rather than by a count, so that a file
    // declaring `default` twice reports the duplicate once and does not also report this twice.
    let named_sibling = connector
        .services
        .iter()
        .find(|other| other.name != DEFAULT_SERVICE);

    if let Some(other) = named_sibling.filter(|_| !service.legacy) {
        problems.push(format!(
            "`[[services]]` declares {DEFAULT_SERVICE:?} beside the named service {:?}. \
             Set `legacy = true` only when this is an already-published, address-elided service that \
             must retain its old GID, OIP, credential address and unsuffixed artifacts while named \
             siblings are added. Otherwise declare the roles, tags and audiences on the named service that \
             actually has them",
            other.name
        ));
        return;
    }

    if named_sibling.is_none() && service.legacy {
        problems.push(format!(
            "`[[services]]` declares {DEFAULT_SERVICE:?} with `legacy = true` but has no named \
             sibling. The marker is an address-migration capability for preserving an \
             already-published default while named services are added, not shorthand for a new or \
             default-only connector"
        ));
    }

    let mut overreaching: Vec<&str> = Vec::new();
    if !service.description.is_empty() {
        overreaching.push("description");
    }
    if service.base_url.is_some() {
        overreaching.push("base_url");
    }
    if service.api_version.is_some() {
        overreaching.push("api_version");
    }

    if !overreaching.is_empty() {
        problems.push(format!(
            "`[[services]]` declares {DEFAULT_SERVICE:?} with `{}`. {DEFAULT_SERVICE:?} is \
             reserved — it is the service an operation belongs to when it names none, and it is \
             elided from every published address — so the entry may carry `roles`, `tags` and \
             `audiences`, and nothing else. All three attach to a service and a single-surface \
             provider has nowhere else to put them; everything else is already stated at connector level, and a \
             second definition could disagree with it",
            overreaching.join("`, `")
        ));
    } else if named_sibling.is_none()
        && !service.legacy
        && service.roles.is_empty()
        && service.tags.is_empty()
        && service.audiences.is_empty()
    {
        problems.push(format!(
            "`[[services]]` declares {DEFAULT_SERVICE:?} and nothing else. {DEFAULT_SERVICE:?} is \
             reserved: it is the service an operation belongs to when it names none, and a provider \
             with one API surface declares no services at all. The reasons to write the entry \
             are to carry `roles`, `tags`, or discovery-only `audiences`"
        ));
    }
}

/// Refuses the ambiguity C-458's explicit legacy-default shape would otherwise reintroduce.
///
/// Serde intentionally normalizes an omitted `service` to `default` for every existing connector.
/// Only a mixed connector preserving an old default needs to distinguish those authoring forms, so
/// [`implicit_service_members`] retains that presence bit until this check and nowhere beyond it.
pub(super) fn validate_legacy_default_members(loaded: &LoadedProvider, problems: &mut Vec<String>) {
    let connector = &loaded.connector;
    let preserves_legacy_default = connector
        .services
        .iter()
        .any(|service| service.name == DEFAULT_SERVICE && service.legacy)
        && connector
            .services
            .iter()
            .any(|service| service.name != DEFAULT_SERVICE);

    if !preserves_legacy_default {
        return;
    }

    for member in &loaded.implicit_service_members {
        problems.push(format!(
            "{} {:?} names no `service` in a connector preserving legacy {DEFAULT_SERVICE:?} \
             beside named services. State `service = {DEFAULT_SERVICE:?}` for the published legacy \
             owner or name its sibling; omission remains refused so a new member cannot silently \
             enter the address-elided service",
            member.kind, member.name
        ));
    }
}

/// Checks that every role a service claims is one it satisfies, and claimed once — C-120.
///
/// A role is a *contract*, and the checking is the whole value of declaring one: a consumer reading
/// the catalogue relies on `llm_catalogue` without reading the provider's TOML, so an unsatisfied
/// claim would make the catalogue lie. The unknown-name case is not here because `serde` refuses it
/// first, at the parse — [`Role`] is a closed enum, and serde's error already quotes the name that
/// was written and lists the ones that exist.
fn validate_service_roles(connector: &Connector, service: &Service, problems: &mut Vec<String>) {
    let name = service.name.as_str();
    let mut seen: Vec<Role> = Vec::new();

    for role in &service.roles {
        let word = role.word();
        if seen.contains(role) {
            problems.push(format!(
                "service {name:?} declares role {word:?} more than once. A role is a claim; stating \
                 it twice states nothing the first one did not, and a set that tolerates repeats is \
                 a list pretending to be a set"
            ));
            continue;
        }
        seen.push(*role);

        for missing in connector.missing_role_members(name, *role) {
            problems.push(format!(
                "service {name:?} claims role {word:?} but has no {missing:?} operation. A role \
                 names what it requires by the member's name *within the service* — the trailing \
                 segments, so that `openai-models-list` and `openrouter-models-list` fill one slot \
                 and the shape is the same whatever the vendor calls its endpoint. It must be an \
                 `[[operations]]` entry: a role is a claim that something is callable, and an event \
                 or a channel binding is emitted into no module, so filling the slot with one would \
                 publish a capability nothing can call. {word:?} requires: {}",
                role.required_members().join(", ")
            ));
        }
    }
}

/// Checks that an operation's service is one this provider has.
///
/// The set is the declared names, or exactly `default` when nothing is declared — so a
/// single-surface provider needs no `[[services]]` block, and a multi-service provider has no
/// implicit `default` for an operation to fall into. That second half is the important one: an
/// operation that omitted `service` in a multi-service file would otherwise be emitted into an
/// `<provider>-default.flux` nobody declared or asked for.
pub(super) fn validate_operation_service(
    connector: &Connector,
    operation: &Operation,
    problems: &mut Vec<String>,
) {
    let available = connector.service_names();
    if available.contains(&operation.service.as_str()) {
        return;
    }
    let listed = available.join(", ");
    let id = operation.id.as_str();
    problems.push(if operation.service == DEFAULT_SERVICE {
        format!(
            "operation {id:?} names no `service`, which means the reserved {DEFAULT_SERVICE:?} \
             service — but this provider declares named services and no `[[services]]` entry \
             declares {DEFAULT_SERVICE:?}. Every operation of a multi-service provider names one of: \
             {listed}"
        )
    } else {
        format!(
            "operation {id:?} names service {:?}, which no `[[services]]` entry declares. This \
             provider declares: {listed}",
            operation.service
        )
    });
}
