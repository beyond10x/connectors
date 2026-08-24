use super::*;

/// **The whole overlay: select, then patch, then publish** — C-6, widened by C-411/412/414.
///
/// The order is [`Patch`]'s and it is total. Selectors state what they state about the sets they
/// matched; a `[[patch.operations]]` block overrides that field by field for the one operation it
/// names; and everything neither statement covers falls to the per-field rules, each of which either
/// has a value nobody can get wrong or refuses.
///
/// Returns the operations to publish. Every failure is a pushed problem and a skipped operation
/// rather than an early return, so a file with fifty bad statements reports fifty lines.
pub(super) fn publish(
    patch: &Patch,
    specs: &[SpecSource],
    ingested: &[IngestedDocument],
    documents: &[SpecDocument<'_>],
    config: &[ConfigField],
    problems: &mut Vec<String>,
) -> (Vec<Operation>, BTreeMap<String, OperationSpecSource>) {
    if let Some(naming) = patch.naming.as_ref() {
        check_pins(naming, ingested, problems);
    }
    check_directions(&patch.directions, ingested, problems);
    check_descriptions(&patch.descriptions, ingested, problems);
    let naming = patch.naming.as_ref();

    // **2 · select.** What every selector states about every operation it matched, merged — and a
    // disagreement between two of them refused here rather than resolved by declaration order.
    let mut matched: BTreeMap<(&str, &str), Stated> = BTreeMap::new();
    for selector in &patch.select {
        let subject = selector.describe();
        let Some(document) = resolve_document(
            ingested,
            specs,
            selector.service.as_deref(),
            &subject,
            problems,
        ) else {
            continue;
        };

        let mut hits = 0usize;
        for operation in &document.ingested.operations {
            if !selector.matches(operation) {
                continue;
            }
            // **Matched but not eligible.** A bulk statement never asked for the vendor's
            // internals, so sweeping one up is silent; naming one by hand is a different act and is
            // refused below. Counted out of `hits` so a selector that reached *only* internal paths
            // still reports as matching nothing.
            if is_internal(&operation.path) {
                continue;
            }
            hits += 1;
            matched
                .entry((document.service.as_str(), operation.operation_id.as_str()))
                .or_default()
                .absorb(selector, &subject, &operation.operation_id, problems);
        }

        if hits == 0 {
            problems.push(format!(
                "{subject} matches no operation in {}. A selector that selects nothing is refused \
                 for the same reason a `select` naming an absent `operationId` is: a prefix that \
                 stopped matching after an upstream reshuffle would empty this connector quietly \
                 and the build would stay green",
                document.path
            ));
        }
    }

    // **3 · per-operation patch.** File order, and it wins field by field over any selector that
    // also matched — which is why the selector's statement is looked up rather than discarded.
    let mut published: Vec<Operation> = Vec::new();
    let mut operation_specs: BTreeMap<String, OperationSpecSource> = BTreeMap::new();
    let mut taken: BTreeMap<String, Claim> = BTreeMap::new();
    let mut claimed: BTreeSet<(&str, &str)> = BTreeSet::new();

    for block in &patch.operations {
        let select = block.select.as_str();
        let subject = format!("patch for {select:?}");
        let Some(document) = resolve_document(
            ingested,
            specs,
            block.service.as_deref(),
            &subject,
            problems,
        ) else {
            continue;
        };
        let Some(spec) = document.ingested.operation(select) else {
            // Loud rather than a silent no-op, because a `select` that quietly matches nothing is
            // how a patch set rots underneath a vendor's rename: the operation disappears from the
            // connector and the build stays green.
            problems.push(format!(
                "`[[patch.operations]] select = {select:?}` names no `operationId` in {}. {}",
                document.path,
                nearest(&document.ingested, select)
            ));
            continue;
        };
        claimed.insert((document.service.as_str(), select));

        if is_internal(&spec.path) {
            problems.push(format!(
                "`[[patch.operations]] select = {select:?}` names an operation whose path {:?} \
                 carries an `internal` segment. An endpoint a vendor keeps behind that word is not \
                 a supported call, so it is selectable neither in bulk nor by name",
                spec.path
            ));
            continue;
        }

        let stated = matched.get(&(document.service.as_str(), select));
        let reviewed_direction = direction_for(patch, &document.service, select);
        let reviewed_description = description_for(patch, &document.service, select);
        if let Some(reason) = block.defer.as_deref() {
            let mut incompatible = Vec::new();
            if block.rename.is_some() {
                incompatible.push("rename");
            }
            if block.description.is_some() {
                incompatible.push("description");
            }
            if block.risk.is_some() {
                incompatible.push("risk");
            }
            if block.idempotency.is_some() {
                incompatible.push("idempotency");
            }
            if block.auth.is_some() {
                incompatible.push("auth");
            }
            if block.pagination.is_some() {
                incompatible.push("pagination");
            }
            if block.rate_limit.is_some() {
                incompatible.push("rate_limit");
            }
            if block.error_envelope.is_some() {
                incompatible.push("error_envelope");
            }
            if !block.params.is_empty() {
                incompatible.push("params");
            }
            if !block.omit.is_empty() {
                incompatible.push("omit");
            }
            if block.expose.is_some() {
                incompatible.push("expose");
            }

            if stated.is_none() {
                problems.push(format!(
                    "`[[patch.operations]] select = {select:?}` uses `defer`, but no \
                     `[[patch.select]]` matched that operation. Deferral may only narrow an \
                     explicitly selected set; it is not an opt-out selection mechanism"
                ));
            }
            if reason.trim().is_empty() {
                problems.push(format!(
                    "`[[patch.operations]] select = {select:?}` uses `defer` without a non-empty \
                     reason. A withheld operation must say what model or prerequisite keeps it out"
                ));
            }
            if !incompatible.is_empty() {
                problems.push(format!(
                    "`[[patch.operations]] select = {select:?}` defers the operation and also \
                     states {}, but corrections to an operation that will not publish have no \
                     effect. Keep only `service`, `select` and `defer`",
                    incompatible.join(", ")
                ));
            }
            continue;
        }
        let source = source_of(specs, document);
        if let Some((operation, claim)) = compose(
            document,
            spec,
            ComposeOverlay {
                patch: Some(block),
                reviewed_direction,
                reviewed_description,
                selected: stated.is_some(),
                stated: stated.unwrap_or(&Stated::EMPTY),
                naming,
            },
            &mut ComposeContext { config, problems },
        ) {
            offer(
                &mut taken,
                &mut published,
                &mut operation_specs,
                operation,
                claim,
                operation_source(source, document, documents, select),
                problems,
            );
        }
    }

    // Everything a selector matched that no block named, in document order per `[[spec]]` entry —
    // so the published order is a function of the inputs and of nothing else.
    for document in ingested {
        let source = source_of(specs, document);
        for spec in &document.ingested.operations {
            let key = (document.service.as_str(), spec.operation_id.as_str());
            if claimed.contains(&key) {
                continue;
            }
            let Some(stated) = matched.get(&key) else {
                continue;
            };
            let reviewed_direction = direction_for(patch, &document.service, &spec.operation_id);
            let reviewed_description =
                description_for(patch, &document.service, &spec.operation_id);
            if let Some((operation, claim)) = compose(
                document,
                spec,
                ComposeOverlay {
                    patch: None,
                    reviewed_direction,
                    reviewed_description,
                    selected: true,
                    stated,
                    naming,
                },
                &mut ComposeContext { config, problems },
            ) {
                offer(
                    &mut taken,
                    &mut published,
                    &mut operation_specs,
                    operation,
                    claim,
                    operation_source(source, document, documents, &spec.operation_id),
                    problems,
                );
            }
        }
    }

    (published, operation_specs)
}

/// The exact pin that produced one ingested document.
///
/// Both path and service participate. A service-only lookup would recreate C-481's defect for a
/// mixed or multi-document provider. [`ingest_specs`] establishes this pair when it resolves each
/// [`SpecSource`], so absence here is an internal invariant failure rather than provider input.
fn source_of<'a>(specs: &'a [SpecSource], document: &IngestedDocument) -> &'a SpecSource {
    specs
        .iter()
        .find(|source| source.path == document.path && source.service() == document.service)
        .expect("every ingested document came from one exact SpecSource")
}

/// Public operation provenance projected from one pin, with no local refresh metadata.
fn operation_source(
    source: &SpecSource,
    ingested: &IngestedDocument,
    documents: &[SpecDocument<'_>],
    operation_id: &str,
) -> OperationSpecSource {
    let document = documents
        .iter()
        .find(|document| document.path == ingested.path)
        .expect("every ingested document came from one provided document");
    OperationSpecSource {
        operation_id: operation_id.to_owned(),
        source_url: source.source_url.clone(),
        upstream_version: ingested.ingested.upstream_version.clone(),
        sha256: sha256_hex(document.document.as_bytes()),
    }
}

/// What the selectors that matched one operation stated about it, and which one stated each field.
///
/// The second half of each pair is what makes a disagreement reportable: "two selectors disagree"
/// is not actionable, and "`path_prefix = "/api/v2/agents"` and `path_prefix =
/// "/api/v2/agents/{id}"` disagree about `risk`" is.
#[derive(Debug, Clone, Default)]
struct Stated {
    risk: Option<(Risk, String)>,
    idempotency: Option<(Idempotency, String)>,
    effects: Option<(Vec<HostEffect>, String)>,
    interaction_shape: Option<(InteractionShape, String)>,
    protocol_driver: Option<(ProtocolDriver, String)>,
    placement_requirement: Option<(PlacementRequirement, String)>,
    implementation_form: Option<(ImplementationForm, String)>,
    required_capabilities: Option<(Vec<RequiredCapability>, String)>,
    expose: Option<(bool, String)>,
}

/// The provider facts one operation composition needs beyond the overlay statements themselves.
struct ComposeContext<'a> {
    config: &'a [ConfigField],
    problems: &'a mut Vec<String>,
}

/// The identity-stable and selector-authored declarations applied to one operation.
struct ComposeOverlay<'a> {
    patch: Option<&'a OperationPatch>,
    reviewed_direction: Option<OperationDirection>,
    reviewed_description: Option<&'a str>,
    selected: bool,
    stated: &'a Stated,
    naming: Option<&'a Naming>,
}

impl Stated {
    /// What a selector states about an operation no selector matched: nothing.
    const EMPTY: Self = Self {
        risk: None,
        idempotency: None,
        effects: None,
        interaction_shape: None,
        protocol_driver: None,
        placement_requirement: None,
        implementation_form: None,
        required_capabilities: None,
        expose: None,
    };

    /// Fold one more selector's statement in, reporting any field the two disagree about.
    fn absorb(
        &mut self,
        selector: &OperationSelector,
        subject: &str,
        operation_id: &str,
        problems: &mut Vec<String>,
    ) {
        agree(
            &mut self.risk,
            selector.risk,
            risk_word,
            "risk",
            subject,
            operation_id,
            problems,
        );
        agree(
            &mut self.idempotency,
            selector.idempotency,
            idempotency_word,
            "idempotency",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.effects,
            selector.effects.as_ref(),
            "effects",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.interaction_shape,
            selector.interaction_shape.as_ref(),
            "interaction_shape",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.protocol_driver,
            selector.protocol_driver.as_ref(),
            "protocol_driver",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.placement_requirement,
            selector.placement_requirement.as_ref(),
            "placement_requirement",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.implementation_form,
            selector.implementation_form.as_ref(),
            "implementation_form",
            subject,
            operation_id,
            problems,
        );
        agree_debug(
            &mut self.required_capabilities,
            selector.required_capabilities.as_ref(),
            "required_capabilities",
            subject,
            operation_id,
            problems,
        );
        agree(
            &mut self.expose,
            selector.expose,
            bool_word,
            "expose",
            subject,
            operation_id,
            problems,
        );
    }
}

fn agree_debug<T: PartialEq + Clone + std::fmt::Debug>(
    held: &mut Option<(T, String)>,
    stated: Option<&T>,
    field: &str,
    subject: &str,
    operation_id: &str,
    problems: &mut Vec<String>,
) {
    let Some(value) = stated else {
        return;
    };
    match held {
        Some((existing, first)) if existing != value => problems.push(format!(
            "two selectors match {operation_id:?} and disagree about `{field}`: {first} states \
             {existing:?} and {subject} states {value:?}. Overlapping selectors are legal only \
             while they agree"
        )),
        Some(_) => {}
        None => *held = Some((value.clone(), subject.to_owned())),
    }
}

fn required_declaration<T: Clone>(
    exact: Option<&T>,
    selected: Option<&(T, String)>,
    field: &str,
    operation_id: &str,
    problems: &mut Vec<String>,
) -> Option<T> {
    exact
        .cloned()
        .or_else(|| selected.map(|(value, _)| value.clone()))
        .or_else(|| {
            problems.push(format!(
            "{operation_id:?} states no `{field}` on its exact `[[patch.operations]]` block or \
             reviewed `[[patch.select]]`; this fact is required and is never inferred from \
             direction, method, host, risk, or driver"
        ));
            None
        })
}

/// Merge one field of one selector's statement into what is already held for an operation.
///
/// Silence is not disagreement — a selector that states nothing about `risk` is not fighting with
/// one that does, it is simply saying less. Two *stated* values that differ are refused, because
/// picking one would make the merge order depend on the order the selectors happen to be written
/// in, and an author would have no way to see which won.
fn agree<T: PartialEq + Copy>(
    held: &mut Option<(T, String)>,
    stated: Option<T>,
    word: fn(T) -> &'static str,
    field: &str,
    subject: &str,
    operation_id: &str,
    problems: &mut Vec<String>,
) {
    let Some(value) = stated else {
        return;
    };
    match held {
        Some((existing, first)) if *existing != value => problems.push(format!(
            "two selectors match {operation_id:?} and disagree about `{field}`: {first} states \
             {:?} and {subject} states {:?}. Overlapping selectors are legal only while they \
             agree — two statements fighting over one operation is how the merge order stops being \
             total",
            word(*existing),
            word(value)
        )),
        Some(_) => {}
        None => *held = Some((value, subject.to_owned())),
    }
}

/// Where a published op id came from, for a collision that has to explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdSource {
    /// A `[[patch.operations]] rename`.
    Renamed,
    /// A `[patch.naming.pin]` entry.
    Pinned,
    /// The `[patch.naming]` rule.
    Derived,
}

/// One published op id and the statement that produced it.
#[derive(Debug, Clone)]
struct Claim {
    service: String,
    operation_id: String,
    source: IdSource,
}

/// Publish an operation unless its op id is already claimed — **collisions refuse**.
///
/// An op id is what a user or a model calls by name, so two operations deriving one id cannot be
/// resolved by order: whichever lost would still be documented, still be catalogued, and simply be
/// unreachable. The colliding operation is dropped rather than published so that
/// [`validate_operations`] does not report the same cause a second time in different words.
///
/// Two **authored** `rename`s colliding is left alone: `validate_patch` already reports that case
/// with a message about the `rename` key, which is the one an author would go and edit.
fn offer(
    taken: &mut BTreeMap<String, Claim>,
    published: &mut Vec<Operation>,
    operation_specs: &mut BTreeMap<String, OperationSpecSource>,
    operation: Operation,
    claim: Claim,
    spec_source: OperationSpecSource,
    problems: &mut Vec<String>,
) {
    if let Some(first) = taken.get(&operation.id) {
        if first.source == IdSource::Renamed && claim.source == IdSource::Renamed {
            operation_specs.insert(operation.id.clone(), spec_source);
            published.push(operation);
            return;
        }
        problems.push(format!(
            "op id {:?} is claimed twice: by `operationId` {:?} in service {:?} and by {:?} in \
             service {:?}. An op id is the public name users and models call, so two operations \
             deriving one is refused rather than resolved by order — pin one of them with \
             `[patch.naming.pin]`, or rename it with a `[[patch.operations]]` block, which states \
             its `service` and outranks a pin",
            operation.id, first.operation_id, first.service, claim.operation_id, claim.service
        ));
        return;
    }
    taken.insert(operation.id.clone(), claim);
    operation_specs.insert(operation.id.clone(), spec_source);
    published.push(operation);
}

/// One ingested operation plus everything stated about it, or a problem saying why not.
///
/// This is where the three declarations meet, and every field resolves by the same sentence:
/// **the block, then the selector, then the rule for that field.**
fn compose(
    document: &IngestedDocument,
    spec: &crate::openapi::SpecOperation,
    overlay: ComposeOverlay<'_>,
    context: &mut ComposeContext<'_>,
) -> Option<(Operation, Claim)> {
    let ComposeOverlay {
        patch,
        reviewed_direction,
        reviewed_description,
        selected,
        stated,
        naming,
    } = overlay;
    let config = context.config;
    let problems = &mut *context.problems;
    let select = spec.operation_id.as_str();

    // **Naming: `rename`, then a pin, then the rule.** An op id is a public contract users and
    // models call by name and `operationId` is a volatile vendor field, so nothing here promotes
    // one into the other by silence — `predecessor:docs/designs/connector-pipeline.md`, "Op naming is a public
    // contract". C-412 makes the *pinned override* bulk; it does not remove the requirement to
    // decide.
    let (id, source) = match patch.and_then(|patch| patch.rename.clone()) {
        Some(rename) => (rename, IdSource::Renamed),
        None => match naming {
            Some(naming) => match naming.derive(select) {
                Ok(id) => (
                    id,
                    if naming.pin.contains_key(select) {
                        IdSource::Pinned
                    } else {
                        IdSource::Derived
                    },
                ),
                Err(reason) => {
                    problems.push(format!(
                        "`operationId` {select:?} in {} derives no legal op id: {reason}. A name a \
                         user calls is never mangled into one that happens to parse — pin this \
                         operation with `[patch.naming.pin]`, or select it with a \
                         `[[patch.operations]]` block that states `rename`",
                        document.path
                    ));
                    return None;
                }
            },
            None if patch.is_some() => {
                problems.push(format!(
                    "patch for {select:?} states no `rename`. An op id is a public name that users \
                     and models call, and `operationId` is a volatile vendor field, so ingest will \
                     not promote one into one — state `rename`, or declare a `[patch.naming]` rule"
                ));
                return None;
            }
            None => {
                problems.push(format!(
                    "a `[[patch.select]]` matched {select:?} in {}, but this connector declares no \
                     `[patch.naming]` rule, so nothing says what to publish it as. An op id is a \
                     public name that users and models call — declare `[patch.naming]`, or select \
                     this operation with a `[[patch.operations]]` block that states `rename`",
                    document.path
                ));
                return None;
            }
        },
    };

    // **Direction: stable identity only.** An exact operation block and the service/operationId map
    // are both immune to method/path/name rematching. A selector cannot state direction.
    let exact_direction = patch.and_then(|patch| patch.direction);
    if let (Some(exact), Some(reviewed)) = (exact_direction, reviewed_direction) {
        if exact != reviewed {
            problems.push(format!(
                "{select:?} has conflicting identity-stable directions: its \
                 `[[patch.operations]]` block says {:?}, while \
                 `[patch.directions.{}]` says {:?}",
                exact.word(),
                document.service,
                reviewed.word()
            ));
            return None;
        }
    }
    let direction = exact_direction.or(reviewed_direction);
    let Some(direction) = direction else {
        problems.push(format!(
            "{select:?} states no `direction`. HTTP method, operation name, description, risk, \
             idempotency, semantic effects and exposure cannot prove whether vendor state changes \
             — state it under `[patch.directions.{}]` keyed by this vendor `operationId`, or on its \
             exact `[[patch.operations]]` block",
            document.service
        ));
        return None;
    };

    let exact_description = patch.and_then(|patch| patch.description.as_deref());
    if let (Some(exact), Some(reviewed)) = (exact_description, reviewed_description) {
        if exact != reviewed {
            problems.push(format!(
                "{select:?} has conflicting identity-stable descriptions: its \
                 `[[patch.operations]]` block and `[patch.descriptions.{}]` disagree",
                document.service
            ));
            return None;
        }
    }
    let description = exact_description
        .or(reviewed_description)
        .unwrap_or(spec.description.as_str())
        .to_owned();

    // **Risk and idempotency: the block, then the selector, then authored direction.** See
    // [`OperationSelector::idempotency`] for why the last step exists on a read and refuses on a
    // write, and why that asymmetry is the safe direction rather than a convenience.
    let risk = patch
        .and_then(|patch| patch.risk)
        .or(stated.risk.as_ref().map(|(value, _)| *value));
    let idempotency = patch
        .and_then(|patch| patch.idempotency)
        .or(stated.idempotency.as_ref().map(|(value, _)| *value));
    let mutating = direction == OperationDirection::Write;

    let (risk, idempotency) = match (risk, idempotency) {
        (Some(risk), Some(idempotency)) => (risk, idempotency),
        // A read a selector matched takes the two values a read cannot have wrong. This is the
        // only default in the whole overlay, and it is available only to an authored read.
        (risk, idempotency) if !mutating && selected => (
            risk.unwrap_or(Risk::Low),
            idempotency.unwrap_or(Idempotency::Idempotent),
        ),
        (risk, idempotency) => {
            let missing = match (risk, idempotency) {
                (None, Some(_)) => "`risk`",
                (Some(_), None) => "`idempotency`",
                _ => "`risk` and no `idempotency`",
            };
            problems.push(if selected {
                format!(
                    "{select:?} is an authored {} and states no {missing}. No OpenAPI document \
                     publishes either, and silence about damage on a write is \
                     refused rather than defaulted to `low` — state it on the `[[patch.select]]` \
                     that matched this operation, or on a `[[patch.operations]]` block for it",
                    direction.word()
                )
            } else {
                format!(
                    "patch for {select:?} states no {missing}. No OpenAPI document publishes \
                     either, so a selected operation states both or is not published; guessing on \
                     the operation's behalf is how a `retry` turns one charge into three and how a \
                     delete is waved through an approval gate"
                )
            });
            return None;
        }
    };

    let effects = required_declaration(
        patch.and_then(|patch| patch.effects.as_ref()),
        stated.effects.as_ref(),
        "effects",
        select,
        problems,
    )?;
    let interaction_shape = required_declaration(
        patch.and_then(|patch| patch.interaction_shape.as_ref()),
        stated.interaction_shape.as_ref(),
        "interaction_shape",
        select,
        problems,
    )?;
    let protocol_driver = required_declaration(
        patch.and_then(|patch| patch.protocol_driver.as_ref()),
        stated.protocol_driver.as_ref(),
        "protocol_driver",
        select,
        problems,
    )?;
    let placement_requirement = required_declaration(
        patch.and_then(|patch| patch.placement_requirement.as_ref()),
        stated.placement_requirement.as_ref(),
        "placement_requirement",
        select,
        problems,
    )?;
    let implementation_form = required_declaration(
        patch.and_then(|patch| patch.implementation_form.as_ref()),
        stated.implementation_form.as_ref(),
        "implementation_form",
        select,
        problems,
    )?;
    let required_capabilities = required_declaration(
        patch.and_then(|patch| patch.required_capabilities.as_ref()),
        stated.required_capabilities.as_ref(),
        "required_capabilities",
        select,
        problems,
    )?;

    let mut params = spec.params.clone();
    if let Some(patch) = patch {
        for correction in &patch.params {
            correct(&mut params, correction, select, problems);
        }
        // **Corrections first, then omissions**, because the omission rules read the corrected
        // parameter: `required` is refused as the *connector* states it, not as the vendor guessed
        // it. A document that marks a filter required when it is not would otherwise pin that
        // argument into the tool with no way out, and the way out has to stay a written statement —
        // correct the flag, then drop the parameter.
        for (position, name) in patch.omit.entries() {
            omit(
                &mut params,
                position,
                name,
                select,
                &document.service,
                config,
                problems,
            );
        }
    }

    let request = match protocol_driver {
        ProtocolDriver::HttpV1 => OperationRequest::HttpV1 {
            method: spec.method,
            path: spec.path.clone(),
        },
        ProtocolDriver::SipV1 => OperationRequest::SipV1,
        ProtocolDriver::AudioV1 => OperationRequest::AudioV1,
        ProtocolDriver::CdpV1 => OperationRequest::CdpV1,
        ProtocolDriver::SqlV1 => OperationRequest::SqlV1,
    };

    let operation = Operation {
        id,
        // **The document decides the service, not the patch's own opinion of it** — C-410. A
        // `[[spec]]` entry becomes a service and a patch selects out of a document, so the two
        // statements are one and cannot disagree. Before C-410 every selected operation landed in
        // `DEFAULT_SERVICE`, which made a provider declaring named services beside a `[spec]` a loud
        // load error and a single-document one the only shape that worked.
        service: document.service.clone(),
        request,
        direction,
        description,
        risk,
        idempotency,
        effects,
        semantic_effects: patch
            .and_then(|patch| patch.semantic_effects.clone())
            .unwrap_or_default(),
        interaction_shape,
        placement_requirement,
        implementation_form,
        required_capabilities,
        // **Never stated in bulk.** A selector may declare `idempotency = "conditional"`, and each
        // matched write then still owes the condition C-186 requires — which arrives here as `None`
        // and is refused, by name, by `validate_repeatability_condition`. One sentence about 54
        // endpoints is not a condition, so there is no key here for a selector to write it in.
        repeatable_because: None,
        auth: patch.and_then(|patch| patch.auth.clone()),
        params,
        response_schema: spec.response_schema.clone(),
        // **A vendor document cannot make this claim, and no `[[patch.operations]]` key writes it
        // either** (C-430). "This response field is a credential" is a judgement about what a value
        // *is*; a document that returns a token describes it as a string like any other, which is
        // precisely how postmark's `ApiTokens` and zoom's `start_url` shipped. So the spec route
        // lands `[]` and an author who finds one states it in a `[[operations]]` block, where the
        // gate refuses it and a reviewer reads the reason beside it.
        credential_response: Vec::new(),
        // **And a vendor document cannot make this one either** (C-136). "This call's purpose is to
        // mint a credential" is a judgement about what an endpoint is *for*, and a token endpoint is
        // described by its document as an operation returning a JSON object like any other. There is
        // no `[[patch.operations]]` key for it for the same reason `credential_response` has none:
        // the declaration belongs beside the reviewer who read the vendor's own documentation, in a
        // `[[operations]]` block.
        produces_credential: None,
        pagination: patch.and_then(|patch| patch.pagination.clone()),
        rate_limit: patch.and_then(|patch| patch.rate_limit.clone()),
        error_envelope: patch.and_then(|patch| patch.error_envelope.clone()),
        // **The block, then the selector, then the field's own default** — which is exposed, so a
        // connector nobody said anything about behaves exactly as it did before C-413. `exposed()`
        // rather than a bare `true` so the spec route and the file route take one default from one
        // place and cannot drift into a catalogue nobody re-reads.
        expose: patch
            .and_then(|patch| patch.expose)
            .or(stated.expose.as_ref().map(|(value, _)| *value))
            .unwrap_or_else(crate::ir::exposed),
    };

    Some((
        operation,
        Claim {
            service: document.service.clone(),
            operation_id: select.to_owned(),
            source,
        },
    ))
}

/// Every `[patch.naming.pin]` entry names an operation, and names exactly one — C-412.
///
/// A pin that matches nothing is the rot `select` is already loud about, one field over: the vendor
/// renames an `operationId`, the pin stops applying, and the op id it was holding still quietly
/// moves to whatever the rule derives. A pin that matches *two* is the C-410 problem in a key that
/// cannot carry a service — babelforce declares `getUser` in `manager` and in `user` — so it is
/// refused rather than applied to both, which would only collide one step later with a worse
/// message.
fn check_pins(naming: &Naming, ingested: &[IngestedDocument], problems: &mut Vec<String>) {
    for operation_id in naming.pin.keys() {
        let declaring: Vec<&str> = ingested
            .iter()
            .filter(|document| document.ingested.operation(operation_id).is_some())
            .map(|document| document.service.as_str())
            .collect();

        match declaring.len() {
            0 => problems.push(format!(
                "`[patch.naming.pin]` pins {operation_id:?}, which no vendored document declares. \
                 A pin that matches nothing is how a public name rots underneath a vendor's \
                 rename: the pin stops applying, the rule derives a different id, and the build \
                 stays green"
            )),
            1 => {}
            count => problems.push(format!(
                "`[patch.naming.pin]` pins {operation_id:?}, which {count} of this connector's \
                 documents declare ({}). An `operationId` is unique inside one document and \
                 nowhere else, so one pin cannot say which of them it means — name the one you \
                 mean with a `[[patch.operations]]` block, which states its `service` and outranks \
                 a pin",
                declaring.join(", ")
            )),
        }
    }
}

/// Apply one [`ParamPatch`] to a selected operation's parameters.
///
/// A correction that matches nothing is a problem, not a no-op: it is the same rot a `select`
/// naming an absent operation is, one level down — the vendor renamed a field and the correction
/// that used to fix its type silently stopped applying.
fn correct(
    params: &mut ParamSet,
    correction: &ParamPatch,
    select: &str,
    problems: &mut Vec<String>,
) {
    let group = match correction.position {
        ParamPosition::Path => &mut params.path,
        ParamPosition::Query => &mut params.query,
        ParamPosition::Header => &mut params.header,
        ParamPosition::Body => &mut params.body,
    };
    let Some(param) = group.iter_mut().find(|param| param.name == correction.name) else {
        problems.push(format!(
            "patch for {select:?} corrects a `{:?}` parameter named {:?}, which the vendored spec \
             does not declare there",
            correction.position, correction.name
        ));
        return;
    };
    if let Some(required) = correction.required {
        param.required = required;
    }
    if let Some(description) = &correction.description {
        param.description = description.clone();
    }
    if let Some(schema) = &correction.schema {
        param.schema = schema.clone();
    }
}

/// Drop one parameter a [`ParamOmission`] names from a selected operation — C-422.
///
/// Three refusals, and each is the same sentence pointed somewhere different: **an omission may only
/// drop a parameter the request can still be composed without, and only one the document actually
/// declares.**
///
/// - **A name the document does not declare there** is a problem rather than a no-op, for the reason
///   [`correct`] gives one: the vendor renames a parameter, the line that used to drop it stops
///   applying, and the argument this connector removed on purpose is silently back in the tool with
///   the build green. It is also what catches a name listed twice — the second lookup finds nothing,
///   because the first one removed it.
/// - **A required parameter** composes a request the vendor rejects. Every other consequence of
///   omission is a wider or narrower tool; this one is a runtime failure, so it is the case where
///   silence would be actively unsafe rather than merely unhelpful. Judged *after* corrections, so
///   an author who believes the vendor's flag is wrong says so in `params` and is then free to drop
///   it.
/// - **A path parameter without an exact configuration pin.** The path template keeps its
///   placeholder, so dropping `id` from `/tickets/{id}` leaves a URL nothing can fill — unless the
///   operation's service declares `path.id`, in which case omission is what prevents the same tenant
///   scope from also being a caller argument. The pin is exact and service-scoped; no name or service
///   inference is performed.
fn omit(
    params: &mut ParamSet,
    position: ParamPosition,
    name: &str,
    select: &str,
    service: &str,
    config: &[ConfigField],
    problems: &mut Vec<String>,
) {
    let group = match position {
        ParamPosition::Path => &mut params.path,
        ParamPosition::Query => &mut params.query,
        ParamPosition::Header => &mut params.header,
        ParamPosition::Body => &mut params.body,
    };
    let Some(index) = group.iter().position(|param| param.name == name) else {
        problems.push(format!(
            "patch for {select:?} omits a `{position:?}` parameter named {name:?}, which the \
             vendored spec does not declare there"
        ));
        return;
    };
    if position == ParamPosition::Path {
        let pinned = config
            .iter()
            .filter(|field| field.service == service)
            .any(|field| {
                field
                    .pins()
                    .iter()
                    .any(|pin| pin.position == Position::Path && pin.name == name)
            });
        if pinned {
            group.remove(index);
            return;
        }
        problems.push(format!(
            "patch for {select:?} omits the path parameter {name:?}, which cannot be dropped: the \
             path template still carries `{{{name}}}` and nothing composes a URL with that left in \
             it. A path parameter leaves only when the path does, or when this service declares an \
             exact `path.{name}` configuration pin"
        ));
        return;
    }
    if group[index].required {
        problems.push(format!(
            "patch for {select:?} omits {name:?}, which the vendored spec declares **required** — \
             dropping it composes a request the vendor rejects. If the vendor's flag is wrong, \
             correct it with a `[[patch.operations.params]]` block stating `required = false` and \
             then omit it"
        ));
        return;
    }
    group.remove(index);
}

/// What the spec cache actually holds, for a refusal about a pin that resolved to nothing.
///
/// The paths, not a count: an author who mistyped a pin needs to see the spelling that would have
/// worked, and one who vendored nothing needs to be told that rather than left comparing a number
/// against a directory listing.
pub(super) fn describe_cache(documents: &[SpecDocument<'_>]) -> String {
    if documents.is_empty() {
        "The spec cache holds no document for this provider at all — the cache is committed, so a \
         pointer at a file that is not there is a connector that cannot be built rather than one \
         that builds empty"
            .to_owned()
    } else {
        format!(
            "The cache holds {}",
            documents
                .iter()
                .map(|document| document.path)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// What to say after "names no `operationId`" — the closest spellings, or how many there were.
///
/// A document with 356 operations cannot have them all listed in a refusal, and a bare count helps
/// nobody. A prefix match catches the overwhelmingly common cause, which is a typo or a vendor's
/// casing change.
fn nearest(ingested: &crate::openapi::Ingested, select: &str) -> String {
    let folded = select.to_lowercase();
    let near: Vec<&str> = ingested
        .operation_ids()
        .into_iter()
        .filter(|id| {
            let id = id.to_lowercase();
            id.starts_with(&folded) || folded.starts_with(&id) || id.contains(&folded)
        })
        .take(5)
        .collect();
    if near.is_empty() {
        format!(
            "The document declares {} operations, none of them by that name",
            ingested.operations.len()
        )
    } else {
        format!("Did you mean {}?", near.join(", "))
    }
}

/// The refusal for a provider-level `roles` key — C-120.
///
/// A provider's roles are the union of its services' and are computed, so the key does not exist at
/// that level at all. Saying only "unknown field" would leave an author to guess; the message that
/// pays for itself names the level that does own it, including for the single-surface case, which is
/// the one an author is most likely to be in when they reach for the key.
pub(super) const PROVIDER_LEVEL_ROLES: &str = "\
    `roles` is not a provider-level key. A role is a capability of one API surface, so it is \
    declared on a `[[services]]` entry, and a provider's roles are derived as the union of its \
    services' — never authored, for the reason a config field's `level` is derived from its \
    `binds`. A provider with a single API surface declares `[[services]]` with `name = \"default\"` \
    and puts them there";

/// Whether the file states a **top-level** `roles` key, so [`load`] can say where it belongs.
///
/// Reached only when the typed parse has already failed, and deliberately tolerant: a file too
/// malformed to parse as a table is not a roles problem, so it falls through to `toml`'s own error.
pub(super) fn declares_provider_roles(source: &str) -> bool {
    source
        .parse::<toml::Table>()
        .is_ok_and(|table| table.contains_key("roles"))
}

/// Turns the parsed file into a [`LoadedProvider`], folding `[spec]` into the connector's
/// provenance and distributing provider-level constant headers onto every operation. No validation
/// happens here — assembling and judging are separate so that validation can see the finished value
/// and report on all of it at once.
pub(super) fn assemble(
    file: ProviderFile,
    source: &str,
    implicit_service_members: Vec<ImplicitServiceMember>,
) -> LoadedProvider {
    let specs = file.specs;
    let mut operations = file.operations;
    distribute_const_headers(&file.const_headers, &mut operations);
    // **The four scalar fields describe a connector, so they are filled only when one document
    // describes the connector** — C-410. With several documents there is no single `sha256`,
    // `fetched_at` or `upstream_version` that is true, and filling them from the first would record
    // one document's provenance as the whole connector's. `Provenance::specs` is the per-document
    // record and is filled either way.
    let sole = specs.first().filter(|_| specs.len() == 1);
    let provenance = Provenance {
        source_url: sole.and_then(|s| s.source_url.clone()),
        upstream_version: sole.and_then(|s| s.upstream_version.clone()),
        fetched_at: sole.and_then(|s| s.fetched_at.clone()),
        spec_sha256: sole.and_then(|s| s.sha256.clone()),
        specs: specs.clone(),
        operation_specs: BTreeMap::new(),
        toml_sha256: Some(sha256_hex(source.as_bytes())),
    };

    LoadedProvider {
        connector: Connector {
            id: file.id,
            authority: file.authority,
            api_version: file.api_version,
            services: file.services,
            vendor: file.vendor,
            base_url: file.base_url,
            description: file.description,
            auth: file.auth,
            default_auth: file.default_auth,
            operations,
            events: file.events,
            channels: file.channels,
            discoveries: file.discoveries,
            config: file.config,
            verify: file.verify,
            graphs: file.graphs,
            provenance,
        },
        specs,
        patch: file.patch,
        // Filled by `ingest_specs` when documents were supplied; assembling reads the TOML alone.
        ingested: Vec::new(),
        ingested_events: Vec::new(),
        implicit_service_members,
    }
}

/// Records which service-bearing TOML tables omitted `service` before serde turns that omission
/// into `default`.
///
/// The normalized IR deliberately has one spelling for the default service. That stays correct for
/// default-only connectors; C-458 adds one authoring-time distinction in a mixed connector, so the
/// loader retains only the presence bit and discards the raw TOML immediately after validation.
pub(super) fn implicit_service_members(source: &str) -> Vec<ImplicitServiceMember> {
    let table: toml::Table = source
        .parse()
        .expect("ProviderFile already parsed this source as TOML");
    let mut omitted = Vec::new();

    for (key, kind, identity) in [
        ("operations", "operation", "id"),
        ("events", "event", "name"),
        ("channels", "channel binding", "name"),
        ("discoveries", "discovery", "id"),
        ("config", "configuration field", "name"),
        ("graphs", "graph", "name"),
    ] {
        let Some(entries) = table.get(key).and_then(toml::Value::as_array) else {
            continue;
        };
        for entry in entries.iter().filter_map(toml::Value::as_table) {
            if entry.contains_key("service") {
                continue;
            }
            omitted.push(ImplicitServiceMember {
                kind,
                name: entry
                    .get(identity)
                    .and_then(toml::Value::as_str)
                    .unwrap_or("<unnamed>")
                    .to_owned(),
            });
        }
    }

    match table.get("spec") {
        Some(toml::Value::Table(spec)) if !spec.contains_key("service") => {
            omitted.push(ImplicitServiceMember {
                kind: "spec document",
                name: spec
                    .get("path")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("<unnamed>")
                    .to_owned(),
            });
        }
        Some(toml::Value::Array(specs)) => {
            for spec in specs.iter().filter_map(toml::Value::as_table) {
                if spec.contains_key("service") {
                    continue;
                }
                omitted.push(ImplicitServiceMember {
                    kind: "spec document",
                    name: spec
                        .get("path")
                        .and_then(toml::Value::as_str)
                        .unwrap_or("<unnamed>")
                        .to_owned(),
                });
            }
        }
        _ => {}
    }

    omitted
}

/// Copies the provider's constant headers onto every operation, an operation's own entry winning.
///
/// **Resolved here rather than carried as inheritance**, unlike [`Connector::default_auth`]. Auth
/// inheritance has to survive into the IR because [`Operation::auth`] is a three-state field whose
/// `None` means *inherit* and carries meaning that resolving would erase. A constant header has no
/// such state — it is request content, not policy — so an operation whose IR states every header it
/// sends is one that no consumer (emitter, manifest, catalogue) has to re-derive an inheritance to
/// read. The file keeps the one-line shorthand; the IR is the normalized form, which is what it is
/// for.
///
/// The match is case-insensitive because HTTP field names are (RFC 9110 §5.1). `Notion-Version` and
/// `notion-version` are one header, so keeping both would send it twice with two values; the
/// operation's own spelling and value are the ones that survive.
pub(super) fn distribute_const_headers(
    provider: &BTreeMap<String, String>,
    operations: &mut [Operation],
) {
    if provider.is_empty() {
        return;
    }
    for operation in operations {
        for (name, value) in provider {
            let overridden = operation
                .params
                .const_headers
                .keys()
                .any(|own| own.eq_ignore_ascii_case(name));
            if !overridden {
                operation
                    .params
                    .const_headers
                    .insert(name.clone(), value.clone());
            }
        }
    }
}
