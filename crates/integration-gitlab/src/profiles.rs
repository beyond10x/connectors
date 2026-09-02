//! GitLab's closed self-service profile vocabulary and catalog projection.

use serde::{Deserialize, Serialize};

use crate::backend::INTEGRATION_REF;

pub(super) const PROFILE_OAUTH: &str = "gitlab.oauth_user";
pub(super) const PROFILE_PAT: &str = "gitlab.personal_token";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GitlabProfile {
    OAuthUser,
    PersonalToken,
}

impl GitlabProfile {
    pub(super) fn parse(value: Option<&str>) -> Option<Self> {
        match value {
            Some(PROFILE_OAUTH) => Some(Self::OAuthUser),
            Some(PROFILE_PAT) => Some(Self::PersonalToken),
            _ => None,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::OAuthUser => PROFILE_OAUTH,
            Self::PersonalToken => PROFILE_PAT,
        }
    }
}

pub(super) fn setup_profiles(
    provider_ref: &str,
) -> Vec<protocol::catalog::SetupProfileSummary> {
    if provider_ref != INTEGRATION_REF {
        return Vec::new();
    }
    [PROFILE_OAUTH, PROFILE_PAT]
        .into_iter()
        .map(|auth_profile| protocol::catalog::SetupProfileSummary {
            auth_profile: auth_profile.to_owned(),
            actor: protocol::catalog::SetupProfileActor::Person,
        })
        .collect()
}
