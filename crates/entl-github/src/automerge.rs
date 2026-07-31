use serde_yaml_ng::Value;

use crate::DependabotAutomergeWorkflowFacts;

pub fn inspect_dependabot_automerge_workflow(
    text: &str,
) -> Result<DependabotAutomergeWorkflowFacts, String> {
    let value = serde_yaml_ng::from_str::<Value>(text)
        .map_err(|error| format!("workflow is invalid YAML: {error}"))?;
    let triggers = mapping_value(&value, "on")
        .map(trigger_names)
        .unwrap_or_default();
    let mut facts = DependabotAutomergeWorkflowFacts {
        pull_request_trigger: triggers
            .iter()
            .any(|trigger| matches!(trigger.as_str(), "pull_request" | "pull_request_target")),
        ..DependabotAutomergeWorkflowFacts::default()
    };
    let Some(jobs) = mapping_value(&value, "jobs").and_then(Value::as_mapping) else {
        return Ok(facts);
    };
    for job in jobs.values() {
        if let Some(condition) = mapping_value(job, "if").and_then(Value::as_str) {
            inspect_condition(condition, &mut facts);
        }
        let Some(steps) = mapping_value(job, "steps").and_then(Value::as_sequence) else {
            continue;
        };
        for step in steps {
            if let Some(condition) = mapping_value(step, "if").and_then(Value::as_str) {
                inspect_condition(condition, &mut facts);
            }
            if mapping_value(step, "uses")
                .and_then(Value::as_str)
                .and_then(|uses| uses.split('@').next())
                .is_some_and(|action| action.eq_ignore_ascii_case("dependabot/fetch-metadata"))
            {
                facts.fetches_metadata = true;
            }
            if let Some(command) = mapping_value(step, "run").and_then(Value::as_str) {
                let command = command.to_ascii_lowercase();
                if (command.contains("gh pr merge") && command.contains("--auto"))
                    || command.contains("enablepullrequestautomerge")
                {
                    facts.enables_auto_merge = true;
                }
            }
        }
    }
    Ok(facts)
}

fn inspect_condition(condition: &str, facts: &mut DependabotAutomergeWorkflowFacts) {
    let condition = condition.to_ascii_lowercase();
    if condition.contains("dependabot[bot]") {
        facts.dependabot_only = true;
    }
    if condition.contains("update-type")
        && condition.contains("semver-major")
        && (condition.contains("!=") || condition.contains("!= "))
    {
        facts.excludes_major_updates = true;
    }
}

fn trigger_names(value: &Value) -> Vec<String> {
    match value {
        Value::String(trigger) => vec![trigger.clone()],
        Value::Sequence(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Value::Mapping(values) => values
            .keys()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn mapping_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_mapping()?.get(Value::String(key.to_owned()))
}
