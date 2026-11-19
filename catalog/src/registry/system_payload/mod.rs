//! Structural payload definitions for seven canonical kinds and declared relay capacity.
//!
//! These definitions do not establish ownership, physical capability, proof of
//! state, or network admission. In particular, `ipv6_native` proves no routability.
//! Application schemas and the remaining seven system kinds are not defined here.

mod hardware;
mod messaging;
mod schema;
mod transmission;
mod types;

use hypermesh_lib::SystemAssetKind;
use serde_json::Value;

pub use transmission::TransmissionPayload;
pub use types::ValidatedSystemPayload;

/// An unsupported kind or a declared-type serde deserialization failure.
#[derive(Debug, thiserror::Error)]
pub enum SystemPayloadError {
    #[error("system payload definition not yet defined for {kind}")]
    NotYetDefined { kind: SystemAssetKind },
    #[error("invalid {kind} payload: {source}")]
    InvalidPayload {
        kind: SystemAssetKind,
        #[source]
        source: serde_json::Error,
    },
}

/// A kind-bound, local definition; not a published or versioned registry entry.
#[derive(Debug, Clone)]
pub struct SystemPayloadDefinition {
    kind: SystemAssetKind,
    json_schema: Value,
}

impl SystemPayloadDefinition {
    pub fn kind(&self) -> SystemAssetKind {
        self.kind
    }

    /// JSON Schema draft 2020-12 for the canonical object's field structure.
    ///
    /// JSON Schema treats `1.0` as an integer; canonical serde rejects a floating
    /// representation for unsigned Rust fields. Use `validate` for typed parity,
    /// not a schema engine alone. Unknown object fields are accepted, as by serde.
    /// The schema describes serialized objects only; canonical serde additionally
    /// accepts positional arrays in declaration-field order. `validate` preserves
    /// that sequence behavior, which this object schema does not describe.
    /// Messaging byte fields serialize as arrays; canonical serde also accepts
    /// UTF-8 strings as raw bytes (not hex/base64). Both appear in their schemas.
    pub fn json_schema(&self) -> &Value {
        &self.json_schema
    }

    /// Deserialize the declared type, including its positional sequence inputs.
    /// Transmission uses the new Catalog DTO, not a preexisting canonical lib DTO.
    /// This is not object-schema validation and does not attest any resource.
    pub fn validate(&self, value: &Value) -> Result<ValidatedSystemPayload, SystemPayloadError> {
        if self.kind == SystemAssetKind::Transmission {
            return transmission::validate(value);
        }
        hardware::validate(self.kind, value)
    }
}

/// Obtain a definition for seven canonical kinds or declared Transmission capacity.
pub fn definition(kind: SystemAssetKind) -> Result<SystemPayloadDefinition, SystemPayloadError> {
    let json_schema = schema::for_kind(kind)?;
    Ok(SystemPayloadDefinition { kind, json_schema })
}
