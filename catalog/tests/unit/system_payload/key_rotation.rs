use catalog::registry::system_payload::{definition, ValidatedSystemPayload};
use hypermesh_lib::SystemAssetKind::KeyRotation;
use serde_json::{json, Value};
use trustchain::identity::{KeyRotationEntry, KeyRotationReason};

fn rotation() -> KeyRotationEntry {
    KeyRotationEntry {
        old_pubkey_hash: [0; 32],
        new_pubkey: vec![0, 255],
        new_kyber_pubkey: vec![],
        rotation_signature: vec![99],
        block_index: u64::MAX,
        reason: KeyRotationReason::Scheduled,
    }
}

fn assert_serde(value: &Value, expected: bool) {
    let canonical = serde_json::from_value::<KeyRotationEntry>(value.clone());
    let actual = definition(KeyRotation).unwrap().validate(value);
    assert_eq!(canonical.is_ok(), expected, "{value}");
    assert_eq!(actual.is_ok(), expected, "{value}");
    if let Ok(canonical) = canonical {
        assert_eq!(
            actual.unwrap(),
            ValidatedSystemPayload::KeyRotation(canonical)
        );
    }
}

#[test]
fn rotation_event_uses_canonical_enum_and_byte_shapes_not_key_verification() {
    let mut value = serde_json::to_value(rotation()).unwrap();
    let reasons = ["Scheduled", "Compromise", "Upgrade", "Recovery"];
    for reason in reasons {
        value["reason"] = json!(reason);
        assert_serde(&value, true);
    }
    value["reason"] = json!("scheduled");
    assert_serde(&value, false);

    let def = definition(KeyRotation).unwrap();
    let fields = &def.json_schema()["properties"];
    assert_eq!(fields["reason"]["enum"], json!(reasons));
    assert_eq!(fields["old_pubkey_hash"]["minItems"], 32);
    assert_eq!(fields["old_pubkey_hash"]["maxItems"], 32);
    assert_eq!(fields["block_index"]["maximum"], u64::MAX);
    assert!(fields["new_pubkey"]["maxItems"].is_null());
}

#[test]
fn positional_inputs_and_plain_bytes_follow_canonical_serde() {
    let value = serde_json::to_value(rotation()).unwrap();
    let mut seq = json!([
        value["old_pubkey_hash"],
        value["new_pubkey"],
        value["new_kyber_pubkey"],
        value["rotation_signature"],
        value["block_index"],
        value["reason"]
    ]);
    assert_serde(&seq, true);
    seq[3] = json!("not a byte-array");
    assert_serde(&seq, false);
    seq[3] = json!([256]);
    assert_serde(&seq, false);
    seq[3] = json!([]);
    seq[0] = json!(vec![0; 31]);
    assert_serde(&seq, false);
}
