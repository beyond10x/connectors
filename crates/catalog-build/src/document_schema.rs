/// The versioned JSON Schema every document validates against — committed beside the documents and
/// enforced by [`render`].
pub fn schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let mut schema = schema_v2().clone();
        schema["$id"] = json!(SCHEMA_ID);
        schema["properties"]["$schema"]["const"] = json!(SCHEMA_ID);
        schema["properties"]["schema_version"]["const"] = json!(SCHEMA_VERSION);
        schema["$defs"]["operation"]["properties"]["request_semantics"] = json!({"enum":["legacy_v1","openapi_3_0_json_v1"]});
        schema["$defs"]["operation"]["required"].as_array_mut().expect("operation required fields").push(json!("request_semantics"));
        schema["$defs"]["contract"]["description"] = json!("The stored caller contract under the explicit request-semantics profile; source profiles preserve the vendor constraints through deterministic dialect translation.");
        schema["$defs"]["contract"]["properties"]["output_schema"] = json!({"$ref":"#/$defs/json_schema"});
        schema
    })
}

/// Historical schema 2, frozen independently of the current profile vocabulary and identity.
fn schema_v2() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": SCHEMA_V2_ID,
            "title": "connectors canonical catalog document",
            "description": "One canonical, deterministic catalog document per provider (Decision 0022, C-536): the complete published connector surface, including an explicit request template per operation. The template vocabulary is closed and total — literal values, `{var}` interpolation over caller parameters and endpoint slots, and `$param` splices — and the oauth2 object deliberately has no field for a registration value: the client identifier and secret are per-deployment, published only as operator-level configuration requirements through the `binds` grammar.",
            "type": "object",
            "properties": {
                "$schema": { "const": SCHEMA_V2_ID },
                "schema_version": { "const": 2 },
                "generator": { "type": "string", "minLength": 1 },
                "connector": { "type": "string", "minLength": 1 },
                "vendor": { "type": "string" },
                "description": { "type": "string" },
                "authority": { "type": "string", "minLength": 1 },
                "verify": { "type": "string", "minLength": 1 },
                "custody_only": { "const": true },
                "services": { "type": "array", "items": { "$ref": "#/$defs/service" } },
                "auth": { "type": "array", "items": { "$ref": "#/$defs/auth" } },
                "default_auth": { "$ref": "#/$defs/requirements" },
                "default_auth_requirements": { "$ref": "#/$defs/auth_requirements" },
                "config": { "type": "array", "items": { "$ref": "#/$defs/config_field" } },
                "operations": { "type": "array", "items": { "$ref": "#/$defs/operation" } },
                "events": { "type": "array", "items": { "$ref": "#/$defs/event" } },
                "channels": { "type": "array", "items": { "$ref": "#/$defs/channel" } },
                "discoveries": { "type": "array", "items": { "$ref": "#/$defs/discovery" } }
            },
            "required": ["$schema", "schema_version", "generator", "connector", "services", "operations"],
            "additionalProperties": false,
            "allOf": [
                {
                    "description": "A custody-only provider publishes no service and no operation; every other provider publishes at least one service. The two branches are what makes `custody_only` a property of the document rather than a claim about it.",
                    "if": { "required": ["custody_only"] },
                    "then": { "properties": { "services": { "maxItems": 0 }, "operations": { "maxItems": 0 }, "events": { "maxItems": 0 }, "channels": { "maxItems": 0 }, "discoveries": { "maxItems": 0 }, "verify": false } },
                    "else": { "properties": { "services": { "minItems": 1 } } }
                }
            ],
            "$defs": {
                "json_schema": {
                    "description": "A JSON Schema value, carried verbatim from the vendor's declaration.",
                    "type": ["object", "boolean"]
                },
                "requirements": {
                    "description": "Auth alternatives: OR of AND-groups of declared credential names.",
                    "type": "array",
                    "items": { "type": "array", "items": { "type": "string", "minLength": 1 } }
                },
                "auth_requirements": {
                    "description": "Non-lossy auth alternatives with credential-local OR-of-AND granted-scope requirements.",
                    "type": "array",
                    "items": { "$ref": "#/$defs/auth_requirement" }
                },
                "auth_requirement": {
                    "type": "object",
                    "properties": {
                        "credentials": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                        "scopes": {
                            "type": "object",
                            "additionalProperties": {
                                "type": "array",
                                "minItems": 1,
                                "items": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
                            }
                        }
                    },
                    "required": ["credentials"],
                    "additionalProperties": false
                },
                "service": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "gid": { "type": "string" },
                        "description": { "type": "string" },
                        "base_url": { "type": "string", "minLength": 1 },
                        "api_version": { "type": "string" },
                        "legacy": { "type": "boolean" },
                        "roles": { "type": "array", "items": { "enum": ["llm_catalogue"] } },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "audiences": { "type": "array", "uniqueItems": true, "items": { "enum": ["developer", "sre", "security-engineer", "data-analyst", "product-manager", "project-manager", "designer", "sales-rep", "support-agent", "marketer", "finance", "ecommerce-manager", "content-manager"] } }
                    },
                    "required": ["name", "base_url"],
                    "additionalProperties": false
                },
                "auth": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "scheme": { "$ref": "#/$defs/scheme" },
                        "env": { "type": "array", "items": { "type": "string" } },
                        "entry": { "enum": ["connect_session"] },
                        "user_env": { "type": "array", "items": { "type": "string" } },
                        "user_suffix": { "type": "string" },
                        "description": { "type": "string" },
                        "subject": { "enum": ["unstated", "app", "user"] },
                        "hazard": { "type": "string" },
                        "oauth2": { "$ref": "#/$defs/oauth2" },
                        "token_endpoint_workarounds": { "type": "array", "items": { "$ref": "#/$defs/token_endpoint_workaround" } }
                    },
                    "required": ["name", "scheme", "subject"],
                    "additionalProperties": false
                },
                "scheme": {
                    "type": "object",
                    "properties": {
                        "kind": { "enum": ["bearer", "basic", "header", "query", "signing"] },
                        "name": { "type": "string" },
                        "prefix": { "type": "string" }
                    },
                    "required": ["kind"],
                    "additionalProperties": false
                },
                "oauth2": {
                    "description": "The complete OAuth2 declaration. Deliberately, no field exists for a registration value: the client identifier and secret are per-deployment, published only as operator-level configuration requirements through the `binds` grammar (C-536).",
                    "type": "object",
                    "properties": {
                        "endpoint": { "type": "string" },
                        "token_endpoint": { "type": "string" },
                        "authorize_path": { "type": "string" },
                        "token_path": { "type": "string" },
                        "scopes": { "type": "array", "items": { "type": "string" } },
                        "scope_separator": { "enum": ["space", "comma"] },
                        "scope_response_pointer": { "type": "string", "pattern": "^/" },
                        "grants": { "type": "array", "items": { "enum": ["authorization_code", "password", "refresh_token", "client_credentials"] } },
                        "public_client": { "type": "boolean" },
                        "redirect": {
                            "type": "object",
                            "properties": {
                                "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                                "path": { "type": "string" }
                            },
                            "required": ["port", "path"],
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                },
                "token_endpoint_workaround": {
                    "type": "object",
                    "properties": {
                        "grant": { "type": "string" },
                        "behaviour": { "type": "string" },
                        "attribution": { "type": "string" },
                        "measured": { "type": "string" }
                    },
                    "required": ["grant", "behaviour", "attribution", "measured"],
                    "additionalProperties": false
                },
                "config_field": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "service": { "type": "string", "minLength": 1 },
                        "label": { "type": "string", "minLength": 1 },
                        "help": { "type": "string" },
                        "example": { "type": "string" },
                        "format": { "enum": ["text", "subdomain", "hostname", "url", "origin", "email", "token"] },
                        "choices": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "value": { "type": "string" },
                                    "label": { "type": "string" }
                                },
                                "required": ["value", "label"],
                                "additionalProperties": false
                            }
                        },
                        "required": { "type": "boolean" },
                        "default": { "type": "string" },
                        "approval": { "enum": ["none", "operator"] },
                        "secret": { "type": "boolean" },
                        "docs_url": { "type": "string" },
                        "binds": { "type": "string", "minLength": 1 },
                        "also_binds": { "type": "array", "items": { "type": "string" } },
                        "also_services": { "type": "array", "items": { "type": "string" } },
                        "level": { "enum": ["operator", "connection"] }
                    },
                    "required": ["name", "service", "label", "format", "required", "approval", "secret", "binds", "level"],
                    "additionalProperties": false
                },
                "operation": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1 },
                        "service": { "type": "string", "minLength": 1 },
                        "direction": { "enum": ["read", "write"] },
                        "description": { "type": "string" },
                        "risk": { "enum": ["low", "medium", "high", "destructive"] },
                        "idempotency": { "enum": ["idempotent", "non_idempotent", "conditional"] },
                        "effects": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "enum": ["read", "write", "network", "process", "browser", "filesystem", "local_system"] } },
                        "repeatability_condition": { "type": "string" },
                        "semantic_effects": { "type": "array", "items": { "type": "string" } },
                        "interaction_shape": { "enum": ["unary", "stream", "subscription", "leased_session", "session_establishment"] },
                        "protocol_driver": { "enum": ["http_v1", "sip_v1", "audio_v1", "cdp_v1", "sql_v1"] },
                        "placement_requirement": { "enum": ["connectors_deployment", "substrate_workload", "federated_satellite"] },
                        "implementation_form": { "enum": ["built_in"] },
                        "required_capabilities": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "enum": ["public_network", "private_network", "unix_socket", "file_secret", "process", "container", "device"] } },
                        "contract": { "$ref": "#/$defs/contract" },
                        "expose": { "type": "boolean" },
                        "auth": { "$ref": "#/$defs/requirements" },
                        "auth_requirements": { "$ref": "#/$defs/auth_requirements" },
                        "credential_requirement": {
                            "description": "What the effective `auth` list cannot say when it is empty (S-001): whether the connector declared that nothing is required (`no-credential-required`) or declared nothing anywhere (`no-credential`, the fail-closed reading). C-206's published tokens, carried as data so no consumer resolves the connector default to reconstruct the distinction.",
                            "enum": ["declared", "no-credential-required", "no-credential"]
                        },
                        "produces_credential": { "$ref": "#/$defs/produces_credential" },
                        "request": { "type": "object" },
                        "params": { "type": "array", "items": { "$ref": "#/$defs/param" } },
                        "response_schema": { "$ref": "#/$defs/json_schema" },
                        "endpoint": {
                            "type": "object",
                            "additionalProperties": {
                                "type": "array",
                                "minItems": 1,
                                "items": { "enum": ["origin", "host", "path", "query", "header"] }
                            }
                        },
                        "pagination": { "$ref": "#/$defs/pagination" },
                        "rate_limit": { "$ref": "#/$defs/rate_limit" },
                        "error_envelope": { "$ref": "#/$defs/error_envelope" }
                    },
                    "required": ["id", "service", "direction", "risk", "idempotency", "effects", "semantic_effects", "interaction_shape", "protocol_driver", "placement_requirement", "implementation_form", "required_capabilities", "contract", "expose", "auth", "credential_requirement", "request"],
                    "allOf": [
                        {
                            "if": { "properties": { "protocol_driver": { "const": "http_v1" } } },
                            "then": { "properties": { "request": { "$ref": "#/$defs/http_request" } } }
                        },
                        {
                            "if": { "properties": { "protocol_driver": { "const": "sip_v1" } } },
                            "then": { "properties": { "request": { "$ref": "#/$defs/sip_request" } } }
                        },
                        {
                            "if": { "properties": { "protocol_driver": { "const": "audio_v1" } } },
                            "then": { "properties": { "request": { "$ref": "#/$defs/audio_request" } } }
                        },
                        {
                            "if": { "properties": { "protocol_driver": { "const": "cdp_v1" } } },
                            "then": { "properties": { "request": { "$ref": "#/$defs/cdp_request" } } }
                        },
                        {
                            "if": { "properties": { "protocol_driver": { "const": "sql_v1" } } },
                            "then": { "properties": { "request": { "$ref": "#/$defs/sql_request" } } }
                        }
                    ],
                    "additionalProperties": false
                },
                "contract": {
                    "description": "The model-facing contract projection (S-001; predecessor C-552): the error-envelope-extended description and the lowered, caller-typed input schema, computed at build time and stored so a consumer builds the caller's contract from document data alone.",
                    "type": "object",
                    "properties": {
                        "description": { "type": "string" },
                        "input_schema": { "$ref": "#/$defs/json_schema" }
                    },
                    "required": ["description", "input_schema"],
                    "additionalProperties": false
                },
                "produces_credential": {
                    "description": "The minting join (S-001): this operation's call mints a declared credential — which credential the value is stored as, and where in the response body the secret arrives (one JSON Pointer, no wildcard).",
                    "type": "object",
                    "properties": {
                        "credential": { "type": "string", "minLength": 1 },
                        "secret": { "type": "string", "minLength": 1 }
                    },
                    "required": ["credential", "secret"],
                    "additionalProperties": false
                },
                "http_request": {
                    "type": "object",
                    "properties": {
                        "method": { "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] },
                        "url": { "type": "string", "pattern": "^\\{base\\}/" },
                        "headers": { "type": "object", "additionalProperties": { "$ref": "#/$defs/value_template" } },
                        "query": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "minLength": 1 },
                                    "value": { "$ref": "#/$defs/value_template" }
                                },
                                "required": ["name", "value"],
                                "additionalProperties": false
                            }
                        },
                        "body": { "$ref": "#/$defs/body" }
                    },
                    "required": ["method", "url"],
                    "additionalProperties": false
                },
                "sip_request": {
                    "description": "SIP driver request marker. HTTP-shaped request facts are structurally impossible.",
                    "type": "object",
                    "maxProperties": 0
                },
                "audio_request": {
                    "description": "Local-audio driver request marker. HTTP-shaped request facts are structurally impossible, and no device, synthesizer, voice or sink is nameable here.",
                    "type": "object",
                    "maxProperties": 0
                },
                "cdp_request": {
                    "description": "Browser driver request marker. HTTP-shaped request facts are structurally impossible: the browser's own traffic belongs to the driver, and no executable, profile directory or artifact directory is nameable here.",
                    "type": "object",
                    "maxProperties": 0
                },
                "sql_request": {
                    "description": "SQL driver request marker. HTTP-shaped request facts are structurally impossible, and no host, port, database, user or credential value is nameable here.",
                    "type": "object",
                    "maxProperties": 0
                },
                "param_splice": {
                    "description": "The whole value of the named caller parameter.",
                    "type": "object",
                    "properties": { "$param": { "type": "string", "minLength": 1 } },
                    "required": ["$param"],
                    "additionalProperties": false
                },
                "value_template": {
                    "description": "A literal (which may interpolate `{endpoint-slot}` placeholders) or a `$param` splice — the closed vocabulary, in value position.",
                    "oneOf": [
                        { "type": "string" },
                        { "$ref": "#/$defs/param_splice" }
                    ]
                },
                "body_template": {
                    "description": "A JSON body: literals, nested objects and arrays, `$param` splices at caller leaves. Nothing else has a spelling.",
                    "oneOf": [
                        { "$ref": "#/$defs/param_splice" },
                        {
                            "type": "object",
                            "not": { "required": ["$param"] },
                            "additionalProperties": { "$ref": "#/$defs/body_template" }
                        },
                        { "type": "array", "items": { "$ref": "#/$defs/body_template" } },
                        { "type": ["string", "number", "boolean", "null"] }
                    ]
                },
                "body": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "encoding": { "const": "json" },
                                "template": { "$ref": "#/$defs/body_template" }
                            },
                            "required": ["encoding", "template"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "encoding": { "const": "form" },
                                "fields": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string", "minLength": 1 },
                                            "value": { "$ref": "#/$defs/value_template" },
                                            "required": { "type": "boolean" }
                                        },
                                        "required": ["name", "value", "required"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["encoding", "fields"],
                            "additionalProperties": false
                        }
                    ]
                },
                "param": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "position": { "enum": ["path", "query", "header", "body"] },
                        "symbol": {
                            "description": "The caller-facing symbol the contract declares this parameter by (S-001): the name a caller addresses it under, which is not its document `name`.",
                            "type": "string",
                            "minLength": 1
                        },
                        "wire": { "type": "string" },
                        "description": { "type": "string" },
                        "required": { "type": "boolean" },
                        "schema": { "$ref": "#/$defs/json_schema" }
                    },
                    "required": ["name", "position", "symbol", "required", "schema"],
                    "additionalProperties": false
                },
                "pagination": {
                            "oneOf": [
                                {
                                    "type": "object",
                                    "properties": {
                                        "page": {
                                            "type": "object",
                                            "properties": {
                                                "page_param": { "type": "string" },
                                                "size_param": { "type": "string" },
                                                "page_size": { "type": "integer" },
                                                "max_pages": { "type": "integer" }
                                            },
                                            "required": ["page_param", "max_pages"],
                                            "additionalProperties": false
                                        }
                                    },
                                    "required": ["page"],
                                    "additionalProperties": false
                                },
                                {
                                    "type": "object",
                                    "properties": {
                                        "cursor": {
                                            "type": "object",
                                            "properties": {
                                                "cursor_param": { "type": "string" },
                                                "next_cursor_pointer": { "type": "string" },
                                                "max_pages": { "type": "integer" }
                                            },
                                            "required": ["cursor_param", "next_cursor_pointer", "max_pages"],
                                            "additionalProperties": false
                                        }
                                    },
                                    "required": ["cursor"],
                                    "additionalProperties": false
                                }
                            ]
                },
                "rate_limit": {
                            "type": "object",
                            "properties": {
                                "requests": { "type": "integer" },
                                "per_seconds": { "type": "integer" },
                                "bucket": { "type": "string" }
                            },
                            "required": ["requests", "per_seconds"],
                            "additionalProperties": false
                },
                "error_envelope": {
                            "type": "object",
                            "properties": {
                                "message_pointer": { "type": "string" },
                                "code_pointer": { "type": "string" }
                            },
                            "required": ["message_pointer"],
                            "additionalProperties": false
                },
                "discovery": {
                    "description": "A bounded observation source. It creates no Connection or authority; target Provider availability and route-adapter support are checked when a candidate is materialized.",
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1 },
                        "service": { "type": "string", "minLength": 1 },
                        "operation": { "type": "string", "minLength": 1 },
                        "driver": { "enum": ["grafana_datasource_v1"] },
                        "mappings": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "observed_type": { "type": "string", "pattern": "^[a-z0-9._-]+$" },
                                    "target_provider": { "type": "string", "pattern": "^[a-z0-9._-]+$" },
                                    "route_adapter": { "enum": ["grafana_datasource_proxy_v1"] }
                                },
                                "required": ["observed_type", "target_provider", "route_adapter"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["id", "service", "operation", "driver", "mappings"],
                    "additionalProperties": false
                },
                "event": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "service": { "type": "string", "minLength": 1 },
                        "oip": { "type": "string" },
                        "wire_value": { "type": "string" },
                        "auth": { "$ref": "#/$defs/requirements" },
                        "auth_requirements": { "$ref": "#/$defs/auth_requirements" },
                        "description": { "type": "string" },
                        "default": { "type": "boolean" },
                        "group": { "type": "string" },
                        "when": { "type": "object", "additionalProperties": { "$ref": "#/$defs/json_schema" } },
                        "schema": { "$ref": "#/$defs/json_schema" }
                    },
                    "required": ["name", "service", "default"],
                    "additionalProperties": false
                },
                "channel": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "service": { "type": "string", "minLength": 1 },
                        "oip": { "type": "string" },
                        "description": { "type": "string" },
                        "transport": { "enum": ["webhook", "socket", "poll", "session"] },
                        "session": { "$ref": "#/$defs/session_binding" },
                        "connect": { "type": "object" },
                        "auth": { "$ref": "#/$defs/requirements" },
                        "auth_requirements": { "$ref": "#/$defs/auth_requirements" },
                        "events": { "type": "array", "items": { "type": "string" } },
                        "cursor": { "type": "string" },
                        "interval": { "type": "string" },
                        "verification": {
                            "type": "object",
                            "properties": {
                                "kind": { "type": "string" },
                                "verified": { "type": "boolean" },
                                "hmac": { "type": "object" }
                            },
                            "required": ["kind", "verified"],
                            "additionalProperties": false
                        },
                        "discriminator": { "$ref": "#/$defs/selector" },
                        "delivery_id": { "$ref": "#/$defs/selector" },
                        "payload": { "type": "object", "additionalProperties": { "type": "string" } },
                        "payload_root": { "type": "boolean" },
                        "reply": {
                            "type": "object",
                            "properties": {
                                "operation": { "type": "string" },
                                "oip": { "type": "string" },
                                "result": { "type": "string" },
                                "bind": { "type": "object", "additionalProperties": { "type": "string" } }
                            },
                            "required": ["operation"],
                            "additionalProperties": false
                        },
                        "subscription": { "type": "object" },
                        "setup": { "type": "object" }
                    },
                    "required": ["name", "service", "transport", "verification"],
                    "allOf": [
                        {
                            "if": { "properties": { "transport": { "const": "session" } } },
                            "then": { "required": ["session"] },
                            "else": { "not": { "required": ["session"] } }
                        }
                    ],
                    "additionalProperties": false
                },
                "session_binding": {
                    "type": "object",
                    "properties": {
                        "interaction_shape": { "const": "session_establishment" },
                        "protocol_driver": { "const": "sip_v1" },
                        "placement_requirement": { "enum": ["connectors_deployment", "substrate_workload", "federated_satellite"] },
                        "implementation_form": { "const": "built_in" },
                        "required_capabilities": { "type": "array", "minItems": 1, "uniqueItems": true, "items": { "enum": ["public_network", "private_network", "unix_socket", "file_secret", "process", "container", "device"] } }
                    },
                    "required": ["interaction_shape", "protocol_driver", "placement_requirement", "implementation_form", "required_capabilities"],
                    "additionalProperties": false
                },
                "selector": {
                    "type": "object",
                    "properties": {
                        "source": { "enum": ["header", "body"] },
                        "name": { "type": "string" }
                    },
                    "required": ["source", "name"],
                    "additionalProperties": false
                }
            }
        })
    })
}
