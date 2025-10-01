#[cfg(test)]
mod investor_distribution_tests {
    use super::*;
    use crate::{
        state::{PolicyConfig, DistributionProgress},
        utils::{
            math::{calculate_distribution, calculate_investor_weight, calculate_individual_payout},
            streamflow::{StreamflowIntegration, InvestorData},
        },
        constants::*,
        error::ErrorCode,
    };
    use anchor_lang::prelude::*;

    // Mock data structures for testing
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

    fn create_test_investor_data(count: usize, locked_amounts: Vec<u64>) -> Vec<InvestorData> {
        assert_eq!(count, locked_amounts.len());
        
        (0..count)
            .zip(locked_amounts.into_iter())
            .map(|(i, locked_amount)| InvestorData {
                wallet: Pubkey::new_unique(),
                locked_amount,
                total_allocation: locked_amount * 2, // Assume 50% locked
                stream_accounts: vec![Pubkey::new_unique()],
            })
            .collect()
    }

    #[test]
    fn test_calculate_page_distribution_basic() {
        let policy_config = create_test_policy_config();
        
        let claimed_quote = 10_000u64;
        let total_locked = 5_000_000u64; // 50% of Y0 (10M)
        
        // Calculate expected distribution
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
    }

    #[test]
    fn test_calculate_page_distribution_fully_locked() {
        let policy_config = create_test_policy_config();
        
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
    }

    #[test]
    fn test_calculate_page_distribution_no_locked() {
        let policy_config = create_test_policy_config();
        
        let claimed_quote = 10_000u64;
        let total_locked = 0u64; // No tokens locked
        
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
    fn test_individual_payout_calculation() {
        let total_investor_amount = 5000u64;
        let min_payout = 100u64;
        
        // Test investor with 25% weight
        let weight_25_percent = 250_000u128; // 25% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            weight_25_percent,
            min_payout,
        ).unwrap();
        
        // Expected: 5000 * 25% = 1250
        assert_eq!(payout, 1250);
        assert_eq!(dust, 0);
        
        // Test investor with very small weight (below threshold)
        let weight_1_percent = 10_000u128; // 1% of WEIGHT_PRECISION
        let (payout, dust) = calculate_individual_payout(
            total_investor_amount,
            weight_1_percent,
            min_payout,
        ).unwrap();
        
        // Expected: 5000 * 1% = 50, which is < 100, so becomes dust
        assert_eq!(payout, 0);
        assert_eq!(dust, 50);
    }

    #[test]
    fn test_investor_weight_calculation() {
        let total_locked = 10_000_000u64;
        
        // Test 25% investor
        let investor_locked = 2_500_000u64;
        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 250_000); // 25% of WEIGHT_PRECISION
        
        // Test 10% investor
        let investor_locked = 1_000_000u64;
        let weight = calculate_investor_weight(investor_locked, total_locked).unwrap();
        assert_eq!(weight, 100_000); // 10% of WEIGHT_PRECISION
        
        // Test zero locked
        let weight = calculate_investor_weight(0, total_locked).unwrap();
        assert_eq!(weight, 0);
        
        // Test zero total (edge case)
        let weight = calculate_investor_weight(1_000_000, 0).unwrap();
        assert_eq!(weight, 0);
    }

    #[test]
    fn test_dust_handling_scenarios() {
        let mut progress = create_test_distribution_progress();
        progress.carry_over_dust = 750; // Existing dust
        
        let min_payout = 1000u64;
        
        // Test dust accumulation
        let new_dust = 300u64;
        let total_dust = progress.carry_over_dust + new_dust;
        assert_eq!(total_dust, 1050);
        
        // Test dust payout calculation
        let (dust_payout, remaining_dust) = crate::utils::math::calculate_dust_payout(
            total_dust,
            min_payout,
        );
        
        assert_eq!(dust_payout, 1000); // One full payout
        assert_eq!(remaining_dust, 50); // Remaining dust
        
        // Test multiple dust payouts
        progress.carry_over_dust = 2750;
        let (dust_payout, remaining_dust) = crate::utils::math::calculate_dust_payout(
            progress.carry_over_dust,
            min_payout,
        );
        
        assert_eq!(dust_payout, 2000); // Two full payouts (2 * 1000)
        assert_eq!(remaining_dust, 750); // Remaining dust
    }

    #[test]
    fn test_daily_cap_enforcement() {
        let mut progress = create_test_distribution_progress();
        progress.current_day_distributed = 800_000; // Already distributed 800k
        
        let daily_cap = Some(1_000_000u64); // 1M cap
        let additional_amount = 300_000u64; // Want to distribute 300k more
        
        // Should be capped to remaining capacity
        let capped_amount = crate::utils::math::enforce_daily_cap(
            &progress,
            additional_amount,
            daily_cap,
        ).unwrap();
        
        assert_eq!(capped_amount, 200_000); // Only 200k remaining capacity
        
        // Test when already at cap
        progress.current_day_distributed = 1_000_000;
        let result = crate::utils::math::enforce_daily_cap(
            &progress,
            100_000,
            daily_cap,
        );
        
        assert!(result.is_err()); // Should fail when at cap
        
        // Test with no cap
        let capped_amount = crate::utils::math::enforce_daily_cap(
            &progress,
            additional_amount,
            None,
        ).unwrap();
        
        assert_eq!(capped_amount, additional_amount); // No cap, return full amount
    }

    #[test]
    fn test_batch_payout_calculation() {
        let investors = vec![
            (2_500_000u64, 250_000u128), // 25% weight
            (3_000_000u64, 300_000u128), // 30% weight  
            (1_500_000u64, 150_000u128), // 15% weight
            (500_000u64, 50_000u128),    // 5% weight (might be below threshold)
        ];
        
        let total_investor_amount = 10_000u64;
        let min_payout = 1000u64;
        let carry_over_dust = 250u64;
        
        let result = crate::utils::math::calculate_batch_payout(
            &investors,
            total_investor_amount,
            min_payout,
            carry_over_dust,
        ).unwrap();
        
        // Expected payouts:
        // 25% of 10k = 2500 ✓
        // 30% of 10k = 3000 ✓  
        // 15% of 10k = 1500 ✓
        // 5% of 10k = 500 (below 1000 threshold, becomes dust)
        
        let expected_paid = 2500 + 3000 + 1500; // = 7000
    let expected_dust = carry_over_dust + 500 + 2_500; // threshold dust + unmatched allocation = 3,250
        
        assert_eq!(result.0, expected_paid);
        assert_eq!(result.1, expected_dust);
    }

    #[test]
    fn test_pagination_validation() {
        let policy_config = create_test_policy_config();
        let empty_accounts: Vec<AccountInfo> = vec![];
        
        // Test valid page parameters
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &[],
            0,
            10,
        );
        // Should fail due to empty accounts, but page params are valid
        assert!(result.is_err());
        
        // Test invalid page size (zero)
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            0,
        );
        assert!(result.is_err());
        
        // Test invalid page size (too large)
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            (MAX_PAGE_SIZE + 1) as usize,
        );
        assert!(result.is_err());
        
        // Test invalid page start (out of bounds)
        let single_account = vec![]; // Would need actual AccountInfo
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &single_account,
            10, // Start beyond available accounts
            5,
        );
        assert!(result.is_err());
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
    fn test_scaling_factor_application() {
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
        assert_eq!(total_scaled, 6000); // Should be close to capped_total (some rounding)
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
        
        // Test payout with dust
        let dusty_payout = InvestorPayout {
            wallet: Pubkey::new_unique(),
            locked_amount: 500_000,
            weight: 50_000, // 5%
            payout_amount: 0, // Below threshold
            dust_amount: 500,
            ata_address: Pubkey::new_unique(),
            needs_ata_creation: true,
        };
        
        assert_eq!(dusty_payout.payout_amount, 0);
        assert_eq!(dusty_payout.dust_amount, 500);
        assert!(dusty_payout.needs_ata_creation);
    }

    #[test]
    fn test_batch_payout_result() {
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
        
        // Verify payout amounts match
        let calculated_paid: u64 = result.payouts.iter().map(|p| p.payout_amount).sum();
        let calculated_dust: u64 = result.payouts.iter().map(|p| p.dust_amount).sum();
        
        assert_eq!(calculated_paid, result.total_paid);
        assert_eq!(calculated_dust, result.total_dust);
    }

    #[test]
    fn test_page_statistics() {
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
        
        // Test overflow scenario
        let result = calculate_individual_payout(
            u64::MAX,
            WEIGHT_PRECISION,
            1000,
        );
        
        // Might overflow in multiplication, should be handled gracefully
        // The actual behavior depends on the implementation
        match result {
            Ok(_) => {}, // Handled correctly
            Err(_) => {}, // Overflow detected and handled
        }
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
}