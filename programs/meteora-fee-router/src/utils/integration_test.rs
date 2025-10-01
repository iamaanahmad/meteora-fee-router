#[cfg(test)]
mod integration_tests {
    use crate::utils::math::*;
    use crate::utils::streamflow::*;
    use crate::state::{PolicyConfig, DistributionProgress};
    use anchor_lang::prelude::*;

    /// Test integration between math functions and actual state structures
    #[test]
    fn test_math_integration_with_state() {
        // Create a mock policy config
        let policy = PolicyConfig {
            vault: Pubkey::default(),
            quote_mint: Pubkey::default(),
            creator_wallet: Pubkey::default(),
            investor_fee_share_bps: 8000, // 80%
            daily_cap_lamports: Some(1_000_000),
            min_payout_lamports: 1000,
            y0_total_allocation: 10_000_000,
            bump: 255,
        };

        // Create a mock distribution progress
        let mut progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 0,
            current_day_distributed: 250_000,
            carry_over_dust: 500,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        };

        // Test scenario: 50% of tokens are still locked
        let claimed_quote = 100_000u64;
        let locked_total = 5_000_000u64; // 50% of Y0
        
        // Test distribution calculation
        let (investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            locked_total,
            policy.y0_total_allocation,
            policy.investor_fee_share_bps,
        ).unwrap();

        // f_locked = 5M/10M = 0.5 = 5000 bps
        // eligible_share = min(8000, 5000) = 5000 bps = 50%
        // investor_amount = 100K * 50% = 50K
        assert_eq!(investor_amount, 50_000);
        assert_eq!(creator_amount, 50_000);

        // Test daily cap enforcement
        let capped_amount = enforce_daily_cap(
            &progress,
            investor_amount,
            policy.daily_cap_lamports,
        ).unwrap();

        // Current distributed: 250K, cap: 1M, available: 750K
        // Requested: 50K, should get full amount
        assert_eq!(capped_amount, 50_000);

        // Test with amount that would exceed cap
        progress.current_day_distributed = 980_000;
        let capped_amount = enforce_daily_cap(
            &progress,
            investor_amount,
            policy.daily_cap_lamports,
        ).unwrap();

        // Current distributed: 980K, cap: 1M, available: 20K
        // Requested: 50K, should get only 20K
        assert_eq!(capped_amount, 20_000);

        // Test dust handling with carry over
        let dust_payout = calculate_dust_payout(
            progress.carry_over_dust,
            policy.min_payout_lamports,
        );

        // Carry over: 500, min payout: 1000
        // Should not pay out dust (below threshold)
        assert_eq!(dust_payout, (0, 500));

        // Test with more accumulated dust
        let dust_payout = calculate_dust_payout(
            2500, // Accumulated dust
            policy.min_payout_lamports,
        );

        // Should pay out 2 * 1000 = 2000, leaving 500 as dust
        assert_eq!(dust_payout, (2000, 500));
    }

    #[test]
    fn test_investor_weight_calculations() {
        // Test realistic investor scenario
        let investors_locked = vec![
            1_000_000u64, // Investor 1: 1M locked
            2_500_000u64, // Investor 2: 2.5M locked  
            1_500_000u64, // Investor 3: 1.5M locked
            0u64,         // Investor 4: fully unlocked
        ];

        let total_locked = calculate_total_locked(&investors_locked).unwrap();
        assert_eq!(total_locked, 5_000_000);

        let weights = calculate_all_weights(&investors_locked, total_locked).unwrap();
        
        // Verify weights sum to WEIGHT_PRECISION (within rounding error)
        verify_weight_sum(&weights).unwrap();

        // Test individual weight calculations
        assert_eq!(weights[0], 200_000); // 1M/5M * 1_000_000 = 200_000
        assert_eq!(weights[1], 500_000); // 2.5M/5M * 1_000_000 = 500_000
        assert_eq!(weights[2], 300_000); // 1.5M/5M * 1_000_000 = 300_000
        assert_eq!(weights[3], 0);       // 0/5M * 1_000_000 = 0

        // Test batch payout calculation
        let investor_data: Vec<(u64, u128)> = investors_locked
            .iter()
            .zip(weights.iter())
            .map(|(&locked, &weight)| (locked, weight))
            .collect();

        let total_investor_amount = 50_000u64;
        let min_payout = 1_000u64;
        let carry_over_dust = 0u64;

        let (total_paid, total_dust) = calculate_batch_payout(
            &investor_data,
            total_investor_amount,
            min_payout,
            carry_over_dust,
        ).unwrap();

        // Expected payouts:
        // Investor 1: 50K * 200_000 / 1_000_000 = 10K
        // Investor 2: 50K * 500_000 / 1_000_000 = 25K  
        // Investor 3: 50K * 300_000 / 1_000_000 = 15K
        // Investor 4: 50K * 0 / 1_000_000 = 0
        let expected_paid = 10_000 + 25_000 + 15_000;
        assert_eq!(total_paid, expected_paid);
        assert_eq!(total_dust, 0); // No dust since all payouts above threshold
    }

    #[test]
    fn test_edge_case_scenarios() {
        // Test with very small amounts that create dust
        let small_investors = vec![
            100u64, 200u64, 150u64, 50u64
        ];
        
        let total_locked = calculate_total_locked(&small_investors).unwrap();
        let weights = calculate_all_weights(&small_investors, total_locked).unwrap();
        
        let investor_data: Vec<(u64, u128)> = small_investors
            .iter()
            .zip(weights.iter())
            .map(|(&locked, &weight)| (locked, weight))
            .collect();

        let total_investor_amount = 100u64; // Very small amount
        let min_payout = 50u64; // Relatively high threshold
        let carry_over_dust = 0u64;

        let (total_paid, total_dust) = calculate_batch_payout(
            &investor_data,
            total_investor_amount,
            min_payout,
            carry_over_dust,
        ).unwrap();

        // With small amounts, some payouts might be below threshold
        // Verify that total_paid + total_dust <= total_investor_amount
        assert!(total_paid + total_dust <= total_investor_amount);
    }

    #[test]
    fn test_parameter_validation() {
        // Test validation with valid parameters
        let result = validate_distribution_params(
            100_000,  // claimed_quote
            10_000_000, // y0_total
            8000,     // investor_fee_share_bps
            1000,     // min_payout
        );
        assert!(result.is_ok());

        // Test validation with invalid basis points
        let result = validate_distribution_params(
            100_000,
            10_000_000,
            10001, // Invalid: > 10000
            1000,
        );
        assert!(result.is_err());

        // Test validation with zero Y0
        let result = validate_distribution_params(
            100_000,
            0, // Invalid: zero Y0
            8000,
            1000,
        );
        assert!(result.is_err());

        // Test validation with zero min payout
        let result = validate_distribution_params(
            100_000,
            10_000_000,
            8000,
            0, // Invalid: zero min payout
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_streamflow_integration_with_distribution() {
        // Test complete integration: Streamflow -> Math -> Distribution
        let mint = Pubkey::new_unique();
        let investor1 = Pubkey::new_unique();
        let investor2 = Pubkey::new_unique();
        let investor3 = Pubkey::new_unique();

        // Create mock streams with different vesting states
        let current_time = 1500i64; // Midpoint of vesting

        // Investor 1: 1M tokens, 50% vested (500k locked)
        let stream1 = StreamflowStream {
            recipient: investor1,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 1_000_000,
            withdrawn_amount: 0,
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        // Investor 2: 2M tokens, 50% vested (1M locked)
        let stream2 = StreamflowStream {
            recipient: investor2,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 2_000_000,
            withdrawn_amount: 0,
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        // Investor 3: 1M tokens, fully vested (0 locked)
        let stream3 = StreamflowStream {
            recipient: investor3,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 1_000_000,
            withdrawn_amount: 0,
            start_time: 500,
            end_time: 1000, // Already ended
            cliff_time: 500,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        // Calculate locked amounts
        let locked1 = StreamflowIntegration::calculate_locked_amount(&stream1, current_time).unwrap();
        let locked2 = StreamflowIntegration::calculate_locked_amount(&stream2, current_time).unwrap();
        let locked3 = StreamflowIntegration::calculate_locked_amount(&stream3, current_time).unwrap();

        assert_eq!(locked1, 500_000); // 50% of 1M
        assert_eq!(locked2, 1_000_000); // 50% of 2M
        assert_eq!(locked3, 0); // Fully vested

        let total_locked = locked1 + locked2 + locked3;
        assert_eq!(total_locked, 1_500_000);

        // Test distribution calculation with Streamflow data
        let policy = PolicyConfig {
            vault: Pubkey::default(),
            quote_mint: mint,
            creator_wallet: Pubkey::default(),
            investor_fee_share_bps: 8000, // 80%
            daily_cap_lamports: None,
            min_payout_lamports: 1000,
            y0_total_allocation: 4_000_000, // Total allocation
            bump: 255,
        };

        let claimed_quote = 100_000u64;

        // Calculate distribution
        let (investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy.y0_total_allocation,
            policy.investor_fee_share_bps,
        ).unwrap();

        // f_locked = 1.5M / 4M = 0.375 = 3750 bps
        // eligible_share = min(8000, 3750) = 3750 bps = 37.5%
        // investor_amount = 100K * 37.5% = 37.5K
        assert_eq!(investor_amount, 37_500);
        assert_eq!(creator_amount, 62_500);

        // Calculate individual investor weights and payouts
        let locked_amounts = vec![locked1, locked2, locked3];
        let weights = calculate_all_weights(&locked_amounts, total_locked).unwrap();

        // Expected weights (floor division):
        // Investor 1: 500K / 1.5M = 1/3 ≈ 333,333
        // Investor 2: 1M / 1.5M = 2/3 ≈ 666,666
        // Investor 3: 0 / 1.5M = 0
        assert_eq!(weights[0], 333_333); // 500K/1.5M * 1M
        assert_eq!(weights[1], 666_666); // 1M/1.5M * 1M  
        assert_eq!(weights[2], 0);       // 0/1.5M * 1M

        // Calculate individual payouts
        let investor_data: Vec<(u64, u128)> = locked_amounts
            .iter()
            .zip(weights.iter())
            .map(|(&locked, &weight)| (locked, weight))
            .collect();

        let (total_paid, total_dust) = calculate_batch_payout(
            &investor_data,
            investor_amount,
            policy.min_payout_lamports,
            0,
        ).unwrap();

        // Expected payouts (floor division):
        // Investor 1: 37.5K * 333,333 / 1,000,000 ≈ 12,499
        // Investor 2: 37.5K * 666,666 / 1,000,000 ≈ 24,999
        // Investor 3: 37.5K * 0 / 1,000,000 = 0
        let expected_total = 12_499 + 24_999;
        assert_eq!(total_paid, expected_total);
        assert_eq!(total_dust, 2); // Two lamports become dust due to floor rounding

        // Verify total distribution adds up correctly
        assert_eq!(total_paid + total_dust + creator_amount, claimed_quote);
    }

    #[test]
    fn test_streamflow_edge_cases_integration() {
        let mint = Pubkey::new_unique();
        let investor = Pubkey::new_unique();

        // Test with withdrawn tokens
        let stream_with_withdrawals = StreamflowStream {
            recipient: investor,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 1_000_000,
            withdrawn_amount: 300_000, // 30% already withdrawn
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        let current_time = 1500; // 50% through vesting
        let locked = StreamflowIntegration::calculate_locked_amount(&stream_with_withdrawals, current_time).unwrap();
        
        // Available: 1M - 300K = 700K
        // Vested: 50% of 1M = 500K
        // Locked: 700K - 500K = 200K
        assert_eq!(locked, 200_000);

        // Test distribution with this edge case
        let claimed_quote = 10_000u64;
        let y0_total = 1_000_000u64;
        let investor_fee_share_bps = 5000u16; // 50%

        let (investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            locked,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 200K / 1M = 0.2 = 2000 bps
        // eligible_share = min(5000, 2000) = 2000 bps = 20%
        // investor_amount = 10K * 20% = 2K
        assert_eq!(investor_amount, 2_000);
        assert_eq!(creator_amount, 8_000);
    }

    #[test]
    fn test_multiple_streams_per_investor() {
        let mint = Pubkey::new_unique();
        let investor = Pubkey::new_unique();
        let current_time = 1500i64;

        // Investor has multiple streams
        let stream1 = StreamflowStream {
            recipient: investor,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 600_000,
            withdrawn_amount: 0,
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        let stream2 = StreamflowStream {
            recipient: investor,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 400_000,
            withdrawn_amount: 100_000, // Some withdrawn
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
        };

        // Calculate locked amounts for each stream
        let locked1 = StreamflowIntegration::calculate_locked_amount(&stream1, current_time).unwrap();
        let locked2 = StreamflowIntegration::calculate_locked_amount(&stream2, current_time).unwrap();

        // Stream 1: 50% of 600K = 300K locked
        assert_eq!(locked1, 300_000);
        
        // Stream 2: Available = 400K - 100K = 300K, Vested = 50% of 400K = 200K, Locked = 300K - 200K = 100K
        assert_eq!(locked2, 100_000);

        let total_locked_for_investor = locked1 + locked2;
        assert_eq!(total_locked_for_investor, 400_000);

        // Test that this integrates correctly with distribution calculations
        let claimed_quote = 50_000u64;
        let y0_total = 1_000_000u64;
        let investor_fee_share_bps = 6000u16; // 60%

        let (investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked_for_investor,
            y0_total,
            investor_fee_share_bps,
        ).unwrap();

        // f_locked = 400K / 1M = 0.4 = 4000 bps
        // eligible_share = min(6000, 4000) = 4000 bps = 40%
        // investor_amount = 50K * 40% = 20K
        assert_eq!(investor_amount, 20_000);
        assert_eq!(creator_amount, 30_000);
    }
}