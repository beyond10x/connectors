use super::*;
use connectors_cli_contract::{
    Context, DynamicError, DynamicPhase, DynamicValidator, OutputMode, ProcessOutput,
};
use connectors_host::local::{
    oauth,
    owner::{self, Code},
    protected, runtime,
};
use connectors_sdk::Secret;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const MARKER: &str = "connectors-admitted-private-capture/1";
pub(super) type Shared = Rc<RefCell<Session>>;

struct Selection {
    callable: String,
    context: Context,
    selectors: Value,
}
pub(super) struct Session {
    selection: Option<Selection>,
    captured: Option<(owner::Capture, Secret)>,
    signals: Option<protected::Signals>,
    early: Option<(String, Context, owner::Error)>,
    pub deadline: u64,
    pub approval_deadline: Instant,
    pub mutation_deadline: Option<owner::mutation::Deadline>,
    pub cancelled: bool,
    /// The exit code of a consumer `connections launch` ran.
    pub launched: Option<i32>,
    /// The access token `connections token --reveal` issued, printed alone.
    pub revealed: Option<connectors_oauth2::Secret>,
    /// What `invoke --fields` or `--each` asks of the answer.
    pub shaping: Option<super::invoke::Shaping>,
    /// The full answer of an `invoke` given `--fields` or `--each`, kept for
    /// shaping once the runner has checked and printed it.
    pub answer: Option<Value>,
}
impl Session {
    pub fn new(args: &[OsString]) -> Shared {
        let approval_deadline = Instant::now() + Duration::from_secs(20);
        let mutation_deadline = owner::mutation::Deadline::start().ok();
        let selection = (|| {
            // Use the generated command and its argument ids. No effect is
            // performed by this preparse, including help and invalid arguments.
            let matches = connectors_cli_contract::command()
                .try_get_matches_from(args)
                .ok()?;
            let context = Context {
                config: matches.get_one("global_config").cloned(),
                state_dir: matches.get_one("global_state").cloned(),
                output: if matches
                    .get_one::<String>("global_output")
                    .is_some_and(|v| v == "json")
                {
                    OutputMode::Json
                } else {
                    OutputMode::Human
                },
            };
            let mut path = Vec::new();
            let mut leaf = &matches;
            while let Some((name, next)) = leaf.subcommand() {
                path.push(name.to_owned());
                leaf = next;
            }
            let plan = connectors_cli_contract::plan();
            let command = plan
                .commands
                .iter()
                .find(|c| c.path == path || c.aliases.contains(&path))?;
            let mut selectors = json!({});
            for field in [
                "adapter",
                "connector",
                "instance",
                "profile",
                "connection",
                "expected_revision",
                "operation",
                "device",
                "target",
                "endpoint",
            ] {
                if let Ok(Some(value)) = leaf.try_get_one::<String>(&format!("field:{field}")) {
                    selectors[field] = json!(value);
                }
            }
            // A connect's `<connector>/<instance>` selects as its options do;
            // a target that cannot is refused by `endpoint::prepare`.
            if command.callable == "connections-connect" {
                selectors = super::endpoint::selection(&selectors).unwrap_or(selectors);
            }
            Some(Selection {
                callable: command.callable.clone(),
                context,
                selectors,
            })
        })();
        Rc::new(RefCell::new(Self {
            selection,
            captured: None,
            signals: None,
            early: None,
            deadline: 0,
            approval_deadline,
            mutation_deadline,
            cancelled: false,
            launched: None,
            revealed: None,
            shaping: None,
            answer: None,
        }))
    }
    /// The preparsed callable and its context, when the arguments parsed.
    pub fn selected(&self) -> Option<(&str, &Context)> {
        self.selection
            .as_ref()
            .map(|s| (s.callable.as_str(), &s.context))
    }
    /// The preparsed selectors, when the arguments parsed.
    pub fn selectors(&self) -> Option<&Value> {
        self.selection.as_ref().map(|s| &s.selectors)
    }
    pub fn signals(&mut self) -> owner::Result<()> {
        if self.signals.is_none() {
            self.signals = Some(protected::Signals::install()?);
        }
        if self
            .signals
            .as_ref()
            .is_some_and(protected::Signals::interrupted)
        {
            return Err(Code::Interrupted.into());
        }
        Ok(())
    }
    fn acquire(&mut self, source: ProtectedSource) -> owner::Result<String> {
        self.signals()?;
        let is_write = (|| {
            let selection = self.selection.as_ref()?;
            if selection.callable != "invoke" {
                return None;
            }
            let paths = Paths::resolve(
                selection.context.config.as_deref(),
                selection.context.state_dir.as_deref(),
            )
            .ok()?;
            let alias = super::invoke::alias_at(&selection.selectors, &paths).ok()?;
            let bootstrap = owner::cached(&paths, &alias).ok()?;
            let operation = selection.selectors["operation"].as_str()?;
            Some(
                bootstrap
                    .requirements
                    .iter()
                    .any(|r| r.operation == operation && r.effect == runtime::Effect::Write),
            )
        })()
        .unwrap_or(false);
        let approval = self
            .selection
            .as_ref()
            .is_some_and(|s| matches!(s.callable.as_str(), "approval-prepare" | "approval-issue"));
        // An approval command's refused business document names `input`
        // (contracts/cli/v1alpha1/semantics.md, "The refused argument"); an
        // approved write's `invoke` names its own (`invoke::early_failure`).
        let named = |mut error: owner::Error| {
            if approval && error.code == Code::InvalidInput && error.field.is_none() {
                error.field = Some(owner::Argument::Input);
            }
            error
        };
        if approval || is_write {
            return match source {
                ProtectedSource::DocumentFile(path) => protected::document_until(
                    Some(&path),
                    self.approval_deadline,
                    owner::approval_issuance::TARGET_LIMIT,
                )
                .map_err(named),
                ProtectedSource::DocumentStdin => protected::document_until(
                    None,
                    self.approval_deadline,
                    owner::approval_issuance::TARGET_LIMIT,
                )
                .map_err(named),
                _ => Err(Code::InvalidInput.into()),
            };
        }
        match source {
            ProtectedSource::DocumentFile(path) => return protected::document(Some(&path)),
            ProtectedSource::DocumentStdin => return protected::document(None),
            _ => {}
        }
        let selection = self.selection.as_ref().ok_or(Code::InvalidInput)?;
        if !matches!(
            selection.callable.as_str(),
            "connections-connect" | "connections-reconnect"
        ) || self.captured.is_some()
        {
            return Err(Code::InvalidInput.into());
        }
        let paths = Paths::resolve(
            selection.context.config.as_deref(),
            selection.context.state_dir.as_deref(),
        )?;
        // The read-only binding takes no protected source: the owner reads
        // the configured item itself.
        if Config::load(&paths.config)?.selection().read_only() {
            return Err(Code::InvalidInput.into());
        }
        let device = selection.selectors.get("device").and_then(Value::as_str) == Some("true");
        let text = |key| selection.selectors.get(key).and_then(Value::as_str);
        let alias = super::select::alias_at(&selection.selectors, &paths)?;
        let alias = alias.as_str();
        let connection = reconnected(&selection.callable, &paths, alias, text("connection"))?;
        owner::admit_capture(
            &paths,
            alias,
            text("profile"),
            connection.as_deref(),
            text("expected_revision"),
        )?;
        let capture = owner::Client::connect(&paths, true)?.begin(
            alias,
            text("profile").map(str::to_owned),
            connection,
            text("expected_revision").map(str::to_owned),
        )?;
        // An `oauth2_grant` profile's source is a confidential client's secret,
        // with which its grant runs here; `--device` names that grant's flow and
        // belongs to no other profile.
        let grant = capture.profile.grant().is_some();
        let granted = |document: Secret| -> owner::Result<Secret> {
            if grant {
                super::dialects::granted(&capture, Some(document), device)
            } else {
                Ok(document)
            }
        };
        if device && !grant {
            let mut error = owner::Error::from(Code::InvalidInput);
            error.acquisition = Some(capture.acquisition.clone());
            return Err(error);
        }
        let result = match source {
            // Google's client file under an OAuth-acquired profile becomes the
            // entry by consent; every other file is submitted as it is.
            ProtectedSource::File(path) => {
                protected::file(&path, capture.expires_at_ms).and_then(|document| {
                    if grant {
                        granted(document)
                    } else {
                        oauth::entry(&capture.profile, document, capture.expires_at_ms)
                    }
                })
            }
            ProtectedSource::Stdin => protected::stdin(capture.expires_at_ms).and_then(granted),
            ProtectedSource::HiddenTty => {
                protected::terminal(&capture.profile.fields, capture.expires_at_ms)
                    .and_then(granted)
            }
            _ => unreachable!(),
        };
        let secret = result.map_err(|mut error| {
            error.acquisition = Some(capture.acquisition.clone());
            if error.code == Code::InvalidInput {
                error.code = Code::ProtectedEntryUnavailable;
            }
            error
        })?;
        self.captured = Some((capture, secret));
        Ok(MARKER.into())
    }
    /// `connections connect` or `reconnect` with no protected source. Under the
    /// read-only `mounted_secret` binding the owner reads the item
    /// `<adapter alias>/<profile>` itself and no bytes pass through this
    /// process. Under a platform binding only an `oauth2_grant` profile of a
    /// public client takes none: its grant runs here and its token response is
    /// the entry. Any other profile is `protected_entry_unavailable`, before
    /// custody, an owner or the adapter is reached when the cached description
    /// names its kind (`owner::admit_sourceless_grant`), and otherwise by the
    /// owner before the registry is written.
    pub fn complete_without_source(&mut self, call: &Invocation<'_>) -> owner::Result<Value> {
        self.signals()?;
        let selection = self.selection.as_ref().ok_or(Code::InvalidInput)?;
        if call.callable != selection.callable || call.context != selection.context {
            return Err(Code::InvalidInput.into());
        }
        let paths = Paths::resolve(
            call.context.config.as_deref(),
            call.context.state_dir.as_deref(),
        )?;
        let read_only = Config::load(&paths.config)?.selection().read_only();
        let device = call.input.get("device").and_then(Value::as_bool) == Some(true);
        if read_only && device {
            return Err(Code::InvalidInput.into());
        }
        let text = |key| call.input.get(key).and_then(Value::as_str);
        let alias = text("adapter").ok_or(Code::InvalidInput)?;
        let connection = reconnected(call.callable, &paths, alias, text("connection"))?;
        let admit = if read_only {
            owner::admit_capture
        } else {
            owner::admit_sourceless_grant
        };
        admit(
            &paths,
            alias,
            text("profile"),
            connection.as_deref(),
            text("expected_revision"),
        )?;
        if !read_only {
            let capture = owner::Client::connect(&paths, true)?.begin_grant(
                alias,
                text("profile").map(str::to_owned),
                connection,
                text("expected_revision").map(str::to_owned),
            )?;
            let entry = super::dialects::granted(&capture, None, device).map_err(|mut error| {
                error.acquisition = Some(capture.acquisition.clone());
                if error.code == Code::InvalidInput {
                    error.code = Code::ProtectedEntryUnavailable;
                }
                error
            })?;
            return capture.complete(&entry);
        }
        owner::Client::connect(&paths, true)?
            .begin(
                alias,
                text("profile").map(str::to_owned),
                connection,
                text("expected_revision").map(str::to_owned),
            )?
            .complete_mounted()
    }

    pub fn complete(&mut self, call: &Invocation<'_>) -> owner::Result<Value> {
        let selection = self.selection.as_ref().ok_or(Code::InvalidInput)?;
        if call.callable != selection.callable
            || call.context != selection.context
            || call.input["credential_document"] != MARKER
        {
            return Err(Code::InvalidInput.into());
        }
        // The handler's input names the alias the preparsed selector named.
        let paths = Paths::resolve(
            call.context.config.as_deref(),
            call.context.state_dir.as_deref(),
        )?;
        if call.input.get("adapter")
            != Some(&json!(super::select::alias_at(
                &selection.selectors,
                &paths
            )?))
        {
            return Err(Code::InvalidInput.into());
        }
        for field in ["profile", "connection", "expected_revision"] {
            if call.input.get(field) != selection.selectors.get(field) {
                return Err(Code::InvalidInput.into());
            }
        }
        let (capture, secret) = self.captured.take().ok_or(Code::InvalidInput)?;
        capture.complete(&secret)
    }
    pub fn finish(&mut self, output: &mut ProcessOutput) {
        if let Some((callable, context, error)) = self.early.take() {
            // Sources and DynamicValidator expose only generic errors. Render
            // the owner's closed failure through the same generated error shape
            // so admission refusals retain their actionable CLI semantics.
            let reply = if matches!(callable.as_str(), "approval-prepare" | "approval-issue") {
                super::approval_failure(error)
            } else if callable == "invoke" {
                // Every `invalid_input` of `invoke` names its argument.
                let selectors = self.selection.as_ref().map(|s| &s.selectors);
                super::invoke::early_failure(error, selectors.unwrap_or(&Value::Null))
            } else {
                super::owner_failure(error)
            };
            let (code, data, exit_code) = match reply {
                HandlerReply::UsageError { code, data } => (code, data, 2),
                HandlerReply::Error { code, data } => (code, data, 1),
                _ => return,
            };
            let plan = connectors_cli_contract::plan();
            if output.exit_code == 0
                || !plan
                    .callables
                    .get(&callable)
                    .and_then(|c| c.errors.get(&code))
                    .is_some_and(|c| c.shape.accepts(&data))
            {
                return;
            }
            let exit_code = if data["kind"] == "interrupted" {
                130
            } else {
                exit_code
            };
            let stderr = if context.output == OutputMode::Json {
                format!(
                    "{}\n",
                    json!({"ok":false,"error":{"code":code,"data":data}})
                )
            } else {
                format!("{code}: {data}\n")
            };
            *output = ProcessOutput {
                exit_code,
                stdout: String::new(),
                stderr,
            };
        }
        // A received terminal success has already won the completion race.
        // Never emit ok:true with an interruption exit status.
        if output.exit_code != 0
            && (self.cancelled
                || self
                    .signals
                    .as_ref()
                    .is_some_and(protected::Signals::interrupted))
        {
            output.exit_code = 130;
        }
    }
}
pub(super) struct AdmittedSources(pub Shared);
impl Sources for AdmittedSources {
    fn acquire(&mut self, source: ProtectedSource) -> Result<String, AcquireError> {
        let mut session = self.0.borrow_mut();
        session.acquire(source).map_err(|error| {
            let interrupted = error.code == Code::Interrupted;
            if let Some(selection) = &session.selection {
                session.early =
                    Some((selection.callable.clone(), selection.context.clone(), error));
            }
            if interrupted {
                AcquireError::Interrupted
            } else {
                AcquireError::Unavailable
            }
        })
    }
}
pub(super) struct NativeValidator(pub Shared);
impl DynamicValidator for NativeValidator {
    fn validate(
        &mut self,
        call: &Invocation<'_>,
        phase: DynamicPhase,
        value: &Value,
    ) -> Result<(), DynamicError> {
        // Failure data has already passed the generated closed Failure shape.
        if matches!(phase, DynamicPhase::Error(_)) {
            return Ok(());
        }
        // Write delivery is validated under current admission by the owner and
        // against the selected native schema before the handler projects it.
        // A second mutable snapshot lookup here could erase known effect data.
        if matches!(phase, DynamicPhase::Result)
            && value.get("mutation").is_some_and(|v| !v.is_null())
        {
            return Ok(());
        }
        let result: owner::Result<()> = (|| {
            let mut session = self.0.borrow_mut();
            session.signals()?;
            let paths = Paths::resolve(
                call.context.config.as_deref(),
                call.context.state_dir.as_deref(),
            )?;
            // `invoke` is the one dynamic callable: its target selects the alias.
            let alias = super::invoke::alias_at(&call.input, &paths)?;
            let bootstrap = owner::operation_snapshot(&paths, &alias, &call.input)?;
            let descriptor = bootstrap.descriptor()?;
            let operation = descriptor
                .operation(call.input["operation"].as_str().ok_or(Code::InvalidInput)?)
                .map_err(|_| Code::NotFound)?;
            match phase {
                DynamicPhase::Input => {
                    session.deadline = connectors_sdk::now_ms() + 30_000;
                    owner::validate_document(
                        &operation.input_schema,
                        call.input["input"]
                            .as_str()
                            .ok_or(Code::InvalidInput)?
                            .as_bytes(),
                        runtime::INPUT_LIMIT,
                    )
                }
                // The result is the provider's JSON value, held to the selected
                // output schema and the result bound as the document it encodes.
                // The generated `Json` field admits any value; this check is what
                // refuses a string holding JSON text where the schema names an
                // object.
                DynamicPhase::Result => owner::validate_document(
                    &operation.output_schema,
                    &serde_json::to_vec(&value["result"]).map_err(|_| Code::InvalidInput)?,
                    runtime::RESULT_LIMIT,
                ),
                DynamicPhase::Error(_) => unreachable!(),
            }
        })();
        result.map_err(|error| {
            let kind = match error.code {
                Code::Interrupted => DynamicError::Interrupted,
                Code::InvalidInput => DynamicError::InvalidValue,
                _ => DynamicError::Unavailable,
            };
            self.0.borrow_mut().early = Some((call.callable.into(), call.context.clone(), error));
            kind
        })
    }
}

/// The connection a `connections reconnect` reconnects: the one it names, or
/// else the instance's one published non-revoked connection
/// (story:connect-refused-while-instance-holds-connection). A connect names
/// none.
fn reconnected(
    callable: &str,
    paths: &Paths,
    alias: &str,
    named: Option<&str>,
) -> owner::Result<Option<String>> {
    match named {
        Some(connection) => Ok(Some(connection.to_owned())),
        None if callable == "connections-reconnect" => {
            super::connections::held_one(paths, alias).map(Some)
        }
        None => Ok(None),
    }
}
