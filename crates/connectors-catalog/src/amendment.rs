//! Cited amendments to a pinned source, of three kinds: an optional query
//! parameter the vendor's own reference documents for an operation that its
//! published OpenAPI document leaves out, a path correction that resolves
//! a vendor notation in the operation's path template which the template
//! reader would otherwise send literally, such as GitLab's `(-/)` marking an
//! optional segment, and a request media type correction for a write the
//! vendor documents with a JSON body while its document declares only a form
//! media type, such as GitLab's commit create. The pinned bytes stay the
//! vendor's (and their digest stays the one the source record and the hash
//! manifest name); the amendment file sits beside them, is bound to their
//! digest, and is applied to the inventory the pipeline extracts, so the bundle
//! carries the change and records the file it came from.
//!
//! No kind can retarget an operation or change its method. An added
//! parameter only widens what a caller may send: it is one optional,
//! unrepeated query parameter. A corrected path must be the operation's
//! current path with each parenthesised group either removed or kept without
//! its parentheses, so it keeps every path parameter and literal segment the
//! source declares outside those groups. A media type correction replaces the
//! one request media type the operation declares with `application/json`, the
//! only body the engine sends, so it makes the bundle say what the engine
//! does. Everything else is refused.

use crate::SourceRecord;
use crate::inventory::{Inventory, Location, Operation, Parameter};
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

/// One cited amendment: exactly one of `add_parameter`, `correct_path` and
/// `correct_media_type`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amendment {
    operation_id: String,
    #[serde(default)]
    add_parameter: Option<Parameter>,
    #[serde(default)]
    correct_path: Option<PathCorrection>,
    #[serde(default)]
    correct_media_type: Option<MediaTypeCorrection>,
    /// The vendor page that documents the change, https.
    cite: String,
    /// Why the pinned document needs it, in a sentence.
    reason: String,
}

/// An operation's path template as the source declares it and as the
/// provider serves it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathCorrection {
    from: String,
    to: String,
}

/// The one request media type an operation declares, and the one the provider
/// reads.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaTypeCorrection {
    from: String,
    to: String,
}

/// The only request media type a correction may name: the body the engine
/// sends.
const JSON: &str = "application/json";

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
/// list, an amendment naming both or neither change, an operation the inventory
/// lacks or holds twice, a citation that is not an https URL or a blank reason;
/// for an added parameter, one that is not an optional, unrepeated query
/// parameter or that the operation already declares; for a path correction, a
/// `from` other than the operation's current path, or a `to` equal to `from` or
/// not derived from it by removing or unwrapping each parenthesised group; for
/// a media type correction, an operation that does not declare exactly one
/// request media type, a `from` other than that one, or a `to` that is not
/// `application/json` or equals `from`.
/// Nothing is applied when any amendment is refused.
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
        let id = &amendment.operation_id;
        if !amendment.cite.starts_with("https://") || amendment.reason.trim().is_empty() {
            return Err(invalid(format!(
                "`{file_name}` amends `{id}` without an https citation and a reason"
            )));
        }
        let mut matching = amended
            .iter_mut()
            .filter(|operation| operation.operation_id.as_deref() == Some(id));
        let (Some(operation), None) = (matching.next(), matching.next()) else {
            return Err(invalid(format!(
                "`{file_name}` amends `{id}`, which the source does not declare exactly once"
            )));
        };
        match (
            &amendment.add_parameter,
            &amendment.correct_path,
            &amendment.correct_media_type,
        ) {
            (Some(parameter), None, None) => add_parameter(id, parameter, operation)?,
            (None, Some(correction), None) => correct_path(id, correction, operation)?,
            (None, None, Some(correction)) => correct_media_type(id, correction, operation)?,
            _ => {
                return Err(invalid(format!(
                    "`{file_name}` amends `{id}` with none or several of `add_parameter`, `correct_path` and `correct_media_type`; an amendment makes exactly one change"
                )));
            }
        }
    }
    inventory.operations = amended;
    Ok(AmendmentRecord {
        file_name: file_name.to_owned(),
        sha256: hex::encode(Sha256::digest(bytes)),
        format: FORMAT.to_owned(),
        count: amendments.amendments.len(),
    })
}

fn add_parameter(id: &str, parameter: &Parameter, operation: &mut Operation) -> Result<(), Error> {
    if parameter.location != Location::Query
        || parameter.required
        || parameter.repeated
        || parameter.name.is_empty()
    {
        return Err(invalid(format!(
            "`{}` on `{id}`: an amendment adds one named, optional, unrepeated query parameter",
            parameter.name
        )));
    }
    if operation
        .parameters
        .iter()
        .any(|declared| declared.name == parameter.name && declared.location == parameter.location)
    {
        return Err(invalid(format!(
            "`{id}` already declares `{}`; the amendment is stale",
            parameter.name
        )));
    }
    operation.parameters.push(parameter.clone());
    Ok(())
}

fn correct_path(
    id: &str,
    correction: &PathCorrection,
    operation: &mut Operation,
) -> Result<(), Error> {
    if correction.from != operation.path {
        return Err(invalid(format!(
            "`{id}` has the path `{}`, not `{}`; the correction is stale",
            operation.path, correction.from
        )));
    }
    if correction.to == correction.from || !resolves(&correction.from, &correction.to) {
        return Err(invalid(format!(
            "`{id}`: `{}` is not `{}` with each parenthesised optional segment removed or kept without its parentheses (a segment holding a path parameter is kept)",
            correction.to, correction.from
        )));
    }
    operation.path.clone_from(&correction.to);
    Ok(())
}

fn correct_media_type(
    id: &str,
    correction: &MediaTypeCorrection,
    operation: &mut Operation,
) -> Result<(), Error> {
    if operation.request_media_types != [correction.from.as_str()] {
        return Err(invalid(format!(
            "`{id}` declares the request media types {:?}, not only `{}`; the correction is stale",
            operation.request_media_types, correction.from
        )));
    }
    if correction.to != JSON || correction.to == correction.from {
        return Err(invalid(format!(
            "`{id}`: a media type correction replaces `{}` with `{JSON}`, the body the engine sends",
            correction.from
        )));
    }
    operation.request_media_types = vec![correction.to.clone()];
    Ok(())
}

/// One piece of a path template: literal text, or the contents of a
/// parenthesised optional group.
enum Piece<'a> {
    Literal(&'a str),
    Optional(&'a str),
}

/// Splits `path` into literal text and parenthesised groups; `None` when a
/// parenthesis is unbalanced or nested.
fn pieces(path: &str) -> Option<Vec<Piece<'_>>> {
    let mut pieces = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find(['(', ')']) {
        if rest.as_bytes()[open] == b')' {
            return None;
        }
        let close = open + 1 + rest[open + 1..].find(['(', ')'])?;
        if rest.as_bytes()[close] == b'(' {
            return None;
        }
        pieces.push(Piece::Literal(&rest[..open]));
        pieces.push(Piece::Optional(&rest[open + 1..close]));
        rest = &rest[close + 1..];
    }
    pieces.push(Piece::Literal(rest));
    Some(pieces)
}

/// Whether `to` is `from` with each parenthesised group either removed or kept
/// without its parentheses. A group holding a path parameter may only be kept,
/// so a correction never drops a parameter and cannot retarget the operation.
fn resolves(from: &str, to: &str) -> bool {
    fn matches(pieces: &[Piece<'_>], to: &str) -> bool {
        match pieces.split_first() {
            None => to.is_empty(),
            Some((Piece::Literal(text), rest)) => {
                to.strip_prefix(text).is_some_and(|to| matches(rest, to))
            }
            Some((Piece::Optional(text), rest)) => {
                (!text.contains('{') && matches(rest, to))
                    || to.strip_prefix(text).is_some_and(|to| matches(rest, to))
            }
        }
    }
    pieces(from).is_some_and(|pieces| matches(&pieces, to))
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

    const SEARCH: &str = "/api/v4/projects/{id}/(-/)search";

    /// An inventory holding one operation whose path carries GitLab's `(-/)`
    /// notation for an optional segment, beside the Zendesk export.
    fn search_inventory() -> Inventory {
        let mut inventory = inventory();
        inventory.operations.push(Operation {
            method: "get".into(),
            path: SEARCH.into(),
            operation_id: Some("getApiV4ProjectsIdDashSearch".into()),
            parameters: vec![Parameter {
                name: "id".into(),
                location: Location::Path,
                required: true,
                value_type: Some(ValueType::String),
                repeated: false,
            }],
            request_media_types: Vec::new(),
            responses: Vec::new(),
        });
        inventory
    }

    fn corrections(sha: &str, entries: &[(&str, &str, &str)]) -> Vec<u8> {
        let entries: Vec<String> = entries
            .iter()
            .map(|(operation, from, to)| {
                format!(
                    r#"{{"operation_id":"{operation}","correct_path":{{"from":"{from}","to":"{to}"}},
                    "cite":"https://docs.gitlab.example.test/api/search/","reason":"documented as served"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{}]}}"#,
            entries.join(",")
        )
        .into_bytes()
    }

    /// A cited path correction replaces the operation's template with the
    /// path the provider serves, keeps its method and parameters, and is
    /// recorded like an added parameter; the other operation is untouched.
    #[test]
    fn a_path_correction_replaces_the_template_and_is_recorded() {
        let source = source();
        let mut inventory = search_inventory();
        let bytes = corrections(
            &source.source_sha256,
            &[(
                "getApiV4ProjectsIdDashSearch",
                SEARCH,
                "/api/v4/projects/{id}/search",
            )],
        );
        let record = apply("amendments.json", &bytes, &source, &mut inventory).unwrap();
        assert_eq!(record.count, 1);
        assert_eq!(record.format, FORMAT);
        assert_eq!(record.sha256, hex::encode(Sha256::digest(&bytes)));
        let mut expected = search_inventory();
        expected.operations[1].path = "/api/v4/projects/{id}/search".into();
        assert_eq!(inventory, expected);
    }

    /// Keeping the optional segment, without its parentheses, is the other
    /// reading the notation admits.
    #[test]
    fn a_path_correction_may_keep_the_optional_segment() {
        let source = source();
        let mut inventory = search_inventory();
        let bytes = corrections(
            &source.source_sha256,
            &[(
                "getApiV4ProjectsIdDashSearch",
                SEARCH,
                "/api/v4/projects/{id}/-/search",
            )],
        );
        apply("amendments.json", &bytes, &source, &mut inventory).unwrap();
        assert_eq!(
            inventory.operations[1].path,
            "/api/v4/projects/{id}/-/search"
        );
    }

    /// A correction can only resolve the source's own optional-segment
    /// notation: a stale `from`, a `to` that retargets the operation, drops
    /// or renames a path parameter, or leaves the notation in place, an
    /// entry naming both or neither change, and a second correction of the
    /// same operation are each refused, and nothing is applied.
    #[test]
    fn every_path_correction_refusal_leaves_the_inventory_unchanged() {
        let source = source();
        let sha = source.source_sha256.clone();
        let op = "getApiV4ProjectsIdDashSearch";
        let mut refused: Vec<Vec<u8>> = [
            (
                "/api/v4/projects/{id}/search",
                "/api/v4/projects/{id}/search",
            ),
            (SEARCH, "/api/v4/groups/{id}/search"),
            (SEARCH, "/api/v4/projects/search"),
            (SEARCH, "/api/v4/projects/{project}/search"),
            (SEARCH, "/api/v4/projects/{id}/(-/)search"),
            (SEARCH, "/api/v4/projects/{id}/-search"),
            (SEARCH, "/api/v4/projects/{id}/search/blobs"),
            (SEARCH, ""),
        ]
        .iter()
        .map(|(from, to)| corrections(&sha, &[(op, from, to)]))
        .collect();
        refused.push(corrections(
            &sha,
            &[
                (op, SEARCH, "/api/v4/projects/{id}/search"),
                (op, SEARCH, "/api/v4/projects/{id}/-/search"),
            ],
        ));
        refused.push(corrections(
            &sha,
            &[(
                "IncrementalTicketExportCursor",
                "/api/v2/incremental/tickets/cursor",
                "/api/v2/incremental/tickets",
            )],
        ));
        refused.push(
            format!(
                r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{{"operation_id":"{op}",
                "add_parameter":{PER_PAGE},"correct_path":{{"from":"{SEARCH}","to":"/api/v4/projects/{{id}}/search"}},
                "cite":"https://docs.gitlab.example.test/api/search/","reason":"both"}}]}}"#
            )
            .into_bytes(),
        );
        refused.push(
            format!(
                r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{{"operation_id":"{op}",
                "cite":"https://docs.gitlab.example.test/api/search/","reason":"neither"}}]}}"#
            )
            .into_bytes(),
        );
        refused.push(
            String::from_utf8(corrections(
                &sha,
                &[(op, SEARCH, "/api/v4/projects/{id}/search")],
            ))
            .unwrap()
            .replace(
                "https://docs.gitlab.example.test",
                "http://docs.gitlab.example.test",
            )
            .into_bytes(),
        );
        for bytes in refused {
            let mut inventory = search_inventory();
            assert!(
                apply("amendments.json", &bytes, &source, &mut inventory).is_err(),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
            assert_eq!(inventory, search_inventory());
        }
    }

    /// An inventory holding one write whose source declares its body only as
    /// `multipart/form-data`, as GitLab's commit create does, beside the
    /// Zendesk export, which declares no body.
    fn form_inventory() -> Inventory {
        let mut inventory = inventory();
        inventory.operations.push(Operation {
            method: "post".into(),
            path: "/api/v4/projects/{id}/repository/commits".into(),
            operation_id: Some("postApiV4ProjectsIdRepositoryCommits".into()),
            parameters: vec![Parameter {
                name: "id".into(),
                location: Location::Path,
                required: true,
                value_type: None,
                repeated: false,
            }],
            request_media_types: vec!["multipart/form-data".into()],
            responses: Vec::new(),
        });
        inventory
    }

    fn media(sha: &str, entries: &[(&str, &str, &str)]) -> Vec<u8> {
        let entries: Vec<String> = entries
            .iter()
            .map(|(operation, from, to)| {
                format!(
                    r#"{{"operation_id":"{operation}","correct_media_type":{{"from":"{from}","to":"{to}"}},
                    "cite":"https://docs.gitlab.example.test/api/commits/","reason":"documented as JSON"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{}]}}"#,
            entries.join(",")
        )
        .into_bytes()
    }

    /// A cited media type correction replaces the one declared request media
    /// type with `application/json` and changes nothing else; it is recorded
    /// like the other kinds.
    #[test]
    fn a_media_type_correction_declares_json_and_is_recorded() {
        let source = source();
        let mut inventory = form_inventory();
        let bytes = media(
            &source.source_sha256,
            &[(
                "postApiV4ProjectsIdRepositoryCommits",
                "multipart/form-data",
                "application/json",
            )],
        );
        let record = apply("amendments.json", &bytes, &source, &mut inventory).unwrap();
        assert_eq!(record.count, 1);
        assert_eq!(record.sha256, hex::encode(Sha256::digest(&bytes)));
        let mut expected = form_inventory();
        expected.operations[1].request_media_types = vec!["application/json".into()];
        assert_eq!(inventory, expected);
    }

    /// A media type correction can only make a form-declared body JSON: a
    /// stale `from`, a `to` other than `application/json`, a `to` equal to
    /// `from`, an operation that declares no body, a second correction of the
    /// same operation and an entry naming it beside another change are each
    /// refused, and nothing is applied.
    #[test]
    fn every_media_type_correction_refusal_leaves_the_inventory_unchanged() {
        let source = source();
        let sha = source.source_sha256.clone();
        let op = "postApiV4ProjectsIdRepositoryCommits";
        let mut refused: Vec<Vec<u8>> = [
            (op, "application/x-www-form-urlencoded", "application/json"),
            (op, "multipart/form-data", "text/plain"),
            (op, "multipart/form-data", "multipart/form-data"),
            (op, "multipart/form-data", "application/json; charset=utf-8"),
            (
                "IncrementalTicketExportCursor",
                "multipart/form-data",
                "application/json",
            ),
        ]
        .iter()
        .map(|entry| media(&sha, &[*entry]))
        .collect();
        refused.push(media(
            &sha,
            &[
                (op, "multipart/form-data", "application/json"),
                (op, "multipart/form-data", "application/json"),
            ],
        ));
        refused.push(
            format!(
                r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{{"operation_id":"{op}",
                "add_parameter":{PER_PAGE},"correct_media_type":{{"from":"multipart/form-data","to":"application/json"}},
                "cite":"https://docs.gitlab.example.test/api/commits/","reason":"both"}}]}}"#
            )
            .into_bytes(),
        );
        refused.push(
            format!(
                r#"{{"format":"{FORMAT}","source_sha256":"{sha}","amendments":[{{"operation_id":"{op}",
                "correct_path":{{"from":"/api/v4/projects/{{id}}/repository/commits","to":"/api/v4/projects/{{id}}/commits"}},
                "correct_media_type":{{"from":"multipart/form-data","to":"application/json"}},
                "cite":"https://docs.gitlab.example.test/api/commits/","reason":"both"}}]}}"#
            )
            .into_bytes(),
        );
        for bytes in refused {
            let mut inventory = form_inventory();
            assert!(
                apply("amendments.json", &bytes, &source, &mut inventory).is_err(),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
            assert_eq!(inventory, form_inventory());
        }
    }
}
