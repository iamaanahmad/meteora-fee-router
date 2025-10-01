#[cfg(test)]
mod investor_distribution_unit_tests {
    use super::*;
    use crate::{
        constants::*,
        utils::math::{
            calculate_distribution, calculate_investor_weight, calculate_individual_payout,
            calculate_dust_payout, enforce_daily_cap,
        },
        state::{PolicyConfig, DistributionProgress},
    };
    use anchor_lang::prelude::Pubkey;

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

    fn create_test_distribution_progress() -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1000,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_investor_payout_structure() {
        let payout = InvestorPayout {
            wallet: Pubkey::new_unique(),
            locked_amount: 2_500_000,
            weight: 250_000, // 25%
            payout_amount: 2500,
            dust_amount: 0,
            ata_address: Pubkey::new_unique(),
            needs_ata_creation: false,
        };
        
        assert_eq!(payout.locked_amount, 2_500_000);
        assert_eq!(payout.weight, 250_000);
        assert_eq!(payout.payout_amount, 2500);
        assert_eq!(payout.dust_amount, 0);
        assert!(!payout.needs_ata_creation);
    }

    #[test]
    fn test_batch_payout_result_structure() {
        let payouts = vec![
            InvestorPayout {
                wallet: Pubkey::new_unique(),
                locked_amount: 2_500_000,
                weight: 250_000,
                payout_amount: 2500,
                dust_amount: 0,
                ata_address: Pubkey::new_unique(),
                needs_ata_creation: false,
            },
            InvestorPayout {
                wallet: Pubkey::new_unique(),
                locked_amount: 500_000,
                weight: 50_000,
                payout_amount: 0,
                dust_amount: 500,
                ata_address: Pubkey::new_unique(),
                needs_ata_creation: true,
            },
        ];
        
        let result = BatchPayoutResult {
            total_paid: 2500,
            total_dust: 500,
            processed_count: 2,
            payouts: payouts.clone(),
        };
        
        assert_eq!(result.total_paid, 2500);
        assert_eq!(result.total_dust, 500);
        assert_eq!(result.processed_count, 2);
        assert_eq!(result.payouts.len(), 2);
    }

    #[test]
    fn test_page_statistics_structure() {
        let stats = PageStatistics {
            total_investors: 20,
            eligible_investors: 16,
            total_locked_amount: 8_000_000,
            total_allocation_amount: 16_000_000,
            page_start: 10,
            page_size: 10,
        };
        
        assert_eq!(stats.total_investors, 20);
        assert_eq!(stats.eligible_investors, 16);
        assert_eq!(stats.total_locked_amount, 8_000_000);
        assert_eq!(stats.total_allocation_amount, 16_000_000);
        assert_eq!(stats.page_start, 10);
        assert_eq!(stats.page_size, 10);
        
        // Calculate derived metrics
        let eligibility_rate = stats.eligible_investors as f64 / stats.total_investors as f64;
        assert_eq!(eligibility_rate, 0.8); // 80% eligible
        
        let lock_rate = stats.total_locked_amount as f64 / stats.total_allocation_amount as f64;
        assert_eq!(lock_rate, 0.5); // 50% locked
    }

    #[test]
    fn test_ata_derivation() {
        let investor_wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        
        let (ata_address, needs_creation) = InvestorDistribution::get_or_derive_investor_ata(
            &investor_wallet,
            &mint,
        ).unwrap();
        
        // Should derive a valid ATA address
        assert_ne!(ata_address, Pubkey::default());
        assert_ne!(ata_address, investor_wallet);
        assert_ne!(ata_address, mint);
        
        // Mock implementation always returns needs_creation = true
        assert!(needs_creation);
        
        // Test that same inputs produce same ATA
        let (ata_address_2, _) = InvestorDistribution::get_or_derive_investor_ata(
            &investor_wallet,
            &mint,
        ).unwrap();
        
        assert_eq!(ata_address, ata_address_2);
    }

    #[test]
    fn test_distribution_calculation_scenarios() {
        let policy_config = create_test_policy_config();
        
        // Test fully locked scenario
        let claimed_quote = 10_000u64;
        let total_locked = 10_000_000u64; // 100% of Y0
        
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();
        
        // f_locked = 10M / 10M = 1.0 = 10000 bps
        // eligible_share = min(8000, 10000) = 8000 bps = 80%
        // investor_amount = 10000 * 80% = 8000
        // creator_amount = 10000 - 8000 = 2000
        assert_eq!(total_investor_amount, 8000);
        assert_eq!(creator_amount, 2000);
        
        // Test partially locked scenario
        let total_locked = 5_000_000u64; // 50% of Y0
        
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();
        
        // f_locked = 5M / 10M = 0.5 = 5000 bps
        // eligible_share = min(8000, 5000) = 5000 bps = 50%
        // investor_amount = 10000 * 50% = 5000
        // creator_amount = 10000 - 5000 = 5000
        assert_eq!(total_investor_amount, 5000);
        assert_eq!(creator_amount, 5000);
        
        // Test no locked scenario
        let total_locked = 0u64; // 0% of Y0
        
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();
        
        // f_locked = 0 / 10M = 0
        // eligible_share = min(8000, 0) = 0
        // investor_amount = 10000 * 0% = 0
        // creator_amount = 10000 - 0 = 10000
        assert_eq!(total_investor_amount, 0);
        assert_eq!(creator_amount, 10000);
    }

    #[test]
    fn test_individual_payout_calculations() {
        let total_investor_amount = 10_000u64;
        let min_payout = 1000u64;
        
        // Test investor with 25% weight (above threshold)
        let weight_25_percent = 250_000u128; // 25% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            weight_25_percent,
            min_payout,
        ).unwrap();
        
        // Expected: 10000 * 25% = 2500
        assert_eq!(payout, 2500);
        assert_eq!(dust, 0);
        
        // Test investor with 5% weight (below threshold)
        let weight_5_percent = 50_000u128; // 5% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            weight_5_percent,
            min_payout,
        ).unwrap();
        
        // Expected: 10000 * 5% = 500, which is < 1000, so becomes dust
        assert_eq!(payout, 0);
        assert_eq!(dust, 500);
        
        // Test investor with exactly threshold amount
        let weight_10_percent = 100_000u128; // 10% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            weight_10_percent,
            min_payout,
        ).unwrap();
        
        // Expected: 10000 * 10% = 1000, which equals threshold
        assert_eq!(payout, 1000);
        assert_eq!(dust, 0);
    }

    #[test]
    fn test_investor_weight_calculations() {
        let total_locked = 10_000_000u64;
        
        // Test 25% investor
        let investor_locked = 2_500_000u64;
        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 250_000); // 25% of WEIGHT_PRECISION
        
        // Test 10% investor
        let investor_locked = 1_000_000u64;
        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 100_000); // 10% of WEIGHT_PRECISION
        
        // Test 50% investor
        let investor_locked = 5_000_000u64;
        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 500_000); // 50% of WEIGHT_PRECISION
        
        // Test zero locked
        let weight = calculate_investor_weight(0, total_locked).unwrap();
        assert_eq!(weight, 0);
        
        // Test zero total (edge case)
        let weight = calculate_investor_weight(1_000_000, 0).unwrap();
        assert_eq!(weight, 0);
    }

    #[test]
    fn test_dust_handling_scenarios() {
        let min_payout = 1000u64;
        
        // Test dust below threshold
        let accumulated_dust = 750u64;
        let (dust_payout, remaining_dust) = calculate_dust_payout(accumulated_dust, min_payout);
        assert_eq!(dust_payout, 0);
        assert_eq!(remaining_dust, 750);
        
        // Test dust exactly at threshold
        let accumulated_dust = 1000u64;
        let (dust_payout, remaining_dust) = calculate_dust_payout(accumulated_dust, min_payout);
        assert_eq!(dust_payout, 1000);
        assert_eq!(remaining_dust, 0);
        
        // Test dust above threshold
        let accumulated_dust = 2750u64;
        let (dust_payout, remaining_dust) = calculate_dust_payout(accumulated_dust, min_payout);
        assert_eq!(dust_payout, 2000); // 2 * 1000
        assert_eq!(remaining_dust, 750);
        
        // Test large dust amount
        let accumulated_dust = 5500u64;
        let (dust_payout, remaining_dust) = calculate_dust_payout(accumulated_dust, min_payout);
        assert_eq!(dust_payout, 5000); // 5 * 1000
        assert_eq!(remaining_dust, 500);
    }

    #[test]
    fn test_daily_cap_enforcement() {
        let mut progress = create_test_distribution_progress();
        
        // Test within cap
        progress.current_day_distributed = 500_000;
        let daily_cap = Some(1_000_000u64);
        let additional_amount = 300_000u64;
        
        let capped_amount = enforce_daily_cap(&progress, additional_amount, daily_cap).unwrap();
        assert_eq!(capped_amount, 300_000); // Within limit
        
        // Test at cap limit
        progress.current_day_distributed = 800_000;
        let additional_amount = 200_000u64;
        
        let capped_amount = enforce_daily_cap(&progress, additional_amount, daily_cap).unwrap();
        assert_eq!(capped_amount, 200_000); // Exactly at limit
        
        // Test exceeding cap
        progress.current_day_distributed = 800_000;
        let additional_amount = 300_000u64;
        
        let capped_amount = enforce_daily_cap(&progress, additional_amount, daily_cap).unwrap();
        assert_eq!(capped_amount, 200_000); // Capped to remaining capacity
        
        // Test already at cap
        progress.current_day_distributed = 1_000_000;
        let additional_amount = 100_000u64;
        
        let result = enforce_daily_cap(&progress, additional_amount, daily_cap);
        assert!(result.is_err()); // Should fail when at cap
        
        // Test no cap
        let capped_amount = enforce_daily_cap(&progress, additional_amount, None).unwrap();
        assert_eq!(capped_amount, additional_amount); // No cap, return full amount
    }

    #[test]
    fn test_scaling_factor_calculations() {
        let original_total = 10_000u64;
        let capped_total = 8_000u64; // 80% due to daily cap
        
        let scale_factor = (capped_total as u128 * WEIGHT_PRECISION) / original_total as u128;
        assert_eq!(scale_factor, 800_000); // 80% of WEIGHT_PRECISION
        
        // Test scaling individual payouts
        let payouts = vec![2500u64, 3000u64, 1500u64, 500u64];
        let scaled_payouts: Vec<u64> = payouts
            .iter()
            .map(|&payout| {
                ((payout as u128 * scale_factor) / WEIGHT_PRECISION) as u64
            })
            .collect();
        
        let expected_scaled = vec![2000u64, 2400u64, 1200u64, 400u64];
        assert_eq!(scaled_payouts, expected_scaled);
        
        // Verify total is correct
        let total_scaled: u64 = scaled_payouts.iter().sum();
        assert_eq!(total_scaled, 6000); // Should be close to capped_total
    }

    #[test]
    fn test_minimum_payout_threshold_scenarios() {
        let total_amount = 10_000u64;
        let min_payout = 1000u64;
        
        // Test various weight scenarios
        let test_cases = vec![
            (100_000u128, 1000u64, 0u64),   // 10% -> 1000, meets threshold
            (50_000u128, 0u64, 500u64),     // 5% -> 500, below threshold (dust)
            (150_000u128, 1500u64, 0u64),   // 15% -> 1500, above threshold
            (1_000u128, 0u64, 10u64),       // 0.1% -> 10, below threshold (dust)
            (0u128, 0u64, 0u64),            // 0% -> 0, no payout or dust
        ];
        
        for (weight, expected_payout, expected_dust) in test_cases {
            let (payout, dust) = calculate_individual_payout(
                total_amount,
                weight,
                min_payout,
            ).unwrap();
            
            assert_eq!(payout, expected_payout, "Weight: {}", weight);
            assert_eq!(dust, expected_dust, "Weight: {}", weight);
        }
    }

    #[test]
    fn test_edge_case_zero_amounts() {
        // Test with zero total amount
        let (payout, dust) = calculate_individual_payout(0, 250_000, 1000).unwrap();
        assert_eq!(payout, 0);
        assert_eq!(dust, 0);
        
        // Test with zero weight
        let (payout, dust) = calculate_individual_payout(10_000, 0, 1000).unwrap();
        assert_eq!(payout, 0);
        assert_eq!(dust, 0);
        
        // Test with zero minimum payout
        let (payout, dust) = calculate_individual_payout(10_000, 250_000, 0).unwrap();
        assert_eq!(payout, 2500); // 25% of 10k
        assert_eq!(dust, 0);
    }

    #[test]
    fn test_precision_and_rounding() {
        let total_amount = 1000u64;
        let min_payout = 1u64; // Very low threshold to test precision
        
        // Test weight that results in fractional amount
        let weight = 333_333u128; // 33.3333% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_amount,
            weight,
            min_payout,
        ).unwrap();
        
        // Should use floor division: 1000 * 333333 / 1000000 = 333
        assert_eq!(payout, 333);
        assert_eq!(dust, 0);
        
        // Test very small weight
        let weight = 1u128; // Minimal weight
        let (payout, dust) = calculate_individual_payout(
            total_amount,
            weight,
            min_payout,
        ).unwrap();
        
        // Should round down to 0: 1000 * 1 / 1000000 = 0
        assert_eq!(payout, 0);
        assert_eq!(dust, 0);
    }

    #[test]
    fn test_distribution_progress_integration() {
        let mut progress = create_test_distribution_progress();
        
        // Test adding distributed amount
        progress.add_distributed(5000).unwrap();
        assert_eq!(progress.current_day_distributed, 5000);
        
        // Test adding dust
        progress.add_dust(250).unwrap();
        assert_eq!(progress.carry_over_dust, 250);
        
        // Test consuming dust
        let consumed = progress.consume_dust(100).unwrap();
        assert_eq!(consumed, 100);
        assert_eq!(progress.carry_over_dust, 150);
        
        // Test consuming more dust than available
        let consumed = progress.consume_dust(200).unwrap();
        assert_eq!(consumed, 150); // Only what's available
        assert_eq!(progress.carry_over_dust, 0);
        
        // Test cursor advancement
        let new_cursor = progress.advance_cursor(10).unwrap();
        assert_eq!(new_cursor, 10);
        assert_eq!(progress.pagination_cursor, 10);
    }

    #[test]
    fn test_validation_parameters() {
        let policy_config = create_test_policy_config();
        
        // Test valid policy configuration
        assert!(policy_config.validate().is_ok());
        
        // Test invalid investor fee share
        let mut invalid_config = policy_config.clone();
        invalid_config.investor_fee_share_bps = 10001; // > 10000
        assert!(invalid_config.validate().is_err());
        
        // Test zero minimum payout
        let mut invalid_config = policy_config.clone();
        invalid_config.min_payout_lamports = 0;
        assert!(invalid_config.validate().is_err());
        
        // Test zero Y0 allocation
        let mut invalid_config = policy_config.clone();
        invalid_config.y0_total_allocation = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_arithmetic_overflow_protection() {
        // Test with values that could cause overflow
        let large_amount = u64::MAX / 2;
        let large_weight = WEIGHT_PRECISION;
        
        let result = calculate_individual_payout(
            large_amount,
            large_weight,
            1000,
        );
        
        // Should handle large values without overflow
        assert!(result.is_ok());
        
        // Test weight calculation with large values
        let result = calculate_investor_weight(u64::MAX / 2, u64::MAX);
        assert!(result.is_ok());
        
        // Test distribution calculation with large values
        let result = calculate_distribution(
            u64::MAX / 4,
            u64::MAX / 2,
            u64::MAX,
            5000,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_distribution_scenario() {
        let policy_config = create_test_policy_config();
        let mut progress = create_test_distribution_progress();
        
        // Simulate a complex distribution scenario
        let claimed_quote = 100_000u64;
        let total_locked = 7_500_000u64; // 75% of Y0
        
        // Calculate distribution
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();
        
        // f_locked = 7.5M / 10M = 0.75 = 7500 bps
        // eligible_share = min(8000, 7500) = 7500 bps = 75%
        // investor_amount = 100000 * 75% = 75000
        // creator_amount = 100000 - 75000 = 25000
        assert_eq!(total_investor_amount, 75000);
        assert_eq!(creator_amount, 25000);
        
        // Simulate individual investor calculations
        let investors = vec![
            (2_000_000u64, "Investor A"), // ~26.67% of locked
            (3_000_000u64, "Investor B"), // ~40% of locked
            (1_500_000u64, "Investor C"), // ~20% of locked
            (1_000_000u64, "Investor D"), // ~13.33% of locked
        ];
        
        let mut total_payouts = 0u64;
        let mut total_dust = 0u64;
        
        for (locked_amount, name) in investors {
            let weight = calculate_investor_weight(locked_amount, total_locked).unwrap();
            let (payout, dust) = calculate_individual_payout(
                total_investor_amount,
                weight,
                policy_config.min_payout_lamports,
            ).unwrap();
            
            total_payouts += payout;
            total_dust += dust;
            
            println!("{}: locked={}, weight={}, payout={}, dust={}", 
                     name, locked_amount, weight, payout, dust);
        }
        
        // Verify totals are reasonable
        assert!(total_payouts <= total_investor_amount);
        assert!(total_payouts + total_dust <= total_investor_amount + 100); // Allow for rounding
        
        // Test daily cap enforcement
        progress.current_day_distributed = 900_000;
        let capped_amount = enforce_daily_cap(
            &progress,
            total_payouts,
            policy_config.daily_cap_lamports,
        ).unwrap();
        
        // Should be capped to remaining capacity
        let expected_cap = 1_000_000 - 900_000;
        assert_eq!(capped_amount, std::cmp::min(total_payouts, expected_cap));
    }
}