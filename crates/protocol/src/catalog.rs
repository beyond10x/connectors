//! Credential-free, paginated projection of the complete Connector catalog.
//!
//! Catalog membership is descriptive, never invocation authority. Callers must use the Operation
//! contract to discover callable Connections and to describe or invoke an operation.

use serde::{Deserialize, Serialize};

use crate::operation::OwnerContext;

/// Exact wire contract identity.
pub const CONTRACT: &str = "b10x.connector-catalog.v0alpha1";
/// Largest request frame accepted by either transport.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
/// Largest response frame accepted by either transport.
pub const MAX_RESPONSE_BYTES: usize = 512 * 1024;
/// Largest provider page.
pub const MAX_PROVIDER_RESULTS: u16 = 64;

/// One authenticated catalog request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Exact protocol identity.
    pub protocol: String,
    /// Caller-generated bounded correlation reference.
    pub request_id: String,
    /// Authority context. It filters access to the catalog route, not catalog visibility.
    pub context: OwnerContext,
    /// Closed request family.
    pub request: CatalogRequest,
}

/// Closed catalog reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CatalogRequest {
    /// Search provider summaries in stable catalog order.
    Search(SearchRequest),
    /// Describe one provider, including its complete operation index.
    Describe(DescribeRequest),
}

/// One bounded provider page request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// Case-insensitive text matched against id, vendor and description.
    pub query: String,
    /// Stable provider offset returned by an earlier page.
    pub offset: u16,
    /// Maximum provider summaries to return.
    pub limit: u16,
}

/// Exact provider lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DescribeRequest {
    /// Stable catalog provider id.
    pub provider_ref: String,
}

/// One catalog provider summary. No field can carry a configured value or credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderSummary {
    /// Stable catalog id.
    pub provider_ref: String,
    /// Reverse-DNS authority when declared.
    pub authority: Option<String>,
    /// Display vendor name.
    pub vendor: String,
    /// One-line provider description.
    pub description: String,
    /// Discovery-only audiences; never authorization.
    pub audiences: Vec<String>,
    /// Declared service names.
    pub services: Vec<String>,
    /// Complete operation count, including operations not exposed to a model.
    pub operation_count: u32,
    /// Whether this runtime publishes a Connector-owned setup flow for the provider.
    pub configurable: bool,
    /// The credential-acquisition flows this deployment can actually complete for a person who
    /// is not its operator, in catalog order.
    ///
    /// Empty means a product must offer that person no setup control at all: either the provider
    /// has no self-service flow, or this deployment is missing the configuration one would need.
    /// It carries no credential and no client identifier — only the fact that a flow exists.
    pub setup_profiles: Vec<SetupProfileSummary>,
}

/// One credential-acquisition flow a person may start for themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetupProfileSummary {
    /// Stable auth-profile id, exactly as `ConnectSessionCreateRequest.auth_profile` accepts it.
    pub auth_profile: String,
    /// Whose access the resulting Connection exercises.
    pub actor: SetupProfileActor,
}

/// Whose access a completed setup flow yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SetupProfileActor {
    /// The signed-in person themselves. Reads are bounded by what they can already see.
    Person,
    /// A bot or application the person supplies. Reads are bounded by that bot's own access.
    Application,
}

/// One operation in a provider's descriptive catalog index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogOperationSummary {
    /// Stable operation id.
    pub operation_ref: String,
    /// Declared service.
    pub service: String,
    /// One-line description.
    pub description: String,
    /// Authored risk token.
    pub risk: String,
    /// Whether it is curated for model exposure. This is not callability.
    pub exposed: bool,
}

/// Complete descriptive provider projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderDescription {
    /// Provider summary.
    pub provider: ProviderSummary,
    /// Complete operation index in catalog order.
    pub operations: Vec<CatalogOperationSummary>,
}

/// Successful catalog result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogResult {
    /// One stable search page.
    Search {
        /// Matching provider summaries.
        providers: Vec<ProviderSummary>,
        /// Next stable offset, absent at the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next_offset: Option<u16>,
    },
    /// One exact provider description.
    Describe(ProviderDescription),
}

/// Closed response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    /// A result is present.
    Ok,
    /// An owner-safe error is present.
    Error,
}

/// One owner-safe catalog refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogError {
    /// Stable refusal code.
    pub code: String,
    /// Bounded, credential-free explanation.
    pub message: String,
}

/// One catalog response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    /// Exact protocol identity.
    pub protocol: String,
    /// Echoed request correlation.
    pub request_id: String,
    /// Closed status.
    pub status: ResponseStatus,
    /// Present exactly on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<CatalogResult>,
    /// Present exactly on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CatalogError>,
}

impl RequestEnvelope {
    /// Validate framing and request bounds before catalog work.
    pub fn validate(&self) -> Result<(), CatalogError> {
        if self.protocol != CONTRACT
            || !valid_ref(&self.request_id, 128)
            || !valid_ref(&self.context.tenant_id, 256)
            || !valid_ref(&self.context.agent_id, 256)
            || self.context.agent_revision == 0
            || !valid_ref(&self.context.authority_snapshot_id, 512)
            || self.context.authority_snapshot_sha256.len() != 64
            || !self
                .context
                .authority_snapshot_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(protocol_error());
        }
        match &self.request {
            CatalogRequest::Search(request)
                if request.query.len() <= 512
                    && request.limit > 0
                    && request.limit <= MAX_PROVIDER_RESULTS => {}
            CatalogRequest::Describe(request) if valid_ref(&request.provider_ref, 256) => {}
            _ => return Err(invalid_input()),
        }
        serde_json::to_vec(self)
            .ok()
            .filter(|bytes| bytes.len() <= MAX_FRAME_BYTES)
            .ok_or_else(protocol_error)?;
        Ok(())
    }
}

impl ResponseEnvelope {
    /// Construct a successful response.
    #[must_use]
    pub fn success(request_id: impl Into<String>, response: CatalogResult) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Ok,
            response: Some(response),
            error: None,
        }
    }

    /// Construct a failed response.
    #[must_use]
    pub fn failure(request_id: impl Into<String>, error: CatalogError) -> Self {
        Self {
            protocol: CONTRACT.to_owned(),
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            response: None,
            error: Some(error),
        }
    }

    /// Validate the exact success/error shape and response bound.
    pub fn validate(&self) -> Result<(), CatalogError> {
        if self.protocol != CONTRACT || !valid_ref(&self.request_id, 128) {
            return Err(protocol_error());
        }
        match (self.status, &self.response, &self.error) {
            (ResponseStatus::Ok, Some(result), None) => validate_result(result)?,
            (ResponseStatus::Error, None, Some(error))
                if valid_ref(&error.code, 128)
                    && !error.message.is_empty()
                    && error.message.len() <= 4096 => {}
            _ => return Err(protocol_error()),
        }
        serde_json::to_vec(self)
            .ok()
            .filter(|bytes| bytes.len() <= MAX_RESPONSE_BYTES)
            .ok_or_else(protocol_error)?;
        Ok(())
    }
}

fn validate_result(result: &CatalogResult) -> Result<(), CatalogError> {
    match result {
        CatalogResult::Search {
            providers,
            next_offset: _,
        } => {
            if providers.len() > usize::from(MAX_PROVIDER_RESULTS) {
                return Err(protocol_error());
            }
            for provider in providers {
                validate_provider(provider)?;
            }
        }
        CatalogResult::Describe(description) => {
            validate_provider(&description.provider)?;
            if description.operations.len() > 4096 {
                return Err(protocol_error());
            }
            for operation in &description.operations {
                if !valid_ref(&operation.operation_ref, 256)
                    || !valid_ref(&operation.service, 128)
                    || operation.description.is_empty()
                    || operation.description.len() > 16 * 1024
                    || !valid_ref(&operation.risk, 128)
                {
                    return Err(protocol_error());
                }
            }
        }
    }
    Ok(())
}

fn validate_provider(provider: &ProviderSummary) -> Result<(), CatalogError> {
    if !valid_ref(&provider.provider_ref, 256)
        || provider
            .authority
            .as_deref()
            .is_some_and(|value| !valid_ref(value, 256))
        || provider.vendor.trim().is_empty()
        || provider.vendor.len() > 256
        || provider.description.trim().is_empty()
        || provider.description.len() > 16 * 1024
        || provider.audiences.len() > 64
        || provider.services.len() > 64
        || provider
            .audiences
            .iter()
            .any(|value| !valid_ref(value, 128))
        || provider.services.iter().any(|value| !valid_ref(value, 128))
        || provider.setup_profiles.len() > 16
        || provider
            .setup_profiles
            .iter()
            .any(|profile| !valid_ref(&profile.auth_profile, 128))
        || !provider.configurable && !provider.setup_profiles.is_empty()
    {
        return Err(protocol_error());
    }
    Ok(())
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn protocol_error() -> CatalogError {
    CatalogError {
        code: "protocol".to_owned(),
        message: "catalog frame does not match the contract".to_owned(),
    }
}

fn invalid_input() -> CatalogError {
    CatalogError {
        code: "invalid_input".to_owned(),
        message: "catalog request is outside the admitted bounds".to_owned(),
    }
}

/// Owner-safe not-found response.
#[must_use]
pub fn not_found() -> CatalogError {
    CatalogError {
        code: "not_found".to_owned(),
        message: "catalog provider was not found".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_membership_carries_no_callability_or_credential_value() {
        let provider = ProviderSummary {
            provider_ref: "slack".to_owned(),
            authority: Some("com.slack.api".to_owned()),
            vendor: "Slack".to_owned(),
            description: "Collaboration".to_owned(),
            audiences: vec!["developer".to_owned()],
            services: vec!["default".to_owned()],
            operation_count: 4,
            configurable: true,
            setup_profiles: vec![SetupProfileSummary {
                auth_profile: "slack.org_user".to_owned(),
                actor: SetupProfileActor::Person,
            }],
        };
        let encoded = serde_json::to_string(&provider).unwrap();
        assert!(!encoded.contains("callable"));
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("credential"));
        assert!(!encoded.contains("client_id"));
        assert!(!encoded.contains("secret"));
    }

    #[test]
    fn a_setup_profile_cannot_exist_without_a_setup_form() {
        let mut provider = ProviderSummary {
            provider_ref: "slack".to_owned(),
            authority: Some("com.slack.api".to_owned()),
            vendor: "Slack".to_owned(),
            description: "Collaboration".to_owned(),
            audiences: vec!["developer".to_owned()],
            services: vec!["default".to_owned()],
            operation_count: 4,
            configurable: false,
            setup_profiles: vec![SetupProfileSummary {
                auth_profile: "slack.org_user".to_owned(),
                actor: SetupProfileActor::Person,
            }],
        };
        assert!(validate_provider(&provider).is_err());
        provider.configurable = true;
        assert!(validate_provider(&provider).is_ok());
        provider.setup_profiles.push(SetupProfileSummary {
            auth_profile: String::new(),
            actor: SetupProfileActor::Application,
        });
        assert!(validate_provider(&provider).is_err());
    }
}
