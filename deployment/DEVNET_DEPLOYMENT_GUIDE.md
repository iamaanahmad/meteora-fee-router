# Meteora Fee Router - Solana Devnet Deployment Guide

## Status: ✅ DEPLOYED TO DEVNET

| Property | Value |
|----------|-------|
| **Program ID** | `6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc` |
| **Deployed Slot** | 429803842 |
| **Owner** | BPFLoaderUpgradeab1e11111111111111111111111 |
| **Upgrade Authority** | `EwrEb3sWWiaz7mAN4XaDiADcjmBL85Eiq6JFVXrKU7En` |
| **Data Length** | 368,544 bytes |
| **Balance** | 2.57 SOL (rent) |
| **Deploy Date** | December 21, 2025 |

### View on Explorer
- **Solana Explorer:** https://explorer.solana.com/address/6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc?cluster=devnet
- **Solscan:** https://solscan.io/account/6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc?cluster=devnet

## Upgrading the Program

### Step 1: Build the program
```bash
cd /mnt/c/Projects/meteora-fee-routing-anchor
anchor build --no-idl
```

### Step 2: Fix ELF section names (if needed)
The Solana 3.x loader requires section names ≤16 bytes. If you get an ELF error, strip problematic sections:
```bash
# Find long section names
strings target/deploy/meteora_fee_router.so | grep -E '^\.data\._ZN'

# Remove the section
~/.local/share/solana/install/active_release/bin/platform-tools-sdk/sbf/dependencies/platform-tools/llvm/bin/llvm-objcopy \
  --remove-section '.data._ZN13solana_pubkey6Pubkey10new_unique1I17h3c1ca50d93a4bbd4E' \
  target/deploy/meteora_fee_router.so \
  target/deploy/meteora_fee_router_stripped.so
```

### Step 3: Deploy upgrade
```bash
solana program deploy --url devnet target/deploy/meteora_fee_router_stripped.so \
  --program-id 6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc
```

### Step 4: Verify deployment
```bash
solana program show 6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc --url devnet
```

## Fresh Deployment (New Program ID)

If you need a new program ID:

### Step 1: Generate new keypair
```bash
solana-keygen new -o target/deploy/meteora_fee_router-keypair.json
```

### Step 2: Update Anchor.toml
```toml
[programs.devnet]
meteora_fee_router = "<NEW_PROGRAM_ID>"
```

### Step 3: Build and deploy
```bash
anchor build --no-idl
# Fix ELF if needed (see above)
solana program deploy --url devnet target/deploy/meteora_fee_router_stripped.so \
  --program-id target/deploy/meteora_fee_router-keypair.json
```

## Troubleshooting

### ELF Error: Section name longer than 16 bytes
This is a known issue with Solana 3.x. Use `llvm-objcopy` to strip or rename the section (see Step 2 above).

### Insufficient funds
```bash
solana airdrop 2 --url devnet
```

### `anchor` command not found
```bash
avm use 0.32.1
# Or use full path: ~/.avm/bin/anchor
```
