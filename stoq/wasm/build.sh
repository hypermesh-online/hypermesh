#!/bin/bash
set -e

echo "Building STOQ WebAssembly client..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    source ~/.cargo/env
fi

# Check if required target is installed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build the WASM package
echo "Building WASM package..."
wasm-pack build --target web --out-dir ../ui/frontend/public/wasm --scope stoq

# Copy TypeScript definitions to the frontend lib directory
echo "Copying TypeScript definitions..."
cp ../ui/frontend/public/wasm/stoq_wasm.d.ts ../ui/frontend/lib/stoq-wasm-generated.d.ts

echo "STOQ WebAssembly client built successfully!"
echo "Files generated:"
echo "  - ../ui/frontend/public/wasm/stoq_wasm.js"
echo "  - ../ui/frontend/public/wasm/stoq_wasm_bg.wasm"
echo "  - ../ui/frontend/lib/stoq-wasm-generated.d.ts"

# Show file sizes
echo ""
echo "File sizes:"
ls -lh ../ui/frontend/public/wasm/stoq_wasm_bg.wasm
ls -lh ../ui/frontend/public/wasm/stoq_wasm.js