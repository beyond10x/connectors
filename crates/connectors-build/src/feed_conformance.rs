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
//! provider. Before admission, since a Rust producer admits no skip, a scenario a weaker
//! capability makes inapplicable is left out, and in every scenario kept, an expected field a
//! weaker capability makes inapplicable is masked or held to what the profile states. [`Run`]
//! names each scenario left out and each field changed with the capability that did it, beside
//! the count of scenarios run and the family's total.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capabilities {
    pub(crate) deletions: Deletions,
    pub(crate) kind: Kind,
    pub(crate) revision: Revision,
    pub(crate) visibility: Visibility,
}

impl Capabilities {
    /// What the strongest provider gives: every scenario the family defines applies.
    pub(crate) const STRONGEST: Self = Self {
        deletions: Deletions::Observed,
        kind: Kind::ProviderWord,
        revision: Revision::Opaque,
        visibility: Visibility::Mapped,
    };

    /// The capabilities this profile states below the strongest provider's.
    fn weaker(&self) -> Vec<Weaker> {
        [
            (self.deletions == Deletions::NotObserved).then_some(Weaker::DeletionsNotObserved),
            (self.kind == Kind::FixedWord).then_some(Weaker::KindFixedWord),
            (self.revision == Revision::UpdateTime).then_some(Weaker::RevisionUpdateTime),
            (self.visibility == Visibility::AllPrivate).then_some(Weaker::VisibilityAllPrivate),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// A capability below the strongest provider's, and what it makes inapplicable: a whole scenario,
/// or one field of what a scenario expects of the binding's own reads.
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
    /// what the scenario expects of the binding's own reads (`ListedContainers`,
    /// `ContainerItems`), never from its id:
    /// - `deletions: not-observed`: it expects a consumer to hold a tombstone (`state: Deleted`);
    /// - `revision: update-time`: it reads one item's items after the source gave that item two
    ///   revisions at one instant, which an update-time revision cannot tell apart.
    ///
    /// `kind` and `visibility` leave no scenario out; [`Weaker::mask`] changes one field.
    fn leaves_out(self, scenario: &Value) -> bool {
        let steps = scenario["steps"].as_array().map_or(&[][..], Vec::as_slice);
        match self {
            Self::DeletionsNotObserved => expects(steps, "ContainerItems").any(|expectation| {
                expectation["expect"] == "contains"
                    && literal(&expectation["fields"]["state"]) == Some(&Value::from("Deleted"))
            }),
            Self::RevisionUpdateTime => {
                reads(steps, "ContainerItems") && one_item_reuses_an_instant(steps)
            }
            Self::KindFixedWord | Self::VisibilityAllPrivate => false,
        }
    }

    /// The field of a binding view this capability speaks about.
    fn field(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::DeletionsNotObserved => None,
            Self::KindFixedWord => Some(("ListedContainers", "kind")),
            Self::RevisionUpdateTime => Some(("ContainerItems", "revision")),
            Self::VisibilityAllPrivate => Some(("ListedContainers", "visibility")),
        }
    }

    /// Change, in a scenario the binding is held to, what this capability makes inapplicable in
    /// the expectations on the binding's own reads, and record each change:
    /// - `kind: fixed-word` masks a listed container's `kind`: the stored word is not the one the
    ///   profile fixes;
    /// - `revision: update-time` masks a read item's `revision`: the scenario's revision word is
    ///   not the update time the binding carries;
    /// - `visibility: all-private` holds a container stored `public` to being listed `private`,
    ///   which is what the profile states.
    ///
    /// A masked field leaves a `contains` with the rest of its fields; an `excludes` naming it is
    /// masked whole, since without the field it would refuse more rather than less.
    fn mask(self, scenario_id: &str, scenario: &mut Value) -> Vec<Masked> {
        let Some((view, field)) = self.field() else {
            return Vec::new();
        };
        let Some(steps) = scenario["steps"].as_array_mut() else {
            return Vec::new();
        };
        let qualified = format!("{DOMAIN}.{view}");
        let mut masked = Vec::new();
        steps.retain_mut(|step| {
            if step["step"] != "expect_view" || step["view"] != qualified.as_str() {
                return true;
            }
            let expectation = &mut step["expectation"];
            let expect = expectation["expect"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let Some(fields) = expectation.get_mut("fields").and_then(Value::as_object_mut) else {
                return true;
            };
            let Some(value) = fields.get_mut(field) else {
                return true;
            };
            let record = |held_to: Option<&'static str>| Masked {
                scenario: scenario_id.to_owned(),
                view,
                field,
                expect: expect.clone(),
                held_to,
                by: self,
            };
            if self == Self::VisibilityAllPrivate {
                if expect == "contains" && literal(value) == Some(&Value::from("public")) {
                    value["value"] = Value::from("private");
                    masked.push(record(Some("private")));
                }
                return true;
            }
            masked.push(record(None));
            if expect == "contains" {
                fields.remove(field);
                return !fields.is_empty();
            }
            false
        });
        masked
    }
}

/// One field of what a kept scenario expects of a binding view, changed by a capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Masked {
    pub(crate) scenario: String,
    pub(crate) view: &'static str,
    pub(crate) field: &'static str,
    /// `contains` or `excludes`.
    pub(crate) expect: String,
    /// The value the field is held to instead; `None` where its assertion is masked.
    pub(crate) held_to: Option<&'static str>,
    pub(crate) by: Weaker,
}

/// Every expectation a scenario states about one of the family's views.
fn expects<'a>(steps: &'a [Value], view: &str) -> impl Iterator<Item = &'a Value> {
    let view = format!("{DOMAIN}.{view}");
    steps
        .iter()
        .filter(move |step| step["step"] == "expect_view" && step["view"] == view.as_str())
        .map(|step| &step["expectation"])
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

/// True when the source gives one item two different revisions at one instant: an item added,
/// revised, removed or restored at a time it already carried under another revision. Only
/// changes the scenario expects to take effect count; a refused change moves nothing.
fn one_item_reuses_an_instant(steps: &[Value]) -> bool {
    // An instance an earlier step bound stands for the item id the command that bound it took.
    let mut instances: BTreeMap<String, Value> = BTreeMap::new();
    let mut input: Option<&Value> = None;
    let mut change: Option<(Value, Value, Value)> = None;
    let mut revisions: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let resolve = |instances: &BTreeMap<String, Value>, value: &Value| -> Value {
        match (literal(value), value["instance"].as_str()) {
            (Some(literal), _) => literal.clone(),
            (None, Some(instance)) => instances
                .get(instance)
                .cloned()
                .unwrap_or_else(|| Value::from(instance)),
            (None, None) => value.clone(),
        }
    };
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
                    resolve(&instances, &input["item_id"]),
                    resolve(&instances, &input[at]),
                    resolve(&instances, &input["revision"]),
                ));
            }
            Some("expect_outcome") => {
                let took = matches!(
                    step["outcome"]["outcome"].as_str(),
                    Some("added" | "revised" | "removed" | "restored")
                );
                if let (true, Some((item, at, revision))) = (took, change.take()) {
                    revisions
                        .entry((item.to_string(), at.to_string()))
                        .or_default()
                        .insert(revision.to_string());
                }
            }
            Some("capture_instance") => {
                if let (Some(instance), Some(field), Some(input)) =
                    (step["instance"].as_str(), step["field"].as_str(), input)
                {
                    let bound = resolve(&instances, &input[field]);
                    instances.insert(instance.to_owned(), bound);
                }
            }
            _ => {}
        }
    }
    revisions.values().any(|revisions| revisions.len() > 1)
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
    /// Every field of a kept scenario a capability masked or held to another value.
    pub(crate) masked: Vec<Masked>,
}

/// The suite a binding stating `capabilities` is held to: the family's scenarios less the ones a
/// capability leaves out, with the fields a capability makes inapplicable masked in the rest.
pub(crate) fn suite(root: &Path, capabilities: &Capabilities) -> Selected {
    let mut suite = family(root);
    let weaker = capabilities.weaker();
    let mut left_out = Vec::new();
    let mut masked = Vec::new();
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
        let changed: Vec<Masked> = weaker
            .iter()
            .flat_map(|capability| capability.mask(&id, &mut value))
            .collect();
        if !changed.is_empty() {
            *scenario = serde_json::from_value(value).expect("a masked scenario is a scenario");
            masked.extend(changed);
        }
        true
    });
    masked.sort();
    masked.dedup();
    Selected {
        admitted: AdmittedSuite::from_suite(&suite).expect("admitted"),
        left_out,
        masked,
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
    pub(crate) counts: ess_conformance::counts::ScenarioCounts,
    pub(crate) status: CountStatus,
    /// The scenarios that did not pass.
    pub(crate) failed: BTreeSet<String>,
    /// The family's scenarios this profile's capabilities left out, named.
    pub(crate) left_out: Vec<LeftOut>,
    /// The fields of kept scenarios this profile's capabilities masked or held to another value.
    pub(crate) masked: Vec<Masked>,
    /// The runner's report, for a failure message.
    pub(crate) report: String,
}

impl Run {
    /// The scenarios run beside the family's total, then every one left out and every field
    /// masked, each with the capability that did it.
    pub(crate) fn summary(&self) -> String {
        let mut summary = format!(
            "{}: ran {} of the {EXPECTED_SCENARIOS} scenarios the family defines ({} passed)",
            self.profile, self.counts.total, self.counts.passed
        );
        for left in &self.left_out {
            let by: Vec<&str> = left.by.iter().map(|capability| capability.name()).collect();
            summary.push_str(&format!(
                "\n  left out {} ({})",
                left.scenario,
                by.join(", ")
            ));
        }
        for masked in &self.masked {
            let change = match masked.held_to {
                Some(value) => format!("held {}.{} to `{value}`", masked.view, masked.field),
                None => format!("masked {}.{}", masked.view, masked.field),
            };
            summary.push_str(&format!(
                "\n  {change} ({}) in {} ({})",
                masked.expect,
                masked.scenario,
                masked.by.name()
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

    /// The kept scenarios with a field a capability changed, by id.
    pub(crate) fn masked_ids(&self) -> BTreeSet<&str> {
        self.masked
            .iter()
            .map(|masked| masked.scenario.as_str())
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
    let Selected {
        admitted,
        left_out,
        masked,
    } = suite(&root(), &binding.capabilities());
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
        counts: report.counts().clone(),
        status: report.execution_status(),
        failed,
        left_out,
        masked,
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

/// A profile that does not observe deletions passes with the tombstone scenarios left out and
/// named; one that claims to observe them while its provider deletes outright fails exactly
/// those.
#[test]
fn a_profile_that_observes_no_deletions_is_not_held_to_tombstones() {
    let not_observed = Capabilities {
        deletions: Deletions::NotObserved,
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        deletes_outright: true,
        ..Shaped::declaring(not_observed)
    });
    assert_passes(&honest);
    assert_eq!(honest.left_out_ids(), ids(&TOMBSTONES));
    assert!(
        honest
            .left_out
            .iter()
            .all(|left| left.by == [Weaker::DeletionsNotObserved])
    );
    assert!(honest.summary().contains(&format!(
        "left out {} (deletions: not-observed)",
        TOMBSTONES[2]
    )));

    let overclaimed = run(Shaped {
        deletes_outright: true,
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    println!("{}", overclaimed.summary());
    assert!(overclaimed.left_out.is_empty());
    assert_eq!(overclaimed.status, CountStatus::Failed);
    assert_eq!(
        overclaimed
            .failed
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        ids(&TOMBSTONES),
        "{}",
        overclaimed.report
    );
}

fn failed(run: &Run) -> BTreeSet<&str> {
    run.failed.iter().map(String::as_str).collect()
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
    assert_eq!(honest.masked_ids(), ids(&PUBLIC_LISTING));
    assert!(
        honest
            .masked
            .iter()
            .all(|masked| masked.field == "visibility"
                && masked.held_to == Some("private")
                && masked.by == Weaker::VisibilityAllPrivate)
    );
    assert!(honest.summary().contains(&format!(
        "held ListedContainers.visibility to `private` (contains) in {} (visibility: all-private)",
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

/// A profile that fixes one `kind` word runs every scenario with a listed container's stored
/// kind masked, and is still held to the rest of each listing; one that claims the provider's
/// word fails the scenarios that compare it.
#[test]
fn a_fixed_kind_word_profile_is_not_held_to_the_stored_kind() {
    let fixed = Capabilities {
        kind: Kind::FixedWord,
        ..Capabilities::STRONGEST
    };
    let honest = run(Shaped {
        kind_word: Some("room"),
        ..Shaped::declaring(fixed)
    });
    assert_passes(&honest);
    assert!(honest.left_out.is_empty(), "{}", honest.summary());
    assert_eq!(honest.masked_ids(), ids(&STORED_KIND));
    assert!(honest.masked.iter().all(|masked| masked.field == "kind"
        && masked.held_to.is_none()
        && masked.by == Weaker::KindFixedWord));

    let overclaimed = run(Shaped {
        kind_word: Some("room"),
        ..Shaped::declaring(Capabilities::STRONGEST)
    });
    assert_eq!(failed(&overclaimed), ids(&STORED_KIND));
}

/// A profile whose revision is the item's update time passes with the scenarios that give one
/// item two revisions at one instant left out and every read item's revision word masked in the
/// rest, an `excludes` naming one masked whole; claiming an opaque revision fails all of them.
#[test]
fn an_update_time_profile_is_not_held_to_the_revision_words() {
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
    assert_eq!(honest.masked_ids(), ids(&REVISION_WORDS));
    assert!(honest.masked.iter().all(|masked| masked.field == "revision"
        && masked.held_to.is_none()
        && masked.by == Weaker::RevisionUpdateTime));
    let excludes: BTreeSet<&str> = honest
        .masked
        .iter()
        .filter(|masked| masked.expect == "excludes")
        .map(|masked| masked.scenario.as_str())
        .collect();
    assert_eq!(
        excludes,
        BTreeSet::from([
            "connectors.feed/authored/more-unseen-than-limit",
            "connectors.feed/authored/resumed-read"
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

/// What each capability leaves out of the family's 28 and masks in the rest, and that
/// capabilities combine: a field two of them speak about is changed by each, and a scenario two
/// of them leave out is left out once.
#[test]
fn each_capability_leaves_out_or_masks_exactly_what_it_makes_inapplicable() {
    let root = root();
    assert_eq!(family(&root).len() as u64, EXPECTED_SCENARIOS);
    type Changed = (BTreeSet<String>, BTreeSet<(String, &'static str)>);
    let select = |capabilities: Capabilities| -> Changed {
        let selected = suite(&root, &capabilities);
        (
            selected
                .left_out
                .into_iter()
                .map(|left| left.scenario)
                .collect(),
            selected
                .masked
                .into_iter()
                .map(|masked| (masked.scenario, masked.field))
                .collect(),
        )
    };
    let owned = |ids: &[&str]| -> BTreeSet<String> { ids.iter().map(|id| (*id).into()).collect() };
    let fields = |ids: &[&str], field: &'static str| -> BTreeSet<(String, &'static str)> {
        ids.iter().map(|id| ((*id).into(), field)).collect()
    };
    assert_eq!(select(Capabilities::STRONGEST), Changed::default());
    let single = [
        (
            Capabilities {
                deletions: Deletions::NotObserved,
                ..Capabilities::STRONGEST
            },
            (owned(&TOMBSTONES), BTreeSet::new()),
        ),
        (
            Capabilities {
                kind: Kind::FixedWord,
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
        revision: Revision::UpdateTime,
        visibility: Visibility::AllPrivate,
    };
    let (left_out, masked) = select(weakest);
    let mut expected_left_out = owned(&TOMBSTONES);
    expected_left_out.extend(owned(&RESTORE_AT_A_REUSED_INSTANT));
    assert_eq!(left_out, expected_left_out);
    assert_eq!(left_out.len(), 5);
    assert_eq!(masked.len(), 11, "{masked:#?}");
    assert!(masked.contains(&("connectors.feed.AddContainer/outcome/added".into(), "kind")));
    assert!(masked.contains(&(
        "connectors.feed.AddContainer/outcome/added".into(),
        "visibility"
    )));
    assert_eq!(suite(&root, &weakest).admitted.suite().len(), 23);
}

/// The capabilities this harness reads are the shared model's: the same four fields, and each
/// the same closed vocabulary, in `connectors.feed.ProfileCapabilities`.
#[test]
fn the_capabilities_are_the_shared_models() {
    let ir = metadata_entities::compile(&metadata_entities::load(&root()).expect("load ess"))
        .expect("compile ess");
    let model = serde_json::to_value(ir.types()).expect("types");
    let fields = struct_fields(&model, "connectors.feed.ProfileCapabilities");
    let rust = serde_json::to_value(Capabilities::STRONGEST).unwrap();
    assert_eq!(
        fields.keys().cloned().collect::<BTreeSet<_>>(),
        rust.as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
    );
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

/// A struct type's fields in the compiled model, each with the declared type it names.
fn struct_fields(model: &Value, name: &str) -> BTreeMap<String, String> {
    let body = &model[name]["body"];
    assert_eq!(body["kind"], "struct", "{name}");
    body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| {
            (
                field["name"].as_str().expect("name").to_owned(),
                field["type_ref"]["name"]
                    .as_str()
                    .expect("a declared type")
                    .to_owned(),
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
