//! Identity-stable overlay fields keyed by source service and vendor operation id.

use std::collections::BTreeMap;

use crate::OperationDirection;

use super::{IngestedDocument, Patch};

pub(super) fn direction_for(
    patch: &Patch,
    service: &str,
    operation_id: &str,
) -> Option<OperationDirection> {
    patch
        .directions
        .get(service)
        .and_then(|directions| directions.get(operation_id))
        .copied()
}

pub(super) fn description_for<'a>(
    patch: &'a Patch,
    service: &str,
    operation_id: &str,
) -> Option<&'a str> {
    patch
        .descriptions
        .get(service)
        .and_then(|descriptions| descriptions.get(operation_id))
        .map(String::as_str)
}

pub(super) fn check_directions(
    directions: &BTreeMap<String, BTreeMap<String, OperationDirection>>,
    ingested: &[IngestedDocument],
    problems: &mut Vec<String>,
) {
    for (service, operations) in directions {
        let Some(document) = ingested
            .iter()
            .find(|document| &document.service == service)
        else {
            problems.push(format!(
                "`[patch.directions.{service}]` names no ingested service. Direction is keyed by \
                 stable service and vendor `operationId`; available services: {}",
                ingested
                    .iter()
                    .map(|document| document.service.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        };
        for operation_id in operations.keys() {
            if document.ingested.operation(operation_id).is_none() {
                problems.push(format!(
                    "`[patch.directions.{service}]` names no `operationId` {operation_id:?} in {}. \
                     A renamed or removed upstream operation must be reviewed rather than silently \
                     losing its direction",
                    document.path
                ));
            }
        }
    }
}

pub(super) fn check_descriptions(
    descriptions: &BTreeMap<String, BTreeMap<String, String>>,
    ingested: &[IngestedDocument],
    problems: &mut Vec<String>,
) {
    for (service, operations) in descriptions {
        let Some(document) = ingested
            .iter()
            .find(|document| &document.service == service)
        else {
            problems.push(format!(
                "`[patch.descriptions.{service}]` names no ingested service. Descriptions are keyed \
                 by stable service and vendor `operationId`; available services: {}",
                ingested
                    .iter()
                    .map(|document| document.service.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        };
        for (operation_id, description) in operations {
            if document.ingested.operation(operation_id).is_none() {
                problems.push(format!(
                    "`[patch.descriptions.{service}]` names no `operationId` {operation_id:?} in \
                     {}. A renamed or removed upstream operation must be reviewed rather than \
                     silently losing its description",
                    document.path
                ));
            }
            if description.trim().is_empty() {
                problems.push(format!(
                    "`[patch.descriptions.{service}]` gives `operationId` {operation_id:?} an empty \
                     description; a correction must state the source-grounded one-line fact"
                ));
            }
        }
    }
}
