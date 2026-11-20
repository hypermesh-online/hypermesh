use super::TransmissionPayload;
use blockmatrix::{
    dashboard::DashboardManifest, dns::DnsBlockEntry, messaging::message::DirectMessage,
    sharing::invite::ShareInvite,
};
use hypermesh_lib::{
    CpuCapabilities, GpuCapabilities, IdentityAssetRecord, MemoryCapabilities, NetworkCapabilities,
    StorageCapabilities, SystemAssetKind,
};

use trustchain::identity::KeyRotationEntry;

/// Successfully deserialized canonical data, bound to its requested system kind.
/// No hardware, ownership, signature, proof-of-state or admission claim is made.
#[derive(Debug, Clone)]
pub enum ValidatedSystemPayload {
    Cpu(CpuCapabilities),
    Gpu(GpuCapabilities),
    Memory(MemoryCapabilities),
    Storage(StorageCapabilities),
    Network(NetworkCapabilities),
    Invitation(ShareInvite),
    Message(DirectMessage),
    Transmission(TransmissionPayload),
    Dashboard(DashboardManifest),
    Dns(DnsBlockEntry),
    Identity(IdentityAssetRecord),
    KeyRotation(KeyRotationEntry),
}

impl ValidatedSystemPayload {
    pub fn kind(&self) -> SystemAssetKind {
        match self {
            Self::Cpu(_) => SystemAssetKind::Cpu,
            Self::Gpu(_) => SystemAssetKind::Gpu,
            Self::Memory(_) => SystemAssetKind::Memory,
            Self::Storage(_) => SystemAssetKind::Storage,
            Self::Network(_) => SystemAssetKind::Network,
            Self::Invitation(_) => SystemAssetKind::Invitation,
            Self::Message(_) => SystemAssetKind::Message,
            Self::Transmission(_) => SystemAssetKind::Transmission,
            Self::Dashboard(_) => SystemAssetKind::Dashboard,
            Self::Dns(_) => SystemAssetKind::Dns,
            Self::Identity(_) => SystemAssetKind::Identity,
            Self::KeyRotation(_) => SystemAssetKind::KeyRotation,
        }
    }
}

impl PartialEq for ValidatedSystemPayload {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cpu(a), Self::Cpu(b)) => a == b,
            (Self::Gpu(a), Self::Gpu(b)) => a == b,
            (Self::Memory(a), Self::Memory(b)) => a == b,
            (Self::Storage(a), Self::Storage(b)) => a == b,
            (Self::Network(a), Self::Network(b)) => a == b,
            (Self::Invitation(a), Self::Invitation(b)) => invite_eq(a, b),
            (Self::Message(a), Self::Message(b)) => message_eq(a, b),
            (Self::Transmission(a), Self::Transmission(b)) => a == b,
            (Self::Dashboard(a), Self::Dashboard(b)) => a == b,
            (Self::Dns(a), Self::Dns(b)) => super::dns::equal(a, b),
            (Self::Identity(a), Self::Identity(b)) => super::identity::equal(a, b),
            (Self::KeyRotation(a), Self::KeyRotation(b)) => super::key_rotation::equal(a, b),
            _ => false,
        }
    }
}

impl Eq for ValidatedSystemPayload {}

fn invite_eq(a: &ShareInvite, b: &ShareInvite) -> bool {
    a.invite_id == b.invite_id
        && a.asset_id == b.asset_id
        && a.sender_node_id == b.sender_node_id
        && a.sender_name == b.sender_name
        && a.recipient_node_id == b.recipient_node_id
        && a.asset_name == b.asset_name
        && a.asset_size == b.asset_size
        && a.shard_count == b.shard_count
        && a.shard_map_json == b.shard_map_json
        && a.encrypted_key == b.encrypted_key
        && a.key_kem_ciphertext == b.key_kem_ciphertext
        && a.created_at == b.created_at
        && a.signature == b.signature
}

fn message_eq(a: &DirectMessage, b: &DirectMessage) -> bool {
    a.message_id == b.message_id
        && a.sender_node_id == b.sender_node_id
        && a.sender_name == b.sender_name
        && a.recipient_node_id == b.recipient_node_id
        && a.encrypted_body == b.encrypted_body
        && a.kem_ciphertext == b.kem_ciphertext
        && a.reply_to == b.reply_to
        && a.content_type == b.content_type
        && a.created_at == b.created_at
        && a.signature == b.signature
}
