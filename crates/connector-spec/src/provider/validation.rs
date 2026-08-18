use super::*;

/// Everything wrong with the file, in the order an author would read it: the connector itself, then
/// its credentials, then its operations, then the patch set.
///
/// Returning a `Vec` rather than short-circuiting is deliberate — see the module docs.
pub(super) fn validate(
    loaded: &LoadedProvider,
    provider_headers: &BTreeMap<String, String>,
    inline: &[String],
) -> Vec<String> {
    let mut problems = Vec::new();
    let connector = &loaded.connector;

    if connector.id.trim().is_empty() {
        problems.push("`id` must not be empty — it names the generated `<id>.flux`".to_owned());
    }
    if connector.base_url.trim().is_empty() {
        problems.push(
            "`base_url` must not be empty. It is stated explicitly even when a spec is present: \
             the babelforce document declares staging as `servers[0]`, so a positional default \
             would silently target the dev environment"
                .to_owned(),
        );
    }

    // The two messages below are pinned verbatim by `tests/golden/nothing-to-generate.error` and
    // `tests/golden/patch-without-spec.error`, and they are about the *absence* of any `[spec]` —
    // which C-410 did not change. A file with no spec block has none in either spelling.
    if loaded.specs.is_empty() && connector.operations.is_empty() {
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
