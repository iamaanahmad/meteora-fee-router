# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2025-12-21

### Added

- ✅ **Deployed to Solana Devnet** - Program is now live!
  - Program ID: `6LHfK4a941ABKnyCfyhUiGhVdQR6z7q8Xnb8uxVb3Zfc`
  - Latest Deployed Slot: 429815311
  - Upgrade Authority: `EwrEb3sWWiaz7mAN4XaDiADcjmBL85Eiq6JFVXrKU7En`

- ✅ **Streamflow Integration Hardened**
  - Added `STREAMFLOW_PROGRAM_ID` constant (`strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m`)
  - Added owner validation - streams must be owned by Streamflow program
  - Added `InvalidStreamflowAccountOwner` error code
  - Fixed account layout to match @streamflow/stream SDK v10.x (period-based vesting)
  - Verified byte offsets: sender@49, recipient@113, mint@177, period@425, etc.

### Changed

- Updated to Anchor 0.32.1 (from 0.29.0)
- Updated to Rust 1.83.0 (from 1.80.0)
- Updated Solana SDK to 2.x compatibility
- Fixed `solana_program` imports to use `anchor_lang::solana_program`

### Fixed

- ELF section name issue with Solana 3.x loader (section names must be ≤16 bytes)
- Dependency compatibility issues with Anchor 0.32.x

### Documentation

- Updated README with live deployment status
- Updated deployment guide with upgrade instructions
- Added ELF troubleshooting documentation

---

## [1.0.0] - 2025-11-14

### Added

- Initial production release
- Quote-only fee accrual for Meteora DAMM V2 pools
- Permissionless 24-hour distribution crank system
- Streamflow vesting integration for dynamic distribution
- Pagination support for scalable investor payouts
- Comprehensive test suite
- Security audit completed
- Complete documentation and deployment guides
- NPM package published (`@ashqking/meteora-fee-router`)
- GitHub Actions CI/CD pipeline
- Solana devnet deployment ready

### Features

- **Fee Collection**: Automated quote-only fee collection from DAMM pools
- **Distribution Engine**: Proportional fee distribution based on vesting schedules
- **Streamflow Integration**: Real-time vesting data reading
- **Pagination System**: Handle unlimited investors across multiple transactions
- **Access Control**: PDA-based deterministic address generation
- **Error Handling**: Comprehensive validation and error reporting

### Testing

- Unit tests: comprehensive Rust test coverage
- Integration tests: end-to-end validation coverage
- Performance tests: Compute optimization benchmarks
- Security tests: Vulnerability and edge case testing

### Documentation

- Main README with setup and architecture examples
- Deployment guide for build and release workflows
- Troubleshooting references for common issues
- Security audit summary and recommendations

### Infrastructure

- GitHub Actions workflow for CI/CD
- Automated testing on every push
- Security validation checks
- Release automation setup

---

## Release Notes

### Version 1.0.0 (Production Release)

**Deployment Information:**
- **Network**: Solana Devnet
- **Anchor Version**: 0.32.1
- **Rust Version**: 1.83.0+
- **Node Version**: 18.17.0+
- **Status**: ✅ Production-ready

**Quick Start:**
```bash
npm install @ashqking/meteora-fee-router
npm run build
cargo test --manifest-path programs/meteora-fee-router/Cargo.toml
npm run deploy:devnet
```

**Supported Platforms:**
- Solana Devnet (verified)
- Solana Mainnet-beta (ready, not deployed yet)

---

## Upcoming

### [1.1.0] - Planned

- [ ] Multi-protocol fee aggregation (Orca, Raydium)
- [ ] Creator economy revenue splitting
- [ ] Enhanced monitoring and analytics
- [ ] Performance optimizations
- [ ] Additional integration examples

### [2.0.0] - Future

- [ ] Full multi-protocol aggregator
- [ ] DAO treasury management suite
- [ ] Mainnet deployment with state migration
- [ ] Advanced distribution strategies
- [ ] Solana payroll integration

---

## How to Use This Changelog

- **Users**: Check for new features, breaking changes, bug fixes
- **Developers**: Reference for version-specific APIs and changes
- **Contributors**: See what's planned for future releases

## Versioning

This project follows Semantic Versioning:

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

Example: `1.0.0` = Major.Minor.Patch

---

**Last Updated:** November 14, 2025

For questions or to report issues, please visit:
https://github.com/iamaanahmad/meteora-fee-router
