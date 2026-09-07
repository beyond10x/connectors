//! Credential-free hosted completion page and catalogue-owned help.

use service::HostedCompletionPage;

use super::hosted::html_escape;

pub(super) fn completion_page(
    provider: &catalog::Provider,
    credential: &str,
    origins: &[String],
    expires_at_unix_ms: u64,
) -> HostedCompletionPage {
    let title = html_escape(provider.id);
    let binding = format!("credential.{credential}");
    let field = provider.config.iter().find(|field| {
        field.secret && (field.binds == binding || field.also_binds.contains(&binding.as_str()))
    });
    let label = html_escape(field.map_or(credential, |field| field.label));
    let auth_description = credential_description(provider.id, credential);
    let mut help = field.map_or_else(String::new, |field| html_escape(field.help));
    if !auth_description.is_empty() && field.is_none_or(|field| field.help != auth_description) {
        if !help.is_empty() {
            help.push_str("</p><p>");
        }
        help.push_str(&html_escape(&auth_description));
    }
    let documentation = field
        .and_then(|field| field.docs_url)
        .and_then(documentation_link)
        .unwrap_or_default();
    let destinations = if origins.is_empty() {
        String::new()
    } else {
        format!(
            "<p>Provider destination: <strong>{}</strong></p>",
            html_escape(&origins.join(", "))
        )
    };
    HostedCompletionPage {
        title: format!("Connect {title}"),
        html: format!(
            r#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Connect {title}</title>
<style>body{{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}}label,input,button{{display:block;width:100%;box-sizing:border-box}}input,button{{padding:.8rem;margin-top:.5rem}}button{{margin-top:1rem}}a{{color:#93c5fd}}</style>
<h1>Connect {title}</h1>{destinations}<p>Enter the provider credential once. Connectors verifies it with the provider and stores it in the configured credential store.</p>
<div id="credential-help"><p>{help}</p>{documentation}</div>
<p id="expiry" role="timer"></p><form data-expires-at="{expires_at_unix_ms}"><label>{label}<input name="credential" type="password" autocomplete="off" aria-describedby="credential-help" maxlength="8192" required></label><button>Connect</button></form><p id="status" role="status"></p>
<script>{SCRIPT}</script>"#
        ),
    }
}

pub(super) fn credential_description(provider: &str, credential: &str) -> String {
    // Authentication descriptions belong to the canonical catalogue, including profiles without
    // a form field. Reading that reviewed document avoids inventing provider-specific UI copy.
    catalog::reader::provider(provider)
        .and_then(|provider| serde_json::from_str::<serde_json::Value>(provider.document()).ok())
        .and_then(|document| {
            document.get("auth")?.as_array()?.iter().find_map(|auth| {
                (auth.get("name")?.as_str()? == credential)
                    .then(|| auth.get("description")?.as_str().map(str::to_owned))?
            })
        })
        .unwrap_or_default()
}

pub(super) fn documentation_link(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(format!(
        "<p><a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">Credential setup documentation</a></p>",
        html_escape(url.as_str())
    ))
}

// The script contains no provider-specific branch and reads only nonsecret page metadata.
const SCRIPT: &str = r#"
const form=document.querySelector('form');
const status=document.querySelector('#status');
const expiry=document.querySelector('#expiry');
const button=document.querySelector('button');
const field=form.elements.credential;
const expiresAt=Number(form.dataset.expiresAt);
const capability=new URL(location.href).hash.match(/^#token=([A-Za-z0-9_-]{32,256})$/)?.[1];
history.replaceState(null,'',location.pathname);
const expiredMessage='This Connect form has expired. Return to the application and start Connect again.';
const uncertainMessage='The result could not be confirmed. Return to the application and check your connections before starting Connect again.';
const messages={
  'connect-session-credential-rejected':'The provider did not accept this credential. Check that it is a current API credential for the displayed destination, then start Connect again.',
  'connect-session-credential-permission':'The provider refused the verification read with HTTP 403. Check the service account permissions in the setup documentation, then start Connect again.',
  'connect-session-provider-unreachable':'Connectors could not reach the provider. Ask the deployment operator to check destination policy, network access and TLS trust, then start Connect again.',
  'connect-session-provider-rate-limited':'The provider rate limited credential verification. Wait before starting Connect again.',
  'connect-session-provider-refused':'The provider refused credential verification. Ask the deployment operator to check the reported upstream HTTP status, then start Connect again.',
  'connect-session-provider-response-invalid':'The provider response exceeded the supported size. Ask the deployment operator to check credential verification.',
  'connect-session-verification-unavailable':'Credential verification could not be prepared. Ask the deployment operator to check the declared provider configuration.',
  'connect-session-custody-unavailable':'Credential storage failed. Ask the deployment operator to check the custody failure stage before starting Connect again.',
  'connect-session-custody-unconfirmed':'Saving the Connection could not be confirmed. Check your connections and ask the deployment operator to inspect custody recovery before starting Connect again.',
  'connect-session-unavailable':'The connection service or credential store is unavailable. Start Connect again later.',
  'connect-session-invalid':'The credential submission was invalid. Check the API credential and start Connect again.'
};
let submitted=false;
function updateExpiry(){
  if(submitted)return;
  const seconds=Math.max(0,Math.ceil((expiresAt-Date.now())/1000));
  expiry.textContent=seconds>0?'This form expires in '+seconds+' seconds.':'This form has expired.';
  if(seconds===0){field.value='';field.disabled=true;button.disabled=true;status.textContent=expiredMessage;}
}
updateExpiry();
const timer=setInterval(updateExpiry,1000);
form.addEventListener('submit',async event=>{
  event.preventDefault();
  if(submitted)return;
  if(Date.now()>=expiresAt){updateExpiry();return;}
  if(!capability){status.textContent='This Connect form has no valid session capability. Return to the application and start Connect again.';return;}
  const value=field.value;
  if(!value||value.length>8192){status.textContent='Check the credential value.';return;}
  submitted=true;clearInterval(timer);field.value='';field.disabled=true;button.disabled=true;
  status.textContent='Verifying and saving the credential…';
  try{
    const response=await fetch(location.pathname,{method:'POST',redirect:'error',headers:{'Content-Type':'application/octet-stream','X-Connect-Session':capability},body:value});
    const result=await response.json().catch(()=>null);
    if(response.ok){status.textContent=result?.accepted===true?'Account connected. You may close this tab.':uncertainMessage;return;}
    if(Object.hasOwn(messages,result?.error)){status.textContent=messages[result.error];return;}
    status.textContent=Date.now()>=expiresAt?expiredMessage:'This Connect session was refused. Return to the application and start Connect again.';
  }catch{status.textContent=uncertainMessage;}
});
"#;
