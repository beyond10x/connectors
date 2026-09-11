use super::*;
use connectors_cli_contract::{
    Context, DynamicError, DynamicPhase, DynamicValidator, OutputMode, ProcessOutput,
};
use connectors_host::local::{
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
                "profile",
                "connection",
                "expected_revision",
                "operation",
            ] {
                if let Ok(Some(value)) = leaf.try_get_one::<String>(&format!("field:{field}")) {
                    selectors[field] = json!(value);
                }
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
        }))
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
            if selection.callable != "operations-invoke" {
                return None;
            }
            let paths = Paths::resolve(
                selection.context.config.as_deref(),
                selection.context.state_dir.as_deref(),
            )
            .ok()?;
            let bootstrap = owner::cached(&paths, selection.selectors["adapter"].as_str()?).ok()?;
            let operation = selection.selectors["operation"].as_str()?;
            Some(
                bootstrap
                    .requirements
                    .iter()
                    .any(|r| r.operation == operation && r.effect == runtime::Effect::Write),
            )
        })()
        .unwrap_or(false);
        if self
            .selection
            .as_ref()
            .is_some_and(|s| matches!(s.callable.as_str(), "approval-prepare" | "approval-issue"))
            || is_write
        {
            return match source {
                ProtectedSource::DocumentFile(path) => protected::document_until(
                    Some(&path),
                    self.approval_deadline,
                    owner::approval_issuance::TARGET_LIMIT,
                ),
                ProtectedSource::DocumentStdin => protected::document_until(
                    None,
                    self.approval_deadline,
                    owner::approval_issuance::TARGET_LIMIT,
                ),
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
            "connections-connect" | "connections-repair"
        ) || self.captured.is_some()
        {
            return Err(Code::InvalidInput.into());
        }
        let paths = Paths::resolve(
            selection.context.config.as_deref(),
            selection.context.state_dir.as_deref(),
        )?;
        let text = |key| selection.selectors.get(key).and_then(Value::as_str);
        let alias = text("adapter").ok_or(Code::InvalidInput)?;
        owner::admit_capture(
            &paths,
            alias,
            text("profile"),
            text("connection"),
            text("expected_revision"),
        )?;
        let capture = owner::Client::connect(&paths, true)?.begin(
            alias,
            text("profile").map(str::to_owned),
            text("connection").map(str::to_owned),
            text("expected_revision").map(str::to_owned),
        )?;
        let result = match source {
            ProtectedSource::File(path) => protected::file(&path, capture.expires_at_ms),
            ProtectedSource::Stdin => protected::stdin(capture.expires_at_ms),
            ProtectedSource::HiddenTty => {
                protected::terminal(&capture.profile.fields, capture.expires_at_ms)
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
    pub fn complete(&mut self, call: &Invocation<'_>) -> owner::Result<Value> {
        let selection = self.selection.as_ref().ok_or(Code::InvalidInput)?;
        if call.callable != selection.callable
            || call.context != selection.context
            || call.input["credential_document"] != MARKER
        {
            return Err(Code::InvalidInput.into());
        }
        for field in ["adapter", "profile", "connection", "expected_revision"] {
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
            let reply = super::owner_failure(error);
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
            let alias = call.input["adapter"].as_str().ok_or(Code::InvalidInput)?;
            let bootstrap = owner::operation_snapshot(&paths, alias, &call.input)?;
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
                DynamicPhase::Result => owner::validate_document(
                    &operation.output_schema,
                    value["result"]
                        .as_str()
                        .ok_or(Code::InvalidInput)?
                        .as_bytes(),
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
