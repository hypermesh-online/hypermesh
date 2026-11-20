// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Userspace adaptive-control tests over genuine IPv6-loopback QUIC streams.
//! Tier/parameter records and stream continuity are observed, not physical network
//! capacity, live buffer/MTU reconfiguration, congestion-controller replacement,
//! PoS authorization, or kernel acceleration. No synthetic adaptation result.

use anyhow::Result;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use stoq::transport::{
    AdaptiveConnection, Connection, Endpoint, NetworkTier, StoqTransport, TransportConfig,
};
use tokio::time::{sleep, timeout};

const DEADLINE: Duration = Duration::from_secs(10);

struct Fixture {
    transport: StoqTransport,
    client: Arc<Connection>,
    peer: Arc<Connection>,
    adaptive: Arc<AdaptiveConnection>,
}

fn config() -> TransportConfig {
    TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        enable_kernel_acceleration: false, // Userspace fixture, never attach kernel programs.
        send_buffer_size: 1024 * 1024,
        ..TransportConfig::default()
    }
}

impl Fixture {
    async fn new(enabled: bool) -> Self {
        let transport = StoqTransport::new(config()).await.unwrap();
        transport.start_adaptation().await;
        transport.set_adaptation_enabled(enabled);
        let addr = transport.local_addr().unwrap();
        assert_eq!(addr.ip(), Ipv6Addr::LOCALHOST);
        assert_ne!(addr.port(), 0);
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, addr.port());
        // Observe BOTH accept/connect results; no detached server task or hidden error.
        let (client, peer) = timeout(DEADLINE, async {
            tokio::try_join!(transport.connect(&endpoint), transport.accept())
        })
        .await
        .expect("connect/accept deadline")
        .expect("connect/accept failed");
        let adaptive = transport
            .get_adaptive_connection(&client.id())
            .expect("connected client must have an adaptive wrapper");
        let fixture = Self {
            transport,
            client,
            peer,
            adaptive,
        };
        fixture.echo(b"before adaptation").await;
        fixture
    }

    async fn echo(&self, payload: &[u8]) {
        let server = async {
            let (mut send, mut recv) = self.peer.accept_bi().await?;
            let bytes = recv.read_to_end(4096).await?;
            anyhow::ensure!(bytes == payload, "server received unexpected data");
            send.write_all(&bytes).await?;
            send.finish()?;
            Ok::<_, anyhow::Error>(())
        };
        let client = async {
            let (mut send, mut recv) = self.client.open_bi().await?;
            send.write_all(payload).await?;
            send.finish()?;
            let bytes = recv.read_to_end(4096).await?;
            anyhow::ensure!(bytes == payload, "client received unexpected echo");
            Ok::<_, anyhow::Error>(())
        };
        timeout(DEADLINE, async { tokio::try_join!(server, client) })
            .await
            .expect("echo deadline")
            .expect("checked bidirectional exchange failed");
    }

    async fn finish(self) {
        self.echo(b"after adaptation").await;
        self.transport.shutdown().await;
    }
}

async fn wait_for_count(adaptive: &AdaptiveConnection, previous: u64) {
    timeout(DEADLINE, async {
        while adaptive.adaptation_stats().adaptation_count <= previous {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("expected adaptation counter transition");
}

/// The live-config call triggers adaptation; exact new buffer values are NOT applied
/// by this production API, so this does not pretend to verify buffer reconfiguration.
#[tokio::test]
async fn test_live_connection_adaptation() {
    let mut fixture = Fixture::new(true).await;
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    let mut next = config();
    next.send_buffer_size = 16 * 1024 * 1024;
    next.max_concurrent_streams = 1000;
    fixture.transport.update_live_config(next).await;
    wait_for_count(&fixture.adaptive, before).await;
    assert!(fixture.adaptive.adaptation_stats().enabled);
    fixture.finish().await;
}

/// Detect actual loopback observations, not a fabricated bandwidth measurement.
#[tokio::test]
async fn test_tier_detection() {
    let fixture = Fixture::new(true).await;
    fixture
        .transport
        .set_connection_tier(
            &fixture.client.id(),
            NetworkTier::DataCenter { gbps: 100.0 },
        )
        .await
        .unwrap();
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    timeout(DEADLINE, async {
        while fixture.adaptive.adaptation_stats().adaptation_count == before {
            fixture.transport.auto_detect_tiers().await;
            sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("automatic tier detection must actually transition");
    assert!(!matches!(
        fixture.adaptive.current_tier(),
        NetworkTier::DataCenter { .. }
    ));
    assert!(fixture.adaptive.conditions().last_update.elapsed() < Duration::from_secs(1));
    fixture.finish().await;
}

/// Normal adaptation must respect stability; force_adapt explicitly bypasses it.
#[tokio::test]
async fn test_hysteresis() {
    let fixture = Fixture::new(true).await;
    fixture
        .transport
        .set_connection_tier(
            &fixture.client.id(),
            NetworkTier::DataCenter { gbps: 100.0 },
        )
        .await
        .unwrap();
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    timeout(Duration::from_secs(2), async {
        for _ in 0..5 {
            fixture.adaptive.adapt().await.unwrap();
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("rapid stability-window observations");
    assert_eq!(fixture.adaptive.adaptation_stats().adaptation_count, before);
    assert!(matches!(
        fixture.adaptive.current_tier(),
        NetworkTier::DataCenter { .. }
    ));
    fixture
        .transport
        .force_connection_adaptation(&fixture.client.id())
        .await
        .unwrap();
    assert_eq!(
        fixture.adaptive.adaptation_stats().adaptation_count,
        before + 1
    );
    fixture.finish().await;
}

#[tokio::test]
async fn test_disable_adaptation() {
    let fixture = Fixture::new(false).await;
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    assert!(!fixture.adaptive.adaptation_stats().enabled);
    fixture
        .transport
        .force_connection_adaptation(&fixture.client.id())
        .await
        .unwrap();
    sleep(Duration::from_millis(200)).await;
    assert_eq!(fixture.adaptive.adaptation_stats().adaptation_count, before);
    fixture.transport.set_adaptation_enabled(true);
    fixture
        .transport
        .force_connection_adaptation(&fixture.client.id())
        .await
        .unwrap();
    assert!(fixture.adaptive.adaptation_stats().enabled);
    assert_eq!(
        fixture.adaptive.adaptation_stats().adaptation_count,
        before + 1
    );
    fixture.finish().await;
}

#[tokio::test]
async fn test_manual_tier_setting() {
    let fixture = Fixture::new(true).await;
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    fixture
        .transport
        .set_connection_tier(
            &fixture.client.id(),
            NetworkTier::DataCenter { gbps: 100.0 },
        )
        .await
        .unwrap();
    assert!(
        matches!(fixture.adaptive.current_tier(), NetworkTier::DataCenter { gbps } if gbps == 100.0)
    );
    assert_eq!(
        fixture.adaptive.adaptation_stats().adaptation_count,
        before + 1
    );
    let params = fixture.adaptive.parameters();
    assert!(params.stream_window >= 32 * 1024 * 1024);
    assert!(params.max_streams >= 1000);
    fixture.finish().await;
}

/// Manual tier transitions on one real connection, NOT simulated physical networks.
#[tokio::test]
async fn test_manual_tier_transitions_with_live_streams() {
    let fixture = Fixture::new(true).await;
    let scenarios = [
        (NetworkTier::Home { mbps: 300.0 }, 2 * 1024 * 1024, 50),
        (NetworkTier::Standard { gbps: 1.0 }, 8 * 1024 * 1024, 100),
        (
            NetworkTier::DataCenter { gbps: 100.0 },
            32 * 1024 * 1024,
            1000,
        ),
    ];
    for (tier, window, streams) in scenarios {
        let before = fixture.adaptive.adaptation_stats().adaptation_count;
        fixture
            .transport
            .set_connection_tier(&fixture.client.id(), tier.clone())
            .await
            .unwrap();
        assert_eq!(
            format!("{:?}", fixture.adaptive.current_tier()),
            format!("{tier:?}")
        );
        assert_eq!(
            fixture.adaptive.adaptation_stats().adaptation_count,
            before + 1
        );
        let params = fixture.adaptive.parameters();
        assert_eq!(params.stream_window, window);
        assert_eq!(params.max_streams, streams);
        fixture.echo(format!("tier {tier:?}").as_bytes()).await;
    }
    fixture.finish().await;
}

/// Existing local latency limits, with every requested adaptation observed.
#[tokio::test]
async fn test_adaptation_overhead() -> Result<()> {
    let fixture = Fixture::new(true).await;
    let before = fixture.adaptive.adaptation_stats().adaptation_count;
    let start = Instant::now();
    fixture
        .transport
        .force_connection_adaptation(&fixture.client.id())
        .await?;
    assert!(start.elapsed() < Duration::from_millis(100));
    assert_eq!(
        fixture.adaptive.adaptation_stats().adaptation_count,
        before + 1
    );
    let start = Instant::now();
    for _ in 0..10 {
        fixture
            .transport
            .force_connection_adaptation(&fixture.client.id())
            .await?;
    }
    assert!(start.elapsed() / 10 < Duration::from_millis(10));
    assert_eq!(
        fixture.adaptive.adaptation_stats().adaptation_count,
        before + 11
    );
    fixture.finish().await;
    Ok(())
}
