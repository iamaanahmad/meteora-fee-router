# Streamflow Integration Module

This document describes the Streamflow integration module for the Meteora Fee Router program.

## Overview

The Streamflow integration module provides functionality to:
- Read and validate Streamflow vesting accounts
- Calculate locked token amounts based on vesting schedules
- Aggregate investor data for fee distribution calculations
- Handle edge cases and error conditions

## Core Components

### StreamflowStream Structure

Represents a Streamflow vesting stream with all necessary fields for locked amount calculations:

```rust
pub struct StreamflowStream {
    pub recipient: Pubkey,        // Investor wallet
    pub mint: Pubkey,            // Token mint
    pub deposited_amount: u64,   // Total tokens deposited
    pub withdrawn_amount: u64,   // Tokens already withdrawn
    pub start_time: i64,         // Vesting start timestamp
    pub end_time: i64,           // Vesting end timestamp
    pub cliff_time: i64,         // Cliff timestamp
    pub closed_at: Option<i64>,  // Stream closure timestamp
    // ... other fields
}
```

### StreamflowIntegration Utilities

Main utility struct providing static methods for Streamflow operations:

#### Key Methods

1. **`calculate_locked_amount(stream, timestamp)`**
   - Calculates still-locked tokens for a stream at given timestamp
   - Handles linear vesting, cliff periods, and withdrawals
   - Returns 0 for closed or fully vested streams

2. **`validate_and_parse_stream(account_info, expected_mint)`**
   - Validates Streamflow account data and ownership
   - Parses account data into StreamflowStream structure
   - Validates mint matches expected token

3. **`aggregate_investor_data(stream_accounts, mint, timestamp)`**
   - Aggregates multiple streams per investor
   - Returns Vec<InvestorData> with total locked amounts per investor
   - Handles deduplication by recipient wallet

4. **`calculate_total_locked(stream_accounts, mint, timestamp)`**
   - Calculates total locked amount across all streams
   - Used for distribution percentage calculations

## Usage in Distribution Flow

### 1. Validation Phase

```rust
// Validate all Streamflow accounts before processing
StreamflowIntegration::validate_all_streams_mint(&stream_accounts, &expected_mint)?;

// Check minimum investor requirements
let investor_count = StreamflowIntegration::get_unique_investor_count(&stream_accounts, &expected_mint)?;
require!(investor_count >= MIN_INVESTORS, ErrorCode::InsufficientInvestors);
```

### 2. Distribution Calculation

```rust
let current_timestamp = StreamflowIntegration::get_current_timestamp()?;

// Calculate total locked for distribution percentage
let total_locked = StreamflowIntegration::calculate_total_locked(
    &stream_accounts,
    &policy_config.quote_mint,
    current_timestamp,
)?;

// Calculate investor vs creator split
let (investor_amount, creator_amount) = calculate_distribution(
    claimed_quote_fees,
    total_locked,
    policy_config.y0_total_allocation,
    policy_config.investor_fee_share_bps,
)?;
```

### 3. Paginated Processing

```rust
// Process investors in pages to manage compute budget
let (processed_count, page_locked_total, investor_payouts) = 
    StreamflowIntegration::process_investor_page(
        &page_accounts,
        &policy_config.quote_mint,
        current_timestamp,
        page_start,
        page_size,
    )?;

// Calculate individual payouts based on locked amounts
for (investor_wallet, locked_amount) in investor_payouts {
    let weight = calculate_investor_weight(locked_amount, page_locked_total)?;
    let payout = (investor_amount * weight) / WEIGHT_PRECISION;
    
    if payout >= policy_config.min_payout_lamports {
        // Transfer payout to investor
        transfer_to_investor(investor_wallet, payout)?;
    } else {
        // Accumulate dust for later distribution
        distribution_progress.carry_over_dust += payout;
    }
}
```

## Vesting Calculation Logic

The locked amount calculation follows this logic:

1. **Before Start Time**: All deposited tokens are locked
2. **Before Cliff Time**: All deposited tokens are locked
3. **Linear Vesting Period**: 
   ```
   vested_amount = deposited_amount * (current_time - start_time) / (end_time - start_time)
   available_amount = deposited_amount - withdrawn_amount
   locked_amount = available_amount - vested_amount
   ```
4. **After End Time**: No tokens are locked (fully vested)
5. **Closed Streams**: No tokens are locked

## Error Handling

The module provides comprehensive error handling for:

- **StreamflowValidationFailed**: Invalid account data or ownership
- **InvalidStreamMint**: Stream mint doesn't match expected token
- **StreamClosed**: Attempting to process closed streams
- **InvalidStreamTimeParameters**: Invalid vesting time configuration
- **StreamflowDataParsingFailed**: Corrupted account data

## Testing Support

### Mock Data Generation

```rust
use crate::utils::mock_streamflow::MockStreamflowBuilder;

// Create test streams with different vesting states
let fully_locked = MockStreamflowBuilder::fully_locked_at(investor, mint, 1_000_000, timestamp);
let half_vested = MockStreamflowBuilder::half_vested_at(investor, mint, 2_000_000, timestamp);
let custom_vested = MockStreamflowBuilder::vested_percentage_at(investor, mint, 500_000, timestamp, 0.75);
```

### Test Scenarios

```rust
use crate::utils::mock_streamflow::MockInvestorScenario;

// Pre-built test scenarios
let diverse_scenario = MockInvestorScenario::diverse_vesting_scenario();
let multi_stream_scenario = MockInvestorScenario::multiple_streams_per_investor();
let dust_scenario = MockInvestorScenario::dust_scenario();
```

## Integration Patterns

### 1. Batch Processing with Recovery

- Save pagination cursor before processing each page
- Process pages atomically
- Update cursor only after successful processing
- Enable retry from last successful position

### 2. Gas Optimization

- Use optimal page sizes (typically 10-20 investors per transaction)
- Precompute total locked amounts when possible
- Minimize cross-program invocations
- Use efficient weight calculation algorithms

### 3. Edge Case Handling

- Handle fully vested investor sets gracefully
- Manage dust accumulation and periodic payouts
- Validate stream integrity before processing
- Provide clear error messages for debugging

### 4. Event Emission

Emit events for monitoring and debugging:
- `InvestorPageProcessed`: After each page completion
- `DustAccumulated`: When payouts fall below threshold
- `StreamflowValidationError`: When stream validation fails

## Security Considerations

1. **Account Validation**: Always validate Streamflow account ownership and data integrity
2. **Arithmetic Safety**: Use overflow-protected arithmetic for all calculations
3. **State Consistency**: Ensure pagination state remains consistent across transactions
4. **Access Control**: Verify caller permissions for distribution operations

## Performance Considerations

1. **Compute Budget**: Limit page sizes to stay within Solana compute limits
2. **Account Size**: Optimize data structures for minimal rent costs
3. **Memory Usage**: Use efficient algorithms for large investor sets
4. **Network Calls**: Minimize the number of transactions required for distribution

## Future Enhancements

Potential improvements for production deployment:

1. **Advanced Vesting Models**: Support for non-linear vesting curves
2. **Stream Aggregation**: Optimize processing for investors with many streams
3. **Caching**: Cache frequently accessed stream data
4. **Batch Operations**: Support for batch stream validation and processing
5. **Monitoring**: Enhanced metrics and alerting for distribution health