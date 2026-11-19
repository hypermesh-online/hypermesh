use catalog::registry::system_payload::{definition, SystemPayloadError, ValidatedSystemPayload};
use hypermesh_lib::{StorageType, SystemAssetKind};
use serde_json::{json, Value};

use crate::fixtures::{canonical_accepts, mutations, payloads, schema_accepts};

#[test]
fn canonical_serialized_fixtures_match_definitions() {
    for (kind, value) in payloads() {
        let def = definition(kind).expect("first five defined");
        assert_eq!(def.kind(), kind);
        assert_eq!(
            def.validate(&value).expect("canonical payload").kind(),
            kind
        );
        assert!(canonical_accepts(kind, value.clone()));
        assert!(schema_accepts(def.json_schema(), &value));
        assert_eq!(
            def.json_schema()["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(def.json_schema()["additionalProperties"], true);
    }
}

#[test]
fn malformed_missing_and_wrong_types_match_canonical_serde() {
    for (kind, original) in payloads() {
        let def = definition(kind).unwrap();
        for malformed in [Value::Null, json!([]), json!("payload"), json!(1)] {
            assert!(!schema_accepts(def.json_schema(), &malformed));
            assert!(def.validate(&malformed).is_err());
        }
        check_fields(kind, &original);
    }
}

fn check_fields(kind: SystemAssetKind, original: &Value) {
    let def = definition(kind).unwrap();
    for (key, field) in def.json_schema()["properties"].as_object().unwrap() {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(!schema_accepts(def.json_schema(), &missing));
        assert!(def.validate(&missing).is_err());
        for mutation in mutations(field) {
            let mut value = original.clone();
            value[key] = mutation;
            assert_parity(kind, &value, false);
        }
    }
}

fn assert_parity(kind: SystemAssetKind, value: &Value, expected: bool) {
    let def = definition(kind).unwrap();
    assert_eq!(
        canonical_accepts(kind, value.clone()),
        expected,
        "{kind}: {value}"
    );
    assert_eq!(def.validate(value).is_ok(), expected, "{kind}: {value}");
    assert_eq!(
        schema_accepts(def.json_schema(), value),
        expected,
        "{kind}: {value}"
    );
}

#[test]
fn unsigned_boundaries_are_exact_and_zero_is_structurally_valid() {
    for (kind, original) in payloads() {
        check_unsigned_fields(kind, &original);
    }
}

fn check_unsigned_fields(kind: SystemAssetKind, original: &Value) {
    let def = definition(kind).unwrap();
    for (key, field) in def.json_schema()["properties"].as_object().unwrap() {
        if field["type"] != "integer" {
            continue;
        }
        let max = field["maximum"].as_u64().unwrap();
        check_unsigned_values(kind, original, key, max);
        check_overflow(kind, original, key, max);
    }
}

fn check_unsigned_values(kind: SystemAssetKind, original: &Value, key: &str, max: u64) {
    for number in [0, max - 1, max] {
        let mut value = original.clone();
        value[key] = json!(number);
        assert_parity(kind, &value, true);
    }
}

fn check_overflow(kind: SystemAssetKind, original: &Value, key: &str, max: u64) {
    let mut value = original.clone();
    value[key] = if max == u64::MAX {
        serde_json::from_str("18446744073709551616").unwrap()
    } else {
        json!(max + 1)
    };
    assert_parity(kind, &value, false);
}

#[test]
fn storage_enum_uses_serde_names_not_display_names() {
    let mut value = payloads()
        .into_iter()
        .find(|(k, _)| *k == SystemAssetKind::Storage)
        .unwrap()
        .1;
    for medium in [
        StorageType::Hdd,
        StorageType::Ssd,
        StorageType::Nvme,
        StorageType::Unknown,
    ] {
        value["storage_type"] = serde_json::to_value(medium).unwrap();
        assert_parity(SystemAssetKind::Storage, &value, true);
    }
    for invalid in ["HDD", "SSD", "NVMe", "hdd", "", "Other"] {
        value["storage_type"] = json!(invalid);
        assert_parity(SystemAssetKind::Storage, &value, false);
    }
}

#[test]
fn unknown_fields_and_no_invented_physical_constraints() {
    for (kind, mut value) in payloads() {
        value["unknown"] = json!({"ignored": true});
        assert_parity(kind, &value, true);
    }
    assert_parity(
        SystemAssetKind::Cpu,
        &json!({"core_count":0,"clock_mhz":0,"architecture":""}),
        true,
    );
    assert_parity(
        SystemAssetKind::Memory,
        &json!({"total_bytes":0,"available_bytes":u64::MAX}),
        true,
    );
    let def = definition(SystemAssetKind::Network).unwrap();
    let value = json!({"bandwidth_bps":0,"latency_us":u64::MAX,"ipv6_native":false});
    assert_parity(SystemAssetKind::Network, &value, true);
    assert!(matches!(
        def.validate(&value).unwrap(),
        ValidatedSystemPayload::Network(_)
    ));
}

#[test]
fn mathematical_integer_schema_is_not_typed_representation_validation() {
    let def = definition(SystemAssetKind::Cpu).unwrap();
    let value = json!({"core_count":1.0,"clock_mhz":0,"architecture":""});
    assert!(schema_accepts(def.json_schema(), &value));
    assert!(!canonical_accepts(SystemAssetKind::Cpu, value.clone()));
    assert!(matches!(
        def.validate(&value),
        Err(SystemPayloadError::InvalidPayload {
            kind: SystemAssetKind::Cpu,
            ..
        })
    ));
}

#[test]
fn remaining_seven_are_explicitly_not_yet_defined() {
    use SystemAssetKind::*;
    for kind in [
        Container,
        Economic,
        Blockchain,
        Dns,
        Dashboard,
        Identity,
        KeyRotation,
    ] {
        assert!(
            matches!(definition(kind), Err(SystemPayloadError::NotYetDefined { kind: actual }) if actual == kind)
        );
    }
}

#[test]
fn schema_units_are_canonical_not_inferred_from_bps_suffix() {
    let storage = definition(SystemAssetKind::Storage).unwrap();
    let network = definition(SystemAssetKind::Network).unwrap();
    assert_eq!(
        storage.json_schema()["properties"]["read_speed_bps"]["description"],
        "bytes/sec"
    );
    assert_eq!(
        network.json_schema()["properties"]["bandwidth_bps"]["description"],
        "bits per second"
    );
    assert_eq!(
        network.json_schema()["properties"]["latency_us"]["description"],
        "microseconds"
    );
}

#[test]
fn canonical_positional_sequences_are_preserved_not_object_schema_validated() {
    for (kind, value) in crate::fixtures::positional_payloads() {
        let def = definition(kind).unwrap();
        assert!(canonical_accepts(kind, value.clone()));
        assert_eq!(def.validate(&value).unwrap().kind(), kind);
        assert!(!schema_accepts(def.json_schema(), &value));
    }
}

#[test]
fn malformed_positional_sequences_fail_for_all_five_kinds() {
    for (kind, value) in crate::fixtures::positional_payloads() {
        check_sequence_lengths(kind, &value);
        check_sequence_fields(kind, &value);
    }
}

fn check_sequence_lengths(kind: SystemAssetKind, value: &Value) {
    let mut short = value.as_array().unwrap().clone();
    short.pop();
    assert_parity(kind, &Value::Array(short), false);
    let mut long = value.as_array().unwrap().clone();
    long.push(Value::Null);
    assert_parity(kind, &Value::Array(long), false);
}

fn check_sequence_fields(kind: SystemAssetKind, value: &Value) {
    for index in 0..value.as_array().unwrap().len() {
        let mut malformed = value.clone();
        malformed[index] = Value::Null;
        assert_parity(kind, &malformed, false);
    }
}
