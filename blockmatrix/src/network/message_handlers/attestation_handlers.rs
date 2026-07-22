// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.4 — receive and send mirror attestations over STOQ.
//!
//! # One gate, not two
//!
//! S3.3's blocking QA finding was a SECOND, parallel list of structural checks:
//! `verify_attestation` re-listed the checks it cared about instead of calling
//! the audit gate, and the two drifted — an attestation the accept path let in
//! was one the audit path would reject forever. The fix was structural: exactly
//! one list, reached by delegation.
//!
//! This handler holds that line. It performs NO verification of its own. It
//! parses the payload and calls
//! [`accept_wire_attestation`](crate::blockchain::NodeBlockchain::accept_wire_attestation),
//! which calls `record_mirror_attestation`, which calls `verify_attestation`,
//! which calls `MirrorAttestation::is_structurally_valid`. Adding a check here
//! — even a seemingly redundant one — would recreate exactly the divergence
//! S3.3 closed.
//!
//! In particular the handler does NOT re-check the identity binding
//! (`hex(BLAKE3(signer_pubkey)) == mirror`). That check lives inside
//! `is_structurally_valid` and is unconditional, so nothing here can bypass it:
//! a peer submitting an attestation naming a mirror other than its own signing
//! key is refused by the one gate, whatever the peer claims about itself. Note
//! that the submitter need not BE the mirror — relaying someone else's signed
//! attestation is legitimate and useful (that is how an owner learns about a
//! mirror it is not directly connected to). What is impossible is attesting AS
//! somebody else, because the signature, not the sender, names the attestor.
//!
//! # Admission policy (which is not verification)
//!
//! Two admission rules live inside `accept_wire_attestation` and the pool, and
//! neither restates a verification check:
//!
//! 1. **We must hold the asset.** `by_asset` is keyed by a 32-byte hash the
//!    sender chooses freely, so without this the key space is attacker-owned
//!    and the global pool bound becomes a lever: fill it with statements about
//!    assets nobody has, and honest attestations for real assets are refused.
//!    Requiring the asset to exist here — on the spine, or as an S3.4-adopted
//!    foreign chain — ties the key space to real local state.
//! 2. **The pool is bounded**, per asset and globally, and refuses newcomers at
//!    capacity rather than evicting incumbents. See
//!    [`MirrorAttestationPool`](crate::blockchain::MirrorAttestationPool).
//!
//! Rule 1 is also what keeps FALCON verification — the expensive part — off the
//! table for junk: an unknown asset is refused by a hash-map probe, before any
//! signature work.

use tracing::{info, warn};

use hypermesh_lib::attestation::MirrorAttestation;

use super::super::attestation_wire::{decode_mirror_attestation, encode_mirror_attestation};
use super::super::PeerContext;

/// Handle a received `TAG_MIRROR_ATTEST` message (tag 0x54).
///
/// `data` is the WHOLE message including the tag byte, matching
/// `handle_shard_announce` and `handle_block_announce`.
///
/// Returns `true` when the attestation was recorded. Failures are logged and
/// swallowed — this is a fire-and-forget announcement, and a peer learns
/// nothing from our silence that it could not learn by re-reading the rules.
pub(super) async fn handle_mirror_attestation(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
) -> bool {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];

    let attestation = match decode_mirror_attestation(data) {
        Ok(attestation) => attestation,
        Err(e) => {
            warn!("Mirror attestation from {short_id} could not be decoded: {e}");
            return false;
        }
    };

    // The ONE gate: the held-asset admission rule, then `verify_attestation`
    // and the bounded pool — all of it inside the chain, none of it restated
    // here. `accept_wire_attestation` is the whole of this handler's policy.
    match ctx
        .blockchain
        .accept_wire_attestation(attestation.clone())
        .await
    {
        Ok(()) => {
            info!(
                mirror = %attestation.mirror,
                cell = %attestation.matrix_index,
                "S3.4: recorded a mirror attestation submitted by peer {short_id}"
            );
            true
        }
        Err(e) => {
            warn!("Mirror attestation from {short_id} REJECTED: {e}");
            false
        }
    }
}

/// Send one mirror attestation to a connected peer.
///
/// The producing half of the surface: a mirror that has validated an asset
/// tells the peer it fetched from. Fire-and-forget — the stream is opened,
/// written and dropped, exactly as `broadcast_shard_announce` does.
pub async fn send_mirror_attestation(
    connection: &stoq::Connection,
    attestation: &MirrorAttestation,
) -> Result<(), String> {
    let payload = encode_mirror_attestation(attestation)
        .map_err(|e| format!("failed to encode mirror attestation: {e}"))?;

    let mut stream = connection
        .open_stream()
        .await
        .map_err(|e| format!("failed to open stream for mirror attestation: {e}"))?;
    stream
        .send(&payload)
        .await
        .map_err(|e| format!("failed to send mirror attestation: {e}"))?;
    Ok(())
}
