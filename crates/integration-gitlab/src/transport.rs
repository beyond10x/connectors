//! GitLab request encoding and bounded response decoding above the injected egress port.

use std::collections::BTreeMap;

use connector_secrets::Secret;
use serde::Deserialize;
use serde_json::Value;
use service::EgressHttpResponse;
use zeroize::Zeroizing;

use crate::backend::GitlabError;

pub(crate) fn http_request(
    method: &str,
    target: url::Url,
    headers: BTreeMap<String, String>,
    body: Option<String>,
) -> connector_resolve::Request {
    connector_resolve::Request {
        method: method.to_owned(),
        url: target.into(),
        headers,
        body,
    }
}

pub(crate) fn form_body(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs.iter().copied())
        .finish()
}

pub(crate) fn bearer_headers(token: &Secret) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "authorization".to_owned(),
        format!("Bearer {}", token.expose_secret()),
    )])
}

pub(crate) fn decode_response<T: for<'de> Deserialize<'de>>(
    response: EgressHttpResponse,
) -> Result<T, GitlabError> {
    let value = decode_value_response(response)?;
    serde_json::from_value(value).map_err(|_| GitlabError::new("provider-response"))
}

pub(crate) fn decode_value_response(response: EgressHttpResponse) -> Result<Value, GitlabError> {
    if !response.is_success() {
        return Err(GitlabError::new("provider-refused"));
    }
    let bytes = Zeroizing::new(response.body);
    serde_json::from_slice(&bytes).map_err(|_| GitlabError::new("provider-response"))
}

pub(crate) fn decode_page_response(
    response: EgressHttpResponse,
) -> Result<(Value, Option<u64>), GitlabError> {
    let next_page = response
        .header("x-next-page")
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok());
    decode_value_response(response).map(|value| (value, next_page))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_forms_encode_secret_delimiters_without_logging_values() {
        assert_eq!(
            form_body(&[("client_id", "id"), ("client_secret", "a+b&c=d")]),
            "client_id=id&client_secret=a%2Bb%26c%3Dd"
        );
    }

    #[test]
    fn page_decoding_reads_only_the_selected_cursor_header() {
        let response = EgressHttpResponse {
            status: 200,
            headers: BTreeMap::from([("x-next-page".to_owned(), "7".to_owned())]),
            body: br#"[{"id":1}]"#.to_vec(),
        };
        let (value, next_page) = decode_page_response(response).expect("bounded response");
        assert_eq!(value, serde_json::json!([{"id": 1}]));
        assert_eq!(next_page, Some(7));
    }
}
