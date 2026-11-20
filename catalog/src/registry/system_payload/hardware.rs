use hypermesh_lib::SystemAssetKind;
use serde_json::Value;

use super::{SystemPayloadError, ValidatedSystemPayload};

pub(super) fn validate(
    kind: SystemAssetKind,
    value: &Value,
) -> Result<ValidatedSystemPayload, SystemPayloadError> {
    use SystemAssetKind::*;
    let result = match kind {
        Cpu => serde_json::from_value(value.clone()).map(ValidatedSystemPayload::Cpu),
        Gpu => serde_json::from_value(value.clone()).map(ValidatedSystemPayload::Gpu),
        Memory => serde_json::from_value(value.clone()).map(ValidatedSystemPayload::Memory),
        Storage => serde_json::from_value(value.clone()).map(ValidatedSystemPayload::Storage),
        Network => serde_json::from_value(value.clone()).map(ValidatedSystemPayload::Network),
        Invitation | Message => return super::messaging::validate(kind, value),
        _ => return Err(SystemPayloadError::NotYetDefined { kind }),
    };
    result.map_err(|source| SystemPayloadError::InvalidPayload { kind, source })
}
