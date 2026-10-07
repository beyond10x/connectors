//! A fixture binding of `datasource.feed/v1alpha1`, held to the `feed-binding` suite.
//!
//! The suite is synthesized in process from the shared specification and the authored
//! scenarios under `contracts/datasources/feed/v1alpha1/scenarios`, exactly as
//! `ess verify conform synthesize --component feed-binding --scenarios …` does.
//!
//! Three parts, kept apart so that the binding is what the suite observes:
//! - [`Source`] is the provider: what the specification's Provider commands seed.
//! - [`Source::containers`] and [`Source::items`] are the binding: `feed.containers` and
//!   `feed.items`, with an opaque watermark, paging and the direct-conversation exclusion.
//! - [`Fixture`] adapts the suite onto them. `ListedContainers` is read through
//!   `feed.containers`, and `ContainerItems` is what a consumer holds after reading again from
//!   the watermark it kept, one item per page, deduplicating by `(container, id, revision)`. A
//!   binding that loses a change on resume, or advances its watermark past one, leaves that view
//!   stale and the scenarios that read it fail. `SourceContainers` and `SourceItems` read the
//!   provider itself; they are the specification's witnesses, not family operations.
//!
//! The adapter holds any [`Binding`] over the same provider: [`Native`] is the binding above, and
//! `catalog_feed_conformance` holds the catalog engine's declared bindings to the same suite.
//!
//! A binding is held to what its profile declares ([`Capabilities`]), not to the strongest
//! provider. Before admission, since a Rust producer admits no skip, a scenario no binding
//! stating a weaker capability can be held to is left out, and in every scenario kept, each
//! expectation on the binding's own reads that a weaker capability speaks about is held to what
//! the profile states ([`Weaker::change`]). [`Run`] names each scenario left out and each
//! expectation changed with the capability that did it, beside the count of scenarios run and the
//! family's total.
use crate::metadata_entities;
use ess_conformance::{
    AdmittedSuite, CountReport, CountStatus, Runner,
    scenario::{CommandRef, ConformanceSuite, ErrorRef, EventRef, OutcomeRef},
    target::*,
};
use ess_domain::{command::OutcomeName, name::QualifiedName};
use ess_primitives::{consistency::ConsistencyToken, facts::Number, node::Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const COMPONENT: &str = "feed-binding";

/// The scenarios the family defines today: 22 synthesized and 6 authored. A run executes these
/// less the ones its profile's capabilities leave out, and names those; a run that executes
/// fewer is not running what the specification obliges.
pub(crate) const EXPECTED_SCENARIOS: u64 = 28;

const DOMAIN: &str = "connectors.feed";

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

// ---- the profile's capabilities --------------------------------------------------------------

/// Whether a removed item comes back as a tombstone (`connectors.feed.DeletionCapability`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Deletions {
    Observed,
    NotObserved,
}

/// Where a container's `kind` comes from (`connectors.feed.KindCapability`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    ProviderWord,
    FixedWord,
}

/// How a revision is formed (`connectors.feed.RevisionCapability`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Revision {
    Opaque,
    UpdateTime,
}

/// How a container's visibility is stated (`connectors.feed.VisibilityCapability`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Visibility {
    Mapped,
    AllPrivate,
}

/// What a binding's profile states its provider lets it observe
/// (`connectors.feed.ProfileCapabilities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Capabilities {
    pub(crate) deletions: Deletions,
    pub(crate) kind: Kind,
    /// The one word every listed container carries, stated exactly under `kind: fixed-word`. A
    /// `&'static str` keeps a claim `Copy`; a word read from a declaration is leaked, which a test
    /// harness affords.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind_word: Option<&'static str>,
    pub(crate) revision: Revision,
    pub(crate) visibility: Visibility,
}

/// A claim as read, its word owned.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stated {
    deletions: Deletions,
    kind: Kind,
    #[serde(default)]
    kind_word: Option<String>,
    revision: Revision,
    visibility: Visibility,
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let stated = Stated::deserialize(deserializer)?;
        Ok(Self {
            deletions: stated.deletions,
            kind: stated.kind,
            kind_word: stated
                .kind_word
                .map(|word| &*Box::leak(word.into_boxed_str())),
            revision: stated.revision,
            visibility: stated.visibility,
        })
    }
}

impl Capabilities {
    /// What the strongest provider gives: every scenario the family defines applies as written.
    pub(crate) const STRONGEST: Self = Self {
        deletions: Deletions::Observed,
        kind: Kind::ProviderWord,
        kind_word: None,
        revision: Revision::Opaque,
        visibility: Visibility::Mapped,
    };

    /// The capabilities this profile states below the strongest provider's. A `fixed-word` claim
    /// that states no word is not one: it is held to the provider's word.
    fn weaker(&self) -> Vec<Weaker> {
        [
            (self.deletions == Deletions::NotObserved).then_some(Weaker::DeletionsNotObserved),
            (self.kind == Kind::FixedWord && self.kind_word.is_some())
                .then_some(Weaker::KindFixedWord),
            (self.revision == Revision::UpdateTime).then_some(Weaker::RevisionUpdateTime),
            (self.visibility == Visibility::AllPrivate).then_some(Weaker::VisibilityAllPrivate),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// What this claim states that the model's invariants refuse, and how the suite reads it:
    /// always as the strongest provider's.
    pub(crate) fn refused(&self) -> Option<&'static str> {
        match (self.kind, self.kind_word) {
            (Kind::FixedWord, None) => {
                Some("claims `kind: fixed-word` and states no word: held to the provider's word")
            }
            (Kind::ProviderWord, Some(_)) => {
                Some("states a kind word without `kind: fixed-word`: held to the provider's word")
            }
            _ => None,
        }
    }
}

/// A capability below the strongest provider's, and what it makes inapplicable: a whole scenario,
/// or one expectation of what a scenario expects of the binding's own reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Weaker {
    DeletionsNotObserved,
    KindFixedWord,
    RevisionUpdateTime,
    VisibilityAllPrivate,
}

impl Weaker {
    /// The capability as a profile states it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::DeletionsNotObserved => "deletions: not-observed",
            Self::KindFixedWord => "kind: fixed-word",
            Self::RevisionUpdateTime => "revision: update-time",
            Self::VisibilityAllPrivate => "visibility: all-private",
        }
    }

    /// True when a binding stating this capability cannot be held to `scenario` at all, read from
    /// what the scenario seeds and expects, never from its id: under `revision: update-time`, one
    /// that reads an item after the source gave it two revisions at one instant, which an
    /// update-time revision cannot tell apart. Every other capability changes expectations
    /// ([`Weaker::change`]) and leaves no scenario out.
    fn leaves_out(self, scenario: &Value) -> bool {
        let steps = scenario["steps"].as_array().map_or(&[][..], Vec::as_slice);
        self == Self::RevisionUpdateTime
            && reads(steps, "ContainerItems")
            && Seeded::of(steps).reuses_an_instant()
    }

    /// The binding view and the field of it this capability speaks about.
    fn field(self) -> (&'static str, &'static str) {
        match self {
            Self::DeletionsNotObserved => ("ContainerItems", "state"),
            Self::KindFixedWord => ("ListedContainers", "kind"),
            Self::RevisionUpdateTime => ("ContainerItems", "revision"),
            Self::VisibilityAllPrivate => ("ListedContainers", "visibility"),
        }
    }

    /// Hold each expectation a kept scenario states about the binding's own reads to what this
    /// capability states, and record each change:
    /// - `deletions: not-observed`: an expected tombstone (`contains … state: Deleted`) becomes an
    ///   `excludes` of the deleted state for that item, since the binding never reports one; an
    ///   expectation that a removed item is no longer held present (`excludes … state: Present`)
    ///   cannot be held without observing the removal, and is masked whole;
    /// - `kind: fixed-word`: a listed container's `kind` is held to the stated word;
    /// - `revision: update-time`: a read item's revision word, in a `contains` or an `excludes`,
    ///   is held to the instant the scenario gave the item that revision;
    /// - `visibility: all-private`: a container stored `public` is held to being listed `private`.
    fn change(
        self,
        capabilities: &Capabilities,
        scenario_id: &str,
        scenario: &mut Value,
    ) -> Vec<Changed> {
        let (view, field) = self.field();
        let seeded = Seeded::of(scenario["steps"].as_array().map_or(&[][..], Vec::as_slice));
        let Some(steps) = scenario["steps"].as_array_mut() else {
            return Vec::new();
        };
        let qualified = format!("{DOMAIN}.{view}");
        let mut changed = Vec::new();
        steps.retain_mut(|step| {
            if step["step"] != "expect_view" || step["view"] != qualified.as_str() {
                return true;
            }
            let expect = step["expectation"]["expect"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let fields = &step["expectation"]["fields"];
            let Some(stated) = fields.get(field).and_then(literal) else {
                return true;
            };
            let Some(change) = self.held(capabilities, &seeded, &expect, fields, stated) else {
                return true;
            };
            let keep = match &change {
                Change::HeldTo(value) => {
                    step["expectation"]["fields"][field] =
                        serde_json::json!({"kind": "literal", "value": value});
                    true
                }
                Change::Excluded => {
                    if let Some(fields) = step["expectation"]["fields"].as_object_mut() {
                        fields.retain(|name, _| name == "item_id" || name == field);
                    }
                    step["expectation"]["expect"] = Value::from("excludes");
                    true
                }
                Change::Masked => false,
            };
            changed.push(Changed {
                scenario: scenario_id.to_owned(),
                view,
                field,
                expect,
                change,
                by: self,
            });
            keep
        });
        changed
    }

    /// What one expectation stating `stated` for this capability's field becomes, if anything.
    fn held(
        self,
        capabilities: &Capabilities,
        seeded: &Seeded,
        expect: &str,
        fields: &Value,
        stated: &Value,
    ) -> Option<Change> {
        match self {
            Self::DeletionsNotObserved => match (expect, stated.as_str()) {
                ("contains", Some("Deleted")) => Some(Change::Excluded),
                ("excludes", Some("Present")) => Some(Change::Masked),
                _ => None,
            },
            Self::KindFixedWord => {
                let word = capabilities.kind_word?;
                match expect {
                    "contains" => Some(Change::HeldTo(word.to_owned())),
                    // Every listed container carries the word; excluding a stored one says nothing.
                    "excludes" => Some(Change::Masked),
                    _ => None,
                }
            }
            Self::RevisionUpdateTime => seeded
                .instant_of(&fields["item_id"], stated)
                .map(Change::HeldTo),
            Self::VisibilityAllPrivate => (expect == "contains" && stated == "public")
                .then(|| Change::HeldTo("private".to_owned())),
        }
    }
}

/// How a capability changed one expectation on a binding view.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Change {
    /// The field is held to this value instead of the stored one.
    HeldTo(String),
    /// A `contains` of the field's value is now an `excludes` of it for the same item.
    Excluded,
    /// The expectation cannot be held under the capability and is masked whole.
    Masked,
}

/// One expectation of a kept scenario on a binding view, changed by a capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Changed {
    pub(crate) scenario: String,
    pub(crate) view: &'static str,
    pub(crate) field: &'static str,
    /// The expectation as the scenario states it: `contains` or `excludes`.
    pub(crate) expect: String,
    pub(crate) change: Change,
    pub(crate) by: Weaker,
}

/// True when the scenario reads one of the family's views.
fn reads(steps: &[Value], view: &str) -> bool {
    let view = format!("{DOMAIN}.{view}");
    steps
        .iter()
        .any(|step| step["step"] == "query_view" && step["view"] == view.as_str())
}

/// A scenario value's literal, if it is one.
fn literal(value: &Value) -> Option<&Value> {
    (value["kind"] == "literal").then(|| &value["value"])
}

/// Every revision the source gives each item in one scenario, and the instant it gives it at: an
/// item added, revised, removed or restored. Only changes the scenario expects to take effect
/// count; a refused change moves nothing.
struct Seeded {
    /// An instance an earlier step bound, and the value it stands for: the item id the command
    /// that bound it took.
    instances: BTreeMap<String, Value>,
    /// `(item, revision)` and the instant the item took that revision at.
    at: BTreeMap<(String, String), String>,
    /// `(item, instant)` and every revision the item took at that instant.
    revisions: BTreeMap<(String, String), BTreeSet<String>>,
}

impl Seeded {
    fn of(steps: &[Value]) -> Self {
        let mut seeded = Self {
            instances: BTreeMap::new(),
            at: BTreeMap::new(),
            revisions: BTreeMap::new(),
        };
        let mut input: Option<&Value> = None;
        let mut change: Option<(Value, Value, Value)> = None;
        for step in steps {
            match step["step"].as_str() {
                Some("execute_command") => {
                    input = Some(&step["input"]);
                    let at = match step["command"].as_str() {
                        Some("connectors.feed.AddItem") => "created_at",
                        Some("connectors.feed.ReviseItem" | "connectors.feed.RestoreItem") => {
                            "updated_at"
                        }
                        Some("connectors.feed.RemoveItem") => "removed_at",
                        _ => {
                            change = None;
                            continue;
                        }
                    };
                    let input = &step["input"];
                    change = Some((
                        seeded.resolve(&input["item_id"]),
                        seeded.resolve(&input[at]),
                        seeded.resolve(&input["revision"]),
                    ));
                }
                Some("expect_outcome") => {
                    let took = matches!(
                        step["outcome"]["outcome"].as_str(),
                        Some("added" | "revised" | "removed" | "restored")
                    );
                    if let (true, Some((item, at, revision))) = (took, change.take()) {
                        let instant = at.as_str().map_or_else(|| at.to_string(), str::to_owned);
                        seeded
                            .at
                            .insert((item.to_string(), revision.to_string()), instant);
                        seeded
                            .revisions
                            .entry((item.to_string(), at.to_string()))
                            .or_default()
                            .insert(revision.to_string());
                    }
                }
                Some("capture_instance") => {
                    if let (Some(instance), Some(field), Some(input)) =
                        (step["instance"].as_str(), step["field"].as_str(), input)
                    {
                        let bound = seeded.resolve(&input[field]);
                        seeded.instances.insert(instance.to_owned(), bound);
                    }
                }
                _ => {}
            }
        }
        seeded
    }

    /// A scenario value as the value it stands for.
    fn resolve(&self, value: &Value) -> Value {
        match (literal(value), value["instance"].as_str()) {
            (Some(literal), _) => literal.clone(),
            (None, Some(instance)) => self
                .instances
                .get(instance)
                .cloned()
                .unwrap_or_else(|| Value::from(instance)),
            (None, None) => value.clone(),
        }
    }

    /// True when the source gives one item two different revisions at one instant.
    fn reuses_an_instant(&self) -> bool {
        self.revisions.values().any(|revisions| revisions.len() > 1)
    }

    /// The instant the scenario gave `item` the revision `revision`.
    fn instant_of(&self, item: &Value, revision: &Value) -> Option<String> {
        self.at
            .get(&(self.resolve(item).to_string(), revision.to_string()))
            .cloned()
    }
}

/// A scenario of the family a binding's suite leaves out, and the capabilities that left it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LeftOut {
    pub(crate) scenario: String,
    pub(crate) by: Vec<Weaker>,
}

/// The family's suite, every scenario it defines, before admission.
fn family(root: &Path) -> ConformanceSuite {
    let ir = metadata_entities::compile(&metadata_entities::load(root).expect("load ess"))
        .expect("compile ess");
    let synthesis =
        ess_conformance::synthesize::synthesize_for(&ir, COMPONENT).expect("feed-binding");
    let feed_refusals: Vec<String> = synthesis
        .refusals
        .iter()
        .map(ToString::to_string)
        .filter(|refusal| refusal.contains("connectors.feed."))
        .collect();
    assert!(feed_refusals.is_empty(), "{feed_refusals:#?}");
    let directory = root.join("contracts/datasources/feed/v1alpha1/scenarios");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("scenarios")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    paths.sort();
    let sources: Vec<_> = paths
        .iter()
        .map(|path| {
            ess_conformance::authored::Source::new(
                path.strip_prefix(root).unwrap().display().to_string(),
                std::fs::read_to_string(path).expect("scenario"),
            )
        })
        .collect();
    let authoring = ess_conformance::authored::compile(&ir, &sources);
    let refusals: Vec<String> = authoring.refusals.iter().map(ToString::to_string).collect();
    assert!(refusals.is_empty(), "{refusals:#?}");
    let mut suite = synthesis.suite;
    for (id, scenario) in authoring.scenarios {
        suite.insert(id, scenario).expect("authored id is new");
    }
    suite.select_fresh_format_for(&ir);
    suite
}

/// The suite a binding stating some capabilities is held to.
pub(crate) struct Selected {
    pub(crate) admitted: AdmittedSuite,
    /// The family's scenarios left out, each with the capabilities that left it out.
    pub(crate) left_out: Vec<LeftOut>,
    /// Every expectation of a kept scenario a capability changed.
    pub(crate) changed: Vec<Changed>,
}

/// The suite a binding stating `capabilities` is held to: the family's scenarios less the ones a
/// capability leaves out, with each expectation a capability speaks about held to what it states.
pub(crate) fn suite(root: &Path, capabilities: &Capabilities) -> Selected {
    let mut suite = family(root);
    let weaker = capabilities.weaker();
    let mut left_out = Vec::new();
    let mut changed = Vec::new();
    suite.scenarios.retain(|id, scenario| {
        let id = id.to_string();
        let mut value = serde_json::to_value(&*scenario).expect("scenario");
        let by: Vec<Weaker> = weaker
            .iter()
            .copied()
            .filter(|capability| capability.leaves_out(&value))
            .collect();
        if !by.is_empty() {
            left_out.push(LeftOut { scenario: id, by });
            return false;
        }
        let here: Vec<Changed> = weaker
            .iter()
            .flat_map(|capability| capability.change(capabilities, &id, &mut value))
            .collect();
        if !here.is_empty() {
            *scenario = serde_json::from_value(value).expect("a changed scenario is a scenario");
            changed.extend(here);
        }
        true
    });
    changed.sort();
    changed.dedup();
    Selected {
        admitted: AdmittedSuite::from_suite(&suite).expect("admitted"),
        left_out,
        changed,
    }
}

// ---- the provider ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct Container {
    /// The connection this container is read through; it references the connection.
    pub(crate) connection: String,
    pub(crate) name: Option<String>,
    pub(crate) kind: String,
    pub(crate) visibility: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Item {
    pub(crate) container: String,
    pub(crate) revision: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) parent: Option<String>,
    pub(crate) deleted: bool,
    /// The provider's change sequence at this item's last change.
    pub(crate) changed: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Source {
    pub(crate) containers: BTreeMap<String, Container>,
    pub(crate) items: BTreeMap<String, Item>,
    pub(crate) sequence: u64,
}

// ---- the binding -----------------------------------------------------------------------------

/// The service contract codes a binding answers with; any other is carried by name.
#[derive(Debug, PartialEq)]
pub(crate) enum Code {
    InvalidInput,
    NotFound,
    StaleCursor,
    Other(String),
}

pub(crate) struct ContainersPage {
    pub(crate) containers: Vec<(String, Container)>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) complete: bool,
}

pub(crate) struct ItemsPage {
    pub(crate) items: Vec<(String, Item)>,
    pub(crate) next_watermark: String,
    pub(crate) complete: bool,
}

/// `feed.containers` and `feed.items` of one binding, reading the provider as it stands.
pub(crate) trait Binding {
    /// The implementation name and the native profile the suite reports.
    fn identity(&self) -> (String, String);
    /// What the profile states; the suite holds the binding to it.
    fn capabilities(&self) -> Capabilities;
    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code>;
    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code>;
}

/// The binding written beside the suite, in [`Source::containers`] and [`Source::items`].
pub(crate) struct Native;

impl Binding for Native {
    fn identity(&self) -> (String, String) {
        ("connectors-feed-fixture".into(), WATERMARK.into())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::STRONGEST
    }
    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code> {
        source.containers(limit, cursor)
    }
    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        source.items(container, watermark, limit)
    }
}

/// The fixture profile's watermark: `fixture-feed/1:<container>:<sequence>`. Opaque to a consumer.
const WATERMARK: &str = "fixture-feed/1";
const MAX_LIMIT: i64 = 100;

impl Source {
    /// `feed.containers`: every container except direct conversations, which this profile does
    /// not declare. Recognised by the provider's own visibility field.
    fn containers(&self, limit: Option<i64>, cursor: Option<&str>) -> Result<ContainersPage, Code> {
        let limit = limit.unwrap_or(MAX_LIMIT);
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(Code::InvalidInput);
        }
        let listed: Vec<_> = self
            .containers
            .iter()
            .filter(|(_, container)| container.visibility != "direct")
            .collect();
        let start = match cursor {
            None => 0,
            Some(cursor) => cursor
                .strip_prefix("c")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n <= listed.len())
                .ok_or(Code::StaleCursor)?,
        };
        let containers: Vec<_> = listed
            .iter()
            .skip(start)
            .take(limit as usize)
            .map(|(id, container)| ((*id).clone(), (*container).clone()))
            .collect();
        let end = start + containers.len();
        let complete = end >= listed.len();
        Ok(ContainersPage {
            containers,
            next_cursor: (!complete).then(|| format!("c{end}")),
            complete,
        })
    }

    /// `feed.items`: the container's items changed after the watermark, oldest change first,
    /// at most `limit`. The next watermark never passes an item this page did not return.
    fn items(
        &self,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(Code::InvalidInput);
        }
        match self.containers.get(container) {
            Some(found) if found.visibility != "direct" => {}
            _ => return Err(Code::NotFound),
        }
        let after = match watermark {
            None => 0,
            Some(watermark) => self.position(container, watermark)?,
        };
        let mut changed: Vec<(String, Item)> = self
            .items
            .iter()
            .filter(|(_, item)| item.container == container && item.changed > after)
            .map(|(id, item)| (id.clone(), item.clone()))
            .collect();
        changed.sort_by_key(|(_, item)| item.changed);
        let complete = changed.len() <= limit as usize;
        changed.truncate(limit as usize);
        let next = changed.last().map_or(after, |(_, item)| item.changed);
        Ok(ItemsPage {
            items: changed,
            next_watermark: format!("{WATERMARK}:{container}:{next}"),
            complete,
        })
    }

    /// A watermark this binding issued for this container; anything else is `stale_cursor`.
    fn position(&self, container: &str, watermark: &str) -> Result<u64, Code> {
        let rest = watermark
            .strip_prefix(WATERMARK)
            .and_then(|rest| rest.strip_prefix(':'))
            .ok_or(Code::StaleCursor)?;
        let (issued_for, position) = rest.rsplit_once(':').ok_or(Code::StaleCursor)?;
        if issued_for != container {
            return Err(Code::StaleCursor);
        }
        position
            .parse::<u64>()
            .ok()
            .filter(|position| *position <= self.sequence)
            .ok_or(Code::StaleCursor)
    }
}

// ---- the consumer ----------------------------------------------------------------------------

/// What a consumer keeps for one container: its watermark and the newest revision of each item.
#[derive(Clone, Debug, Default)]
struct Held {
    watermark: Option<String>,
    items: BTreeMap<String, Item>,
}

impl Held {
    fn take(&mut self, page: ItemsPage) {
        for (id, item) in page.items {
            // Deduplicate by (container, id, revision): a repeat replaces with the same value,
            // a change replaces with the newer one, since pages deliver oldest change first.
            self.items.insert(id, item);
        }
        self.watermark = Some(page.next_watermark);
    }

    /// Read again from the kept watermark until the binding says it is complete. A binding that
    /// never says so within one page per item the source holds, and a few more, is not resuming.
    fn caught_up(
        &self,
        binding: &dyn Binding,
        source: &Source,
        container: &str,
    ) -> Result<Held, Code> {
        let mut held = self.clone();
        for _ in 0..source.items.len() + 3 {
            let page = binding.items(source, container, held.watermark.as_deref(), 1)?;
            let complete = page.complete;
            held.take(page);
            if complete {
                return Ok(held);
            }
        }
        Err(Code::Other("reading again never completes".into()))
    }
}

// ---- the suite's adapter ---------------------------------------------------------------------

#[derive(Default)]
struct Scenario {
    source: Source,
    consumers: BTreeMap<String, Held>,
    events: Vec<ObservedEvent>,
    writes: u64,
}

pub(crate) struct Fixture<B> {
    binding: B,
    scenario: RefCell<Option<Scenario>>,
}

impl<B: Binding> Fixture<B> {
    pub(crate) fn new(binding: B) -> Self {
        Self {
            binding,
            scenario: RefCell::default(),
        }
    }
}

/// One binding's run: what it was held to, what it was not, and how it ended.
pub(crate) struct Run {
    pub(crate) profile: String,
    /// What the binding's profile states.
    pub(crate) capabilities: Capabilities,
    pub(crate) counts: ess_conformance::counts::ScenarioCounts,
    pub(crate) status: CountStatus,
    /// The scenarios that did not pass.
    pub(crate) failed: BTreeSet<String>,
    /// The family's scenarios this profile's capabilities left out, named.
    pub(crate) left_out: Vec<LeftOut>,
    /// The expectations of kept scenarios this profile's capabilities changed.
    pub(crate) changed: Vec<Changed>,
    /// The runner's report, for a failure message.
    pub(crate) report: String,
}

impl Run {
    /// The scenarios run beside the family's total, then every one left out and every
    /// expectation changed, each with the capability that did it.
    pub(crate) fn summary(&self) -> String {
        let mut summary = format!(
            "{}: ran {} of the {EXPECTED_SCENARIOS} scenarios the family defines ({} passed)",
            self.profile, self.counts.total, self.counts.passed
        );
        if let Some(refused) = self.capabilities.refused() {
            summary.push_str(&format!("\n  {refused}"));
        }
        for left in &self.left_out {
            let by: Vec<&str> = left.by.iter().map(|capability| capability.name()).collect();
            summary.push_str(&format!(
                "\n  left out {} ({})",
                left.scenario,
                by.join(", ")
            ));
        }
        for changed in &self.changed {
            let what = format!("{}.{} ({})", changed.view, changed.field, changed.expect);
            let change = match &changed.change {
                Change::HeldTo(value) => format!("held {what} to `{value}`"),
                Change::Excluded => format!("turned {what} into an excludes of it"),
                Change::Masked => format!("masked {what}"),
            };
            summary.push_str(&format!(
                "\n  {change} in {} ({})",
                changed.scenario,
                changed.by.name()
            ));
        }
        summary
    }

    /// The scenarios left out, by id.
    pub(crate) fn left_out_ids(&self) -> BTreeSet<&str> {
        self.left_out
            .iter()
            .map(|left| left.scenario.as_str())
            .collect()
    }

    /// The kept scenarios with an expectation a capability changed, by id.
    pub(crate) fn changed_ids(&self) -> BTreeSet<&str> {
        self.changed
            .iter()
            .map(|changed| changed.scenario.as_str())
            .collect()
    }

    /// Every scenario the family defines is either run or left out and named.
    pub(crate) fn accounts_for_the_family(&self) -> bool {
        self.counts.total + self.left_out.len() as u64 == EXPECTED_SCENARIOS
    }
}

/// Run the suite a binding's capabilities select against it.
pub(crate) fn run<B: Binding>(binding: B) -> Run {
    let (_, profile) = binding.identity();
    let capabilities = binding.capabilities();
    let Selected {
        admitted,
        left_out,
        changed,
    } = suite(&root(), &capabilities);
    let fixture = Fixture::new(binding);
    let executed = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &fixture);
    let report = CountReport::from_run(&executed, &admitted).expect("report");
    let failed = report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(id, _)| id)
        .collect();
    Run {
        profile,
        capabilities,
        counts: report.counts().clone(),
        status: report.execution_status(),
        failed,
        left_out,
        changed,
        report: executed.report().to_string(),
    }
}

fn unsupported(what: impl Into<String>, why: impl Into<String>) -> TargetError {
    TargetError::unsupported(what, why)
}

fn qualified(local: &str) -> Result<QualifiedName, TargetError> {
    let name = format!("{DOMAIN}.{local}");
    QualifiedName::new(&name).map_err(|error| unsupported(name, format!("{error:?}")))
}

fn outcome(command: &str, outcome: &str) -> Result<SemanticCommandResult, TargetError> {
    Ok(SemanticCommandResult::took(OutcomeRef::new(
        CommandRef::new(qualified(command)?),
        OutcomeName::new(outcome).map_err(|error| unsupported(outcome, format!("{error:?}")))?,
    )))
}

fn error(local: &str, field: &str, value: Node) -> Result<DeclaredErrorValue, TargetError> {
    Ok(DeclaredErrorValue::new(ErrorRef::new(qualified(local)?)).with(field, value))
}

fn text(input: &BTreeMap<String, Node>, field: &str) -> Option<String> {
    input.get(field).and_then(Node::as_text).map(str::to_owned)
}

fn required(input: &BTreeMap<String, Node>, field: &str) -> Result<String, TargetError> {
    text(input, field).ok_or_else(|| unsupported(field, "missing text input"))
}

fn integer(input: &BTreeMap<String, Node>, field: &str) -> Result<Option<i64>, TargetError> {
    match input.get(field) {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Number(number)) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| unsupported(field, "not an integer")),
        Some(_) => Err(unsupported(field, "not a number")),
    }
}

fn optional(value: &Option<String>) -> Node {
    value.clone().map_or(Node::Null, Node::Text)
}

fn token(writes: u64) -> Result<ConsistencyToken, TargetError> {
    ConsistencyToken::new(format!("w{writes}"))
        .map_err(|error| unsupported("consistency", format!("{error:?}")))
}

impl<B: Binding> Fixture<B> {
    fn execute(
        &self,
        scenario: &mut Scenario,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let local = command
            .strip_prefix(&format!("{DOMAIN}."))
            .ok_or_else(|| unsupported(&command, "not a feed command"))?;
        let input = &request.input;
        let event = |name: &str, fields: Vec<(&str, Node)>| -> Result<ObservedEvent, TargetError> {
            let mut observed = ObservedEvent::new(EventRef::new(qualified(name)?));
            for (field, value) in fields {
                observed = observed.with(field, value);
            }
            Ok(observed.in_activity(request.correlation.clone()))
        };
        match local {
            "AddContainer" => {
                let id = required(input, "container_id")?;
                if scenario.source.containers.contains_key(&id) {
                    return Err(unsupported(
                        &command,
                        "the source already holds this container",
                    ));
                }
                let visibility = required(input, "visibility")?;
                scenario.source.containers.insert(
                    id.clone(),
                    Container {
                        connection: required(input, "connection_ref")?,
                        name: text(input, "name"),
                        kind: required(input, "kind")?,
                        visibility: visibility.clone(),
                    },
                );
                scenario.writes += 1;
                let added = event(
                    "ContainerAdded",
                    vec![
                        ("container_id", Node::Text(id)),
                        ("visibility", Node::Text(visibility)),
                    ],
                )?;
                scenario.events.push(added.clone());
                Ok(outcome(local, "added")?.emitting(added))
            }
            "AddItem" => {
                let id = required(input, "item_id")?;
                if scenario.source.items.contains_key(&id) {
                    return Err(unsupported(&command, "the source already holds this item"));
                }
                let container = required(input, "container_id")?;
                if !scenario.source.containers.contains_key(&container) {
                    return Ok(outcome(local, "no-such-container")?.with_error(error(
                        "ContainerNotFound",
                        "container_id",
                        Node::Text(container),
                    )?));
                }
                let revision = required(input, "revision")?;
                let created_at = required(input, "created_at")?;
                scenario.source.sequence += 1;
                scenario.source.items.insert(
                    id.clone(),
                    Item {
                        container: container.clone(),
                        revision: revision.clone(),
                        created_at: created_at.clone(),
                        updated_at: created_at,
                        parent: text(input, "parent"),
                        deleted: false,
                        changed: scenario.source.sequence,
                    },
                );
                scenario.writes += 1;
                let added = event(
                    "ItemAdded",
                    vec![
                        ("item_id", Node::Text(id)),
                        ("container_id", Node::Text(container)),
                        ("revision", Node::Text(revision)),
                    ],
                )?;
                scenario.events.push(added.clone());
                Ok(outcome(local, "added")?.emitting(added))
            }
            "ReviseItem" | "RemoveItem" | "RestoreItem" => {
                let (from_deleted, to_deleted, at, done, emitted) = match local {
                    "ReviseItem" => (false, false, "updated_at", "revised", "ItemRevised"),
                    "RemoveItem" => (false, true, "removed_at", "removed", "ItemRemoved"),
                    _ => (true, false, "updated_at", "restored", "ItemRestored"),
                };
                let id = required(input, "item_id")?;
                let revision = required(input, "revision")?;
                let Some(item) = scenario.source.items.get_mut(&id) else {
                    return Ok(outcome(local, "no-such-item")?.with_error(error(
                        "ItemNotFound",
                        "item_id",
                        Node::Text(id),
                    )?));
                };
                if item.revision == revision {
                    return Ok(outcome(local, "same-revision")?.with_error(error(
                        "RevisionUnchanged",
                        "revision",
                        Node::Text(revision),
                    )?));
                }
                if item.deleted != from_deleted {
                    let state = if item.deleted { "Deleted" } else { "Present" };
                    return Ok(outcome(local, "wrong-state")?.with_error(error(
                        "ItemStateConflict",
                        "state",
                        Node::Text(state.to_owned()),
                    )?));
                }
                scenario.source.sequence += 1;
                item.revision = revision.clone();
                item.updated_at = required(input, at)?;
                item.deleted = to_deleted;
                item.changed = scenario.source.sequence;
                scenario.writes += 1;
                let changed = event(
                    emitted,
                    vec![
                        ("item_id", Node::Text(id)),
                        ("revision", Node::Text(revision)),
                    ],
                )?;
                scenario.events.push(changed.clone());
                Ok(outcome(local, done)?.emitting(changed))
            }
            "ListContainers" => {
                let limit = integer(input, "limit")?;
                // The binding refuses a limit it cannot honour; the suite's name for that
                // refusal of a non-positive limit is `bad-limit`.
                let bad_limit = |limit: i64| -> Result<SemanticCommandResult, TargetError> {
                    Ok(outcome(local, "bad-limit")?.with_error(error(
                        "InvalidLimit",
                        "limit",
                        Node::Number(Number::from(limit)),
                    )?))
                };
                let mut cursor = text(input, "cursor");
                loop {
                    let page =
                        match self
                            .binding
                            .containers(&scenario.source, limit, cursor.as_deref())
                        {
                            Ok(page) => page,
                            Err(Code::InvalidInput) if limit.is_some_and(|limit| limit <= 0) => {
                                return bad_limit(limit.unwrap_or_default());
                            }
                            Err(code) => return Err(unsupported(&command, format!("{code:?}"))),
                        };
                    if page.complete {
                        break;
                    }
                    cursor = page.next_cursor;
                }
                outcome(local, "listed")
            }
            "ReadItems" => {
                let limit =
                    integer(input, "limit")?.ok_or_else(|| unsupported("limit", "missing"))?;
                let container = required(input, "container_id")?;
                let held = scenario.consumers.entry(container.clone()).or_default();
                // A scenario cannot carry the watermark a read returned into the next read, so a
                // watermark in its input stands for the one this consumer kept. A consumer that
                // kept none reads from the start.
                let watermark = match text(input, "watermark") {
                    Some(_) => held.watermark.clone(),
                    None => None,
                };
                match self
                    .binding
                    .items(&scenario.source, &container, watermark.as_deref(), limit)
                {
                    Ok(page) => {
                        held.take(page);
                        outcome(local, "read")
                    }
                    Err(Code::InvalidInput) if limit <= 0 => Ok(outcome(local, "bad-limit")?
                        .with_error(error(
                            "InvalidLimit",
                            "limit",
                            Node::Number(Number::from(limit)),
                        )?)),
                    Err(Code::NotFound) => {
                        let not_found = error(
                            "ContainerNotFound",
                            "container_id",
                            Node::Text(container.clone()),
                        )?;
                        if scenario.source.containers.contains_key(&container) {
                            Ok(outcome(local, "direct-conversation")?.with_error(not_found))
                        } else {
                            // The specification declares no answer for an unknown container
                            // (ESS-LIMIT in ess/domains/feed.yaml); the binding's is `not_found`.
                            Ok(SemanticCommandResult::undeclared().with_error(not_found))
                        }
                    }
                    Err(code) => Err(unsupported(&command, format!("{code:?}"))),
                }
            }
            _ => Err(unsupported(&command, "not bound")),
        }
    }

    fn rows(
        &self,
        scenario: &Scenario,
        request: &SemanticViewRequest,
    ) -> Result<Vec<ViewRow>, TargetError> {
        let view = request.view.to_string();
        let local = view
            .strip_prefix(&format!("{DOMAIN}."))
            .ok_or_else(|| unsupported(&view, "not a feed view"))?;
        let container_row = |id: &str, container: &Container| -> ViewRow {
            BTreeMap::from([
                ("container_id".to_owned(), Node::Text(id.to_owned())),
                ("name".to_owned(), optional(&container.name)),
                ("kind".to_owned(), Node::Text(container.kind.clone())),
                (
                    "visibility".to_owned(),
                    Node::Text(container.visibility.clone()),
                ),
            ])
        };
        let item_row = |id: &str, item: &Item| -> ViewRow {
            let state = if item.deleted { "Deleted" } else { "Present" };
            BTreeMap::from([
                ("item_id".to_owned(), Node::Text(id.to_owned())),
                (
                    "container_id".to_owned(),
                    Node::Text(item.container.clone()),
                ),
                ("revision".to_owned(), Node::Text(item.revision.clone())),
                ("created_at".to_owned(), Node::Text(item.created_at.clone())),
                ("updated_at".to_owned(), Node::Text(item.updated_at.clone())),
                ("parent".to_owned(), optional(&item.parent)),
                ("state".to_owned(), Node::Text(state.to_owned())),
            ])
        };
        match local {
            "ListedContainers" => {
                let mut rows = Vec::new();
                let mut cursor = None;
                loop {
                    let page = self
                        .binding
                        .containers(&scenario.source, Some(1), cursor.as_deref())
                        .map_err(|code| unsupported(&view, format!("{code:?}")))?;
                    rows.extend(page.containers.iter().map(|(id, c)| container_row(id, c)));
                    if page.complete {
                        return Ok(rows);
                    }
                    cursor = page.next_cursor;
                }
            }
            "SourceContainers" => Ok(scenario
                .source
                .containers
                .iter()
                .map(|(id, container)| {
                    let mut row = container_row(id, container);
                    row.insert(
                        "connection_ref".to_owned(),
                        Node::Text(container.connection.clone()),
                    );
                    row.insert("state".to_owned(), Node::Text("Present".to_owned()));
                    row
                })
                .collect()),
            "SourceItems" => Ok(scenario
                .source
                .items
                .iter()
                .map(|(id, item)| item_row(id, item))
                .collect()),
            "ContainerItems" => {
                let container = request
                    .params
                    .get("container_id")
                    .and_then(Node::as_text)
                    .ok_or_else(|| unsupported(&view, "container_id parameter"))?;
                let held = scenario
                    .consumers
                    .get(container)
                    .cloned()
                    .unwrap_or_default();
                match held.caught_up(&self.binding, &scenario.source, container) {
                    Ok(held) => Ok(held
                        .items
                        .iter()
                        .map(|(id, item)| {
                            let mut row = item_row(id, item);
                            row.retain(|field, _| {
                                matches!(
                                    field.as_str(),
                                    "item_id"
                                        | "container_id"
                                        | "revision"
                                        | "updated_at"
                                        | "state"
                                )
                            });
                            row
                        })
                        .collect()),
                    // Unknown or an undeclared direct conversation: the binding discloses nothing.
                    Err(Code::NotFound) => Ok(Vec::new()),
                    Err(code) => Err(unsupported(&view, format!("{code:?}"))),
                }
            }
            _ => Err(unsupported(&view, "no such view")),
        }
    }
}

impl<B: Binding> ConformanceTarget for Fixture<B> {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        let (implementation, profile) = self.binding.identity();
        Ok(ImplementationIdentity::new(implementation, profile))
    }

    fn begin_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = Some(Scenario::default());
        Ok(())
    }

    fn end_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = None;
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unsupported("command", "no scenario is open"))?;
        let result = self.execute(scenario, &request)?;
        // Every effect applies synchronously; the token names the write level after it.
        Ok(result.with_consistency(token(scenario.writes)?))
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unsupported("view", "no scenario is open"))?;
        if let Some(token) = request.consistency.token() {
            let written = token
                .as_str()
                .strip_prefix('w')
                .and_then(|n| n.parse::<u64>().ok())
                .ok_or_else(|| unsupported("view", "foreign consistency token"))?;
            if written > scenario.writes {
                return Err(unsupported(
                    "view",
                    "token from a write this scenario never made",
                ));
            }
        }
        Ok(SemanticViewResult::of(self.rows(scenario, &request)?))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unsupported("events", "no scenario is open"))?;
        Ok(scenario
            .events
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(&self, _: ExternalOutcomeControl) -> Result<(), TargetError> {
        Err(unsupported("external", "the feed family declares none"))
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(unsupported(request.event.to_string(), "no event bindings"))
    }
}

#[test]
fn fixture_binding_passes_the_feed_binding_suite() {
    let run = run(Native);
    println!("{}", run.summary());
    println!("{}", serde_json::to_string(&run.counts).unwrap());
    assert!(run.left_out.is_empty(), "{}", run.summary());
    assert_eq!(run.counts.total, EXPECTED_SCENARIOS, "scenario count moved");
    assert_eq!(run.status, CountStatus::Passed, "{}", run.report);
    assert_eq!(run.counts.passed, run.counts.total, "{}", run.report);
}

// ---- profiles weaker than the strongest provider ---------------------------------------------

/// The native binding over a provider that offers less than the strongest, stating `declares`.
struct Shaped {
    declares: Capabilities,
    /// The provider deletes items outright: a removed item is no longer listed at all.
    deletes_outright: bool,
    /// Every listed container is `private`.
    lists_all_private: bool,
    /// The binding cannot tell a direct conversation from a private container, so it lists one.
    lists_direct_conversations: bool,
    /// Every listed container carries this word as its `kind`.
    kind_word: Option<&'static str>,
    /// The provider gives no version: an item's revision is its `updated_at`.
    revision_is_update_time: bool,
}

impl Shaped {
    fn declaring(declares: Capabilities) -> Self {
        Self {
            declares,
            deletes_outright: false,
            lists_all_private: false,
            lists_direct_conversations: false,
            kind_word: None,
            revision_is_update_time: false,
        }
    }
}

impl Binding for Shaped {
    fn identity(&self) -> (String, String) {
        ("connectors-feed-fixture-shaped".into(), WATERMARK.into())
    }
    fn capabilities(&self) -> Capabilities {
        self.declares
    }
    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code> {
        let mut page = if self.lists_direct_conversations {
            let mut unclassified = source.clone();
            for container in unclassified.containers.values_mut() {
                if container.visibility == "direct" {
                    container.visibility = "private".into();
                }
            }
            unclassified.containers(limit, cursor)?
        } else {
            source.containers(limit, cursor)?
        };
        for (_, container) in &mut page.containers {
            if self.lists_all_private {
                container.visibility = "private".into();
            }
            if let Some(word) = self.kind_word {
                container.kind = word.into();
            }
        }
        Ok(page)
    }
    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        let mut page = if self.deletes_outright {
            let mut listed = source.clone();
            listed.items.retain(|_, item| !item.deleted);
            listed.items(container, watermark, limit)?
        } else {
            source.items(container, watermark, limit)?
        };
        if self.revision_is_update_time {
            for (_, item) in &mut page.items {
                item.revision = item.updated_at.clone();
            }
        }
        Ok(page)
    }
}

const TOMBSTONES: [&str; 3] = [
    "connectors.feed.FeedItem/transition/remove/by/connectors.feed.RemoveItem/removed",
    "connectors.feed.RemoveItem/outcome/removed",
    "connectors.feed/authored/deleted-item",
];
const STORED_KIND: [&str; 2] = [
    "connectors.feed.AddContainer/outcome/added",
    "connectors.feed.ReadItems/outcome/read",
];
const RESTORE_AT_A_REUSED_INSTANT: [&str; 2] = [
    "connectors.feed.FeedItem/transition/restore/by/connectors.feed.RestoreItem/restored",
    "connectors.feed.RestoreItem/outcome/restored",
];
const PUBLIC_LISTING: [&str; 2] = [
    "connectors.feed.AddContainer/outcome/added",
    "connectors.feed.ReadItems/outcome/read",
];
/// Every kept scenario that compares a read item's revision with the scenario's revision word.
const REVISION_WORDS: [&str; 10] = [
    "connectors.feed.AddItem/outcome/added",
    "connectors.feed.FeedItem/transition/remove/by/connectors.feed.RemoveItem/removed",
    "connectors.feed.FeedItem/transition/revise/by/connectors.feed.ReviseItem/revised",
    "connectors.feed.RemoveItem/outcome/removed",
    "connectors.feed.ReviseItem/outcome/revised",
    "connectors.feed/authored/deleted-item",
    "connectors.feed/authored/first-read",
    "connectors.feed/authored/more-unseen-than-limit",
    "connectors.feed/authored/page-boundary-inside-one-instant",
    "connectors.feed/authored/resumed-read",
];
const OMITS_DIRECT: &str = "connectors.feed/authored/listing-omits-direct-conversation";

fn ids(ids: &[&'static str]) -> BTreeSet<&'static str> {
    ids.iter().copied().collect()
}

/// A run that passes everything it ran, and accounts for every scenario the family defines.
fn assert_passes(run: &Run) {
    println!("{}", run.summary());
    assert!(run.accounts_for_the_family(), "{}", run.summary());
    assert_eq!(run.status, CountStatus::Passed, "{}", run.report);
    assert_eq!(run.counts.passed, run.counts.total, "{}", run.report);
}

fn failed(run: &Run) -> BTreeSet<&str> {
    run.failed.iter().map(String::as_str).collect()
}

/// `(scenario, field, expectation as stated, change)` of every expectation a run changed.
fn changes(run: &Run) -> BTreeSet<(&str, &str, &str, Change)> {
    run.changed
        .iter()
        .map(|changed| {
            (
                changed.scenario.as_str(),
                changed.field,
                changed.expect.as_str(),
                changed.change.clone(),
            )
        })
        .collect()
}

/// A profile that does not observe deletions runs every scenario: each expected tombstone is
/// held to never being reported, and the one expectation a removal must be observed to hold is
/// masked. A binding claiming it that reports tombstones fails exactly the tombstone scenarios;
/// so does one that claims to observe deletions while its provider deletes outright.
#[test]
fn a_not_observed_profile_is_held_to_reporting_no_tombstone() {
    let not_observed = Capabilities {
        deletions: Deletions::NotObserved,
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        deletes_outright: true,
        ..Shaped::declaring(not_observed)
    });
    assert_passes(&honest);
    assert!(honest.left_out.is_empty(), "{}", honest.summary());
    let mut expected: BTreeSet<(&str, &str, &str, Change)> = TOMBSTONES
        .iter()
        .map(|id| (*id, "state", "contains", Change::Excluded))
        .collect();
    expected.insert((TOMBSTONES[2], "state", "excludes", Change::Masked));
    assert_eq!(changes(&honest), expected);
    assert!(honest.summary().contains(&format!(
        "turned ContainerItems.state (contains) into an excludes of it in {} (deletions: not-observed)",
        TOMBSTONES[2]
    )));

    let reports_tombstones = run(Shaped::declaring(not_observed));
    println!("{}", reports_tombstones.summary());
    assert_eq!(failed(&reports_tombstones), ids(&TOMBSTONES));

    let overclaimed = run(Shaped {
        deletes_outright: true,
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    println!("{}", overclaimed.summary());
    assert!(overclaimed.left_out.is_empty());
    assert_eq!(
        failed(&overclaimed),
        ids(&TOMBSTONES),
        "{}",
        overclaimed.report
    );
}

/// A profile that lists every container `private` runs every scenario: a container stored
/// `public` is held to being listed `private`, and the profile is still held to the listed
/// container's `kind` and `name` and to omitting a direct conversation. Claiming `all-private`
/// while listing one `public` fails; so does claiming `mapped` while listing everything
/// `private`.
#[test]
fn an_all_private_profile_is_held_to_listing_every_container_private() {
    let all_private = Capabilities {
        visibility: Visibility::AllPrivate,
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        lists_all_private: true,
        ..Shaped::declaring(all_private)
    });
    assert_passes(&honest);
    assert!(honest.left_out.is_empty(), "{}", honest.summary());
    let expected: BTreeSet<(&str, &str, &str, Change)> = PUBLIC_LISTING
        .iter()
        .map(|id| {
            (
                *id,
                "visibility",
                "contains",
                Change::HeldTo("private".into()),
            )
        })
        .collect();
    assert_eq!(changes(&honest), expected);
    assert!(honest.summary().contains(&format!(
        "held ListedContainers.visibility (contains) to `private` in {} (visibility: all-private)",
        PUBLIC_LISTING[1]
    )));

    let lists_public = run(Shaped::declaring(all_private));
    println!("{}", lists_public.summary());
    assert_eq!(failed(&lists_public), ids(&PUBLIC_LISTING));

    let lists_direct = run(Shaped {
        lists_all_private: true,
        lists_direct_conversations: true,
        ..Shaped::declaring(all_private)
    });
    println!("{}", lists_direct.summary());
    assert_eq!(lists_direct.status, CountStatus::Failed);
    assert!(
        lists_direct.failed.contains(OMITS_DIRECT),
        "{}",
        lists_direct.report
    );

    let overclaimed = run(Shaped {
        lists_all_private: true,
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    println!("{}", overclaimed.summary());
    assert_eq!(failed(&overclaimed), ids(&PUBLIC_LISTING));
}

/// A profile that fixes one `kind` word runs every scenario with a listed container's kind held
/// to that word. Listing another word fails; claiming `fixed-word` with no word is held to the
/// provider's word, as is claiming the provider's word.
#[test]
fn a_fixed_kind_word_profile_is_held_to_its_word() {
    let room = Capabilities {
        kind: Kind::FixedWord,
        kind_word: Some("room"),
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        kind_word: Some("room"),
        ..Shaped::declaring(room)
    });
    assert_passes(&honest);
    assert!(honest.left_out.is_empty(), "{}", honest.summary());
    let expected: BTreeSet<(&str, &str, &str, Change)> = STORED_KIND
        .iter()
        .map(|id| (*id, "kind", "contains", Change::HeldTo("room".into())))
        .collect();
    assert_eq!(changes(&honest), expected);

    let other_word = run(Shaped {
        kind_word: Some("chat"),
        ..Shaped::declaring(room)
    });
    assert_eq!(failed(&other_word), ids(&STORED_KIND));

    let no_word = run(Shaped {
        kind_word: Some("room"),
        ..Shaped::declaring(Capabilities {
            kind: Kind::FixedWord,
            ..Capabilities::STRONGEST
        })
    });
    assert!(no_word.changed.is_empty());
    assert!(
        no_word
            .summary()
            .contains("claims `kind: fixed-word` and states no word: held to the provider's word")
    );
    assert_eq!(failed(&no_word), ids(&STORED_KIND));

    let overclaimed = run(Shaped {
        kind_word: Some("room"),
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    assert_eq!(failed(&overclaimed), ids(&STORED_KIND));
}

/// A profile whose revision is the item's update time passes with the scenarios that give one
/// item two revisions at one instant left out, and every read item's revision word, in a
/// `contains` and an `excludes` alike, held to the instant the item took it; claiming an opaque
/// revision fails all of them.
#[test]
fn an_update_time_profile_is_held_to_the_instant_of_each_revision() {
    let update_time = Capabilities {
        revision: Revision::UpdateTime,
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        revision_is_update_time: true,
        ..Shaped::declaring(update_time)
    });
    assert_passes(&honest);
    assert_eq!(honest.left_out_ids(), ids(&RESTORE_AT_A_REUSED_INSTANT));
    assert_eq!(honest.changed_ids(), ids(&REVISION_WORDS));
    assert!(
        honest
            .changed
            .iter()
            .all(|changed| changed.field == "revision"
                && matches!(changed.change, Change::HeldTo(_))
                && changed.by == Weaker::RevisionUpdateTime)
    );
    let excludes: BTreeSet<(&str, Change)> = honest
        .changed
        .iter()
        .filter(|changed| changed.expect == "excludes")
        .map(|changed| (changed.scenario.as_str(), changed.change.clone()))
        .collect();
    assert_eq!(
        excludes,
        BTreeSet::from([
            (
                "connectors.feed/authored/more-unseen-than-limit",
                Change::HeldTo("2026-10-06T09:00:01Z".into())
            ),
            (
                "connectors.feed/authored/resumed-read",
                Change::HeldTo("2026-10-06T09:00:01Z".into())
            ),
        ])
    );

    let overclaimed = run(Shaped {
        revision_is_update_time: true,
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    let mut expected = ids(&REVISION_WORDS);
    expected.extend(RESTORE_AT_A_REUSED_INSTANT);
    assert_eq!(failed(&overclaimed), expected, "{}", overclaimed.report);
}

/// What each capability leaves out of the family's 28 and changes in the rest, and that
/// capabilities combine: a scenario two of them speak about is changed by each.
#[test]
fn each_capability_leaves_out_or_changes_exactly_what_it_speaks_about() {
    let root = root();
    assert_eq!(family(&root).len() as u64, EXPECTED_SCENARIOS);
    type Selection = (BTreeSet<String>, BTreeSet<(String, &'static str)>);
    let select = |capabilities: Capabilities| -> Selection {
        let selected = suite(&root, &capabilities);
        (
            selected
                .left_out
                .into_iter()
                .map(|left| left.scenario)
                .collect(),
            selected
                .changed
                .into_iter()
                .map(|changed| (changed.scenario, changed.field))
                .collect(),
        )
    };
    let owned = |ids: &[&str]| -> BTreeSet<String> { ids.iter().map(|id| (*id).into()).collect() };
    let fields = |ids: &[&str], field: &'static str| -> BTreeSet<(String, &'static str)> {
        ids.iter().map(|id| ((*id).into(), field)).collect()
    };
    assert_eq!(select(Capabilities::STRONGEST), Selection::default());
    let without_word = Capabilities {
        kind: Kind::FixedWord,
        ..Capabilities::STRONGEST
    };
    assert_eq!(select(without_word), Selection::default());
    let single = [
        (
            Capabilities {
                deletions: Deletions::NotObserved,
                ..Capabilities::STRONGEST
            },
            (BTreeSet::new(), fields(&TOMBSTONES, "state")),
        ),
        (
            Capabilities {
                kind: Kind::FixedWord,
                kind_word: Some("room"),
                ..Capabilities::STRONGEST
            },
            (BTreeSet::new(), fields(&STORED_KIND, "kind")),
        ),
        (
            Capabilities {
                revision: Revision::UpdateTime,
                ..Capabilities::STRONGEST
            },
            (
                owned(&RESTORE_AT_A_REUSED_INSTANT),
                fields(&REVISION_WORDS, "revision"),
            ),
        ),
        (
            Capabilities {
                visibility: Visibility::AllPrivate,
                ..Capabilities::STRONGEST
            },
            (BTreeSet::new(), fields(&PUBLIC_LISTING, "visibility")),
        ),
    ];
    for (capabilities, expected) in single {
        assert_eq!(select(capabilities), expected, "{capabilities:?}");
    }
    let weakest = Capabilities {
        deletions: Deletions::NotObserved,
        kind: Kind::FixedWord,
        kind_word: Some("room"),
        revision: Revision::UpdateTime,
        visibility: Visibility::AllPrivate,
    };
    let (left_out, changed) = select(weakest);
    assert_eq!(left_out, owned(&RESTORE_AT_A_REUSED_INSTANT));
    // A tombstone held to never being reported names no revision, so the three tombstone
    // scenarios' revision words are gone before `update-time` reads them.
    let revision_words: Vec<&str> = REVISION_WORDS
        .iter()
        .copied()
        .filter(|id| !TOMBSTONES.contains(id))
        .collect();
    let mut expected = fields(&TOMBSTONES, "state");
    expected.extend(fields(&STORED_KIND, "kind"));
    expected.extend(fields(&revision_words, "revision"));
    expected.extend(fields(&PUBLIC_LISTING, "visibility"));
    assert_eq!(changed, expected);
    assert_eq!(changed.len(), 14);
    assert_eq!(suite(&root, &weakest).admitted.suite().len(), 26);
}

/// The capabilities this harness reads are the shared model's: the same fields, each enum the
/// same closed vocabulary, and `kind_word` an optional text. The rule tying the word to `kind:
/// fixed-word` is the harness's ([`Capabilities::refused`]), as the model's ESS-LIMIT states.
#[test]
fn the_capabilities_are_the_shared_models() {
    let ir = metadata_entities::compile(&metadata_entities::load(&root()).expect("load ess"))
        .expect("compile ess");
    let model = serde_json::to_value(ir.types()).expect("types");
    let fields = struct_fields(&model, "connectors.feed.ProfileCapabilities");
    let stated = Capabilities {
        kind: Kind::FixedWord,
        kind_word: Some("room"),
        ..Capabilities::STRONGEST
    };
    let rust = serde_json::to_value(stated).unwrap();
    assert_eq!(
        fields.keys().cloned().collect::<BTreeSet<_>>(),
        rust.as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        fields["kind_word"],
        serde_json::json!({"kind": "optional", "of": {"kind": "primitive", "name": "string"}})
            .to_string()
    );
    let read: Capabilities = serde_json::from_value(rust).unwrap();
    assert_eq!(read, stated);
    for (kind, word, refused) in [
        (Kind::ProviderWord, None, false),
        (Kind::ProviderWord, Some("room"), true),
        (Kind::FixedWord, None, true),
        (Kind::FixedWord, Some("room"), false),
    ] {
        let claim = Capabilities {
            kind,
            kind_word: word,
            ..Capabilities::STRONGEST
        };
        assert_eq!(claim.refused().is_some(), refused, "{claim:?}");
    }
    let variants = |value: Vec<Value>| -> BTreeSet<String> {
        value
            .into_iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    let cases: [(&str, Vec<Value>); 4] = [
        (
            "deletions",
            vec![
                serde_json::to_value(Deletions::Observed).unwrap(),
                serde_json::to_value(Deletions::NotObserved).unwrap(),
            ],
        ),
        (
            "kind",
            vec![
                serde_json::to_value(Kind::ProviderWord).unwrap(),
                serde_json::to_value(Kind::FixedWord).unwrap(),
            ],
        ),
        (
            "revision",
            vec![
                serde_json::to_value(Revision::Opaque).unwrap(),
                serde_json::to_value(Revision::UpdateTime).unwrap(),
            ],
        ),
        (
            "visibility",
            vec![
                serde_json::to_value(Visibility::Mapped).unwrap(),
                serde_json::to_value(Visibility::AllPrivate).unwrap(),
            ],
        ),
    ];
    for (field, rust) in cases {
        assert_eq!(
            enum_variants(&model, &fields[field]),
            variants(rust),
            "{field}"
        );
    }
}

/// A struct type's fields in the compiled model, each with the declared type it names, or the
/// compiled type reference itself where it names none.
fn struct_fields(model: &Value, name: &str) -> BTreeMap<String, String> {
    let body = &model[name]["body"];
    assert_eq!(body["kind"], "struct", "{name}");
    body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| {
            let type_ref = &field["type_ref"];
            (
                field["name"].as_str().expect("name").to_owned(),
                type_ref["name"]
                    .as_str()
                    .filter(|_| type_ref["kind"] == "declared")
                    .map_or_else(|| type_ref.to_string(), str::to_owned),
            )
        })
        .collect()
}

/// An enum type's variants in the compiled model.
fn enum_variants(model: &Value, name: &str) -> BTreeSet<String> {
    let body = &model[name]["body"];
    assert_eq!(body["kind"], "enum", "{name}");
    body["variants"]
        .as_array()
        .expect("variants")
        .iter()
        .map(|variant| variant.as_str().expect("variant").to_owned())
        .collect()
}

/// The resume rule the specification cannot state: reading again from the last watermark
/// returns every change since, including an item changed again after it was returned, and a
/// tombstone under a new revision; the watermark never passes an undelivered change.
#[test]
fn a_resumed_read_returns_every_change_since_the_watermark() {
    let mut source = Source::default();
    source.containers.insert(
        "room".into(),
        Container {
            connection: "workspace-a".into(),
            name: None,
            kind: "channel".into(),
            visibility: "public".into(),
        },
    );
    fn add(source: &mut Source, id: &str, revision: &str) {
        source.sequence += 1;
        source.items.insert(
            id.into(),
            Item {
                container: "room".into(),
                revision: revision.into(),
                created_at: "2026-10-06T09:00:00Z".into(),
                updated_at: "2026-10-06T09:00:00Z".into(),
                parent: None,
                deleted: false,
                changed: source.sequence,
            },
        );
    }
    add(&mut source, "a", "r1");
    add(&mut source, "b", "r1");
    let first = source.items("room", None, 1).unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(!first.complete, "a page bounded by limit is not complete");
    let second = source
        .items("room", Some(&first.next_watermark), 10)
        .unwrap();
    assert_eq!(
        second
            .items
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["b"]
    );
    assert!(second.complete);
    // Nothing new: an empty page still carries a watermark, at the same position.
    let idle = source
        .items("room", Some(&second.next_watermark), 10)
        .unwrap();
    assert!(idle.items.is_empty() && idle.complete);
    assert_eq!(idle.next_watermark, second.next_watermark);
    // `a` changes, then is removed; `c` is new.
    add(&mut source, "c", "r1");
    source.sequence += 1;
    let a = source.items.get_mut("a").unwrap();
    a.revision = "r2".into();
    a.changed = source.sequence;
    source.sequence += 1;
    let a = source.items.get_mut("a").unwrap();
    a.revision = "r3-removed".into();
    a.deleted = true;
    a.changed = source.sequence;
    let resumed = source
        .items("room", Some(&idle.next_watermark), 10)
        .unwrap();
    let got: Vec<_> = resumed
        .items
        .iter()
        .map(|(id, item)| (id.as_str(), item.revision.as_str(), item.deleted))
        .collect();
    assert_eq!(got, [("c", "r1", false), ("a", "r3-removed", true)]);
    // Foreign, malformed and forged watermarks are refused, never read as a first read.
    source.containers.insert(
        "other".into(),
        Container {
            connection: "workspace-a".into(),
            name: None,
            kind: "channel".into(),
            visibility: "private".into(),
        },
    );
    assert_eq!(
        source.items("other", Some(&idle.next_watermark), 10).err(),
        Some(Code::StaleCursor)
    );
    assert_eq!(
        source.items("room", Some("opaque"), 10).err(),
        Some(Code::StaleCursor)
    );
    assert_eq!(
        source
            .items("room", Some(&format!("{WATERMARK}:room:999")), 10)
            .err(),
        Some(Code::StaleCursor)
    );
    assert_eq!(
        source.items("room", None, 0).err(),
        Some(Code::InvalidInput)
    );
}

/// Direct conversations are neither listed nor readable, and the answer to reading one is the
/// answer to reading a container that does not exist.
#[test]
fn direct_conversations_are_excluded_indistinguishably() {
    let mut source = Source::default();
    for (id, visibility) in [
        ("a-public", "public"),
        ("b-direct", "direct"),
        ("c-private", "private"),
    ] {
        source.containers.insert(
            id.into(),
            Container {
                connection: "workspace-a".into(),
                name: None,
                kind: "k".into(),
                visibility: visibility.into(),
            },
        );
    }
    let first = source.containers(Some(1), None).unwrap();
    let second = source
        .containers(Some(1), first.next_cursor.as_deref())
        .unwrap();
    let listed: Vec<_> = first
        .containers
        .iter()
        .chain(&second.containers)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(listed, ["a-public", "c-private"]);
    assert!(second.complete && second.next_cursor.is_none());
    assert_eq!(
        source.items("b-direct", None, 10).err(),
        source.items("missing", None, 10).err()
    );
    assert_eq!(
        source.items("b-direct", None, 10).err(),
        Some(Code::NotFound)
    );
}

// ---- adversary, wave 20261007a U1 ----------------------------------------------------------

/// A profile that fixes its kind word still claims `visibility: mapped`, so it is still held to
/// listing a public container `public` ("mapped bindings stay checked for public"). Listing every
/// container `private` under that claim must fail.
#[test]
fn adversary_a_fixed_word_profile_claiming_mapped_visibility_is_held_to_public() {
    let overclaimed = run(Shaped {
        kind_word: Some("room"),
        lists_all_private: true,
        ..Shaped::declaring(Capabilities {
            kind: Kind::FixedWord,
            ..Capabilities::STRONGEST
        })
    });
    println!("{}", overclaimed.summary());
    assert_eq!(
        overclaimed.status,
        CountStatus::Failed,
        "every container listed `private` passed under `visibility: mapped`:\n{}",
        overclaimed.summary()
    );
}

// ---- adversary pass 2, wave 20261007a U1 -----------------------------------------------------

/// The native binding with a defect a weaker capability's masking might hide.
struct Defective {
    declares: Capabilities,
    /// The watermark follows each item's creation, not its last change: an item changed after a
    /// consumer read it is never read again.
    loses_changes: bool,
    /// The provider deletes items outright.
    deletes_outright: bool,
    /// What the binding carries as an item's revision.
    revision: fn(&Item) -> String,
    /// What the binding carries as a listed container's kind, from its id and the stored kind.
    kind: fn(&str, &Container) -> String,
}

impl Defective {
    fn declaring(declares: Capabilities) -> Self {
        Self {
            declares,
            loses_changes: false,
            deletes_outright: false,
            revision: |item| item.revision.clone(),
            kind: |_, container| container.kind.clone(),
        }
    }
}

impl Binding for Defective {
    fn identity(&self) -> (String, String) {
        ("connectors-feed-fixture-defective".into(), WATERMARK.into())
    }
    fn capabilities(&self) -> Capabilities {
        self.declares
    }
    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code> {
        let mut page = source.containers(limit, cursor)?;
        for (id, container) in &mut page.containers {
            container.kind = (self.kind)(id, container);
        }
        Ok(page)
    }
    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        let mut seen = source.clone();
        if self.loses_changes {
            let mut order: Vec<(String, String)> = seen
                .items
                .iter()
                .map(|(id, item)| (item.created_at.clone(), id.clone()))
                .collect();
            order.sort();
            for (rank, (_, id)) in order.iter().enumerate() {
                seen.items.get_mut(id).unwrap().changed = rank as u64 + 1;
            }
        }
        if self.deletes_outright {
            seen.items.retain(|_, item| !item.deleted);
        }
        let mut page = seen.items(container, watermark, limit)?;
        for (_, item) in &mut page.items {
            item.revision = (self.revision)(item);
        }
        Ok(page)
    }
}

/// `semantics.md` (Model and conformance): "A binding that loses a change on resume then fails the
/// scenarios that read it." Under `revision: update-time`, the GitLab-shaped profile, the
/// `resumed-read` scenario keeps only `item-1 … state: Present` once its revision is masked and
/// its `excludes item-1 r1` is masked whole, which the stale item satisfies. The same binding with
/// an opaque revision fails it, so it is the masking, not the scenario, that lets the loss through.
#[test]
fn adversary2_an_update_time_binding_that_loses_a_change_on_resume_fails() {
    let opaque = run(Defective {
        loses_changes: true,
        deletes_outright: true,
        ..Defective::declaring(Capabilities {
            deletions: Deletions::NotObserved,
            ..Capabilities::STRONGEST
        })
    });
    println!("{}", opaque.summary());
    assert!(
        failed(&opaque).contains("connectors.feed/authored/resumed-read"),
        "{}",
        opaque.report
    );
    let update_time = run(Defective {
        loses_changes: true,
        deletes_outright: true,
        revision: |item| item.updated_at.clone(),
        ..Defective::declaring(Capabilities {
            deletions: Deletions::NotObserved,
            revision: Revision::UpdateTime,
            ..Capabilities::STRONGEST
        })
    });
    println!("{}", update_time.summary());
    assert!(
        failed(&update_time).contains("connectors.feed/authored/resumed-read"),
        "a binding that never reads a changed item again passed under `revision: update-time` \
         (failed: {:?}):\n{}",
        update_time.failed,
        update_time.summary()
    );
}

/// `revision: update-time` states that a revision is the item's update time. A binding claiming it
/// whose revision is the creation time, so a revised item never carries a new revision
/// (`semantics.md`, Revisions: "each of those gives a new revision that the item has not carried
/// before"), must fail; the claim is never compared with anything once `revision` is masked.
#[test]
fn adversary2_an_update_time_claim_is_held_to_the_update_time() {
    let frozen = run(Defective {
        revision: |item| item.created_at.clone(),
        ..Defective::declaring(Capabilities {
            revision: Revision::UpdateTime,
            ..Capabilities::STRONGEST
        })
    });
    println!("{}", frozen.summary());
    assert_eq!(
        frozen.status,
        CountStatus::Failed,
        "a revision that never changes passed under `revision: update-time`:\n{}",
        frozen.summary()
    );
}

/// `kind: fixed-word` states "the one word the profile states for every container it lists"
/// (`semantics.md`, Containers). A binding claiming it that lists each container's id as its kind
/// lists as many words as containers, and must fail; once `kind` is masked nothing compares it.
#[test]
fn adversary2_a_fixed_word_claim_is_held_to_one_word() {
    let many_words = run(Defective {
        kind: |id, _| id.to_owned(),
        ..Defective::declaring(Capabilities {
            kind: Kind::FixedWord,
            ..Capabilities::STRONGEST
        })
    });
    println!("{}", many_words.summary());
    assert_eq!(
        many_words.status,
        CountStatus::Failed,
        "a kind that differs per container passed under `kind: fixed-word`:\n{}",
        many_words.summary()
    );
}

/// `semantics.md` (Deletions): a profile that declares `deletions: not-observed` "never reports
/// `deleted: true`". A binding claiming it that still reports tombstones must fail; with the
/// tombstone scenarios left out, nothing it is held to removes an item and reads it again.
#[test]
fn adversary2_a_not_observed_claim_is_held_to_reporting_no_tombstone() {
    let reports_tombstones = run(Defective::declaring(Capabilities {
        deletions: Deletions::NotObserved,
        ..Capabilities::STRONGEST
    }));
    println!("{}", reports_tombstones.summary());
    assert_eq!(
        reports_tombstones.status,
        CountStatus::Failed,
        "a binding reporting tombstones passed under `deletions: not-observed`:\n{}",
        reports_tombstones.summary()
    );
}
