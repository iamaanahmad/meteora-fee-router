use anchor_lang::prelude::*;
use crate::error::ErrorCode;

/// Pool token order enumeration for validation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenOrder {
    QuoteIsTokenA,
    QuoteIsTokenB,
}

/// Pool configuration data structure for validation
/// This represents the essential data we need from DAMM V2 pool accounts
#[derive(Debug, Clone)]
pub struct PoolValidationData {
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub current_price: u128,
    pub tick_current: i32,
    pub tick_spacing: u16,
}

/// Position validation data structure
/// This represents the essential data we need from DAMM V2 position accounts
#[derive(Debug, Clone)]
pub struct PositionValidationData {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
}

/// Validate that the pool configuration supports quote-only fee accrual
/// This is the main entry point for quote-only validation
pub fn validate_quote_only_config(
    pool_data: &PoolValidationData,
    position_data: &PositionValidationData,
    quote_mint: &Pubkey,
) -> Result<TokenOrder> {
    // Step 1: Validate pool token order and identify quote mint
    let token_order = validate_pool_token_order(pool_data, quote_mint)?;
    
    // Step 2: Validate tick range for quote-only fee accrual
    validate_tick_range_for_quote_only(pool_data, position_data, token_order)?;
    
    // Step 3: Perform preflight validation
    preflight_validate_quote_only(pool_data, position_data, token_order)?;
    
    msg!("Quote-only validation passed for mint: {}", quote_mint);
    Ok(token_order)
}

/// Validate pool token order and identify which token is the quote mint
pub fn validate_pool_token_order(
    pool_data: &PoolValidationData,
    quote_mint: &Pubkey,
) -> Result<TokenOrder> {
    if pool_data.token_mint_a == *quote_mint {
        msg!("Quote mint identified as token A");
        Ok(TokenOrder::QuoteIsTokenA)
    } else if pool_data.token_mint_b == *quote_mint {
        msg!("Quote mint identified as token B");
        Ok(TokenOrder::QuoteIsTokenB)
    } else {
        msg!("Quote mint {} not found in pool tokens A: {} B: {}", 
             quote_mint, pool_data.token_mint_a, pool_data.token_mint_b);
        Err(ErrorCode::InvalidQuoteMint.into())
    }
}

/// Validate tick range to ensure only quote fees will accrue
/// This is critical for ensuring the position only collects fees in the quote token
pub fn validate_tick_range_for_quote_only(
    pool_data: &PoolValidationData,
    position_data: &PositionValidationData,
    token_order: TokenOrder,
) -> Result<()> {
    let current_tick = pool_data.tick_current;
    let tick_lower = position_data.tick_lower;
    let tick_upper = position_data.tick_upper;
    
    // Validate tick range is properly ordered
    require!(
        tick_lower < tick_upper,
        ErrorCode::InvalidTickRange
    );
    
    // Validate tick spacing alignment
    require!(
        tick_lower % pool_data.tick_spacing as i32 == 0,
        ErrorCode::InvalidTickRange
    );
    require!(
        tick_upper % pool_data.tick_spacing as i32 == 0,
        ErrorCode::InvalidTickRange
    );
    
    // Critical validation: Ensure position only accrues quote fees
    // This depends on the token order and current price position
    match token_order {
        TokenOrder::QuoteIsTokenA => {
            // When quote is token A, we need the position to be "above" current price
            // so that swaps only move through the quote token side
            require!(
                tick_lower > current_tick,
                ErrorCode::InvalidTickRange
            );
            msg!("Quote-only validation: Position above current price (quote is token A)");
        },
        TokenOrder::QuoteIsTokenB => {
            // When quote is token B, we need the position to be "below" current price
            // so that swaps only move through the quote token side
            require!(
                tick_upper < current_tick,
                ErrorCode::InvalidTickRange
            );
            msg!("Quote-only validation: Position below current price (quote is token B)");
        }
    }
    
    Ok(())
}

/// Preflight validation that rejects configurations allowing base fee accrual
/// This provides deterministic validation before position creation
pub fn preflight_validate_quote_only(
    pool_data: &PoolValidationData,
    position_data: &PositionValidationData,
    token_order: TokenOrder,
) -> Result<()> {
    let current_tick = pool_data.tick_current;
    let tick_lower = position_data.tick_lower;
    let tick_upper = position_data.tick_upper;
    
    // Ensure position has non-zero liquidity for meaningful validation
    require!(
        position_data.liquidity > 0,
        ErrorCode::InvalidPoolConfiguration
    );
    
    // Validate that the position range doesn't span the current price
    // which would allow both base and quote fee accrual
    let position_spans_current_price = tick_lower <= current_tick && current_tick < tick_upper;
    
    if position_spans_current_price {
        msg!("Position spans current price - would accrue both base and quote fees");
        return Err(ErrorCode::BaseFeeDetected.into());
    }
    
    // Additional validation: Check if position is too close to current price
    // This provides a safety buffer to prevent accidental base fee accrual
    let safety_buffer_ticks = (pool_data.tick_spacing as i32) * 2; // 2 tick spacings buffer
    
    match token_order {
        TokenOrder::QuoteIsTokenA => {
            require!(
                tick_lower > current_tick + safety_buffer_ticks,
                ErrorCode::InvalidTickRange
            );
        },
        TokenOrder::QuoteIsTokenB => {
            require!(
                tick_upper < current_tick - safety_buffer_ticks,
                ErrorCode::InvalidTickRange
            );
        }
    }
    
    msg!("Preflight validation passed - configuration will only accrue quote fees");
    Ok(())
}

/// Validate claimed fees to ensure no base tokens were received
/// This is used during the distribution crank to enforce quote-only at runtime
pub fn validate_claimed_fees_quote_only(
    claimed_quote_amount: u64,
    claimed_base_amount: u64,
) -> Result<()> {
    // Ensure we actually claimed some quote fees
    require!(
        claimed_quote_amount > 0,
        ErrorCode::InsufficientFunds
    );
    
    // Critical: Ensure no base fees were claimed
    require!(
        claimed_base_amount == 0,
        ErrorCode::BaseFeeDetected
    );
    
    msg!("Fee claim validation passed: {} quote tokens, 0 base tokens", claimed_quote_amount);
    Ok(())
}

/// Helper function to extract pool validation data from account info
/// This would be used to deserialize DAMM V2 pool account data
pub fn extract_pool_validation_data(pool_account: &AccountInfo) -> Result<PoolValidationData> {
    // This is a placeholder implementation
    // In the actual implementation, this would deserialize the DAMM V2 pool account
    // and extract the necessary fields for validation
    
    // For now, return a mock structure for testing
    // This will be replaced with actual DAMM V2 deserialization
    msg!("Extracting pool validation data - placeholder implementation");
    
    // Return error for now to indicate this needs actual implementation
    Err(ErrorCode::InvalidPoolConfiguration.into())
}

/// Helper function to extract position validation data from account info
/// This would be used to deserialize DAMM V2 position account data
pub fn extract_position_validation_data(position_account: &AccountInfo) -> Result<PositionValidationData> {
    // This is a placeholder implementation
    // In the actual implementation, this would deserialize the DAMM V2 position account
    // and extract the necessary fields for validation
    
    msg!("Extracting position validation data - placeholder implementation");
    
    // Return error for now to indicate this needs actual implementation
    Err(ErrorCode::InvalidPoolConfiguration.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use anchor_lang::prelude::*;

    // Helper function to create test pool data
    fn create_test_pool_data() -> PoolValidationData {
        PoolValidationData {
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            current_price: 1_000_000_000_000, // 1.0 in some fixed-point representation
            tick_current: 0,
            tick_spacing: 64,
        }
    }

    // Helper function to create test position data
    fn create_test_position_data(tick_lower: i32, tick_upper: i32) -> PositionValidationData {
        PositionValidationData {
            tick_lower,
            tick_upper,
            liquidity: 1_000_000,
        }
    }

    #[test]
    fn test_validate_pool_token_order_quote_is_token_a() {
        let pool_data = create_test_pool_data();
        let quote_mint = pool_data.token_mint_a;
        
        let result = validate_pool_token_order(&pool_data, &quote_mint);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TokenOrder::QuoteIsTokenA);
    }

    #[test]
    fn test_validate_pool_token_order_quote_is_token_b() {
        let pool_data = create_test_pool_data();
        let quote_mint = pool_data.token_mint_b;
        
        let result = validate_pool_token_order(&pool_data, &quote_mint);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TokenOrder::QuoteIsTokenB);
    }

    #[test]
    fn test_validate_pool_token_order_invalid_quote_mint() {
        let pool_data = create_test_pool_data();
        let invalid_quote_mint = Pubkey::new_unique(); // Not in pool
        
        let result = validate_pool_token_order(&pool_data, &invalid_quote_mint);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tick_range_quote_is_token_a_valid() {
        let pool_data = create_test_pool_data();
        // Position above current price (tick_lower > current_tick)
        let position_data = create_test_position_data(128, 256); // Above current tick (0)
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tick_range_quote_is_token_a_invalid() {
        let pool_data = create_test_pool_data();
        // Position at or below current price (invalid for quote-only when quote is token A)
        let position_data = create_test_position_data(-128, 0); // Below/at current tick (0)
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tick_range_quote_is_token_b_valid() {
        let pool_data = create_test_pool_data();
        // Position below current price (tick_upper < current_tick)
        let position_data = create_test_position_data(-256, -128); // Below current tick (0)
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenB
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tick_range_quote_is_token_b_invalid() {
        let pool_data = create_test_pool_data();
        // Position at or above current price (invalid for quote-only when quote is token B)
        let position_data = create_test_position_data(0, 128); // At/above current tick (0)
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenB
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tick_range_invalid_order() {
        let pool_data = create_test_pool_data();
        // tick_lower >= tick_upper (invalid)
        let position_data = create_test_position_data(256, 128);
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tick_range_invalid_spacing() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_spacing = 64;
        
        // Ticks not aligned to spacing
        let position_data = create_test_position_data(100, 200); // Not multiples of 64
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tick_range_valid_spacing() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_spacing = 64;
        
        // Ticks properly aligned to spacing
        let position_data = create_test_position_data(128, 256); // Multiples of 64
        
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_preflight_validate_quote_only_success_token_a() {
        let pool_data = create_test_pool_data();
        // Position well above current price with safety buffer
        let position_data = create_test_position_data(256, 384); // Well above current tick (0)
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_preflight_validate_quote_only_success_token_b() {
        let pool_data = create_test_pool_data();
        // Position well below current price with safety buffer
        let position_data = create_test_position_data(-384, -256); // Well below current tick (0)
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenB
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_preflight_validate_spans_current_price() {
        let pool_data = create_test_pool_data();
        // Position spans current price (would accrue both base and quote fees)
        let position_data = create_test_position_data(-128, 128); // Spans current tick (0)
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
        // Should be BaseFeeDetected error
    }

    #[test]
    fn test_preflight_validate_too_close_to_current_price_token_a() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_spacing = 64;
        
        // Position too close to current price (within safety buffer)
        let position_data = create_test_position_data(64, 128); // Only 1 tick spacing above
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_preflight_validate_too_close_to_current_price_token_b() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_spacing = 64;
        
        // Position too close to current price (within safety buffer)
        let position_data = create_test_position_data(-128, -64); // Only 1 tick spacing below
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenB
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_preflight_validate_zero_liquidity() {
        let pool_data = create_test_pool_data();
        let mut position_data = create_test_position_data(256, 384);
        position_data.liquidity = 0; // Invalid liquidity
        
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_claimed_fees_quote_only_success() {
        let result = validate_claimed_fees_quote_only(1_000_000, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_claimed_fees_base_detected() {
        let result = validate_claimed_fees_quote_only(1_000_000, 1); // Any base amount fails
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_claimed_fees_no_quote_claimed() {
        let result = validate_claimed_fees_quote_only(0, 0); // No fees claimed
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_quote_only_config_full_flow_success() {
        let pool_data = create_test_pool_data();
        let quote_mint = pool_data.token_mint_a;
        let position_data = create_test_position_data(256, 384); // Valid position above current
        
        let result = validate_quote_only_config(&pool_data, &position_data, &quote_mint);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TokenOrder::QuoteIsTokenA);
    }

    #[test]
    fn test_validate_quote_only_config_full_flow_invalid_mint() {
        let pool_data = create_test_pool_data();
        let invalid_quote_mint = Pubkey::new_unique(); // Not in pool
        let position_data = create_test_position_data(256, 384);
        
        let result = validate_quote_only_config(&pool_data, &position_data, &invalid_quote_mint);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_quote_only_config_full_flow_invalid_position() {
        let pool_data = create_test_pool_data();
        let quote_mint = pool_data.token_mint_a;
        let position_data = create_test_position_data(-128, 128); // Spans current price
        
        let result = validate_quote_only_config(&pool_data, &position_data, &quote_mint);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_order_equality() {
        assert_eq!(TokenOrder::QuoteIsTokenA, TokenOrder::QuoteIsTokenA);
        assert_eq!(TokenOrder::QuoteIsTokenB, TokenOrder::QuoteIsTokenB);
        assert_ne!(TokenOrder::QuoteIsTokenA, TokenOrder::QuoteIsTokenB);
    }

    #[test]
    fn test_edge_case_current_tick_boundaries() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = 1024; // Use a tick aligned to spacing (64)
        pool_data.tick_spacing = 64;
        
        // Test exact boundary conditions for quote is token A
        let position_data_at_boundary = create_test_position_data(1024, 1088); // tick_lower == current_tick
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data_at_boundary,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err()); // Should fail because tick_lower == current_tick
        
        let position_data_just_above = create_test_position_data(1088, 1152); // tick_lower > current_tick
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data_just_above,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok()); // Should pass because tick_lower > current_tick
    }

    #[test]
    fn test_edge_case_large_tick_values() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = i32::MAX - 1000;
        pool_data.tick_spacing = 1;
        
        // Test with large tick values near i32::MAX
        let position_data = create_test_position_data(i32::MAX - 500, i32::MAX - 100);
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_edge_case_negative_tick_values() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = i32::MIN + 1000;
        pool_data.tick_spacing = 1;
        
        // Test with large negative tick values near i32::MIN
        let position_data = create_test_position_data(i32::MIN + 100, i32::MIN + 500);
        let result = validate_tick_range_for_quote_only(
            &pool_data,
            &position_data,
            TokenOrder::QuoteIsTokenB
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_various_tick_spacings() {
        let spacings = vec![1, 8, 16, 32, 64, 128, 256];
        
        for spacing in spacings {
            let mut pool_data = create_test_pool_data();
            pool_data.tick_spacing = spacing;
            pool_data.tick_current = 0;
            
            // Create position aligned to this spacing
            let tick_lower = spacing as i32 * 4; // 4 spacings above current
            let tick_upper = spacing as i32 * 6; // 6 spacings above current
            let position_data = create_test_position_data(tick_lower, tick_upper);
            
            let result = validate_tick_range_for_quote_only(
                &pool_data,
                &position_data,
                TokenOrder::QuoteIsTokenA
            );
            assert!(result.is_ok(), "Failed for tick spacing: {}", spacing);
        }
    }

    #[test]
    fn test_safety_buffer_calculation() {
        let mut pool_data = create_test_pool_data();
        pool_data.tick_spacing = 100;
        pool_data.tick_current = 1000;
        
        // Safety buffer should be 2 * tick_spacing = 200
        // So for QuoteIsTokenA, tick_lower must be > current_tick + 200 = 1200
        
        let position_just_inside_buffer = create_test_position_data(1200, 1300);
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_just_inside_buffer,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_err()); // Should fail (not > 1200)
        
        let position_outside_buffer = create_test_position_data(1300, 1400);
        let result = preflight_validate_quote_only(
            &pool_data,
            &position_outside_buffer,
            TokenOrder::QuoteIsTokenA
        );
        assert!(result.is_ok()); // Should pass (> 1200)
    }

    #[test]
    fn test_extract_pool_validation_data_placeholder() {
        // Create a mock account info
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = vec![0u8; 100];
        let owner = Pubkey::new_unique();
        
        let account_info = AccountInfo::new(
            &key,
            false,
            false,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );
        
        // This should return an error since it's a placeholder implementation
        let result = extract_pool_validation_data(&account_info);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_position_validation_data_placeholder() {
        // Create a mock account info
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = vec![0u8; 100];
        let owner = Pubkey::new_unique();
        
        let account_info = AccountInfo::new(
            &key,
            false,
            false,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );
        
        // This should return an error since it's a placeholder implementation
        let result = extract_position_validation_data(&account_info);
        assert!(result.is_err());
    }

    /// Test complete validation flow with various scenarios
    #[test]
    fn test_complete_validation_scenario_1() {
        // Scenario: USDC/SOL pool where USDC is quote (token A)
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = 0;
        pool_data.tick_spacing = 64;
        
        let quote_mint = pool_data.token_mint_a; // USDC as quote
        let position_data = create_test_position_data(256, 384); // Well above current price
        
        // This should succeed - position only accrues USDC fees
        let result = validate_quote_only_config(&pool_data, &position_data, &quote_mint);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TokenOrder::QuoteIsTokenA);
    }

    #[test]
    fn test_complete_validation_scenario_2() {
        // Scenario: SOL/USDC pool where USDC is quote (token B)
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = 0;
        pool_data.tick_spacing = 64;
        
        let quote_mint = pool_data.token_mint_b; // USDC as quote
        let position_data = create_test_position_data(-384, -256); // Well below current price
        
        // This should succeed - position only accrues USDC fees
        let result = validate_quote_only_config(&pool_data, &position_data, &quote_mint);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TokenOrder::QuoteIsTokenB);
    }

    #[test]
    fn test_complete_validation_scenario_failure() {
        // Scenario: Attempting to create position that would accrue both tokens
        let mut pool_data = create_test_pool_data();
        pool_data.tick_current = 1000;
        pool_data.tick_spacing = 64;
        
        let quote_mint = pool_data.token_mint_a;
        let position_data = create_test_position_data(896, 1152); // Spans current price
        
        // This should fail - position would accrue both base and quote fees
        let result = validate_quote_only_config(&pool_data, &position_data, &quote_mint);
        assert!(result.is_err());
    }
}

/// Validate pagination parameters
pub fn validate_pagination_params(
    cursor: u32,
    page_size: u32,
    max_investors: u32,
) -> Result<()> {
    require!(
        page_size > 0 && page_size <= crate::constants::MAX_PAGE_SIZE,
        ErrorCode::InvalidPaginationCursor
    );
    
    require!(
        cursor <= max_investors,
        ErrorCode::InvalidPaginationCursor
    );
    
    Ok(())
}

/// Validate 24-hour cooldown
pub fn validate_cooldown(
    last_distribution_ts: i64,
    current_ts: i64,
) -> Result<()> {
    let elapsed = current_ts.saturating_sub(last_distribution_ts);
    require!(
        elapsed >= crate::constants::TWENTY_FOUR_HOURS,
        ErrorCode::CooldownNotElapsed
    );
    
    Ok(())
}

/// Validate investor fee share basis points
pub fn validate_investor_fee_share(fee_share_bps: u16) -> Result<()> {
    require!(
        fee_share_bps <= crate::constants::MAX_BASIS_POINTS,
        ErrorCode::InvalidInvestorFeeShare
    );
    
    Ok(())
}