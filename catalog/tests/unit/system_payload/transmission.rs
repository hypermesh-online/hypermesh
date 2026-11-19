use catalog::registry::system_payload::{
    definition, SystemPayloadError, TransmissionPayload, ValidatedSystemPayload,
};
use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use crate::fixtures::schema_accepts;

fn assert_parity(value: &Value, expected: bool) {
    let def = definition(SystemAssetKind::Transmission).unwrap();
    assert_eq!(
        serde_json::from_value::<TransmissionPayload>(value.clone()).is_ok(),
        expected
    );
    assert_eq!(def.validate(value).is_ok(), expected, "{value}");
    assert_eq!(
        schema_accepts(def.json_schema(), value),
        expected,
        "{value}"
    );
}

#[test]
fn declared_capacity_allows_zero_nonzero_and_exact_u64_boundaries() {
    for capacity in [0, 1, 1_000_000, u64::MAX - 1, u64::MAX] {
        let typed = TransmissionPayload {
            bandwidth_bps: capacity,
        };
        let value = serde_json::to_value(&typed).unwrap();
        assert_eq!(value, json!({"bandwidth_bps": capacity}));
        assert_parity(&value, true);
        let parsed = definition(SystemAssetKind::Transmission)
            .unwrap()
            .validate(&value)
            .unwrap();
        assert_eq!(parsed.kind(), SystemAssetKind::Transmission);
        assert_eq!(parsed, ValidatedSystemPayload::Transmission(typed));
    }
}

#[test]
fn missing_malformed_negative_wrong_type_and_overflow_fail() {
    for value in [
        Value::Null,
        json!(true),
        json!(0),
        json!("resource"),
        json!({}),
        json!([]),
    ] {
        assert_parity(&value, false);
    }
    for invalid in [
        Value::Null,
        json!(-1),
        json!(1.5),
        json!("1"),
        json!(true),
        json!({}),
        json!([]),
    ] {
        assert_parity(&json!({"bandwidth_bps": invalid}), false);
    }
    let overflow: Value = serde_json::from_str("18446744073709551616").unwrap();
    let def = definition(SystemAssetKind::Transmission).unwrap();
    let value = json!({"bandwidth_bps": overflow});
    assert_parity(&value, false);
    assert!(matches!(
        def.validate(&value),
        Err(SystemPayloadError::InvalidPayload {
            kind: SystemAssetKind::Transmission,
            ..
        })
    ));
}

#[test]
fn object_schema_is_exact_and_has_no_invented_nonzero_policy() {
    let def = definition(SystemAssetKind::Transmission).unwrap();
    let schema = def.json_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["bandwidth_bps"]));
    assert_eq!(schema["additionalProperties"], true);
    assert_eq!(schema["properties"].as_object().unwrap().len(), 1);
    let field = &schema["properties"]["bandwidth_bps"];
    assert_eq!(field["type"], "integer");
    assert_eq!(field["minimum"], 0);
    assert_eq!(field["maximum"].as_u64(), Some(u64::MAX));
    assert!(field["description"]
        .as_str()
        .unwrap()
        .contains("bits per second"));
    assert!(field["description"]
        .as_str()
        .unwrap()
        .contains("zero means no declared capacity"));
    assert!(!schema["title"].as_str().unwrap().contains("canonical"));
}

#[test]
fn unknown_object_fields_are_ignored_without_creating_authority() {
    let value = json!({"bandwidth_bps":0,"unknown":{"ignored":true}});
    assert_parity(&value, true);
    let parsed = definition(SystemAssetKind::Transmission)
        .unwrap()
        .validate(&value)
        .unwrap();
    assert_eq!(
        parsed,
        ValidatedSystemPayload::Transmission(TransmissionPayload { bandwidth_bps: 0 })
    );
}

#[test]
fn positional_serde_input_is_preserved_but_is_not_an_object_schema_input() {
    let def = definition(SystemAssetKind::Transmission).unwrap();
    for capacity in [0, 1, u64::MAX] {
        let sequence = json!([capacity]);
        assert!(serde_json::from_value::<TransmissionPayload>(sequence.clone()).is_ok());
        assert_eq!(
            def.validate(&sequence).unwrap(),
            def.validate(&json!({"bandwidth_bps":capacity})).unwrap()
        );
        assert!(!schema_accepts(def.json_schema(), &sequence));
    }
    for malformed in [
        json!([]),
        json!([0, 1]),
        json!([-1]),
        json!(["1"]),
        json!([null]),
    ] {
        assert_parity(&malformed, false);
    }
}

#[test]
fn schema_mathematical_integer_does_not_override_typed_float_rejection() {
    let value = json!({"bandwidth_bps":1.0});
    let def = definition(SystemAssetKind::Transmission).unwrap();
    assert!(schema_accepts(def.json_schema(), &value));
    assert!(serde_json::from_value::<TransmissionPayload>(value.clone()).is_err());
    assert!(def.validate(&value).is_err());
}
