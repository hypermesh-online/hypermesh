use hypermesh_lib::{IdentityAssetRecord, SystemAssetKind};
use serde_json::{json, Value};

use super::schema;

pub(super) fn schema() -> Value {
    let fields = vec![
        ("asset_id", json!({"type": "string"})),
        (
            "kind",
            json!({
                "type": "string",
                "enum": [
                    "Cpu", "Gpu", "Memory", "Storage", "Network", "Container", "Economic",
                    "Blockchain", "Dns", "Transmission", "Dashboard", "Identity", "KeyRotation",
                    "Invitation", "Message"
                ],
                "description": "Genesis emits Identity; canonical serde accepts any system kind."
            }),
        ),
        ("address", schema::fixed_bytes(16)),
        ("falcon_public_key", schema::bytes()),
        ("kyber_public_key", schema::bytes()),
        ("genesis_block_hash", schema::fixed_bytes(32)),
        (
            "registered_at",
            schema::unsigned(u64::MAX, "UTC milliseconds since Unix epoch"),
        ),
    ];
    schema::object(SystemAssetKind::Identity, fields, &[])
}

pub(super) fn equal(a: &IdentityAssetRecord, b: &IdentityAssetRecord) -> bool {
    a.asset_id == b.asset_id
        && a.kind == b.kind
        && a.address == b.address
        && a.falcon_public_key == b.falcon_public_key
        && a.kyber_public_key == b.kyber_public_key
        && a.genesis_block_hash == b.genesis_block_hash
        && a.registered_at == b.registered_at
}
