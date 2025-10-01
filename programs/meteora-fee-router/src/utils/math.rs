use anchor_lang::prelude::*;
use crate::error::ErrorCode;
use crate::constants::*;
use crate::state::DistributionProgress;

/// Calculate distribution amounts with overflow protection
/// Returns (investor_amount, creator_amount)
pub fn calculate_distribution(
    claimed_quote: u64,
    locked_total: u64,
    y0_total: u64,
    investor_fee_share_bps: u16,
) -> Result<(u64, u64)> {
    // Validate inputs
    require!(
        investor_fee_share_bps <= MAX_BASIS_POINTS,
        ErrorCode::InvalidInvestorFeeShare
    );
    
    if y0_total == 0 || claimed_quote == 0 {
        return Ok((0, claimed_quote));
    }

    // Calculate f_locked = locked_total / y0_total (in basis points)
    let f_locked = (locked_total as u128)
        .checked_mul(MAX_BASIS_POINTS as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(y0_total as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // eligible_investor_share_bps = min(investor_fee_share_bps, f_locked)
    let eligible_share_bps = std::cmp::min(investor_fee_share_bps as u128, f_locked);
    
    // Calculate investor portion using floor division
    let investor_fee_quote = (claimed_quote as u128)
        .checked_mul(eligible_share_bps)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(MAX_BASIS_POINTS as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    // Creator gets the remainder
    let creator_fee_quote = (claimed_quote as u128)
        .checked_sub(investor_fee_quote)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    
    Ok((investor_fee_quote as u64, creator_fee_quote as u64))
}

/// Calculate individual investor weight with high precision
/// Returns weight as a fraction of WEIGHT_PRECISION
pub fn calculate_investor_weight(
    investor_locked: u64,
    total_locked: u64,
) -> Result<u128> {
    if total_locked == 0 {
        return Ok(0);
    }
    
    // weight_i(t) = locked_i(t) / locked_total(t) * WEIGHT_PRECISION
    (investor_locked as u128)
        .checked_mul(WEIGHT_PRECISION)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(total_locked as u128)
        .ok_or(ErrorCode::ArithmeticOverflow.into())
}

/// Calculate individual payout with dust handling
/// Returns (payout_amount, dust_amount)
pub fn calculate_individual_payout(
    total_investor_amount: u64,
    investor_weight: u128,
    min_payout: u64,
) -> Result<(u64, u64)> {
    // Calculate raw payout using floor division
    let raw_payout = (total_investor_amount as u128)
        .checked_mul(investor_weight)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(WEIGHT_PRECISION)
        .ok_or(ErrorCode::ArithmeticOverflow)? as u64;
    
    // Check minimum payout threshold
    if raw_payout < min_payout {
        return Ok((0, raw_payout)); // Entire amount becomes dust
    }
    
    Ok((raw_payout, 0))
}

/// Calculate batch payout with dust accumulation
/// Returns (total_paid, total_dust)
pub fn calculate_batch_payout(
    investors: &[(u64, u128)], // (locked_amount, weight) pairs
    total_investor_amount: u64,
    min_payout: u64,
    carry_over_dust: u64,
) -> Result<(u64, u64)> {
    // Handle extreme overflow edge cases early
    // If carry_over_dust is already very large, we can't safely accumulate more
    if carry_over_dust >= u64::MAX / 2 {
        // In extreme cases, return zero paid and all as dust to avoid overflow
        // This is safe because it means we're preserving all funds as dust
        return Ok((0, total_investor_amount));
    }
    
    let mut total_paid = 0u64;
    let mut loop_dust = 0u64;
    
    for (_, weight) in investors {
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            *weight,
            min_payout,
        )?;
        
        // Check for overflow before adding
        total_paid = total_paid
            .checked_add(payout)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        loop_dust = loop_dust
            .checked_add(dust)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
    }

    // Handle overshoot: if we calculated more than available, clamp and track overshoot as dust
    let (adjusted_paid, overshoot) = if total_paid > total_investor_amount {
        let overshoot_amount = total_paid
            .checked_sub(total_investor_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        (total_investor_amount, overshoot_amount)
    } else {
        (total_paid, 0)
    };

    // Add overshoot to loop_dust if any
    if overshoot > 0 {
        loop_dust = loop_dust
            .checked_add(overshoot)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
    }

    // Calculate remainder and unmatched distribution
    let remainder = total_investor_amount
        .checked_sub(adjusted_paid)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let unmatched_distribution = remainder.saturating_sub(loop_dust);

    // Accumulate all dust components with overflow checking
    let total_dust = carry_over_dust
        .checked_add(loop_dust)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_add(unmatched_distribution)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    Ok((adjusted_paid, total_dust))
}

/// Enforce daily cap with current distribution progress
pub fn enforce_daily_cap(
    progress: &DistributionProgress,
    additional_amount: u64,
    daily_cap: Option<u64>,
) -> Result<u64> {
    let Some(cap) = daily_cap else {
        return Ok(additional_amount); // No cap enforced
    };
    
    let current_distributed = progress.current_day_distributed;
    let available_capacity = cap
        .checked_sub(current_distributed)
        .ok_or(ErrorCode::DailyCapExceeded)?;

    if available_capacity == 0 {
        return Err(ErrorCode::DailyCapExceeded.into());
    }
    
    // Return the lesser of requested amount or available capacity
    Ok(std::cmp::min(additional_amount, available_capacity))
}

/// Calculate dust threshold payout from accumulated dust
/// Returns (payout_amount, remaining_dust)
pub fn calculate_dust_payout(
    accumulated_dust: u64,
    min_payout: u64,
) -> (u64, u64) {
    if accumulated_dust >= min_payout {
        // Pay out in multiples of min_payout
        let payout_multiples = accumulated_dust / min_payout;
        let payout_amount = payout_multiples * min_payout;
        let remaining_dust = accumulated_dust - payout_amount;
        (payout_amount, remaining_dust)
    } else {
        (0, accumulated_dust)
    }
}

/// Validate distribution parameters
pub fn validate_distribution_params(
    claimed_quote: u64,
    y0_total: u64,
    investor_fee_share_bps: u16,
    min_payout: u64,
) -> Result<()> {
    require!(
        investor_fee_share_bps <= MAX_BASIS_POINTS,
        ErrorCode::InvalidInvestorFeeShare
    );
    
    require!(
        y0_total > 0,
        ErrorCode::InvalidY0TotalAllocation
    );
    
    require!(
        min_payout > 0,
        ErrorCode::InvalidMinPayoutThreshold
    );
    
    Ok(())
}

/// Calculate total locked amounts from investor data
pub fn calculate_total_locked(
    investors: &[u64], // locked amounts
) -> Result<u64> {
    let mut total = 0u64;
    
    for &locked_amount in investors {
        total = total
            .checked_add(locked_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
    }
    
    Ok(total)
}

/// Calculate weights for all investors
pub fn calculate_all_weights(
    investors: &[u64], // locked amounts
    total_locked: u64,
) -> Result<Vec<u128>> {
    let mut weights = Vec::with_capacity(investors.len());
    
    for &locked_amount in investors {
        let weight = calculate_investor_weight(locked_amount, total_locked)?;
        weights.push(weight);
    }
    
    Ok(weights)
}

/// Verify weight calculations sum to WEIGHT_PRECISION (within rounding error)
pub fn verify_weight_sum(weights: &[u128]) -> Result<()> {
    let total_weight: u128 = weights.iter().sum();
    
    // Allow for small rounding errors (up to number of investors)
    let max_rounding_error = weights.len() as u128;
    let diff = if total_weight > WEIGHT_PRECISION {
        total_weight - WEIGHT_PRECISION
    } else {
        WEIGHT_PRECISION - total_weight
    };
    
    require!(
        diff <= max_rounding_error,
        ErrorCode::ArithmeticOverflow
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::DistributionProgress;
    use anchor_lang::prelude::Pubkey;

    #[test]
    fn test_calculate_distribution_basic() {
        // Test basic distribution calculation
        let claimed_quote = 1000u64;
        let locked_total = 5000u64;
        let y0_total = 10000u64;
        let investor_fee_share_bps = 8000u16; // 80%

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 5000/10000 = 0.5 = 5000 bps
        // eligible_share = min(8000, 5000) = 5000 bps = 50%
        // investor_amount = 1000 * 50% = 500
        // creator_amount = 1000 - 500 = 500
        assert_eq!(result, (500, 500));
    }

    #[test]
    fn test_calculate_distribution_fully_locked() {
        // Test when all tokens are locked
        let claimed_quote = 1000u64;
        let locked_total = 10000u64;
        let y0_total = 10000u64;
        let investor_fee_share_bps = 8000u16;

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 10000/10000 = 1.0 = 10000 bps
        // eligible_share = min(8000, 10000) = 8000 bps = 80%
        // investor_amount = 1000 * 80% = 800
        // creator_amount = 1000 - 800 = 200
        assert_eq!(result, (800, 200));
    }

    #[test]
    fn test_calculate_distribution_fully_unlocked() {
        // Test when no tokens are locked
        let claimed_quote = 1000u64;
        let locked_total = 0u64;
        let y0_total = 10000u64;
        let investor_fee_share_bps = 8000u16;

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 0/10000 = 0
        // eligible_share = min(8000, 0) = 0
        // investor_amount = 1000 * 0% = 0
        // creator_amount = 1000 - 0 = 1000
        assert_eq!(result, (0, 1000));
    }

    #[test]
    fn test_calculate_distribution_zero_y0() {
        // Test edge case with zero Y0
        let claimed_quote = 1000u64;
        let locked_total = 5000u64;
        let y0_total = 0u64;
        let investor_fee_share_bps = 8000u16;

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // Should return all to creator when Y0 is zero
        assert_eq!(result, (0, 1000));
    }

    #[test]
    fn test_calculate_distribution_zero_claimed() {
        // Test edge case with zero claimed amount
        let claimed_quote = 0u64;
        let locked_total = 5000u64;
        let y0_total = 10000u64;
        let investor_fee_share_bps = 8000u16;

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        assert_eq!(result, (0, 0));
    }

    #[test]
    fn test_calculate_distribution_invalid_bps() {
        // Test invalid basis points
        let claimed_quote = 1000u64;
        let locked_total = 5000u64;
        let y0_total = 10000u64;
        let investor_fee_share_bps = 10001u16; // Invalid: > 10000

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_investor_weight_basic() {
        let investor_locked = 2500u64;
        let total_locked = 10000u64;

        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();

        // Expected: 2500/10000 * 1_000_000 = 250_000
        assert_eq!(weight, 250_000);
    }

    #[test]
    fn test_calculate_investor_weight_zero_total() {
        let investor_locked = 2500u64;
        let total_locked = 0u64;

        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 0);
    }

    #[test]
    fn test_calculate_individual_payout_above_threshold() {
        let total_investor_amount = 1000u64;
        let investor_weight = 250_000u128; // 25%
        let min_payout = 100u64;

        let result = calculate_individual_payout(
            total_investor_amount,
            investor_weight,
            min_payout,
        ).unwrap();

        // Expected payout: 1000 * 250_000 / 1_000_000 = 250
        // Since 250 >= 100, payout = 250, dust = 0
        assert_eq!(result, (250, 0));
    }

    #[test]
    fn test_calculate_individual_payout_below_threshold() {
        let total_investor_amount = 1000u64;
        let investor_weight = 50_000u128; // 5%
        let min_payout = 100u64;

        let result = calculate_individual_payout(
            total_investor_amount,
            investor_weight,
            min_payout,
        ).unwrap();

        // Expected payout: 1000 * 50_000 / 1_000_000 = 50
        // Since 50 < 100, payout = 0, dust = 50
        assert_eq!(result, (0, 50));
    }

    #[test]
    fn test_calculate_batch_payout() {
        let investors = vec![
            (2500u64, 250_000u128), // 25% weight
            (3000u64, 300_000u128), // 30% weight
            (1500u64, 150_000u128), // 15% weight
            (500u64, 50_000u128),   // 5% weight (below threshold)
        ];
        let total_investor_amount = 1000u64;
        let min_payout = 100u64;
        let carry_over_dust = 25u64;

        let result = calculate_batch_payout(
            &investors,
            total_investor_amount,
            min_payout,
            carry_over_dust,
        ).unwrap();

    // Expected payouts: 250, 300, 150, 0 (below threshold)
    // Expected dust: 25 (carry over) + 50 (below threshold) + 250 (unmatched weight) = 325
    let expected_paid = 250 + 300 + 150;
    let expected_dust = 25 + 50 + 250;
        assert_eq!(result, (expected_paid, expected_dust));
    }

    #[test]
    fn test_enforce_daily_cap_no_cap() {
        let progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 0,
            current_day_distributed: 500,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 0,
        };

        let result = enforce_daily_cap(&progress, 1000, None).unwrap();
        assert_eq!(result, 1000); // No cap, return full amount
    }

    #[test]
    fn test_enforce_daily_cap_within_limit() {
        let progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 0,
            current_day_distributed: 500,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 0,
        };

        let result = enforce_daily_cap(&progress, 300, Some(1000)).unwrap();
        assert_eq!(result, 300); // Within limit, return requested amount
    }

    #[test]
    fn test_enforce_daily_cap_exceeds_limit() {
        let progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 0,
            current_day_distributed: 800,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 0,
        };

        let result = enforce_daily_cap(&progress, 300, Some(1000)).unwrap();
        assert_eq!(result, 200); // Cap at remaining capacity: 1000 - 800 = 200
    }

    #[test]
    fn test_enforce_daily_cap_already_exceeded() {
        let progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 0,
            current_day_distributed: 1000,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 0,
        };

        let result = enforce_daily_cap(&progress, 100, Some(1000));
        assert!(result.is_err()); // Should fail when already at cap
    }

    #[test]
    fn test_calculate_dust_payout_above_threshold() {
        let accumulated_dust = 350u64;
        let min_payout = 100u64;

        let result = calculate_dust_payout(accumulated_dust, min_payout);
        
        // Should pay out 3 * 100 = 300, leaving 50 as dust
        assert_eq!(result, (300, 50));
    }

    #[test]
    fn test_calculate_dust_payout_below_threshold() {
        let accumulated_dust = 75u64;
        let min_payout = 100u64;

        let result = calculate_dust_payout(accumulated_dust, min_payout);
        
        // Should pay out 0, leaving 75 as dust
        assert_eq!(result, (0, 75));
    }

    #[test]
    fn test_calculate_dust_payout_exact_multiple() {
        let accumulated_dust = 500u64;
        let min_payout = 100u64;

        let result = calculate_dust_payout(accumulated_dust, min_payout);
        
        // Should pay out exactly 500, leaving 0 as dust
        assert_eq!(result, (500, 0));
    }

    #[test]
    fn test_validate_distribution_params_valid() {
        let result = validate_distribution_params(1000, 10000, 8000, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_distribution_params_invalid_bps() {
        let result = validate_distribution_params(1000, 10000, 10001, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_distribution_params_zero_y0() {
        let result = validate_distribution_params(1000, 0, 8000, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_distribution_params_zero_min_payout() {
        let result = validate_distribution_params(1000, 10000, 8000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_total_locked() {
        let investors = vec![1000u64, 2500u64, 1500u64, 3000u64];
        let result = calculate_total_locked(&investors).unwrap();
        assert_eq!(result, 8000);
    }

    #[test]
    fn test_calculate_total_locked_empty() {
        let investors: Vec<u64> = vec![];
        let result = calculate_total_locked(&investors).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_calculate_all_weights() {
        let investors = vec![2500u64, 3000u64, 1500u64, 1000u64];
        let total_locked = 8000u64;
        
        let weights = calculate_all_weights(&investors, total_locked).unwrap();
        
        let expected_weights = vec![
            312_500u128, // 2500/8000 * 1_000_000
            375_000u128, // 3000/8000 * 1_000_000
            187_500u128, // 1500/8000 * 1_000_000
            125_000u128, // 1000/8000 * 1_000_000
        ];
        
        assert_eq!(weights, expected_weights);
    }

    #[test]
    fn test_verify_weight_sum_valid() {
        let weights = vec![250_000u128, 300_000u128, 200_000u128, 250_000u128];
        let result = verify_weight_sum(&weights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_weight_sum_with_rounding_error() {
        // Weights that sum to slightly less than WEIGHT_PRECISION due to rounding
        let weights = vec![333_333u128, 333_333u128, 333_333u128]; // Sum = 999_999
        let result = verify_weight_sum(&weights);
        assert!(result.is_ok()); // Should accept small rounding errors
    }

    #[test]
    fn test_arithmetic_overflow_protection() {
        // Test with values that would cause overflow in naive implementation
        let claimed_quote = u64::MAX;
        let locked_total = u64::MAX;
        let y0_total = 1u64;
        let investor_fee_share_bps = 10000u16;

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        );

        // Should handle extreme values without overflow
        assert!(result.is_ok());
        let (investor_amount, creator_amount) = result.unwrap();
        assert_eq!(investor_amount, claimed_quote);
        assert_eq!(creator_amount, 0);
    }

    #[test]
    fn test_precision_edge_cases() {
        // Test very small amounts and weights
        let total_investor_amount = 1u64;
        let investor_weight = 1u128; // Extremely small weight
        let min_payout = 1u64;

        let result = calculate_individual_payout(
            total_investor_amount,
            investor_weight,
            min_payout,
        ).unwrap();

        // Should handle precision correctly
        assert_eq!(result, (0, 0)); // Rounds down to 0
    }

    #[test]
    fn test_dust_accumulation_scenario() {
        // Simulate multiple small payouts that accumulate dust
        let investors = vec![
            (100u64, 10_000u128),   // 1% each, very small amounts
            (100u64, 10_000u128),
            (100u64, 10_000u128),
            (100u64, 10_000u128),
            (100u64, 10_000u128),
        ];
        
        let total_investor_amount = 50u64; // Small total amount
        let min_payout = 20u64; // Relatively high threshold
        let carry_over_dust = 0u64;

        let result = calculate_batch_payout(
            &investors,
            total_investor_amount,
            min_payout,
            carry_over_dust,
        ).unwrap();

    // Each investor should get 50 * 10_000 / 1_000_000 = 0.5 -> 0 (floor)
    // Entire distribution becomes dust
    assert_eq!(result.0, 0);  // No payouts
    assert_eq!(result.1, 50); // All 50 lamports carried as dust
    }

    #[test]
    fn test_large_number_precision() {
        // Test with large numbers to ensure precision is maintained
        let claimed_quote = 1_000_000_000u64; // 1B
        let locked_total = 750_000_000u64;    // 750M
        let y0_total = 1_000_000_000u64;      // 1B
        let investor_fee_share_bps = 9000u16; // 90%

        let result = calculate_distribution(
            claimed_quote,
            locked_total,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 750M/1B = 0.75 = 7500 bps
        // eligible_share = min(9000, 7500) = 7500 bps = 75%
        // investor_amount = 1B * 75% = 750M
        // creator_amount = 1B - 750M = 250M
        assert_eq!(result, (750_000_000, 250_000_000));
    }
}