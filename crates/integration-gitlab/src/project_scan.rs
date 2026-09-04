//! Bounded GitLab membership-project traversal shared by discovery and redemption.

use connector_secrets::Secret;
use serde_json::Value;

use crate::backend::{GitlabError, GitlabInner, MAX_PROVIDER_RESPONSE_BYTES};
use crate::transport::{bearer_headers, decode_response, http_request};

const MAX_PROJECT_SCAN_PAGES: u64 = 100;

impl GitlabInner {
    pub(crate) async fn scan_membership_projects(
        &self,
        connection_ref: &str,
        token: &Secret,
        order_by_activity: bool,
        mut inspect: impl FnMut(&Value) -> bool,
    ) -> Result<bool, GitlabError> {
        let mut page = 1_u64;
        for scanned in 0..MAX_PROJECT_SCAN_PAGES {
            let mut query = vec![
                ("membership".to_owned(), "true".to_owned()),
                ("simple".to_owned(), "true".to_owned()),
                ("per_page".to_owned(), "100".to_owned()),
                ("page".to_owned(), page.to_string()),
            ];
            if order_by_activity {
                query.push(("order_by".to_owned(), "last_activity_at".to_owned()));
            }
            let mut target = self.provider_url("/api/v4/projects")?;
            target.query_pairs_mut().extend_pairs(&query);
            let response = self
                .execute(
                    connection_ref,
                    http_request("GET", target, bearer_headers(token), None),
                    MAX_PROVIDER_RESPONSE_BYTES,
                    vec!["x-next-page".to_owned()],
                )
                .await?;
            let next_page = response.header("x-next-page").map(str::to_owned);
            let projects: Vec<Value> = decode_response(response)?;
            for project in &projects {
                if inspect(project) {
                    return Ok(true);
                }
            }
            let Some(next_page) = next_page.filter(|value| !value.is_empty()) else {
                return Ok(false);
            };
            let next_page = next_page
                .parse::<u64>()
                .ok()
                .filter(|next| *next > page && *next <= 10_000)
                .ok_or_else(|| GitlabError::new("provider-pagination"))?;
            if scanned + 1 == MAX_PROJECT_SCAN_PAGES {
                return Err(GitlabError::new("provider-pagination"));
            }
            page = next_page;
        }
        Err(GitlabError::new("provider-pagination"))
    }
}
