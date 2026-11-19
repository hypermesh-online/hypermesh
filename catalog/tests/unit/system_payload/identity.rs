use catalog::registry::system_payload::{definition, ValidatedSystemPayload};
use hypermesh_lib::{
    AssetAddress, AssetId, ContentHash, IdentityAssetRecord, SystemAssetKind::Identity,
};
use serde_json::{json, Value};

fn record() -> IdentityAssetRecord {
    let hash = ContentHash::zeroed();
    IdentityAssetRecord {
        asset_id: AssetId("genesis-fixture".into()),
        kind: Identity,
        address: AssetAddress::new(0, 0, 0, &hash).unwrap(),
        falcon_public_key: vec![0, 255],
        kyber_public_key: vec![],
        genesis_block_hash: hash,
        registered_at: u64::MAX,
    }
}

fn assert_serde(value: &Value, expected: bool) {
    let canonical = serde_json::from_value::<IdentityAssetRecord>(value.clone());
    let actual = definition(Identity).unwrap().validate(value);
    assert_eq!(canonical.is_ok(), expected, "{value}");
    assert_eq!(actual.is_ok(), expected, "{value}");
    if let Ok(canonical) = canonical {
        assert_eq!(actual.unwrap(), ValidatedSystemPayload::Identity(canonical));
    }
}

#[test]
fn serde_shape_not_a_genesis_or_key_attestation() {
    let mut value = serde_json::to_value(record()).unwrap();
    assert_serde(&value, true);
    value["kind"] = json!("Message");
    assert_serde(&value, true);
    value["kind"] = json!("identity");
    assert_serde(&value, false);

    let def = definition(Identity).unwrap();
    let fields = &def.json_schema()["properties"];
    assert_eq!(fields["address"]["minItems"], 16);
    assert_eq!(fields["address"]["maxItems"], 16);
    assert_eq!(fields["genesis_block_hash"]["minItems"], 32);
    assert_eq!(fields["genesis_block_hash"]["maxItems"], 32);
    assert_eq!(fields["falcon_public_key"]["items"]["maximum"], 255);
    assert!(fields["kyber_public_key"]["maxItems"].is_null());
    assert_eq!(fields["registered_at"]["maximum"], u64::MAX);
}

#[test]
fn positional_input_and_fixed_length_fields_follow_canonical_serde() {
    let value = serde_json::to_value(record()).unwrap();
    let mut seq = json!([
        value["asset_id"],
        value["kind"],
        value["address"],
        value["falcon_public_key"],
        value["kyber_public_key"],
        value["genesis_block_hash"],
        value["registered_at"]
    ]);
    assert_serde(&seq, true);
    seq[3] = json!("raw strings are not plain Vec<u8>");
    assert_serde(&seq, false);
    seq[3] = json!([]);
    seq[2] = json!(vec![0; 15]);
    assert_serde(&seq, false);
    seq[2] = value["address"].clone();
    seq[5] = json!(vec![0; 33]);
    assert_serde(&seq, false);
}
