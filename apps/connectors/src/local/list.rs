//! `connectors list` (story:cli-lists-available-connectors;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Listing connectors"): one row
//! per connector whose declaration this build ships, with its auth profiles
//! and the instances the configuration names. It needs no configuration and
//! no state directory. A configuration it finds is checked as every command
//! checks it, but no instance is derived, so it starts no process.
use super::*;
use std::collections::BTreeMap;

pub(super) fn execute(call: &Invocation<'_>) -> Result<Value, HandlerReply> {
    // Only the configuration's path is resolved: `list` reads no state.
    let listed = match Paths::config(call.context.config.as_deref()) {
        Err(_) => BTreeMap::new(),
        Ok(config) => match Config::listed_instances(&config) {
            Ok(listed) => listed,
            Err(Refusal::Rule(Rule::ConfigMissing)) => BTreeMap::new(),
            Err(refusal) => return Err(configuration_refusal(refusal)),
        },
    };
    let mut connectors = declarations::all()
        .iter()
        .filter_map(|declaration| {
            let id = declaration["connector_id"].as_str()?;
            let profiles = declaration["profiles"]
                .as_array()
                .map(|profiles| {
                    profiles
                        .iter()
                        .filter_map(|profile| profile["profile"].as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let instances = listed.get(id).cloned().unwrap_or_default();
            Some(json!({"id":id,"profiles":profiles,"instances":instances}))
        })
        .collect::<Vec<_>>();
    connectors.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(json!({"connectors":connectors}))
}

/// The human rendering of a successful result's stdout (`{result}\n`): a
/// table with one row per connector, `-` where a row has none.
pub(super) fn human(stdout: &str) -> Option<String> {
    let result: Value = serde_json::from_str(stdout.trim_end()).ok()?;
    let joined = |row: &Value, field: &str| {
        let words = row[field]
            .as_array()
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        if words.is_empty() {
            "-".to_owned()
        } else {
            words.join(", ")
        }
    };
    let mut rows = vec![[
        "CONNECTOR".to_owned(),
        "PROFILES".to_owned(),
        "INSTANCES".to_owned(),
    ]];
    for row in result["connectors"].as_array()? {
        rows.push([
            row["id"].as_str()?.to_owned(),
            joined(row, "profiles"),
            joined(row, "instances"),
        ]);
    }
    let width = |column: usize| rows.iter().map(|row| row[column].len()).max().unwrap_or(0);
    let (first, second) = (width(0), width(1));
    Some(
        rows.iter()
            .map(|[id, profiles, instances]| {
                format!("{id:<first$}  {profiles:<second$}  {instances}\n")
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_a_header_and_a_dash_for_none() {
        let stdout = format!(
            "{}\n",
            json!({"connectors":[
                {"id":"gitlab","profiles":["gitlab.pat"],"instances":["work"]},
                {"id":"zendesk","profiles":["zendesk.oauth","zendesk.basic"],"instances":[]}
            ]})
        );
        let expected = [
            format!("CONNECTOR  PROFILES{}  INSTANCES\n", " ".repeat(20)),
            format!("gitlab     gitlab.pat{}  work\n", " ".repeat(18)),
            "zendesk    zendesk.oauth, zendesk.basic  -\n".to_owned(),
        ]
        .concat();
        assert_eq!(human(&stdout).unwrap(), expected);
    }
}
