# Caesar SDK - Universal Payment Interface

**Status: Alpha | 5 files | ~1K lines | 2 tests | 83% feature-complete**

Caesar SDK defines the Universal Payment Interface (UPI) adapter traits for integrating external payment rails with the Caesar EVP system. It has zero dependencies on Caesar internals -- only `hypermesh-lib` canonical types. Payment providers implement `IngressAdapter` to accept value into Caesar and `EgressAdapter` to withdraw value out.

## Architecture

```
External Rail                    Caesar EVP
  |                                |
  +-> IngressAdapter.lock() ----> CaesPacket minted
  |     (lock value on rail)       |
  |                                +-> route -> transit -> deliver
  |                                |
  +<-- EgressAdapter.settle() <-- Settlement pipeline
        (release value on rail)
```

The SDK is extracted from `caesar/src/upi/` so that adapter authors can depend on traits without pulling in the full Caesar crate.

## Traits

### IngressAdapter (7 methods)

Accepts value from an external rail into Caesar.

| Method | Purpose |
|--------|---------|
| `adapter_id()` | Unique identifier for this adapter |
| `supported_denominations()` | Currencies/assets this adapter handles |
| `lock_external_value()` | Lock value on the external rail |
| `verify_lock()` | Confirm the lock is still valid |
| `release_lock()` | Release a lock (cancellation/expiry) |
| `liquidity_pressure()` | Report current liquidity conditions |
| `to_gold_grams()` | Convert denomination amount to gold grams |

### EgressAdapter (5 methods)

Withdraws value from Caesar to an external rail.

| Method | Purpose |
|--------|---------|
| `adapter_id()` | Unique identifier for this adapter |
| `supported_denominations()` | Currencies/assets this adapter handles |
| `available_capacity()` | Current settlement capacity |
| `settle()` | Execute settlement on the external rail |
| `capacity_ratio()` | Available vs total capacity ratio |

## Types

- `IngressLockProof` -- proof that value is locked on the external rail
- `SettlementReceipt` -- confirmation of completed settlement with finality level
- `SettlementFinality` -- Instant, Delayed, Probabilistic, or Manual
- `LiquidityPressure` -- current liquidity conditions on the external rail
- `UpiError` -- error type for adapter operations

## Reference Implementation

`MeshCreditAdapter` implements both traits for the internal BlockMatrix mesh credit ledger (1:1 CAES denomination). Use it as a reference when building custom adapters.

## Test Utilities

`MockIngressAdapter` and `MockEgressAdapter` are public, so adapter developers can use them in their own test suites.

## Quick Start

```bash
# Build
cargo build -p caesar-sdk

# Run tests
cargo test -p caesar-sdk
```

## Usage

```rust
use caesar_sdk::{IngressAdapter, EgressAdapter, MeshCreditAdapter};
use hypermesh_lib::economic::GoldGrams;

// Use the reference mesh credit adapter
let adapter = MeshCreditAdapter::new(node_id);

// Lock value for ingress
let proof = adapter.lock_external_value(amount, denomination).await?;

// Settle for egress
let receipt = adapter.settle(packet_id, gold_grams, destination).await?;
```

## Dependencies

- `hypermesh-lib` -- canonical EVP types (GoldGrams, PacketId)
- `async-trait` -- async trait definitions
- `rust_decimal` -- precise decimal arithmetic

## License

BSL-1.1
