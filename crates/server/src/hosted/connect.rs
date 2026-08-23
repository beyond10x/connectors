//! Browser-facing connect-session completion: the OAuth landing and the credential
//! completion page. Moved verbatim from `hosted.rs` when the module hit its size
//! fence (S-053 + S-054 coexistence, 2026-08-24); these handlers speak to a person
//! mid-connect-session, not to the operation admission seam.

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OAuthCallbackQuery {
    state: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

pub(super) async fn oauth_callback(
    State(state): State<HostedState>,
    AxumPath(integration_ref): AxumPath<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Response {
    let valid = |value: &str, maximum: usize| {
        !value.is_empty()
            && value.len() <= maximum
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    if !valid(&query.state, 256)
        || query.code.as_deref().is_some_and(|code| !valid(code, 1024))
        || query
            .error
            .as_deref()
            .is_some_and(|error| !valid(error, 128))
        || query.code.is_some() == query.error.is_some()
        || !valid(&integration_ref, 64)
        || !state
            .backend
            .owns_hosted_oauth_state(&integration_ref, &query.state)
    {
        return secure_completion_response(error(
            StatusCode::BAD_REQUEST,
            "oauth-callback-invalid",
        ));
    }
    let result = state
        .backend
        .complete_hosted_oauth(
            &integration_ref,
            &query.state,
            query.code.as_deref(),
            query.error.as_deref(),
        )
        .await;
    let (status, message) = match result {
        Ok(()) => (StatusCode::OK, "Account connected. You may close this tab."),
        Err(HostedCompletionError::Refused | HostedCompletionError::Invalid) => (
            StatusCode::FORBIDDEN,
            "Authorization was refused. Close this tab and start Connect again.",
        ),
        Err(HostedCompletionError::NotFound) => (
            StatusCode::NOT_FOUND,
            "This authorization is unknown or expired.",
        ),
        Err(HostedCompletionError::Unavailable) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Authorization is temporarily unavailable. Start Connect again later.",
        ),
    };
    secure_completion_response((status, Html(format!(
        "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Connect account</title><style>body{{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}}</style><h1>Connect account</h1><p>{message}</p>"
    )))
        .into_response())
}

pub(super) async fn completion_page(
    State(state): State<HostedState>,
    AxumPath(session_ref): AxumPath<String>,
) -> Response {
    let response = match state.backend.hosted_completion_page(&session_ref) {
        Ok(page) => Html(page.html).into_response(),
        Err(HostedCompletionError::NotFound | HostedCompletionError::Refused) => {
            error(StatusCode::NOT_FOUND, "connect-session-not-found")
        }
        Err(HostedCompletionError::Invalid) => {
            error(StatusCode::BAD_REQUEST, "connect-session-invalid")
        }
        Err(HostedCompletionError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connect-session-unavailable",
        ),
    };
    secure_completion_response(response)
}

pub(super) async fn complete_session(
    State(state): State<HostedState>,
    AxumPath(session_ref): AxumPath<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let capability = headers
        .get("x-connect-session")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some("application/octet-stream")
        || capability.is_none()
    {
        return secure_completion_response(error(
            StatusCode::BAD_REQUEST,
            "connect-session-invalid",
        ));
    }
    let submission = match read_completion_submission(body).await {
        Ok(submission) if !submission.is_empty() => submission,
        Ok(_) | Err(()) => {
            return secure_completion_response(error(
                StatusCode::BAD_REQUEST,
                "connect-session-invalid",
            ));
        }
    };
    let result = state
        .backend
        .complete_hosted_session(
            &session_ref,
            capability.expect("checked capability"),
            submission,
        )
        .await;
    let response = match result {
        Ok(()) => Json(serde_json::json!({"accepted": true})).into_response(),
        Err(HostedCompletionError::NotFound | HostedCompletionError::Refused) => {
            error(StatusCode::FORBIDDEN, "connect-session-refused")
        }
        Err(HostedCompletionError::Invalid) => {
            error(StatusCode::BAD_REQUEST, "connect-session-invalid")
        }
        Err(HostedCompletionError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connect-session-unavailable",
        ),
    };
    secure_completion_response(response)
}

pub(super) async fn read_completion_submission(
    body: Body,
) -> Result<HostedCompletionSubmission, ()> {
    // Reserving the full admitted bound prevents Vec growth from leaving an earlier copy of
    // credential bytes in freed heap storage. HTTP frame buffers remain transport-owned and are
    // copied exactly once, directly into the zeroizing-owned allocation.
    let mut submission = HostedCompletionSubmission::with_capacity(MAX_COMPLETION_BYTES);
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ())?;
        if submission.len().saturating_add(chunk.len()) > MAX_COMPLETION_BYTES {
            return Err(());
        }
        if !submission.extend_from_slice(&chunk) {
            return Err(());
        }
    }
    Ok(submission)
}

fn secure_completion_response(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("static header"),
    );
    headers.insert("pragma", "no-cache".parse().expect("static header"));
    headers.insert(
        "referrer-policy",
        "no-referrer".parse().expect("static header"),
    );
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    headers.insert(
        "content-security-policy",
        "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; form-action 'self'; base-uri 'none'"
            .parse()
            .expect("static header"),
    );
    response
}
