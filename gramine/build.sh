#!/bin/bash
# Gramine SGX build script for Aegis-Flow

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🔒 Building Aegis-Flow for Gramine SGX..."

# Build release binary
cd "$PROJECT_DIR"
cargo build --release

# Generate Gramine manifest
cd "$SCRIPT_DIR"
gramine-manifest \
    -Dlog_level=error \
    aegis-proxy.manifest.template \
    aegis-proxy.manifest

# Sign the manifest (generates .sig file)
gramine-sgx-sign \
    --manifest aegis-proxy.manifest \
    --output aegis-proxy.manifest.sgx

echo "✅ Gramine SGX build complete!"
echo ""
echo "To run in simulation mode:"
echo "  gramine-direct ./aegis-proxy"
echo ""
echo "To run in SGX enclave (requires SGX hardware):"
echo "  gramine-sgx ./aegis-proxy"
