//! Sustained live read loop: run a declared set of reads through the built `connectors`
//! binary for a bounded time, with no revalidation and no other step by the caller, and
//! report every call's outcome, latency and refusal.
//!
//! The loop drives the `connectors` command line as a subprocess, not `connectors-client`:
//! the failures it exists to find (an expired connection lease, a stale provider binary
//! after an upgrade, a provider `ok:false` answered as success, a refusal that names no
//! field) live in the local CLI, its owner and the host, which a client of one adapter
//! service bypasses, and a downstream consumer calls the command line too.
//!
//! The reads are declared in a `connectors-live-read-loop/1` file the operator keeps outside
//! the repository, since it names live connections. Each read is resolved once with
//! `operations describe`, then invoked each round with `invoke`; those are the
//! only commands the loop issues. The report records no input, no result and no output
//! bytes besides a refusal's `error` object, and it is refused inside the repository.
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The declaration format this loop reads.
pub const FORMAT: &str = "connectors-live-read-loop/1";
/// The report format this loop writes.
pub const REPORT_FORMAT: &str = "connectors-live-read-loop-report/1";

#[derive(clap::Args, Debug)]
pub struct Options {
    /// The `connectors-live-read-loop/1` declaration of the reads to run.
    #[arg(long)]
    pub reads: PathBuf,
    /// Where to write the report: a new file outside the repository.
    #[arg(long)]
    pub report: PathBuf,
    /// The `connectors` binary to run.
    #[arg(long, default_value = "connectors")]
    pub connectors: PathBuf,
    /// Passed to `connectors` as `--config`.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Passed to `connectors` as `--state-dir`.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
    /// How long to keep starting rounds; a round in progress finishes.
    #[arg(long, default_value_t = 600)]
    pub duration_seconds: u64,
    /// The pause between rounds.
    #[arg(long, default_value_t = 10)]
    pub pause_seconds: u64,
    /// A call that has not exited by then is killed and counted unanswered.
    #[arg(long, default_value_t = 60)]
    pub call_timeout_seconds: u64,
}

/// One declared read.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub name: String,
    pub adapter: String,
    pub connection: String,
    pub operation: String,
    #[serde(default = "empty_object")]
    pub input: Value,
}

fn empty_object() -> Value {
    json!({})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    format: String,
    reads: Vec<Read>,
}

/// Parse and check a declaration: its format, at least one read, unique non-empty names,
/// non-empty adapter, connection and operation, and an object input.
pub fn declaration(bytes: &[u8]) -> Result<Vec<Read>> {
    let declaration: Declaration = serde_json::from_slice(bytes)?;
    if declaration.format != FORMAT {
        return Err(format!(
            "read loop declaration format is {:?}, not {FORMAT:?}",
            declaration.format
        )
        .into());
    }
    if declaration.reads.is_empty() {
        return Err("read loop declaration names no reads".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for read in &declaration.reads {
        for (field, value) in [
            ("name", &read.name),
            ("adapter", &read.adapter),
            ("connection", &read.connection),
            ("operation", &read.operation),
        ] {
            if value.trim().is_empty() {
                return Err(format!("read {:?} has an empty {field}", read.name).into());
            }
        }
        if !read.input.is_object() {
            return Err(format!("read {:?} input is not a JSON object", read.name).into());
        }
        if !names.insert(read.name.as_str()) {
            return Err(format!("read name {:?} is declared twice", read.name).into());
        }
    }
    Ok(declaration.reads)
}

/// What one `connectors` run left behind, before it is judged.
#[derive(Clone, Debug, Default)]
pub struct Raw {
    pub exit: Option<i32>,
    pub stdout: Vec<u8>,
    pub timed_out: bool,
    pub spawn_error: Option<String>,
}

/// Runs `connectors` with the given arguments (after its global options).
pub trait Caller {
    fn call(&mut self, arguments: &[String]) -> Raw;
}

/// A call's verdict.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Answered(Value),
    Unanswered(Value),
}

/// Judge one run: answered only on exit 0 with `{"ok":true}` and, where the result carries
/// a provider `status`, a 2xx one whose body is not `{"ok":false}`. Otherwise the refusal is the envelope's `error` object,
/// or a description of what the run did instead.
pub fn classify(raw: &Raw) -> Outcome {
    if let Some(message) = &raw.spawn_error {
        return Outcome::Unanswered(json!({"kind":"spawn","message":message}));
    }
    if raw.timed_out {
        return Outcome::Unanswered(json!({"kind":"timeout"}));
    }
    let unparsed = || Outcome::Unanswered(json!({"kind":"unparsed","exit":raw.exit}));
    let Ok(envelope) = serde_json::from_slice::<Value>(raw.stdout.trim_ascii()) else {
        return unparsed();
    };
    match envelope.get("ok") {
        Some(Value::Bool(false)) => match envelope.get("error") {
            Some(error @ Value::Object(_)) => Outcome::Unanswered(error.clone()),
            _ => unparsed(),
        },
        Some(Value::Bool(true)) => {
            if raw.exit != Some(0) {
                return Outcome::Unanswered(json!({"kind":"exit","exit":raw.exit}));
            }
            let result = envelope.get("result").cloned().unwrap_or(Value::Null);
            if let Some(status) = result.pointer("/result/status").and_then(Value::as_u64)
                && !(200..300).contains(&status)
            {
                return Outcome::Unanswered(json!({"kind":"provider_status","status":status}));
            }
            // A provider that refuses inside a 2xx body (Slack's `{"ok":false}`) has not
            // answered; only the status is reported, never the body.
            if result.pointer("/result/body/ok") == Some(&Value::Bool(false)) {
                let status = result.pointer("/result/status").cloned();
                return Outcome::Unanswered(json!({"kind":"provider_ok_false","status":status}));
            }
            Outcome::Answered(result)
        }
        _ => unparsed(),
    }
}

/// How long the loop runs.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub duration: Duration,
    pub pause: Duration,
}

#[derive(Clone, Debug, Serialize)]
pub struct CallRecord {
    pub read: String,
    pub phase: &'static str,
    pub round: u64,
    pub started_ms: u64,
    pub latency_ms: u64,
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReadSummary {
    pub name: String,
    pub adapter: String,
    pub operation: String,
    pub calls: u64,
    pub answered: u64,
    pub unanswered: u64,
    pub max_latency_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub format: &'static str,
    pub duration_ms: u64,
    pub elapsed_ms: u64,
    pub rounds: u64,
    pub calls: u64,
    pub answered: u64,
    pub unanswered: u64,
    /// Calls the loop made to `connections revalidate`; the loop issues none.
    pub revalidations: u64,
    /// The distinct commands the loop issued, such as `invoke`.
    pub commands: Vec<String>,
    pub reads: Vec<ReadSummary>,
    pub records: Vec<CallRecord>,
}

impl Report {
    /// Every call answered, every read answered at least once, no revalidation.
    pub fn passed(&self) -> bool {
        self.unanswered == 0
            && self.revalidations == 0
            && !self.reads.is_empty()
            && self.reads.iter().all(|read| read.answered > 0)
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Run the loop over `reads` until `settings.duration` has passed. Every round invokes each
/// read once; a read whose operation is not yet resolved is described first, and a failed
/// describe is retried in the next round. A round that has started finishes, so a run always
/// has at least one round.
pub fn drive(reads: &[Read], caller: &mut dyn Caller, settings: Settings) -> Report {
    let started = Instant::now();
    let mut resolved: Vec<Option<(String, String, String)>> = vec![None; reads.len()];
    let mut summaries: Vec<ReadSummary> = reads
        .iter()
        .map(|read| ReadSummary {
            name: read.name.clone(),
            adapter: read.adapter.clone(),
            operation: read.operation.clone(),
            calls: 0,
            answered: 0,
            unanswered: 0,
            max_latency_ms: 0,
        })
        .collect();
    let mut records = Vec::new();
    let mut commands: Vec<String> = Vec::new();
    let mut revalidations = 0;
    let mut round = 0;
    loop {
        round += 1;
        for (index, read) in reads.iter().enumerate() {
            let phases: &[&'static str] = if resolved[index].is_some() {
                &["invoke"]
            } else {
                &["describe", "invoke"]
            };
            for &phase in phases {
                let arguments = match (phase, &resolved[index]) {
                    ("describe", _) => vec![
                        "operations".into(),
                        "describe".into(),
                        "--adapter".into(),
                        read.adapter.clone(),
                        "--operation".into(),
                        read.operation.clone(),
                    ],
                    (_, Some((target, schema, revision))) => vec![
                        "invoke".into(),
                        target.clone(),
                        read.operation.clone(),
                        "--connection".into(),
                        read.connection.clone(),
                        "--schema".into(),
                        schema.clone(),
                        "--revision".into(),
                        revision.clone(),
                        "--input-json".into(),
                        read.input.to_string(),
                    ],
                    // The describe in this round failed: the read is described again next round.
                    (_, None) => break,
                };
                // `invoke` names its target where a group names its command.
                let command = match arguments[0].as_str() {
                    "invoke" => arguments[0].clone(),
                    _ => arguments[..2].join(" "),
                };
                if command == "connections revalidate" {
                    revalidations += 1;
                }
                if !commands.contains(&command) {
                    commands.push(command);
                }
                let at = Instant::now();
                let raw = caller.call(&arguments);
                let latency_ms = millis(at.elapsed());
                let mut outcome = classify(&raw);
                if phase == "describe"
                    && let Outcome::Answered(result) = &outcome
                {
                    let text = |field: &str| result.get(field).and_then(Value::as_str);
                    // `invoke` names the instance the describe answered for
                    // (story:connectors-do-short-invocation).
                    match (
                        text("connector"),
                        text("instance"),
                        text("schema"),
                        text("revision"),
                    ) {
                        (Some(connector), Some(instance), Some(schema), Some(revision)) => {
                            resolved[index] = Some((
                                format!("{connector}/{instance}"),
                                schema.to_string(),
                                revision.to_string(),
                            ));
                        }
                        _ => {
                            outcome = Outcome::Unanswered(json!({
                                "kind":"describe_incomplete",
                                "missing":["connector","instance","schema","revision"]
                            }));
                        }
                    }
                }
                let summary = &mut summaries[index];
                summary.calls += 1;
                summary.max_latency_ms = summary.max_latency_ms.max(latency_ms);
                let (outcome, refusal) = match outcome {
                    Outcome::Answered(_) => {
                        summary.answered += 1;
                        ("answered", None)
                    }
                    Outcome::Unanswered(refusal) => {
                        summary.unanswered += 1;
                        ("unanswered", Some(refusal))
                    }
                };
                records.push(CallRecord {
                    read: read.name.clone(),
                    phase,
                    round,
                    started_ms: millis(at.duration_since(started)),
                    latency_ms,
                    outcome,
                    refusal,
                });
            }
        }
        if started.elapsed() >= settings.duration {
            break;
        }
        std::thread::sleep(settings.pause);
        if started.elapsed() >= settings.duration {
            break;
        }
    }
    let answered = summaries.iter().map(|s| s.answered).sum();
    let unanswered = summaries.iter().map(|s| s.unanswered).sum();
    Report {
        format: REPORT_FORMAT,
        duration_ms: millis(settings.duration),
        elapsed_ms: millis(started.elapsed()),
        rounds: round,
        calls: records.len() as u64,
        answered,
        unanswered,
        revalidations,
        commands,
        reads: summaries,
        records,
    }
}

/// The `connectors` binary as a subprocess: stdin and stderr are null, stdout is captured.
/// A call still running at `timeout` is killed (only that process, never its process group,
/// so an owner it started is left to the loop's next call) and counted timed out.
pub struct Process {
    pub program: PathBuf,
    pub globals: Vec<String>,
    pub timeout: Duration,
}

impl Caller for Process {
    fn call(&mut self, arguments: &[String]) -> Raw {
        use std::io::Read as _;
        use std::process::{Command, Stdio};
        let deadline = Instant::now() + self.timeout;
        let mut child = match Command::new(&self.program)
            .args(&self.globals)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return Raw {
                    spawn_error: Some(error.to_string()),
                    ..Raw::default()
                };
            }
        };
        // Read stdout on its own thread, so that a full pipe cannot stall the child and a
        // descendant holding the pipe open cannot hold the loop past the deadline.
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            let _ = sender.send(bytes);
        });
        let timed_out = |mut child: std::process::Child| {
            let _ = child.kill();
            let _ = child.wait();
            Raw {
                timed_out: true,
                ..Raw::default()
            }
        };
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => return timed_out(child),
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Raw {
                        spawn_error: Some(error.to_string()),
                        ..Raw::default()
                    };
                }
            }
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(remaining) {
            Ok(stdout) => Raw {
                exit: status.code(),
                stdout,
                ..Raw::default()
            },
            Err(_) => Raw {
                timed_out: true,
                ..Raw::default()
            },
        }
    }
}

/// Run the loop as `options` says and write its report; fails when any read went unanswered,
/// any read was never answered, or the loop revalidated. The report must be a new file
/// outside `root`; both are checked before the first call.
pub fn run(root: &Path, options: Options) -> Result<()> {
    let reads = declaration(&std::fs::read(&options.reads)?)?;
    let report_path = report_destination(root, &options.report)?;
    let mut globals = Vec::new();
    if let Some(config) = &options.config {
        globals.extend(["--config".to_string(), config.display().to_string()]);
    }
    if let Some(state_dir) = &options.state_dir {
        globals.extend(["--state-dir".to_string(), state_dir.display().to_string()]);
    }
    globals.extend(["--output".to_string(), "json".to_string()]);
    let mut process = Process {
        program: options.connectors.clone(),
        globals,
        timeout: Duration::from_secs(options.call_timeout_seconds),
    };
    let settings = Settings {
        duration: Duration::from_secs(options.duration_seconds),
        pause: Duration::from_secs(options.pause_seconds),
    };
    let report = drive(&reads, &mut process, settings);
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&report_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    println!(
        "live read loop: {} rounds, {} calls, {} answered, {} unanswered, {} revalidations; report {}",
        report.rounds,
        report.calls,
        report.answered,
        report.unanswered,
        report.revalidations,
        report_path.display()
    );
    if report.passed() {
        Ok(())
    } else {
        let never: Vec<&str> = report
            .reads
            .iter()
            .filter(|read| read.answered == 0)
            .map(|read| read.name.as_str())
            .collect();
        Err(format!(
            "live read loop failed: {} unanswered of {} calls, {} revalidations, never answered: {never:?}",
            report.unanswered, report.calls, report.revalidations
        )
        .into())
    }
}

/// The report path, absolute, after checking it does not exist and does not lie in `root`.
fn report_destination(root: &Path, report: &Path) -> Result<PathBuf> {
    if report.symlink_metadata().is_ok() {
        return Err(format!("report {} already exists", report.display()).into());
    }
    let name = report
        .file_name()
        .ok_or_else(|| format!("report {} names no file", report.display()))?;
    let parent = match report.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let parent = parent.canonicalize()?;
    if parent.starts_with(root.canonicalize()?) {
        return Err(format!(
            "report {} is inside the repository; write it outside, since it names live connections",
            report.display()
        )
        .into());
    }
    Ok(parent.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn read(name: &str, input: Value) -> Read {
        Read {
            name: name.into(),
            adapter: format!("{name}-adapter"),
            connection: format!("conn_{name}"),
            operation: format!("{name}.list"),
            input,
        }
    }

    fn envelope(value: Value) -> Raw {
        let ok = value["ok"] == json!(true);
        Raw {
            exit: Some(if ok { 0 } else { 1 }),
            stdout: serde_json::to_vec(&value).unwrap(),
            ..Raw::default()
        }
    }

    /// A fake `connectors`: describes every operation as schema `s-<adapter>` at revision
    /// `r1`, answers every invoke with a 200 and a body, except where `refuse` names the
    /// read's adapter and the invoke's ordinal (1-based, per adapter).
    #[derive(Default)]
    struct Fake {
        argv: Vec<Vec<String>>,
        invokes: BTreeMap<String, u64>,
        refuse: Vec<(String, u64, Value)>,
    }

    fn flag<'a>(arguments: &'a [String], name: &str) -> &'a str {
        let at = arguments.iter().position(|a| a == name).unwrap();
        &arguments[at + 1]
    }

    impl Caller for Fake {
        fn call(&mut self, arguments: &[String]) -> Raw {
            self.argv.push(arguments.to_vec());
            match (arguments[0].as_str(), arguments[1].as_str()) {
                ("operations", "describe") => {
                    let adapter = flag(arguments, "--adapter");
                    envelope(
                        json!({"ok":true,"result":{"adapter":adapter,"connector":"fixture","instance":adapter,"schema":format!("s-{adapter}"),"revision":"r1"}}),
                    )
                }
                ("invoke", target) => {
                    let adapter = target.strip_prefix("fixture/").unwrap().to_string();
                    let count = self.invokes.entry(adapter.clone()).or_default();
                    *count += 1;
                    if let Some((_, _, error)) = self
                        .refuse
                        .iter()
                        .find(|(a, n, _)| *a == adapter && *n == *count)
                    {
                        return envelope(json!({"ok":false,"error":error}));
                    }
                    envelope(
                        json!({"ok":true,"result":{"result":{"status":200,"body":{"ok":true,"text":"result-bytes-never-reported"}}}}),
                    )
                }
                _ => Raw {
                    exit: Some(2),
                    ..Raw::default()
                },
            }
        }
    }

    fn short() -> Settings {
        Settings {
            duration: Duration::from_millis(150),
            pause: Duration::from_millis(10),
        }
    }

    #[test]
    fn a_short_loop_answers_every_read_with_no_revalidate() {
        let reads = [read("slack", json!({})), read("jira", json!({"limit":5}))];
        let mut fake = Fake::default();
        let report = drive(&reads, &mut fake, short());
        assert!(report.rounds >= 2, "rounds: {}", report.rounds);
        assert_eq!(report.unanswered, 0);
        assert_eq!(report.answered, report.calls);
        assert_eq!(report.calls, fake.argv.len() as u64);
        assert_eq!(report.revalidations, 0);
        assert!(report.passed());
        assert_eq!(
            report.commands,
            ["operations describe", "invoke"].map(String::from)
        );
        for argv in &fake.argv {
            assert!(!argv.iter().any(|a| a.contains("revalidate")), "{argv:?}");
        }
        // Described once per read, then invoked every round with what describe answered:
        // its instance as the target (story:connectors-do-short-invocation).
        let describes = fake.argv.iter().filter(|a| a[1] == "describe").count();
        assert_eq!(describes, 2);
        let invokes: Vec<_> = fake.argv.iter().filter(|a| a[0] == "invoke").collect();
        assert_eq!(invokes.len() as u64, 2 * report.rounds);
        let jira = invokes
            .iter()
            .find(|a| a[1] == "fixture/jira-adapter")
            .unwrap();
        assert_eq!(flag(jira, "--connection"), "conn_jira");
        assert_eq!(jira[2], "jira.list");
        assert_eq!(flag(jira, "--schema"), "s-jira-adapter");
        assert_eq!(flag(jira, "--revision"), "r1");
        assert_eq!(
            serde_json::from_str::<Value>(flag(jira, "--input-json")).unwrap(),
            json!({"limit":5})
        );
        let summary = &report.reads[1];
        assert_eq!(
            (summary.name.as_str(), summary.calls, summary.unanswered),
            ("jira", report.rounds + 1, 0)
        );
    }

    #[test]
    fn each_failure_is_reported_with_its_refusal() {
        let reads = [read("slack", json!({})), read("gitlab", json!({}))];
        let refusal = json!({"code":"failure","data":{"code":"unauthenticated","kind":"operational","next_action":"repair_connection","stage":"readiness"}});
        let mut fake = Fake {
            refuse: vec![("gitlab-adapter".into(), 2, refusal.clone())],
            ..Fake::default()
        };
        let report = drive(&reads, &mut fake, short());
        assert!(report.rounds >= 2);
        assert_eq!(report.unanswered, 1);
        assert!(!report.passed());
        let failed: Vec<_> = report
            .records
            .iter()
            .filter(|r| r.outcome == "unanswered")
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].read, "gitlab");
        assert_eq!(failed[0].phase, "invoke");
        assert_eq!(failed[0].round, 2);
        assert_eq!(failed[0].refusal.as_ref(), Some(&refusal));
        assert_eq!(report.reads[1].unanswered, 1);
        // The loop went on after the failure: the read is invoked again in later rounds.
        assert_eq!(report.reads[1].calls, report.rounds + 1);
    }

    #[test]
    fn a_failed_describe_is_retried_next_round_and_counted() {
        struct Flaky(u64);
        impl Caller for Flaky {
            fn call(&mut self, arguments: &[String]) -> Raw {
                if arguments[1] == "describe" {
                    self.0 += 1;
                    if self.0 == 1 {
                        return envelope(
                            json!({"ok":false,"error":{"code":"failure","data":{"code":"not_found"}}}),
                        );
                    }
                    return envelope(
                        json!({"ok":true,"result":{"connector":"k8s","instance":"main","schema":"s","revision":"r"}}),
                    );
                }
                envelope(json!({"ok":true,"result":{"result":{"status":200}}}))
            }
        }
        let report = drive(&[read("k8s", json!({}))], &mut Flaky(0), short());
        assert_eq!(report.unanswered, 1);
        assert_eq!(report.records[0].phase, "describe");
        assert_eq!(report.records[1].phase, "describe");
        assert_eq!(report.records[1].round, 2);
        assert_eq!(report.records[2].phase, "invoke");
        assert!(!report.passed());
    }

    #[test]
    fn a_call_is_answered_only_on_ok_true_with_a_2xx_status() {
        let answered = envelope(json!({"ok":true,"result":{"result":{"status":200}}}));
        assert!(matches!(classify(&answered), Outcome::Answered(_)));
        let native = envelope(json!({"ok":true,"result":{"rows":[]}}));
        assert!(matches!(classify(&native), Outcome::Answered(_)));
        let error = json!({"code":"failure","data":{"code":"invalid_input","field":"channel"}});
        assert_eq!(
            classify(&envelope(json!({"ok":false,"error":error}))),
            Outcome::Unanswered(error)
        );
        assert_eq!(
            classify(&envelope(
                json!({"ok":true,"result":{"result":{"status":429}}})
            )),
            Outcome::Unanswered(json!({"kind":"provider_status","status":429}))
        );
        let mismatch = Raw {
            exit: Some(1),
            ..envelope(json!({"ok":true,"result":{}}))
        };
        assert_eq!(
            classify(&mismatch),
            Outcome::Unanswered(json!({"kind":"exit","exit":1}))
        );
        let garbage = Raw {
            exit: Some(0),
            stdout: b"password: hunter2\n".to_vec(),
            ..Raw::default()
        };
        assert_eq!(
            classify(&garbage),
            Outcome::Unanswered(json!({"kind":"unparsed","exit":0}))
        );
        let timed_out = Raw {
            timed_out: true,
            ..Raw::default()
        };
        assert_eq!(
            classify(&timed_out),
            Outcome::Unanswered(json!({"kind":"timeout"}))
        );
        let missing = Raw {
            spawn_error: Some("No such file or directory".into()),
            ..Raw::default()
        };
        assert_eq!(
            classify(&missing),
            Outcome::Unanswered(json!({"kind":"spawn","message":"No such file or directory"}))
        );
    }

    /// A provider that answers HTTP 200 with `{"ok":false}` (Slack's way of refusing) has not
    /// answered the read, even when the command line reports success.
    #[test]
    fn a_provider_ok_false_body_is_unanswered() {
        let refused = envelope(json!({"ok":true,"result":{"result":{"status":200,
            "body":{"ok":false,"error":"not_in_channel","token":"xoxb-secret"}}}}));
        assert_eq!(
            classify(&refused),
            Outcome::Unanswered(json!({"kind":"provider_ok_false","status":200}))
        );
        let answered =
            envelope(json!({"ok":true,"result":{"result":{"status":200,"body":{"ok":true}}}}));
        assert!(matches!(classify(&answered), Outcome::Answered(_)));
    }

    #[test]
    fn the_declaration_is_checked() {
        let good = br#"{"format":"connectors-live-read-loop/1","reads":[
            {"name":"slack","adapter":"slack-bot","connection":"conn_a","operation":"conversations.list"},
            {"name":"jira","adapter":"jira","connection":"conn_b","operation":"issue.search","input":{"jql":"x"}}]}"#;
        let reads = declaration(good).unwrap();
        assert_eq!(reads.len(), 2);
        assert_eq!(reads[0].input, json!({}));
        assert_eq!(reads[1].input, json!({"jql":"x"}));
        for (bad, why) in [
            (r#"{"format":"other/1","reads":[]}"#, "format"),
            (
                r#"{"format":"connectors-live-read-loop/1","reads":[]}"#,
                "no reads",
            ),
            (
                r#"{"format":"connectors-live-read-loop/1","reads":[{"name":"a","adapter":"x","connection":"c","operation":"o"},{"name":"a","adapter":"y","connection":"c","operation":"o"}]}"#,
                "duplicate name",
            ),
            (
                r#"{"format":"connectors-live-read-loop/1","reads":[{"name":"a","adapter":"x","connection":"c","operation":"o","schema":"s"}]}"#,
                "unknown field",
            ),
            (
                r#"{"format":"connectors-live-read-loop/1","reads":[{"name":"a","adapter":"","connection":"c","operation":"o"}]}"#,
                "empty adapter",
            ),
            (
                r#"{"format":"connectors-live-read-loop/1","reads":[{"name":"a","adapter":"x","connection":"c","operation":"o","input":[1]}]}"#,
                "input not an object",
            ),
        ] {
            assert!(declaration(bad.as_bytes()).is_err(), "{why} accepted");
        }
    }

    /// A fake `connectors` executable in `directory`, a shell script and so Unix only:
    /// logs its arguments to `argv.log`, answers `operations describe` with the instance
    /// `fixture/main`, schema `s1` and revision `r1`, runs the shell command `invoke`
    /// for `invoke` and exits 3 on anything else.
    #[cfg(unix)]
    fn fake_binary(directory: &Path, invoke: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = directory.join("connectors");
        let script = format!(
            "#!/bin/sh\necho \"$@\" >> \"$(dirname \"$0\")/argv.log\"\ncase \"$*\" in\n  \
             *\"operations describe\"*) echo '{{\"ok\":true,\"result\":{{\"connector\":\"fixture\",\"instance\":\"main\",\"schema\":\"s1\",\"revision\":\"r1\"}}}}' ;;\n  \
             *\"invoke \"*) {invoke} ;;\n  *) exit 3 ;;\nesac\n"
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    fn write_reads(directory: &Path, input: Value) -> PathBuf {
        let path = directory.join("reads.json");
        let body = json!({"format":FORMAT,"reads":[
            {"name":"slack","adapter":"slack-bot","connection":"conn_a","operation":"conversations.list","input":input}]});
        std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
        path
    }

    #[cfg(unix)]
    fn options(directory: &Path, reads: PathBuf, report: PathBuf) -> Options {
        Options {
            reads,
            report,
            connectors: directory.join("connectors"),
            config: Some(directory.join("config.toml")),
            state_dir: None,
            duration_seconds: 0,
            pause_seconds: 0,
            call_timeout_seconds: 5,
        }
    }

    #[test]
    #[cfg(unix)]
    fn the_process_caller_runs_the_binary_and_times_out() {
        let temp = tempfile::tempdir().unwrap();
        fake_binary(
            temp.path(),
            "echo '{\"ok\":true,\"result\":{\"result\":{\"status\":200}}}'",
        );
        let mut process = Process {
            program: temp.path().join("connectors"),
            globals: vec!["--output".into(), "json".into()],
            timeout: Duration::from_secs(5),
        };
        let raw = process.call(&["operations".into(), "describe".into()]);
        assert_eq!(raw.exit, Some(0));
        assert_eq!(
            classify(&raw),
            Outcome::Answered(
                json!({"connector":"fixture","instance":"main","schema":"s1","revision":"r1"})
            )
        );
        let log = std::fs::read_to_string(temp.path().join("argv.log")).unwrap();
        assert_eq!(log, "--output json operations describe\n");

        let slow = tempfile::tempdir().unwrap();
        fake_binary(slow.path(), "exec sleep 30");
        let mut process = Process {
            program: slow.path().join("connectors"),
            globals: Vec::new(),
            timeout: Duration::from_millis(300),
        };
        let started = Instant::now();
        let raw = process.call(&["invoke".into(), "fixture/main".into()]);
        assert!(started.elapsed() < Duration::from_secs(10));
        assert_eq!(
            classify(&raw),
            Outcome::Unanswered(json!({"kind":"timeout"}))
        );

        let mut missing = Process {
            program: temp.path().join("absent"),
            globals: Vec::new(),
            timeout: Duration::from_secs(1),
        };
        let raw = missing.call(&["invoke".into(), "fixture/main".into()]);
        assert!(matches!(classify(&raw), Outcome::Unanswered(v) if v["kind"] == "spawn"));
    }

    #[test]
    #[cfg(unix)]
    fn run_writes_a_report_outside_the_repository_with_no_secret() {
        let repository = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fake_binary(
            outside.path(),
            "echo '{\"ok\":true,\"result\":{\"result\":{\"status\":200,\"body\":{\"token\":\"xoxb-result-secret\"}}}}'",
        );
        let reads = write_reads(outside.path(), json!({"token":"xoxp-input-secret"}));
        let report = outside.path().join("report.json");
        run(
            repository.path(),
            options(outside.path(), reads.clone(), report.clone()),
        )
        .unwrap();
        let bytes = std::fs::read_to_string(&report).unwrap();
        assert!(!bytes.contains("secret"), "{bytes}");
        let value: Value = serde_json::from_str(&bytes).unwrap();
        assert_eq!(value["format"], REPORT_FORMAT);
        assert_eq!(value["unanswered"], 0);
        assert_eq!(value["revalidations"], 0);
        assert_eq!(value["rounds"], 1);
        let log = std::fs::read_to_string(outside.path().join("argv.log")).unwrap();
        let config = outside.path().join("config.toml");
        let globals = format!("--config {} --output json ", config.display());
        assert!(
            log.lines().all(|l| {
                l.starts_with(&format!("{globals}operations describe "))
                    || l.starts_with(&format!("{globals}invoke fixture/main "))
            }),
            "{log}"
        );

        // An existing report is not overwritten, and a report inside the repository is refused.
        let again = run(
            repository.path(),
            options(outside.path(), reads.clone(), report.clone()),
        );
        assert!(again.is_err());
        let inside = repository.path().join("report.json");
        let refused = run(
            repository.path(),
            options(outside.path(), reads, inside.clone()),
        );
        assert!(refused.is_err());
        assert!(!inside.exists());
    }

    #[test]
    #[cfg(unix)]
    fn run_fails_on_an_unanswered_read_and_still_writes_the_report() {
        let repository = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fake_binary(
            outside.path(),
            "echo '{\"ok\":false,\"error\":{\"code\":\"failure\",\"data\":{\"code\":\"unauthenticated\"}}}'; exit 1",
        );
        let reads = write_reads(outside.path(), json!({}));
        let report = outside.path().join("report.json");
        let result = run(
            repository.path(),
            options(outside.path(), reads, report.clone()),
        );
        assert!(result.is_err());
        let value: Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(value["unanswered"], 1);
        assert_eq!(
            value["records"][1]["refusal"],
            json!({"code":"failure","data":{"code":"unauthenticated"}})
        );
    }
}
