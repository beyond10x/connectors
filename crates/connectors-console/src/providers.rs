//! `connectors providers` — what the catalogue can actually reach, as a measured fact.
//!
//! # Why this is a command and not a document
//!
//! "61 providers" is a number that ages the moment it is written down, and a plan or a README that
//! carries it is wrong silently. Every field here is read from the compiled catalogue this binary
//! embeds, so the answer belongs to the build rather than to whoever last edited a table.
//!
//! It also answers the question a person actually has before connecting something: *is this one
//! ready, and what will it ask me for?* A provider with no declared authority cannot render a
//! credential address; one with no declared credential has nothing to ask for; one with no verify
//! probe can be connected and cannot be checked. Those are three different kinds of not-ready and
//! they are worth telling apart.

use std::collections::BTreeSet;

use serde_json::{json, Value};

/// One provider's readiness, and what connecting it will cost the operator in answers.
fn describe(provider: &'static catalog::Provider) -> Value {
    let required_config = provider
        .config
        .iter()
        .filter(|field| field.required)
        .count();
    // The distinct alternative credential sets across this provider's operations. Each one is a way
    // to authenticate — GitLab's personal token, its group token, its OAuth token — and each is an
    // `auth_profile` a Connect Session can name.
    let mechanisms = provider
        .operations
        .iter()
        .flat_map(|operation| operation.credentials.iter())
        .map(|mechanism| mechanism.join("+"))
        .collect::<BTreeSet<_>>();
    let exposed = provider
        .operations
        .iter()
        .filter(|operation| operation.expose)
        .count();
    let reads = provider
        .operations
        .iter()
        .filter(|operation| matches!(operation.direction, catalog::OperationDirection::Read))
        .count();

    json!({
        "provider": provider.id,
        "vendor": provider.vendor,
        // Without an authority no credential address renders at all, so a consumer resolving a
        // credential can only refuse. That is the correct answer and a distinct one from "this
        // provider needs no credential".
        "authority": provider.authority,
        "credentials": provider.auth.iter().map(|credential| credential.name).collect::<Vec<_>>(),
        "mechanisms": mechanisms.len(),
        "required_config_fields": required_config,
        // `None` means the provider declares no Test-connection read. It is not an invitation to
        // guess one: `auth test` reports that it cannot check rather than reporting success.
        "verify": provider.verify,
        "operations": provider.operations.len(),
        "exposed": exposed,
        "reads": reads,
        "writes": provider.operations.len() - reads,
        "ready": provider.authority.is_some() && !provider.auth.is_empty() && provider.verify.is_some(),
    })
}

/// Every catalogued provider, or those whose id or vendor contains `query`.
#[must_use]
pub fn run(query: &str) -> Value {
    let needle = query.trim().to_ascii_lowercase();
    let rows = catalog::providers()
        .iter()
        .filter(|provider| {
            needle.is_empty()
                || provider.id.to_ascii_lowercase().contains(&needle)
                || provider.vendor.to_ascii_lowercase().contains(&needle)
        })
        .map(|provider| describe(provider))
        .collect::<Vec<_>>();

    // The summary is the part a person quotes, so it is computed over what was shown rather than
    // over the whole catalogue — a filtered listing that reported the global totals would be a
    // number that looks measured and is not.
    let ready = rows.iter().filter(|row| row["ready"] == true).count();
    let operations: u64 = rows
        .iter()
        .filter_map(|row| row["operations"].as_u64())
        .sum();
    json!({
        "providers": rows,
        "summary": {
            "listed": rows.len(),
            "ready": ready,
            "operations": operations,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_catalogue_is_reported_rather_than_asserted() {
        let all = run("");
        let listed = all["summary"]["listed"].as_u64().expect("a count");
        // Bounds, not an exact number: this test's job is to prove the command reads the real
        // catalogue, not to pin a figure that a legitimate provider addition would break.
        assert!(listed > 40, "the shipped catalogue has {listed} providers");
        assert!(all["summary"]["operations"].as_u64().expect("a count") > 500);
        assert!(all["summary"]["ready"].as_u64().expect("a count") <= listed);
    }

    #[test]
    fn a_query_narrows_to_one_provider_and_its_summary_follows() {
        let gitlab = run("gitlab");
        assert_eq!(gitlab["summary"]["listed"], 1);
        assert_eq!(gitlab["providers"][0]["provider"], "gitlab");
        // GitLab is the worked example throughout this story's plan: an authority, several
        // credentials, and a declared probe.
        assert_eq!(gitlab["providers"][0]["ready"], true);
        assert!(
            gitlab["providers"][0]["mechanisms"]
                .as_u64()
                .expect("a count")
                > 1
        );
        assert_eq!(
            gitlab["summary"]["operations"],
            gitlab["providers"][0]["operations"]
        );
    }

    #[test]
    fn a_provider_without_a_probe_is_not_ready_and_says_why_by_omission() {
        // Jira declares an authority and credentials and no Test-connection read. Reporting it as
        // ready would tell an operator they can verify something they cannot.
        let jira = run("jira");
        assert_eq!(jira["providers"][0]["verify"], Value::Null);
        assert_eq!(jira["providers"][0]["ready"], false);
        assert_ne!(jira["providers"][0]["authority"], Value::Null);
    }

    #[test]
    fn an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue() {
        let none = run("no-such-provider");
        assert_eq!(none["summary"]["listed"], 0);
        assert_eq!(none["summary"]["operations"], 0);
    }
}
