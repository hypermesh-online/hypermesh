#!/bin/bash
# HyperMesh Quick Fix Script - Addresses immediate compilation issues

set -e

echo "Starting HyperMesh Quick Fix..."

# Navigate to project root
cd /home/persist/repos/projects/web3/hypermesh

echo "Step 1: Fixing dependency versions..."

# Fix Quinn version conflicts (downgrade to 0.10 for API compatibility)
find . -name "Cargo.toml" -type f -exec sed -i 's/quinn = "0\.11"/quinn = "0.10"/g' {} \;

# Fix rustls version to match quinn 0.10
find . -name "Cargo.toml" -type f -exec sed -i 's/rustls = "0\.23"/rustls = "0.21"/g' {} \;
find . -name "Cargo.toml" -type f -exec sed -i 's/rustls-pki-types = "1\.0"/rustls = "0.21"/g' {} \;
find . -name "Cargo.toml" -type f -exec sed -i 's/rustls-pemfile = "2\.0"/rustls-pemfile = "1.0"/g' {} \;
find . -name "Cargo.toml" -type f -exec sed -i 's/rcgen = "0\.13"/rcgen = "0.11"/g' {} \;

# Fix ring version for compatibility
find . -name "Cargo.toml" -type f -exec sed -i 's/ring = "0\.17"/ring = "0.16"/g' {} \;

# Standardize rocksdb version
find . -name "Cargo.toml" -type f -exec sed -i 's/rocksdb = "0\.22"/rocksdb = "0.21"/g' {} \;

echo "Step 2: Removing problematic ML dependencies from non-ML modules..."

# Remove candle-core from modules that don't need it
for module in src/transport src/consensus src/orchestration src/container src/security; do
    if [ -f "$module/Cargo.toml" ]; then
        sed -i '/candle-core/d' "$module/Cargo.toml"
        sed -i '/candle-nn/d' "$module/Cargo.toml"
        sed -i '/tch/d' "$module/Cargo.toml"
    fi
done

echo "Step 3: Adding missing dependencies..."

# Add uuid and crc32fast to transport if not present
if ! grep -q "uuid" src/transport/Cargo.toml; then
    echo "" >> src/transport/Cargo.toml
    echo "# Additional dependencies" >> src/transport/Cargo.toml
    echo 'uuid = { version = "1.0", features = ["v4"] }' >> src/transport/Cargo.toml
    echo 'crc32fast = "1.3"' >> src/transport/Cargo.toml
fi

echo "Step 4: Cleaning build cache..."
cargo clean

echo "Step 5: Attempting incremental module builds..."

# Try to build core modules one by one
MODULES=(
    "hypermesh-core"
    "stoq"
    "hypermesh-transport"
    "hypermesh-consensus"
    "hypermesh-assets"
    "hypermesh-orchestration"
)

for module in "${MODULES[@]}"; do
    echo ""
    echo "Building $module..."
    if cargo build -p "$module" 2>&1 | grep -q "Compiling"; then
        echo "  ✅ $module build started"
    else
        echo "  ❌ $module failed to build"
    fi
done

echo ""
echo "Quick fix complete!"
echo ""
echo "Next steps:"
echo "1. Review any remaining compilation errors"
echo "2. Fix Quinn API usage (ClientConfig methods have changed)"
echo "3. Update async/await patterns for new tokio version"
echo "4. Implement Nova/Vulkan GPU abstraction"
echo ""
echo "For detailed plan, see: REMEDIATION_PLAN.md"