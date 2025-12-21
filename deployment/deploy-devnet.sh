#!/bin/bash

# Meteora Fee Router Devnet Deployment Script
# Run this from WSL: bash deployment/deploy-devnet.sh

set -e

# Set up PATH for Solana and Node
export PATH=$HOME/.local/share/solana/install/active_release/bin:$HOME/.nvm/versions/node/*/bin:$PATH

echo "🚀 Starting Meteora Fee Router deployment to Devnet..."

# Configure Solana to use devnet
echo "📋 Configuring Solana CLI for devnet..."
~/.local/share/solana/install/active_release/bin/solana config set --url devnet
~/.local/share/solana/install/active_release/bin/solana config get

# Check balance
echo ""
echo "💰 Checking wallet balance..."
BALANCE_OUTPUT=$(~/.local/share/solana/install/active_release/bin/solana balance)
# Extract just the number part
BALANCE=$(echo $BALANCE_OUTPUT | awk '{print $1}')
echo "Balance: $BALANCE SOL"

# Check if balance is sufficient (at least 1 SOL)
if [ $(echo "$BALANCE < 1" | bc 2>/dev/null) -eq 1 ] 2>/dev/null; then
    echo "❌ Insufficient balance. Need at least 1 SOL for deployment."
    exit 1
fi

# Build the program
echo ""
echo "🔨 Building program..."
anchor build

# Get program ID
echo ""
echo "📍 Getting program ID..."
PROGRAM_ID=$(anchor keys list | grep meteora_fee_router | awk '{print $2}')
echo "Program ID: $PROGRAM_ID"

# Deploy to devnet
echo ""
echo "🌐 Deploying to devnet..."
anchor deploy --provider.cluster devnet

# Verify deployment
echo ""
echo "✅ Deployment complete!"
echo "📍 Program deployed at: $PROGRAM_ID"
echo "🔗 Explorer: https://explorer.solana.com/address/$PROGRAM_ID?cluster=devnet"

# Generate/update IDL
echo ""
echo "📄 Generating IDL..."
anchor idl init --filepath target/idl/meteora_fee_router.json $PROGRAM_ID --provider.cluster devnet 2>/dev/null || true

echo ""
echo "🎉 Deployment successful!"
echo ""
echo "Next steps:"
echo "1. Update your client code with the program ID: $PROGRAM_ID"
echo "2. Test the deployment with integration tests"
echo "3. Initialize honorary positions for your vaults"
