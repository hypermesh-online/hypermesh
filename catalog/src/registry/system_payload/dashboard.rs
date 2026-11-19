use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use super::schema;

pub(super) fn schema() -> Value {
    let text = json!({"type": "string"});
    let optional_text = json!({"type": ["string", "null"]});
    let fields = vec![
        (
            "dashboard",
            json!({
                "type": "object",
                "properties": {
                    "name": text, "version": text, "description": text, "domain": text
                },
                "required": ["name", "version", "description", "domain"]
            }),
        ),
        (
            "access",
            json!({
                "type": "object",
                "properties": {"public": optional_text, "private": optional_text}
            }),
        ),
        (
            "dependencies",
            json!({"type": "object", "additionalProperties": text, "default": {}}),
        ),
    ];
    let mut schema = schema::object(SystemAssetKind::Dashboard, fields, &["dependencies"]);
    schema["description"] = json!(
        "JSON object projection of the BlockMatrix dashboard.toml manifest. Does not validate the referenced directories, access policy, or deployed bundle."
    );
    schema
}
