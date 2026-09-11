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
    fn parse(value: &str) -> Option<Self> {
        match value {
            "path" => Some(Self::Path),
            "query" => Some(Self::Query),
            "header" => Some(Self::Header),
            "cookie" => Some(Self::Cookie),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub location: Location,
    pub required: bool,
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

fn parameters(raw: Option<&Value>, designation: &str, gaps: &mut Vec<Unsupported>) -> Vec<Parameter> {
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
                Some(location) => out.push(Parameter {
                    name: name.to_owned(),
                    location,
                    required: parameter
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                }),
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

/// Build the inventory from a parsed document. The document is the one the source
/// ingest accepted; this function re-reads it rather than trusting a cached shape.
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

    let Some(paths) = document.get("paths").and_then(Value::as_object) else {
        return Inventory {
            operations,
            unsupported,
        };
    };

    for (path, item) in paths {
        if item.get("$ref").is_some() {
            unsupported.push(Unsupported {
                designation: path.clone(),
                reason: "path item is a $ref this pass does not resolve".into(),
            });
            continue;
        }
        let shared = item.get("parameters");
        for method in METHODS {
            let Some(body) = item.get(method) else {
                continue;
            };
            let operation_id = body
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let designation = match &operation_id {
                Some(id) => id.clone(),
                None => format!("{} {}", method.to_uppercase(), path),
            };
            let mut declared = parameters(shared, &designation, &mut unsupported);
            declared.extend(parameters(body.get("parameters"), &designation, &mut unsupported));
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
