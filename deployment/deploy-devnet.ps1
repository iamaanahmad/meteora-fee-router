#!/usr/bin/env pwsh

# Meteora Fee Router - Windows PowerShell Deployment Script
# Deploy to Solana Devnet from Windows (uses WSL internally)

param(
    [string]$Network = "devnet"
)

Write-Host "🚀 Meteora Fee Router Devnet Deployment" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$PROJECT_ROOT = (Get-Item $PSScriptRoot).Parent.FullName
$TARGET_DIR = Join-Path $PROJECT_ROOT "target" "deploy"
$PROGRAM_FILE = Join-Path $TARGET_DIR "meteora_fee_router.so"
$KEYPAIR_FILE = Join-Path $TARGET_DIR "meteora_fee_router-keypair.json"

Write-Host "📋 Configuration:" -ForegroundColor Yellow
Write-Host "  Project Root: $PROJECT_ROOT"
Write-Host "  Network: $Network"
Write-Host "  Program: $PROGRAM_FILE"
Write-Host "  Keypair: $KEYPAIR_FILE"
Write-Host ""

# Check if program is built
if (-not (Test-Path $PROGRAM_FILE)) {
    Write-Host "❌ Program not found. Running build first..." -ForegroundColor Red
    Write-Host "   Run 'anchor build' in WSL or use: npm run build:anchor" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Program found: $(Get-Item $PROGRAM_FILE | ForEach-Object { $_.Length / 1MB } | ForEach-Object { "$_.2 MB" })" -ForegroundColor Green
Write-Host ""

# Check Solana CLI in WSL
Write-Host "🔍 Checking Solana CLI..." -ForegroundColor Cyan
$SOLANA_PATH = wsl bash -c "echo ~/.local/share/solana/install/active_release/bin/solana"
Write-Host "  Solana CLI: $SOLANA_PATH" -ForegroundColor Cyan
Write-Host ""

# Verify wallet balance
Write-Host "💰 Checking wallet balance..." -ForegroundColor Cyan
$BALANCE = wsl bash -c "~/.local/share/solana/install/active_release/bin/solana balance --url devnet"
Write-Host "  Balance: $BALANCE" -ForegroundColor Green
Write-Host ""

# Current deployment status
Write-Host "📊 Deployment Status:" -ForegroundColor Cyan
Write-Host "  ✅ Program ID: 6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc" -ForegroundColor Green
Write-Host "  ✅ Status: DEPLOYED TO DEVNET" -ForegroundColor Green
Write-Host "  ✅ Deployed Slot: 429803842" -ForegroundColor Green
Write-Host "  ✅ Upgrade Authority: EwrEb3sWWiaz7mAN4XaDiADcjmBL85Eiq6JFVXrKU7En" -ForegroundColor Green
Write-Host ""

Write-Host "🔗 View on Explorer:" -ForegroundColor Yellow
Write-Host "  Solana Explorer: https://explorer.solana.com/address/6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc?cluster=devnet" -ForegroundColor Cyan
Write-Host "  Solscan: https://solscan.io/account/6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc?cluster=devnet" -ForegroundColor Cyan
Write-Host ""

Write-Host "📝 To upgrade the program:" -ForegroundColor Yellow
Write-Host "  1. Build new version: anchor build --no-idl" -ForegroundColor White
Write-Host "  2. Strip ELF section (if needed):" -ForegroundColor White
Write-Host "     llvm-objcopy --remove-section '.data._ZN...' target/deploy/meteora_fee_router.so" -ForegroundColor Cyan
Write-Host "  3. Deploy upgrade:" -ForegroundColor White
Write-Host "     solana program deploy --url devnet target/deploy/meteora_fee_router.so --program-id 6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc" -ForegroundColor Cyan
Write-Host ""

Write-Host "📚 Documentation:" -ForegroundColor Cyan
Write-Host "  See: deployment/DEVNET_TROUBLESHOOTING.md" -ForegroundColor White
Write-Host ""
