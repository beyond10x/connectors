//! Native-shaped CI responses for the production CLI's private HTTPS fixture.
use serde_json::{Value, json};
pub const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
pub fn reply(route: &str, path: &str, calls: &[String]) -> Option<(u16, Vec<u8>, &'static str)> {
    let prefix = "/api/v4/projects/org%2Fproject/";
    let route = route.strip_prefix(prefix)?;
    let pipeline =
        |id, status: &str| json!({"id":id,"project_id":7,"sha":SHA,"ref":"main","status":status});
    let job = |id| json!({"id":id,"pipeline":{"id":11,"sha":SHA},"name":"test","stage":"test","status":if id==42 {"failed"}else{"success"},"allow_failure":false});
    let (value, next): (Value, &str) = match route {
        "pipelines" => (json!([pipeline(11, "pending")]), "x-next-page: \r\n"),
        "pipelines/11" => {
            let count = calls
                .iter()
                .filter(|p| p.split('?').next().unwrap().ends_with("/pipelines/11"))
                .count();
            (
                pipeline(
                    11,
                    match count {
                        1 => "pending",
                        2 => "running",
                        _ => "failed",
                    },
                ),
                "",
            )
        }
        "pipelines/12" => {
            let mut value = pipeline(12, "success");
            value["sha"] = json!("f".repeat(40));
            (value, "")
        }
        "pipelines/11/jobs" if path.contains("page=2") => (json!([job(41)]), "x-next-page: \r\n"),
        "pipelines/11/jobs" => (json!([job(42)]), "x-next-page: 2\r\n"),
        "jobs/42" => (job(42), ""),
        "jobs/42/trace" => {
            return Some((
                200,
                "test failed: €\n".as_bytes().to_vec(),
                "Content-Type: text/plain\r\n",
            ));
        }
        "jobs/41/trace" => {
            let mut bytes = vec![b'a'; 511999];
            bytes.extend_from_slice("€\n".as_bytes());
            return Some((200, bytes, "Content-Type: text/plain\r\n"));
        }
        "jobs/999/trace" => return Some((404, b"erased fixture trace".to_vec(), "")),
        _ => return None,
    };
    Some((200, serde_json::to_vec(&value).unwrap(), next))
}
