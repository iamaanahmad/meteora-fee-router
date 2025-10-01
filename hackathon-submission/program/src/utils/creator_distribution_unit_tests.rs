use anchor_lang::prelude::*;
use crate::{
    state::{PolicyConfig, DistributionProgress},
    utils::creator_distribution::{CreatorDistribution, DayCompletionStats},
    error::ErrorCode,
};

/// Comprehensive unit tests for creator distribution functionality
/// These tests focus on the mathematical calculations and business logic
/// without requiring full Anchor context
#[cfg(test)]
mod creator_distribution_unit_tests {
    use super::*;

    fn create_test_policy_config() -> PolicyConfig {
        PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 8000, // 80%
            daily_cap_lamports: Some(1_000_000),
            min_payout_lamports: 1000,
            y0_total_allocation: 10_000_000,
            bump: 255,
        }
    }

    fn create_test_distribution_progress(day_complete: bool) -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1000,
            current_day_distributed: 750_000,
            carry_over_dust: 500,
            pagination_cursor: 50,
            day_complete,
            bump: 255,
        }
    }

    #[test]
    fn test_calculate_creator_remainder_basic_scenarios() {
        let policy_config = create_test_policy_config();

        // Test 1: 50% locked tokens
        let claimed_quote = 1000u64;
        let total_locked = 5_000_000u64; // 50% of Y0

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(80%, 50%) = 50%
        // investor_amount = 1000 * 50% = 500
        // creator_amount = 1000 - 500 = 500
        assert_eq!(creator_amount, 500);

        // Test 2: 25% locked tokens
        let total_locked = 2_500_000u64; // 25% of Y0
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 25%, eligible_share = min(80%, 25%) = 25%
        // creator_amount = 1000 - 250 = 750
        assert_eq!(creator_amount, 750);

        // Test 3: 90% locked tokens (exceeds investor fee share)
        let total_locked = 9_000_000u64; // 90% of Y0
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 90%, eligible_share = min(80%, 90%) = 80%
        // creator_amount = 1000 - 800 = 200
        assert_eq!(creator_amount, 200);
    }

    #[test]
    fn test_calculate_creator_remainder_edge_cases() {
        let policy_config = create_test_policy_config();

        // Test 1: No locked tokens (all unlocked)
        let claimed_quote = 1000u64;
        let total_locked = 0u64;

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // All tokens unlocked, creator gets 100%
        assert_eq!(creator_amount, 1000);

        // Test 2: All tokens locked
        let total_locked = 10_000_000u64; // 100% of Y0
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 100%, eligible_share = min(80%, 100%) = 80%
        // creator_amount = 1000 - 800 = 200
        assert_eq!(creator_amount, 200);

        // Test 3: Zero claimed amount
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            0,
            total_locked,
        ).unwrap();

        assert_eq!(creator_amount, 0);

        // Test 4: Very small amounts
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            1,
            5_000_000,
        ).unwrap();

        // Should handle precision correctly (1 * 50% = 0.5 -> 0, so creator gets 1)
        assert_eq!(creator_amount, 1);
    }

    #[test]
    fn test_calculate_creator_remainder_different_fee_shares() {
        let mut policy_config = create_test_policy_config();
        let claimed_quote = 1000u64;
        let total_locked = 5_000_000u64; // 50% locked

        // Test with 60% investor fee share
        policy_config.investor_fee_share_bps = 6000;
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(60%, 50%) = 50%
        // creator gets 50%
        assert_eq!(creator_amount, 500);

        // Test with 30% investor fee share
        policy_config.investor_fee_share_bps = 3000;
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(30%, 50%) = 30%
        // creator gets 70%
        assert_eq!(creator_amount, 700);

        // Test with 100% investor fee share
        policy_config.investor_fee_share_bps = 10000;
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(100%, 50%) = 50%
        // creator gets 50%
        assert_eq!(creator_amount, 500);

        // Test with 0% investor fee share
        policy_config.investor_fee_share_bps = 0;
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(0%, 50%) = 0%
        // creator gets 100%
        assert_eq!(creator_amount, 1000);
    }

    #[test]
    fn test_get_creator_ata_address_deterministic() {
        let creator_wallet = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let ata_address1 = CreatorDistribution::get_creator_ata_address(
            &creator_wallet,
            &quote_mint,
        );

        let ata_address2 = CreatorDistribution::get_creator_ata_address(
            &creator_wallet,
            &quote_mint,
        );

        // Should be deterministic
        assert_eq!(ata_address1, ata_address2);
        assert_ne!(ata_address1, Pubkey::default());

        // Different inputs should produce different addresses
        let different_wallet = Pubkey::new_unique();
        let ata_address3 = CreatorDistribution::get_creator_ata_address(
            &different_wallet,
            &quote_mint,
        );

        assert_ne!(ata_address1, ata_address3);
    }

    #[test]
    fn test_validate_creator_payout_params_success() {
        let policy_config = create_test_policy_config();
        let distribution_progress = create_test_distribution_progress(true);

        let result = CreatorDistribution::validate_creator_payout_params(
            &policy_config,
            &distribution_progress,
            1000,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_creator_payout_params_with_invalid_policy() {
        let mut policy_config = create_test_policy_config();
        let distribution_progress = create_test_distribution_progress(true);

        // Test with invalid policy config
        policy_config.investor_fee_share_bps = 10001; // Invalid BPS
        let result = CreatorDistribution::validate_creator_payout_params(
            &policy_config,
            &distribution_progress,
            1000,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_day_completion_stats_basic() {
        let policy_config = create_test_policy_config();
        let distribution_progress = create_test_distribution_progress(true);

        let claimed_quote = 2000u64;
        let total_locked = 5_000_000u64; // 50% locked

        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            claimed_quote,
            total_locked,
        ).unwrap();

        assert_eq!(stats.claimed_quote_amount, 2000);
        assert_eq!(stats.total_investor_amount, 1000); // 50% of 2000
        assert_eq!(stats.creator_amount, 1000); // Remainder
        assert_eq!(stats.investor_distributed, 750_000); // From progress
        assert_eq!(stats.total_distributed, 751_000); // 750_000 + 1000
        assert_eq!(stats.carry_over_dust, 500); // From progress
    }

    #[test]
    fn test_calculate_day_completion_stats_edge_cases() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(true);
        distribution_progress.current_day_distributed = 0;
        distribution_progress.carry_over_dust = 0;

        // Test 1: All tokens unlocked
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            1000,
            0, // No locked tokens
        ).unwrap();

        assert_eq!(stats.total_investor_amount, 0);
        assert_eq!(stats.creator_amount, 1000);
        assert_eq!(stats.total_distributed, 1000);

        // Test 2: All tokens locked
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            1000,
            10_000_000, // Fully locked
        ).unwrap();

        assert_eq!(stats.total_investor_amount, 800); // 80% to investors
        assert_eq!(stats.creator_amount, 200); // 20% to creator
        assert_eq!(stats.total_distributed, 200);

        // Test 3: Zero claimed amount
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            0,
            5_000_000,
        ).unwrap();

        assert_eq!(stats.claimed_quote_amount, 0);
        assert_eq!(stats.total_investor_amount, 0);
        assert_eq!(stats.creator_amount, 0);
        assert_eq!(stats.total_distributed, 0);
    }

    #[test]
    fn test_calculate_day_completion_stats_overflow_protection() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(true);
        distribution_progress.current_day_distributed = u64::MAX - 100;

        // Should handle overflow gracefully
        let result = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            1000,
            5_000_000,
        );

        assert!(result.is_err()); // Should fail due to overflow
    }

    #[test]
    fn test_creator_remainder_precision_large_numbers() {
        let mut policy_config = create_test_policy_config();

        // Test with very large numbers
        let claimed_quote = 1_000_000_000u64; // 1B tokens
        let total_locked = 500_000_000u64; // 500M locked (50% of Y0)

        // Align Y0 allocation scale with large amount scenario to maintain 50% lock ratio
        policy_config.y0_total_allocation = 1_000_000_000;

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 50%, eligible_share = min(80%, 50%) = 50%
        // creator gets 50%
        assert_eq!(creator_amount, 500_000_000);

        // Test with maximum values that don't overflow
        let claimed_quote = u32::MAX as u64; // Large but safe
        let total_locked = (policy_config.y0_total_allocation as u64) / 4; // 25% locked

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // Should handle large numbers without overflow
        assert!(creator_amount <= claimed_quote);
        assert!(creator_amount > 0);
    }

    #[test]
    fn test_creator_remainder_with_different_y0_allocations() {
        let mut policy_config = create_test_policy_config();
        let claimed_quote = 1000u64;

        // Test with different Y0 total allocations
        let test_cases = [
            (1_000_000u64, 500_000u64),   // 50% locked
            (5_000_000u64, 2_500_000u64), // 50% locked
            (20_000_000u64, 10_000_000u64), // 50% locked
        ];

        for (y0_total, locked_amount) in test_cases {
            policy_config.y0_total_allocation = y0_total;

            let creator_amount = CreatorDistribution::calculate_creator_remainder(
                &policy_config,
                claimed_quote,
                locked_amount,
            ).unwrap();

            // All cases have 50% locked, so creator should get 50%
            assert_eq!(creator_amount, 500);
        }
    }

    #[test]
    fn test_day_completion_stats_consistency() {
        let policy_config = create_test_policy_config();
        let distribution_progress = create_test_distribution_progress(true);

        let claimed_quote = 1500u64;
        let total_locked = 7_500_000u64; // 75% locked

        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            claimed_quote,
            total_locked,
        ).unwrap();

        // Verify consistency: total_investor_amount + creator_amount = claimed_quote
        assert_eq!(
            stats.total_investor_amount + stats.creator_amount,
            stats.claimed_quote_amount
        );

        // Verify total_distributed = investor_distributed + creator_amount
        assert_eq!(
            stats.total_distributed,
            stats.investor_distributed + stats.creator_amount
        );

        // f_locked = 75%, eligible_share = min(80%, 75%) = 75%
        // investor_amount = 1500 * 75% = 1125
        // creator_amount = 1500 - 1125 = 375
        assert_eq!(stats.total_investor_amount, 1125);
        assert_eq!(stats.creator_amount, 375);
    }

    #[test]
    fn test_creator_remainder_boundary_conditions() {
        let mut policy_config = create_test_policy_config();
        let claimed_quote = 10000u64;

        // Test boundary: locked amount exactly equals investor fee share
        policy_config.investor_fee_share_bps = 6000; // 60%
        let total_locked = 6_000_000u64; // 60% of Y0

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // f_locked = 60%, eligible_share = min(60%, 60%) = 60%
        // creator gets 40%
        assert_eq!(creator_amount, 4000);

        // Test boundary: locked amount slightly less than investor fee share
        let total_locked = 5_999_999u64; // Just under 60%

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // Should be very close to 40% but slightly more due to rounding
        assert!(creator_amount >= 4000);
        assert!(creator_amount <= claimed_quote);

        // Test boundary: locked amount slightly more than investor fee share
        let total_locked = 6_000_001u64; // Just over 60%

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        // Should be exactly 40% since eligible share is capped at 60%
        assert_eq!(creator_amount, 4000);
    }

    #[test]
    fn test_creator_remainder_mathematical_properties() {
        let policy_config = create_test_policy_config();

        // Property 1: Creator amount should never exceed claimed amount
        let test_cases = [
            (1000u64, 0u64),
            (1000u64, 2_500_000u64),
            (1000u64, 5_000_000u64),
            (1000u64, 7_500_000u64),
            (1000u64, 10_000_000u64),
        ];

        for (claimed, locked) in test_cases {
            let creator_amount = CreatorDistribution::calculate_creator_remainder(
                &policy_config,
                claimed,
                locked,
            ).unwrap();

            assert!(creator_amount <= claimed);
        }

        // Property 2: As locked amount increases, creator amount should decrease (monotonic)
        let claimed_quote = 1000u64;
        let mut prev_creator_amount = u64::MAX;

        for locked_pct in [0, 25, 50, 75, 100] {
            let locked_amount = (policy_config.y0_total_allocation as u64 * locked_pct) / 100;
            let creator_amount = CreatorDistribution::calculate_creator_remainder(
                &policy_config,
                claimed_quote,
                locked_amount,
            ).unwrap();

            if prev_creator_amount != u64::MAX {
                assert!(creator_amount <= prev_creator_amount);
            }
            prev_creator_amount = creator_amount;
        }
    }

    #[test]
    fn test_creator_distribution_with_investor_distribution_integration() {
        let policy_config = create_test_policy_config();
        let claimed_quote = 10000u64;
        let total_locked = 6_000_000u64; // 60% locked

        // Calculate distribution split
        let (investor_amount, creator_amount) = crate::utils::math::calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();

        // Verify using creator distribution utility
        let creator_remainder = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();

        assert_eq!(creator_amount, creator_remainder);
        assert_eq!(investor_amount + creator_amount, claimed_quote);

        // f_locked = 60%, eligible_share = min(80%, 60%) = 60%
        // investor gets 60%, creator gets 40%
        assert_eq!(investor_amount, 6000);
        assert_eq!(creator_amount, 4000);
    }

    #[test]
    fn test_full_distribution_cycle_simulation() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(false);

        // Simulate a full distribution cycle
        let claimed_quote = 5000u64;
        let total_locked = 7_500_000u64; // 75% locked

        // Step 1: Calculate initial distribution
        let (investor_amount, creator_amount) = crate::utils::math::calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();

        // Step 2: Simulate investor distribution (simplified)
        distribution_progress.current_day_distributed = investor_amount;

        // Step 3: Complete the day
        distribution_progress.complete_day();

        // Step 4: Calculate day completion stats
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            claimed_quote,
            total_locked,
        ).unwrap();

        // Verify consistency
        assert_eq!(stats.creator_amount, creator_amount);
        assert_eq!(stats.total_investor_amount, investor_amount);
        assert_eq!(stats.investor_distributed, investor_amount);
        assert_eq!(
            stats.total_distributed,
            investor_amount + creator_amount
        );

        // f_locked = 75%, eligible_share = min(80%, 75%) = 75%
        // investor gets 75%, creator gets 25%
        assert_eq!(investor_amount, 3750);
        assert_eq!(creator_amount, 1250);
    }

    #[test]
    fn test_creator_distribution_error_propagation() {
        let mut policy_config = create_test_policy_config();

        // Test with invalid policy configuration
        policy_config.investor_fee_share_bps = 10001; // Invalid

        let result = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            1000,
            5_000_000,
        );

        assert!(result.is_err());

        // Test with zero Y0 allocation
        policy_config.investor_fee_share_bps = 8000; // Valid
        policy_config.y0_total_allocation = 0; // Invalid

        let result = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            1000,
            5_000_000,
        );

        // Should handle gracefully (returns all to creator)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1000);
    }

    #[test]
    fn test_creator_payout_scenarios_comprehensive() {
        let policy_config = create_test_policy_config();

        // Scenario 1: Early stage - most tokens locked
        let claimed_quote = 10000u64;
        let total_locked = 9_500_000u64; // 95% locked
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        // f_locked = 95%, eligible_share = min(80%, 95%) = 80%
        // creator gets 20%
        assert_eq!(creator_amount, 2000);

        // Scenario 2: Mid stage - half tokens locked
        let total_locked = 5_000_000u64; // 50% locked
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        // f_locked = 50%, eligible_share = min(80%, 50%) = 50%
        // creator gets 50%
        assert_eq!(creator_amount, 5000);

        // Scenario 3: Late stage - few tokens locked
        let total_locked = 1_000_000u64; // 10% locked
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        // f_locked = 10%, eligible_share = min(80%, 10%) = 10%
        // creator gets 90%
        assert_eq!(creator_amount, 9000);

        // Scenario 4: Fully vested - no tokens locked
        let total_locked = 0u64; // 0% locked
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        // creator gets 100%
        assert_eq!(creator_amount, 10000);
    }

    #[test]
    fn test_creator_payout_with_different_claim_amounts() {
        let policy_config = create_test_policy_config();
        let total_locked = 5_000_000u64; // 50% locked

        // Test various claim amounts
        let test_amounts = [1u64, 100u64, 1000u64, 10000u64, 100000u64, 1000000u64];

        for claimed_quote in test_amounts {
            let creator_amount = CreatorDistribution::calculate_creator_remainder(
                &policy_config,
                claimed_quote,
                total_locked,
            ).unwrap();

            let (_, expected_creator_amount) = crate::utils::math::calculate_distribution(
                claimed_quote,
                total_locked,
                policy_config.y0_total_allocation,
                policy_config.investor_fee_share_bps,
            ).unwrap();

            assert_eq!(creator_amount, expected_creator_amount);
        }
    }

    #[test]
    fn test_creator_ata_address_generation() {
        // Test that ATA addresses are generated correctly
        let creator_wallet1 = Pubkey::new_unique();
        let creator_wallet2 = Pubkey::new_unique();
        let quote_mint1 = Pubkey::new_unique();
        let quote_mint2 = Pubkey::new_unique();

        // Same wallet, same mint should produce same address
        let ata1 = CreatorDistribution::get_creator_ata_address(&creator_wallet1, &quote_mint1);
        let ata2 = CreatorDistribution::get_creator_ata_address(&creator_wallet1, &quote_mint1);
        assert_eq!(ata1, ata2);

        // Different wallet, same mint should produce different address
        let ata3 = CreatorDistribution::get_creator_ata_address(&creator_wallet2, &quote_mint1);
        assert_ne!(ata1, ata3);

        // Same wallet, different mint should produce different address
        let ata4 = CreatorDistribution::get_creator_ata_address(&creator_wallet1, &quote_mint2);
        assert_ne!(ata1, ata4);

        // All addresses should be valid (non-default)
        assert_ne!(ata1, Pubkey::default());
        assert_ne!(ata3, Pubkey::default());
        assert_ne!(ata4, Pubkey::default());
    }
}