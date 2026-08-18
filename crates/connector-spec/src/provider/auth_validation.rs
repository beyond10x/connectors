use super::*;

/// Checks a header credential's literal prefix (C-184).
///
/// The prefix is the closest this repository gets to authoring a credential value — it is the text
/// immediately before one — so it is the seam where "no credential value" has to be enforced rather
/// than assumed. Everything refused here is refused because it is either an attempt to reach the
/// secret through the prefix, or a request the connector did not describe.
///
/// # The separator rule is the load-bearing one
///
/// **The host appends the credential to the prefix with nothing in between.** So a prefix that ends
/// in an alphanumeric would be glued onto the secret — `SSWS` + `<token>` is `SSWS<token>`, a header
/// no vendor accepts. A well-formed prefix therefore *always* ends in a separator, and requiring
/// that catches two failures one rule apart:
///
/// - **A pasted credential.** `Bearer sk-live-51H8…` ends in an opaque blob, so it is refused. This
///   is the case a `CREDENTIAL_VALUE_PREFIXES` check would only half-catch: that list is matched
///   with `starts_with` and holds `"bearer "`, `"basic "`, `"token "`, `"apikey "`, `"digest "`, so
///   it would refuse a pasted `Bearer …` but not a pasted `SSWS …` or `OAuth …` — one of C-184's
///   three vendors, not three. The separator rule is indifferent to the scheme word and catches all
///   of them.
/// - **A missing trailing space.** `prefix = "SSWS"` was previously uncatchable, and
///   `crates/connector-flux/tests/okta_connector.rs` says so in as many words. It is the same rule:
///   `SSWS` does not end in a separator.
///
/// `Token token=` passes, because `=` *is* a separator — which is the point. The rule is about the
/// boundary between connector data and the secret, not about the vendor's choice of syntax.
fn validate_auth_prefix(
    connector: &Connector,
    method: &AuthMethod,
    prefix: &str,
    problems: &mut Vec<String>,
) {
    if prefix.is_empty() {
        return;
    }
    let name = method.name.as_str();
    let folded = prefix.to_ascii_lowercase();

    // A prefix is emitted as a literal and nothing interpolates it, so a marker is either a broken
    // request or an author reaching for the value. Both end the same way: the only spelling that
    // "works" is the credential itself, pasted in.
    if let Some(marker) = RESOLUTION_MARKERS
        .iter()
        .find(|marker| folded.contains(*marker))
    {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` spelling {marker:?}, but a prefix is a \
             literal and nothing interpolates it — the vendor would receive those characters. The \
             prefix carries the vendor's scheme word (`SSWS `, `Token token=`); the credential is \
             appended by the host and is never written here"
        ));
    }

    // Every credential the connector declares, not just this one, and folded — matching the sibling
    // `credential_shaped_value`, which has always iterated `connector.auth`. A prefix naming another
    // credential's variable is the same mistake spelled sideways, and case never made it less of one.
    for other in &connector.auth {
        if folded.contains(&other.name.to_ascii_lowercase()) {
            problems.push(format!(
                "credential {name:?} declares a header `prefix` naming credential {:?}. A prefix is \
                 a literal, not a reference — nothing resolves the name, and the value that would \
                 make it work is one this file must never hold",
                other.name
            ));
        }
        for key in other.env.iter().chain(&other.user_env) {
            if !key.trim().is_empty() && folded.contains(&key.to_ascii_lowercase()) {
                problems.push(format!(
                    "credential {name:?} declares a header `prefix` naming the environment variable \
                     {key:?}, which resolves credential {:?}. A prefix is emitted as a literal, so \
                     the name would travel as text and the value it stands for must never be \
                     written here at all",
                    other.name
                ));
            }
        }
    }

    // See the separator rule on this function. `SSWS ` ends in a space, `Token token=` in `=`; a
    // prefix ending in an alphanumeric would be concatenated onto the secret.
    if prefix
        .chars()
        .next_back()
        .is_some_and(|last| last.is_ascii_alphanumeric())
    {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` ending in an alphanumeric character. \
             The host appends the credential directly, so this would send the prefix and the secret \
             glued together. A scheme word ends in a separator — `\"SSWS \"` with the trailing \
             space, `\"Token token=\"` with the `=`. If the text after the scheme word is the \
             credential itself, it does not belong in this file at all"
        ));
    }

    // A prefix of only spaces contributes no scheme word and puts leading whitespace in front of the
    // credential, which `field-content` does not allow at the edges of a header value.
    if prefix.trim().is_empty() {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` of only whitespace, which carries no \
             scheme word and would send a header value beginning with a space (RFC 9110 §5.5 \
             field-content permits no leading or trailing whitespace). Omit `prefix` for a header \
             whose whole value is the secret"
        ));
    }

    // **The whitespace-corruption class, found by C-184's own review.**
    //
    // The separator rule above catches a prefix with *no* trailing separator. It does not catch one
    // with too many, and neither did anything else: `"SSWS  "` and `" SSWS "` both loaded, and both
    // send a header the vendor answers `401` to. Worse, nothing downstream could catch them either —
    // a connector's own suite asserts the prefix against a constant in the same file, so an author
    // editing both together leaves every test green.
    //
    // Deliberately narrow. It refuses *whitespace* corruption, which is an HTTP hygiene rule that
    // holds for every vendor, and says nothing about repeated punctuation: `"Token token=="` is
    // wrong for PagerDuty but this model has no basis to declare `==` wrong in general, and guessing
    // at a vendor's syntax is how a checker starts refusing correct connectors.
    if prefix.starts_with([' ', '\t']) {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` beginning with whitespace. It would send \
             a header value whose first character is a space, which RFC 9110 §5.5 field-content does \
             not permit at the edges — and which a vendor answers with `401` rather than a message \
             naming the space"
        ));
    }
    if let Some(run) = prefix
        .as_bytes()
        .windows(2)
        .position(|pair| pair.iter().all(|byte| matches!(byte, b' ' | b'\t')))
    {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` with two consecutive whitespace \
             characters at byte {run}. One separator is what the scheme word needs — `\"SSWS \"`, \
             `\"OAuth \"` — and a second one travels to the vendor verbatim, which answers `401` \
             without saying why. Nothing downstream catches this: a connector's own test asserts the \
             prefix against a constant beside it, so editing both together leaves the suite green"
        ));
    }

    // The value half of the grammar check `name` has had since C-3. A prefix reaches a header value
    // verbatim, so a CR or LF in one ends the header and begins another — header injection, from a
    // committed artifact. RFC 9110 §5.5 field-content: visible ASCII, plus space and horizontal tab.
    if let Some(bad) = prefix
        .chars()
        .find(|c| !matches!(c, ' ' | '\t') && !c.is_ascii_graphic())
    {
        problems.push(format!(
            "credential {name:?} declares a header `prefix` containing {bad:?}, which is not \
             visible ASCII, space or tab. A prefix is placed into a header value verbatim, so a \
             newline in one would end the header and begin another (RFC 9110 §5.5 field-content)"
        ));
    }
}

/// **One credential, one acquisition** (C-525) — the sibling of
/// [`validate_one_credential_disposition`], one axis over.
///
/// That function refuses two declarations of *where a credential appears in a response*. This one
/// refuses two declarations of *how a credential is obtained at all*: `[auth.oauth2]` says the host
/// runs a grant against the vendor's own OAuth endpoints, and an operation's `produces_credential`
/// naming the same credential says one of this connector's own calls mints it. Both cannot be true,
/// and the cost of not refusing falls on the emitter — `catalog::Acquisition` has one variant per
/// credential, so something downstream would have to *choose*, silently, and publish an acquisition
/// the author never declared.
///
/// The refusal carries the discriminator, because it is the one thing neither field's own
/// documentation supplies: an authorize or token endpoint is **never a connector operation**
/// (`AGENTS.md` § Authentication contract), so a credential obtained from the vendor's OAuth
/// endpoints is always the `[auth.oauth2]` case. `produces_credential` is for a credential minted by
/// an ordinary operation the connector genuinely declares — a session login, a device registration.
fn validate_one_credential_acquisition(
    connector: &Connector,
    method: &AuthMethod,
    problems: &mut Vec<String>,
) {
    if method.oauth2.is_none() && method.entry.is_none() {
        return;
    }
    let name = method.name.as_str();
    for operation in &connector.operations {
        let Some(produced) = &operation.produces_credential else {
            continue;
        };
        if produced.credential != method.name {
            continue;
        }
        if method.oauth2.is_some() {
            problems.push(format!(
                "credential {name:?} declares an `[auth.oauth2]` grant, and operation {:?} declares \
                 `produces_credential` naming it. Those state two different acquisitions of one \
                 credential — the host runs a token grant, or this connector's own call mints it — and \
                 exactly one governs. An authorize or token endpoint is never a connector operation, so \
                 a credential obtained from the vendor's OAuth endpoints declares only `[auth.oauth2]` \
                 and the minting operation is removed; `produces_credential` is for a credential minted \
                 by an ordinary operation this connector declares, and such a credential declares no \
                 `[auth.oauth2]` block",
                operation.id
            ));
        }
        if method.entry.is_some() {
            problems.push(format!(
                "credential {name:?} declares Connect Session `entry`, and operation {:?} declares \
                 `produces_credential` naming it. Those state two different acquisitions of one \
                 credential",
                operation.id
            ));
        }
    }
}

/// **An OAuth2 `token_endpoint` names a declared service, or the loader refuses it** (C-556).
///
/// The token endpoint may live on a different host from the authorize endpoint — Anthropic's
/// subscription flow authorizes on `claude.ai` and redeems its token on `platform.claude.com`. The
/// second host is declared by *reference*: [`OAuth2Spec::token_endpoint`] names a `[[services]]`
/// entry whose base URL the token exchange resolves against. That is what keeps the host set derived
/// from declared services rather than from a URL nothing admitted — so a name no service declares is
/// a typo pointing the token exchange at a host the allow-list never admitted, and it is refused
/// loudly. An empty value is the common case and means the exchange resolves against `endpoint`,
/// which needs no check here.
fn validate_one_credential_token_endpoint(
    connector: &Connector,
    method: &AuthMethod,
    problems: &mut Vec<String>,
) {
    let Some(spec) = &method.oauth2 else {
        return;
    };
    if spec.token_endpoint.is_empty()
        || connector
            .service_names()
            .contains(&spec.token_endpoint.as_str())
    {
        return;
    }
    let listed = connector.service_names().join(", ");
    problems.push(format!(
        "credential {:?} resolves its token exchange against token_endpoint {:?}, which is not a \
         declared service — a `token_endpoint` names the declared service whose base URL the token \
         exchange resolves against, and a name nothing declares reaches a host the allow-list never \
         admitted. This provider declares: {listed}. Leaving it empty is the other legal answer, and \
         means the token exchange resolves against the `endpoint` service",
        method.name, spec.token_endpoint
    ));
}

/// A nonstandard OAuth granted-scope location is a JSON Pointer to capability evidence, never a
/// second way to extract credential material. The token response has no schema in this repository,
/// so the loader can prove syntax and refuse the credential-field spellings that would turn a
/// capability slot into a secret-export slot; the runtime still treats the resolved value only as
/// a scope list and never as connection metadata.
fn validate_one_credential_scope_response_pointer(method: &AuthMethod, problems: &mut Vec<String>) {
    let Some(spec) = &method.oauth2 else {
        return;
    };
    let pointer = spec.scope_response_pointer.as_str();
    if pointer.is_empty() {
        return;
    }
    if !pointer.starts_with('/')
        || pointer.split('/').skip(1).any(|segment| {
            let bytes = segment.as_bytes();
            bytes.iter().enumerate().any(|(index, byte)| {
                *byte == b'~' && !matches!(bytes.get(index + 1), Some(b'0' | b'1'))
            })
        })
    {
        problems.push(format!(
            "credential {:?} declares scope_response_pointer {:?}, which is not an RFC 6901 JSON Pointer",
            method.name, pointer
        ));
        return;
    }

    let forbidden = [
        "access_token",
        "refresh_token",
        "client_secret",
        "client_assertion",
    ];
    if let Some(segment) = pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .find(|segment| forbidden.contains(&segment.as_str()))
    {
        problems.push(format!(
            "credential {:?} declares scope_response_pointer {:?}, whose segment {:?} names credential material rather than granted scopes",
            method.name, pointer, segment
        ));
    }
}

/// **A grant that carries a declared weakness must declare it** (C-440).
///
/// The closed [`AuthHazard`] vocabulary is only worth having if a connector cannot opt out of it by
/// silence. A host's deployment filter refuses on the *presence* of a hazard, so a connector that
/// allows the resource-owner password grant and declares no hazard is admitted by the very
/// deployment that set out to refuse exactly this — and the omission is one line nobody wrote rather
/// than anything a reviewer sees. `AGENTS.md` puts it generally: a marking that reads as a safety
/// decision while recording only that the question was never asked is worse than no marking at all.
///
/// The rule runs one way. A hazard on a credential whose grants do not include `password` is not
/// refused here: the vocabulary is about how a credential is *obtained*, and a future hazard need
/// not be an OAuth grant at all.
fn validate_one_credential_hazard(method: &AuthMethod, problems: &mut Vec<String>) {
    let Some(spec) = &method.oauth2 else {
        return;
    };
    if !spec.grants.contains(&OAuthGrant::Password) || method.hazard.is_some() {
        return;
    }
    problems.push(format!(
        "credential {:?} allows the `password` grant and declares no `hazard`. The resource owner's \
         own password reaching this host is a named weakness — RFC 9700 §2.4 says the grant MUST \
         NOT be used, and OAuth 2.1 drops it — and a host refuses it by declared property rather \
         than by connector name, so an undeclared one is admitted by the deployment that set out to \
         refuse it. Declare `hazard = {:?}` beside the grant, or remove `password` from `grants`",
        method.name,
        AuthHazard::ResourceOwnerSecretShared.word()
    ));
}

/// **Every auth workaround names a grant, says what was measured, and says who measured it when**
/// (C-440).
///
/// A workaround is asserted against a vendor's implementation and contradicted by that vendor's own
/// document, so the two provenance fields are what separate it from a guess that aged. They are
/// checked rather than trusted because the cost of an unattributed one is already on the record:
/// `providers/babelforce.toml` carries an open question to a vendor's API owners that nobody can now
/// answer, because whoever raised it did not write down what they had read.
fn validate_one_credential_workarounds(method: &AuthMethod, problems: &mut Vec<String>) {
    let name = method.name.as_str();
    if method.workarounds.is_empty() {
        return;
    }

    // A token endpoint the connector never declared is one nothing will ever read — the same rule
    // an `oauth.redirect_uri` binding already carries.
    if method.oauth2.is_none() {
        problems.push(format!(
            "credential {name:?} declares a `workarounds.token_endpoint` measurement and no \
             `[auth.oauth2]` block. A token-endpoint workaround describes an endpoint the host reaches to \
             run a grant, and a credential declaring no grant has no such endpoint, so nothing would \
             ever read it"
        ));
    }

    let mut seen: Vec<&str> = Vec::new();
    for workaround in &method.workarounds.token_endpoint {
        let grant = workaround.grant.trim();
        if grant.is_empty() {
            problems.push(format!(
                "credential {name:?} declares a `workarounds.token_endpoint` measurement with an empty \
                 `grant`. The vendor's own `grant_type` word is what says which of the endpoint's \
                 behaviours was measured; one endpoint answers differently per grant, which is the \
                 whole reason these are recorded one at a time"
            ));
        } else if seen.contains(&grant) {
            problems.push(format!(
                "credential {name:?} declares two `workarounds.token_endpoint` measurements for grant \
                 {grant:?}. That is two answers to one question, and nothing downstream could say \
                 which was measured last — record one, and supersede it in place when the vendor \
                 changes"
            ));
        }
        seen.push(grant);

        for (field, value) in [
            ("behaviour", workaround.behaviour.as_str()),
            ("attribution", workaround.attribution.as_str()),
        ] {
            if value.trim().is_empty() {
                problems.push(format!(
                    "credential {name:?}'s `workarounds.token_endpoint` measurement for grant \
                     {grant:?} declares an empty `{field}`. A workaround contradicts the vendor's own \
                     document, so a reader a year from now needs to know what was measured and \
                     against what — an unattributed one is indistinguishable from a guess"
                ));
            }
        }

        if !is_iso_date(&workaround.measured) {
            problems.push(format!(
                "credential {name:?}'s `workarounds.token_endpoint` measurement for grant {grant:?} \
                 declares `measured = {:?}`, which is not a date. It must be `YYYY-MM-DD`: a workaround \
                 is a timestamped claim about a vendor's running implementation, and \"recently\" \
                 does not let a reader decide whether it predates the release they are debugging",
                workaround.measured
            ));
        }
    }
}

/// Whether `value` is a calendar date spelled `YYYY-MM-DD`.
///
/// Deliberately a shape-and-range check rather than a date library: the question is whether an
/// author wrote a date at all, and a leap-year rule would be a dependency bought to reject
/// `2026-02-30` in a provenance field no arithmetic is ever done on.
fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }
    let number = |range: std::ops::Range<usize>| value[range].parse::<u32>().unwrap_or(0);
    (1..=12).contains(&number(5..7)) && (1..=31).contains(&number(8..10))
}

/// Checks the connector's own credential declarations.
pub(super) fn validate_credentials(connector: &Connector, problems: &mut Vec<String>) {
    let mut seen: Vec<&str> = Vec::new();

    for method in &connector.auth {
        let name = method.name.as_str();
        if name.trim().is_empty() {
            problems.push("a credential has an empty `name`".to_owned());
            continue;
        }
        if seen.contains(&name) {
            problems.push(format!(
                "credential {name:?} is declared more than once; a requirement naming it could not \
                 say which declaration it meant"
            ));
        }
        seen.push(name);

        // A Basic credential's user half is not optional: without it the host has nothing to put
        // before the colon, and `base64(":secret")` authenticates as nobody.
        if method.scheme == AuthScheme::Basic && method.user_env.is_empty() {
            problems.push(format!(
                "credential {name:?} uses the `basic` scheme but declares no `user_env`. Basic \
                 sends `base64(<user>:<secret>)`, and the user half comes from `user_env`; for \
                 zendesk that is `user_env = [\"ZENDESK_USER\"]` with `user_suffix = \"/token\"`"
            ));
        }
        if method.scheme != AuthScheme::Basic && !method.user_env.is_empty() {
            problems.push(format!(
                "credential {name:?} declares `user_env`, which only the `basic` scheme uses"
            ));
        }
        if method.scheme != AuthScheme::Basic && method.user_suffix.is_some() {
            problems.push(format!(
                "credential {name:?} declares `user_suffix`, which only the `basic` scheme uses"
            ));
        }

        if let AuthScheme::Header { prefix, .. } = &method.scheme {
            validate_auth_prefix(connector, method, prefix, problems);
        }

        // Connect Session entry is deliberately the only non-ambient static source.
        if method.env.is_empty() && method.oauth2.is_none() && method.entry.is_none() {
            problems.push(format!(
                "credential {name:?} names no `env` keys and no `entry`, so nothing can resolve it \
                 to a value"
            ));
        }
        if method.entry.is_some() && !method.env.is_empty() {
            problems.push(format!(
                "credential {name:?} declares Connect Session `entry` and `env` keys. Those are \
                 two acquisition paths for one credential; a Connect Session credential is never \
                 read from ambient process state"
            ));
        }
        if method.entry.is_some() && method.oauth2.is_some() {
            problems.push(format!(
                "credential {name:?} declares Connect Session `entry` and OAuth2. OAuth callbacks \
                 already complete through their own Connect Session flow; operator/API-key entry \
                 is a different acquisition"
            ));
        }

        validate_one_credential_acquisition(connector, method, problems);
        validate_one_credential_token_endpoint(connector, method, problems);
        validate_one_credential_scope_response_pointer(method, problems);
        validate_one_credential_hazard(method, problems);
        validate_one_credential_workarounds(method, problems);
        for key in method.env.iter().chain(&method.user_env) {
            if key.trim().is_empty() {
                problems.push(format!("credential {name:?} lists an empty env-var key"));
            }
        }
    }

    validate_requirements(
        connector,
        &connector.default_auth,
        "`default_auth`",
        problems,
    );
}

/// Header names the `$auth` seam owns. A constant header may not spell one whatever its value is.
///
/// `authorization` and `proxy-authorization` are where a credential goes; `cookie` is a session, which
/// is the same thing arriving by another route. Any of the three declared as a literal would be a
/// credential written into a committed artifact — see [`validate_const_headers`].
pub(super) const AUTH_OWNED_HEADERS: &[&str] = &["authorization", "proxy-authorization", "cookie"];

/// Value prefixes that spell a credential rather than a constant, whatever header carries them.
const CREDENTIAL_VALUE_PREFIXES: &[&str] = &["bearer ", "basic ", "token ", "apikey ", "digest "];

/// Spellings that say "resolve this from somewhere else". None of them resolves: a constant header is
/// a literal, emitted verbatim, so a value in one of these shapes reaches the vendor as its own text.
const RESOLUTION_MARKERS: &[&str] = &["${", "{{", "$secret", "$auth", "env:", "secret:"];

/// Checks every constant request header — the vendor-fixed `Accept`, `Notion-Version`, `User-Agent`
/// (C-55).
///
/// **The rule that earns its keep is the credential one.** Every other field in this file that could
/// hold a secret is a *reference* — a credential name, an env-var key — resolved by the host at
/// request time and never written down. This one is a literal that reaches generated Flux, the
/// capability manifest and the public catalogue verbatim, so an author who reached for it to send
/// `Authorization: Bearer sk-…` would be committing the token to the repository, and the pipeline
/// would carry it all the way to a published artifact without a word. That is precisely the failure
/// `AGENTS.md` forbids ("no credential value enters provider TOML, generated Flux, a manifest, the
/// public catalogue, or the lockfile"), and the refusals below are what keep the field from becoming
/// a second, ungated path to the `$auth` seam C-10 owns.
///
/// The provider-level table is checked once and the operations' own entries after it, so a header
/// declared once for the whole provider is reported once.
pub(super) fn validate_const_headers(
    connector: &Connector,
    provider_headers: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) {
    check_const_header_table(connector, provider_headers, "`[const_headers]`", problems);

    for operation in &connector.operations {
        // Entries the provider contributed are already reported above, spelling and value alike.
        let own: BTreeMap<String, String> = operation
            .params
            .const_headers
            .iter()
            .filter(|(name, value)| provider_headers.get(*name) != Some(*value))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        check_const_header_table(
            connector,
            &own,
            &format!("operation {:?}: `const_headers`", operation.id),
            problems,
        );
    }
}

/// One table of constant headers, provider-level or operation-level.
fn check_const_header_table(
    connector: &Connector,
    headers: &BTreeMap<String, String>,
    context: &str,
    problems: &mut Vec<String>,
) {
    let mut seen: Vec<String> = Vec::new();

    for (name, value) in headers {
        // A map is keyed by exact spelling and HTTP field names are case-insensitive, so two
        // spellings of one header would reach the request record as two entries and be sent twice.
        let folded = name.to_ascii_lowercase();
        if seen.contains(&folded) {
            problems.push(format!(
                "{context}: header {name:?} is declared twice under two spellings. HTTP field names \
                 are case-insensitive (RFC 9110 §5.1), so both would travel as one header sent twice"
            ));
        }
        seen.push(folded.clone());

        if !is_http_field_name(name) {
            problems.push(format!(
                "{context}: header name {name:?} is not an HTTP field name — only ASCII token \
                 characters are allowed (RFC 9110 §5.1), and a request carrying it could never be \
                 built"
            ));
        }
        // Emitted verbatim into a header record, so a CR or LF would append a header of the
        // author's choosing to every request — and a non-ASCII byte is not a field value at all.
        if let Some(bad) = value
            .chars()
            .find(|c| !c.is_ascii() || (c.is_ascii_control() && *c != '\t'))
        {
            problems.push(format!(
                "{context}: header {name:?} has a value carrying {bad:?}, which is not an HTTP \
                 field value (RFC 9110 §5.5). A newline in particular would append a header of its \
                 own to every request"
            ));
        }
        if value.trim().is_empty() {
            problems.push(format!(
                "{context}: header {name:?} has an empty value. A header that says nothing is a \
                 header the vendor did not ask for — remove it, or state what it sends"
            ));
        }

        if folded == "content-type" {
            problems.push(format!(
                "{context}: `content-type` is the emitter's, not a provider's. It is derived from \
                 the request body — `application/json` for every body this pipeline builds — so \
                 declaring it here would describe an encoding the emitted module does not produce"
            ));
        }
        if AUTH_OWNED_HEADERS.contains(&folded.as_str()) {
            problems.push(format!(
                "{context}: header {name:?} carries a credential, and a constant header is a \
                 literal in a committed artifact. Credentials are declared in `[[auth]]` and \
                 injected by the host at the `$auth` seam, which is what keeps the value out of the \
                 generated module, the manifest and the public catalogue"
            ));
        }
        for method in &connector.auth {
            if let AuthScheme::Header { name: owned, .. } = &method.scheme {
                if owned.eq_ignore_ascii_case(name) {
                    problems.push(format!(
                        "{context}: header {name:?} is where credential {:?} is injected, so a \
                         constant would either be overwritten by the host or overwrite the \
                         credential. Declare the header on one side only",
                        method.name
                    ));
                }
            }
        }

        credential_shaped_value(connector, name, value, context, problems);
    }
}

/// Whether a constant header's *value* is a credential, or something the author expects to resolve
/// into one.
///
/// Nothing here resolves. A constant header is emitted as a literal, so `${GITHUB_TOKEN}` reaches
/// GitHub as those fourteen characters — the benign reading is a broken request, and the dangerous
/// one is an author who pastes the real value in once the placeholder does not work. Both are
/// refused at the declaration.
fn credential_shaped_value(
    connector: &Connector,
    name: &str,
    value: &str,
    context: &str,
    problems: &mut Vec<String>,
) {
    let folded = value.to_ascii_lowercase();

    if let Some(marker) = RESOLUTION_MARKERS
        .iter()
        .find(|marker| folded.contains(*marker))
    {
        problems.push(format!(
            "{context}: header {name:?} has a value spelling {marker:?}, but a constant header is a \
             literal and nothing interpolates it — the vendor would receive those characters. A \
             value that has to be resolved is a credential or configuration: declare it in \
             `[[auth]]` or `[[config]]`"
        ));
    }
    if let Some(prefix) = CREDENTIAL_VALUE_PREFIXES
        .iter()
        .find(|prefix| folded.starts_with(*prefix))
    {
        problems.push(format!(
            "{context}: header {name:?} has a value beginning {prefix:?}, which is a credential. It \
             would be committed to this repository verbatim and published in the catalogue. \
             Credentials are declared in `[[auth]]` and injected by the host"
        ));
    }
    for method in &connector.auth {
        if value.contains(&method.name) {
            problems.push(format!(
                "{context}: header {name:?} has a value naming credential {:?}. A constant header is \
                 a literal, not a reference — nothing resolves the name, and the value that would \
                 make it work is one this file must never hold",
                method.name
            ));
        }
        for key in method.env.iter().chain(&method.user_env) {
            if !key.trim().is_empty() && value.contains(key.as_str()) {
                problems.push(format!(
                    "{context}: header {name:?} has a value naming the environment variable \
                     {key:?}, which resolves credential {:?}. A constant header is emitted as a \
                     literal, so the name would travel as text and the value it stands for must \
                     never be written here at all",
                    method.name
                ));
            }
        }
    }
}

/// Whether `name` is a valid HTTP field name (RFC 9110 §5.1 `token`).
///
/// The emitter checks the same grammar on the way out (`connector-flux`'s `is_http_token`), for the
/// same reason the member-name rules are split: this guards what an author may *declare*, and the
/// emitter guards what may reach `http.request`.
pub(super) fn is_http_field_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}
