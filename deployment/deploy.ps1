# Meteora Fee Router Deployment Script (PowerShell)
# This script builds and deploys the Meteora Fee Router program

param(
    [string]$Cluster = "devnet"
)

Write-Host "🚀 Starting Meteora Fee Router deployment..." -ForegroundColor Green

# Check if Anchor is installed
if (-not (Get-Command anchor -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Anchor CLI not found. Please install Anchor first." -ForegroundColor Red
    exit 1
}

# Check if Solana CLI is installed
if (-not (Get-Command solana -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Solana CLI not found. Please install Solana CLI first." -ForegroundColor Red
    exit 1
}

Write-Host "📋 Deployment Configuration:" -ForegroundColor Cyan
Write-Host "  Cluster: $Cluster" -ForegroundColor White
$programId = (anchor keys list | Select-String "meteora_fee_router" | ForEach-Object { $_.Line.Split()[1] })
Write-Host "  Program ID: $programId" -ForegroundColor White

# Build the program
Write-Host "🔨 Building program..." -ForegroundColor Yellow
anchor build

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}

# Deploy to specified cluster
Write-Host "🌐 Deploying to $Cluster..." -ForegroundColor Yellow
anchor deploy --provider.cluster $Cluster

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Deployment failed!" -ForegroundColor Red
    exit 1
}

# Verify deployment
$programId = (anchor keys list | Select-String "meteora_fee_router" | ForEach-Object { $_.Line.Split()[1] })
Write-Host "✅ Deployment complete!" -ForegroundColor Green
Write-Host "📍 Program deployed at: $programId" -ForegroundColor White
Write-Host "🔗 Explorer: https://explorer.solana.com/address/$programId?cluster=$Cluster" -ForegroundColor Blue

# Generate IDL
Write-Host "📄 Generating IDL..." -ForegroundColor Yellow
anchor idl init --filepath target/idl/meteora_fee_router.json $programId --provider.cluster $Cluster

Write-Host "🎉 Deployment successful!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Update your client code with the new program ID: $programId" -ForegroundColor White
Write-Host "2. Test the deployment with the integration examples" -ForegroundColor White
Write-Host "3. Initialize honorary positions for your vaults" -ForegroundColor White