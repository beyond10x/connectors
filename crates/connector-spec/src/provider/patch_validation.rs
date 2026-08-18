use super::*;

/// Checks the patch set the overlay (C-6) will consume.
pub(super) fn validate_patch(
    loaded: &LoadedProvider,
    inline: &[String],
    problems: &mut Vec<String>,
) {
    // **Keyed by service as well as by `operationId`** — C-410. An `operationId` is unique inside
    // one document and nowhere else: babelforce declares `getUser` in `manager-2026-07-10` and again
    // in `user-2026-06-25`, as two different requests, so selecting both is the ordinary case and
    // only a repeat *within* one document is the duplicate this refuses.
    let mut selected: Vec<(&str, &str)> = Vec::new();
    let mut renamed: Vec<&str> = Vec::new();
    let openapi_specs: Vec<&SpecSource> = loaded
        .specs
        .iter()
        .filter(|spec| spec.kind == SpecKind::Openapi)
        .collect();

    for patch in &loaded.patch.operations {
        let select = patch.select.as_str();
        let service = patch.service.as_deref().map(str::trim).unwrap_or_else(|| {
            openapi_specs
                .first()
                .filter(|_| openapi_specs.len() == 1)
                .map_or(DEFAULT_SERVICE, |source| source.service())
        });
        if select.trim().is_empty() {
            problems.push(
                "a `[[patch.operations]]` entry has an empty `select`; it names the spec's \
                 `operationId`"
                    .to_owned(),
            );
        } else if selected.contains(&(service, select)) {
            problems.push(format!(
                "`[[patch.operations]]` selects {select:?} more than once from service {service:?}"
            ));
        }
        selected.push((service, select));

        if let Some(rename) = &patch.rename {
            if rename.trim().is_empty() {
                problems.push(format!("patch for {select:?} has an empty `rename`"));
            } else if renamed.contains(&rename.as_str()) {
                problems.push(format!(
                    "`[[patch.operations]]` renames two operations to {rename:?}; the op id is a \
                     public name and must be unique"
                ));
            // Asked of the **inline** ids, not of the connector's — after selection every rename is
            // among the connector's operations by construction, which would make this fire on every
            // successful patch (C-4).
            } else if inline.iter().any(|id| id == rename) {
                problems.push(format!(
                    "patch for {select:?} renames to {rename:?}, which an inline `[[operations]]` \
                     block already declares"
                ));
            }
            renamed.push(rename);
        }

        if let Some(alternatives) = &patch.auth {
            validate_requirements(
                &loaded.connector,
                alternatives,
                &format!("patch for {select:?}"),
                problems,
            );
        }

        for param in &patch.params {
            if param.name.trim().is_empty() {
                problems.push(format!(
                    "patch for {select:?} has a parameter correction with an empty `name`"
                ));
            }
        }

        for (position, name) in patch.omit.entries() {
            if name.trim().is_empty() {
                problems.push(format!(
                    "patch for {select:?} omits a `{position:?}` parameter with an empty `name`"
                ));
            }
        }
    }

    let mut selected_events: Vec<(&str, &str)> = Vec::new();
    let mut renamed_events: Vec<&str> = Vec::new();
    let asyncapi_specs: Vec<&SpecSource> = loaded
        .specs
        .iter()
        .filter(|spec| spec.kind == SpecKind::Asyncapi)
        .collect();
    for patch in &loaded.patch.events {
        let service = patch.service.as_deref().map(str::trim).unwrap_or_else(|| {
            asyncapi_specs
                .first()
                .filter(|_| asyncapi_specs.len() == 1)
                .map_or(DEFAULT_SERVICE, |source| source.service())
        });
        if patch.select.trim().is_empty() {
            problems.push(
                "a `[[patch.events]]` entry has an empty `select`; it names an AsyncAPI component \
                 message"
                    .to_owned(),
            );
        } else if selected_events.contains(&(service, patch.select.as_str())) {
            problems.push(format!(
                "`[[patch.events]]` selects {:?} more than once from service {service:?}",
                patch.select
            ));
        }
        selected_events.push((service, patch.select.as_str()));
        if let Some(rename) = patch.rename.as_deref() {
            if rename.trim().is_empty() {
                problems.push(format!(
                    "event patch for {:?} has an empty `rename`",
                    patch.select
                ));
            } else if renamed_events.contains(&rename) {
                problems.push(format!(
                    "`[[patch.events]]` renames two events to {rename:?}"
                ));
            }
            renamed_events.push(rename);
        }
        if let Some(alternatives) = &patch.auth {
            validate_requirements(
                &loaded.connector,
                alternatives,
                &format!("event patch for {:?}", patch.select),
                problems,
            );
        }
    }

    validate_selectors(&loaded.patch, problems);
    if let Some(naming) = loaded.patch.naming.as_ref() {
        validate_naming(naming, problems);
    }
}

/// The `[[patch.select]]` statements themselves — C-411.
///
/// Only what can be judged without a document. Whether a selector *matches* anything is
/// [`publish`]'s, because it needs the ingest; whether it is a well-formed statement is here, so
/// `load` refuses a malformed one exactly as `load_with_spec` does.
fn validate_selectors(patch: &Patch, problems: &mut Vec<String>) {
    for selector in &patch.select {
        let subject = selector.describe();
        if let Some(prefix) = selector.path_prefix.as_deref() {
            let prefix = prefix.trim();
            if prefix.is_empty() {
                problems.push(format!(
                    "{subject} states an empty `path_prefix`. Omit the key to match every path in \
                     the document — an empty string is the same statement written so it reads like \
                     a mistake"
                ));
            } else if !prefix.starts_with('/') {
                problems.push(format!(
                    "{subject} states `path_prefix = {prefix:?}`, which must start with `/`: it is \
                     matched against the document's own path templates, and those do"
                ));
            }
        }
    }
}

/// The `[patch.naming]` declaration itself — C-412.
///
/// The pins are checked against the documents in [`check_pins`]; this is the half that holds
/// without one, so a prefix or a pinned value that could never produce a legal op id is refused
/// even by [`load`].
fn validate_naming(naming: &Naming, problems: &mut Vec<String>) {
    if let Some(prefix) = naming.prefix.as_deref() {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            problems.push(
                "`[patch.naming] prefix` is empty. Omit the key for no prefix — an empty string is \
                 the same statement written so it reads like a mistake"
                    .to_owned(),
            );
        } else if let Err(reason) = legal_op_id(prefix) {
            problems.push(format!(
                "`[patch.naming] prefix = {prefix:?}` cannot begin a legal op id: {reason}"
            ));
        }
    }

    for (operation_id, pinned) in &naming.pin {
        if operation_id.trim().is_empty() {
            problems.push(
                "`[patch.naming.pin]` has an entry with an empty key; a pin is keyed by the spec's \
                 `operationId`"
                    .to_owned(),
            );
            continue;
        }
        if let Err(reason) = legal_op_id(pinned.trim()) {
            problems.push(format!(
                "`[patch.naming.pin]` pins {operation_id:?} to {pinned:?}, which is not a legal op \
                 id: {reason}"
            ));
        }
    }
}
