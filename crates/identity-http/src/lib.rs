#![forbid(unsafe_code)]

//! HTTP Identity verification adapter.

mod adapter;
#[cfg(feature = "local-identity")]
mod local_identity;

// A hosted Connector resolves every access token against this origin. Accepting a plaintext one in
// a deployment puts every access token and every authority answer on the wire in the clear, which
// is a total authority bypass: anybody on the path reads a token and is then that principal. The
// exception is therefore not a flag a deployment leaves off — enabling it in the profile every
// deployment builds is this error, so no released binary can contain the code at all.
#[cfg(all(feature = "local-identity", not(debug_assertions)))]
compile_error!(
    "feature `local-identity` lets the hosted Connector resolve access tokens over plaintext HTTP, \
     which puts every access token and every authority answer on the network in the clear and is a \
     complete authority bypass for this service. It is admitted only in a debug-profile build \
     whose listener and whose Identity origin are both loopback. A deployment builds --release, \
     where enabling it is this compile error."
);

pub use adapter::{IdentityHttpVerifier, IdentityVerifierConfigError};
#[cfg(feature = "local-identity")]
pub use local_identity::admitted as local_identity_admitted;
