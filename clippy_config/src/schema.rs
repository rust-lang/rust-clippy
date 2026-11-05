#![allow(dead_code)]

use crate::ConfMetadata;
use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema};
use serde_json::json;

pub(crate) fn schema_json<T: JsonSchema>(metadata: Vec<ConfMetadata>) -> String {
    let mut schema = SchemaSettings::draft07().into_generator().into_root_schema_for::<T>();
    let root = schema.as_object_mut().unwrap();
    root.remove("required");
    let properties = root.get_mut("properties").unwrap().as_object_mut().unwrap();
    properties.insert(
        "third-party".into(),
        json!({ "description": "Third-party configuration. Clippy ignores the value of this setting." }),
    );
    let mut aliases = Vec::new();
    for entry in metadata {
        if let Some(canonical) = entry.renamed_to {
            let mut alias = properties[canonical].clone();
            let alias_object = alias.as_object_mut().unwrap();
            alias_object.insert("deprecated".into(), true.into());
            let description = alias_object
                .get("description")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            alias_object.insert(
                "description".into(),
                format!("Deprecated. Use `{canonical}` instead.\n\n{description}").into(),
            );
            properties.insert(entry.name.into(), alias);
            aliases.push(json!({ "not": { "required": [entry.name, canonical] } }));
        }
    }
    if !aliases.is_empty() {
        root.insert("allOf".into(), aliases.into());
    }
    let mut output = serde_json::to_string_pretty(&schema).unwrap();
    output.push('\n');
    output
}

#[derive(JsonSchema)]
#[schemars(untagged)]
pub(crate) enum DisallowedPath {
    Path(String),
    Table(DisallowedPathTable),
}

#[derive(JsonSchema)]
#[schemars(untagged)]
pub(crate) enum DisallowedPathWithoutReplacement {
    Path(String),
    Table(DisallowedPathTableWithoutReplacement),
}

#[derive(JsonSchema)]
#[schemars(deny_unknown_fields, transform = require_path)]
pub(crate) struct DisallowedPathTable {
    path: String,
    reason: String,
    replacement: String,
    #[schemars(rename = "allow-invalid")]
    allow_invalid: bool,
}

#[derive(JsonSchema)]
#[schemars(deny_unknown_fields, transform = require_path)]
pub(crate) struct DisallowedPathTableWithoutReplacement {
    path: String,
    reason: String,
    #[schemars(rename = "allow-invalid")]
    allow_invalid: bool,
}

fn require_path(schema: &mut Schema) {
    schema
        .as_object_mut()
        .unwrap()
        .insert("required".into(), json!(["path"]));
}

#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
pub(crate) struct Rename {
    path: String,
    rename: String,
}

#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
pub(crate) struct MacroMatcher {
    name: String,
    #[schemars(schema_with = "brace_schema")]
    brace: String,
}

fn brace_schema(_: &mut schemars::SchemaGenerator) -> Schema {
    schemars::json_schema!({ "type": "string", "enum": ["{", "{}", "[", "[]", "(", "()"] })
}

#[derive(JsonSchema)]
#[schemars(untagged)]
pub(crate) enum WithinGroupings {
    Selection(GroupingSelection),
    Groups(Vec<String>),
}

#[derive(JsonSchema)]
#[schemars(rename_all = "lowercase")]
pub(crate) enum GroupingSelection {
    All,
    None,
}
