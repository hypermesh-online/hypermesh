// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P2P file sharing: invite types, key wrapping, and inbox storage.
//!
//! A sender creates a [`ShareInvite`] containing a shard map and a
//! Kyber-1024-wrapped decryption key, signs it with FALCON-1024, and
//! sends it to the recipient over STOQ (TAG_SHARE_INVITE = 0x05).
//! The recipient's [`InboxStore`] persists the invite until the user
//! accepts or dismisses it.

pub mod inbox;
pub mod invite;
pub mod key_wrap;
