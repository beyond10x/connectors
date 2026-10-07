//! The GitLab catalog provider's `datasource.feed/v1alpha1` binding, held to the `feed-binding`
//! suite its profile's capabilities select.
//!
//! The binding is the `feed` of `adapters/catalog/providers/gitlab/operations.json`, realized by
//! the catalog engine over the committed GitLab bundle; its profile statement is
//! `adapters/catalog/contracts/feed/v1alpha1/gitlab.md`. Nothing here maps a field or forms a
//! watermark: [`GitLab`] only answers the requests the engine sends, from the suite's provider
//! state, the way GitLab answers them.
//!
//! [`GitLab`] is the remote service. A suite container is a project and a suite item one of its
//! merge requests, rendered over the recorded shapes in `adapters/catalog/tests/feed/gitlab/`.
//! It answers as GitLab does where the binding depends on it:
//! - `GET /projects` lists the member projects by id ascending only when asked (`order_by=id`,
//!   `sort=asc`; GitLab's default is newest first), continues after `id_after`, and without
//!   `membership=true` also lists a public project of the instance the user is not a member of;
//! - `GET /projects/:id` answers 404 for a project the user cannot see;
//! - `GET /projects/:id/merge_requests` filters `updated_after` inclusively and answers oldest
//!   `updated_at` first only when asked (`order_by=updated_at`, `sort=asc`);
//! - GitLab holds no direct conversation, so a suite container that is one is no project; and
//!   GitLab deletes a merge request outright, so a removed item is no longer listed.
//!
//! GitLab numbers projects and merge requests itself. [`Names`] keeps the provider's numbers for
//! the suite's ids, so the suite adapter names what the binding returned in the suite's terms. A
//! merge request carries no revision of its own: the binding's revision is its `updated_at`, and
//! the profile says so (`revision: update-time`), so the suite holds each revision it reads to
//! the instant the item took it.
use crate::{
    catalog_feed_conformance::claimed,
    feed_conformance::{
        self, Binding, Capabilities, Code, Container, ContainersPage, Item, ItemsPage, Source,
    },
};
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, feed::Declaration};
use connectors_core::{Error, ErrorCode};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use ess_conformance::CountStatus;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

const INSTANCE: &str = "gitlab-feed-instance";
/// The first number GitLab gives a project of the suite; a lower one is not the user's.
const FIRST_PROJECT: u64 = 1000;
/// A public project of the instance that the user is not a member of.
const ELSEWHERE: u64 = 7;

/// The scenarios the GitLab profile's capabilities select, of the 28 the family defines.
const RUN: u64 = 26;
/// The family's scenarios `revision: update-time` leaves out: each restores an item at an
/// instant it already carried another revision at, which an update-time revision cannot tell
/// apart.
const LEFT_OUT: [&str; 2] = [
    "connectors.feed.FeedItem/transition/restore/by/connectors.feed.RestoreItem/restored",
    "connectors.feed.RestoreItem/outcome/restored",
];

fn provider() -> PathBuf {
    feed_conformance::root().join("adapters/catalog")
}

fn recorded(name: &str) -> Value {
    let path = provider().join("tests/feed/gitlab").join(name);
    serde_json::from_slice(&std::fs::read(&path).expect("recorded shape")).unwrap()
}

fn shipped() -> Declaration {
    let raw: Value = serde_json::from_slice(
        &std::fs::read(provider().join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(raw["feed"].clone()).expect("the shipped selection declares a feed")
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..1000 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
    panic!("the engine waited on a provider that answers at once")
}

// ---- the provider's numbers ------------------------------------------------------------------

/// GitLab's numbers for the suite's ids, given in the order GitLab first answers them.
#[derive(Default)]
struct Names {
    projects: BTreeMap<String, u64>,
    merge_requests: BTreeMap<String, u64>,
}

impl Names {
    fn project(&mut self, container: &str) -> u64 {
        let next = FIRST_PROJECT + self.projects.len() as u64;
        *self.projects.entry(container.to_owned()).or_insert(next)
    }

    fn merge_request(&mut self, item: &str) -> u64 {
        let next = 1 + self.merge_requests.len() as u64;
        *self.merge_requests.entry(item.to_owned()).or_insert(next)
    }

    fn container_of(&self, project: &str) -> Option<String> {
        self.projects
            .iter()
            .find(|(_, number)| number.to_string() == project)
            .map(|(id, _)| id.clone())
    }

    fn item_of(&self, iid: &str) -> Option<String> {
        self.merge_requests
            .iter()
            .find(|(_, number)| number.to_string() == iid)
            .map(|(id, _)| id.clone())
    }
}

// ---- the simulated GitLab --------------------------------------------------------------------

struct GitLab<'a> {
    source: &'a Source,
    names: &'a Mutex<Names>,
}

impl GitLab<'_> {
    fn project(number: u64, container: Option<&Container>) -> Value {
        let mut project = recorded("project.json");
        project["id"] = json!(number);
        match container {
            Some(container) => {
                // GitLab names every project; the suite may not.
                project["name_with_namespace"] = json!(
                    container
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Group / {number}"))
                );
                project["visibility"] = json!(container.visibility);
            }
            None => {
                project["name_with_namespace"] = json!("Elsewhere / Public");
                project["visibility"] = json!("public");
            }
        }
        project
    }

    /// A member project for each suite container that GitLab can hold: every one that is not a
    /// direct conversation.
    fn member_projects(&self) -> Vec<(u64, &Container)> {
        let mut names = self.names.lock().unwrap();
        self.source
            .containers
            .iter()
            .filter(|(_, container)| container.visibility != "direct")
            .map(|(id, container)| (names.project(id), container))
            .collect()
    }

    fn container(&self, project: &str) -> Option<(&String, &Container)> {
        let id = self.names.lock().unwrap().container_of(project)?;
        self.source
            .containers
            .get_key_value(&id)
            .filter(|(_, container)| container.visibility != "direct")
    }

    fn answer(&self, segments: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        let per_page = match get("per_page").map(str::parse::<usize>) {
            None => 20,
            Some(Ok(n)) if (1..=100).contains(&n) => n,
            Some(_) => return (400, json!({"error": "per_page is invalid"})),
        };
        match segments {
            ["projects"] => {
                let mut projects: Vec<(u64, Option<&Container>)> = self
                    .member_projects()
                    .into_iter()
                    .map(|(number, container)| (number, Some(container)))
                    .collect();
                if get("membership") != Some("true") {
                    projects.push((ELSEWHERE, None));
                }
                if let Some(after) = get("id_after") {
                    let Ok(after) = after.parse::<u64>() else {
                        return (400, json!({"error": "id_after is invalid"}));
                    };
                    projects.retain(|(number, _)| *number > after);
                }
                projects.sort_by_key(|(number, _)| *number);
                if !(get("order_by") == Some("id") && get("sort") == Some("asc")) {
                    projects.reverse();
                }
                projects.truncate(per_page);
                let page: Vec<Value> = projects
                    .into_iter()
                    .map(|(number, container)| Self::project(number, container))
                    .collect();
                (200, json!(page))
            }
            ["projects", project] => match self.container(project) {
                Some((_, container)) => {
                    let number = project.parse::<u64>().expect("a project number");
                    (200, Self::project(number, Some(container)))
                }
                None => (404, json!({"message": "404 Project Not Found"})),
            },
            ["projects", project, "merge_requests"] => {
                let Some((container, _)) = self.container(project) else {
                    return (404, json!({"message": "404 Project Not Found"}));
                };
                let since = get("updated_after");
                let mut names = self.names.lock().unwrap();
                let mut listed: Vec<(u64, &String, &Item)> = self
                    .source
                    .items
                    .iter()
                    // GitLab deletes a merge request outright: it is no longer listed.
                    .filter(|(_, item)| &item.container == container && !item.deleted)
                    .filter(|(_, item)| since.is_none_or(|since| item.updated_at.as_str() >= since))
                    .map(|(id, item)| (names.merge_request(id), id, item))
                    .collect();
                if get("order_by") == Some("updated_at") && get("sort") == Some("asc") {
                    listed.sort_by(|a, b| (&a.2.updated_at, a.0).cmp(&(&b.2.updated_at, b.0)));
                } else {
                    listed.sort_by(|a, b| (&b.2.created_at, b.0).cmp(&(&a.2.created_at, a.0)));
                }
                listed.truncate(per_page);
                let project = project.parse::<u64>().expect("a project number");
                let page: Vec<Value> = listed
                    .into_iter()
                    .map(|(iid, id, item)| {
                        let mut record = recorded("merge_request.json");
                        record["iid"] = json!(iid);
                        record["id"] = json!(50_000 + iid);
                        record["project_id"] = json!(project);
                        record["created_at"] = json!(item.created_at);
                        record["updated_at"] = json!(item.updated_at);
                        record["description"] = json!(format!("{id} at {}", item.revision));
                        record
                    })
                    .collect();
                (200, json!(page))
            }
            _ => (404, json!({"error": "404 Not Found"})),
        }
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for GitLab<'_> {
    async fn get(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<HttpResponse> {
        let (status, body) = self.answer(segments, query);
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

// ---- the binding under test ------------------------------------------------------------------

/// A GitLab declaration, realized by the catalog engine over the committed bundle.
struct Shipped {
    engine: Engine,
    profile: String,
    capabilities: Capabilities,
    names: Mutex<Names>,
}

impl Shipped {
    fn new(feed: &Declaration) -> Self {
        let root = provider();
        let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").expect("bundle");
        let engine = Engine::with_feed(&bundle, "/api/v4", &[], Some(feed)).expect("engine");
        Self {
            engine,
            profile: feed.profile.clone(),
            capabilities: claimed(feed),
            names: Mutex::default(),
        }
    }

    fn call(&self, source: &Source, operation: &str, input: Value) -> Result<Value, Error> {
        let http = GitLab {
            source,
            names: &self.names,
        };
        let result = block_on(self.engine.read(&http, INSTANCE, operation, input));
        if let Ok(output) = &result {
            let declaration = self
                .engine
                .declarations(&[Effect::Read])
                .into_iter()
                .find(|o| o.id == operation)
                .expect("declared");
            let validator = jsonschema::validator_for(&declaration.output_schema).unwrap();
            let errors: Vec<String> = validator
                .iter_errors(output)
                .map(|e| format!("{}: {e}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{operation}: {errors:?}\n{output:#}");
        }
        result
    }
}

fn code(error: Error) -> Code {
    match error.code {
        ErrorCode::InvalidInput => Code::InvalidInput,
        ErrorCode::NotFound => Code::NotFound,
        ErrorCode::StaleCursor => Code::StaleCursor,
        other => Code::Other(format!("{other:?}: {}", error.message)),
    }
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

impl Binding for Shipped {
    fn identity(&self) -> (String, String) {
        (
            "connectors-catalog-feed-gitlab".into(),
            self.profile.clone(),
        )
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code> {
        let mut input = json!({});
        if let Some(limit) = limit {
            input["limit"] = json!(limit);
        }
        if let Some(cursor) = cursor {
            input["cursor"] = json!(cursor);
        }
        let page = self.call(source, "feed.containers", input).map_err(code)?;
        let names = self.names.lock().unwrap();
        let containers = page["containers"]
            .as_array()
            .expect("containers")
            .iter()
            .map(|c| {
                let project = text(&c["id"]).expect("id");
                // A project the suite never held keeps GitLab's own number. The suite's views
                // hold what a listing contains and excludes, not that it holds nothing more, so
                // the membership filter is checked by the exact requests in
                // `adapters/catalog/tests/gitlab_feed.rs`, not here.
                let id = names.container_of(&project).unwrap_or(project);
                (
                    id,
                    Container {
                        connection: String::new(),
                        name: text(&c["name"]),
                        kind: text(&c["kind"]).expect("kind"),
                        visibility: text(&c["visibility"]).expect("visibility"),
                    },
                )
            })
            .collect();
        Ok(ContainersPage {
            containers,
            next_cursor: text(&page["next_cursor"]),
            complete: page["complete"].as_bool().expect("complete"),
        })
    }

    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        let project = self.names.lock().unwrap().project(container);
        let mut input = json!({"container": project.to_string(), "limit": limit});
        if let Some(watermark) = watermark {
            input["watermark"] = json!(watermark);
        }
        let page = self.call(source, "feed.items", input).map_err(code)?;
        let names = self.names.lock().unwrap();
        let items = page["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|i| {
                let iid = text(&i["id"]).expect("id");
                (
                    names.item_of(&iid).unwrap_or(iid),
                    Item {
                        container: container.to_owned(),
                        revision: text(&i["revision"]).expect("revision"),
                        created_at: text(&i["created_at"]).expect("created_at"),
                        updated_at: text(&i["updated_at"]).expect("updated_at"),
                        parent: text(&i["parent"]),
                        deleted: i["deleted"].as_bool().expect("deleted"),
                        changed: 0,
                    },
                )
            })
            .collect();
        Ok(ItemsPage {
            items,
            next_watermark: text(&page["next_watermark"]).expect("next_watermark"),
            complete: page["complete"].as_bool().expect("complete"),
        })
    }
}

// ---- the cases -------------------------------------------------------------------------------

/// The shipped declaration passes every scenario its four capabilities select, and the run names
/// the ones it leaves out: 26 of the family's 28, the two left out by `revision: update-time`.
/// Each of the four capabilities changed at least one expectation, so each claim was exercised.
#[test]
fn the_gitlab_feed_binding_passes_the_scenarios_its_profile_declares() {
    let run = feed_conformance::run(Shipped::new(&shipped()));
    println!("{}", run.summary());
    assert!(run.accounts_for_the_family(), "{}", run.summary());
    assert_eq!(
        run.status,
        CountStatus::Passed,
        "{:?}\n{}\n{}",
        run.failed,
        run.summary(),
        run.report
    );
    assert_eq!(run.counts.passed, run.counts.total, "{}", run.report);
    assert_eq!(run.counts.total, RUN, "{}", run.summary());
    assert_eq!(
        run.left_out_ids(),
        BTreeSet::from(LEFT_OUT),
        "{}",
        run.summary()
    );
    for left in &run.left_out {
        let by: Vec<&str> = left.by.iter().map(|weaker| weaker.name()).collect();
        assert_eq!(by, ["revision: update-time"], "{}", left.scenario);
    }
    let changed_by: BTreeSet<&str> = run.changed.iter().map(|c| c.by.name()).collect();
    assert_eq!(
        changed_by,
        BTreeSet::from([
            "deletions: not-observed",
            "kind: fixed-word",
            "revision: update-time",
            "visibility: all-private",
        ]),
        "{}",
        run.summary()
    );
}
