use super::*;

/// Everything wrong with the file, in the order an author would read it: the connector itself, then
/// its credentials, then its operations, then the patch set.
///
/// Returning a `Vec` rather than short-circuiting is deliberate — see the module docs.
pub(super) fn validate(
    loaded: &LoadedProvider,
    source: &str,
    provider_headers: &BTreeMap<String, String>,
    inline: &[String],
) -> Vec<String> {
    let mut problems = Vec::new();
    let connector = &loaded.connector;

    if connector.id.trim().is_empty() {
        problems.push("`id` must not be empty — it names the generated `<id>.flux`".to_owned());
    }
    if !connector.custody_only && connector.base_url.trim().is_empty() {
        problems.push(
            "`base_url` must not be empty. It is stated explicitly even when a spec is present: \
             the babelforce document declares staging as `servers[0]`, so a positional default \
             would silently target the dev environment"
                .to_owned(),
        );
    }
    validate_custody_only(loaded, source, &mut problems);

    // The two messages below are pinned verbatim by `tests/golden/nothing-to-generate.error` and
    // `tests/golden/patch-without-spec.error`, and they are about the *absence* of any `[spec]` —
    // which C-410 did not change. A file with no spec block has none in either spelling.
    if !connector.custody_only && loaded.specs.is_empty() && connector.operations.is_empty() {
        problems.push(
            "declares neither `[spec]` nor any `[[operations]]`, so it describes no operations at \
             all. Write the operations inline for a hand-authored connector, or point `[spec]` at \
             a vendored spec and select operations with `[[patch.operations]]`"
                .to_owned(),
        );
    }
    if loaded.specs.is_empty() && !loaded.patch.is_empty() {
        // The key is the one the file actually wrote: a message about `[[patch.operations]]` sends
        // an author who only declared a selector looking for a block they never authored. The
        // `[[patch.operations]]` rendering is the golden's, byte for byte.
        problems.push(format!(
            "declares `{}` but no `[spec]`; there is nothing for the patches to apply to",
            loaded.patch.declared()
        ));
    }
    validate_specs(loaded, &mut problems);

    validate_services(connector, &mut problems);
    validate_legacy_default_members(loaded, &mut problems);
    validate_credentials(connector, &mut problems);
    validate_const_headers(connector, provider_headers, &mut problems);
    validate_operations(connector, &mut problems);
    validate_events(connector, &mut problems);
    validate_channels(connector, &mut problems);
    validate_discoveries(connector, &mut problems);
    validate_config(connector, &mut problems);
    validate_verify(connector, &mut problems);
    validate_graphs(connector, &mut problems);
    validate_member_namespace(connector, &mut problems);
    validate_patch(loaded, inline, &mut problems);

    problems
}

/// **A custody-only provider declares a credential and nothing else.**
///
/// Every entry is a refusal rather than a silent allowance. The kind exists so that a credential
/// whose *use* belongs to another component can still have an owner, an address and a lifecycle
/// here; the moment it could also describe a request, "connectors cannot spend this" would be a
/// claim in a comment instead of a property of the declaration.
///
/// `base_url` is refused rather than ignored for the same reason the others are: an author who
/// wrote one believed this provider would call something, and silently dropping it would leave
/// that belief in the file for the next reader to act on.
fn validate_custody_only(loaded: &LoadedProvider, source: &str, problems: &mut Vec<String>) {
    let connector = &loaded.connector;
    if !connector.custody_only {
        return;
    }
    // Asked of the **declared keys**, not of the assembled values. `#[serde(default)]` makes
    // `base_url = ""` and `operations = []` indistinguishable from absent once parsed, and this
    // kind's whole security value is that an author who wrote a request surface is refused rather
    // than silently emptied. `implicit_service_members` reads the source the same way and for the
    // same reason.
    let table: toml::Table = source
        .parse()
        .expect("ProviderFile already parsed this source as TOML");
    for (key, spelling, why) in CUSTODY_ONLY_REFUSED_KEYS {
        if table.contains_key(*key) {
            problems.push(format!(
                "`custody_only` provider declares {spelling}, but {why}. A custody-only provider \
                 holds a credential another component spends; remove {spelling}, or remove \
                 `custody_only` and declare the surface properly"
            ));
        }
    }
    for method in &connector.auth {
        if method.oauth2.is_some() {
            problems.push(format!(
                "`custody_only` provider's credential `{}` declares `oauth2`, but that says the \
                 host runs the token grants — which is a request. A custody-only credential \
                 arrives already minted, so declare `entry` and no `oauth2`",
                method.name
            ));
        }
    }
    if connector.auth.is_empty() {
        problems.push(
            "`custody_only` provider declares no `[[auth]]`, so it holds nothing and has no \
             reason to exist"
                .to_owned(),
        );
    }
}

/// Every top-level key a `custody_only` provider is refused for declaring, with the TOML spelling
/// the author wrote and the reason it describes a request.
///
/// The list is the security property. `[[channels]]` in particular is not decoration: a channel
/// binding carries its own `auth`, and `connector-resolve`'s channel composition places those
/// resolved credentials onto the composed URL and headers — so a custody-only provider that could
/// declare one could spend the very credential this kind exists to make unspendable.
///
/// `[[config]]` and `api_version` are deliberately **absent** from the list. A configuration field
/// is how the connect UI labels and binds the credential, which a custody-only provider needs more
/// than an ordinary one does, and `api_version` is metadata that reaches no request.
const CUSTODY_ONLY_REFUSED_KEYS: &[(&str, &str, &str)] = &[
    ("spec", "`[spec]`", "a spec describes a request surface"),
    (
        "operations",
        "`[[operations]]`",
        "an operation is a request this provider could make",
    ),
    (
        "services",
        "`[[services]]`",
        "a service exists to carry operations",
    ),
    (
        "base_url",
        "`base_url`",
        "there is no request to build a URL for",
    ),
    ("verify", "`verify`", "a verification probe is a request"),
    (
        "channels",
        "`[[channels]]`",
        "a channel binding is an authenticated outbound handshake, and its `auth` is resolved onto \
         the composed URL and headers",
    ),
    (
        "events",
        "`[[events]]`",
        "an event source is a subscription this provider would open",
    ),
    (
        "discoveries",
        "`[[discoveries]]`",
        "a discovery names an operation to run",
    ),
    ("graphs", "`[[graphs]]`", "a graph is built from operations"),
    (
        "patch",
        "`[patch]`",
        "a patch selects operations out of a spec",
    ),
    (
        "const_headers",
        "`const_headers`",
        "a constant header exists to ride a request",
    ),
    (
        "default_auth",
        "`default_auth`",
        "it states which credentials an operation needs, and there are no operations",
    ),
];

/// Checks the `[spec]` / `[[spec]]` declarations themselves — C-410.
///
/// Everything here is about the *set* of documents rather than about any one of them, which is why
/// it cannot live in [`ingest_specs`]: those checks must hold whether or not the cache was supplied,
/// so `load` refuses a contradictory declaration exactly as `load_with_spec` does.
///
/// # Why a document's service must be declared, and is not declared *by* the document
///
/// A `[[spec]]` entry **joins** a service; it does not create one. A service carries a description,
/// possibly its own base URL and API version, and the roles it claims — none of which an OpenAPI
/// document supplies — and it names the emitted `<provider>-<service>.flux`. Letting a `service` key
/// conjure one would make a typo a silently-emitted extra module rather than a refusal, which is the
/// rule [`validate_member_service`] already keeps for every other member kind.
fn validate_specs(loaded: &LoadedProvider, problems: &mut Vec<String>) {
    let many = loaded.specs.len() > 1;
    let available = loaded.connector.service_names();
    let mut seen_paths: Vec<&str> = Vec::new();
    let mut seen_services: Vec<(SpecKind, &str)> = Vec::new();

    for spec in &loaded.specs {
        let path = spec.path.trim();
        if path.is_empty() {
            problems.push(format!(
                "`{} path` must not be empty — it points at the vendored spec under `specs/`",
                block(many)
            ));
        } else if seen_paths.contains(&path) {
            problems.push(format!(
                "`{}` names {path:?} more than once. One document is one service, so compiling it \
                 twice would put one vendor's operations in two places with no way to say which a \
                 caller meant",
                block(many)
            ));
        } else {
            seen_paths.push(path);
        }

        if spec
            .service
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            problems.push(format!(
                "`{} service` is empty for {path:?}; omit the key to join the reserved \
                 {DEFAULT_SERVICE:?} service, or name one a `[[services]]` entry declares",
                block(many)
            ));
            continue;
        }

        // Asked of the resolved name, so two entries that both omit the key are caught: they both
        // join `default`, which is one namespace and cannot hold two documents.
        let service = spec.service();
        if !available.contains(&service) {
            problems.push(if service == DEFAULT_SERVICE {
                format!(
                    "`{}` for {path:?} names no `service`, which means the reserved \
                     {DEFAULT_SERVICE:?} service — but this provider declares named services and no \
                     `[[services]]` entry declares {DEFAULT_SERVICE:?}. Each document of a \
                     multi-service provider names one of: {}",
                    block(many),
                    available.join(", ")
                )
            } else {
                format!(
                    "`{} service = {service:?}` for {path:?} names a service no `[[services]]` \
                     entry declares. A document joins a service, it does not declare one — a \
                     service carries a description, a base URL, an API version and its roles, none \
                     of which an OpenAPI document supplies. This provider declares: {}",
                    block(many),
                    available.join(", ")
                )
            });
        }
        if seen_services.contains(&(spec.kind, service)) {
            problems.push(format!(
                "`{}` gives service {service:?} two documents of kind {}. One source grammar may have \
                 only one document per service, or component/operation identity becomes \
                 ambiguous — give each document its own service",
                block(many),
                spec.kind.label()
            ));
        } else {
            seen_services.push((spec.kind, service));
        }
    }
}
