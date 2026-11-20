// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shared plain-QUIC example helpers, not STOQ or mesh authorization.
use anyhow::{bail, Context, Result};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const LIMIT: usize = 64 * 1024;
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum Example {
    Basic,
    Working,
}

impl Example {
    fn port(self) -> u16 {
        match self {
            Self::Basic => 5000,
            Self::Working => 5001,
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Basic => 1,
            Self::Working => 3,
        }
    }

    fn request(self, index: usize) -> String {
        match self {
            Self::Basic => "Hello from client!".into(),
            Self::Working => format!("Message #{index} from client"),
        }
    }

    fn response(self, request: &str) -> String {
        match self {
            Self::Basic => "Hello from server!".into(),
            Self::Working => format!("Echo: {request}"),
        }
    }
}

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(Ipv6Addr::LOCALHOST.into(), port)
}

// The caller must control this parent directory and the transfer of its public pin.
// Resolve parents so a symlink into /tmp cannot circumvent the storage restriction.
fn certificate_path(value: &std::ffi::OsStr) -> Result<PathBuf> {
    let path = Path::new(value);
    anyhow::ensure!(path.is_absolute(), "certificate path must be absolute");
    let parent = path.parent().context("certificate path has no parent")?;
    let parent = parent
        .canonicalize()
        .context("certificate parent must exist")?;
    anyhow::ensure!(
        !parent.starts_with("/tmp"),
        "certificate path cannot use /tmp"
    );
    Ok(parent.join(path.file_name().context("certificate filename required")?))
}

fn publish_pin(path: &Path, pin: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("refusing to replace certificate {}", path.display()))?;
    file.write_all(pin)
        .context("write public DER certificate")?;
    file.sync_all().context("sync public DER certificate")?;
    // Never unlink: even an incomplete failed write remains caller-owned.
    Ok(())
}

fn read_pin(path: &Path) -> Result<CertificateDer<'static>> {
    anyhow::ensure!(
        path.symlink_metadata()?.file_type().is_file(),
        "certificate must be a regular file, not a symlink or special file"
    );
    let file = File::open(path).context("open public DER certificate")?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "certificate must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        !bytes.is_empty() && bytes.len() <= LIMIT,
        "invalid certificate size"
    );
    Ok(CertificateDer::from(bytes))
}

fn server(example: Example) -> Result<(Endpoint, CertificateDer<'static>)> {
    let identity = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let pin = identity.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(identity.key_pair.serialize_der());
    let mut config = ServerConfig::with_single_cert(vec![pin.clone()], key.into())?;
    let mut transport = quinn::TransportConfig::default();
    transport.max_concurrent_uni_streams(0_u8.into());
    config.transport_config(Arc::new(transport));
    Ok((Endpoint::server(config, loopback(example.port()))?, pin))
}

fn client(pin: CertificateDer<'static>) -> Result<Endpoint> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(pin).context("invalid DER trust anchor")?;
    let config = ClientConfig::with_root_certificates(Arc::new(roots))?;
    let mut endpoint = Endpoint::client(loopback(0))?;
    endpoint.set_default_client_config(config);
    Ok(endpoint)
}

async fn serve_connection(endpoint: &Endpoint, example: Example) -> Result<()> {
    let incoming = endpoint.accept().await.context("server endpoint closed")?;
    let connection = incoming.await.context("server TLS handshake")?;
    tokio::time::timeout(EXCHANGE_TIMEOUT, serve_streams(&connection, example))
        .await
        .context("server exchange timed out")??;
    Ok(())
}

async fn serve_streams(connection: &Connection, example: Example) -> Result<()> {
    for _ in 0..example.count() {
        let (mut send, mut recv) = connection.accept_bi().await?;
        let request = recv.read_to_end(LIMIT).await?;
        let request = std::str::from_utf8(&request).context("request is not UTF-8")?;
        send.write_all(example.response(request).as_bytes()).await?;
        send.finish()?;
    }
    // The client closes only after validating every full response, preventing truncation.
    match connection.closed().await {
        quinn::ConnectionError::ApplicationClosed(close) if close.error_code == 0_u32.into() => {
            Ok(())
        }
        error => Err(error).context("peer did not complete the exchange"),
    }
}

async fn exchange(endpoint: &Endpoint, example: Example) -> Result<()> {
    let connection = endpoint
        .connect(loopback(example.port()), "localhost")?
        .await
        .context("client TLS handshake against supplied pin")?;
    for index in 1..=example.count() {
        let request = example.request(index);
        let (mut send, mut recv) = connection.open_bi().await?;
        send.write_all(request.as_bytes()).await?;
        send.finish()?;
        let response = recv.read_to_end(LIMIT).await?;
        anyhow::ensure!(
            response == example.response(&request).as_bytes(),
            "unexpected response"
        );
        println!("Received: {}", std::str::from_utf8(&response)?);
    }
    connection.close(0_u32.into(), b"exchange complete");
    Ok(())
}

async fn run_client(pin: CertificateDer<'static>, example: Example) -> Result<()> {
    let endpoint = client(pin)?;
    let result = tokio::time::timeout(EXCHANGE_TIMEOUT, exchange(&endpoint, example))
        .await
        .context("client exchange timed out")?;
    endpoint.close(0_u32.into(), b"client finished");
    endpoint.wait_idle().await;
    result
}

async fn run_server(path: &Path, example: Example) -> Result<()> {
    let (endpoint, pin) = server(example)?;
    publish_pin(path, pin.as_ref())?;
    println!(
        "Listening on {}; public pin {}",
        endpoint.local_addr()?,
        path.display()
    );
    loop {
        serve_connection(&endpoint, example).await?;
        if matches!(example, Example::Basic) {
            endpoint.wait_idle().await;
            return Ok(());
        }
    }
}

async fn run_both(example: Example) -> Result<()> {
    let (endpoint, pin) = server(example)?;
    let result = tokio::try_join!(
        serve_connection(&endpoint, example),
        run_client(pin, example)
    );
    endpoint.close(0_u32.into(), b"both finished");
    endpoint.wait_idle().await;
    result?;
    Ok(())
}

pub async fn run(working: bool) -> Result<()> {
    let example = if working {
        Example::Working
    } else {
        Example::Basic
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [mode] if mode == "both" && matches!(example, Example::Working) => run_both(example).await,
        [mode, path] if mode == "server" => run_server(&certificate_path(path)?, example).await,
        [mode, path] if mode == "client" => run_client(read_pin(&certificate_path(path)?)?, example).await,
        _ => bail!("use server /absolute/new-public.der or client /absolute/public.der; quic_working also accepts both"),
    }
}
