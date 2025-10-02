# Investor Distribution Implementation

## Overview

This document details the implementation of investor fee distribution logic for the Meteora Fee Router, enabling pro-rata distribution based on Streamflow vesting schedules.

## Core Components

### 1. Distribution Data Structures

```rust
#[derive(Debug, Clone)]
pub struct InvestorPayout {
    pub investor_ata: Pubkey,
    pub amount: u64,
    pub weight: u64,
    pub locked_amount: u64,
}

#[derive(Debug)]
pub struct BatchPayoutResult {
    pub total_distributed: u64,
    pub dust_accumulated: u64,
    pub processed_count: u32,
    pub payouts: Vec<InvestorPayout>,
}

#[derive(Debug)]
pub struct PageStatistics {
    pub total_weight: u64,
    pub total_locked: u64,
    pub average_payout: u64,
    pub min_payout: u64,
    pub max_payout: u64,
}
```

### 2. Pro-Rata Distribution Calculation

The core algorithm for calculating investor distributions:

```rust
pub fn calculate_page_distribution(
    investor_fee_quote: u64,
    investor_locked_amounts: &[(Pubkey, u64)],
    min_payout_lamports: u64,
) -> Result<BatchPayoutResult> {
    // Calculate total locked amount for weight calculation
    let total_locked = calculate_total_locked(investor_locked_amounts)?;
    
    if total_locked == 0 {
        return Ok(BatchPayoutResult {
            total_distributed: 0,
            dust_accumulated: investor_fee_quote,
            processed_count: 0,
            payouts: vec![],
        });
    }

    let mut payouts = Vec::new();
    let mut total_distributed = 0u64;
    let scaling_factor = calculate_scaling_factor(investor_fee_quote, total_locked)?;

    for (investor_ata, locked_amount) in investor_locked_amounts {
        // Calculate individual weight and payout
        let weight = calculate_investor_weight(*locked_amount, total_locked)?;
        let raw_payout = calculate_individual_payout(
            investor_fee_quote,
            weight,
            WEIGHT_PRECISION,
        )?;

        // Apply minimum payout threshold
        let final_payout = if raw_payout >= min_payout_lamports {
            raw_payout
        } else {
            0 // Below threshold - becomes dust
        };

        if final_payout > 0 {
            payouts.push(InvestorPayout {
                investor_ata: *investor_ata,
                amount: final_payout,
                weight,
                locked_amount: *locked_amount,
            });

            total_distributed = total_distributed
                .checked_add(final_payout)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }
    }

    let dust_accumulated = investor_fee_quote
        .checked_sub(total_distributed)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    Ok(BatchPayoutResult {
        total_distributed,
        dust_accumulated,
        processed_count: payouts.len() as u32,
        payouts,
    })
}
```

### 3. Weight Calculation System

```rust
pub fn calculate_investor_weight(
    locked_amount: u64,
    total_locked: u64,
) -> Result<u64> {
    if total_locked == 0 {
        return Ok(0);
    }

    // Use high precision for accurate weight calculation
    let weight = (locked_amount as u128)
        .checked_mul(WEIGHT_PRECISION as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(total_locked as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    Ok(weight as u64)
}

pub fn calculate_individual_payout(
    total_amount: u64,
    weight: u64,
    precision: u64,
) -> Result<u64> {
    let payout = (total_amount as u128)
        .checked_mul(weight as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(precision as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    Ok(payout as u64)
}
```

### 4. Dust Management

```rust
pub fn accumulate_dust(
    current_dust: u64,
    new_dust: u64,
    max_dust_threshold: u64,
) -> Result<(u64, u64)> {
    let total_dust = current_dust
        .checked_add(new_dust)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    // Check if dust exceeds threshold for distribution
    if total_dust >= max_dust_threshold {
        let distributable_dust = total_dust;
        let remaining_dust = 0;
        Ok((distributable_dust, remaining_dust))
    } else {
        Ok((0, total_dust))
    }
}

pub fn calculate_dust_payout(
    dust_amount: u64,
    min_payout_threshold: u64,
) -> Result<u64> {
    if dust_amount >= min_payout_threshold {
        // Calculate payout as multiple of threshold
        let payout_multiple = dust_amount / min_payout_threshold;
        Ok(payout_multiple * min_payout_threshold)
    } else {
        Ok(0)
    }
}
```

## Key Features

### 1. High-Precision Mathematics
- Uses `WEIGHT_PRECISION` constant for accurate calculations
- Prevents rounding errors in pro-rata distribution
- Maintains mathematical invariants

### 2. Dust Handling
- Accumulates small amounts below minimum threshold
- Distributes dust when threshold is reached
- Carries forward remaining dust to next distribution

### 3. Minimum Payout Enforcement
- Prevents spam with tiny payouts
- Configurable minimum payout threshold
- Converts sub-threshold amounts to dust

### 4. Batch Processing
- Efficient processing of multiple investors
- Pagination support for large investor sets
- Comprehensive statistics and reporting

## Implementation Details

### 1. Distribution Execution

```rust
pub fn execute_investor_distribution<'info>(
    investor_payouts: &[InvestorPayout],
    treasury_ata: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    authority_seeds: &[&[&[u8]]],
) -> Result<u64> {
    let mut total_distributed = 0u64;

    for payout in investor_payouts {
        if payout.amount > 0 {
            // Create transfer instruction
            let transfer_instruction = spl_token::instruction::transfer(
                &spl_token::ID,
                treasury_ata.key,
                &payout.investor_ata,
                treasury_ata.key, // Authority is treasury owner
                &[],
                payout.amount,
            )?;

            // Execute transfer
            invoke_signed(
                &transfer_instruction,
                &[
                    treasury_ata.clone(),
                    // investor_ata would be passed in remaining_accounts
                    token_program.clone(),
                ],
                authority_seeds,
            )?;

            total_distributed = total_distributed
                .checked_add(payout.amount)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }
    }

    Ok(total_distributed)
}
```

### 2. Validation and Error Handling

```rust
pub fn validate_distribution_params(
    investor_fee_share_bps: u16,
    min_payout_lamports: u64,
    total_allocation: u64,
) -> Result<()> {
    // Validate fee share is within bounds
    if investor_fee_share_bps > MAX_BASIS_POINTS {
        return Err(ErrorCode::InvalidFeeShareBps.into());
    }

    // Validate minimum payout is reasonable
    if min_payout_lamports == 0 {
        return Err(ErrorCode::InvalidMinPayout.into());
    }

    // Validate total allocation
    if total_allocation == 0 {
        return Err(ErrorCode::InvalidTotalAllocation.into());
    }

    Ok(())
}
```

### 3. Statistics and Monitoring

```rust
pub fn calculate_page_statistics(
    payouts: &[InvestorPayout],
) -> Result<PageStatistics> {
    if payouts.is_empty() {
        return Ok(PageStatistics {
            total_weight: 0,
            total_locked: 0,
            average_payout: 0,
            min_payout: 0,
            max_payout: 0,
        });
    }

    let total_weight = payouts.iter().map(|p| p.weight).sum();
    let total_locked = payouts.iter().map(|p| p.locked_amount).sum();
    let total_amount: u64 = payouts.iter().map(|p| p.amount).sum();
    
    let average_payout = total_amount / payouts.len() as u64;
    let min_payout = payouts.iter().map(|p| p.amount).min().unwrap_or(0);
    let max_payout = payouts.iter().map(|p| p.amount).max().unwrap_or(0);

    Ok(PageStatistics {
        total_weight,
        total_locked,
        average_payout,
        min_payout,
        max_payout,
    })
}
```

## Testing Coverage

### Unit Tests
- `test_calculate_page_distribution_*`: Distribution calculations
- `test_weight_calculation_*`: Weight calculation accuracy
- `test_dust_accumulation_*`: Dust handling logic
- `test_minimum_payout_enforcement_*`: Threshold validation

### Integration Tests
- End-to-end distribution with multiple investors
- Edge cases with varying locked amounts
- Performance testing with large investor sets

## Mathematical Properties

### 1. Conservation of Value
- Total distributed + dust = total available
- No value is lost in calculations
- Rounding errors are minimized

### 2. Proportional Fairness
- Distribution is proportional to locked amounts
- Higher locked amounts receive proportionally more fees
- Mathematical precision ensures fairness

### 3. Dust Minimization
- Dust accumulation prevents value loss
- Threshold-based distribution of accumulated dust
- Efficient handling of small amounts

This implementation ensures fair, accurate, and efficient distribution of fees to investors based on their vesting schedules.