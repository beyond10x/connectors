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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two verifications of the same person on different access tokens, shaped exactly as
    /// the identity-http adapter produces them: the token id becomes the authority snapshot
    /// id and the introspection-envelope hash becomes the snapshot sha, so BOTH differ per
    /// token while the admitted authority is identical.
    fn verified(token_id: &str, envelope_sha: char, scope: &str) -> HostedPrincipal {
        HostedPrincipal {
            issuer: "https://identity.example.test".to_owned(),
            tenant_id: "tenant-test".to_owned(),
            subject: "person:owner".to_owned(),
            actor_subject: "person:owner".to_owned(),
            email: Some("owner@example.test".to_owned()),
            token_id: token_id.to_owned(),
            scopes: BTreeSet::from([scope.to_owned()]),
            groups: BTreeSet::from(["dev".to_owned(), "operator".to_owned()]),
            authority_snapshot_sha256: envelope_sha.to_string().repeat(64),
            deployment_id: None,
        }
    }

    #[test]
    fn lease_seeds_survive_the_verifier_token_rotation_composition() {
        // The production flow: describe runs on a catalog-scoped token, the invoke that
        // follows runs on an invoke-scoped token, and every field the verifier derives
        // from the token differs between them. A description lease minted during the
        // first request must admit the second, so the stable authority seed of the two
        // contexts must be identical end to end through principal_context().
        let describe = verified("token-catalog-1", 'c', "connectors.catalog.read")
            .principal_context("request-describe")
            .unwrap();
        let invoke = verified("token-invoke-2", 'd', "connectors.invoke")
            .principal_context("request-invoke")
            .unwrap();
        assert_eq!(
            describe.stable_authority_seed(),
            invoke.stable_authority_seed()
        );
    }
}
