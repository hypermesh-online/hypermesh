use super::*;

/// Whether a wire tag may only be handled for an AUTHENTICATED, same-network
/// peer.
///
/// This is the single source of truth for the dispatch auth gate — extracted so
/// the security boundary is a testable predicate rather than an inline
/// `matches!` no test can reach. A tag that returns `true` here is refused for
/// any peer not in the [`AuthenticatedPeers`] map (see [`dispatch_message`]):
/// requiring auth ⟹ an unauthenticated connection never reaches the handler.
///
/// Block announcements ARE gated (defense-in-depth, P1): although each block is
/// independently validated by BLAKE3 content integrity, per-entry `proof_hash`,
/// signed-to-content binding, and `state_proof.validate()`, the announcing peer
/// must also have passed the bilateral PoS handshake. Shard access, sync,
/// distributed-CA and cross-network transfer operations are gated for the same
/// reason.
pub fn message_requires_auth(tag: u8) -> bool {
    matches!(
        tag,
        TAG_SHARD_SEND | TAG_SHARD_FETCH
            | TAG_BLOCK_ANNOUNCE
            | TAG_SYNC_MESSAGE | TAG_BLOCK_FETCH_REQUEST
            | TAG_CA_KEY_SHARE | TAG_CA_SIGN_REQUEST | TAG_CA_SIGN_RESPONSE
            | TAG_CRL_REQUEST | TAG_CRL_RESPONSE
            | TAG_TRANSFER_LOCK | TAG_TRANSFER_REGISTER_REQ | TAG_TRANSFER_REGISTER_ACK
            | TAG_TRANSFER_RELEASE | TAG_TRANSFER_ROLLBACK
            // S3.4: an attestation is a third party's statement about an asset
            // we hold, cached in a BOUNDED pool and eventually sealed on-chain
            // by the owner. Same standing as a block announcement: each one is
            // independently FALCON-verified, AND the submitting peer must have
            // passed the bilateral PoS handshake for this network.
            | TAG_MIRROR_ATTEST
            // D3: a presented asset chain is a peer offering an asset's verified
            // sub-chain for adoption into our off-spine received store. Its
            // internal lineage and every signer are FALCON-verified inside
            // `accept_asset_chain`, AND — because the store is now network-fed —
            // an UNAUTHENTICATED peer must never reach the accept path. This
            // gate is the only thing standing between an anonymous connection
            // and the received store; it is not optional.
            | TAG_ASSET_CHAIN
    )
}

/// The handler that owns a given wire tag.
///
/// A pure name for "which arm of the dispatch table runs this tag", extracted
/// so the tag→handler wiring is a value a test can assert on, rather than an
/// inline `match` arm no test can reach. [`dispatch_to_handler`] is the single
/// place that turns one of these back into the actual handler call, so this
/// enum and that `match` are the only two things that must agree — and
/// [`route_message`] (which produces these) is exhaustively unit-tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    /// Shard send/fetch (fetch additionally F6-authorized). `TAG_SHARD_SEND`
    /// / `TAG_SHARD_FETCH`.
    ShardDispatch,
    /// Block announcement. `TAG_BLOCK_ANNOUNCE`.
    BlockAnnounce,
    /// Sync / reflector message. `TAG_SYNC_MESSAGE`.
    SyncMessage,
    /// Block fetch request. `TAG_BLOCK_FETCH_REQUEST`.
    BlockFetchRequest,
    /// Shard availability announcement. `TAG_SHARD_ANNOUNCE`.
    ShardAnnounce,
    /// S3.4 mirror attestation. `TAG_MIRROR_ATTEST` (0x54).
    MirrorAttestation,
    /// D3 presented asset chain. `TAG_ASSET_CHAIN` (0x55).
    AssetChain,
    /// A2 shard-locate query. `TAG_SHARD_LOCATE`.
    ShardLocate,
    /// Share invite. `TAG_SHARE_INVITE`.
    ShareInvite,
    /// Direct message. `TAG_DIRECT_MESSAGE`.
    DirectMessage,
    /// Cross-network transfer message. `TAG_TRANSFER`.
    Transfer,
    /// Distributed-CA key share. `TAG_CA_KEY_SHARE`.
    CaKeyShare,
    /// Distributed-CA threshold sign request. `TAG_CA_SIGN_REQUEST`.
    CaSignRequest,
    /// Distributed-CA threshold sign response. `TAG_CA_SIGN_RESPONSE`.
    CaSignResponse,
    /// CRL fetch request. `TAG_CRL_REQUEST`.
    CrlRequest,
    /// CRL fetch response. `TAG_CRL_RESPONSE`.
    CrlResponse,
    /// Key rotation announcement. `TAG_KEY_ROTATION`.
    KeyRotation,
    /// DNS resolution request. `TAG_DNS_RESOLVE`.
    DnsResolve,
    /// Phase H.1 rich DNS query. `TAG_DNS_QUERY`.
    DnsQuery,
    /// Cross-network transfer lock. `TAG_TRANSFER_LOCK`.
    TransferLock,
    /// Cross-network transfer register request. `TAG_TRANSFER_REGISTER_REQ`.
    TransferRegisterReq,
    /// Cross-network transfer register ack. `TAG_TRANSFER_REGISTER_ACK`.
    TransferRegisterAck,
    /// Cross-network transfer release. `TAG_TRANSFER_RELEASE`.
    TransferRelease,
    /// Cross-network transfer rollback. `TAG_TRANSFER_ROLLBACK`.
    TransferRollback,
    /// Gossip message (logged, not gated). `TAG_GOSSIP`.
    Gossip,
}

/// The routing decision for a wire tag: which handler owns it, that an
/// unauthenticated peer must be dropped BEFORE any handler, or that no arm
/// handles the tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// The tag is handled; [`dispatch_to_handler`] runs the named handler.
    Handler(Handler),
    /// The tag requires authentication and the peer is not authenticated — the
    /// message is dropped before the handler runs (see [`message_requires_auth`]).
    DropUnauthenticated,
    /// No arm handles this tag.
    Unknown,
}

/// The dispatch decision as a pure function of `(tag, is_authenticated)`.
///
/// This is the wiring [`dispatch_message`] executes, lifted out of the async
/// I/O path so it is exhaustively unit-testable WITHOUT a QUIC stream. Two
/// invariants live here and are proved in-module:
///
///  1. **Auth gate.** Every tag for which [`message_requires_auth`] is `true`
///     routes to [`Route::DropUnauthenticated`] when `is_authenticated` is
///     `false` — an unauthenticated peer never reaches the handler. Ungated
///     tags ignore `is_authenticated`.
///  2. **Tag→handler.** Each handled tag maps to exactly one [`Handler`];
///     everything else is [`Route::Unknown`].
///
/// `dispatch_message` computes `is_authenticated` from the live
/// [`AuthenticatedPeers`] map (only consulting it for gated tags, preserving
/// the original short-circuit) and then executes this decision. A future
/// refactor that misroutes a tag or drops the gate on one fails the unit tests
/// below without needing a live connection.
pub(crate) fn route_message(tag: u8, is_authenticated: bool) -> Route {
    // Gate asset-level operations on peer authentication + network scope.
    if message_requires_auth(tag) && !is_authenticated {
        return Route::DropUnauthenticated;
    }

    match tag {
        TAG_SHARD_SEND | TAG_SHARD_FETCH => Route::Handler(Handler::ShardDispatch),
        TAG_BLOCK_ANNOUNCE => Route::Handler(Handler::BlockAnnounce),
        TAG_SYNC_MESSAGE => Route::Handler(Handler::SyncMessage),
        TAG_BLOCK_FETCH_REQUEST => Route::Handler(Handler::BlockFetchRequest),
        TAG_SHARD_ANNOUNCE => Route::Handler(Handler::ShardAnnounce),
        TAG_MIRROR_ATTEST => Route::Handler(Handler::MirrorAttestation),
        TAG_ASSET_CHAIN => Route::Handler(Handler::AssetChain),
        TAG_SHARD_LOCATE => Route::Handler(Handler::ShardLocate),
        TAG_SHARE_INVITE => Route::Handler(Handler::ShareInvite),
        TAG_DIRECT_MESSAGE => Route::Handler(Handler::DirectMessage),
        TAG_TRANSFER => Route::Handler(Handler::Transfer),
        TAG_CA_KEY_SHARE => Route::Handler(Handler::CaKeyShare),
        TAG_CA_SIGN_REQUEST => Route::Handler(Handler::CaSignRequest),
        TAG_CA_SIGN_RESPONSE => Route::Handler(Handler::CaSignResponse),
        TAG_CRL_REQUEST => Route::Handler(Handler::CrlRequest),
        TAG_CRL_RESPONSE => Route::Handler(Handler::CrlResponse),
        TAG_KEY_ROTATION => Route::Handler(Handler::KeyRotation),
        TAG_DNS_RESOLVE => Route::Handler(Handler::DnsResolve),
        TAG_DNS_QUERY => Route::Handler(Handler::DnsQuery),
        TAG_TRANSFER_LOCK => Route::Handler(Handler::TransferLock),
        TAG_TRANSFER_REGISTER_REQ => Route::Handler(Handler::TransferRegisterReq),
        TAG_TRANSFER_REGISTER_ACK => Route::Handler(Handler::TransferRegisterAck),
        TAG_TRANSFER_RELEASE => Route::Handler(Handler::TransferRelease),
        TAG_TRANSFER_ROLLBACK => Route::Handler(Handler::TransferRollback),
        TAG_GOSSIP => Route::Handler(Handler::Gossip),
        _ => Route::Unknown,
    }
}
