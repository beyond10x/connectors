use super::*;

/// Parses and validates one `providers/<name>.toml`.
///
/// `name` is only ever used to label errors — `providers/zendesk.toml` — so the caller decides how
/// the file identifies itself. `source` is the file's bytes as text; **no IO happens here**.
///
/// The connector's [`Provenance::toml_sha256`] is computed from `source` on the way through, which
/// is what lets `connectors.lock` (C-7) detect an edited provider file without re-reading it.
///
/// # A file that pins a `[spec]` is refused here — C-421
///
/// A spec-backed connector's operations are a function of the file's bytes **and** of the vendored
/// documents it pins. This entry point is handed only the first, so on that input it is being asked
/// a question it does not have the material to answer. Use [`load_with_spec`], which takes the cache.
///
/// Until C-421 it answered anyway. It returned `Ok` with a *skeleton* — the id, the base URL, the
/// credentials, the provenance, and **zero operations** — and every caller in this workspace treated
/// that as a compiled connector. Ninety-one files call this function and eighty-six of them are
/// tests, so the first shipped provider to convert to `[spec]` would have turned the whole
/// catalogue-wide suite into a set of assertions passing vacuously over a connector they believed
/// they had checked. `AGENTS.md`'s "a loud compile-time refusal is better than plausible but
/// incorrect Flux" decides that case, and it decides it against the skeleton.
///
/// **Why the signature did not grow a `documents` parameter instead.** The alternative considered was
/// folding [`load_with_spec`] into this function, so that "load" had one meaning everywhere. It was
/// rejected on what it does to the callers who have no cache — the majority, and every unit test
/// that authors its own TOML. The only argument they could pass is an empty slice, and an empty
/// slice against a pinned `[spec]` already refuses one layer down, in `ingest_specs`, with a message
/// about a pin that resolves to nothing. So the parameter would not give "load" one meaning; it
/// would give it one signature and two meanings, the second spelled `&[]`, and it would put a
/// vestigial argument on roughly forty golden-error tests that will never own a document. Keeping
/// the pure entry point pure and making it *say* what it is missing costs one refusal and no
/// argument, and it leaves the fifty-three hand-authored providers loading byte-identically.
///
/// The split callers face is therefore one sentence: **bytes you read from `providers/` go through
/// [`load_with_spec`] with that provider's cache; TOML you authored yourself goes through here.**
///
/// # Errors
///
/// [`Error::ParseProvider`](crate::Error::ParseProvider) when the file is not well-formed TOML or
/// does not match the schema, and [`Error::InvalidProvider`](crate::Error::InvalidProvider) — with
/// *every* problem found, not just the first — when it parses but is not a valid connector, or when
/// it pins a `[spec]` and so cannot be compiled without one.
pub fn load(name: &str, source: &str) -> crate::Result<LoadedProvider> {
    load_inner(name, source, None)
}

/// One vendored document available to a provider, as the spec cache holds it.
///
/// The [`path`](Self::path) is what makes this a document rather than a pile of bytes: `[spec] path`
/// names exactly one file, and the loader resolves the pin against these rather than trusting a
/// caller to have picked the right one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecDocument<'a> {
    /// The repository-relative path, spelled exactly as `[spec] path` spells it —
    /// `specs/babelforce/manager-2026-07-10.yaml`.
    pub path: &'a str,
    /// The document's bytes as text.
    pub document: &'a str,
}

/// The same, with the vendored documents the provider's spec cache holds — C-4.
///
/// This is the whole spec front-end in one call: **spec -> patch -> validate**, in that fixed order.
/// [`openapi::ingest`] turns the document into every operation it declares, the file's
/// `[[patch.operations]]` says which of them this connector publishes and what it corrects about
/// each, and the result is validated by exactly the same pass a hand-authored file goes through — so
/// a selected operation is held to every rule an inline one is.
///
/// # The pin decides which document is read, and it is resolved here
///
/// `documents` is the **cache**, not a choice already made: every file under `specs/<provider>/`,
/// and this function picks the one `[spec] path` names. That is deliberate and it is load-bearing.
/// The cache ordinarily holds more than one file — `specs/zendesk/2024-06-01.json` beside a later
/// `2025-01-01.json` is what versioning a vendored document *looks* like — so a caller that picked
/// one and passed it alone would be deciding, silently, something only the provider file may decide.
/// A build that compiled `getUser` out of a document the file never named would emit plausible,
/// wrong Flux and exit 0.
///
/// A pin naming a file the cache does not hold is refused, listing what is there.
///
/// # A connector may pin several documents, one per service — C-410
///
/// `[[spec]]` declares a document per [`Service`], and `[spec]` is the one-element case of it. Each
/// entry is resolved, hash-checked and ingested **separately**, and its selected operations join the
/// service the entry names; nothing is merged. That is what lets babelforce be one connector rather
/// than five, and it is also what keeps the manager document's root `oauth2` from describing
/// `task-automation`'s per-operation `bearerAuth`.
///
/// Because two documents may declare one `operationId` — babelforce's `getUser` does — a
/// [`OperationPatch`] states which `service` it reads from as soon as a second document exists.
///
/// # The declared `sha256` is checked against the bytes, not copied past them
///
/// [`SpecSource::sha256`] reaches [`Provenance::specs`] and from there `connectors.lock`. If nothing
/// compared it against the document actually ingested, provenance would be a claim the file makes
/// about itself — and the lockfile would record a hash for bytes it never saw. So a declared hash
/// that disagrees with the document is a refusal here, **per document**: a connector whose five
/// documents share one hash could not say which of them moved. (Comparing against *upstream* is
/// different and is C-14's; this is the local claim against the local bytes.)
///
/// # The file decides whether any document is read at all
///
/// A provider with no `[spec]` block ignores the cache entirely. `specs/<provider>/` holding a file
/// is not a declaration; `[spec] path` is.
///
/// # Ingest selects nothing
///
/// A file that points at a 398-operation document and names none of them loads to a connector with
/// **no operations**. That is not a degenerate case to be worked around, it is the property that
/// keeps a vendor catalogue from becoming 398 LLM tools by default — see [`Patch`].
///
/// # Errors
///
/// The two [`load`] returns, plus an [`Error::InvalidProvider`](crate::Error::InvalidProvider)
/// naming the spec path when the pin resolves to nothing, when the declared hash disagrees with the
/// bytes, when the document is not an OpenAPI 3.x document at all, or when a patch selects an
/// operation the document does not declare. A document's *narrower* problems — one endpoint with an
/// unresolvable `$ref`, one parameter with no schema — are not errors: they arrive as
/// [`LoadedProvider::diagnostics`].
pub fn load_with_spec(
    name: &str,
    source: &str,
    documents: &[SpecDocument<'_>],
) -> crate::Result<LoadedProvider> {
    load_inner(name, source, Some(documents))
}

fn load_inner(
    name: &str,
    source: &str,
    documents: Option<&[SpecDocument<'_>]>,
) -> crate::Result<LoadedProvider> {
    let file: ProviderFile = match toml::from_str(source) {
        Ok(file) => file,
        // `deny_unknown_fields` has already reported `roles` as an unknown top-level key and listed
        // every key that *would* have been valid — which says the key is wrong without saying where
        // it belongs. This is the one key worth naming a destination for, because it is not wrong,
        // only one level too high. A well-formed `ProviderFile` can never carry it, so the extra
        // parse is paid on the error path alone.
        Err(parse) => {
            return Err(if declares_provider_roles(source) {
                crate::Error::InvalidProvider {
                    name: name.to_owned(),
                    problems: vec![PROVIDER_LEVEL_ROLES.to_owned()],
                }
            } else {
                crate::Error::ParseProvider {
                    name: name.to_owned(),
                    source: Box::new(parse),
                }
            });
        }
    };
    let implicit_service_members = implicit_service_members(source);

    // Kept before `assemble` distributes it, so a provider-level constant header is reported once
    // rather than once per operation that inherited it.
    let provider_headers = file.const_headers.clone();
    let mut loaded = assemble(file, source, implicit_service_members);

    // The ids the file writes out **inline**, captured before selection appends to them. C-6's
    // `validate_patch` asks whether a `rename` collides with one, and after selection every rename
    // is trivially present — so the question has to be asked of the set that existed first.
    let inline: Vec<String> = loaded
        .connector
        .operations
        .iter()
        .map(|operation| operation.id.clone())
        .collect();

    // **spec -> patch -> validate**, in that order, so a selected operation is validated by exactly
    // the pass a hand-authored one is rather than by a second, weaker one.
    let mut problems = Vec::new();
    if !loaded.specs.is_empty() {
        match documents {
            Some(documents) => {
                ingest_specs(&mut loaded, documents, &mut problems);
                // Re-run, because selection appended operations after `assemble` distributed. The
                // pass only fills a header an operation does not already carry, so a second run over
                // the inline ones changes nothing.
                distribute_const_headers(&provider_headers, &mut loaded.connector.operations);
            }
            // **No cache was supplied at all, so this file cannot be compiled here** — C-421. See
            // [`load`] for why this is a refusal rather than the skeleton it used to be.
            None => problems.push(no_spec_cache(&loaded.specs)),
        }
    }

    // A semantic-effect list is a set. Canonicalising it before validation makes equivalent input
    // hash and emit identically; duplicates remain adjacent so the validator can refuse them rather
    // than silently absorbing an authoring error.
    for operation in &mut loaded.connector.operations {
        operation.semantic_effects.sort_unstable();
    }

    problems.extend(validate(&loaded, source, &provider_headers, &inline));
    if !problems.is_empty() {
        return Err(crate::Error::InvalidProvider {
            name: name.to_owned(),
            problems,
        });
    }

    Ok(loaded)
}

/// The refusal [`load`] answers a spec-backed file with — C-421.
///
/// Written to be actionable from the message alone, because the reader is as likely to be an author
/// wondering why their connector is empty as a caller who picked the wrong function: it names every
/// document the file pins, so the cache to assemble is legible, and it names [`load_with_spec`], so
/// the fix is one identifier away.
fn no_spec_cache(specs: &[SpecSource]) -> String {
    let many = specs.len() > 1;
    let pinned: Vec<String> = specs
        .iter()
        .map(|spec| format!("{:?}", spec.path.trim()))
        .collect();

    format!(
        "`{}` pins {}, so this connector's operations are a function of {} as well as of this file \
         — and `provider::load` was given no spec cache to resolve {} against. Load a spec-backed \
         provider with `provider::load_with_spec`, handing it every document under \
         `specs/<provider>/`.",
        block(many),
        pinned.join(", "),
        if many {
            "those documents"
        } else {
            "that document"
        },
        if many { "them" } else { "it" },
    )
}

/// Ingest every vendored document the file pins and publish the operations the patch set selects.
///
/// Everything here is a *statement the author made*: which documents to compile, which operations of
/// each to publish, what to call each one, how risky it is. Nothing is inferred from a document,
/// because the three fields an `Operation` needs that a specification never carries — the op id,
/// [`Risk`] and [`Idempotency`] — are the three this repository refuses to decide by silence.
///
/// # Each document is ingested on its own — C-410
///
/// One [`IngestedDocument`] per `[[spec]]` entry, keyed by the service the entry names. Nothing is
/// merged into a single "the ingest", because merging is how one document's security model would
/// come to describe another's: babelforce's manager document declares root `oauth2` and **zero**
/// operation overrides, while `task-automation` declares `bearerAuth`+`oauth2` on all 31 of its
/// operations. Whichever was folded in last would have spoken for both.
fn ingest_specs(
    loaded: &mut LoadedProvider,
    documents: &[SpecDocument<'_>],
    problems: &mut Vec<String>,
) {
    let specs = loaded.specs.clone();
    let many = specs.len() > 1;
    let openapi_specs: Vec<SpecSource> = specs
        .iter()
        .filter(|spec| spec.kind == SpecKind::Openapi)
        .cloned()
        .collect();
    let asyncapi_specs: Vec<SpecSource> = specs
        .iter()
        .filter(|spec| spec.kind == SpecKind::Asyncapi)
        .cloned()
        .collect();

    // **The pin, resolved — once per entry.** `specs/<provider>/` ordinarily holds more files than a
    // connector compiles: versions of one document beside the documents of another service. Only a
    // `[[spec]] path` says which of them this connector is built from. Reading whichever happened to
    // sort last is precisely the defect `Provider::spec()` carried, and it compiled an operation out
    // of a document the provider file never named, successfully and silently.
    let mut ingested: Vec<IngestedDocument> = Vec::new();
    let mut ingested_events: Vec<IngestedEventDocument> = Vec::new();
    for spec in &specs {
        let path = spec.path.clone();
        let Some(found) = documents
            .iter()
            .find(|candidate| candidate.path == path.trim())
        else {
            problems.push(format!(
                "`{} path = {path:?}` names no vendored document. {}",
                block(many),
                describe_cache(documents)
            ));
            continue;
        };
        let document = found.document;

        // **Provenance is checked, not copied, and it is checked per document.** `sha256` travels
        // from here into `connectors.lock`; a value nothing compared against the ingested bytes
        // would be the file's claim about itself, recorded as though it were a measurement. One hash
        // for a five-document connector could not say *which* document moved, which is the only
        // question a drift check is asked. Checking against upstream is C-14's — this is the local
        // claim against the local bytes.
        if let Some(declared) = spec
            .sha256
            .as_deref()
            .map(str::trim)
            .filter(|hash| !hash.is_empty())
        {
            let measured = sha256_hex(document.as_bytes());
            if !declared.eq_ignore_ascii_case(&measured) {
                problems.push(format!(
                    "`{} sha256` declares {declared:?}, but {path} hashes to {measured:?}. The \
                     declared value reaches `connectors.lock`, so a build that ignored the \
                     difference would record a hash for bytes it never read — re-vendor the \
                     document or correct the declaration",
                    block(many)
                ));
                continue;
            }
        }

        match spec.kind {
            SpecKind::Openapi => match crate::openapi::ingest(document) {
                Ok(document) => ingested.push(IngestedDocument {
                    path,
                    service: spec.service().to_owned(),
                    ingested: document,
                }),
                Err(error) => problems.push(format!("`{} path = {path:?}`: {error}", block(many))),
            },
            SpecKind::Asyncapi => match crate::asyncapi::ingest(document) {
                Ok(document) => ingested_events.push(IngestedEventDocument {
                    path,
                    service: spec.service().to_owned(),
                    ingested: document,
                }),
                Err(error) => problems.push(format!("`{} path = {path:?}`: {error}", block(many))),
            },
        }
    }

    let (selected, operation_specs) = publish(
        &loaded.patch,
        &openapi_specs,
        &ingested,
        documents,
        &loaded.connector.config,
        problems,
    );
    loaded.connector.operations.extend(selected);
    let selected_events = publish_events(
        &loaded.patch.events,
        &asyncapi_specs,
        &ingested_events,
        problems,
    );
    loaded.connector.events.extend(selected_events);
    loaded
        .connector
        .provenance
        .operation_specs
        .extend(operation_specs);
    loaded.ingested = ingested;
    loaded.ingested_events = ingested_events;
}

/// Exact AsyncAPI message selection. Source payload schemas are authoritative; every admission
/// fact remains an explicit overlay.
fn publish_events(
    patches: &[EventPatch],
    specs: &[SpecSource],
    ingested: &[IngestedEventDocument],
    problems: &mut Vec<String>,
) -> Vec<EventDecl> {
    let mut published = Vec::new();
    let mut selected: Vec<(&str, &str)> = Vec::new();
    for patch in patches {
        let document = match patch.service.as_deref().map(str::trim) {
            Some(service) => ingested.iter().find(|document| document.service == service),
            None if specs.len() == 1 => ingested.first(),
            None => {
                problems.push(format!(
                    "`[[patch.events]] select = {:?}` states no `service`, but this connector \
                     declares {} AsyncAPI documents; name the event source explicitly",
                    patch.select,
                    specs.len()
                ));
                None
            }
        };
        let Some(document) = document else {
            if patch.service.is_some() {
                problems.push(format!(
                    "`[[patch.events]] select = {:?}` names service {:?}, which no AsyncAPI \
                     `[[spec]]` entry declares",
                    patch.select, patch.service
                ));
            }
            continue;
        };
        let key = (document.service.as_str(), patch.select.as_str());
        if selected.contains(&key) {
            problems.push(format!(
                "`[[patch.events]]` selects {:?} more than once from service {:?}",
                patch.select, document.service
            ));
            continue;
        }
        selected.push(key);
        let Some(source) = document.ingested.event(&patch.select) else {
            problems.push(format!(
                "`[[patch.events]] select = {:?}` names no component message in {}. Available: {}",
                patch.select,
                document.path,
                document
                    .ingested
                    .events
                    .iter()
                    .map(|event| event.message_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        };
        published.push(EventDecl {
            name: patch.rename.clone().unwrap_or_else(|| source.name.clone()),
            wire_value: patch.wire_value.clone(),
            service: document.service.clone(),
            auth: patch.auth.clone(),
            description: patch
                .description
                .clone()
                .unwrap_or_else(|| source.description.clone()),
            default: patch.default.unwrap_or(true),
            group: patch.group.clone().unwrap_or_default(),
            when: patch.when.clone(),
            schema: Some(source.payload.clone()),
        });
    }
    published
}

/// How to spell the block an author would go and edit — `[spec]` or `[[spec]]`.
///
/// A refusal that named the array form to someone who wrote a single table sends them looking for a
/// key they did not write; the two forms are one field, so the message follows the file.
pub(super) fn block(many: bool) -> &'static str {
    if many {
        "[[spec]]"
    } else {
        "[spec]"
    }
}

/// Which ingested document one `[[patch.operations]]` block reads from — C-410.
///
/// The rule is one sentence: **a patch names its document as soon as there is more than one.** With
/// a single `[[spec]]` entry the answer is that entry, which is what keeps every single-`[spec]`
/// file loading exactly as it did. With several, an unqualified `select` is refused rather than
/// resolved, because `getUser` is declared by babelforce's `manager-2026-07-10` *and* by its
/// `user-2026-06-25` as two different requests — and a rule that searched the documents in order
/// would compile one of them by accident, exit 0, and be invisible until someone called it.
pub(super) fn resolve_document<'a>(
    ingested: &'a [IngestedDocument],
    specs: &[SpecSource],
    service: Option<&str>,
    subject: &str,
    problems: &mut Vec<String>,
) -> Option<&'a IngestedDocument> {
    let Some(service) = service.map(str::trim) else {
        if specs.len() == 1 {
            // Present unless *that* document failed to resolve or ingest, which is already reported.
            return ingested.first();
        }
        problems.push(format!(
            "{subject} states no `service`, but this connector declares {} vendored documents \
             ({}). Two documents may declare one `operationId` — babelforce's `getUser` is in both \
             `manager` and `user` — so a `select` alone does not name an operation; state the \
             `service` whose document this patch reads",
            specs.len(),
            declared_services(specs)
        ));
        return None;
    };

    if let Some(document) = ingested.iter().find(|entry| entry.service == service) {
        return Some(document);
    }

    // A `service` that no `[[spec]]` entry names is a typo or a document that was removed from
    // under the patch. Silently selecting nothing is the rot `select` is already loud about.
    if !specs.iter().any(|spec| spec.service() == service) {
        problems.push(format!(
            "{subject} names service {service:?}, which no `[[spec]]` entry declares. The \
             documents this connector compiles are: {}",
            declared_services(specs)
        ));
    }
    None
}

/// The services the file's `[[spec]]` entries name, for a refusal that has to list them.
fn declared_services(specs: &[SpecSource]) -> String {
    specs
        .iter()
        .map(SpecSource::service)
        .collect::<Vec<_>>()
        .join(", ")
}
