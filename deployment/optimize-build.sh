#!/bin/bash

# Meteora Fee Router Build Optimization Script
# This script optimizes the program for production deployment

set -e

echo "🔧 Optimizing Meteora Fee Router build..."

# Clean previous builds
echo "🧹 Cleaning previous builds..."
anchor clean
cargo clean --manifest-path programs/meteora-fee-router/Cargo.toml

# Build with optimizations
echo "🚀 Building with release optimizations..."
anchor build --release

# Check program size
PROGRAM_SIZE=$(wc -c < target/deploy/meteora_fee_router.so)
echo "📏 Program size: $PROGRAM_SIZE bytes"

if [ $PROGRAM_SIZE -gt 1048576 ]; then
    echo "⚠️  Warning: Program size ($PROGRAM_SIZE bytes) exceeds 1MB limit"
    echo "   Consider further optimizations or code reduction"
else
    echo "✅ Program size is within acceptable limits"
fi

# Verify program structure
echo "🔍 Verifying program structure..."
if [ -f "target/deploy/meteora_fee_router.so" ]; then
    echo "✅ Program binary generated successfully"
else
    echo "❌ Program binary not found"
    exit 1
fi

if [ -f "target/idl/meteora_fee_router.json" ]; then
    echo "✅ IDL generated successfully"
else
    echo "❌ IDL not found"
    exit 1
fi

# Check for security issues
echo "🔒 Running security checks..."
cargo audit --file programs/meteora-fee-router/Cargo.lock || echo "⚠️  Audit warnings found - review before deployment"

# Compute budget analysis
echo "💰 Analyzing compute usage..."
echo "   Run 'anchor test' to get detailed compute usage statistics"

echo "🎉 Build optimization complete!"
echo ""
echo "Optimization results:"
echo "  Program size: $PROGRAM_SIZE bytes"
echo "  Binary: target/deploy/meteora_fee_router.so"
echo "  IDL: target/idl/meteora_fee_router.json"
echo ""
echo "Ready for deployment! Run './deploy.sh [cluster]' to deploy."