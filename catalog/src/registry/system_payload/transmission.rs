//! Catalog's explicit declared relay-resource payload, not a preexisting lib DTO.

use hypermesh_lib::SystemAssetKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{schema, SystemPayloadError, ValidatedSystemPayload};

/// Declared mesh relay capacity. This does not measure actual throughput, grant
/// admission or authority, reserve bandwidth, prove hardware, or attest PoS.
/// Trusted network scope and ownership binding are external to this payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransmissionPayload {
    /// Declared relay bandwidth in bits per second; zero means no declared capacity.
    pub bandwidth_bps: u64,
}

pub(super) fn validate(value: &Value) -> Result<ValidatedSystemPayload, SystemPayloadError> {
    serde_json::from_value::<TransmissionPayload>(value.clone())
        .map(ValidatedSystemPayload::Transmission)
        .map_err(|source| SystemPayloadError::InvalidPayload {
            kind: SystemAssetKind::Transmission,
            source,
        })
}

pub(super) fn schema() -> Value {
    let fields = vec![(
        "bandwidth_bps",
        schema::unsigned(
            u64::MAX,
            "Declared relay capacity in bits per second; zero means no declared capacity.",
        ),
    )];
    let mut result = schema::object(SystemAssetKind::Transmission, fields, &[]);
    result["title"] = "Transmission declared relay resource payload".into();
    result["description"] = "New Catalog relay resource description, not a preexisting canonical lib DTO, observed throughput, reservation, admission, authority or hardware proof. Trusted network scope and ownership are external.".into();
    result
}
