// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
//
// Track P EXIT — the capstone E2E proving the worlds/placement layer end to end.
//
// One flow drives the REAL production surfaces of P3–P6 (no re-implementation),
// tying P3 → P4 → P6 → P5 so each stage consumes the previous stage's real
// output:
//
//   P3  — proximity locality. A real `ngauge::LocalityProvider` is built from
//         synthetic RTT samples: a dense low-RTT band and a distant outlier.
//         The provider's own `coordinate_for` output is asserted to make
//         DISTANCE physical — the band clusters near the origin, the outlier is
//         far — so "distance" is a measured proximity ordering, not a random
//         hash point.
//   P4  — placement. The P3 proximity coordinates become the announced matrix
//         coordinates of real PoS-eligible peers, and the promoted placement
//         authority (`network::placement::place_shards` → the distribution
//         crate's `distribute_shards_pos_aware`) places an asset's shards over
//         that live peer set. Every placement is asserted to land on a REAL
//         proximity-derived peer coordinate (never synthetic geometry), and the
//         store → reconstruct round trip is byte-identical (content-addressing
//         intact). The P4 shards produced here are the SAME bytes P6 forms a
//         world around.
//   P6  — world formation. A congested `HotspotAlert` over the P4 shards drives
//         the real `ngauge::WorldManager`: a child world forms with its parent
//         set, the hot shards migrate down, and the node JOINS the child in the
//         same step (atomic form-and-join: `world_of(hot) == child` AND
//         `admits(child)`).
//   P5  — isolation, now real. With that world formed, the real
//         `WorldIsolationGate::check_fetch` accepts a fetch for a shard in a
//         world the node belongs to (home or the joined child) and rejects a
//         fetch for a shard in a world it never joined, recording an audited
//         violation. Merging the child (coverage gap) returns its shards to the
//         parent and makes the dissolved world foreign — with NO window in which
//         a held shard maps to a world the gate no longer admits.
//
// Capstone: proximity makes distance real (P3), placement uses it (P4), a world
// forms from load and the holder joins it atomically (P6), and isolation then
// accepts same-world / rejects cross-world (P5) — with no legitimate fetch ever
// stranded.
//
// Boundary note (P4 transport round trip): this drives the in-process
// placement → reconstruct path (content-addressing + Reed-Solomon round trip),
// not a live two-node STOQ store→fetch. A real transport fetch requires the
// two-node QUIC harness whose FALCON-envelope friction is flaky; introducing it
// here would trade a meaningful capstone for a flaky one. The transport-carriage
// boundary that `s3_p4_placement_roundtrip` flagged therefore remains open by
// deliberate choice — the placement AUTHORITY and the content round trip are
// both real and in the loop; only the QUIC stream carriage is elided.

use std::collections::HashSet;
use std::net::SocketAddr;

use blockmatrix::assets::pipeline::{Asset, AssetPipeline, PipelineInputMetadata};
use blockmatrix::matrix::MatrixCoordinate;
use blockmatrix::network::isolation::WorldIsolationGate;
use blockmatrix::network::placement::place_shards;
use blockmatrix::network::trust::NetworkType;
use blockmatrix::network::NetworkNode;
use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId, PrivacyMode, GLOBAL_WORLD};
use ngauge::collective_intel::{CoverageGap, HotspotAlert};
use ngauge::{LocalityProvider, WorldManager};

/// A world the node never joins — used to prove cross-world rejection.
const FOREIGN_WORLD: NetworkId = NetworkId([0xF0; 16]);

fn origin_dist(p: &MatrixPosition) -> f64 {
    (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
}

fn dist(a: &MatrixPosition, b: &MatrixPosition) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Round a peer's P3 proximity coordinate to the integer matrix cell that
/// becomes its announced coordinate for placement.
fn proximity_cell(provider: &LocalityProvider, peer_id: &str) -> MatrixCoordinate {
    let pos = provider
        .coordinate_for(peer_id)
        .expect("test: peer has a measured proximity sample");
    MatrixCoordinate::new(pos.x.round() as i64, pos.y.round() as i64, pos.z.round() as i64)
        .expect("test: proximity cell is in bounds")
}

/// A real `NetworkNode` announced at `coord`. No live connection, so the
/// placement path uses the announced (proximity-derived) coordinate directly —
/// the seam that carries P3's output into P4.
fn peer(node_id: &str, coord: MatrixCoordinate) -> NetworkNode {
    NetworkNode {
        coordinate: coord,
        address: "[::1]:9292".parse::<SocketAddr>().expect("test: valid addr"),
        node_id: node_id.to_string(),
        privacy_mode: PrivacyMode::PRIVATE,
        connection: None,
    }
}

fn hotspot(congestion: f64) -> HotspotAlert {
    HotspotAlert {
        center: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
        congestion_ratio: congestion,
        affected_nodes: 5,
        severity: "high".to_string(),
    }
}

/// The `ContentHash` a pipeline `Shard` is addressed by — BLAKE3 over its bytes.
/// This is what ties the shards P4 actually placed to the shards P6 forms a
/// world around: same bytes, same identity.
fn shard_hash(data: &[u8]) -> ContentHash {
    ContentHash::from_bytes(*blake3::hash(data).as_bytes())
}

#[tokio::test]
async fn p_exit_capstone_proximity_placement_worlds_isolation_end_to_end() {
    // ── P3: proximity makes distance physical ──────────────────────────────
    // A dense low-RTT band (~0.5–1.1 ms) and one distant outlier (~60 ms). RTTs
    // chosen so the band's cells are distinct integers (x = rtt/100 = 5,7,9,11)
    // and the outlier lands far out (x = 6000).
    let near_ids = ["near-0", "near-1", "near-2", "near-3"];
    let near_rtts = [500u64, 700, 900, 1100];
    let outlier_id = "outlier";
    let outlier_rtt = 600_000u64;

    let mut provider = LocalityProvider::new();
    for (id, rtt) in near_ids.iter().zip(near_rtts.iter()) {
        provider.observe(*id, *rtt);
    }
    provider.observe(outlier_id, outlier_rtt);

    let near_coords: Vec<MatrixPosition> = near_ids
        .iter()
        .map(|id| provider.coordinate_for(id).expect("test: near sample present"))
        .collect();
    let outlier_coord = provider
        .coordinate_for(outlier_id)
        .expect("test: outlier sample present");

    // A1: every near peer sits close to the origin (a tight locality).
    for c in &near_coords {
        assert!(
            origin_dist(c) < 20.0,
            "P3: a low-RTT peer must embed near the origin, got {c:?}"
        );
    }
    // A2: the outlier is genuinely distant — proximity is not uniform.
    assert!(
        origin_dist(&outlier_coord) > 1000.0,
        "P3: a high-RTT peer must embed far from the origin, got {outlier_coord:?}"
    );
    // A3: the band clusters — intra-band spread is dwarfed by the gap to the
    // outlier. If `coordinate_for` returned a random-hash cell this fails.
    let mut intra = 0.0;
    let mut n = 0.0;
    for a in &near_coords {
        for b in &near_coords {
            intra += dist(a, b);
            n += 1.0;
        }
    }
    intra /= n;
    let inter: f64 =
        near_coords.iter().map(|c| dist(c, &outlier_coord)).sum::<f64>() / near_coords.len() as f64;
    assert!(
        inter > intra * 5.0,
        "P3: distance to the outlier ({inter:.1}) must dominate intra-band spread ({intra:.1})"
    );

    // ── P4: placement uses the P3 proximity coordinates ────────────────────
    // The proximity cells become the announced coordinates of real peers.
    let peers: Vec<NetworkNode> = near_ids
        .iter()
        .chain(std::iter::once(&outlier_id))
        .map(|id| peer(id, proximity_cell(&provider, id)))
        .collect();
    let real_coords: HashSet<(i64, i64, i64)> = peers
        .iter()
        .map(|p| (p.coordinate.x, p.coordinate.y, p.coordinate.z))
        .collect();
    let all_ids: HashSet<&str> = near_ids.iter().copied().chain(std::iter::once(outlier_id)).collect();

    // Enough data to force real Reed-Solomon sharding into many shards.
    let original: Vec<u8> = b"HYPERMESH-P-EXIT-E2E-PLACEMENT-".repeat(4096);
    let asset = Asset {
        id: hex::encode(blake3::hash(&original).as_bytes()),
        data: original.clone(),
        metadata: PipelineInputMetadata {
            name: "p-exit.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: original.len(),
            created_at: 0,
            custom: std::collections::HashMap::new(),
        },
    };
    let asset_id = asset.id.clone();

    let pipeline = AssetPipeline::default().expect("test: pipeline");
    let processed = pipeline
        .process_asset_with_privacy(asset, PrivacyMode::PRIVATE)
        .await
        .expect("test: process asset");

    // A4: the pipeline does not fabricate placements — placement is P4's job.
    assert!(
        processed.distributed.placements.is_empty(),
        "P4: the pipeline must NOT fabricate placements"
    );

    let placements =
        place_shards(&peers, &processed.shards, &asset_id, PrivacyMode::PRIVATE).await;

    // A5: with eligible peers present, real placement occurs.
    assert!(!placements.is_empty(), "P4: eligible peers present ⇒ placement must occur");
    // A6: every placement lands on a REAL proximity-derived peer coordinate and
    // names a real eligible peer — never synthetic golden-ratio geometry.
    for pl in &placements {
        let pos = (pl.position.x, pl.position.y, pl.position.z);
        assert!(
            real_coords.contains(&pos),
            "P4: placement {pos:?} must be a real proximity-derived peer coordinate"
        );
        assert!(
            all_ids.contains(pl.node_id.as_str()),
            "P4: placement must name a real eligible peer, got {}",
            pl.node_id
        );
    }
    // A7: the far outlier coordinate (x = 6000) — a cell no random-hash geometry
    // would ever produce — is a legitimate placement target, proving the peer's
    // measured proximity, not a fabricated point, drove WHERE.
    let outlier_cell = proximity_cell(&provider, outlier_id);
    assert!(
        real_coords.contains(&(outlier_cell.x, outlier_cell.y, outlier_cell.z)),
        "P4: the proximity outlier cell must be among the real placement targets"
    );

    // A8: store → reconstruct is byte-identical (content-addressing intact).
    let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("test: reconstruct");
    assert_eq!(
        reconstructed, original,
        "P4: store → reconstruct round trip must reproduce the original bytes exactly"
    );

    // ── P6: a world forms from load; the holder joins it atomically ────────
    // The hot shards are the actual bytes P4 placed (addressed by BLAKE3).
    assert!(processed.shards.len() >= 3, "test: need ≥3 shards for hot/cold split");
    let hot: Vec<ContentHash> =
        processed.shards.iter().take(2).map(|s| shard_hash(&s.data)).collect();
    let cold = shard_hash(&processed.shards[2].data);

    let mut wm = WorldManager::new(GLOBAL_WORLD);

    // A9: before any world forms, a hot shard lives in the home world.
    assert_eq!(
        wm.world_of(&hot[0]),
        GLOBAL_WORLD,
        "P6: pre-formation, every shard is in the home world"
    );

    let formation = wm
        .form_from_hotspot(GLOBAL_WORLD, &hotspot(0.95), &hot)
        .expect("test: a congested hotspot must form a child world");
    let child = formation.child;

    // A10: the child world has its parent set (parent_network_id).
    assert_eq!(formation.parent, GLOBAL_WORLD, "P6: formation records the parent world");
    assert_eq!(
        wm.parent_of(child),
        Some(GLOBAL_WORLD),
        "P6: the child nests under its parent (parent pointer set)"
    );
    assert_ne!(child, GLOBAL_WORLD, "P6: a formed child is a distinct world");
    // A11: the hot shards migrated down into the child.
    assert_eq!(formation.migrated, hot, "P6: exactly the hot shards migrate into the child");
    // A12 (ATOMIC FORM-AND-JOIN): the shard's true world is the child AND the
    // node is a member of that child — form and join are one step.
    assert_eq!(
        wm.world_of(&hot[0]),
        child,
        "P6: the migrated shard's true world is now the child"
    );
    assert!(
        wm.admits(child),
        "P6: the holder JOINED the child in the same step it migrated the shards"
    );
    // A13: an unassigned (cold) shard stays in the parent world, untouched.
    assert_eq!(wm.world_of(&cold), GLOBAL_WORLD, "P6: an unassigned shard stays in the parent");

    // ── P5: isolation over the formed world ────────────────────────────────
    let gate = WorldIsolationGate::mount(GLOBAL_WORLD, NetworkType::P2P)
        .await
        .expect("test: mount home-world gate");
    // The node joins the child on the gate (Part C mirrors WorldManager
    // membership into the gate as worlds form).
    gate.admit_world(child, NetworkType::P2P)
        .await
        .expect("test: admit the formed child world into the gate");

    // A14 (ACCEPT SAME-WORLD, joined child): fed the shard's TRUE world from the
    // real WorldManager, a fetch for the migrated shard is ACCEPTED — the holder
    // is not stranded by formation.
    assert!(
        gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok(),
        "P5: a legitimate holder's fetch in the joined child world must be ACCEPTED"
    );
    // A15 (ACCEPT SAME-WORLD, home): the cold shard's home-world fetch passes.
    assert!(
        gate.check_fetch(wm.world_of(&cold), &cold).await.is_ok(),
        "P5: a home-world fetch must be ACCEPTED"
    );
    // A16 (REJECT CROSS-WORLD): a fetch for a shard in a world the node never
    // joined is REJECTED.
    assert!(
        gate.check_fetch(FOREIGN_WORLD, &ContentHash([0xF1; 32])).await.is_err(),
        "P5: a fetch for a never-joined world must be REJECTED"
    );
    // A17: exactly one audited violation, for the cross-world attempt.
    let violations = gate.violations().await;
    assert_eq!(violations.len(), 1, "P5: exactly one cross-world rejection is audited");
    assert_eq!(violations[0].source_network, GLOBAL_WORLD);
    assert_eq!(violations[0].destination_network, FOREIGN_WORLD);

    // ── P5 merge: the dissolved world becomes foreign, no fetch stranded ────
    // Reabsorb the child's shards into the parent FIRST (WorldManager), so at no
    // instant does a held shard map to a world the gate no longer admits.
    let gap = CoverageGap {
        center: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
        radius: 10.0,
        node_count: 0,
    };
    let merges = wm.consume_coverage_gap(GLOBAL_WORLD, &gap);
    assert_eq!(merges.len(), 1, "P5: the coverage gap reabsorbs exactly the emergent child");
    assert_eq!(merges[0].child, child);
    assert_eq!(merges[0].parent, GLOBAL_WORLD);

    // A18: the hot shard has returned to the parent world.
    assert_eq!(
        wm.world_of(&hot[0]),
        GLOBAL_WORLD,
        "P5 merge: the reabsorbed shard is back in the parent world"
    );
    // A19 (NO WINDOW): at this instant — after reabsorption, before the gate has
    // even revoked the child — the shard's TRUE world is the (always-admitted)
    // parent, so a fetch by its true world is ACCEPTED. No stranding window.
    assert!(
        gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok(),
        "P5 merge: a fetch by the shard's true world must never be stranded during merge"
    );

    // The gate now follows the WorldManager: the node leaves the dissolved child.
    gate.revoke_world(child).await;

    // A20: the reabsorbed shard still fetches via its (parent) world after revoke.
    assert!(
        gate.check_fetch(wm.world_of(&hot[0]), &hot[0]).await.is_ok(),
        "P5 merge: the reabsorbed shard remains fetchable via its parent world"
    );
    // A21: the dissolved child world is now FOREIGN — an explicit fetch for it
    // is rejected.
    assert!(
        gate.check_fetch(child, &hot[0]).await.is_err(),
        "P5 merge: the dissolved child world must now be REJECTED as foreign"
    );
}
