use blockmatrix::dns::{DnsBlockEntry, DnsRecordData};
use hypermesh_lib::SystemAssetKind;
use serde_json::{json, Value};

use super::schema;

pub(super) fn schema() -> Value {
    let text = json!({"type": "string"});
    let fields = vec![
        ("domain_name", text.clone()),
        (
            "record_type",
            json!({"type": "string", "enum": ["AAAA", "CNAME", "TXT", "Matrix", "SRV"]}),
        ),
        ("record_data", record_data()),
        ("ttl", schema::unsigned(u32::MAX as u64, "seconds")),
        ("owner", text),
        (
            "grant_signature",
            json!({"anyOf": [schema::bytes(), {"type": "null"}]}),
        ),
    ];
    schema::object(SystemAssetKind::Dns, fields, &["grant_signature"])
}

fn record_data() -> Value {
    let coordinate = json!({"type": "integer", "minimum": i64::MIN, "maximum": i64::MAX});
    let port = schema::unsigned(u16::MAX as u64, "unsigned 16-bit SRV field");
    json!({
        "description": "Externally tagged data. The separate record_type is not cross-checked by serde.",
        "oneOf": [
            tagged("AAAA", json!({"type": "string", "format": "ipv6"})),
            tagged("CNAME", json!({"type": "string"})),
            tagged("TXT", json!({"type": "string"})),
            tagged("Matrix", json!({
                "type": "object",
                "properties": {"x": coordinate, "y": coordinate, "z": coordinate},
                "required": ["x", "y", "z"]
            })),
            tagged("SRV", json!({
                "type": "object",
                "properties": {"priority": port, "weight": port, "port": port, "target": {"type": "string"}},
                "required": ["priority", "weight", "port", "target"]
            }))
        ]
    })
}

fn tagged(tag: &str, payload: Value) -> Value {
    json!({
        "type": "object",
        "properties": {(tag): payload},
        "required": [tag],
        "additionalProperties": false
    })
}

pub(super) fn equal(a: &DnsBlockEntry, b: &DnsBlockEntry) -> bool {
    a.domain_name == b.domain_name
        && a.record_type == b.record_type
        && data_equal(&a.record_data, &b.record_data)
        && a.ttl == b.ttl
        && a.owner == b.owner
        && a.grant_signature == b.grant_signature
}

fn data_equal(a: &DnsRecordData, b: &DnsRecordData) -> bool {
    use DnsRecordData::*;
    match (a, b) {
        (AAAA(a), AAAA(b)) => a == b,
        (CNAME(a), CNAME(b)) | (TXT(a), TXT(b)) => a == b,
        (Matrix(a), Matrix(b)) => a == b,
        (
            SRV {
                priority: p1,
                weight: w1,
                port: o1,
                target: t1,
            },
            SRV {
                priority: p2,
                weight: w2,
                port: o2,
                target: t2,
            },
        ) => (p1, w1, o1, t1) == (p2, w2, o2, t2),
        _ => false,
    }
}
