//! A plaintext Identity origin for a hosted Connector nothing but this machine can reach.
//!
//! The hosted Connector resolves every access token by asking Identity, over HTTPS, whether the
//! token is real and what it may do. The local process stack runs Identity on `127.0.0.1` with no
//! certificate any webpki root chain would accept, so the hosted surface — token introspection,
//! principal context, description leases, scope checks — could not be exercised locally at all.
//! This module admits exactly that one loopback case.
//!
//! A deployed Connector that accepted a plaintext Identity origin would be a total authority
//! bypass: every access token and every authority answer would cross the network in the clear, so
//! anybody on the path could read a token and then be that principal. The exception is therefore
//! refused three independent ways, and each one alone is sufficient:
//!
//! 1. It is compiled only behind the non-default `local-identity` feature, and `lib.rs` raises
//!    `compile_error!` when that feature is combined with a profile that has `debug_assertions`
//!    off. Every release build clears `debug_assertions`, so a deployed binary cannot contain this
//!    code — the attempt does not produce a flag to leave off, it produces a build failure.
//! 2. `foundation/connectors/Dockerfile` passes no `--features` and builds `--release`, so the
//!    image is the default feature set even before rule 1 applies.
//! 3. [`admitted`] refuses unless this process both listens on a loopback address and resolves a
//!    loopback plaintext Identity origin. `connectors-runtime` checks it before binding, so a
//!    feature-built binary pointed at a routable address exits instead of serving, and
//!    [`admitted_origin`] is re-checked where the verifier's HTTP client is built.
//!
//! No environment variable, configuration file, or request can turn any of the three off.

use std::net::SocketAddr;

use url::{Host, Url};

/// Whether an Identity origin is a closed plaintext loopback origin.
///
/// A plaintext origin with a routable host is reachable from another machine, which is the case
/// this whole module exists to keep impossible. Credentials, a path, a query or a fragment would
/// mean the value is not an origin at all, and the deployed constructor refuses those too.
pub(crate) fn admitted_origin(origin: &Url) -> bool {
    origin.scheme() == "http"
        && origin.username().is_empty()
        && origin.password().is_none()
        && (origin.path() == "/" || origin.path().is_empty())
        && origin.query().is_none()
        && origin.fragment().is_none()
        && origin.host().is_some_and(|host| match host {
            Host::Ipv4(address) => address.is_loopback(),
            Host::Ipv6(address) => address.is_loopback(),
            Host::Domain(name) => name == "localhost",
        })
}

/// Whether this process is a loopback development hosted Connector.
///
/// Both facts have to hold. A loopback Identity origin behind a routable listener still serves
/// every other machine on the network, and a loopback listener resolving a routable plaintext
/// Identity still sends access tokens across a network in the clear. Only the pair describes a
/// process that nothing but this machine participates in.
#[must_use]
pub fn admitted(listen: &SocketAddr, identity_origin: &Url) -> bool {
    listen.ip().is_loopback() && admitted_origin(identity_origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(value: &str) -> Url {
        Url::parse(value).expect("test origin parses")
    }

    fn listen(value: &str) -> SocketAddr {
        value.parse().expect("test listener parses")
    }

    #[test]
    fn only_a_loopback_listener_resolving_a_loopback_plaintext_identity_is_admitted() {
        assert!(admitted(
            &listen("127.0.0.1:18080"),
            &origin("http://127.0.0.1:18085")
        ));
        assert!(admitted(
            &listen("[::1]:18080"),
            &origin("http://[::1]:18085")
        ));
        assert!(admitted(
            &listen("127.0.0.1:18080"),
            &origin("http://localhost:18085")
        ));
        // A routable listener serving the whole network is refused even with a loopback Identity.
        assert!(!admitted(
            &listen("0.0.0.0:8080"),
            &origin("http://127.0.0.1:18085")
        ));
        // A plaintext Identity a second machine can address is refused however this process binds.
        assert!(!admitted(
            &listen("127.0.0.1:18080"),
            &origin("http://identity.example.test")
        ));
        assert!(!admitted(
            &listen("127.0.0.1:18080"),
            &origin("http://10.0.0.4:18085")
        ));
    }

    #[test]
    fn an_https_origin_is_not_this_exception() {
        // The deployed path stays the deployed path: this predicate only ever opens the plaintext
        // door, so an HTTPS origin never reaches the client built without `https_only`.
        assert!(!admitted_origin(&origin("https://127.0.0.1:18085")));
        assert!(!admitted_origin(&origin("https://identity.example.test")));
    }

    #[test]
    fn a_loopback_url_carrying_more_than_an_origin_is_refused() {
        assert!(!admitted_origin(&origin("http://127.0.0.1:18085/v1")));
        assert!(!admitted_origin(&origin("http://127.0.0.1:18085/?a=b")));
        assert!(!admitted_origin(&origin("http://127.0.0.1:18085/#f")));
        assert!(!admitted_origin(&origin("http://user@127.0.0.1:18085")));
    }
}
