use std::collections::BTreeSet;

use service::PrincipalContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPrincipal {
    pub issuer: String,
    pub tenant_id: String,
    pub subject: String,
    pub actor_subject: String,
    pub email: Option<String>,
    pub token_id: String,
    pub scopes: BTreeSet<String>,
    pub groups: BTreeSet<String>,
    pub authority_snapshot_sha256: String,
    pub deployment_id: Option<String>,
}

impl HostedPrincipal {
    pub(super) fn allows(&self, scope: &str) -> bool {
        self.scopes.contains(scope)
    }

    pub(super) fn principal_context(
        &self,
        request_id: &str,
    ) -> Result<PrincipalContext, service::PrincipalContextError> {
        PrincipalContext::hosted_with_groups(
            self.tenant_id.clone(),
            self.subject.clone(),
            self.actor_subject.clone(),
            self.email.clone(),
            self.token_id.clone(),
            self.authority_snapshot_sha256.clone(),
            self.groups.clone(),
        )
        .and_then(|context| {
            context.with_hosted_provenance(
                self.issuer.clone(),
                self.token_id.clone(),
                self.deployment_id.clone(),
                request_id.to_owned(),
                request_id.to_owned(),
            )
        })
    }
}
