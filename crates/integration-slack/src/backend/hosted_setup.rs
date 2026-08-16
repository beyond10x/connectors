//! Capability-authenticated hosted Slack credential acquisition.

use connector_secrets::Secret;
use serde::Deserialize;
use service::{HostedCompletionError, HostedCompletionPage};

use super::{SlackCredentials, SlackError};

const MAX_HOSTED_SUBMISSION_BYTES: usize = 8 * 1024;
pub(super) const MAX_AUTH_TEST_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
struct AuthTestResponse {
    ok: bool,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

pub(super) fn completion_page() -> HostedCompletionPage {
    HostedCompletionPage {
        title: "Connect Slack",
        html: HOSTED_SETUP_PAGE,
    }
}

pub(super) fn parse_hosted_submission(
    bytes: &[u8],
) -> Result<SlackCredentials, HostedCompletionError> {
    if bytes.is_empty() || bytes.len() > MAX_HOSTED_SUBMISSION_BYTES {
        return Err(HostedCompletionError::Invalid);
    }
    let value = std::str::from_utf8(bytes).map_err(|_| HostedCompletionError::Invalid)?;
    let mut values = value.split('\n');
    let app = values.next().ok_or(HostedCompletionError::Invalid)?.trim();
    let bot = values.next().ok_or(HostedCompletionError::Invalid)?.trim();
    let user = values.next().ok_or(HostedCompletionError::Invalid)?.trim();
    if values.next().is_some()
        || !valid_slack_token(app, "xapp-")
        || !valid_slack_token(bot, "xoxb-")
        || !valid_slack_token(user, "xoxp-")
    {
        return Err(HostedCompletionError::Invalid);
    }
    Ok(SlackCredentials {
        app_token: Secret::new(app),
        bot_token: Some(Secret::new(bot)),
        user_token: Some(Secret::new(user)),
    })
}

fn valid_slack_token(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|suffix| !suffix.is_empty())
        && value.len() <= 2048
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\\'))
}

pub(super) fn valid_hosted_capability(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn hosted_completion_error(error: SlackError) -> HostedCompletionError {
    match error.code {
        "credential-shape"
        | "credential-verify-refused"
        | "credential-workspace"
        | "connection-conflict"
        | "app-token-conflict" => HostedCompletionError::Refused,
        _ => HostedCompletionError::Unavailable,
    }
}

pub(super) fn classify_auth_test_response(
    status: reqwest::StatusCode,
    content_length: Option<u64>,
    bytes: &[u8],
) -> Result<String, SlackError> {
    if !status.is_success()
        || content_length.is_some_and(|length| length > MAX_AUTH_TEST_RESPONSE_BYTES as u64)
        || bytes.len() > MAX_AUTH_TEST_RESPONSE_BYTES
    {
        return Err(SlackError::new("credential-verify-unavailable"));
    }
    let verified: AuthTestResponse = serde_json::from_slice(bytes)
        .map_err(|_| SlackError::new("credential-verify-unavailable"))?;
    if !verified.ok {
        return if verified.error.as_deref().is_some_and(|error| {
            matches!(
                error,
                "account_inactive"
                    | "invalid_auth"
                    | "invalid_token"
                    | "not_authed"
                    | "token_expired"
                    | "token_revoked"
            )
        }) {
            Err(SlackError::new("credential-verify-refused"))
        } else {
            Err(SlackError::new("credential-verify-unavailable"))
        };
    }
    verified
        .team_id
        .filter(|team| {
            (2..=64).contains(&team.len()) && team.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .ok_or_else(|| SlackError::new("credential-verify-unavailable"))
}

pub(super) fn random_capability() -> Result<String, SlackError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| SlackError::new("randomness"))?;
    Ok(hex::encode(bytes))
}

pub(super) const HOSTED_SETUP_PAGE: &str = r#"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width">
<title>Connect Slack</title>
<style>body{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}input,button{box-sizing:border-box;width:100%;padding:.8rem;margin:.5rem 0}button{cursor:pointer}.hint{color:#aaa}</style>
<h1>Connect Slack</h1>
<p>These credentials go directly to hosted Connectors and are committed to its Secret Store. Zwirn never receives them.</p>
<form>
<label>Socket Mode app token<input name="app" type="password" autocomplete="off" spellcheck="false" maxlength="2048" placeholder="xapp-…" required></label>
<label>Companion bot token<input name="bot" type="password" autocomplete="off" spellcheck="false" maxlength="2048" placeholder="xoxb-…" required></label>
<label>Your user token<input name="user" type="password" autocomplete="off" spellcheck="false" maxlength="2048" placeholder="xoxp-…" required></label>
<button type="submit">Connect</button>
</form>
<p class="hint">The bot token handles mentions and replies. The user token performs reads as you. Verifying the Slack workspace and committing all three credentials can take a moment.</p>
<p id="status" role="status" aria-live="polite"></p>
<script>
const form=document.querySelector('form');
const button=form.querySelector('button');
const status=document.querySelector('#status');
const capability=new URLSearchParams(location.hash.slice(1)).get('token')||'';
const validCapability=/^[A-Fa-f0-9]{64}$/.test(capability);
history.replaceState(null,'',location.pathname+'#ready');
const validToken=(value,prefix)=>value.startsWith(prefix)&&value.length>prefix.length&&value.length<=2048&&[...value].every(character=>{const code=character.charCodeAt(0);return code>=33&&code<=126&&character!=='"'&&character!=='\\';});
if(!validCapability){button.disabled=true;status.textContent='This Connect Session link is incomplete or expired. Start Connect again to get a fresh link.';}
form.addEventListener('submit',async event=>{
  event.preventDefault();
  if(!validCapability)return;
  const fields=[form.elements.app,form.elements.bot,form.elements.user];
  const values=fields.map(field=>field.value.trim());
  if(!validToken(values[0],'xapp-')||!validToken(values[1],'xoxb-')||!validToken(values[2],'xoxp-')){
    status.textContent='Check all three token formats: app xapp-, bot xoxb-, and user xoxp-.';
    return;
  }
  const body=values.join('\n');
  fields.forEach(field=>field.value='');
  button.disabled=true;
  status.textContent='Checking the Slack workspace and saving credentials in the Secret Store. This may take a moment…';
  try{
    const response=await fetch(location.pathname,{method:'POST',headers:{'Content-Type':'application/octet-stream','X-Connect-Session':capability},body});
    if(response.ok){status.textContent='Slack connected. You may close this tab.';return;}
    if(response.status===400){status.textContent='The setup was invalid. Check the xapp-, xoxb-, and xoxp- token formats.';}
    else if(response.status===403){status.textContent='Slack refused these credentials, the tokens name different workspaces, or this workspace conflicts with an existing connection.';}
    else if(response.status===503){status.textContent='Slack or the Secret Store is unavailable. Start Connect again after the deployment is healthy.';}
    else{status.textContent='Slack connection was refused.';}
  }catch{
    status.textContent='Hosted Connectors is unavailable. Start Connect again when the deployment is reachable.';
  }
  button.disabled=false;
});
</script>"#;
