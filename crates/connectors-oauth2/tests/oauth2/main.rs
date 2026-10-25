//! `story:oauth2-grant-core`: the client side of OAuth 2.0 against a fake authorization server
//! on `127.0.0.1` behind TLS with a per-test root (see [`support`]).
mod adversary;
mod authorize;
mod device;
mod exchange;
mod failures;
mod receivers;
mod refresh;
mod support;
