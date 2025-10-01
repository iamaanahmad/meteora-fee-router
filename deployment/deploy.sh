#!/bin/bash

# Meteora Fee Router Deployment Script
# This script builds and deploys the Meteora Fee Router program

set -e

echo "🚀 Starting Meteora Fee Router deployment..."

# Check if Anchor is installed
if ! command -v anchor &> /dev/null; then
    echo "❌ Anchor CLI not found. Please install Anchor first."
    exit 1
fi

# Check if Solana CLI is installed
if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI not found. Please install Solana CLI first."
    exit 1
fi

# Set default cluster if not specified
CLUSTER=${1:-devnet}

echo "📋 Deployment Configuration:"
echo "  Cluster: $CLUSTER"
echo "  Program ID: $(anchor keys list | grep meteora_fee_router | awk '{print $2}')"

# Build the program
echo "🔨 Building program..."
anchor build

# Deploy to specified cluster
echo "🌐 Deploying to $CLUSTER..."
anchor deploy --provider.cluster $CLUSTER

# Verify deployment
PROGRAM_ID=$(anchor keys list | grep meteora_fee_router | awk '{print $2}')
echo "✅ Deployment complete!"
echo "📍 Program deployed at: $PROGRAM_ID"
echo "🔗 Explorer: https://explorer.solana.com/address/$PROGRAM_ID?cluster=$CLUSTER"

# Generate IDL
echo "📄 Generating IDL..."
anchor idl init --filepath target/idl/meteora_fee_router.json $PROGRAM_ID --provider.cluster $CLUSTER

echo "🎉 Deployment successful!"
echo ""
echo "Next steps:"
echo "1. Update your client code with the new program ID: $PROGRAM_ID"
echo "2. Test the deployment with the integration examples"
echo "3. Initialize honorary positions for your vaults"