#!/bin/bash

# Fix hypermesh_transport imports to use crate::transport
echo "Fixing hypermesh_transport imports..."

# List of files to fix
files=(
    "src/integration/lib.rs"
    "src/integration/config.rs"
    "src/integration/tests/platform_integration_test.rs"
    "src/consensus/detection/quantum_security.rs"
    "src/consensus/lib.rs"
    "src/consensus/detection/isolation.rs"
    "src/consensus/byzantine.rs"
    "src/consensus/detection/mod.rs"
    "src/consensus/sharding.rs"
    "src/consensus/detection/recovery.rs"
    "src/consensus/detection/real_time.rs"
    "src/consensus/detection/reputation.rs"
    "src/consensus/benches/consensus_benchmarks.rs"
    "src/assets/examples/consensus_proof_demo.rs"
    "src/consensus/tests/integration_tests.rs"
    "src/consensus/error.rs"
    "src/consensus/engine.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "Fixing $file"
        # Replace hypermesh_transport with crate::transport
        sed -i 's/use hypermesh_transport::/use crate::transport::/g' "$file"
        sed -i 's/hypermesh_transport::NodeId/crate::transport::NodeId/g' "$file"
        sed -i 's/hypermesh_transport::HyperMeshTransportTrait/crate::transport::HyperMeshTransport/g' "$file"
    fi
done

echo "Done fixing imports"