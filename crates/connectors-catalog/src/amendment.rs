//! Cited amendments to a pinned source: an optional query parameter the vendor's
//! own reference documents for an operation that its published OpenAPI document
//! leaves out. The pinned bytes stay the vendor's (and their digest stays the one
//! the source record and the hash manifest name); the amendment file sits beside
//! them, is bound to their digest, and is applied to the inventory the pipeline
//! extracts, so the bundle carries the parameter and records the file it came
//! from. An amendment only widens what a caller may send: it adds an optional
//! query parameter and refuses everything else, so it cannot make a request the
//! selection did not already admit fail, retarget one, or change its method.

use crate::SourceRecord;
use crate::inventory::{Inventory, Location, Parameter};
use connectors_core::{Error, ErrorCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The one format this module reads.
pub const FORMAT: &str = "connectors-source-amendments/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amendments {
    format: String,
    /// The SHA-256 of the pinned source these amendments are written against.
    source_sha256: String,
    amendments: Vec<Amendment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amendment {
    operation_id: String,
    add_parameter: Parameter,
    /// The vendor page that documents the parameter, https.
    cite: String,
    /// Why the pinned document lacks it, in a sentence.
    reason: String,
}

/// What a bundle records about the amendments applied to its source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmendmentRecord {
    pub file_name: String,
    pub sha256: String,
    /// [`FORMAT`].
    pub format: String,
    pub count: usize,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::InvalidInput, message.into())
}

/// Applies the amendments in `bytes`, read from `file_name`, to `inventory`,
/// extracted from the source `source` describes.
///
/// # Errors
/// A malformed file, another format, a digest other than the source's, an empty
/// list, an operation the inventory lacks or holds twice, a parameter that is
/// not an optional query parameter or that the operation already declares, or a
/// citation that is not an https URL. Nothing is applied when any amendment is
/// refused.
pub fn apply(
    file_name: &str,
    bytes: &[u8],
    source: &SourceRecord,
    inventory: &mut Inventory,
) -> Result<AmendmentRecord, Error> {
    let amendments: Amendments = serde_json::from_slice(bytes)
        .map_err(|_| invalid(format!("`{file_name}` is not a {FORMAT} document")))?;
    if amendments.format != FORMAT {
        return Err(invalid(format!("`{file_name}` is not a {FORMAT} document")));
    }
    if amendments.source_sha256 != source.source_sha256 {
        return Err(invalid(format!(
            "`{file_name}` amends a source with SHA-256 {}, not `{}` ({})",
            amendments.source_sha256, source.file_name, source.source_sha256
        )));
    }
    if amendments.amendments.is_empty() {
        return Err(invalid(format!("`{file_name}` lists no amendment")));
    }
    let mut amended = inventory.operations.clone();
    for amendment in &amendments.amendments {
        let parameter = &amendment.add_parameter;
        if parameter.location != Location::Query || parameter.required || parameter.repeated {
            return Err(invalid(format!(
                "`{}` on `{}`: an amendment adds one optional, unrepeated query parameter",
                parameter.name, amendment.operation_id
            )));
        }
        if parameter.name.is_empty()
            || !amendment.cite.starts_with("https://")
            || amendment.reason.trim().is_empty()
        {
            return Err(invalid(format!(
                "`{}` on `{}`: an amendment names its parameter, cites an https page and gives a reason",
                parameter.name, amendment.operation_id
            )));
        }
        let mut matching = amended
            .iter_mut()
            .filter(|operation| operation.operation_id.as_deref() == Some(&amendment.operation_id));
        let (Some(operation), None) = (matching.next(), matching.next()) else {
            return Err(invalid(format!(
                "`{file_name}` amends `{}`, which the source does not declare exactly once",
                amendment.operation_id
            )));
        };
        if operation.parameters.iter().any(|declared| {
            declared.name == parameter.name && declared.location == parameter.location
        }) {
            return Err(invalid(format!(
                "`{}` already declares `{}`; the amendment is stale",
                amendment.operation_id, parameter.name
            )));
        }
        operation.parameters.push(parameter.clone());
    }
    inventory.operations = amended;
    Ok(AmendmentRecord {
        file_name: file_name.to_owned(),
        sha256: hex::encode(Sha256::digest(bytes)),
        format: FORMAT.to_owned(),
        count: amendments.amendments.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{Operation, ValueType};

    fn source() -> SourceRecord {
        crate::ingest(
            "fixture.json",
            br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},"paths":{}}"#,
        )
        .unwrap()
    }

    fn inventory() -> Inventory {
        Inventory {
            operations: vec![Operation {
                method: "get".into(),
                path: "/api/v2/incremental/tickets/cursor".into(),
                operation_id: Some("IncrementalTicketExportCursor".into()),
                parameters: vec![Parameter {
                    name: "cursor".into(),
                    location: Location::Query,
                    required: false,
                    value_type: Some(ValueType::String),
                    repeated: false,
                }],
                request_media_types: Vec::new(),
                responses: Vec::new(),
            }],
            unsupported: Vec::new(),
        }
    }

    fn cited(sha: &str, operation: &str, parameter: &str, cite: &str) -> Vec<u8> {
        format!(
            r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{{"operation_id":"{operation}",
            "add_parameter":{parameter},"cite":"{cite}","reason":"documented, not declared"}}]}}"#
        )
        .into_bytes()
    }

    fn document(sha: &str, operation: &str, parameter: &str) -> Vec<u8> {
        cited(
            sha,
            operation,
            parameter,
            "https://developer.example.test/exports",
        )
    }

    const PER_PAGE: &str =
        r#"{"name":"per_page","location":"query","required":false,"type":"integer"}"#;

    #[test]
    fn an_optional_query_parameter_is_added_and_recorded() {
        let source = source();
        let mut inventory = inventory();
        let bytes = document(
            &source.source_sha256,
            "IncrementalTicketExportCursor",
            PER_PAGE,
        );
        let record = apply("amendments.json", &bytes, &source, &mut inventory).unwrap();
        assert_eq!(record.count, 1);
        assert_eq!(record.sha256, hex::encode(Sha256::digest(&bytes)));
        let parameters = &inventory.operations[0].parameters;
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[1].name, "per_page");
        assert_eq!(parameters[1].value_type, Some(ValueType::Integer));
    }

    #[test]
    fn every_refusal_leaves_the_inventory_unchanged() {
        let source = source();
        let sha = source.source_sha256.clone();
        let refused = [
            document("00", "IncrementalTicketExportCursor", PER_PAGE),
            document(&sha, "NoSuchOperation", PER_PAGE),
            document(
                &sha,
                "IncrementalTicketExportCursor",
                r#"{"name":"cursor","location":"query","required":false,"type":"string"}"#,
            ),
            document(
                &sha,
                "IncrementalTicketExportCursor",
                r#"{"name":"per_page","location":"query","required":true,"type":"integer"}"#,
            ),
            document(
                &sha,
                "IncrementalTicketExportCursor",
                r#"{"name":"per_page","location":"path","required":false,"type":"integer"}"#,
            ),
            document(
                &sha,
                "IncrementalTicketExportCursor",
                r#"{"name":"per_page","location":"header","required":false,"type":"integer"}"#,
            ),
            format!(r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[]}}"#)
                .into_bytes(),
            br#"{"format":"connectors-source-amendments/2","source_sha256":"00","amendments":[]}"#
                .to_vec(),
            cited(
                &sha,
                "IncrementalTicketExportCursor",
                PER_PAGE,
                "http://developer.example.test/exports",
            ),
        ];
        for bytes in refused {
            let mut inventory = inventory();
            assert!(apply("amendments.json", &bytes, &source, &mut inventory).is_err());
            assert_eq!(inventory, self::inventory());
        }
    }
}
