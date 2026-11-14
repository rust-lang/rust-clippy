use crate::ConfMetadata;
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::json;

const JSON_SCHEMA_ID: &str = "https://clippy.rust-lang.org/main/clippy.schema.json";

pub(crate) fn schema_json<T: JsonSchema>(metadata: Vec<ConfMetadata>) -> String {
    let mut schema = SchemaSettings::draft07().into_generator().into_root_schema_for::<T>();
    let root = schema.as_object_mut().unwrap();
    root.insert("$id".into(), JSON_SCHEMA_ID.into());
    root.remove("required");
    let properties = root.get_mut("properties").unwrap().as_object_mut().unwrap();
    properties.insert(
        "third-party".into(),
        json!({ "description": "Third-party configuration. Clippy ignores the value of this setting." }),
    );
    for entry in &metadata {
        if entry.renamed_to.is_none() {
            let property = properties.get_mut(entry.name).unwrap();
            if property.as_object().unwrap().contains_key("$ref") {
                let referenced_schema = std::mem::take(property);
                *property = json!({ "allOf": [referenced_schema] });
            }
            property
                .as_object_mut()
                .unwrap()
                .insert("description".into(), crate::sanitize_explanation(entry.doc).into());
        }
    }
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
#[schemars(transparent, inline, extend("uniqueItems" = true))]
#[allow(dead_code)]
pub(crate) struct UniqueArray<T>(Vec<T>);
