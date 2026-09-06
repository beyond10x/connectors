//! Named CLI failures and their stable public error codes.

use super::*;

#[derive(Debug, thiserror::Error)]
pub(super) enum MainError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Config(#[from] connectors_runtime::ConfigError),
    #[error(transparent)]
    Client(#[from] connectors_client::ClientError),
    #[error(transparent)]
    Identity(#[from] connectors_client::IdentityError),
    #[error(transparent)]
    Hosted(#[from] connectors_client::AuthenticatedHostedError),
    #[error("local Connector request failed: {0}")]
    Io(#[from] io::Error),
    #[error("local Connector response was malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Connect(#[from] connect::ConnectError),
    #[error(transparent)]
    Remediation(#[from] remediation::RemediationError),
    #[error(transparent)]
    Input(#[from] input::InputError),
    #[error(transparent)]
    Auth(#[from] auth::AuthError),
    #[error(transparent)]
    Admin(#[from] admin::AdminError),
    #[error(transparent)]
    Enrol(#[from] enrol::EnrolError),
    #[error(transparent)]
    Refused(#[from] connectors_console::envelope::ReducedError),
    #[error(transparent)]
    Init(#[from] init::InitError),
    #[error(transparent)]
    Output(#[from] output::OutputError),
    #[error("could not write the completion script: {0}")]
    Completions(io::Error),
    /// `doctor` found something that cannot work. The detail is in the report it already printed.
    #[error("this installation has a problem `connectors inspect doctor` named above")]
    Unhealthy,
    #[error("`connectors serve mcp` owns stdout and cannot be combined with --output")]
    McpOutput,
    #[error("--target hosted cannot be combined with local-only --config or --state-root")]
    TargetConflict,
    #[error("events require a persistent daemon; run `connectors serve local` with the same --config and --state-root")]
    DaemonRequired,
}

impl MainError {
    /// A stable token naming the *class* of fault, for a caller that branches on failures.
    ///
    /// Deliberately coarse and deliberately not the message: a script should be able to match on
    /// `configuration` without depending on the sentence a human reads, and the sentence is free to
    /// improve. A refusal forwards the Connector's own code, which is the contract's vocabulary and
    /// more precise than anything this layer could invent. No arm can carry a credential.
    pub(super) fn code(&self) -> &str {
        match self {
            Self::Runtime(_) => "runtime",
            Self::Config(_) | Self::Init(_) => "configuration",
            Self::Client(_) => "connector-unreachable",
            Self::Identity(IdentityError::NoActiveLogin) => "hosted-login-required",
            Self::Identity(_) => "identity",
            Self::Hosted(_) => "hosted-connector",
            Self::Io(_) => "io",
            Self::Json(_) => "malformed-response",
            Self::Connect(connect::ConnectError::Unsupported(_)) => "unsupported-provider",
            Self::Connect(_) => "connect",
            Self::Remediation(error) => error.code(),
            Self::Output(_) => "output",
            Self::Completions(_) => "output",
            Self::Refused(refusal) => &refusal.code,
            Self::Unhealthy => "unhealthy",
            Self::McpOutput => "invalid-argument",
            Self::TargetConflict => "target-conflict",
            Self::DaemonRequired => "daemon-required",
            Self::Input(_) => "invalid-argument",
            Self::Auth(_) => "credential-store",
            Self::Admin(_) => "admin",
            Self::Enrol(_) => "connect",
        }
    }
}
