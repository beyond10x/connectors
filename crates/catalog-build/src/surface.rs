//! The three derivations both catalogue projections share.
//!
//! [`crate::document`] lowers the IR to the canonical artifact and [`crate::site`] projects it for
//! the public explorer. Where the two need the same answer — which operation a name denotes, which
//! host a call reaches, which credentials authenticate it — they read it from here, so the pipeline
//! cannot ship a `catalog.json` that disagrees with `catalog/<id>.catalog.json` about any of them.
//!
//! In the predecessor this lived at the top of the module that rendered the generated Rust tables.
//! That module is gone with the tables; these three are what outlived it.

use anyhow::{bail, Result};
use connector_spec::{Connector, Operation};

/// The operation a name denotes — **and the refusal that keeps the other two member kinds out of
/// every artifact.**
///
/// Both projections resolve an operation through this one function, so neither can be talked into
/// publishing something the other refuses.
///
/// # Why an inbound member gets its own error
///
/// A name that denotes nothing is a bug. A name that denotes an **event or a channel binding** is a
/// specific bug with a specific name: both are declared and neither is invoked, so treating one as
/// an operation is an event dressed up as a pollable op. The two have opposite fixes — "no such
/// operation" says a name is wrong, this says the name is *right* and the caller is wrong — and a
/// generic message would send the reader looking for a typo that is not there.
pub fn operation_for<'a>(connector: &'a Connector, id: &str) -> Result<&'a Operation> {
    if let Some(operation) = connector.operation(id) {
        return Ok(operation);
    }
    if let Some(kind) = inbound_member_kind(connector, id) {
        bail!(
            "connector `{}`: `{id}` is {kind}, not an operation. A binding declares and is never \
             invoked; publishing one as an operation would be an event dressed up as a pollable op, \
             which this pipeline refuses rather than degrades to",
            connector.id
        );
    }
    bail!(
        "connector `{}` has no operation `{id}` to describe",
        connector.id
    );
}

/// Which inbound member kind `name` denotes, or `None` when it denotes neither.
///
/// The three member kinds share one namespace per service, so at most one of these matches.
fn inbound_member_kind(connector: &Connector, name: &str) -> Option<&'static str> {
    if connector.event(name).is_some() {
        return Some("an event");
    }
    if connector.channel(name).is_some() {
        return Some("a channel binding");
    }
    None
}

/// The host a call reaches, taken from a base URL with its templating intact.
///
/// `https://{subdomain}.zendesk.com` yields `{subdomain}.zendesk.com`: the tenant is the operator's
/// to choose, and substituting a placeholder here would invent one. A caller reads this to decide
/// whether their egress policy admits the call, and a pattern is the honest answer to that.
///
/// Always called with the **operation's service's** base URL, never the connector's: a
/// multi-service connector reaches a different host per service, and the union would be a wider
/// egress claim than any single call makes.
pub fn host_of(base_url: &str) -> Result<&str> {
    let after_scheme = base_url
        .split_once("://")
        .map_or(base_url, |(_scheme, rest)| rest);
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    if host.is_empty() {
        bail!("base URL `{base_url}` names no host, so nothing can say which host it reaches");
    }
    Ok(host)
}

/// The operation's credentials as alternatives (OR) of mechanisms (AND), as plain data.
///
/// Flattening is the thing to avoid: babelforce's `accessId` + `accessToken` travel **together** on
/// one request and are an alternative to OAuth2, so a flat list of three names would tell a caller
/// that any one of them authenticates, which is false in both directions.
///
/// Resolved through [`Connector::effective_auth`], never by reading `Operation::auth` directly: an
/// operation that declares nothing inherits the connector default, and one that declares an
/// explicit empty list inherits nothing.
pub fn credential_mechanisms<'a>(
    connector: &'a Connector,
    operation: &'a Operation,
) -> Vec<Vec<&'a str>> {
    connector
        .effective_auth(operation)
        .iter()
        .map(|mechanism| mechanism.iter().map(String::as_str).collect())
        .collect()
}
