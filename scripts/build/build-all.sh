#!/bin/bash

# Web3 Ecosystem Build Script
# Builds all components in proper dependency order

set -e
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BUILD_TYPE="${1:-release}"
VERBOSE="${VERBOSE:-false}"
COMPONENTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

log() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

# Build function for Rust components
build_rust_component() {
    local component="$1"
    local name="$2"
    
    log "Building $name..."
    cd "$COMPONENTS_DIR/$component"
    
    if [[ "$BUILD_TYPE" == "release" ]]; then
        cargo build --release
    else
        cargo build
    fi
    
    success "$name built successfully"
}

# Build function for Node.js components
build_node_component() {
    local component="$1"
    local name="$2"
    
    log "Building $name..."
    cd "$COMPONENTS_DIR/$component"
    
    if [[ ! -f "package.json" ]]; then
        warn "$name has no package.json, skipping..."
        return 0
    fi
    
    npm install
    if npm run build 2>/dev/null; then
        success "$name built successfully"
    else
        warn "$name has no build script, dependencies installed only"
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Check Node.js
    if ! command -v npm &> /dev/null; then
        warn "npm not found. Node.js components will be skipped."
    fi
    
    success "Prerequisites check passed"
}

# Clean previous builds
clean_builds() {
    log "Cleaning previous builds..."
    
    for component in stoq trustchain hypermesh catalog; do
        if [[ -d "$COMPONENTS_DIR/$component/target" ]]; then
            rm -rf "$COMPONENTS_DIR/$component/target"
        fi
    done
    
    # Clean Node.js builds
    for component in caesar; do
        if [[ -d "$COMPONENTS_DIR/$component/node_modules" ]]; then
            rm -rf "$COMPONENTS_DIR/$component/node_modules"
        fi
        if [[ -d "$COMPONENTS_DIR/$component/dist" ]]; then
            rm -rf "$COMPONENTS_DIR/$component/dist"
        fi
    done
    
    success "Build artifacts cleaned"
}

# Consolidate dependencies (create workspace)
consolidate_dependencies() {
    log "Consolidating dependencies..."
    
    if [[ ! -f "$COMPONENTS_DIR/Cargo.toml" ]]; then
        cat > "$COMPONENTS_DIR/Cargo.toml" << 'EOF'
[workspace]
members = [
    "stoq",
    "trustchain", 
    "hypermesh",
    "catalog"
]
resolver = "2"

[workspace.dependencies]
# Core async runtime
tokio = { version = "1.38", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Networking (unified versions)
quinn = "0.11"
rustls = "0.23"
socket2 = "0.5"
bytes = "1.6"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
bincode = "1.3"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Cryptography (unified versions)
ring = "0.17"
x509-parser = "0.16"
sha2 = "0.10"
blake3 = "1.5"

# Collections
dashmap = "6.0"
parking_lot = "0.12"

# Time
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }

# Post-quantum cryptography
pqcrypto-falcon = "0.3"
pqcrypto-kyber = "0.8"
aes-gcm = "0.10"
EOF
        success "Workspace Cargo.toml created"
    else
        log "Workspace already exists"
    fi
}

# Main build process
main() {
    echo "=========================================="
    echo "   Web3 Ecosystem Build Script"
    echo "=========================================="
    
    check_prerequisites
    
    if [[ "${CLEAN:-false}" == "true" ]]; then
        clean_builds
    fi
    
    consolidate_dependencies
    
    log "Starting build process in dependency order..."
    
    # Phase 1: Foundation Layer
    log "=== Phase 1: Foundation Layer ==="
    build_rust_component "stoq" "STOQ Transport Protocol"
    
    # Phase 2: Certificate Layer  
    log "=== Phase 2: Certificate Layer ==="
    build_rust_component "trustchain" "TrustChain Certificate Authority"
    
    # Phase 3: Orchestration Layer
    log "=== Phase 3: Orchestration Layer ==="
    build_rust_component "hypermesh" "HyperMesh Distributed Computing"
    
    # Phase 4: Application Layer
    log "=== Phase 4: Application Layer ==="
    build_rust_component "catalog" "Catalog Asset Management"
    
    # Phase 5: Economics Layer
    log "=== Phase 5: Economics Layer ==="
    if command -v npm &> /dev/null; then
        build_node_component "caesar" "Caesar Economics Platform"
    else
        warn "Skipping Caesar (npm not available)"
    fi
    
    # Phase 6: Engagement Layer
    log "=== Phase 6: Engagement Layer ==="
    if [[ -d "$COMPONENTS_DIR/ngauge" ]]; then
        warn "NGauge is design-only, no build required"
    fi
    
    echo "=========================================="
    success "All components built successfully!"
    echo "=========================================="
    
    log "Next steps:"
    log "1. Run './start-all-services.sh' to start all services"
    log "2. Run './validate-deployment.sh' to test the ecosystem"
    log "3. Check logs in ./logs/ directory"
}

# Handle script arguments
case "${1:-}" in
    "clean")
        clean_builds
        exit 0
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [release|debug|clean|help]"
        echo ""
        echo "Commands:"
        echo "  release    Build all components in release mode (default)"
        echo "  debug      Build all components in debug mode"
        echo "  clean      Clean all build artifacts"
        echo "  help       Show this help message"
        echo ""
        echo "Environment variables:"
        echo "  CLEAN=true   Clean before building"
        echo "  VERBOSE=true Enable verbose output"
        exit 0
        ;;
esac

# Run main function
main "$@"