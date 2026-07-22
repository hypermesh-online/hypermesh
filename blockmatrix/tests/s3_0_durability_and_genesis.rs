// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.0 prerequisite proofs:
//   B1 — runtime blocks are durably persisted and survive a restart.
//   B2 — the genesis path is deterministic (identical inputs → identical hash).
//   B4 — concurrent `add_block` callers never silently lose their work
//        (S3.0 QA follow-up FIX 2: head→insert race).
//
// B3 (real genesis request/response round-trip) is proven in the dispatcher's
// own unit tests (`blockmatrix/src/network/sync_dispatch/tests.rs`) because the
// dispatcher types it exercises are crate-internal.

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{BlockSink, NodeBlockchain};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};
use trustchain::proof_of_state::StateProof;

const NODE_ID: &str = "s3-durability-node";

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(4, 5, 6).expect("test: valid coordinate")
}

/// A content-bound entry that satisfies the signed-to-content invariant.
fn bound_entry(tag: &[u8]) -> BlockAssetEntry {
    let registration =
        blockmatrix::assets::core::AssetRegistration::genesis(coord());
    let asset_hash = *blake3::hash(tag).as_bytes();
    BlockAssetEntry::new_bound(
        asset_hash,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        registration,
    )
}

async fn manager(dir: &std::path::Path) -> std::sync::Arc<PersistenceManager> {
    let config = PersistenceConfig {
        storage_dir: dir.to_path_buf(),
        enable_background: false,
        ..PersistenceConfig::default()
    };
    std::sync::Arc::new(
        PersistenceManager::new(config, NODE_ID.to_string())
            .await
            .expect("test: persistence manager"),
    )
}

/// Rebuild a chain purely from what is on disk, exactly as the node binary's
/// resume path does (`bin/node/bootstrap.rs`).
async fn reload_from_disk(dir: &std::path::Path) -> Vec<blockmatrix::blockchain::Block> {
    let persistence = manager(dir).await;
    let stats = persistence.get_stats().await;
    let height = stats.block_count.saturating_sub(1);

    let mut blocks = Vec::new();
    for idx in 0..=height {
        if let Some(block) = persistence
            .load_block(BlockQuery::ByIndex(idx))
            .await
            .expect("test: load block")
        {
            blocks.push(block);
        }
    }
    blocks
}

/// B1 — with a block sink attached, a block added at runtime is on disk and a
/// chain rebuilt from disk still has it.
#[tokio::test]
async fn b1_runtime_block_survives_restart_with_sink() {
    let dir = tempfile::tempdir().expect("test: temp dir");
    let persistence = manager(dir.path()).await;

    let genesis = blockmatrix::blockchain::Block::genesis(coord());
    persistence
        .save_block(&genesis)
        .await
        .expect("test: persist genesis");

    let chain = NodeBlockchain::from_genesis(coord(), genesis.clone())
        .with_persistence(persistence.clone());

    let block = chain
        .add_block(vec![bound_entry(b"s3-b1-runtime-asset")])
        .await
        .expect("test: add_block");
    assert_eq!(block.index, 1);

    // Drop the live chain entirely — simulate process exit.
    drop(chain);
    drop(persistence);

    let blocks = reload_from_disk(dir.path()).await;
    assert_eq!(
        blocks.len(),
        2,
        "genesis + the runtime block must both be on disk, got {}",
        blocks.len()
    );

    let rebuilt =
        NodeBlockchain::from_blocks(coord(), blocks).expect("test: rebuild chain from disk");
    assert_eq!(rebuilt.get_height().await, 1);
    let recovered = rebuilt
        .get_block(1)
        .await
        .expect("test: runtime block present after restart");
    assert_eq!(recovered.hash, block.hash);
}

/// B1 (regression witness) — WITHOUT a sink the block is accepted in memory and
/// is NOT on disk. This is exactly the pre-S3.0 production behaviour: every
/// runtime block was lost on restart.
#[tokio::test]
async fn b1_without_sink_runtime_block_is_lost_on_restart() {
    let dir = tempfile::tempdir().expect("test: temp dir");
    let persistence = manager(dir.path()).await;

    let genesis = blockmatrix::blockchain::Block::genesis(coord());
    persistence
        .save_block(&genesis)
        .await
        .expect("test: persist genesis");

    // No `.with_persistence(...)` — the pre-S3.0 shape.
    let chain = NodeBlockchain::from_genesis(coord(), genesis.clone());
    chain
        .add_block(vec![bound_entry(b"s3-b1-lost-asset")])
        .await
        .expect("test: add_block");
    assert_eq!(chain.get_height().await, 1, "in memory the block is there");

    drop(chain);
    drop(persistence);

    let blocks = reload_from_disk(dir.path()).await;
    assert_eq!(
        blocks.len(),
        1,
        "pre-S3.0 behaviour: only genesis reaches disk, the runtime block is lost",
    );
}

/// B1 — a received (remote) block also write-throughs, not just locally
/// produced ones. `insert_received_block` and `add_block` share the same
/// insert chokepoint.
#[tokio::test]
async fn b1_received_block_is_persisted() {
    let dir = tempfile::tempdir().expect("test: temp dir");
    let persistence = manager(dir.path()).await;

    let genesis = blockmatrix::blockchain::Block::genesis(coord());
    persistence
        .save_block(&genesis)
        .await
        .expect("test: persist genesis");

    // Producer chain (memory only) builds a block we then "receive".
    let producer = NodeBlockchain::from_genesis(coord(), genesis.clone());
    let produced = producer
        .add_block(vec![bound_entry(b"s3-b1-received-asset")])
        .await
        .expect("test: produce block");

    let receiver = NodeBlockchain::from_genesis(coord(), genesis.clone())
        .with_persistence(persistence.clone());

    // The producer chain has no signer, so the entry carries no envelope;
    // accept it under the documented one-release legacy migration flag.
    std::env::set_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS", "1");
    let result = receiver.insert_received_block(produced.clone()).await;
    std::env::remove_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS");
    result.expect("test: insert_received_block");

    drop(receiver);
    drop(producer);
    drop(persistence);

    let blocks = reload_from_disk(dir.path()).await;
    assert_eq!(blocks.len(), 2, "received block must be durable too");
    assert_eq!(blocks[1].hash, produced.hash);
}

// =====================================================================
// B4 — concurrent append correctness (S3.0 QA follow-up, FIX 2)
// =====================================================================

/// A sink that takes measurable time to "persist", so the window between
/// reading the chain head and inserting the built block is wide enough for
/// concurrent appenders to collide. It also records what it was asked to
/// persist, which is the durable-side count.
struct SlowSink {
    delay: std::time::Duration,
    persisted: tokio::sync::Mutex<Vec<u64>>,
}

impl SlowSink {
    fn new(delay_ms: u64) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            delay: std::time::Duration::from_millis(delay_ms),
            persisted: tokio::sync::Mutex::new(Vec::new()),
        })
    }
}

#[async_trait::async_trait]
impl BlockSink for SlowSink {
    async fn persist_block(&self, block: &Block) -> Result<(), String> {
        tokio::time::sleep(self.delay).await;
        self.persisted.lock().await.push(block.index);
        Ok(())
    }
}

/// B4 — N concurrent `add_block` callers against a chain with a slow durable
/// sink. Every caller must either get its block appended or a real error, and
/// the chain must hold exactly as many blocks as there were successes.
///
/// BEFORE the fix `add_block` read the head, dropped the lock, then built,
/// signed and inserted; the persistence latency inside that window meant all
/// concurrent callers computed the SAME index and all but one lost with
/// "Block N already exists" — their built (and, on a signing chain, FALCON
/// signed) block was discarded. QA measured 2/8 succeeding with a sink.
/// AFTER the fix a head reservation serialises the read→build→insert sequence,
/// so all 8 append.
#[tokio::test]
async fn b4_concurrent_add_block_never_silently_loses_work() {
    const WRITERS: usize = 8;

    let sink = SlowSink::new(20);
    let chain = std::sync::Arc::new(
        NodeBlockchain::from_genesis(coord(), blockmatrix::blockchain::Block::genesis(coord()))
            .with_persistence(sink.clone()),
    );

    let mut handles = Vec::with_capacity(WRITERS);
    for i in 0..WRITERS {
        let chain = chain.clone();
        handles.push(tokio::spawn(async move {
            chain
                .add_block(vec![bound_entry(format!("s3-b4-writer-{i}").as_bytes())])
                .await
        }));
    }

    let mut ok = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for handle in handles {
        match handle.await.expect("test: writer task must not panic") {
            Ok(_) => ok += 1,
            Err(e) => errors.push(e),
        }
    }

    // 1. Nothing vanished: every caller got a verdict.
    assert_eq!(
        ok + errors.len(),
        WRITERS,
        "every writer must return a verdict",
    );

    // 2. No silent loss: the chain holds exactly one block per success.
    assert_eq!(
        chain.get_height().await as usize,
        ok,
        "chain height must equal the number of successful appends (errors: {errors:?})",
    );

    // 3. Durable side agrees with memory (fail-closed write-through).
    assert_eq!(
        sink.persisted.lock().await.len(),
        ok,
        "the sink must have persisted exactly the appended blocks",
    );

    // 4. The whole point: with the head reservation, no writer loses.
    assert_eq!(
        ok, WRITERS,
        "all {WRITERS} concurrent writers must append; lost writers: {errors:?}",
    );

    // 5. The resulting chain is a valid, correctly linked chain.
    assert!(chain.validate_chain().await, "serialised appends must link");
}

/// B2 — the genesis path is a pure function of its inputs: two genesis blocks
/// built from an identical `HardwareAssessment` + genesis epoch are
/// byte-identical.
#[tokio::test]
async fn b2_genesis_is_deterministic_for_identical_inputs() {
    use blockmatrix::proof_of_state::genesis_proof::{
        generate_genesis_proof, GenesisEpoch, HardwareAssessment,
    };
    use blockmatrix::os_integration::{DeviceFingerprint, DeviceIdentifiers};

    let fingerprint = DeviceFingerprint::compose(DeviceIdentifiers {
        machine_id: Some("machine-determinism".to_string()),
        product_uuid: Some("uuid-determinism".to_string()),
        board_serial: Some("board-determinism".to_string()),
        product_serial: None,
        primary_disk_serial: Some("disk-determinism".to_string()),
        primary_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
    });

    let hw = || HardwareAssessment {
        cpu_cores: 4,
        cpu_mhz: 2400,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 500 * 1024 * 1024 * 1024,
        storage_available_bytes: 250 * 1024 * 1024 * 1024,
        node_id: "s3-determinism-node".to_string(),
        coordinate: coord(),
        device_fingerprint: fingerprint.clone(),
        disk_serial: Some("disk-determinism".to_string()),
    };

    let epoch = GenesisEpoch::from_unix_secs(1_780_000_000);

    let proof_a = generate_genesis_proof(&hw(), epoch);
    let proof_b = generate_genesis_proof(&hw(), epoch);
    assert_eq!(
        serde_json::to_vec(&proof_a).expect("test: serialize"),
        serde_json::to_vec(&proof_b).expect("test: serialize"),
        "genesis proof must be byte-identical for identical inputs",
    );

    let block_a = blockmatrix::blockchain::Block::genesis_from_assessment(&hw(), epoch);
    let block_b = blockmatrix::blockchain::Block::genesis_from_assessment(&hw(), epoch);
    assert_eq!(
        block_a.hash, block_b.hash,
        "two genesis blocks from identical inputs must share one hash",
    );
    assert_eq!(
        serde_json::to_vec(&block_a).expect("test: serialize"),
        serde_json::to_vec(&block_b).expect("test: serialize"),
        "two genesis blocks from identical inputs must be byte-identical",
    );
    assert!(block_a.verify_hash());
    assert!(block_a.is_genesis());
}

/// B2 (regression witness) — the PRE-S3.0 shape. The old genesis path read the
/// wall clock internally (three `SystemTime::now()` calls across the proofs,
/// a clock-derived PoTime nonce, and a fourth read inside
/// `AssetRegistration::genesis`), so two calls with identical inputs differed.
/// Driving the epoch from the clock reproduces exactly that and shows the test
/// above could not have passed before the change.
#[tokio::test]
async fn b2_clock_driven_genesis_is_not_reproducible() {
    use blockmatrix::os_integration::{DeviceFingerprint, DeviceIdentifiers};
    use blockmatrix::proof_of_state::genesis_proof::{GenesisEpoch, HardwareAssessment};

    let fingerprint = DeviceFingerprint::compose(DeviceIdentifiers {
        machine_id: Some("machine-witness".to_string()),
        product_uuid: Some("uuid-witness".to_string()),
        board_serial: Some("board-witness".to_string()),
        product_serial: None,
        primary_disk_serial: Some("disk-witness".to_string()),
        primary_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
    });

    let hw = || HardwareAssessment {
        cpu_cores: 4,
        cpu_mhz: 2400,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 500 * 1024 * 1024 * 1024,
        storage_available_bytes: 250 * 1024 * 1024 * 1024,
        node_id: "s3-witness-node".to_string(),
        coordinate: coord(),
        device_fingerprint: fingerprint.clone(),
        disk_serial: Some("disk-witness".to_string()),
    };

    let a = blockmatrix::blockchain::Block::genesis_from_assessment(&hw(), GenesisEpoch::now());
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let b = blockmatrix::blockchain::Block::genesis_from_assessment(&hw(), GenesisEpoch::now());

    assert_ne!(
        a.hash, b.hash,
        "a clock-driven epoch reproduces the pre-S3.0 non-determinism",
    );
}

/// B2 — a DIFFERENT epoch (or a different device) still produces a different
/// genesis. Determinism must not collapse distinct networks/devices into one
/// block.
#[tokio::test]
async fn b2_genesis_differs_when_inputs_differ() {
    use blockmatrix::proof_of_state::genesis_proof::{
        generate_genesis_proof, GenesisEpoch, HardwareAssessment,
    };
    use blockmatrix::os_integration::{DeviceFingerprint, DeviceIdentifiers};

    let fp = |seed: &str| {
        DeviceFingerprint::compose(DeviceIdentifiers {
            machine_id: Some(format!("machine-{seed}")),
            product_uuid: Some(format!("uuid-{seed}")),
            board_serial: Some(format!("board-{seed}")),
            product_serial: None,
            primary_disk_serial: Some(format!("disk-{seed}")),
            primary_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        })
    };

    let hw = |seed: &str| HardwareAssessment {
        cpu_cores: 4,
        cpu_mhz: 2400,
        memory_bytes: 8 * 1024 * 1024 * 1024,
        storage_bytes: 500 * 1024 * 1024 * 1024,
        storage_available_bytes: 250 * 1024 * 1024 * 1024,
        node_id: "s3-determinism-node".to_string(),
        coordinate: coord(),
        device_fingerprint: fp(seed),
        disk_serial: Some(format!("disk-{seed}")),
    };

    let e1 = GenesisEpoch::from_unix_secs(1_780_000_000);
    let e2 = GenesisEpoch::from_unix_secs(1_780_000_001);

    let same_device_other_epoch = generate_genesis_proof(&hw("A"), e2);
    let baseline = generate_genesis_proof(&hw("A"), e1);
    let other_device = generate_genesis_proof(&hw("B"), e1);

    assert_ne!(
        serde_json::to_vec(&baseline).expect("test: serialize"),
        serde_json::to_vec(&same_device_other_epoch).expect("test: serialize"),
        "a different genesis epoch must produce a different proof",
    );
    assert_ne!(
        serde_json::to_vec(&baseline).expect("test: serialize"),
        serde_json::to_vec(&other_device).expect("test: serialize"),
        "a different device must produce a different proof",
    );
}
