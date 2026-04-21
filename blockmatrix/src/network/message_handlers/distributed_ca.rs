// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Distributed CA message handlers (key share, threshold signing).

use anyhow::{anyhow, Result};
use tracing::info;

use super::super::PeerContext;

/// Handle a CA key share distribution message (tag 0x30).
///
/// Wire format: tag(1) + JSON-serialized MatrixMessage::KeyShareDistribute.
/// In production this will store the share in FederationManager. For alpha
/// we log receipt and validate the JSON structure.
pub(super) async fn handle_ca_key_share(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) -> Result<()> {
    let msg_data = &data[1..]; // skip tag
    let msg: serde_json::Value = serde_json::from_slice(msg_data)
        .map_err(|e| anyhow!("Invalid CA key share JSON: {e}"))?;

    let share_index = msg.get("share_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let fingerprint_hex = msg.get("fingerprint")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    info!(
        peer = %&peer_node_id[..8.min(peer_node_id.len())],
        share_index = share_index,
        fingerprint = %&fingerprint_hex[..16.min(fingerprint_hex.len())],
        "Received CA key share from peer"
    );

    // In production: store in FederationManager
    // For alpha: log receipt and validate structure
    Ok(())
}

/// Handle a threshold signing request (tag 0x31).
///
/// Wire format: tag(1) + JSON-serialized MatrixMessage::ThresholdSignRequest.
/// In production this will check if we hold a key share for the requested CA
/// fingerprint and contribute a partial signature. For alpha we log receipt.
pub(super) async fn handle_ca_sign_request(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) -> Result<()> {
    let msg_data = &data[1..];
    let msg: serde_json::Value = serde_json::from_slice(msg_data)
        .map_err(|e| anyhow!("Invalid CA sign request JSON: {e}"))?;

    let request_id = msg.get("request_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    info!(
        peer = %&peer_node_id[..8.min(peer_node_id.len())],
        request_id = request_id,
        "Received threshold sign request"
    );

    // In production: check if we hold a key share, contribute partial signature
    // For alpha: log and acknowledge
    Ok(())
}

/// Handle a threshold signing response (tag 0x32).
///
/// Wire format: tag(1) + JSON-serialized MatrixMessage::ThresholdSignResponse.
/// In production this will collect partial signatures and combine them when
/// enough shares are gathered. For alpha we log receipt.
pub(super) async fn handle_ca_sign_response(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) -> Result<()> {
    let msg_data = &data[1..];
    let msg: serde_json::Value = serde_json::from_slice(msg_data)
        .map_err(|e| anyhow!("Invalid CA sign response JSON: {e}"))?;

    let request_id = msg.get("request_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let accepted = msg.get("accepted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    info!(
        peer = %&peer_node_id[..8.min(peer_node_id.len())],
        request_id = request_id,
        accepted = accepted,
        "Received threshold sign response"
    );

    Ok(())
}
