use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use super::SystemPayloadError;

pub(super) fn for_kind(kind: SystemAssetKind) -> Result<Value, SystemPayloadError> {
    use SystemAssetKind::*;
    let fields = match kind {
        Cpu => vec![
            ("core_count", unsigned(u32::MAX as u64, "physical cores")),
            ("clock_mhz", unsigned(u32::MAX as u64, "MHz")),
            ("architecture", json!({"type": "string"})),
        ],
        Gpu => vec![
            ("model", json!({"type": "string"})),
            ("vram_bytes", unsigned(u64::MAX, "bytes")),
            ("compute_units", unsigned(u32::MAX as u64, "compute units")),
        ],
        Memory => vec![
            ("total_bytes", unsigned(u64::MAX, "bytes")),
            ("available_bytes", unsigned(u64::MAX, "bytes")),
        ],
        Storage => vec![
            ("total_bytes", unsigned(u64::MAX, "bytes")),
            (
                "storage_type",
                json!({"type": "string", "enum": ["Hdd", "Ssd", "Nvme", "Unknown"]}),
            ),
            ("read_speed_bps", unsigned(u64::MAX, "bytes/sec")),
            ("write_speed_bps", unsigned(u64::MAX, "bytes/sec")),
        ],
        Network => vec![
            ("bandwidth_bps", unsigned(u64::MAX, "bits per second")),
            ("latency_us", unsigned(u64::MAX, "microseconds")),
            ("ipv6_native", json!({"type": "boolean"})),
        ],
        Invitation | Message => return super::messaging::schema(kind),
        Transmission => return Ok(super::transmission::schema()),
        Dashboard => return Ok(super::dashboard::schema()),
        Dns => return Ok(super::dns::schema()),
        Identity => return Ok(super::identity::schema()),
        KeyRotation => return Ok(super::key_rotation::schema()),
        _ => return Err(SystemPayloadError::NotYetDefined { kind }),
    };
    Ok(object(kind, fields, &[]))
}

pub(super) fn unsigned(maximum: u64, description: &str) -> Value {
    json!({"type": "integer", "minimum": 0, "maximum": maximum, "description": description})
}

pub(super) fn object(
    kind: SystemAssetKind,
    fields: Vec<(&str, Value)>,
    optional: &[&str],
) -> Value {
    let required: Vec<_> = fields
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !optional.contains(name))
        .collect();
    let properties: serde_json::Map<_, _> = fields
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": format!("{} canonical system payload", kind.type_name()),
        "description": "Structural data only; not resource attestation or network admission.",
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": true
    })
}

pub(super) fn bytes() -> Value {
    json!({"type": "array", "items": {"type": "integer", "minimum": 0, "maximum": u8::MAX}})
}

pub(super) fn fixed_bytes(len: usize) -> Value {
    let mut schema = bytes();
    schema["minItems"] = json!(len);
    schema["maxItems"] = json!(len);
    schema
}
