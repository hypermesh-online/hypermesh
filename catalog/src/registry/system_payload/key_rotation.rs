use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};
use trustchain::identity::KeyRotationEntry;

use super::schema;

pub(super) fn schema() -> Value {
    let fields = vec![
        ("old_pubkey_hash", schema::fixed_bytes(32)),
        ("new_pubkey", schema::bytes()),
        ("new_kyber_pubkey", schema::bytes()),
        ("rotation_signature", schema::bytes()),
        ("block_index", schema::unsigned(u64::MAX, "block index")),
        (
            "reason",
            json!({"type": "string", "enum": ["Scheduled", "Compromise", "Upgrade", "Recovery"]}),
        ),
    ];
    let mut schema = schema::object(SystemAssetKind::KeyRotation, fields, &[]);
    schema["description"] = json!(
        "Structural rotation event for an existing identity lineage; not a second lineage, signature check, or authorization."
    );
    schema
}

pub(super) fn equal(a: &KeyRotationEntry, b: &KeyRotationEntry) -> bool {
    a.old_pubkey_hash == b.old_pubkey_hash
        && a.new_pubkey == b.new_pubkey
        && a.new_kyber_pubkey == b.new_kyber_pubkey
        && a.rotation_signature == b.rotation_signature
        && a.block_index == b.block_index
        && a.reason == b.reason
}
