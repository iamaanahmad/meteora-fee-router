# Solana Devnet Deployment - Manual Steps

## Current Status
✅ Program compiled: `/mnt/c/Projects/meteora-fee-routing-anchor/target/deploy/meteora_fee_router.so`  
✅ Keypair created: `/mnt/c/Projects/meteora-fee-routing-anchor/target/deploy/meteora_fee_router-keypair.json`  
✅ Wallet funded: 10.38 SOL on devnet  
⚠️ Program ID: `6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc` (from local keys, not yet deployed)

## Issue Identified
The compiled .so file has symbol name length issues with Solana CLI. Need to use `cargo-build-sbf` for proper compilation.

## Solution: Recompile with SBF

Run these commands in WSL:

```bash
# 1. Add SBF target
rustup target add sbf-solana-solana

# 2. Build with SBF toolchain
cargo build-sbf --manifest-path programs/meteora-fee-router/Cargo.toml

# 3. Check the output
ls -la target/sbf-solana-solana/release/

# 4. Deploy the SBF binary
solana program deploy target/sbf-solana-solana/release/meteora_fee_router.so \
  --url devnet \
  --keypair ~/.config/solana/id.json

# 5. Verify deployment
solana program show <PROGRAM_ID> --url devnet
```

## Alternative: Use Anchor Deploy with Working Environment

If anchor is properly installed globally:

```bash
cd /mnt/c/Projects/meteora-fee-routing-anchor

# Set cluster to devnet
anchor deploy --provider.cluster devnet

# Get program ID
anchor keys list
```

## Next Steps

1. Choose one approach above
2. Run the deployment command
3. Save the program ID returned
4. Update Anchor.toml with the actual deployed program ID
5. Verify on: https://solscan.io/account/<PROGRAM_ID>?cluster=devnet

## Example Output Expected
```
Program deployed successfully.
Program Id: <YOUR_PROGRAM_ID>
```

## Troubleshooting

If you see "ELF error" - use SBF compilation instead of regular Rust compilation.

If anchor command not found - reinstall with:
```bash
npm install -g @coral-xyz/anchor-cli@0.29.0
```
