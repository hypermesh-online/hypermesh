use blockmatrix::{messaging::message::DirectMessage, sharing::invite::ShareInvite};
use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use super::{schema, SystemPayloadError, ValidatedSystemPayload};

pub(super) fn validate(
    kind: SystemAssetKind,
    value: &Value,
) -> Result<ValidatedSystemPayload, SystemPayloadError> {
    let result = match kind {
        SystemAssetKind::Invitation => serde_json::from_value::<ShareInvite>(value.clone())
            .map(ValidatedSystemPayload::Invitation),
        SystemAssetKind::Message => serde_json::from_value::<DirectMessage>(value.clone())
            .map(ValidatedSystemPayload::Message),
        _ => return Err(SystemPayloadError::NotYetDefined { kind }),
    };
    result.map_err(|source| SystemPayloadError::InvalidPayload { kind, source })
}

pub(super) fn schema(kind: SystemAssetKind) -> Result<Value, SystemPayloadError> {
    let fields = match kind {
        SystemAssetKind::Invitation => invitation_fields(),
        SystemAssetKind::Message => message_fields(),
        _ => return Err(SystemPayloadError::NotYetDefined { kind }),
    };
    Ok(schema::object(kind, fields, &["sender_name", "reply_to"]))
}

fn invitation_fields() -> Vec<(&'static str, Value)> {
    vec![
        ("invite_id", text()),
        ("asset_id", text()),
        ("sender_node_id", text()),
        ("sender_name", optional_text()),
        ("recipient_node_id", text()),
        ("asset_name", text()),
        ("asset_size", schema::unsigned(u64::MAX, "bytes")),
        ("shard_count", schema::unsigned(u32::MAX as u64, "shards")),
        ("shard_map_json", bytes()),
        ("encrypted_key", bytes()),
        ("key_kem_ciphertext", bytes()),
        ("created_at", timestamp()),
        ("signature", bytes()),
    ]
}

fn message_fields() -> Vec<(&'static str, Value)> {
    vec![
        ("message_id", text()),
        ("sender_node_id", text()),
        ("sender_name", optional_text()),
        ("recipient_node_id", text()),
        ("encrypted_body", bytes()),
        ("kem_ciphertext", bytes()),
        ("reply_to", optional_text()),
        ("content_type", text()),
        ("created_at", timestamp()),
        ("signature", bytes()),
    ]
}

fn text() -> Value {
    json!({"type": "string"})
}

fn optional_text() -> Value {
    json!({"type": ["string", "null"]})
}

fn bytes() -> Value {
    json!({
        "anyOf": [
            {"type": "array", "items": {"type": "integer", "minimum": 0, "maximum": 255}},
            {"type": "string"}
        ],
        "description": "Canonical JSON serialization is a byte array; serde also accepts raw UTF-8 strings, not hex/base64 decoding. Structural only, not cryptographic verification."
    })
}

fn timestamp() -> Value {
    json!({"type": "integer", "minimum": i64::MIN, "maximum": i64::MAX, "description": "Unix timestamp in seconds"})
}
