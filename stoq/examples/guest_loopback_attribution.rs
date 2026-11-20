// Guest-only source-bound STOQ/QUIC echo fixture; no public network or PoS claims.
use anyhow::{bail, Result};
use std::io::{Read, Write};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::Duration;
use stoq::transport::{Endpoint, StoqTransport, TransportConfig};
use tokio::time::timeout;

fn stage(label: &str) -> Result<()> {
    println!("STOQ_STAGE:{label}");
    std::io::stdout().flush()?;
    let mut release = [0u8; 1];
    std::io::stdin().read_exact(&mut release)?;
    if release[0] != b'+' {
        bail!("bad stage release");
    }
    Ok(())
}

fn config(port: u16) -> TransportConfig {
    TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port,
        enable_kernel_acceleration: false,
        ..TransportConfig::default()
    }
}

async fn echo_flow(port: u16, prefix: &str) -> Result<()> {
    let server = Arc::new(StoqTransport::new(config(port)).await?);
    let server_ref = Arc::clone(&server);
    let server_task = tokio::spawn(async move {
        let conn = server_ref.accept().await?;
        let mut stream = conn.accept_stream().await?;
        let data = stream.receive().await?;
        stream.send(&data).await?;
        Ok::<Vec<u8>, anyhow::Error>(data.to_vec())
    });
    let client = StoqTransport::new(config(0)).await?;
    let conn = timeout(
        Duration::from_secs(8),
        client.connect(&Endpoint::new(Ipv6Addr::LOCALHOST, port)),
    )
    .await??;
    stage(&format!("{prefix}_HANDSHAKE"))?;
    let mut stream = conn.open_stream().await?;
    let mut payload = b"fresh-guest-stoq-payload:".to_vec();
    payload.extend_from_slice(&rand::random::<[u8; 16]>());
    stream.send(&payload).await?;
    let echoed = timeout(Duration::from_secs(8), stream.receive()).await??;
    let accepted = timeout(Duration::from_secs(8), server_task).await???;
    if echoed.as_ref() != payload || accepted != payload {
        bail!("roundtrip mismatch");
    }
    println!(
        "STOQ_ROUNDTRIP:{prefix}:bytes={}:blake3={}",
        payload.len(),
        blake3::hash(&payload)
    );
    stage(&format!("{prefix}_PAYLOAD"))?;
    client.shutdown().await;
    server.shutdown().await;
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    echo_flow(9292, "ON").await?;
    echo_flow(9293, "OFF").await?;
    println!("STOQ_RESULT:PASS");
    Ok(())
}
