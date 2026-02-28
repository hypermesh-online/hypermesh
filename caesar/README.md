# Caesar - Gold-Denominated Ephemeral Value Protocol

**Status: Alpha | 37 files | ~12.7K lines | 220 tests | 94% feature-complete**

Caesar implements the Ephemeral Value Protocol (EVP) for the HyperMesh ecosystem. Value is denominated in gold grams, transmitted as short-lived packets with built-in demurrage, and settled through a conservation-law-enforced pipeline. A PID-based Governor adjusts network fees in real time based on velocity, volume, liquidity, and in-transit float.

## Architecture

```
Mint -> Route (Governor-adjusted fees) -> Transit -> Deliver -> Settle
  |                                                              |
  +-> Hold (congestion/budget)                                   +-> Fee distribution (80/20 egress/transit)
  +-> Expire (TTL) -> Refund                                     +-> Conservation check (Input = Output + Fees + Demurrage)
  +-> Dissolve (90-day gravity)
```

**Conservation invariant**: `Input = Output + Fees + Demurrage`. Circuit breaker halts the system if conservation is violated.

## Key Features

- **CaesPacket state machine** -- 11 states: Minted, InTransit, Delivered, Settling, Settled, Held, Stalled, Expired, Refunded, Dissolved, Dispersed
- **Governor PID controller** -- adjusts fees based on velocity, gold deviation, volume, liquidity, and in-transit float; constitutional fee caps per tier (L0=5%, L1=2%, L2=0.5%, L3=0.1%)
- **Tier-based demurrage** -- L0 ~5%/hr, L1 ~0.1%/day, L2 ~0.01%/day, L3 ~0.001%/day; encourages velocity
- **Settlement pipeline** -- validate -> egress adapter -> fee distribution -> conservation check
- **Capacity-based routing** -- bandwidth/buffer/latency/load scoring with operator preferences (tier weights, value range, auto mode)
- **Gold oracle** -- pluggable OracleFeed trait, ManualFeed for alpha, 10% band validation, grams-to-USD conversion
- **Gravity dissolution** -- 90-day timeout, 6-criteria qualification, weighted shard-holder distribution
- **Universal Payment Interface** -- IngressAdapter + EgressAdapter traits (extracted to caesar-sdk)
- **STOQ API** -- 5 handlers: route_packet, node_status, governor_params, effective_rate, health
- **Engauge integration** -- feature-gated: fee distribution signals, Governor throttle, capacity-based routing

## Modules

| Module | Purpose |
|--------|---------|
| `evp` | CaesPacket, PacketState machine, core EVP types |
| `governor` | PID controller, fee caps, network pressure classification |
| `settlement` | AcceptanceCriteria, settlement execution, dispersed retry |
| `conservation` | Conservation law verification, circuit breaker |
| `routing` | Capacity scoring, operator preferences, Governor-adjusted fees |
| `gold_oracle` | OracleFeed trait, ManualFeed, price band validation |
| `fee_distribution` | 80/20 egress/transit split, bytes-weighted transit allocation |
| `packet_processor` | Validate, handoff with hop/fee tracking, batch processing |
| `holding` | Orbit buffer for congestion/budget-exceeded packets |
| `storage` | PacketRecord, SettlementRecord, metrics, JSON persistence |
| `protocol` | CaesarProtocol coordinator (orchestrates all modules) |
| `upi` | Thin re-exports from caesar-sdk (IngressAdapter, EgressAdapter) |
| `api/stoq_api` | STOQ API handlers wired to CaesarProtocol |
| `cli` | Packet operations, node management commands |

## Quick Start

```bash
# Build
cargo build -p caesar --release

# Run tests
cargo test -p caesar

# Run the STOQ API server
cargo run -p caesar --release
```

## Usage

```rust
use caesar::protocol::CaesarProtocol;
use caesar::evp::packet::CaesPacket;
use hypermesh_lib::economic::{GoldGrams, MarketTier};

// Initialize protocol coordinator
let protocol = CaesarProtocol::new();

// Mint a packet
let packet = protocol.mint(
    GoldGrams::new(1.5),
    MarketTier::L1,
    sender_id,
    recipient_id,
)?;

// Route with Governor-adjusted fees
let routed = protocol.route(packet)?;
```

## Dependencies

- `hypermesh-lib` -- canonical EVP types (PacketId, GoldGrams, MarketTier, PacketState)
- `caesar-sdk` -- UPI adapter traits (IngressAdapter, EgressAdapter)
- `rust_decimal` -- precise decimal arithmetic for gold gram calculations
- `tokio` -- async runtime

## License

MIT OR Apache-2.0
