//! The typed table, built from the embedded pack rather than from generated Rust.
//!
//! The predecessor stored this as `src/generated/<provider>.rs` — 16,909 lines the build wrote,
//! committed and reviewed, holding exactly the facts the canonical documents already carry plus
//! one they do not: the emitted Flux text. With the emitter gone (design 02 §2) the tables carry
//! nothing the pack does not, so this module derives the same table from the same documents at
//! run time, once.
//!
//! # Why `&'static`, and what it costs
//!
//! The published API hands out `&'static Provider` and `&'static Operation`, because a resolved
//! operation is held by every projection of it and a catalogue is process-lifetime data. The table
//! is therefore built inside a [`OnceLock`] and **leaked**: an allocation that lives as long as
//! the process and is never freed, which is what the generated `.rodata` was. Nothing here is
//! dropped, so nothing here may own a resource other than memory.
//!
//! # Refusals are panics, and that is deliberate
//!
//! The pack is digest-verified by the reader before a byte of it is served, and it is written by
//! this workspace's own `connectors catalog build` from the documents committed beside it. A
//! document this module cannot map is therefore a corrupt build, not an input a caller can act on
//! — the same reasoning `catalog_reader::embedded` states for its own `expect`. Every refusal
//! below names the provider, the member and the value, so the failure is one line rather than an
//! index panic.
//!
//! # Two things the document could not say, and no longer cannot (S-001)
//!
//! - **[`Acquisition::Minted`]** — the minting join used to live only in the provider TOML's
//!   `[[operations]]` block and reached no document field. The document now carries it
//!   (`produces_credential`: which credential the value is stored as, and where in the response
//!   the secret arrives), and [`build`] constructs the variant from it. No shipped connector
//!   declares one yet; the fixture-backed test below proves the path.
//! - **[`CredentialRequirement`]** — the document used to publish only the *effective* auth list,
//!   so "declared `auth = []`" and "nothing declared anywhere" were the same empty list, and this
//!   module *derived* the difference from the connector default — exact for the shipped
//!   catalogue, ambiguous in principle. The document now publishes `credential_requirement`
//!   itself (C-206's tokens, computed at build where the declaration's `Option` still holds the
//!   distinction), and [`credential_requirement`] **reads** it. The pair of documents the old
//!   derivation could not tell apart is pinned below.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    Acquisition, Approval, AuthHazard, Channel, ChannelTransport, Choice, ConfigChoices,
    ConfigField, Credential, CredentialRequirement, Event, HostEffect, Idempotency,
    ImplementationForm, InteractionShape, OAuth2, OAuthGrant, OAuthRedirect, Operation,
    OperationDirection, Pair, Placement, PlacementRequirement, ProtocolDriver, Provider,
    RequiredCapability, Risk, Selector, Service, SocketConnect, Subject,
};

/// Every provider in the embedded pack, ordered by id, built once.
pub(crate) fn providers() -> &'static [&'static Provider] {
    static TABLE: OnceLock<&'static [&'static Provider]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let built: Vec<&'static Provider> = catalog_reader::providers()
            .map(|record| build(record.id(), record.document()))
            .collect();
        leak_slice(built)
    })
}

// ---------------------------------------------------------------------------------------------
// Leaking
// ---------------------------------------------------------------------------------------------

fn leak_str(value: impl Into<String>) -> &'static str {
    Box::leak(value.into().into_boxed_str())
}

fn leak_slice<T>(values: Vec<T>) -> &'static [T] {
    Box::leak(values.into_boxed_slice())
}

fn leak_opt(value: Option<String>) -> Option<&'static str> {
    value.map(leak_str)
}

fn leak_strs(values: Vec<String>) -> &'static [&'static str] {
    leak_slice(values.into_iter().map(leak_str).collect())
}

/// The OR-of-AND requirement shape, leaked.
fn leak_requirements(values: Vec<Vec<String>>) -> &'static [&'static [&'static str]] {
    leak_slice(values.into_iter().map(leak_strs).collect())
}

fn leak_pairs(values: BTreeMap<String, String>) -> &'static [Pair] {
    leak_slice(
        values
            .into_iter()
            .map(|(name, value)| Pair {
                name: leak_str(name),
                value: leak_str(value),
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------------------------
// The document, as JSON
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawDocument {
    connector: String,
    #[serde(default)]
    vendor: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    authority: Option<String>,
    #[serde(default)]
    verify: Option<String>,
    #[serde(default)]
    services: Vec<RawService>,
    #[serde(default)]
    auth: Vec<RawAuth>,
    #[serde(default)]
    config: Vec<RawConfig>,
    #[serde(default)]
    operations: Vec<RawOperation>,
    #[serde(default)]
    events: Vec<RawEvent>,
    #[serde(default)]
    channels: Vec<RawChannel>,
}

#[derive(Deserialize)]
struct RawService {
    name: String,
    base_url: String,
}

#[derive(Deserialize)]
struct RawAuth {
    name: String,
    scheme: RawScheme,
    #[serde(default)]
    user_env: Vec<String>,
    #[serde(default)]
    user_suffix: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    hazard: Option<String>,
    #[serde(default)]
    oauth2: Option<RawOAuth2>,
}

#[derive(Deserialize)]
struct RawScheme {
    kind: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
}

#[derive(Deserialize)]
struct RawOAuth2 {
    #[serde(default)]
    endpoint: String,
    #[serde(default)]
    token_endpoint: String,
    #[serde(default)]
    authorize_path: String,
    #[serde(default)]
    token_path: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    grants: Vec<String>,
    #[serde(default)]
    redirect: Option<RawRedirect>,
    #[serde(default)]
    public_client: bool,
}

#[derive(Deserialize)]
struct RawRedirect {
    port: u16,
    path: String,
}

#[derive(Deserialize)]
struct RawConfig {
    name: String,
    service: String,
    label: String,
    #[serde(default)]
    help: String,
    #[serde(default)]
    example: Option<String>,
    format: String,
    #[serde(default)]
    choices: Vec<RawChoice>,
    required: bool,
    #[serde(default)]
    default: Option<String>,
    approval: String,
    secret: bool,
    #[serde(default)]
    docs_url: Option<String>,
    binds: String,
    #[serde(default)]
    also_binds: Vec<String>,
    #[serde(default)]
    also_services: Vec<String>,
}

#[derive(Deserialize)]
struct RawChoice {
    value: String,
    label: String,
}

#[derive(Deserialize)]
struct RawOperation {
    id: String,
    service: String,
    direction: String,
    #[serde(default)]
    description: String,
    risk: String,
    idempotency: String,
    effects: Vec<String>,
    #[serde(default)]
    semantic_effects: Vec<String>,
    interaction_shape: String,
    protocol_driver: String,
    placement_requirement: String,
    implementation_form: String,
    required_capabilities: Vec<String>,
    #[serde(default)]
    auth: Vec<Vec<String>>,
    /// C-206's token for what the effective `auth` list cannot say when it is empty (S-001).
    /// Absent only in a pre-S-001 document, which this build never serves — refused by name.
    #[serde(default)]
    credential_requirement: String,
    /// The stored model-facing contract projection (S-001): the error-envelope-extended
    /// description and the lowered, caller-typed input schema, as the build computed them.
    #[serde(default)]
    contract: Option<RawContract>,
    #[serde(default)]
    expose: bool,
    /// The minting join (S-001), when this operation's call mints a declared credential.
    #[serde(default)]
    produces_credential: Option<RawProducedCredential>,
}

#[derive(Deserialize)]
struct RawContract {
    #[serde(default)]
    description: String,
    #[serde(default)]
    input_schema: Value,
}

#[derive(Deserialize)]
struct RawProducedCredential {
    credential: String,
    secret: String,
}

#[derive(Deserialize)]
struct RawEvent {
    name: String,
    service: String,
    #[serde(default)]
    wire_value: Option<String>,
    #[serde(default)]
    description: String,
    default: bool,
    #[serde(default)]
    group: String,
    #[serde(default)]
    schema: Option<Value>,
}

#[derive(Deserialize)]
struct RawChannel {
    name: String,
    service: String,
    #[serde(default)]
    description: String,
    transport: String,
    #[serde(default)]
    connect: Option<RawConnect>,
    #[serde(default)]
    events: Vec<String>,
    #[serde(default)]
    discriminator: Option<RawSelector>,
    #[serde(default)]
    delivery_id: Option<RawSelector>,
    #[serde(default)]
    payload: BTreeMap<String, String>,
    #[serde(default)]
    payload_root: bool,
}

#[derive(Deserialize)]
struct RawConnect {
    path: String,
    #[serde(default)]
    query: BTreeMap<String, String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    /// The IR's own `AuthRequirement` shape — `[{ "credentials": [...] }]` — not the flattened
    /// OR-of-AND the document uses at connector and operation level.
    #[serde(default)]
    auth: Vec<RawRequirement>,
    #[serde(default)]
    subprotocols: Vec<String>,
}

#[derive(Deserialize)]
struct RawRequirement {
    #[serde(default)]
    credentials: Vec<String>,
}

#[derive(Deserialize)]
struct RawSelector {
    source: String,
    name: String,
}

// ---------------------------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------------------------

/// One provider's table, from its canonical document text.
fn build(id: &str, text: &str) -> &'static Provider {
    let raw: RawDocument = serde_json::from_str(text).unwrap_or_else(|error| {
        panic!("the catalog pack's document for `{id}` does not parse: {error}")
    });
    let raw_value: Value = serde_json::from_str(text).expect("it parsed once already");

    let base_urls: BTreeMap<&str, &str> = raw
        .services
        .iter()
        .map(|service| (service.name.as_str(), service.base_url.as_str()))
        .collect();
    // The connector-level base URL: the reserved single-surface service when there is one,
    // otherwise the first declared service. A document always declares at least one service.
    let base_url = base_urls
        .get(Service::DEFAULT)
        .copied()
        .or_else(|| {
            raw.services
                .first()
                .map(|service| service.base_url.as_str())
        })
        .unwrap_or_else(|| panic!("`{id}`'s document declares no service, so it names no host"));

    let operations = raw
        .operations
        .iter()
        .map(|operation| build_operation(&raw, operation, &base_urls))
        .collect();

    // The minting joins (S-001): credential name → (minting operation, response pointer). Built
    // before the credentials so `acquisition` can answer the provenance axis from document data.
    let mut mints: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for operation in &raw.operations {
        let Some(produced) = &operation.produces_credential else {
            continue;
        };
        if !raw
            .auth
            .iter()
            .any(|method| method.name == produced.credential)
        {
            panic!(
                "operation `{}` mints `{}`, which its document does not declare",
                operation.id, produced.credential
            );
        }
        if let Some((earlier, _)) = mints.insert(
            produced.credential.as_str(),
            (operation.id.as_str(), produced.secret.as_str()),
        ) {
            panic!(
                "credential `{}` is minted by both `{earlier}` and `{}`",
                produced.credential, operation.id
            );
        }
    }

    let config_choices = raw.config.iter().filter_map(build_choices).collect();

    Box::leak(Box::new(Provider {
        id: leak_str(raw.connector.clone()),
        vendor: leak_str(raw.vendor.clone()),
        description: leak_str(raw.description.clone()),
        authority: leak_opt(raw.authority.clone()),
        services: leak_slice(
            raw.services
                .iter()
                .map(|service| Service {
                    name: leak_str(service.name.clone()),
                    base_url: leak_str(service.base_url.clone()),
                })
                .collect(),
        ),
        base_url: leak_str(base_url),
        auth: leak_slice(
            raw.auth
                .iter()
                .map(|method| build_credential(id, method, mints.get(method.name.as_str())))
                .collect(),
        ),
        operations: leak_slice(operations),
        config: leak_slice(
            raw.config
                .iter()
                .enumerate()
                .map(|(index, field)| build_config(id, field, member(&raw_value, "config", index)))
                .collect(),
        ),
        verify: leak_opt(raw.verify.clone()),
        events: leak_slice(
            raw.events
                .iter()
                .enumerate()
                .map(|(index, event)| build_event(event, member(&raw_value, "events", index)))
                .collect(),
        ),
        channels: leak_slice(
            raw.channels
                .iter()
                .enumerate()
                .map(|(index, channel)| {
                    build_channel(
                        id,
                        channel,
                        &base_urls,
                        base_url,
                        member(&raw_value, "channels", index),
                    )
                })
                .collect(),
        ),
        config_choices: leak_slice(config_choices),
    }))
}

/// The document's own JSON for one member of a top-level array, compact.
///
/// This is what `declaration_json` carries: the canonical form of the declaration, taken verbatim
/// from the reviewed artifact rather than re-serialized from a partial view of it.
fn member(document: &Value, key: &str, index: usize) -> String {
    document
        .get(key)
        .and_then(Value::as_array)
        .and_then(|members| members.get(index))
        .map(|member| serde_json::to_string(member).expect("a parsed value re-serializes"))
        .unwrap_or_else(|| "{}".to_owned())
}

fn build_operation(
    document: &RawDocument,
    raw: &RawOperation,
    base_urls: &BTreeMap<&str, &str>,
) -> Operation {
    let base_url = base_urls
        .get(raw.service.as_str())
        .copied()
        .unwrap_or_else(|| {
            panic!(
                "operation `{}` names service `{}`, which its document does not declare",
                raw.id, raw.service
            )
        });
    Operation {
        id: leak_str(raw.id.clone()),
        provider: leak_str(document.connector.clone()),
        service: leak_str(raw.service.clone()),
        direction: direction(&raw.id, &raw.direction),
        description: leak_str(raw.description.clone()),
        risk: risk(&raw.id, &raw.risk),
        idempotency: idempotency(&raw.id, &raw.idempotency),
        effects: leak_slice(
            raw.effects
                .iter()
                .map(|word| host_effect(&raw.id, word))
                .collect(),
        ),
        semantic_effects: leak_strs(raw.semantic_effects.clone()),
        interaction_shape: interaction_shape(&raw.id, &raw.interaction_shape),
        protocol_driver: protocol_driver(&raw.id, &raw.protocol_driver),
        placement_requirement: placement_requirement(&raw.id, &raw.placement_requirement),
        implementation_form: implementation_form(&raw.id, &raw.implementation_form),
        required_capabilities: leak_slice(
            raw.required_capabilities
                .iter()
                .map(|word| required_capability(&raw.id, word))
                .collect(),
        ),
        credentials: leak_requirements(raw.auth.clone()),
        credential_requirement: credential_requirement(raw),
        // Per operation, through its service: a multi-service provider reaches a different host per
        // service, and the union would be a wider egress claim than any single call makes.
        hosts: leak_slice(vec![leak_str(host_of(&raw.id, base_url))]),
        contract_description: leak_str(
            raw.contract
                .as_ref()
                .map(|contract| contract.description.clone())
                .unwrap_or_else(|| {
                    panic!(
                        "operation `{}` carries no contract, which S-001 made required",
                        raw.id
                    )
                }),
        ),
        input_schema: leak_str(
            raw.contract
                .as_ref()
                .map(|contract| contract.input_schema.to_string())
                .expect("the contract was just proven present"),
        ),
        expose: raw.expose,
    }
}

/// Why an operation's credential list is what it is — **read** from the document (S-001), never
/// derived. The build computes the token where the declaration's `Option` still distinguishes
/// "declared `auth = []`" from "declared nothing anywhere"; resolving it here from the connector
/// default — what this function did before S-001 — could not tell those two apart.
fn credential_requirement(raw: &RawOperation) -> CredentialRequirement {
    match raw.credential_requirement.as_str() {
        "declared" => CredentialRequirement::Declared,
        "no-credential-required" => CredentialRequirement::NoneRequired,
        "no-credential" => CredentialRequirement::Withheld,
        "" => panic!(
            "operation `{}` carries no credential requirement, which S-001 made required",
            raw.id
        ),
        other => panic!(
            "operation `{}` carries unknown credential requirement `{other}`",
            raw.id
        ),
    }
}

/// The host a call reaches, with the base URL's templating intact.
///
/// `https://{subdomain}.zendesk.com` yields `{subdomain}.zendesk.com`: the tenant is the operator's
/// to choose, and substituting a placeholder here would invent one.
fn host_of(operation: &str, base_url: &str) -> String {
    let after_scheme = base_url
        .split_once("://")
        .map_or(base_url, |(_scheme, rest)| rest);
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    if host.is_empty() {
        panic!("operation `{operation}`'s base URL `{base_url}` names no host");
    }
    host.to_owned()
}

fn build_credential(provider: &str, raw: &RawAuth, mint: Option<&(&str, &str)>) -> Credential {
    // The leaf of the credential's address. Credentials share one flat `<connector>.<name>`
    // namespace, so a name whose prefix disagrees would render a path under the wrong vendor.
    let leaf = raw
        .name
        .strip_prefix(&format!("{provider}."))
        .unwrap_or_else(|| panic!("credential `{}` is not prefixed `{provider}.`", raw.name));
    Credential {
        name: leak_str(raw.name.clone()),
        leaf: leak_str(leaf),
        acquire: acquisition(&raw.name, raw, mint),
        place: placement(&raw.name, &raw.scheme),
        subject: subject(&raw.name, raw.subject.as_deref()),
        hazard: hazard(&raw.name, raw.hazard.as_deref()),
    }
}

fn acquisition(name: &str, raw: &RawAuth, mint: Option<&(&str, &str)>) -> Acquisition {
    if let Some(oauth2) = &raw.oauth2 {
        if mint.is_some() {
            panic!(
                "credential `{name}` declares OAuth2 and is minted by an operation — two \
                 provenances for one value"
            );
        }
        return Acquisition::OAuth2(Box::leak(Box::new(OAuth2 {
            endpoint: leak_str(oauth2.endpoint.clone()),
            token_endpoint: leak_str(oauth2.token_endpoint.clone()),
            authorize_path: leak_str(oauth2.authorize_path.clone()),
            token_path: leak_str(oauth2.token_path.clone()),
            scopes: leak_strs(oauth2.scopes.clone()),
            grants: leak_slice(
                oauth2
                    .grants
                    .iter()
                    .map(|grant| oauth_grant(name, grant))
                    .collect(),
            ),
            redirect: oauth2.redirect.as_ref().map(|redirect| OAuthRedirect {
                port: redirect.port,
                path: leak_str(redirect.path.clone()),
            }),
            public_client: oauth2.public_client,
        })));
    }
    if raw.scheme.kind == "basic" {
        return Acquisition::BasicJoin {
            user_env: leak_strs(raw.user_env.clone()),
            user_suffix: leak_str(raw.user_suffix.clone().unwrap_or_default()),
        };
    }
    // The minting join, read from the document (S-001): placement-wise this is `Static` — the
    // stored value goes out unchanged — plus the provenance fact only the document carries.
    if let Some((by, from)) = mint {
        return Acquisition::Minted {
            by: leak_str((*by).to_string()),
            from: leak_str((*from).to_string()),
        };
    }
    Acquisition::Static
}

fn oauth_grant(credential: &str, word: &str) -> OAuthGrant {
    match word {
        "authorization_code" => OAuthGrant::AuthorizationCode,
        "password" => OAuthGrant::Password,
        "refresh_token" => OAuthGrant::RefreshToken,
        "client_credentials" => OAuthGrant::ClientCredentials,
        other => panic!("credential `{credential}` declares unknown OAuth2 grant `{other}`"),
    }
}

/// The placement axis, from the document's flattened `{kind, name, prefix}` scheme.
///
/// Exhaustive with no catch-all: a scheme this did not recognise would have to become *some*
/// placement, and every wrong answer either sends a credential where the vendor does not read it or
/// sends it somewhere it should never go at all.
fn placement(credential: &str, scheme: &RawScheme) -> Placement {
    match scheme.kind.as_str() {
        "bearer" => Placement::Header {
            name: "Authorization",
            prefix: "Bearer ",
        },
        "basic" => Placement::Header {
            name: "Authorization",
            prefix: "Basic ",
        },
        "header" => Placement::Header {
            name: leak_str(required(
                credential,
                "header",
                "name",
                scheme.name.as_deref(),
            )),
            prefix: leak_str(scheme.prefix.clone().unwrap_or_default()),
        },
        "query" => Placement::Query {
            name: leak_str(required(
                credential,
                "query",
                "name",
                scheme.name.as_deref(),
            )),
        },
        "signing" => Placement::Inbound,
        other => panic!("credential `{credential}` declares unknown auth scheme `{other}`"),
    }
}

fn required<'a>(credential: &str, kind: &str, field: &str, value: Option<&'a str>) -> &'a str {
    value.unwrap_or_else(|| {
        panic!("credential `{credential}`'s `{kind}` scheme declares no `{field}`")
    })
}

fn subject(credential: &str, word: Option<&str>) -> Subject {
    match word {
        None | Some("unstated") => Subject::Unstated,
        Some("app") => Subject::App,
        Some("user") => Subject::User,
        Some(other) => panic!("credential `{credential}` declares unknown subject `{other}`"),
    }
}

fn hazard(credential: &str, word: Option<&str>) -> Option<AuthHazard> {
    match word {
        None => None,
        Some("resource_owner_secret_shared") => Some(AuthHazard::ResourceOwnerSecretShared),
        Some(other) => panic!("credential `{credential}` declares unknown hazard `{other}`"),
    }
}

fn build_config(provider: &str, raw: &RawConfig, declaration: String) -> ConfigField {
    ConfigField {
        name: leak_str(raw.name.clone()),
        service: leak_str(raw.service.clone()),
        label: leak_str(raw.label.clone()),
        help: leak_str(raw.help.clone()),
        example: leak_opt(raw.example.clone()),
        format: leak_str(raw.format.clone()),
        required: raw.required,
        default: leak_opt(raw.default.clone()),
        approval: approval(provider, &raw.name, &raw.approval),
        secret: raw.secret,
        docs_url: leak_opt(raw.docs_url.clone()),
        binds: leak_str(raw.binds.clone()),
        also_binds: leak_strs(raw.also_binds.clone()),
        also_services: leak_strs(raw.also_services.clone()),
        declaration_json: leak_str(declaration),
    }
}

fn approval(provider: &str, field: &str, word: &str) -> Approval {
    match word {
        "none" => Approval::None,
        "operator" => Approval::Operator,
        other => {
            panic!("`{provider}`'s config field `{field}` declares unknown approval `{other}`")
        }
    }
}

/// The closed set one configuration field permits, keyed as a host addresses a stored value.
///
/// `None` for an open field, which is most of them. The `(kind, name)` pair is [`binding`]'s
/// reading of the field's `binds` grammar.
fn build_choices(raw: &RawConfig) -> Option<ConfigChoices> {
    if raw.choices.is_empty() {
        return None;
    }
    let (kind, target) = binding(&raw.binds)?;
    Some(ConfigChoices {
        service: leak_str(raw.service.clone()),
        field: leak_str(raw.name.clone()),
        label: leak_str(raw.label.clone()),
        kind: leak_str(kind),
        name: leak_str(target),
        choices: leak_slice(
            raw.choices
                .iter()
                .map(|choice| Choice {
                    value: leak_str(choice.value.clone()),
                    label: leak_str(choice.label.clone()),
                })
                .collect(),
        ),
    })
}

/// `(kind, name)` for a configuration field's `binds` grammar — the address a host stores the
/// value under. `None` for a binding this grammar does not admit, which a loaded connector cannot
/// carry.
fn binding(binds: &str) -> Option<(String, String)> {
    for kind in [
        "endpoint",
        "credential",
        "username",
        "path",
        "query",
        "header",
    ] {
        if let Some(name) = binds.strip_prefix(&format!("{kind}.")) {
            if !name.is_empty() && !(kind == "credential" && name.is_empty()) {
                return Some((kind.to_owned(), name.to_owned()));
            }
        }
    }
    if let Some(rest) = binds.strip_prefix("channel.") {
        let (channel, parameter) = rest.split_once(".query.")?;
        if !channel.is_empty() && !parameter.is_empty() {
            return Some(("channel_query".to_owned(), parameter.to_owned()));
        }
        return None;
    }
    match binds {
        "oauth.client_id" => Some(("oauth".to_owned(), "client_id".to_owned())),
        "oauth.client_secret" => Some(("oauth".to_owned(), "client_secret".to_owned())),
        "oauth.redirect_uri" => Some(("oauth".to_owned(), "redirect_uri".to_owned())),
        _ => None,
    }
}

fn build_event(raw: &RawEvent, declaration: String) -> Event {
    Event {
        name: leak_str(raw.name.clone()),
        service: leak_str(raw.service.clone()),
        description: leak_str(raw.description.clone()),
        wire_value: leak_opt(raw.wire_value.clone()),
        default: raw.default,
        group: leak_str(raw.group.clone()),
        schema: raw.schema.as_ref().map(|schema| {
            leak_str(serde_json::to_string(schema).expect("a parsed schema re-serializes"))
        }),
        declaration_json: leak_str(declaration),
    }
}

fn build_channel(
    provider: &str,
    raw: &RawChannel,
    base_urls: &BTreeMap<&str, &str>,
    connector_base_url: &str,
    declaration: String,
) -> Channel {
    let base_url = base_urls
        .get(raw.service.as_str())
        .copied()
        .unwrap_or(connector_base_url);
    Channel {
        name: leak_str(raw.name.clone()),
        service: leak_str(raw.service.clone()),
        base_url: leak_str(base_url),
        description: leak_str(raw.description.clone()),
        transport: transport(provider, &raw.name, &raw.transport),
        events: leak_strs(raw.events.clone()),
        connect: raw.connect.as_ref().map(|connect| SocketConnect {
            path: leak_str(connect.path.clone()),
            query: leak_pairs(connect.query.clone()),
            headers: leak_pairs(connect.headers.clone()),
            auth: leak_requirements(
                connect
                    .auth
                    .iter()
                    .map(|requirement| requirement.credentials.clone())
                    .collect(),
            ),
            subprotocols: leak_strs(connect.subprotocols.clone()),
        }),
        discriminator: raw.discriminator.as_ref().map(|s| selector(&raw.name, s)),
        delivery_id: raw.delivery_id.as_ref().map(|s| selector(&raw.name, s)),
        payload: leak_pairs(raw.payload.clone()),
        payload_root: raw.payload_root,
        declaration_json: leak_str(declaration),
    }
}

fn transport(provider: &str, channel: &str, word: &str) -> ChannelTransport {
    match word {
        "webhook" => ChannelTransport::Webhook,
        "socket" => ChannelTransport::Socket,
        "poll" => ChannelTransport::Poll,
        other => {
            panic!("`{provider}`'s channel `{channel}` declares unknown transport `{other}`")
        }
    }
}

fn selector(channel: &str, raw: &RawSelector) -> Selector {
    match raw.source.as_str() {
        "header" | "body" => Selector {
            source: leak_str(raw.source.clone()),
            name: leak_str(raw.name.clone()),
        },
        other => panic!("channel `{channel}` declares unknown selector source `{other}`"),
    }
}

fn direction(operation: &str, word: &str) -> OperationDirection {
    match word {
        "read" => OperationDirection::Read,
        "write" => OperationDirection::Write,
        other => panic!("operation `{operation}` declares unknown direction `{other}`"),
    }
}

fn host_effect(operation: &str, word: &str) -> HostEffect {
    match word {
        "read" => HostEffect::Read,
        "write" => HostEffect::Write,
        "network" => HostEffect::Network,
        "process" => HostEffect::Process,
        "browser" => HostEffect::Browser,
        "filesystem" => HostEffect::Filesystem,
        "local_system" => HostEffect::LocalSystem,
        other => panic!("operation `{operation}` declares unknown host effect `{other}`"),
    }
}

fn interaction_shape(operation: &str, word: &str) -> InteractionShape {
    match word {
        "unary" => InteractionShape::Unary,
        "stream" => InteractionShape::Stream,
        "subscription" => InteractionShape::Subscription,
        "leased_session" => InteractionShape::LeasedSession,
        "session_establishment" => InteractionShape::SessionEstablishment,
        other => panic!("operation `{operation}` declares unknown interaction shape `{other}`"),
    }
}

fn protocol_driver(operation: &str, word: &str) -> ProtocolDriver {
    match word {
        "http_v1" => ProtocolDriver::HttpV1,
        "sip_v1" => ProtocolDriver::SipV1,
        other => panic!("operation `{operation}` declares unknown protocol driver `{other}`"),
    }
}

fn placement_requirement(operation: &str, word: &str) -> PlacementRequirement {
    match word {
        "connectors_deployment" => PlacementRequirement::ConnectorsDeployment,
        "substrate_workload" => PlacementRequirement::SubstrateWorkload,
        "federated_satellite" => PlacementRequirement::FederatedSatellite,
        other => panic!("operation `{operation}` declares unknown placement requirement `{other}`"),
    }
}

fn implementation_form(operation: &str, word: &str) -> ImplementationForm {
    match word {
        "built_in" => ImplementationForm::BuiltIn,
        other => panic!("operation `{operation}` declares unknown implementation form `{other}`"),
    }
}

fn required_capability(operation: &str, word: &str) -> RequiredCapability {
    match word {
        "public_network" => RequiredCapability::PublicNetwork,
        "private_network" => RequiredCapability::PrivateNetwork,
        "unix_socket" => RequiredCapability::UnixSocket,
        "file_secret" => RequiredCapability::FileSecret,
        "process" => RequiredCapability::Process,
        "container" => RequiredCapability::Container,
        "device" => RequiredCapability::Device,
        other => panic!("operation `{operation}` declares unknown required capability `{other}`"),
    }
}

fn risk(operation: &str, word: &str) -> Risk {
    match word {
        "low" => Risk::Low,
        "medium" => Risk::Medium,
        "high" => Risk::High,
        "destructive" => Risk::Destructive,
        other => panic!("operation `{operation}` declares unknown risk `{other}`"),
    }
}

fn idempotency(operation: &str, word: &str) -> Idempotency {
    match word {
        "idempotent" => Idempotency::Idempotent,
        "non_idempotent" => Idempotency::NonIdempotent,
        "conditional" => Idempotency::Conditional,
        other => panic!("operation `{operation}` declares unknown idempotency `{other}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal parseable document with one operation, parameterized on the auth-axis fields.
    fn document(operation_extra: &str, auth: &str) -> String {
        format!(
            r#"{{
                "connector": "t",
                "services": [{{ "name": "default", "base_url": "https://api.example.com" }}],
                "auth": {auth},
                "operations": [{{
                    "id": "t-thing-get",
                    "service": "default",
                    "direction": "read",
                    "risk": "low",
                    "idempotency": "idempotent",
                    "effects": ["read", "network"],
                    "semantic_effects": [],
                    "interaction_shape": "unary",
                    "protocol_driver": "http_v1",
                    "placement_requirement": "connectors_deployment",
                    "implementation_form": "built_in",
                    "required_capabilities": ["public_network"],
                    "contract": {{
                        "description": "Get a thing.",
                        "input_schema": {{ "type": "object", "properties": {{}}, "required": [] }}
                    }},
                    "expose": true,
                    "auth": [],
                    "request": {{ "method": "GET", "url": "{{base}}/thing" }}
                    {operation_extra}
                }}]
            }}"#
        )
    }

    /// **The pair the old derivation could not tell apart** (S-001 acceptance): two documents,
    /// byte-identical on the effective `auth` list (empty) and the connector default (absent),
    /// differing only in the stored `credential_requirement`. Under the pre-S-001 derivation —
    /// empty `auth` + empty `default_auth` → `Withheld` — both classified `Withheld`, so the
    /// connector that positively declared `auth = []` was reported as withholding a credential it
    /// never needed. Reading the stored token tells them apart.
    #[test]
    fn the_document_tells_apart_the_pair_the_derivation_could_not() {
        let declared_none = build(
            "t",
            &document(
                r#", "credential_requirement": "no-credential-required""#,
                "[]",
            ),
        );
        let withheld = build(
            "t",
            &document(r#", "credential_requirement": "no-credential""#, "[]"),
        );
        assert_eq!(
            declared_none.operations[0].credential_requirement,
            CredentialRequirement::NoneRequired
        );
        assert_eq!(
            withheld.operations[0].credential_requirement,
            CredentialRequirement::Withheld
        );
    }

    #[test]
    fn sip_v1_survives_the_generated_table() {
        let document = document(
            r#", "credential_requirement": "no-credential-required""#,
            "[]",
        )
        .replace(
            "\"interaction_shape\": \"unary\"",
            "\"interaction_shape\": \"session_establishment\"",
        )
        .replace(
            "\"protocol_driver\": \"http_v1\"",
            "\"protocol_driver\": \"sip_v1\"",
        );
        let provider = build("t", &document);
        assert_eq!(
            provider.operations[0].interaction_shape,
            InteractionShape::SessionEstablishment
        );
        assert_eq!(
            provider.operations[0].protocol_driver,
            ProtocolDriver::SipV1
        );
    }

    /// **The minting join reaches [`Acquisition::Minted`] from document data alone** (S-001): the
    /// document states which call mints the credential and where in the response the secret
    /// arrives, and the table constructs the variant no shipped connector reaches yet.
    #[test]
    fn a_minting_join_in_the_document_reaches_acquisition_minted() {
        let provider = build(
            "t",
            &document(
                r#", "credential_requirement": "no-credential-required",
                    "produces_credential": {{ "credential": "t.token", "secret": "/access_token" }}"#
                    .replace("{{", "{")
                    .replace("}}", "}")
                    .as_str(),
                r#"[{ "name": "t.token", "scheme": { "kind": "bearer" } }]"#,
            ),
        );
        let credential = provider.credential("t.token").expect("t.token is declared");
        assert_eq!(
            credential.acquire,
            Acquisition::Minted {
                by: "t-thing-get",
                from: "/access_token"
            }
        );
    }
}
