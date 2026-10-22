//! Adversarial pass over wave 20261021: the Jira feed fixture (`tests/feed/jira/feed.json`) at
//! minute precision, against the ceiling `adapters/catalog/contracts/feed/v1alpha1/jira.md`
//! states ("Ceilings and cost": a resume answers `unavailable` when its 10 pages of 100 hold no
//! issue it has not returned, "about 1,000 issues changed in that window, as a bulk edit of that
//! many can"). A simulated Jira answers in the shapes of `tests/feed/jira/`; no live provider
//! is called.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path, sync::Mutex};

const INSTANCE: &str = "jira-feed-instance";
const BASE: &str = "/rest/api/3";
const PROJECT: &str = "10000";

type Call = (Vec<String>, Vec<(String, String)>);

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recorded(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join("tests/feed/jira").join(name)).unwrap())
        .unwrap()
}

fn engine() -> Engine {
    let feed: Declaration = serde_json::from_value(recorded("feed.json")).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    Engine::with_feed(&bundle, BASE, &[], Some(&feed)).unwrap()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Milliseconds since the epoch of the wall time `YYYY-MM-DD HH:MM` or `YYYY-MM-DD HH:MM:SS`
/// read in UTC; `None` for any other text.
fn wall(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = text.get(range)?;
        digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let shaped = bytes.len() >= 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b' '
        && bytes[13] == b':';
    if !shaped {
        return None;
    }
    let second = match bytes.len() {
        16 => 0,
        19 if bytes[16] == b':' => number(17..19)?,
        _ => return None,
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute) = (number(11..13)?, number(14..16)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    Some(((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60_000 + second * 1000)
}

/// `at` as Jira writes it in a zone `offset` minutes east of UTC:
/// `YYYY-MM-DDTHH:MM:SS.mmm±hhmm`.
fn jira_time(at: i64, offset: i64) -> String {
    let local = at + offset * 60_000;
    let (year, month, day) = civil_from_days(local.div_euclid(86_400_000));
    let rem = local.rem_euclid(86_400_000);
    let sign = if offset < 0 { '-' } else { '+' };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}{sign}{:02}{:02}",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000,
        offset.abs() / 60,
        offset.abs() % 60
    )
}

fn at(text: &str) -> i64 {
    wall(text).expect("a wall time")
}

#[derive(Clone, Copy)]
struct Issue {
    number: u64,
    created: i64,
    updated: i64,
}

/// Jira's issue search for one project as `tests/jira_feed.rs` simulates it, for a user whose
/// profile zone is `offset` minutes east of UTC; at most 100 issues an answer.
struct Zoned {
    offset: i64,
    issues: Vec<Issue>,
    calls: Mutex<Vec<Call>>,
}

impl Zoned {
    fn new(offset: i64, issues: Vec<Issue>) -> Self {
        Self {
            offset,
            issues,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn project(&self) -> Value {
        let mut project = recorded("project.json");
        project["id"] = json!(PROJECT);
        project["self"] = json!(format!(
            "https://tracker.example.test/rest/api/3/project/{PROJECT}"
        ));
        project["name"] = json!("Fixture Project");
        project["isPrivate"] = json!(false);
        project
    }

    fn issue(&self, issue: &Issue) -> Value {
        let mut record = recorded("issue.json");
        record["id"] = json!(issue.number.to_string());
        record["key"] = json!(format!("FIX-{}", issue.number - 10_000));
        record["self"] = json!(format!(
            "https://tracker.example.test/rest/api/3/issue/{}",
            issue.number
        ));
        record["fields"]["summary"] = json!(format!("issue {}", issue.number));
        record["fields"]["created"] = json!(jira_time(issue.created, self.offset));
        record["fields"]["updated"] = json!(jira_time(issue.updated, self.offset));
        record
    }

    fn parse(&self, jql: &str) -> Option<(String, Option<i64>)> {
        let rest = jql.strip_prefix("project = \"")?;
        let (project, rest) = rest.split_once('"')?;
        if let Some(rest) = rest.strip_prefix(" AND updated >= \"") {
            let (literal, rest) = rest.split_once('"')?;
            if rest != " ORDER BY updated ASC" || literal.len() != 16 {
                return None;
            }
            let since = wall(literal)? - self.offset * 60_000;
            return Some((project.to_owned(), Some(since)));
        }
        (rest == " ORDER BY updated ASC").then(|| (project.to_owned(), None))
    }

    fn answer(&self, path: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        match path {
            ["project", id] if *id == PROJECT => (200, self.project()),
            ["search", "jql"] => {
                let refused =
                    |message: &str| (400, json!({"errorMessages": [message], "errors": {}}));
                let Some((project, since)) = get("jql").and_then(|jql| self.parse(jql)) else {
                    return refused("The JQL query is not one this simulation answers.");
                };
                if project != PROJECT {
                    return refused("The value does not exist for the field 'project'.");
                }
                let Some(asked) = get("maxResults").map_or(Some(50), |n| n.parse::<usize>().ok())
                else {
                    return refused("maxResults is not a number.");
                };
                let start = match get("nextPageToken") {
                    None => 0,
                    Some(token) => match token.strip_prefix("page-").and_then(|n| n.parse().ok()) {
                        Some(start) => start,
                        None => return refused("The provided next page token is invalid."),
                    },
                };
                let mut listed: Vec<Issue> = self
                    .issues
                    .iter()
                    .filter(|issue| since.is_none_or(|since| issue.updated >= since))
                    .copied()
                    .collect();
                listed.sort_by_key(|issue| (issue.updated, issue.number));
                let end = listed.len().min(start + asked.min(100));
                let issues: Vec<Value> = listed[start.min(end)..end]
                    .iter()
                    .map(|issue| self.issue(issue))
                    .collect();
                let mut body = json!({"issues": issues, "isLast": end == listed.len()});
                if end < listed.len() {
                    body["nextPageToken"] = json!(format!("page-{end}"));
                }
                (200, body)
            }
            _ => (404, json!({"errorMessages": ["Not found"], "errors": {}})),
        }
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Zoned {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let mut recorded: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        recorded.sort();
        self.calls
            .lock()
            .unwrap()
            .push((path.iter().map(|s| s.to_string()).collect(), recorded));
        let (status, body) = self.answer(path, query);
        Ok(HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

/// A bulk edit of 400 issues within one UTC minute (09:00:00.000 to 09:00:39.900), well under
/// the "about 1,000 issues changed in that window" the Jira contract names as the point a resume
/// answers `unavailable`. Read 100 a call, every issue comes back exactly once and the last call
/// is `complete`; no call may answer `unavailable`, since a first read stops at the same issues
/// and would never get past them.
#[tokio::test]
async fn adversary_four_hundred_issues_changed_in_one_minute_are_all_returned() {
    let engine = engine();
    let start = at("2026-10-06 09:00:00");
    let issues: Vec<Issue> = (0..400)
        .map(|n| Issue {
            number: 20_000 + n,
            created: start + n as i64 * 100,
            updated: start + n as i64 * 100,
        })
        .collect();
    let jira = Zoned::new(0, issues);
    let mut watermark = Value::Null;
    let mut returned: Vec<String> = Vec::new();
    let mut reads = 0;
    loop {
        reads += 1;
        assert!(reads <= 10, "no complete page after {reads} reads");
        let mut input = json!({"container": PROJECT, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        let page = match engine.read(&jira, INSTANCE, ITEMS, input).await {
            Ok(page) => page,
            Err(error) => panic!(
                "read {reads}, after {} issues returned (watermark {} bytes): {:?} {:?}",
                returned.len(),
                watermark.as_str().map_or(0, str::len),
                error.code,
                error.message
            ),
        };
        returned.extend(
            page["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["id"].as_str().unwrap().to_owned()),
        );
        watermark = page["next_watermark"].clone();
        if page["complete"] == json!(true) {
            break;
        }
    }
    let unique: BTreeSet<&String> = returned.iter().collect();
    assert_eq!(unique.len(), 400, "issues returned: {}", returned.len());
    assert_eq!(returned.len(), 400, "an issue was returned twice");
}
