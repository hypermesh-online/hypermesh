// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sync/reflector, shard dispatch/announce, and block-fetch handlers.

use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_transport;
use crate::network::stoq_integration::MatrixMessage;
use crate::network::sync_dispatch;
use hypermesh_lib::{ContentHash, NetworkId};

use super::super::PeerContext;
use super::dns_protocol::{DistributedDnsQuery, DistributedDnsResponse};
use super::message_utils::send_sync_reply;
use super::protocol::{TAG_DNS_RESOLVE_RESPONSE, TAG_DNS_RESPONSE};

mod shard;
pub(super) use shard::{handle_shard_announce, handle_shard_dispatch, handle_shard_locate};
mod dns;
pub(super) use dns::{handle_dns_query, handle_dns_resolve_request};
mod sync;
pub(super) use sync::{
    handle_block_fetch_request, handle_sync_message, register_peer_as_reflector,
};
mod messaging;
pub(crate) use messaging::parse_and_store_share_invite;
pub(super) use messaging::{
    handle_direct_message, handle_key_rotation, handle_share_invite, handle_transfer_message,
};
