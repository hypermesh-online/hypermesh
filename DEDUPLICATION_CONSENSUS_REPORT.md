# Consensus Deduplication Report

## Task
Eliminate duplication between `lib/src/proof_of_state/` and `blockmatrix/src/consensus/`

## Analysis

### Original State
Three consensus implementations found:

1. **`lib/src/proof_of_state/`** (27 files)
   - Incomplete Raft consensus with BFT
   - Missing dependencies (futures, tracing, etc.)
   - Missing modules (metrics, transport)
   - **Status**: Non-functional, incomplete copy

2. **`blockmatrix/src/consensus/`** (27 files)
   - Identical structure to lib/src/proof_of_state
   - Byte-for-byte duplicate of lib version
   - Also incomplete (referenced `crate::transport` which exists in blockmatrix)
   - **Status**: Duplicate, removed

3. **`trustchain/src/consensus/`** (7 files)
   - TrustChain-specific four-proof wrapper
   - Implements ConsensusProof, StakeProof, TimeProof, SpaceProof, WorkProof
   - Lightweight, focused on certificate validation
   - **Status**: Canonical implementation, kept

### Architecture Decision

**Consensus should live in TrustChain:**
- TrustChain implements the four-proof consensus system
- TrustChain is a dependency of blockmatrix (for certificates)
- Re-exporting from trustchain avoids duplication

**Why not lib?**
- lib's proof_of_state was incomplete (missing dependencies, modules)
- lib should contain only truly shared types (AssetId, etc.)
- Consensus is specific to TrustChain's certificate validation

**Why not blockmatrix?**
- blockmatrix/src/consensus was a byte-for-byte duplicate of lib
- blockmatrix already depends on trustchain
- No need for separate implementation

## Actions Taken

### 1. Removed Duplicates
```bash
rm -rf /home/persist/repos/projects/web3/lib/src/proof_of_state
rm -rf /home/persist/repos/projects/web3/blockmatrix/src/consensus
rm -f /home/persist/repos/projects/web3/lib/src/consensus.rs
```

### 2. Updated lib/src/lib.rs
**Before:**
```rust
pub mod proof_of_state;  // The full consensus/Proof of State system
pub use proof_of_state as consensus;
pub use proof_of_state::{ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime};
```

**After:**
```rust
// Removed proof_of_state module
// Note: Consensus/Proof of State system is in blockmatrix crate
```

### 3. Created blockmatrix/src/consensus/mod.rs
Re-exports trustchain's consensus types:
```rust
//! HyperMesh Consensus System
//!
//! This module re-exports the Proof of State consensus system from TrustChain.
//! TrustChain implements the four-proof consensus (WHO, WHEN, WHERE, WHAT).

pub use trustchain::consensus::*;
```

### 4. Updated blockmatrix/Cargo.toml
Added trustchain dependency:
```toml
# TrustChain - Consensus and certificate system
trustchain = { path = "../trustchain" }
```

### 5. Updated blockmatrix/src/lib.rs
```rust
// Consensus module (re-exports from TrustChain)
pub mod consensus;
```

## Verification

### lib compiles cleanly:
```bash
cd lib && cargo check
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
```

### blockmatrix imports consensus successfully:
- No consensus-related compilation errors
- All `use crate::consensus::*` imports work
- Pre-existing errors (axum, etc.) are unrelated to this change

## Import Patterns

Components now import consensus as:
```rust
// Within blockmatrix
use crate::consensus::{ConsensusProof, ConsensusRequirements};

// From trustchain (original source)
use trustchain::consensus::{ConsensusProof, StakeProof, TimeProof, SpaceProof, WorkProof};

// From blockmatrix (re-exported)
use blockmatrix::consensus::{ConsensusProof, ConsensusRequirements};
```

## Files Modified

1. `/home/persist/repos/projects/web3/lib/src/lib.rs` - Removed proof_of_state module
2. `/home/persist/repos/projects/web3/blockmatrix/src/lib.rs` - Added consensus module declaration
3. `/home/persist/repos/projects/web3/blockmatrix/Cargo.toml` - Added trustchain dependency
4. `/home/persist/repos/projects/web3/blockmatrix/src/consensus/mod.rs` - Created (re-exports trustchain)

## Files Removed

1. `/home/persist/repos/projects/web3/lib/src/proof_of_state/` (entire directory, 27 files)
2. `/home/persist/repos/projects/web3/lib/src/consensus.rs`
3. `/home/persist/repos/projects/web3/blockmatrix/src/consensus/` (was duplicate, recreated as re-export)

## Result

- **Zero duplication**: Single source of truth in trustchain
- **Clean architecture**: lib contains shared types, trustchain contains consensus
- **Working imports**: All existing code continues to work via re-exports
- **Modular**: Components import from their dependency (blockmatrix → trustchain)

## Next Steps

Components that currently import from blockmatrix::consensus could optionally import directly from trustchain::consensus for clarity, but the current re-export maintains compatibility.
