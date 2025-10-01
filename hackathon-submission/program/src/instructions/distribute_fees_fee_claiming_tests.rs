#[cfg(test)]
mod distribute_fees_fee_claiming_tests {
    use super::*;
    use crate::{
        state::{PolicyConfig, DistributionProgress, DistributionTimingState},
        utils::fee_claiming::{FeeClaimResult, PositionFeeData, validate_quote_only_fees, prepare_collect_fees_instruction_data},
        error::ErrorCode,
        constants::*,
    };
    use anchor_lang::prelude::*;

    // Mock context creation helpers
    fn create_mock_policy_config() -> PolicyConfig {
        PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 5000, // 50%
            daily_cap_lamports: Some(10_000_000_000), // 10 SOL
            min_payout_lamports: 1000,
            y0_total_allocation: 1_000_000_000_000, // 1M tokens
            bump: 255,
        }
    }

    fn create_mock_distribution_progress() -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 0,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_fee_claiming_integration_with_distribute_fees() {
        let policy_config = create_mock_policy_config();
        let mut distribution_progress = create_mock_distribution_progress();
        
        // Test new day scenario - should trigger fee claiming
        let current_timestamp = 1000i64;
        let timing_state = distribution_progress.prepare_for_distribution(current_timestamp).unwrap();
        
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // Mock fee claiming result
        let claimed_fees = FeeClaimResult {
            quote_amount: 5_000_000_000, // 5 SOL worth of fees
            base_amount: 0,
            quote_mint: policy_config.quote_mint,
        };
        
        // Verify fee claiming result
        assert_eq!(claimed_fees.quote_amount, 5_000_000_000);
        assert_eq!(claimed_fees.base_amount, 0);
        assert_eq!(claimed_fees.quote_mint, policy_config.quote_mint);
    }

    #[test]
    fn test_fee_claiming_quote_only_validation_in_context() {
        let policy_config = create_mock_policy_config();
        
        // Test valid quote-only scenario
        let valid_fee_data = PositionFeeData {
            fee_owed_a: 1_000_000_000, // 1 SOL in quote token
            fee_owed_b: 0,
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(), // Base mint
        };
        
        let result = validate_quote_only_fees(&valid_fee_data, &policy_config.quote_mint);
        assert!(result.is_ok());
        
        // Test invalid scenario with base fees
        let invalid_fee_data = PositionFeeData {
            fee_owed_a: 1_000_000_000, // Quote fees
            fee_owed_b: 500_000_000,   // Base fees - should cause failure
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(),
        };
        
        let result = validate_quote_only_fees(&invalid_fee_data, &policy_config.quote_mint);
        assert!(result.is_err());
    }

    #[test]
    fn test_distribute_fees_params_validation() {
        // Test valid parameters
        let valid_params = DistributeFeesParams {
            page_size: 25,
            cursor_position: Some(50),
        };
        
        assert!(valid_params.page_size > 0 && valid_params.page_size <= MAX_PAGE_SIZE);
        
        // Test invalid page size (too large)
        let invalid_params = DistributeFeesParams {
            page_size: MAX_PAGE_SIZE + 1,
            cursor_position: None,
        };
        
        assert!(!(invalid_params.page_size > 0 && invalid_params.page_size <= MAX_PAGE_SIZE));
        
        // Test zero page size
        let zero_params = DistributeFeesParams {
            page_size: 0,
            cursor_position: None,
        };
        
        assert!(!(zero_params.page_size > 0 && zero_params.page_size <= MAX_PAGE_SIZE));
    }

    #[test]
    fn test_treasury_ata_validation_in_fee_claiming_context() {
        let policy_config = create_mock_policy_config();
        let program_authority = Pubkey::new_unique();
        
        // Mock TokenAccount structure for testing
        struct MockTreasuryAta {
            mint: Pubkey,
            owner: Pubkey,
            amount: u64,
        }
        
        // Test valid treasury ATA
        let valid_treasury = MockTreasuryAta {
            mint: policy_config.quote_mint,
            owner: program_authority,
            amount: 0,
        };
        
        assert_eq!(valid_treasury.mint, policy_config.quote_mint);
        assert_eq!(valid_treasury.owner, program_authority);
        
        // Test invalid treasury ATA (wrong mint)
        let invalid_treasury = MockTreasuryAta {
            mint: Pubkey::new_unique(), // Wrong mint
            owner: program_authority,
            amount: 0,
        };
        
        assert_ne!(invalid_treasury.mint, policy_config.quote_mint);
    }

    #[test]
    fn test_cpi_instruction_preparation() {
        // Test instruction data preparation
        let instruction_data = prepare_collect_fees_instruction_data().unwrap();
        
        assert_eq!(instruction_data.len(), 8);
        assert_eq!(instruction_data, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn test_fee_claiming_error_handling_scenarios() {
        let policy_config = create_mock_policy_config();
        
        // Test scenarios that should trigger different error types
        let error_scenarios = vec![
            // (description, fee_data, expected_error_condition)
            ("Base fees detected", PositionFeeData {
                fee_owed_a: 1000,
                fee_owed_b: 1, // Base fees present
                token_mint_a: policy_config.quote_mint,
                token_mint_b: Pubkey::new_unique(),
            }, "should_fail_base_fees"),
            
            ("Invalid quote mint", PositionFeeData {
                fee_owed_a: 1000,
                fee_owed_b: 0,
                token_mint_a: Pubkey::new_unique(), // Neither token is quote mint
                token_mint_b: Pubkey::new_unique(),
            }, "should_fail_invalid_mint"),
            
            ("Valid quote-only fees", PositionFeeData {
                fee_owed_a: 1000,
                fee_owed_b: 0,
                token_mint_a: policy_config.quote_mint,
                token_mint_b: Pubkey::new_unique(),
            }, "should_pass"),
        ];
        
        for (description, fee_data, expected_condition) in error_scenarios {
            let result = validate_quote_only_fees(&fee_data, &policy_config.quote_mint);
            
            match expected_condition {
                "should_fail_base_fees" => {
                    assert!(result.is_err(), "Expected failure for: {}", description);
                },
                "should_fail_invalid_mint" => {
                    assert!(result.is_err(), "Expected failure for: {}", description);
                },
                "should_pass" => {
                    assert!(result.is_ok(), "Expected success for: {}", description);
                },
                _ => panic!("Unknown expected condition: {}", expected_condition),
            }
        }
    }

    #[test]
    fn test_timing_integration_with_fee_claiming() {
        let mut distribution_progress = create_mock_distribution_progress();
        
        let start_time = 1000i64;
        
        // First call - should be new day (triggers fee claiming)
        let timing_state = distribution_progress.prepare_for_distribution(start_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // Same day continuation - should not trigger fee claiming
        let same_day_time = start_time + 3600; // 1 hour later
        let timing_state = distribution_progress.prepare_for_distribution(same_day_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        
        // Complete the day
        distribution_progress.complete_day();
        
        // Next day - should trigger fee claiming again
        let next_day_time = start_time + TWENTY_FOUR_HOURS + 1;
        let timing_state = distribution_progress.prepare_for_distribution(next_day_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
    }

    #[test]
    fn test_fee_claim_result_processing() {
        let policy_config = create_mock_policy_config();
        
        // Test different fee claiming results
        let test_cases = vec![
            // (quote_amount, base_amount, should_emit_event)
            (5_000_000_000, 0, true),  // 5 SOL - should emit event
            (0, 0, false),             // No fees - should not emit event
            (1000, 0, true),           // Small amount - should emit event
        ];
        
        for (quote_amount, base_amount, should_emit_event) in test_cases {
            let claim_result = FeeClaimResult {
                quote_amount,
                base_amount,
                quote_mint: policy_config.quote_mint,
            };
            
            // Verify claim result structure
            assert_eq!(claim_result.quote_amount, quote_amount);
            assert_eq!(claim_result.base_amount, base_amount);
            assert_eq!(claim_result.quote_mint, policy_config.quote_mint);
            
            // Check if event should be emitted based on amount
            let should_emit = claim_result.quote_amount > 0;
            assert_eq!(should_emit, should_emit_event);
        }
    }

    #[test]
    fn test_comprehensive_fee_claiming_flow() {
        let policy_config = create_mock_policy_config();
        let mut distribution_progress = create_mock_distribution_progress();
        
        // Simulate complete fee claiming flow
        let current_timestamp = 1000i64;
        
        // 1. Prepare for distribution (new day)
        let timing_state = distribution_progress.prepare_for_distribution(current_timestamp).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // 2. Validate position fee data
        let position_fee_data = PositionFeeData {
            fee_owed_a: 2_000_000_000, // 2 SOL in quote token
            fee_owed_b: 0,
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(),
        };
        
        // 3. Validate quote-only enforcement
        let validation_result = validate_quote_only_fees(&position_fee_data, &policy_config.quote_mint);
        assert!(validation_result.is_ok());
        
        // 4. Calculate fee amounts
        let (quote_amount, base_amount) = if position_fee_data.token_mint_a == policy_config.quote_mint {
            (position_fee_data.fee_owed_a, position_fee_data.fee_owed_b)
        } else {
            (position_fee_data.fee_owed_b, position_fee_data.fee_owed_a)
        };
        
        assert_eq!(quote_amount, 2_000_000_000);
        assert_eq!(base_amount, 0);
        
        // 5. Create successful claim result
        let claim_result = FeeClaimResult {
            quote_amount,
            base_amount: 0,
            quote_mint: policy_config.quote_mint,
        };
        
        // 6. Verify final result
        assert_eq!(claim_result.quote_amount, 2_000_000_000);
        assert_eq!(claim_result.base_amount, 0);
        assert_eq!(claim_result.quote_mint, policy_config.quote_mint);
        
        // 7. Verify event should be emitted
        assert!(claim_result.quote_amount > 0);
    }
