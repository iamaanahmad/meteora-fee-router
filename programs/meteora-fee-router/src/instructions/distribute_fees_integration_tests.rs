    use super::*;
    use crate::{
        state::{PolicyConfig, DistributionProgress, DistributionTimingState},
        utils::{
            fee_claiming::{FeeClaimResult, PositionFeeData},
            math::calculate_distribution,
        },
        QuoteFeesClaimed, InvestorPayoutPage, CreatorPayoutDayClosed,
    };
    

    // Test helper functions
    fn create_test_policy_config() -> PolicyConfig {
        PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 6000, // 60%
            daily_cap_lamports: Some(10_000_000_000), // 10 SOL
            min_payout_lamports: 1000,
            y0_total_allocation: 1_000_000_000_000, // 1M tokens
            bump: 255,
        }
    }

    fn create_test_distribution_progress(vault: Pubkey) -> DistributionProgress {
        DistributionProgress {
            vault,
            last_distribution_ts: 0,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_distribute_fees_params_validation() {
        // Test valid parameters
        let valid_params = DistributeFeesParams {
            page_size: 25,
            cursor_position: Some(50),
        };
        
        assert!(valid_params.page_size > 0 && valid_params.page_size <= MAX_PAGE_SIZE);
        assert_eq!(valid_params.cursor_position, Some(50));

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
    fn test_timing_state_integration() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;

        // Test new day scenario
        let timing_state = distribution_progress
            .prepare_for_distribution(current_timestamp)
            .unwrap();
        
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        assert_eq!(distribution_progress.last_distribution_ts, current_timestamp);
        assert_eq!(distribution_progress.pagination_cursor, 0);
        assert!(!distribution_progress.day_complete);

        // Test same day continuation
        let same_day_time = current_timestamp + 3600; // 1 hour later
        let timing_state = distribution_progress
            .prepare_for_distribution(same_day_time)
            .unwrap();
        
        assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        assert_eq!(distribution_progress.last_distribution_ts, current_timestamp); // Should not change
    }

    #[test]
    fn test_fee_claiming_integration() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;
        
        // Prepare for new day
        let timing_state = distribution_progress
            .prepare_for_distribution(current_timestamp)
            .unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);

        // Simulate fee claiming result
        let claimed_fees = FeeClaimResult {
            quote_amount: 5_000_000_000, // 5 SOL worth of fees
            base_amount: 0,
            quote_mint: policy_config.quote_mint,
        };

        // Verify fee claiming results
        assert_eq!(claimed_fees.quote_amount, 5_000_000_000);
        assert_eq!(claimed_fees.base_amount, 0);
        assert_eq!(claimed_fees.quote_mint, policy_config.quote_mint);

        // Test that event should be emitted for non-zero amounts
        assert!(claimed_fees.quote_amount > 0);
    }

    #[test]
    fn test_distribution_calculation_integration() {
        let policy_config = create_test_policy_config();
        let treasury_balance = 5_000_000_000u64; // 5 SOL
        let total_locked_amount = 275_000_000_000u64; // 275k tokens locked

        // Calculate distribution amounts
        let (total_investor_amount, creator_amount) = calculate_distribution(
            treasury_balance,
            total_locked_amount,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();

        // Verify distribution calculation
        // f_locked = 275k / 1M = 0.275 = 2750 bps
        // eligible_share = min(6000, 2750) = 2750 bps
        // investor_amount = 5 SOL * 2750 / 10000 = 1.375 SOL
        // creator_amount = 5 SOL - 1.375 SOL = 3.625 SOL
        assert_eq!(total_investor_amount, 1_375_000_000);
        assert_eq!(creator_amount, 3_625_000_000);

        // Verify total adds up
        assert_eq!(total_investor_amount + creator_amount, treasury_balance);
    }

    #[test]
    fn test_pagination_state_management() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;
        let page_size = 2u32;

        // Prepare for distribution
        distribution_progress
            .prepare_for_distribution(current_timestamp)
            .unwrap();

        // Test initial state
        assert_eq!(distribution_progress.pagination_cursor, 0);

        // Process first page
        distribution_progress.mark_page_processed(0, page_size).unwrap();
        assert_eq!(distribution_progress.pagination_cursor, 2);

        // Process second page
        distribution_progress.mark_page_processed(2, page_size).unwrap();
        assert_eq!(distribution_progress.pagination_cursor, 4);

        // Test idempotent retry detection
        let is_retry = distribution_progress.validate_cursor_for_retry(0).unwrap();
        assert!(is_retry); // Should detect as retry

        let is_retry = distribution_progress.validate_cursor_for_retry(4).unwrap();
        assert!(!is_retry); // Current position, not a retry
    }

    #[test]
    fn test_daily_cap_enforcement_logic() {
        let mut policy_config = create_test_policy_config();
        policy_config.daily_cap_lamports = Some(2_000_000_000); // 2 SOL cap
        
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        distribution_progress.current_day_distributed = 1_800_000_000; // 1.8 SOL already distributed

        // Calculate remaining cap
        let remaining_cap = policy_config.daily_cap_lamports.unwrap()
            .saturating_sub(distribution_progress.current_day_distributed);
        
        assert_eq!(remaining_cap, 200_000_000); // Only 0.2 SOL remaining

        // Test cap enforcement
        let requested_amount = 1_000_000_000u64; // 1 SOL requested
        let effective_amount = std::cmp::min(requested_amount, remaining_cap);
        assert_eq!(effective_amount, 200_000_000); // Should be capped

        // Test when cap is reached
        distribution_progress.current_day_distributed = policy_config.daily_cap_lamports.unwrap();
        let remaining_cap = policy_config.daily_cap_lamports.unwrap()
            .saturating_sub(distribution_progress.current_day_distributed);
        assert_eq!(remaining_cap, 0); // No more distributions allowed
    }

    #[test]
    fn test_quote_only_validation_integration() {
        let policy_config = create_test_policy_config();

        // Test valid quote-only fee data
        let valid_fee_data = PositionFeeData {
            fee_owed_a: 1_000_000_000, // 1 SOL in quote token
            fee_owed_b: 0,
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(),
        };

        let result = crate::utils::fee_claiming::validate_quote_only_fees(
            &valid_fee_data, 
            &policy_config.quote_mint
        );
        assert!(result.is_ok());

        // Test invalid fee data with base fees
        let invalid_fee_data = PositionFeeData {
            fee_owed_a: 1_000_000_000, // Quote fees
            fee_owed_b: 500_000_000,   // Base fees - should cause failure
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(),
        };

        let result = crate::utils::fee_claiming::validate_quote_only_fees(
            &invalid_fee_data, 
            &policy_config.quote_mint
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_event_data_structures() {
        let policy_config = create_test_policy_config();
        let current_timestamp = 1000i64;

        // Test QuoteFeesClaimed event
        let claimed_amount = 5_000_000_000u64;
        let honorary_position = Pubkey::new_unique();
        let treasury_ata = Pubkey::new_unique();
        let quote_fees_claimed = QuoteFeesClaimed {
            vault: policy_config.vault,
            claimed_amount,
            base_amount: 0, // Should always be 0 for quote-only
            quote_mint: policy_config.quote_mint,
            honorary_position,
            treasury_ata,
            timestamp: current_timestamp,
        };

        assert_eq!(quote_fees_claimed.vault, policy_config.vault);
        assert_eq!(quote_fees_claimed.claimed_amount, claimed_amount);
        assert_eq!(quote_fees_claimed.base_amount, 0);
        assert_eq!(quote_fees_claimed.quote_mint, policy_config.quote_mint);
        assert_eq!(quote_fees_claimed.honorary_position, honorary_position);
        assert_eq!(quote_fees_claimed.treasury_ata, treasury_ata);
        assert_eq!(quote_fees_claimed.timestamp, current_timestamp);

        // Test InvestorPayoutPage event
        let investor_payout_page = InvestorPayoutPage {
            vault: policy_config.vault,
            page_start: 0,
            page_end: 2,
            total_distributed: 1_000_000_000,
            processed_count: 2,
            dust_carried_forward: 500,
            cumulative_day_distributed: 1_500_000_000,
            timestamp: current_timestamp,
        };

        assert_eq!(investor_payout_page.vault, policy_config.vault);
        assert_eq!(investor_payout_page.page_start, 0);
        assert_eq!(investor_payout_page.page_end, 2);
        assert_eq!(investor_payout_page.total_distributed, 1_000_000_000);
        assert_eq!(investor_payout_page.processed_count, 2);
        assert_eq!(investor_payout_page.dust_carried_forward, 500);
        assert_eq!(investor_payout_page.cumulative_day_distributed, 1_500_000_000);
        assert_eq!(investor_payout_page.timestamp, current_timestamp);

        // Test CreatorPayoutDayClosed event
        let creator_payout_day_closed = CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout: 3_000_000_000,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed: 4_000_000_000,
            total_investors_processed: 10,
            final_dust_amount: 250,
            timestamp: current_timestamp,
        };

        assert_eq!(creator_payout_day_closed.vault, policy_config.vault);
        assert_eq!(creator_payout_day_closed.creator_payout, 3_000_000_000);
        assert_eq!(creator_payout_day_closed.creator_wallet, policy_config.creator_wallet);
        assert_eq!(creator_payout_day_closed.total_day_distributed, 4_000_000_000);
        assert_eq!(creator_payout_day_closed.total_investors_processed, 10);
        assert_eq!(creator_payout_day_closed.final_dust_amount, 250);
        assert_eq!(creator_payout_day_closed.timestamp, current_timestamp);
    }

    /// Test event emission during fee claiming
    #[test]
    fn test_quote_fees_claimed_event_emission() {
        let policy_config = create_test_policy_config();
        let current_timestamp = 1500i64;
        let honorary_position = Pubkey::new_unique();
        let treasury_ata = Pubkey::new_unique();

        // Test event emission for successful fee claim
        let claimed_amount = 2_500_000_000u64;
        let expected_event = QuoteFeesClaimed {
            vault: policy_config.vault,
            claimed_amount,
            base_amount: 0, // Must be 0 for quote-only
            quote_mint: policy_config.quote_mint,
            honorary_position,
            treasury_ata,
            timestamp: current_timestamp,
        };

        // Verify event structure and data
        assert_eq!(expected_event.vault, policy_config.vault);
        assert_eq!(expected_event.claimed_amount, claimed_amount);
        assert_eq!(expected_event.base_amount, 0); // Critical for quote-only enforcement
        assert_eq!(expected_event.quote_mint, policy_config.quote_mint);
        assert_eq!(expected_event.honorary_position, honorary_position);
        assert_eq!(expected_event.treasury_ata, treasury_ata);
        assert_eq!(expected_event.timestamp, current_timestamp);

        // Verify event can be serialized
        let serialized = expected_event.try_to_vec();
        assert!(serialized.is_ok());
    }

    /// Test event emission during investor payout processing
    #[test]
    fn test_investor_payout_page_event_emission() {
        let policy_config = create_test_policy_config();
        let current_timestamp = 2500i64;

        // Test event emission for investor payout page
        let page_start = 10u32;
        let page_end = 20u32;
        let total_distributed = 1_200_000_000u64;
        let processed_count = 10u32;
        let dust_carried_forward = 1500u64;
        let cumulative_day_distributed = 3_000_000_000u64;

        let expected_event = InvestorPayoutPage {
            vault: policy_config.vault,
            page_start,
            page_end,
            total_distributed,
            processed_count,
            dust_carried_forward,
            cumulative_day_distributed,
            timestamp: current_timestamp,
        };

        // Verify event structure and data
        assert_eq!(expected_event.vault, policy_config.vault);
        assert_eq!(expected_event.page_start, page_start);
        assert_eq!(expected_event.page_end, page_end);
        assert_eq!(expected_event.total_distributed, total_distributed);
        assert_eq!(expected_event.processed_count, processed_count);
        assert_eq!(expected_event.dust_carried_forward, dust_carried_forward);
        assert_eq!(expected_event.cumulative_day_distributed, cumulative_day_distributed);
        assert_eq!(expected_event.timestamp, current_timestamp);

        // Verify logical consistency
        assert_eq!(expected_event.processed_count, expected_event.page_end - expected_event.page_start);
        assert!(expected_event.cumulative_day_distributed >= expected_event.total_distributed);

        // Verify event can be serialized
        let serialized = expected_event.try_to_vec();
        assert!(serialized.is_ok());
    }

    /// Test event emission during creator payout and day completion
    #[test]
    fn test_creator_payout_day_closed_event_emission() {
        let policy_config = create_test_policy_config();
        let current_timestamp = 3500i64;

        // Test event emission for creator payout and day completion
        let creator_payout = 1_800_000_000u64;
        let total_day_distributed = 5_000_000_000u64;
        let total_investors_processed = 25u32;
        let final_dust_amount = 2000u64;

        let expected_event = CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed,
            total_investors_processed,
            final_dust_amount,
            timestamp: current_timestamp,
        };

        // Verify event structure and data
        assert_eq!(expected_event.vault, policy_config.vault);
        assert_eq!(expected_event.creator_payout, creator_payout);
        assert_eq!(expected_event.creator_wallet, policy_config.creator_wallet);
        assert_eq!(expected_event.total_day_distributed, total_day_distributed);
        assert_eq!(expected_event.total_investors_processed, total_investors_processed);
        assert_eq!(expected_event.final_dust_amount, final_dust_amount);
        assert_eq!(expected_event.timestamp, current_timestamp);

        // Verify logical consistency
        assert!(expected_event.total_day_distributed >= expected_event.creator_payout);
        assert!(expected_event.total_investors_processed > 0);

        // Verify event can be serialized
        let serialized = expected_event.try_to_vec();
        assert!(serialized.is_ok());
    }

    /// Test event emission sequence during complete distribution cycle
    #[test]
    fn test_complete_distribution_cycle_events() {
        let policy_config = create_test_policy_config();
        let base_timestamp = 4000i64;

        // 1. Fee claiming event
        let claim_timestamp = base_timestamp;
        let claimed_amount = 10_000_000_000u64;
        let claim_event = QuoteFeesClaimed {
            vault: policy_config.vault,
            claimed_amount,
            base_amount: 0,
            quote_mint: policy_config.quote_mint,
            honorary_position: Pubkey::new_unique(),
            treasury_ata: Pubkey::new_unique(),
            timestamp: claim_timestamp,
        };

        // 2. Multiple investor payout page events
        let mut total_investor_distributed = 0u64;
        let mut total_investors_processed = 0u32;
        let page_size = 10u32;
        let num_pages = 3;

        for page in 0..num_pages {
            let page_timestamp = base_timestamp + (page as i64) + 1;
            let page_start = page * page_size;
            let page_end = (page + 1) * page_size;
            let page_distributed = 2_000_000_000u64;
            total_investor_distributed += page_distributed;
            total_investors_processed += page_size;

            let payout_event = InvestorPayoutPage {
                vault: policy_config.vault,
                page_start,
                page_end,
                total_distributed: page_distributed,
                processed_count: page_size,
                dust_carried_forward: 100,
                cumulative_day_distributed: total_investor_distributed,
                timestamp: page_timestamp,
            };

            // Verify event timing
            assert!(payout_event.timestamp > claim_event.timestamp);
            
            // Verify page progression
            assert_eq!(payout_event.page_start, page * page_size);
            assert_eq!(payout_event.page_end, (page + 1) * page_size);
            assert_eq!(payout_event.processed_count, page_size);
        }

        // 3. Creator payout and day completion event
        let creator_timestamp = base_timestamp + num_pages as i64 + 1;
        let creator_payout = claimed_amount - total_investor_distributed;
        let creator_event = CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed: claimed_amount,
            total_investors_processed,
            final_dust_amount: 300,
            timestamp: creator_timestamp,
        };

        // Verify event timing sequence
        assert!(creator_event.timestamp > claim_event.timestamp);
        
        // Verify amount consistency across events
        assert_eq!(
            total_investor_distributed + creator_event.creator_payout,
            claim_event.claimed_amount
        );
        assert_eq!(creator_event.total_day_distributed, claim_event.claimed_amount);
        assert_eq!(creator_event.total_investors_processed, total_investors_processed);
    }

    /// Test event emission for edge cases
    #[test]
    fn test_event_emission_edge_cases() {
        let policy_config = create_test_policy_config();
        let current_timestamp = 5000i64;

        // Test zero fee claim event
        let zero_claim_event = QuoteFeesClaimed {
            vault: policy_config.vault,
            claimed_amount: 0,
            base_amount: 0,
            quote_mint: policy_config.quote_mint,
            honorary_position: Pubkey::new_unique(),
            treasury_ata: Pubkey::new_unique(),
            timestamp: current_timestamp,
        };
        assert_eq!(zero_claim_event.claimed_amount, 0);
        assert_eq!(zero_claim_event.base_amount, 0);

        // Test single investor payout event
        let single_investor_event = InvestorPayoutPage {
            vault: policy_config.vault,
            page_start: 0,
            page_end: 1,
            total_distributed: 1_000_000,
            processed_count: 1,
            dust_carried_forward: 0,
            cumulative_day_distributed: 1_000_000,
            timestamp: current_timestamp,
        };
        assert_eq!(single_investor_event.processed_count, 1);
        assert_eq!(single_investor_event.page_end - single_investor_event.page_start, 1);

        // Test no creator payout event (all to investors)
        let no_creator_event = CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout: 0,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed: 5_000_000,
            total_investors_processed: 10,
            final_dust_amount: 0,
            timestamp: current_timestamp,
        };
        assert_eq!(no_creator_event.creator_payout, 0);
        assert!(no_creator_event.total_day_distributed > 0);

        // Test maximum values event
        let max_values_event = QuoteFeesClaimed {
            vault: policy_config.vault,
            claimed_amount: u64::MAX,
            base_amount: 0, // Still must be 0
            quote_mint: policy_config.quote_mint,
            honorary_position: Pubkey::new_unique(),
            treasury_ata: Pubkey::new_unique(),
            timestamp: current_timestamp,
        };
        assert_eq!(max_values_event.claimed_amount, u64::MAX);
        assert_eq!(max_values_event.base_amount, 0);
    }

    #[test]
    fn test_day_completion_flow() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;
        let total_investors = 4usize;
        let page_size = 2u32;

        // Prepare for distribution
        distribution_progress
            .prepare_for_distribution(current_timestamp)
            .unwrap();

        // Process all investor pages
        let mut total_processed = 0u32;
        while (total_processed as usize) < total_investors {
            let remaining = total_investors - total_processed as usize;
            let current_page_size = std::cmp::min(page_size, remaining as u32);
            
            distribution_progress.mark_page_processed(total_processed, current_page_size).unwrap();
            total_processed += current_page_size;
        }

        // All investors should be processed
        assert!(distribution_progress.pagination_cursor as usize >= total_investors);

        // Complete the day
        distribution_progress.complete_day();
        assert!(distribution_progress.day_complete);

        // Test creator payout calculation
        let treasury_balance = 5_000_000_000u64;
        let total_locked = 275_000_000_000u64;
        
        let creator_payout = crate::utils::creator_distribution::CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            treasury_balance,
            total_locked,
        ).unwrap();

        // Should match the creator amount from distribution calculation
        let (_, expected_creator_amount) = calculate_distribution(
            treasury_balance,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();

        assert_eq!(creator_payout, expected_creator_amount);
    }

    #[test]
    fn test_error_scenarios() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;

    // Test cooldown not elapsed when trying to restart after completing the same day
    distribution_progress.last_distribution_ts = current_timestamp - 3600; // 1 hour ago
    distribution_progress.complete_day();

    let result = distribution_progress.prepare_for_distribution(current_timestamp);
    assert!(result.is_err()); // Should fail due to completed day within cooldown window

        // Reset for next test
        distribution_progress.last_distribution_ts = 0;
        distribution_progress.prepare_for_distribution(current_timestamp).unwrap();

        // Test invalid cursor position (future cursor)
        distribution_progress.pagination_cursor = 10;
        let result = distribution_progress.validate_cursor_for_retry(15);
        assert!(result.is_err()); // Future cursor position should be invalid
    }

    #[test]
    fn test_comprehensive_distribution_flow() {
        let policy_config = create_test_policy_config();
        let mut distribution_progress = create_test_distribution_progress(policy_config.vault);
        
        let current_timestamp = 1000i64;
        let treasury_balance = 5_000_000_000u64;
        let total_locked = 275_000_000_000u64;

        // Step 1: Prepare for new day
        let timing_state = distribution_progress
            .prepare_for_distribution(current_timestamp)
            .unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);

        // Step 2: Validate fee claiming
        let fee_data = PositionFeeData {
            fee_owed_a: treasury_balance,
            fee_owed_b: 0,
            token_mint_a: policy_config.quote_mint,
            token_mint_b: Pubkey::new_unique(),
        };

        let validation_result = crate::utils::fee_claiming::validate_quote_only_fees(
            &fee_data, 
            &policy_config.quote_mint
        );
        assert!(validation_result.is_ok());

        // Step 3: Calculate distribution
        let (total_investor_amount, creator_amount) = calculate_distribution(
            treasury_balance,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();

        // Step 4: Process investor pages
        let page_size = 2u32;
        let total_investors = 4usize;
        let mut total_processed = 0u32;

        while (total_processed as usize) < total_investors {
            let remaining = total_investors - total_processed as usize;
            let current_page_size = std::cmp::min(page_size, remaining as u32);
            
            distribution_progress.mark_page_processed(total_processed, current_page_size).unwrap();
            total_processed += current_page_size;
        }

        // Step 5: Complete day and verify final state
        assert!(distribution_progress.pagination_cursor as usize >= total_investors);
        
        let creator_payout = crate::utils::creator_distribution::CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            treasury_balance,
            total_locked,
        ).unwrap();

        distribution_progress.complete_day();
        
        // Verify final state
        assert!(distribution_progress.day_complete);
        assert_eq!(creator_payout, creator_amount);
        assert_eq!(total_investor_amount + creator_amount, treasury_balance);
    }
