# Deployment Tools

This directory contains all deployment-related scripts and tools for the Meteora Fee Router.

## Deployment Scripts

- **[deploy.sh](deploy.sh)** - Unix/Linux deployment script
- **[deploy.ps1](deploy.ps1)** - Windows PowerShell deployment script
- **[optimize-build.sh](optimize-build.sh)** - Build optimization script

## Validation Tools

- **[validate-deployment.js](validate-deployment.js)** - Deployment validation
- **[validate-security.js](validate-security.js)** - Security validation
- **[validate-tests.js](validate-tests.js)** - Test suite validation
- **[final-validation.js](final-validation.js)** - Comprehensive validation

## Configuration

- **[../config-templates/](../config-templates/)** - Configuration templates

## Usage

1. **Build Optimization**: `./optimize-build.sh`
2. **Deploy to Devnet**: `./deploy.sh devnet`
3. **Validate Deployment**: `node validate-deployment.js`
4. **Final Validation**: `node final-validation.js`

## Requirements

- Node.js 16+
- Anchor CLI
- Solana CLI
- Rust toolchain