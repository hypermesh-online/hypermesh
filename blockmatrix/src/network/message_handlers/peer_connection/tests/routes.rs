use super::super::*;

/// Every wire tag the dispatcher handles, paired with the handler that owns
/// it and whether it is auth-gated. This table is the test's independent
/// statement of the wiring; the tests below cross-check it against BOTH
/// `route_message` and `message_requires_auth`, so a drift in any of the
/// three is caught.
const ROUTES: &[(u8, Handler, bool)] = &[
    (TAG_SHARD_SEND, Handler::ShardDispatch, true),
    (TAG_SHARD_FETCH, Handler::ShardDispatch, true),
    (TAG_BLOCK_ANNOUNCE, Handler::BlockAnnounce, true),
    (TAG_SYNC_MESSAGE, Handler::SyncMessage, true),
    (TAG_BLOCK_FETCH_REQUEST, Handler::BlockFetchRequest, true),
    (TAG_SHARD_ANNOUNCE, Handler::ShardAnnounce, false),
    (TAG_MIRROR_ATTEST, Handler::MirrorAttestation, true),
    (TAG_ASSET_CHAIN, Handler::AssetChain, true),
    (TAG_SHARD_LOCATE, Handler::ShardLocate, false),
    (TAG_SHARE_INVITE, Handler::ShareInvite, false),
    (TAG_DIRECT_MESSAGE, Handler::DirectMessage, false),
    (TAG_TRANSFER, Handler::Transfer, false),
    (TAG_CA_KEY_SHARE, Handler::CaKeyShare, true),
    (TAG_CA_SIGN_REQUEST, Handler::CaSignRequest, true),
    (TAG_CA_SIGN_RESPONSE, Handler::CaSignResponse, true),
    (TAG_CRL_REQUEST, Handler::CrlRequest, true),
    (TAG_CRL_RESPONSE, Handler::CrlResponse, true),
    (TAG_KEY_ROTATION, Handler::KeyRotation, false),
    (TAG_DNS_RESOLVE, Handler::DnsResolve, false),
    (TAG_DNS_QUERY, Handler::DnsQuery, false),
    (TAG_TRANSFER_LOCK, Handler::TransferLock, true),
    (
        TAG_TRANSFER_REGISTER_REQ,
        Handler::TransferRegisterReq,
        true,
    ),
    (
        TAG_TRANSFER_REGISTER_ACK,
        Handler::TransferRegisterAck,
        true,
    ),
    (TAG_TRANSFER_RELEASE, Handler::TransferRelease, true),
    (TAG_TRANSFER_ROLLBACK, Handler::TransferRollback, true),
    (TAG_GOSSIP, Handler::Gossip, false),
];

/// An authenticated peer's framed message reaches the handler that owns its
/// tag; the gated flag in the table matches `message_requires_auth` exactly.
#[test]
fn authenticated_peer_routes_every_tag_to_its_handler() {
    for &(tag, handler, gated) in ROUTES {
        assert_eq!(
            route_message(tag, true),
            Route::Handler(handler),
            "tag 0x{tag:02x} must route to {handler:?} for an authenticated peer",
        );
        assert_eq!(
            message_requires_auth(tag),
            gated,
            "the gated flag for tag 0x{tag:02x} must match message_requires_auth",
        );
    }
}

/// An UNAUTHENTICATED peer is dropped before the handler for every gated
/// tag, and still reaches the handler for every ungated tag. This is the
/// auth gate proved end-to-end at the routing layer.
#[test]
fn unauthenticated_peer_is_dropped_for_gated_tags_only() {
    for &(tag, handler, gated) in ROUTES {
        let route = route_message(tag, false);
        if gated {
            assert_eq!(
                route,
                Route::DropUnauthenticated,
                "gated tag 0x{tag:02x} must be dropped for an unauthenticated peer",
            );
        } else {
            assert_eq!(
                route,
                Route::Handler(handler),
                "ungated tag 0x{tag:02x} must route regardless of authentication",
            );
        }
    }
}

/// The two arms task #51 names — 0x55 asset-chain and 0x54 attestation —
/// stated explicitly: authenticated routes to the handler, unauthenticated
/// is dropped. Both are the same gap; both are covered.
#[test]
fn asset_chain_and_attestation_arms_are_wired_and_gated() {
    assert_eq!(TAG_ASSET_CHAIN, 0x55, "asset-chain tag is 0x55");
    assert_eq!(TAG_MIRROR_ATTEST, 0x54, "mirror-attestation tag is 0x54");

    assert_eq!(
        route_message(TAG_ASSET_CHAIN, true),
        Route::Handler(Handler::AssetChain),
    );
    assert_eq!(
        route_message(TAG_ASSET_CHAIN, false),
        Route::DropUnauthenticated,
    );
    assert_eq!(
        route_message(TAG_MIRROR_ATTEST, true),
        Route::Handler(Handler::MirrorAttestation),
    );
    assert_eq!(
        route_message(TAG_MIRROR_ATTEST, false),
        Route::DropUnauthenticated,
    );
}

/// Exhaustive over all 256 tag values: the routing table is total and
/// consistent. Exactly the tags in `ROUTES` are handled; every other tag is
/// `Unknown`. No tag is ever gated by `message_requires_auth` without
/// appearing (as gated) in `ROUTES`, so the gate can never guard a tag the
/// table does not know about.
#[test]
fn routing_is_total_and_consistent_over_all_tags() {
    for tag in 0..=u8::MAX {
        check_tag_routing(tag);
    }
}

fn check_tag_routing(tag: u8) {
    let known = ROUTES.iter().find(|&&(t, _, _)| t == tag);
    match known {
        Some(&(_, handler, gated)) => {
            assert_eq!(route_message(tag, true), Route::Handler(handler));
            assert_eq!(
                route_message(tag, false),
                expected_unauthenticated_route(handler, gated),
            );
        }
        None => {
            // An unknown tag is never auth-gated, so authentication
            // state cannot change its verdict.
            assert!(
                !message_requires_auth(tag),
                "tag 0x{tag:02x} is gated but has no route",
            );
            assert_eq!(route_message(tag, true), Route::Unknown);
            assert_eq!(route_message(tag, false), Route::Unknown);
        }
    }
}

fn expected_unauthenticated_route(handler: Handler, gated: bool) -> Route {
    if gated {
        Route::DropUnauthenticated
    } else {
        Route::Handler(handler)
    }
}
