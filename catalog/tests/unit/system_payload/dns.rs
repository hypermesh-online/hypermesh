use blockmatrix::dns::{DnsBlockEntry, DnsRecordData, DnsRecordType};
use catalog::registry::system_payload::{definition, ValidatedSystemPayload};
use hypermesh_lib::SystemAssetKind::Dns;
use serde_json::{json, Value};

fn registration() -> DnsBlockEntry {
    DnsBlockEntry {
        domain_name: "example".into(),
        record_type: DnsRecordType::AAAA,
        record_data: DnsRecordData::AAAA("fd48::1".parse().unwrap()),
        ttl: u32::MAX,
        owner: "fixture-owner".into(),
        grant_signature: None,
    }
}

fn assert_serde(value: &Value, expected: bool) {
    let canonical = serde_json::from_value::<DnsBlockEntry>(value.clone());
    let actual = definition(Dns).unwrap().validate(value);
    assert_eq!(canonical.is_ok(), expected, "{value}");
    assert_eq!(actual.is_ok(), expected, "{value}");
    if let Ok(canonical) = canonical {
        assert_eq!(actual.unwrap(), ValidatedSystemPayload::Dns(canonical));
    }
}

#[test]
fn on_chain_entry_serializes_without_an_optional_grant() {
    let value = serde_json::to_value(registration()).unwrap();
    assert_eq!(value["record_data"], json!({"AAAA": "fd48::1"}));
    assert!(!value.as_object().unwrap().contains_key("grant_signature"));
    assert_serde(&value, true);
    let def = definition(Dns).unwrap();
    assert_eq!(
        def.json_schema()["required"],
        json!(["domain_name", "record_type", "record_data", "ttl", "owner"])
    );
    assert_eq!(def.json_schema()["properties"]["ttl"]["maximum"], u32::MAX);
}

#[test]
fn externally_tagged_variants_are_structural_not_authority() {
    let def = definition(Dns).unwrap();
    let schema = &def.json_schema()["properties"]["record_data"]["oneOf"];
    let base = serde_json::to_value(registration()).unwrap();
    let valid = [
        ("AAAA", json!("::1")),
        ("CNAME", json!("target")),
        ("TXT", json!("example")),
        ("Matrix", json!({"x": i64::MIN, "y": i64::MAX, "z": 0})),
        (
            "SRV",
            json!({"priority": 0, "weight": 0, "port": u16::MAX, "target": "peer"}),
        ),
    ];
    for (index, (tag, payload)) in valid.into_iter().enumerate() {
        let mut value = base.clone();
        // Deliberately leave record_type = AAAA, independently of record_data.
        value["record_data"] = json!({(tag): payload});
        assert_serde(&value, true);
        assert_eq!(schema[index]["required"], json!([tag]));
        assert_eq!(schema[index]["additionalProperties"], false);
    }
    let mut value = base;
    value["record_data"] = json!({"AAAA": "127.0.0.1"});
    assert_serde(&value, false);
    value["record_data"] = json!({"AAAA": "::1", "TXT": "extra tag"});
    assert_serde(&value, false);
}

#[test]
fn positional_inputs_and_plain_byte_arrays_follow_serde() {
    let mut value = json!(["example", "Matrix", {"Matrix": [-1, 2, 3]}, 42, "owner", null]);
    assert_serde(&value, true);
    value[5] = json!([0, 255]);
    assert_serde(&value, true);
    value[5] = json!("not a byte-array");
    assert_serde(&value, false);
    value[5] = json!([256]);
    assert_serde(&value, false);
}
