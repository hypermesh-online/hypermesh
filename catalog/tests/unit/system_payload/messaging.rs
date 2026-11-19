use blockmatrix::{messaging::message::DirectMessage, sharing::invite::ShareInvite};
use catalog::registry::system_payload::{definition, SystemPayloadError, ValidatedSystemPayload};
use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use crate::fixtures::{messaging_fields, messaging_payloads, schema_accepts};

fn canonical_accepts(kind: SystemAssetKind, value: &Value) -> bool {
    match kind {
        SystemAssetKind::Invitation => serde_json::from_value::<ShareInvite>(value.clone()).is_ok(),
        SystemAssetKind::Message => serde_json::from_value::<DirectMessage>(value.clone()).is_ok(),
        _ => false,
    }
}

fn assert_parity(kind: SystemAssetKind, value: &Value, expected: bool) {
    let def = definition(kind).unwrap();
    assert_eq!(canonical_accepts(kind, value), expected, "{kind}: {value}");
    assert_eq!(def.validate(value).is_ok(), expected, "{kind}: {value}");
    assert_eq!(
        schema_accepts(def.json_schema(), value),
        expected,
        "{kind}: {value}"
    );
}

#[test]
fn canonical_objects_serialize_bytes_and_bind_the_system_kind() {
    for (kind, value) in messaging_payloads() {
        let def = definition(kind).unwrap();
        assert_eq!(def.kind(), kind);
        assert_eq!(def.validate(&value).unwrap().kind(), kind);
        assert_parity(kind, &value, true);
        assert_eq!(
            def.json_schema()["properties"]["created_at"]["description"],
            "Unix timestamp in seconds"
        );
        check_serialized_bytes(kind, &value);
    }
}

fn check_serialized_bytes(kind: SystemAssetKind, value: &Value) {
    let def = definition(kind).unwrap();
    for (key, field) in def.json_schema()["properties"].as_object().unwrap() {
        if field.get("anyOf").is_some() {
            assert!(value[key].is_array());
        }
    }
}

#[test]
fn malformed_roots_missing_fields_and_wrong_field_types() {
    for (kind, value) in messaging_payloads() {
        for malformed in [Value::Null, json!([]), json!(true), json!("message")] {
            assert_parity(kind, &malformed, false);
        }
        check_required_fields(kind, &value);
        check_field_types(kind, &value);
    }
}

fn check_required_fields(kind: SystemAssetKind, value: &Value) {
    let def = definition(kind).unwrap();
    for key in def.json_schema()["required"].as_array().unwrap() {
        let mut missing = value.clone();
        missing
            .as_object_mut()
            .unwrap()
            .remove(key.as_str().unwrap());
        assert_parity(kind, &missing, false);
    }
}

fn check_field_types(kind: SystemAssetKind, value: &Value) {
    for key in messaging_fields(kind) {
        let mut malformed = value.clone();
        malformed[*key] = json!({});
        assert_parity(kind, &malformed, false);
    }
}

#[test]
fn byte_arrays_have_exact_u8_bounds_and_utf8_strings_are_not_decoded() {
    for (kind, value) in messaging_payloads() {
        check_byte_fields(kind, &value);
    }
}

fn check_byte_fields(kind: SystemAssetKind, value: &Value) {
    let def = definition(kind).unwrap();
    for (key, field) in def.json_schema()["properties"].as_object().unwrap() {
        if field.get("anyOf").is_some() {
            check_byte_field(kind, value, key);
        }
    }
}

fn check_byte_field(kind: SystemAssetKind, value: &Value, key: &str) {
    for replacement in [json!([]), json!([0, 255]), json!("é"), json!("ff")] {
        let mut valid = value.clone();
        valid[key] = replacement;
        assert_parity(kind, &valid, true);
    }
    for replacement in [
        json!([-1]),
        json!([256]),
        json!([1.5]),
        json!(["0"]),
        Value::Null,
    ] {
        let mut invalid = value.clone();
        invalid[key] = replacement;
        assert_parity(kind, &invalid, false);
    }
    let mut raw = value.clone();
    raw["signature"] = json!("ff");
    let typed = definition(kind).unwrap().validate(&raw).unwrap();
    assert_eq!(signature(&typed), b"ff");
}

fn signature(value: &ValidatedSystemPayload) -> &[u8] {
    match value {
        ValidatedSystemPayload::Invitation(invite) => &invite.signature,
        ValidatedSystemPayload::Message(message) => &message.signature,
        _ => panic!("messaging fixture"),
    }
}

#[test]
fn optional_fields_are_omittable_nullable_or_strings() {
    for (kind, value) in messaging_payloads() {
        for key in optional_fields(kind) {
            check_optional_field(kind, &value, key);
        }
    }
}

fn optional_fields(kind: SystemAssetKind) -> &'static [&'static str] {
    if kind == SystemAssetKind::Invitation {
        &["sender_name"]
    } else {
        &["sender_name", "reply_to"]
    }
}

fn check_optional_field(kind: SystemAssetKind, value: &Value, key: &str) {
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove(key);
    assert_parity(kind, &missing, true);
    for replacement in [Value::Null, json!(""), json!("fixture")] {
        let mut valid = value.clone();
        valid[key] = replacement;
        assert_parity(kind, &valid, true);
    }
    let mut invalid = value.clone();
    invalid[key] = json!(3);
    assert_parity(kind, &invalid, false);
}

#[test]
fn timestamp_and_unsigned_numeric_boundaries_are_canonical() {
    for (kind, value) in messaging_payloads() {
        for timestamp in [i64::MIN, -1, 0, i64::MAX] {
            let mut valid = value.clone();
            valid["created_at"] = json!(timestamp);
            assert_parity(kind, &valid, true);
        }
        check_timestamp_overflow(kind, &value);
        check_unsigned_fields(kind, &value);
    }
}

fn check_timestamp_overflow(kind: SystemAssetKind, value: &Value) {
    for replacement in [json!(i64::MAX as u64 + 1), json!("0"), json!(0.5)] {
        let mut invalid = value.clone();
        invalid["created_at"] = replacement;
        assert_parity(kind, &invalid, false);
    }
}

fn check_unsigned_fields(kind: SystemAssetKind, value: &Value) {
    if kind != SystemAssetKind::Invitation {
        return;
    }
    for (key, maximum) in [("asset_size", u64::MAX), ("shard_count", u32::MAX as u64)] {
        check_unsigned_field(kind, value, key, maximum);
    }
}

fn check_unsigned_field(kind: SystemAssetKind, value: &Value, key: &str, maximum: u64) {
    for n in [0, maximum] {
        let mut valid = value.clone();
        valid[key] = json!(n);
        assert_parity(kind, &valid, true);
    }
    let mut invalid = value.clone();
    invalid[key] = json!(-1);
    assert_parity(kind, &invalid, false);
    invalid[key] = if maximum == u64::MAX {
        serde_json::from_str("18446744073709551616").unwrap()
    } else {
        json!(maximum + 1)
    };
    assert_parity(kind, &invalid, false);
}

#[test]
fn sequences_preserve_canonical_order_and_optional_slots() {
    for (kind, object) in messaging_payloads() {
        let values: Vec<_> = messaging_fields(kind)
            .iter()
            .map(|key| object[*key].clone())
            .collect();
        let sequence = Value::Array(values);
        let def = definition(kind).unwrap();
        assert!(canonical_accepts(kind, &sequence));
        assert_eq!(
            def.validate(&sequence).unwrap(),
            def.validate(&object).unwrap()
        );
        assert!(!schema_accepts(def.json_schema(), &sequence));
        check_sequence_errors(kind, &sequence);
    }
}

fn check_sequence_errors(kind: SystemAssetKind, sequence: &Value) {
    let mut short = sequence.as_array().unwrap().clone();
    short.pop();
    assert_parity(kind, &Value::Array(short), false);
    let mut long = sequence.as_array().unwrap().clone();
    long.push(Value::Null);
    assert_parity(kind, &Value::Array(long), false);
    for index in 0..sequence.as_array().unwrap().len() {
        let mut invalid = sequence.clone();
        invalid[index] = json!({});
        assert_parity(kind, &invalid, false);
    }
}

#[test]
fn structural_success_does_not_require_valid_signature_or_uuid_or_timestamp() {
    for (kind, mut value) in messaging_payloads() {
        value["unknown"] = json!("ignored");
        value["signature"] = json!([]);
        value["created_at"] = json!(-1);
        assert_parity(kind, &value, true);
        let parsed = definition(kind).unwrap().validate(&value).unwrap();
        assert!(signature(&parsed).is_empty());
        assert_eq!(parsed, parsed.clone());
        check_equality_field_coverage(kind, &value);
    }
}

fn check_equality_field_coverage(kind: SystemAssetKind, value: &Value) {
    let def = definition(kind).unwrap();
    let original = def.validate(value).unwrap();
    for key in messaging_fields(kind) {
        let mut changed = value.clone();
        changed[*key] = distinct_value(&changed[*key]);
        assert_ne!(original, def.validate(&changed).unwrap(), "field {key}");
    }
}

fn distinct_value(value: &Value) -> Value {
    if value.is_array() {
        return json!([255]);
    }
    if value.is_number() {
        return json!(value.as_i64().unwrap() + 1);
    }
    json!("changed")
}

#[test]
fn integer_representation_limit_is_also_explicit_for_signed_seconds_and_bytes() {
    for (kind, mut value) in messaging_payloads() {
        value["created_at"] = json!(1.0);
        let def = definition(kind).unwrap();
        assert!(schema_accepts(def.json_schema(), &value));
        assert!(!canonical_accepts(kind, &value));
        assert!(
            matches!(def.validate(&value), Err(SystemPayloadError::InvalidPayload {kind: actual, ..}) if actual == kind)
        );
        value["created_at"] = json!(1);
        value["signature"] = json!([1.0]);
        assert!(schema_accepts(def.json_schema(), &value));
        assert!(def.validate(&value).is_err());
    }
}
