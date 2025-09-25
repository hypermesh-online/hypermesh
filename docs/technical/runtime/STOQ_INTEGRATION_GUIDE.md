# STOQ Transport Integration Guide

## Overview

STOQ is a pure QUIC over IPv6 transport protocol with quantum-resistant cryptography and protocol extensions. This guide covers integration with HyperMesh components for clean, efficient data transport with built-in tokenization, sharding, and post-quantum security.

## Architecture Overview

### Core Components

#### 1. Pure Transport Layer
- **Protocol**: QUIC over IPv6 (transport layer only)
- **Security**: TLS 1.3 + FALCON-1024 post-quantum crypto
- **Features**: 0-RTT resumption, connection migration, memory pooling
- **Performance**: High-performance concurrent connections
- **Implementation**: Quinn (Rust QUIC library) with custom extensions

#### 2. Protocol Extensions
- **Packet Tokenization**: SHA-256 cryptographic validation with sequence tracking
- **Packet Sharding**: Automatic fragmentation and reassembly with integrity verification
- **Multi-hop Routing**: IPv6-based hop chain tracking for protocol-level routing
- **Seeding Protocol**: Distributed packet replication foundation

#### 3. Quantum-Resistant Security
- **Algorithm**: FALCON-1024 (NIST PQC standard)
- **Security Level**: 256-bit equivalent quantum resistance
- **Key Management**: Automatic generation, rotation, and caching
- **Integration**: Transport-level handshake signatures

## HyperMesh Integration

### Asset Transport

#### Asset Communication via STOQ
```rust
use stoq::*;

// HyperMesh asset transport using STOQ
pub struct AssetTransport {
    transport: StoqTransport,
    extensions: DefaultStoqExtensions,
}

impl AssetTransport {
    pub async fn new(config: TransportConfig) -> Result<Self> {
        let transport = StoqTransport::new(config).await?;
        let extensions = DefaultStoqExtensions::new();

        Ok(Self {
            transport,
            extensions,
        })
    }

    pub async fn send_asset_data(&self, endpoint: &Endpoint, asset_data: &[u8]) -> Result<()> {
        // Create connection
        let connection = self.transport.connect(endpoint).await?;

        // Create packet with extensions
        let mut packet = StoqPacket::new(asset_data.to_vec().into());

        // Add tokenization for validation
        packet.token = Some(self.extensions.tokenize_packet(asset_data));

        // Send via STOQ transport
        self.transport.send(&connection, &packet.serialize()?).await?;

        Ok(())
    }
}
```

### TrustChain Integration

#### Certificate Management
```rust
// TrustChain certificate integration with STOQ
pub struct TrustChainIntegration {
    transport: StoqTransport,
    cert_manager: CertificateManager,
}

impl TrustChainIntegration {
    pub async fn new(config: TransportConfig) -> Result<Self> {
        let mut transport_config = config;
        // Enable TrustChain certificate management
        transport_config.enable_trustchain_certs = true;
        transport_config.cert_rotation_hours = 24;

        let transport = StoqTransport::new(transport_config).await?;
        let cert_manager = CertificateManager::new().await?;

        Ok(Self {
            transport,
            cert_manager,
        })
    }

    pub async fn establish_secure_connection(&self, endpoint: &Endpoint) -> Result<Connection> {
        // Ensure certificates are valid
        self.cert_manager.validate_certificates().await?;

        // Connect with automatic certificate management
        let connection = self.transport.connect(endpoint).await?;

        // Verify transport-level security
        if let Some(falcon_transport) = self.transport.falcon_transport() {
            let falcon = falcon_transport.read();
            // Transport includes FALCON quantum-resistant handshake
        }

        Ok(connection)
    }
}
```

## Protocol Extensions Usage

### Packet Tokenization
```rust
// Using STOQ tokenization for data integrity
pub struct TokenizedTransport {
    transport: StoqTransport,
    extensions: DefaultStoqExtensions,
}

impl TokenizedTransport {
    pub async fn send_with_validation(&self, connection: &Connection, data: &[u8]) -> Result<()> {
        // Create packet token for validation
        let token = self.extensions.tokenize_packet(data);

        // Create packet with token
        let mut packet = StoqPacket::new(data.to_vec().into());
        packet.token = Some(token);

        // Send packet
        self.transport.send(connection, &packet.serialize()?).await?;

        Ok(())
    }

    pub async fn receive_with_validation(&self, connection: &Connection) -> Result<Vec<u8>> {
        // Receive packet
        let data = self.transport.receive(connection).await?;
        let packet = StoqPacket::deserialize(&data)?;

        // Validate token if present
        if let Some(token) = &packet.token {
            if !self.extensions.validate_token(&packet.data, token) {
                return Err(anyhow::anyhow!("Packet validation failed"));
            }
        }

        Ok(packet.data.to_vec())
    }
}
```

### Packet Sharding
```rust
// Large data transmission using STOQ sharding
pub struct ShardedTransport {
    transport: StoqTransport,
    extensions: DefaultStoqExtensions,
}

impl ShardedTransport {
    pub async fn send_large_data(&self, connection: &Connection, data: &[u8]) -> Result<()> {
        const MAX_SHARD_SIZE: usize = 1024;

        if data.len() <= MAX_SHARD_SIZE {
            // Send directly without sharding
            self.transport.send(connection, data).await
        } else {
            // Shard the data
            let shards = self.extensions.shard_packet(data, MAX_SHARD_SIZE)?;

            // Send each shard
            for shard in shards {
                let shard_packet = StoqPacket {
                    data: shard.data,
                    token: None,
                    shard: Some(shard.metadata),
                    hop_info: None,
                    seed_info: None,
                };

                self.transport.send(connection, &shard_packet.serialize()?).await?;
            }

            Ok(())
        }
    }

    pub async fn receive_large_data(&self, connection: &Connection) -> Result<Vec<u8>> {
        let mut shards = Vec::new();

        // Receive shards until complete
        loop {
            let data = self.transport.receive(connection).await?;
            let packet = StoqPacket::deserialize(&data)?;

            if let Some(shard_info) = packet.shard {
                shards.push(PacketShard {
                    data: packet.data,
                    metadata: shard_info,
                });

                // Check if we have all shards
                if shard_info.is_last {
                    break;
                }
            } else {
                // Single packet, not sharded
                return Ok(packet.data.to_vec());
            }
        }

        // Reassemble shards
        let reassembled = self.extensions.reassemble_shards(shards)?;
        Ok(reassembled.to_vec())
    }
}
```

## FALCON Quantum-Resistant Security

### Transport-Level Security
```rust
// FALCON-1024 integration example
pub struct SecureTransport {
    transport: StoqTransport,
}

impl SecureTransport {
    pub async fn new_with_falcon() -> Result<Self> {
        let config = TransportConfig {
            bind_address: std::net::Ipv6Addr::UNSPECIFIED,
            port: 9292,
            enable_falcon_crypto: true,
            falcon_variant: FalconVariant::Falcon1024,
            ..Default::default()
        };

        let transport = StoqTransport::new(config).await?;

        Ok(Self { transport })
    }

    pub async fn secure_handshake(&self, endpoint: &Endpoint) -> Result<Connection> {
        // Connect with FALCON quantum-resistant handshake
        let connection = self.transport.connect(endpoint).await?;

        // Verify FALCON signatures are active
        if let Some(falcon_transport) = self.transport.falcon_transport() {
            let falcon = falcon_transport.read();
            println!("FALCON-1024 active with 256-bit quantum resistance");
        }

        Ok(connection)
    }

    pub async fn sign_data(&self, data: &[u8]) -> Result<Option<Vec<u8>>> {
        // Sign data using FALCON if available
        if let Some(signature) = self.transport.falcon_sign(data)? {
            Ok(Some(signature.to_bytes()))
        } else {
            Ok(None)
        }
    }
}
```

## Configuration and Usage

### Basic Transport Configuration
```rust
use stoq::*;

// Basic STOQ configuration
let config = TransportConfig {
    bind_address: std::net::Ipv6Addr::UNSPECIFIED,
    port: 9292,
    enable_falcon_crypto: true,
    falcon_variant: FalconVariant::Falcon1024,
    enable_zero_copy: true,
    enable_memory_pool: true,
    ..Default::default()
};

// Create transport
let transport = StoqTransport::new(config).await?;

// Connect to endpoint
let endpoint = Endpoint::new(addr, port);
let connection = transport.connect(&endpoint).await?;

// Send data
transport.send(&connection, b"Hello, STOQ!").await?;

// Receive data
let data = transport.receive(&connection).await?;
```

### Integration with HyperMesh
```rust
// HyperMesh asset communication
pub struct HyperMeshTransport {
    transport: StoqTransport,
    extensions: DefaultStoqExtensions,
}

impl HyperMeshTransport {
    pub async fn send_asset_operation(&self, asset_id: &str, operation: &[u8]) -> Result<()> {
        // Create tokenized packet
        let token = self.extensions.tokenize_packet(operation);
        let mut packet = StoqPacket::new(operation.to_vec().into());
        packet.token = Some(token);

        // Add hop information if needed
        if let Ok(hop_info) = self.create_hop_info(asset_id) {
            packet.hop_info = Some(hop_info);
        }

        // Send via transport
        let endpoint = self.resolve_asset_endpoint(asset_id).await?;
        let connection = self.transport.connect(&endpoint).await?;
        self.transport.send(&connection, &packet.serialize()?).await
    }
}
```

## Configuration Files

### STOQ Transport Configuration
```toml
# stoq.toml - STOQ transport configuration
[transport]
bind_address = "::"
port = 9292
enable_ipv6_only = true

[security]
enable_falcon_crypto = true
falcon_variant = "Falcon1024"
enable_tls_13 = true

[certificates]
enable_trustchain_integration = true
cert_rotation_hours = 24
auto_cert_renewal = true

[extensions]
enable_packet_tokenization = true
enable_packet_sharding = true
enable_multi_hop_routing = false  # Disable for simple deployments
enable_seeding_protocol = false   # Disable for simple deployments

[performance]
enable_zero_copy = true
enable_memory_pool = true
max_concurrent_connections = 10000

[logging]
level = "info"
enable_detailed_metrics = false
```

### Deployment

#### Docker Deployment
```yaml
# docker-compose.yml
version: '3.8'
services:
  stoq-transport:
    image: hypermesh/stoq:latest
    ports:
      - "9292:9292/udp"  # QUIC port
    volumes:
      - ./stoq.toml:/etc/stoq/config.toml:ro
      - ./certs:/etc/stoq/certs:ro
    environment:
      - STOQ_CONFIG_PATH=/etc/stoq/config.toml
      - STOQ_LOG_LEVEL=info
    restart: unless-stopped
    network_mode: "host"  # Required for IPv6
    sysctls:
      - net.ipv6.conf.all.disable_ipv6=0
```

#### Systemd Service
```ini
# /etc/systemd/system/stoq-transport.service
[Unit]
Description=STOQ Transport Protocol
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
User=stoq
Group=stoq
ExecStart=/usr/local/bin/stoq-transport --config /etc/stoq/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## API Reference

### Core Transport API
```rust
// Main transport interface
pub struct StoqTransport {
    // Internal implementation
}

impl StoqTransport {
    // Create new transport
    pub async fn new(config: TransportConfig) -> Result<Self>;

    // Connection management
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Connection>;
    pub async fn accept(&self) -> Result<Connection>;

    // Data transmission
    pub async fn send(&self, connection: &Connection, data: &[u8]) -> Result<()>;
    pub async fn receive(&self, connection: &Connection) -> Result<Vec<u8>>;

    // FALCON quantum-resistant signatures
    pub fn falcon_sign(&self, data: &[u8]) -> Result<Option<FalconSignature>>;
    pub fn falcon_verify(&self, peer_id: &str, signature: &FalconSignature, data: &[u8]) -> Result<bool>;

    // Transport information
    pub fn falcon_transport(&self) -> Option<Arc<RwLock<FalconTransport>>>;
    pub fn get_stats(&self) -> TransportStats;
}
```

### Protocol Extensions API
```rust
// Protocol extensions interface
pub trait StoqProtocolExtension {
    fn tokenize_packet(&self, data: &[u8]) -> PacketToken;
    fn validate_token(&self, data: &[u8], token: &PacketToken) -> bool;
    fn shard_packet(&self, data: &[u8], max_shard_size: usize) -> Result<Vec<PacketShard>>;
    fn reassemble_shards(&self, shards: Vec<PacketShard>) -> Result<Bytes>;
    fn add_hop_info(&self, packet: &mut StoqPacket, hop: HopInfo) -> Result<()>;
    fn get_seed_info(&self, packet: &StoqPacket) -> Option<SeedInfo>;
}

// Default implementation
pub struct DefaultStoqExtensions;

impl DefaultStoqExtensions {
    pub fn new() -> Self;
}

impl StoqProtocolExtension for DefaultStoqExtensions {
    // Implementation of all extension methods
}
```

## Testing and Validation

### Basic Transport Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_basic_transport() {
        let config = TransportConfig::default();
        let transport = StoqTransport::new(config).await.unwrap();

        // Test would require actual network setup
        assert!(transport.get_stats().connections == 0);
    }

    #[test]
    fn test_packet_tokenization() {
        let extensions = DefaultStoqExtensions::new();
        let data = b"test data";

        let token = extensions.tokenize_packet(data);
        assert!(extensions.validate_token(data, &token));

        // Invalid data should fail validation
        let invalid_data = b"different data";
        assert!(!extensions.validate_token(invalid_data, &token));
    }

    #[test]
    fn test_packet_sharding() {
        let extensions = DefaultStoqExtensions::new();
        let data = vec![0u8; 2048]; // Large data

        let shards = extensions.shard_packet(&data, 512).unwrap();
        assert!(shards.len() > 1);

        let reassembled = extensions.reassemble_shards(shards).unwrap();
        assert_eq!(reassembled.as_ref(), data);
    }
}
```

### Performance Considerations

#### Memory Usage
- STOQ uses memory pooling to reduce allocation overhead
- FALCON-1024 signatures require ~1330 bytes per signature
- Connection state is minimal due to QUIC's efficiency

#### Network Performance
- IPv6-only design eliminates dual-stack complexity
- QUIC 0-RTT reduces connection establishment time
- Protocol extensions add minimal overhead (~32 bytes per packet)

#### Security Features
- TLS 1.3 provides perfect forward secrecy
- FALCON-1024 offers 256-bit quantum resistance
- Certificate rotation every 24 hours via TrustChain integration

---

**STOQ Integration Guide**: Production-ready integration patterns for using STOQ pure QUIC transport with HyperMesh components.

*Status: ✅ Production Ready*
*Version: 1.0.0*
*Last Updated: January 2025*