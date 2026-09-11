//! Fictional native MR responses served over the disposable authenticated HTTPS boundary.
use serde_json::{Value, json};
pub fn reply(route: &str, path: &str, calls: &[String]) -> Option<(u16, Vec<u8>, &'static str)> {
    let route = route.strip_prefix("/api/v4/projects/org%2Fproject/merge_requests")?;
    let item = |iid| json!({"id":1000+iid,"iid":iid,"project_id":7,"source_project_id":null,"target_project_id":7,"title":"Fixture change","description":null,"source_branch":"fix","target_branch":"main","state":"opened","draft":false,"detailed_merge_status":"future_status","sha":null,"merge_commit_sha":null,"squash_commit_sha":null,"updated_at":"2026-09-02T00:00:00Z"});
    let (value, next): (Value, &str) = match route {
        "" if path.contains("page=2") => (json!([item(2)]), "x-next-page: \r\n"),
        "" => (json!([item(1)]), "x-next-page: 2\r\n"),
        "/1" => (item(1), ""),
        "/2" => (item(2), ""),
        "/3" | "/4" => {
            let sha = "0123456789abcdef0123456789abcdef01234567";
            let mut value = item(if route == "/3" { 3 } else { 4 });
            value["detailed_merge_status"] = json!("mergeable");
            value["sha"] =
                if route == "/3" && calls.iter().filter(|p| p.as_str() == path).count() > 1 {
                    json!("a".repeat(40))
                } else {
                    json!(sha)
                };
            value["head_pipeline"] = json!({"id":12,"project_id":7,"sha":sha,"status":"success"});
            (value, "")
        }
        "/999" => return Some((404, b"missing fixture MR".to_vec(), "")),
        _ => return None,
    };
    Some((200, serde_json::to_vec(&value).unwrap(), next))
}
