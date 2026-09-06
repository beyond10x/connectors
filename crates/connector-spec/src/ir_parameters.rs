/// The versioned interpretation of source schemas and request templates. This is a transport
/// fact, not an authorization grant. Catalog schema 3 requires consumers to recognize it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestSemantics {
    /// The inherited contract projection and request-template interpretation.
    #[default]
    #[serde(rename = "legacy_v1")]
    LegacyV1,
    /// The supported OpenAPI 3.0 JSON-body and scalar-parameter class, preserving source constraints.
    #[serde(rename = "openapi_3_0_json_v1")]
    OpenApi30JsonV1,
}

impl RequestSemantics {
    /// Whether compatibility defaults may omit this fact from a provider declaration.
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::LegacyV1)
    }

    /// The closed spelling carried by the canonical document.
    pub fn word(&self) -> &'static str {
        match self {
            Self::LegacyV1 => "legacy_v1",
            Self::OpenApi30JsonV1 => "openapi_3_0_json_v1",
        }
    }
}

/// An operation's parameters, grouped by where they travel on the request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamSet {
    /// Closed source schema and request-template interpretation; legacy declarations retain theirs.
    #[serde(default, skip_serializing_if = "RequestSemantics::is_legacy")]
    pub request_semantics: RequestSemantics,
    /// Parameters interpolated into the path template (`/v2/calls/{call_id}`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<Param>,
    /// Query-string parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<Param>,
    /// Request headers the caller supplies. Auth headers are **not** here — they are injected by
    /// the host from an [`AuthMethod`], so no credential passes through the parameter surface.
    ///
    /// A header the *vendor* fixes is [`const_headers`](Self::const_headers), not one of these with
    /// a `const` in its schema: this list means caller-supplied, and reinterpreting one entry of it
    /// by keyword would make a single declaration mean two things. `connector-flux` refuses the
    /// pinned spelling rather than honouring it (C-55).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Param>,
    /// Request headers the **vendor** fixes: `Accept: application/vnd.github+json`,
    /// `Notion-Version: 2022-06-28`, an API version, a `User-Agent`.
    ///
    /// Not parameters, which is why they are a map of literals rather than [`Param`]s and why
    /// [`iter`](Self::iter) does not yield them: nothing is caller-supplied here, nothing reaches
    /// the emitted `op`'s signature, and a model is never asked to guess a value the vendor has
    /// already decided. The emitter binds each value as a literal and puts it in the request's
    /// header record (C-55).
    ///
    /// **Declared at two levels, resolved to one.** A provider states a header once — the file's
    /// top-level `[const_headers]` — and the loader distributes it onto every operation, an
    /// operation's own entry replacing the provider's when they name the same header (HTTP field
    /// names are case-insensitive, so `Notion-Version` and `notion-version` are one header and never
    /// two). So this map is always the *complete* set the operation sends, and no consumer has to
    /// resolve an inheritance to know what travels.
    ///
    /// **It can never carry a credential.** A value here is a literal in a committed artifact, so a
    /// secret placed in one would be a secret in the repository — the one thing generated data must
    /// never hold (AGENTS.md). The loader refuses a value that resolves from an environment variable
    /// or names a declared credential, and refuses a header name the `$auth` seam owns; `Authorization`
    /// is C-10's business and this field must not become a second, ungated path to it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub const_headers: BTreeMap<String, String>,
    /// Fields assembled into the JSON request body, each at the JSON path its
    /// [`Param::wire`] names (or at the root of the body when it names none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<Param>,
    /// The body **is** this schema, rather than being assembled from named fields.
    ///
    /// A free-form object body: `babelforce-call-session-set` and `babelforce-session-update` both
    /// take `{"type": "object"}` with no properties — a map of caller-chosen keys
    /// (`predecessor:docs/designs/provider-operation-inventory.md` §6.5). A field list cannot describe that, and
    /// an operation that declared no body field at all emitted a `PUT` with **no body**, which is
    /// indistinguishable from a legitimately bodiless write.
    ///
    /// Mutually exclusive with [`body`](Self::body): "the body is these fields" and "the body is
    /// this schema" are two answers to one question, and nothing states how to merge them. Codegen
    /// refuses an operation that declares both rather than picking one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<JsonSchema>,
    /// Whether a whole request body must be present, independent of required properties inside it.
    /// Absent preserves the historical required whole-body behavior. Valid only with body_schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_required: Option<bool>,
    /// How the body above is encoded on the wire. See [`BodyEncoding`].
    ///
    /// Defaulted rather than mandatory, unlike [`Risk`] and [`Idempotency`]: silence here is not a
    /// safety decision, it is the answer fifty-three of fifty-three shipped providers already give
    /// (no shipped operation declares a non-default encoding), and making it explicit everywhere
    /// would be fifty-three files restating one fact. What silence must
    /// not do is *change* — so `json` is the default and the emitter's output for an operation that
    /// declares nothing is byte-for-byte what it was before this field existed.
    #[serde(default, skip_serializing_if = "BodyEncoding::is_json")]
    pub body_encoding: BodyEncoding,
}

impl ParamSet {
    /// Whether the operation takes no parameters at all.
    ///
    /// `body_schema` counts: an operation whose only body declaration is a schema would otherwise
    /// encode as an absent `params` and lose the whole body on the way back. So does
    /// [`const_headers`](Self::const_headers), for the same reason — an operation whose only request
    /// declaration is a pinned header would lose it on the way back, and the emitted module would
    /// stop sending the header the vendor requires. And so does a non-default
    /// [`body_encoding`](Self::body_encoding): the emitter refuses that shape rather than emitting
    /// it, so it cannot ship — but a round trip that dropped the declaration would turn a loud
    /// refusal into a silent JSON body, which is the wrong direction to fail in.
    pub fn is_empty(&self) -> bool {
        self.request_semantics.is_legacy()
            && self.body_required.is_none()
            && self.path.is_empty()
            && self.query.is_empty()
            && self.header.is_empty()
            && self.body.is_empty()
            && self.body_schema.is_none()
            && self.const_headers.is_empty()
            && self.body_encoding.is_json()
    }

    /// Every parameter, in request-position order: path, query, header, body.
    ///
    /// [`body_schema`](Self::body_schema) is deliberately absent — it is a schema, not a [`Param`],
    /// and a caller iterating parameters is asking about named ones.
    /// [`const_headers`](Self::const_headers) is absent for the stronger version of that reason:
    /// nothing about it is caller-supplied.
    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.path
            .iter()
            .chain(&self.query)
            .chain(&self.header)
            .chain(&self.body)
    }
}
