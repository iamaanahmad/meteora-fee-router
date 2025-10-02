# Streamflow Integration Implementation

## Overview

This document details the implementation of Streamflow integration for the Meteora Fee Router, enabling dynamic fee distribution based on real-time vesting schedules.

## Core Integration Components

### 1. Streamflow Data Structures

```rust
#[derive(Debug, Clone)]
pub struct StreamflowStream {
    pub mint: Pubkey,
    pub deposited_amount: u64,
    pub withdrawn_amount: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub cliff_time: u64,
    pub cliff_amount: u64,
    pub canceled_at: Option<u64>,
    pub closed_at: Option<u64>,
}
```

### 2. Locked Amount Calculation

The core algorithm for calculating locked amounts:

```rust
pub fn calculate_locked_amount(
    stream: &StreamflowStream,
    current_timestamp: u64,
) -> Result<u64> {
    // Handle closed/canceled streams
    if stream.closed_at.is_some() || stream.canceled_at.is_some() {
        return Ok(0);
    }

    let available_amount = stream.deposited_amount
        .checked_sub(stream.withdrawn_amount)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Before start time - fully locked
    if current_timestamp < stream.start_time {
        return Ok(available_amount);
    }

    // After end time - fully unlocked
    if current_timestamp >= stream.end_time {
        return Ok(0);
    }

    // During cliff period
    if current_timestamp < stream.cliff_time {
        return Ok(available_amount);
    }

    // Linear vesting calculation
    let vesting_duration = stream.end_time - stream.cliff_time;
    let elapsed_since_cliff = current_timestamp - stream.cliff_time;
    
    let vested_amount = (available_amount as u128)
        .checked_mul(elapsed_since_cliff as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(vesting_duration as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)? as u64;

    available_amount
        .checked_sub(vested_amount)
        .ok_or(ErrorCode::ArithmeticOverflow.into())
}
```

### 3. Multi-Stream Processing

```rust
pub fn process_investor_streams(
    investor_streams: &[AccountInfo],
    expected_mint: &Pubkey,
    current_timestamp: u64,
) -> Result<(u32, u64)> {
    let mut processed_count = 0;
    let mut total_locked = 0;

    for stream_account in investor_streams {
        let stream = Self::validate_and_parse_stream(stream_account, expected_mint)?;
        let locked_amount = Self::calculate_locked_amount(&stream, current_timestamp)?;
        
        total_locked = total_locked
            .checked_add(locked_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        processed_count += 1;
    }

    Ok((processed_count, total_locked))
}
```

## Integration Points

### 1. Distribution Calculation Integration

The Streamflow integration feeds into the main distribution calculation:

```rust
// In distribute_fees instruction
let (processed_count, page_locked_total, investor_payouts) = 
    StreamflowIntegration::process_investor_streams(
        &investor_streams,
        &policy_config.quote_mint,
        current_timestamp,
    )?;

// Use locked amounts for distribution calculation
let (total_investor_amount, creator_amount) = calculate_distribution(
    claimed_quote,
    page_locked_total,
    policy_config.total_allocation,
    policy_config.investor_fee_share_bps,
)?;
```

### 2. Validation and Error Handling

```rust
fn validate_and_parse_stream(
    stream_account: &AccountInfo,
    expected_mint: &Pubkey,
) -> Result<StreamflowStream> {
    // Validate account ownership
    if stream_account.owner != &STREAMFLOW_PROGRAM_ID {
        return Err(ErrorCode::InvalidStreamflowAccount.into());
    }

    // Parse stream data
    let stream = Self::parse_stream_data(&stream_account.data.borrow())?;
    
    // Validate mint consistency
    if &stream.mint != expected_mint {
        return Err(ErrorCode::InvalidStreamMint.into());
    }

    Ok(stream)
}
```

## Key Features

### 1. Real-Time Vesting Calculation
- Accurate locked amount calculation based on current timestamp
- Support for cliff periods and linear vesting
- Proper handling of withdrawals and stream states

### 2. Multi-Stream Support
- Process multiple streams per investor
- Aggregate locked amounts across all streams
- Efficient batch processing with pagination

### 3. Error Resilience
- Graceful handling of closed/canceled streams
- Comprehensive validation of stream data
- Arithmetic overflow protection

### 4. Performance Optimization
- Efficient stream data parsing
- Minimal compute unit usage
- Scalable for large investor counts

## Testing Coverage

### Unit Tests
- `test_calculate_locked_amount_*`: Various vesting scenarios
- `test_process_investor_streams_*`: Multi-stream processing
- `test_validate_and_parse_stream_*`: Validation logic

### Integration Tests
- End-to-end distribution with real Streamflow data
- Edge cases and error scenarios
- Performance and scalability testing

## Usage Examples

### Basic Stream Processing
```rust
let current_time = Clock::get()?.unix_timestamp as u64;
let (count, locked_total) = StreamflowIntegration::process_investor_streams(
    &ctx.remaining_accounts[start_idx..end_idx],
    &policy_config.quote_mint,
    current_time,
)?;
```

### Error Handling
```rust
match StreamflowIntegration::calculate_locked_amount(&stream, current_time) {
    Ok(locked) => locked,
    Err(e) => {
        msg!("Stream processing error: {:?}", e);
        return Err(e);
    }
}
```

This implementation provides robust, efficient integration with Streamflow for dynamic fee distribution based on real-time vesting schedules.