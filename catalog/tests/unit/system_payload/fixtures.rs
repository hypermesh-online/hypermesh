use hypermesh_lib::{
    CpuCapabilities, GpuCapabilities, MemoryCapabilities, NetworkCapabilities, StorageCapabilities,
    StorageType, SystemAssetKind,
};
use serde_json::{json, Value};

pub fn payloads() -> Vec<(SystemAssetKind, Value)> {
    vec![
        (
            SystemAssetKind::Cpu,
            encode(CpuCapabilities {
                core_count: 4,
                clock_mhz: 3200,
                architecture: "x86_64".into(),
            }),
        ),
        (
            SystemAssetKind::Gpu,
            encode(GpuCapabilities {
                model: "fixture".into(),
                vram_bytes: 8_000_000_000,
                compute_units: 32,
            }),
        ),
        (
            SystemAssetKind::Memory,
            encode(MemoryCapabilities {
                total_bytes: 8_000_000_000,
                available_bytes: 4_000_000_000,
            }),
        ),
        (
            SystemAssetKind::Storage,
            encode(StorageCapabilities {
                total_bytes: 100_000_000_000,
                storage_type: StorageType::Nvme,
                read_speed_bps: 1_000_000_000,
                write_speed_bps: 500_000_000,
            }),
        ),
        (
            SystemAssetKind::Network,
            encode(NetworkCapabilities {
                bandwidth_bps: 10_000_000,
                latency_us: 1000,
                ipv6_native: true,
            }),
        ),
    ]
}

fn encode(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("canonical fixture serializes")
}

pub fn canonical_accepts(kind: SystemAssetKind, value: Value) -> bool {
    match kind {
        SystemAssetKind::Cpu => serde_json::from_value::<CpuCapabilities>(value).is_ok(),
        SystemAssetKind::Gpu => serde_json::from_value::<GpuCapabilities>(value).is_ok(),
        SystemAssetKind::Memory => serde_json::from_value::<MemoryCapabilities>(value).is_ok(),
        SystemAssetKind::Storage => serde_json::from_value::<StorageCapabilities>(value).is_ok(),
        SystemAssetKind::Network => serde_json::from_value::<NetworkCapabilities>(value).is_ok(),
        _ => false,
    }
}

// Test evaluator for this component's emitted keyword subset, not a schema engine.
// Numeric comparison stays exact for u64 rather than coercing through f64.
pub fn schema_accepts(schema: &Value, value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let required = schema["required"].as_array().expect("required array");
    if !required
        .iter()
        .all(|key| object.contains_key(key.as_str().expect("key")))
    {
        return false;
    }
    schema["properties"]
        .as_object()
        .expect("properties")
        .iter()
        .all(|(key, field)| {
            object
                .get(key)
                .is_none_or(|value| field_accepts(field, value))
        })
}

fn field_accepts(field: &Value, value: &Value) -> bool {
    if let Some(branches) = field.get("anyOf") {
        return branches
            .as_array()
            .unwrap()
            .iter()
            .any(|branch| field_accepts(branch, value));
    }
    if field["type"].is_array() {
        return value.is_null() || value.is_string();
    }
    match field["type"].as_str().expect("field type") {
        "integer" => integer_accepts(field, value),
        "string" => {
            value.is_string()
                && field
                    .get("enum")
                    .is_none_or(|items| items.as_array().expect("enum").contains(value))
        }
        "boolean" => value.is_boolean(),
        "array" => value.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|item| field_accepts(&field["items"], item))
        }),
        other => panic!("unhandled emitted schema keyword: {other}"),
    }
}

fn integer_accepts(field: &Value, value: &Value) -> bool {
    let maximum = field["maximum"].as_u64().expect("nonnegative maximum");
    let minimum = field["minimum"].as_i64().expect("signed minimum");
    if let Some(n) = value.as_u64() {
        return n <= maximum;
    }
    if let Some(n) = value.as_i64() {
        return n >= minimum;
    }
    value.as_f64().is_some_and(|n| {
        n.is_finite() && n >= minimum as f64 && n.fract() == 0.0 && n < (maximum as f64 + 1.0)
    })
}

pub fn mutations(field: &Value) -> Vec<Value> {
    let mut values = vec![Value::Null, json!([]), json!({})];
    match field["type"].as_str().expect("type") {
        "integer" => values.extend([json!(-1), json!(1.5), json!("1"), json!(true)]),
        "string" => values.extend([json!(1), json!(false)]),
        "boolean" => values.extend([json!(1), json!("true")]),
        _ => unreachable!(),
    }
    values
}

// Declaration order from the canonical protocol structs, not JSON object order.
pub fn positional_payloads() -> Vec<(SystemAssetKind, Value)> {
    vec![
        (SystemAssetKind::Cpu, json!([4, 3200, "x86_64"])),
        (
            SystemAssetKind::Gpu,
            json!(["fixture", 8_000_000_000u64, 32]),
        ),
        (
            SystemAssetKind::Memory,
            json!([8_000_000_000u64, 4_000_000_000u64]),
        ),
        (
            SystemAssetKind::Storage,
            json!([100_000_000_000u64, "Nvme", 1_000_000_000, 500_000_000]),
        ),
        (SystemAssetKind::Network, json!([10_000_000, 1000, true])),
    ]
}

pub fn messaging_payloads() -> Vec<(SystemAssetKind, Value)> {
    use blockmatrix::{messaging::message::DirectMessage, sharing::invite::ShareInvite};
    vec![
        (
            SystemAssetKind::Invitation,
            encode(ShareInvite {
                invite_id: "synthetic-invite".into(),
                asset_id: "synthetic-asset".into(),
                sender_node_id: "sender".into(),
                sender_name: None,
                recipient_node_id: "recipient".into(),
                asset_name: "fixture".into(),
                asset_size: 42,
                shard_count: 1,
                shard_map_json: vec![0, 255],
                encrypted_key: vec![1, 2],
                key_kem_ciphertext: vec![3, 4],
                created_at: -1,
                signature: vec![],
            }),
        ),
        (
            SystemAssetKind::Message,
            encode(DirectMessage {
                message_id: "synthetic-message".into(),
                sender_node_id: "sender".into(),
                sender_name: None,
                recipient_node_id: "recipient".into(),
                encrypted_body: vec![0, 255],
                kem_ciphertext: vec![1, 2],
                reply_to: None,
                content_type: "synthetic/type".into(),
                created_at: -1,
                signature: vec![],
            }),
        ),
    ]
}

pub fn messaging_fields(kind: SystemAssetKind) -> &'static [&'static str] {
    match kind {
        SystemAssetKind::Invitation => &[
            "invite_id",
            "asset_id",
            "sender_node_id",
            "sender_name",
            "recipient_node_id",
            "asset_name",
            "asset_size",
            "shard_count",
            "shard_map_json",
            "encrypted_key",
            "key_kem_ciphertext",
            "created_at",
            "signature",
        ],
        SystemAssetKind::Message => &[
            "message_id",
            "sender_node_id",
            "sender_name",
            "recipient_node_id",
            "encrypted_body",
            "kem_ciphertext",
            "reply_to",
            "content_type",
            "created_at",
            "signature",
        ],
        _ => panic!("messaging kind only"),
    }
}
