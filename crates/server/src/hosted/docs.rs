//! The hosted server's own machine-readable contract (S-067).
//!
//! `GET {base_path}/openapi.json` serves a committed OpenAPI 3.1 document, unauthenticated
//! and verbatim: the artifact beside this module is the contract, embedded at compile time
//! and never derived from the running code. Drift is caught by the test suite instead —
//! every request example in the document must be accepted by the exact `protocol` types the
//! routes deserialize with, and every documented route must exist in the real router
//! (`hosted/tests/docs.rs`). The answer is immutable per build, so the `ETag` is the
//! SHA-256 of the exact served bytes.

use std::sync::OnceLock;

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::Digest as _;

/// The committed contract, served byte for byte.
const OPENAPI_JSON: &str = include_str!("docs/openapi.json");

/// Quoted lowercase-hex SHA-256 of [`OPENAPI_JSON`], computed once per process.
fn content_hash_etag() -> &'static str {
    static ETAG: OnceLock<String> = OnceLock::new();
    ETAG.get_or_init(|| {
        let digest = sha2::Sha256::digest(OPENAPI_JSON.as_bytes());
        let mut etag = String::with_capacity(66);
        etag.push('"');
        for byte in digest {
            etag.push_str(&format!("{byte:02x}"));
        }
        etag.push('"');
        etag
    })
}

/// `GET /openapi.json`: the committed contract, no authentication, content-hash `ETag`.
pub(super) async fn openapi() -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::ETAG, content_hash_etag()),
        ],
        OPENAPI_JSON,
    )
        .into_response()
}
