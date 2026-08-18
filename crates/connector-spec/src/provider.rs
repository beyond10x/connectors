//! The provider-TOML front-end: `providers/<name>.toml` in, [`Connector`] out.
//!
//! The file plays **two roles**, and the loader has to serve both from one schema:
//!
//! 1. **Hand-authored** — the whole connector is written out inline, with no vendor spec anywhere.
//!    Ollama, Freshdesk and (for now) Zendesk are in this position: there is no usable OpenAPI
//!    document to ingest. This is the role that matters most today, because it is the shortest route
//!    to an executable `.flux` module.
//! 2. **Spec pointer** — the file names one or more vendored specs under `specs/` and carries a
//!    *patch set* that selects and corrects operations from them. [`load_with_spec`] is that path:
//!    ingest (C-4) turns each document into every operation the vendor declares, and the patch set
//!    says which of them this connector publishes and what it corrects about each. **Selection is
//!    opt-in**, so a pointer with no patch is a connector with no operations. What one *statement*
//!    selects is wide — [`OperationSelector`] matches a set by service, path prefix and method,
//!    [`Naming`] derives op ids through one declared rule with pinned exceptions, and both `risk`
//!    and `expose` may be stated for a whole matched set (C-411, C-412, C-414) — and none of that
//!    changes opt-in, because a selector is still something an author wrote.
//!
//!    **One document is one [`Service`]** (C-410). `[spec]` names one and `[[spec]]` names several;
//!    they are one key in two TOML spellings and the table is the one-element case. A vendor that
//!    splits its API across documents — babelforce publishes five, over two API versions and two
//!    security models — is therefore one connector with a service per document, rather than five
//!    connectors. Each document is resolved, hash-checked and ingested on its own, and nothing is
//!    merged: an `operationId` is unique inside a document and nowhere else.
//!
//! Both roles produce the same [`LoadedProvider`], which is what "two front-ends, one IR" means in
//! practice.
//!
//! # Errors are the interface
//!
//! Nobody debugs a provider TOML with a debugger; they read the error and edit the file. So the
//! error text is a deliverable, pinned by golden files in `tests/provider_toml_errors.rs`, and the
//! loader is arranged to make it good:
//!
//! - **Shape errors are serde's**, because serde's are better. Deserializing straight into the IR
//!   types means an unknown key reports the offending key *and lists every key that would have been
//!   valid*, with a line, a column and a source snippet from `toml`. A hand-rolled checker would
//!   have to reproduce all of that and would drift from the types.
//! - **Semantic errors are ours**, and are reported **all at once** rather than one per run. Fixing
//!   a provider file one error at a time is the authoring experience this repo is written against.
//!
//! # No network, no filesystem
//!
//! [`load`] takes bytes and a display name. Reading `providers/*.toml` off disk and fetching specs
//! is `connector-cli`'s job — see the crate docs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::config::{
    parse_binding, template_variables, Approval, Binding, ConfigField, Format, Position,
};
use crate::graph::{Graph, GraphNode, NodeKind, PortRef};
use crate::inbound::{
    parse_tolerance, signed_placeholders, validate_path, validate_symbol, ChannelBinding,
    EventDecl, FieldSource, HmacSpec, ManualSetup, Reply, Selector, SessionBinding,
    SocketConnectSpec, Subscription, Transport, VerificationScheme, PAYLOAD_PLACEHOLDERS,
    SIGNED_PLACEHOLDERS,
};
use crate::lock::sha256_hex;
use crate::{
    response_location_exists, AuthHazard, AuthMethod, AuthRequirement, AuthScheme, Connector,
    Discovery, DiscoveryDriver, DiscoveryMapping, ErrorEnvelope, HostEffect, HttpMethod,
    Idempotency, ImplementationForm, InteractionShape, JsonSchema, OAuthGrant, Operation,
    OperationDirection, OperationRequest, OperationSpecSource, Pagination, Param, ParamSet,
    PlacementRequirement, ProtocolDriver, Provenance, RateLimit, RequiredCapability, Risk, Role,
    RouteAdapter, SemanticEffect, Service, Tag, DEFAULT_SERVICE, MIN_REPEATABILITY_CONDITION,
};

mod identity_overlay;
use identity_overlay::{check_descriptions, check_directions, description_for, direction_for};

/// The documented JSON Schema for `providers/<name>.toml`.
///
/// TOML is a JSON-shaped data model, so one schema describes both the TOML an author writes and the
/// JSON the IR encodes to. It is hand-written rather than generated (generating it would mean a
/// `schemars` dependency this crate does not take) and is therefore kept honest by a test:
/// `tests/provider_schema.rs` asks serde which keys each type actually accepts and fails if the
/// schema documents a different set.
pub const PROVIDER_TOML_JSON_SCHEMA: &str = include_str!("../schema/provider-toml.schema.json");

mod auth_validation;
mod channel_validation;
mod config_validation;
mod declaration;
mod graph_validation;
mod loading;
mod operation_validation;
mod patch_validation;
mod publishing;
mod schema_sync;
mod service_validation;
mod validation;

use auth_validation::*;
use channel_validation::*;
use config_validation::*;
use declaration::*;
use graph_validation::*;
use loading::*;
use operation_validation::*;
use patch_validation::*;
use publishing::*;
use service_validation::*;
use validation::*;

pub use declaration::{
    EventPatch, IngestedDocument, IngestedEventDocument, LoadedProvider, Naming, NamingRule,
    OperationPatch, OperationSelector, ParamOmission, ParamPatch, ParamPosition, Patch, SpecKind,
    SpecSource,
};
pub use loading::{load, load_with_spec, SpecDocument};
pub use schema_sync::accepted_keys;
