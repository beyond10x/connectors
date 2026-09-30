//! The operation inventory: what a document declares, in an order two runs agree
//! on, with what it cannot represent named beside it rather than dropped. A count
//! that hides its own gaps is the failure this module exists to avoid.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The methods an inventory reads, in the order it emits them. A path item may
/// declare them in any order; the inventory does not inherit that order, so two
/// documents with the same operations produce the same inventory.
pub const METHODS: [&str; 7] = ["get", "put", "post", "delete", "options", "head", "patch"];

/// Where a parameter travels. A location outside these four is unsupported and
/// named; it is not guessed into one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    Path,
    Query,
    Header,
    Cookie,
}

impl Location {
    /// The spellings a document uses, read into the model. Shared with the
    /// authored reader: a second table of these four would be a second answer to
    /// what `header` means, and the readers must give one.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "path" => Some(Self::Path),
            "query" => Some(Self::Query),
            "header" => Some(Self::Header),
            "cookie" => Some(Self::Cookie),
            _ => None,
        }
    }

    /// The reverse: how this location is written when a report or a supplied key
    /// has to name it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Header => "header",
            Self::Cookie => "cookie",
        }
    }
}

/// The scalar type a document declares for a parameter's value, recorded only
/// when its schema names exactly one of these three. An array, an object, a
/// number, a `oneOf` or a `$ref` records none, and a reader keeps its own
/// default for that parameter rather than guessing one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    String,
    Integer,
    Boolean,
}

impl ValueType {
    /// The type a parameter object's schema declares. OpenAPI 3.0 marks a
    /// nullable scalar with `nullable`; 3.1 writes `["integer", "null"]`. Both
    /// are the scalar: a parameter value is present or absent, never null.
    fn declared(parameter: &Value) -> Option<Self> {
        Self::of_schema(parameter.get("schema")?)
    }

    fn of_schema(schema: &Value) -> Option<Self> {
        match single_type(schema)? {
            "string" => Some(Self::String),
            "integer" => Some(Self::Integer),
            "boolean" => Some(Self::Boolean),
            _ => None,
        }
    }
}

/// The one type a schema names, with a 3.1 `"null"` member set aside as 3.0's
/// `nullable` is. A schema naming none, or several, names no single type.
fn single_type(schema: &Value) -> Option<&str> {
    match schema.get("type")? {
        Value::String(name) => Some(name),
        Value::Array(names) => {
            let names: Vec<&str> = names.iter().map(Value::as_str).collect::<Option<_>>()?;
            match names
                .into_iter()
                .filter(|name| *name != "null")
                .collect::<Vec<_>>()
                .as_slice()
            {
                [name] => Some(name),
                _ => None,
            }
        }
        _ => None,
    }
}

/// How a query parameter whose schema is an array is sent. OpenAPI 3.x: a
/// query parameter's `style` defaults to `form`, and `explode` defaults to true
/// for `form` and to false for every other style. Only `form` exploded — one
/// `name=value` pair per element — is sent as an array; any other shape is
/// named, with the parameter kept as the one value it was read as before.
fn array_shape(parameter: &Value, name: &str) -> Result<(), String> {
    let style = match parameter.get("style") {
        None => "form",
        Some(Value::String(style)) => style,
        Some(_) => {
            return Err(format!(
                "array query parameter `{name}` declares a `style` that is not a string; \
                 it is sent as one value as given"
            ));
        }
    };
    let explode = match parameter.get("explode") {
        None => style == "form",
        Some(Value::Bool(explode)) => *explode,
        Some(_) => {
            return Err(format!(
                "array query parameter `{name}` declares an `explode` that is not a boolean; \
                 it is sent as one value as given"
            ));
        }
    };
    if style == "form" && explode {
        Ok(())
    } else {
        Err(format!(
            "array query parameter `{name}` is serialised with style `{style}`, explode {explode}, \
             which this pass does not send as an array; it is sent as one value as given"
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub location: Location,
    pub required: bool,
    /// The declared scalar type, when the document gives one this model
    /// carries. Omitted when absent, so a parameter without one serialises as
    /// it did before types were recorded.
    /// For a repeated parameter, the type of each element.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<ValueType>,
    /// A query parameter declared as an array sent `style: form, explode:
    /// true`: one `name=value` pair per element. Omitted when false, so a
    /// parameter that is not repeated serialises as it did before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub repeated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    /// The status key exactly as written, including `default` and `2XX`.
    pub status: String,
    pub media_types: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub method: String,
    pub path: String,
    /// Absent in a document that declares none; the method and path still identify it.
    pub operation_id: Option<String>,
    pub parameters: Vec<Parameter>,
    pub request_media_types: Vec<String>,
    pub responses: Vec<Response>,
}

impl Operation {
    /// How this operation is named when it has no `operationId`.
    pub fn designation(&self) -> String {
        match &self.operation_id {
            Some(id) => id.clone(),
            None => format!("{} {}", self.method.to_uppercase(), self.path),
        }
    }
}

/// Something the document declares that this pass does not represent. It carries
/// the operation it belongs to, so a reader can decide whether the gap matters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unsupported {
    pub designation: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub operations: Vec<Operation>,
    pub unsupported: Vec<Unsupported>,
}

impl Inventory {
    /// Inventoried and unsupported are reported apart. One total would read as
    /// coverage this pass has not established.
    pub fn coverage(&self) -> (usize, usize) {
        (self.operations.len(), self.unsupported.len())
    }
}

fn media_types(container: Option<&Value>) -> Vec<String> {
    container
        .and_then(|c| c.get("content"))
        .and_then(Value::as_object)
        .map(|content| content.keys().cloned().collect())
        .unwrap_or_default()
}

/// One declared parameter, with the gap its shape leaves when it has one. The
/// gap is held beside the parameter rather than recorded at once, because an
/// operation's own declaration may replace the path item's, and a replaced
/// declaration leaves no gap in the operation.
type Declared = (Parameter, Option<String>);

fn parameters(
    raw: Option<&Value>,
    designation: &str,
    gaps: &mut Vec<Unsupported>,
) -> Vec<Declared> {
    let mut out = Vec::new();
    let Some(list) = raw.and_then(Value::as_array) else {
        return out;
    };
    for parameter in list {
        if parameter.get("$ref").is_some() {
            gaps.push(Unsupported {
                designation: designation.to_owned(),
                reason: "parameter is a $ref this pass does not resolve".into(),
            });
            continue;
        }
        let name = parameter.get("name").and_then(Value::as_str);
        let location = parameter.get("in").and_then(Value::as_str);
        match (name, location) {
            (Some(name), Some(location)) => match Location::parse(location) {
                Some(location) => {
                    let schema = parameter.get("schema");
                    let array = location == Location::Query
                        && schema.and_then(single_type) == Some("array");
                    let shape = array.then(|| array_shape(parameter, name));
                    let repeated = matches!(shape, Some(Ok(())));
                    let value_type = if repeated {
                        schema
                            .and_then(|s| s.get("items"))
                            .and_then(ValueType::of_schema)
                    } else {
                        ValueType::declared(parameter)
                    };
                    out.push((
                        Parameter {
                            name: name.to_owned(),
                            location,
                            required: parameter
                                .get("required")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            value_type,
                            repeated,
                        },
                        shape.and_then(Result::err),
                    ))
                }
                None => gaps.push(Unsupported {
                    designation: designation.to_owned(),
                    reason: format!("parameter `{name}` declares location `{location}`"),
                }),
            },
            _ => gaps.push(Unsupported {
                designation: designation.to_owned(),
                reason: "parameter declares no name or no location".into(),
            }),
        }
    }
    out
}

fn responses(raw: Option<&Value>) -> Vec<Response> {
    let Some(object) = raw.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut out: Vec<Response> = object
        .iter()
        .map(|(status, body)| Response {
            status: status.clone(),
            media_types: media_types(Some(body)),
        })
        .collect();
    out.sort_by(|a, b| a.status.cmp(&b.status));
    out
}

/// The path of one server URL, without a trailing slash. The authority —
/// scheme, host, port, templated or not — is dropped; only the path decides
/// where an operation's path sits below it.
fn url_path(url: &str) -> Result<&str, &'static str> {
    let url = url.split(['?', '#']).next().unwrap_or_default();
    let path = if let Some(start) = url.find("://") {
        let rest = &url[start + 3..];
        rest.find('/').map_or("", |slash| &rest[slash..])
    } else if let Some(rest) = url.strip_prefix("//") {
        rest.find('/').map_or("", |slash| &rest[slash..])
    } else if url.is_empty() || url.starts_with('/') {
        url
    } else {
        return Err("a document server url is relative to the document");
    };
    if path.contains(['{', '}']) {
        return Err("a document server path carries a template variable");
    }
    Ok(path.trim_end_matches('/'))
}

/// The base path the document's operation paths are appended to (OpenAPI 3.x:
/// a path is relative to the server URL). Empty when the document declares no
/// servers, or only servers at the authority's root — every document whose
/// paths already carry their base. Every server must agree, because an
/// inventory records one path per operation.
fn server_path(document: &Value) -> Result<String, &'static str> {
    let Some(servers) = document.get("servers") else {
        return Ok(String::new());
    };
    let servers = servers
        .as_array()
        .ok_or("the document's `servers` is not a list")?;
    let mut agreed: Option<&str> = None;
    for server in servers {
        let url = server
            .get("url")
            .and_then(Value::as_str)
            .ok_or("a document server declares no url")?;
        let path = url_path(url)?;
        match agreed {
            Some(previous) if previous != path => {
                return Err("the document's servers declare different base paths");
            }
            _ => agreed = Some(path),
        }
    }
    Ok(agreed.unwrap_or_default().to_owned())
}

/// Build the inventory from a parsed document. The document is the one the source
/// ingest accepted; this function re-reads it rather than trusting a cached shape.
/// Each recorded path is the document's path below its servers' base path, so it
/// is the path a request sends below the provider's authority. Servers that give
/// no single base are named unsupported, and paths are then recorded as written.
pub fn extract(document: &Value) -> Inventory {
    let mut operations = Vec::new();
    let mut unsupported = Vec::new();

    for member in ["webhooks", "callbacks"] {
        if document.get(member).is_some() {
            unsupported.push(Unsupported {
                designation: format!("document.{member}"),
                reason: format!("`{member}` is a document member this build does not read"),
            });
        }
    }
    let base = server_path(document).unwrap_or_else(|reason| {
        unsupported.push(Unsupported {
            designation: "document.servers".into(),
            reason: reason.into(),
        });
        String::new()
    });

    let Some(paths) = document.get("paths").and_then(Value::as_object) else {
        return Inventory {
            operations,
            unsupported,
        };
    };

    for (written, item) in paths {
        let path = &format!("{base}{written}");
        if item.get("$ref").is_some() {
            unsupported.push(Unsupported {
                designation: path.clone(),
                reason: "path item is a $ref this pass does not resolve".into(),
            });
            continue;
        }
        // OpenAPI 3.x: servers on a path item or an operation override the
        // document's for it. This pass records one base, so an override is
        // named, and its operations are not recorded under the document's base,
        // where they do not live.
        if item.get("servers").is_some() {
            unsupported.push(Unsupported {
                designation: written.clone(),
                reason: "path item declares its own `servers`, overriding the document's; \
                         its operations are not recorded under the document's base"
                    .into(),
            });
            continue;
        }
        let shared = item.get("parameters");
        for method in METHODS {
            let Some(body) = item.get(method) else {
                continue;
            };
            if body.get("servers").is_some() {
                let named = body
                    .get("operationId")
                    .and_then(Value::as_str)
                    .map(|id| format!(" `{id}`"))
                    .unwrap_or_default();
                unsupported.push(Unsupported {
                    designation: format!("{} {written}", method.to_uppercase()),
                    reason: format!(
                        "operation{named} declares its own `servers`, overriding the document's; \
                         it is not recorded under the document's base"
                    ),
                });
                continue;
            }
            let operation_id = body
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let designation = match &operation_id {
                Some(id) => id.clone(),
                None => format!("{} {}", method.to_uppercase(), path),
            };
            let mut declared = parameters(shared, &designation, &mut unsupported);
            for parameter in parameters(body.get("parameters"), &designation, &mut unsupported) {
                // OpenAPI 3.1 section 4.8.9.1: the operation's own parameter
                // overrides the path item's of the same name and location, and a
                // parameter's identity is that pair. Keeping both would inventory
                // one parameter twice and lose the override's `required`.
                match declared
                    .iter_mut()
                    .find(|(d, _)| d.name == parameter.0.name && d.location == parameter.0.location)
                {
                    Some(overridden) => *overridden = parameter,
                    None => declared.push(parameter),
                }
            }
            let declared: Vec<Parameter> = declared
                .into_iter()
                .map(|(parameter, gap)| {
                    if let Some(reason) = gap {
                        unsupported.push(Unsupported {
                            designation: designation.clone(),
                            reason,
                        });
                    }
                    parameter
                })
                .collect();
            let request = body.get("requestBody");
            if request.map(|r| r.get("$ref").is_some()).unwrap_or(false) {
                unsupported.push(Unsupported {
                    designation: designation.clone(),
                    reason: "request body is a $ref this pass does not resolve".into(),
                });
            }
            operations.push(Operation {
                method: method.to_owned(),
                path: path.clone(),
                operation_id,
                parameters: declared,
                request_media_types: media_types(request),
                responses: responses(body.get("responses")),
            });
        }
    }

    Inventory {
        operations,
        unsupported,
    }
}
