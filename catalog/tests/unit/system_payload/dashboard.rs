use blockmatrix::dashboard::{DashboardAccess, DashboardManifest, DashboardMeta};
use catalog::registry::system_payload::{definition, ValidatedSystemPayload};
use hypermesh_lib::SystemAssetKind::Dashboard;
use serde_json::{json, Value};

fn manifest() -> DashboardManifest {
    DashboardManifest {
        dashboard: DashboardMeta {
            name: "example".into(),
            version: "v1".into(),
            description: "fixture".into(),
            domain: "node".into(),
        },
        access: DashboardAccess {
            public: None,
            private: None,
        },
        dependencies: [("theme".into(), "v2".into())].into(),
    }
}

fn assert_serde(value: &Value, expected: bool) {
    let canonical = serde_json::from_value::<DashboardManifest>(value.clone());
    let actual = definition(Dashboard).unwrap().validate(value);
    assert_eq!(canonical.is_ok(), expected, "{value}");
    assert_eq!(actual.is_ok(), expected, "{value}");
    if let Ok(canonical) = canonical {
        assert_eq!(
            actual.unwrap(),
            ValidatedSystemPayload::Dashboard(canonical)
        );
    }
}

#[test]
fn canonical_manifest_projection_preserves_defaults_and_optional_access() {
    let mut value = serde_json::to_value(manifest()).unwrap();
    assert_serde(&value, true);
    value.as_object_mut().unwrap().remove("dependencies");
    value["access"] = json!({"admin": "ignored by the canonical node manifest"});
    assert_serde(&value, true);

    let def = definition(Dashboard).unwrap();
    let ValidatedSystemPayload::Dashboard(decoded) = def.validate(&value).unwrap() else {
        panic!("wrong typed variant");
    };
    assert!(decoded.dependencies.is_empty());
    assert!(decoded.access.public.is_none() && decoded.access.private.is_none());
    let schema = def.json_schema();
    assert_eq!(schema["required"], json!(["dashboard", "access"]));
    assert_eq!(schema["properties"]["dependencies"]["default"], json!({}));
    assert_eq!(
        schema["properties"]["dependencies"]["additionalProperties"]["type"],
        "string"
    );
    assert_eq!(
        schema["properties"]["dashboard"]["required"],
        json!(["name", "version", "description", "domain"])
    );
}

#[test]
fn nested_positional_serde_is_distinct_from_the_object_projection() {
    assert_serde(
        &json!([["example", "v1", "fixture", "node"], [null, "internal"], {}]),
        true,
    );
    assert_serde(
        &json!([["example", "v1", "fixture"], [null, "internal"], {}]),
        false,
    );
    let mut value = serde_json::to_value(manifest()).unwrap();
    value["dependencies"]["theme"] = json!(2);
    assert_serde(&value, false);
    value["dependencies"] = json!({});
    value["access"]["public"] = json!([]);
    assert_serde(&value, false);
}
