#!/bin/bash

# Meteora Fee Router Devnet Deployment Script
# Simplified version - run from WSL terminal

set -e

SOLANA_PATH="$HOME/.local/share/solana/install/active_release/bin/solana"
ANCHOR_CMD="anchor"

echo "🚀 Starting Meteora Fee Router deployment to Devnet..."
echo ""

# Step 1: Configure Solana for devnet
echo "1️⃣  Configuring Solana CLI for devnet..."
$SOLANA_PATH config set --url devnet
echo "✓ Configured for devnet"
echo ""

# Step 2: Check wallet balance
echo "2️⃣  Checking wallet balance..."
$SOLANA_PATH airdrop 2 2>/dev/null || echo "Note: Airdrop may have failed, but continuing..."
BALANCE=$($SOLANA_PATH balance | awk '{print $1}')
echo "✓ Wallet balance: $BALANCE SOL"
echo ""

# Step 3: Build the program
echo "3️⃣  Building Anchor program..."
$ANCHOR_CMD build
echo "✓ Build complete"
echo ""

# Step 4: Deploy to devnet
echo "4️⃣  Deploying to Solana devnet..."
$ANCHOR_CMD deploy --provider.cluster devnet
echo "✓ Deployment complete"
echo ""

# Step 5: Get program ID
echo "5️⃣  Getting program ID..."
PROGRAM_ID=$($ANCHOR_CMD keys list | grep meteora_fee_router | awk '{print $2}')
echo "✓ Program ID: $PROGRAM_ID"
echo ""

# Display results
echo "═════════════════════════════════════════"
echo "✅ DEPLOYMENT SUCCESSFUL!"
echo "═════════════════════════════════════════"
echo ""
echo "📍 Program Address: $PROGRAM_ID"
echo "🔗 View on Explorer:"
echo "   https://explorer.solana.com/address/$PROGRAM_ID?cluster=devnet"
echo ""
echo "📋 Next Steps:"
echo "   1. Update your code with the program ID"
echo "   2. Run integration tests to verify deployment"
echo "   3. Initialize honorary positions for your vaults"
echo ""
