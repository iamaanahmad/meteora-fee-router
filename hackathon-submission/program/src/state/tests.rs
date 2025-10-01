use anchor_lang::prelude::*;
use crate::state::*;

#[cfg(test)]
mod policy_config_tests {
    use super::*;

    fn create_test_policy_config() -> PolicyConfig {
        PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 5000, // 50%
            daily_cap_lamports: Some(1_000_000_000), // 1 SOL
            min_payout_lamports: 1000,
            y0_total_allocation: 1_000_000_000_000, // 1M tokens
            bump: 255,
        }
    }

    #[test]
    fn test_policy_config_validation_success() {
        let policy = create_test_policy_config();
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_policy_config_invalid_fee_share() {
        let mut policy = create_test_policy_config();
        policy.investor_fee_share_bps = 10001; // > 100%
        
        let result = policy.validate();
        assert!(result.is_err());
        // Note: In a real test environment, you'd check the specific error
    }

    #[test]
    fn test_policy_config_zero_min_payout() {
        let mut policy = create_test_policy_config();
        policy.min_payout_lamports = 0;
        
        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_zero_total_allocation() {
        let mut policy = create_test_policy_config();
        policy.y0_total_allocation = 0;
        
        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_zero_daily_cap() {
        let mut policy = create_test_policy_config();
        policy.daily_cap_lamports = Some(0);
        
        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_no_daily_cap() {
        let mut policy = create_test_policy_config();
        policy.daily_cap_lamports = None;
        
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_policy_config_initialization() {
        let mut policy = PolicyConfig {
            vault: Pubkey::default(),
            quote_mint: Pubkey::default(),
            creator_wallet: Pubkey::default(),
            investor_fee_share_bps: 0,
            daily_cap_lamports: None,
            min_payout_lamports: 0,
            y0_total_allocation: 0,
            bump: 0,
        };

        let vault = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let creator_wallet = Pubkey::new_unique();

        let result = policy.initialize(
            vault,
            quote_mint,
            creator_wallet,
            7500, // 75%
            Some(2_000_000_000), // 2 SOL
            5000, // 5000 lamports
            500_000_000_000, // 500K tokens
            254,
        );

        assert!(result.is_ok());
        assert_eq!(policy.vault, vault);
        assert_eq!(policy.quote_mint, quote_mint);
        assert_eq!(policy.creator_wallet, creator_wallet);
        assert_eq!(policy.investor_fee_share_bps, 7500);
        assert_eq!(policy.daily_cap_lamports, Some(2_000_000_000));
        assert_eq!(policy.min_payout_lamports, 5000);
        assert_eq!(policy.y0_total_allocation, 500_000_000_000);
        assert_eq!(policy.bump, 254);
    }

    #[test]
    fn test_policy_config_space_calculation() {
        // Verify the INIT_SPACE calculation is correct
        let expected_space = 32 + 32 + 32 + 2 + 9 + 8 + 8 + 1; // 124 bytes
        assert_eq!(PolicyConfig::INIT_SPACE, expected_space);
    }
}

#[cfg(test)]
mod distribution_progress_tests {
    use super::*;
    use crate::constants::TWENTY_FOUR_HOURS;

    fn create_test_distribution_progress() -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1000000,
            current_day_distributed: 500_000_000,
            carry_over_dust: 1500,
            pagination_cursor: 10,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_distribution_progress_initialization() {
        let mut progress = DistributionProgress {
            vault: Pubkey::default(),
            last_distribution_ts: 999,
            current_day_distributed: 999,
            carry_over_dust: 999,
            pagination_cursor: 999,
            day_complete: true,
            bump: 0,
        };

        let vault = Pubkey::new_unique();
        let result = progress.initialize(vault, 254);

        assert!(result.is_ok());
        assert_eq!(progress.vault, vault);
        assert_eq!(progress.last_distribution_ts, 0);
        assert_eq!(progress.current_day_distributed, 0);
        assert_eq!(progress.carry_over_dust, 0);
        assert_eq!(progress.pagination_cursor, 0);
        assert_eq!(progress.day_complete, false);
        assert_eq!(progress.bump, 254);
    }

    #[test]
    fn test_can_start_new_day() {
        let progress = create_test_distribution_progress();
        
        // Should not be able to start new day immediately
        assert!(!progress.can_start_new_day(progress.last_distribution_ts + 1000));
        
        // Should be able to start new day after 24 hours
        assert!(progress.can_start_new_day(progress.last_distribution_ts + TWENTY_FOUR_HOURS));
        
        // Should be able to start new day after more than 24 hours
        assert!(progress.can_start_new_day(progress.last_distribution_ts + TWENTY_FOUR_HOURS + 1000));
    }

    #[test]
    fn test_start_new_day_success() {
        let mut progress = create_test_distribution_progress();
        let new_timestamp = progress.last_distribution_ts + TWENTY_FOUR_HOURS;
        
        let result = progress.start_new_day(new_timestamp);
        assert!(result.is_ok());
        
        assert_eq!(progress.last_distribution_ts, new_timestamp);
        assert_eq!(progress.current_day_distributed, 0);
        assert_eq!(progress.pagination_cursor, 0);
        assert_eq!(progress.day_complete, false);
    }

    #[test]
    fn test_start_new_day_too_early() {
        let mut progress = create_test_distribution_progress();
        let new_timestamp = progress.last_distribution_ts + 1000; // Less than 24 hours
        
        let result = progress.start_new_day(new_timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_daily_cap_no_cap() {
        let progress = create_test_distribution_progress();
        let result = progress.check_daily_cap(1_000_000_000, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_daily_cap_within_limit() {
        let progress = create_test_distribution_progress();
        let daily_cap = Some(1_000_000_000u64);
        let additional = 400_000_000u64; // current: 500M, additional: 400M, total: 900M < 1B
        
        let result = progress.check_daily_cap(additional, daily_cap);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_daily_cap_exceeds_limit() {
        let progress = create_test_distribution_progress();
        let daily_cap = Some(800_000_000u64);
        let additional = 400_000_000u64; // current: 500M, additional: 400M, total: 900M > 800M
        
        let result = progress.check_daily_cap(additional, daily_cap);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_distributed() {
        let mut progress = create_test_distribution_progress();
        let initial = progress.current_day_distributed;
        
        let result = progress.add_distributed(250_000_000);
        assert!(result.is_ok());
        assert_eq!(progress.current_day_distributed, initial + 250_000_000);
    }

    #[test]
    fn test_add_distributed_overflow() {
        let mut progress = create_test_distribution_progress();
        progress.current_day_distributed = u64::MAX;
        
        let result = progress.add_distributed(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_cursor_valid() {
        let mut progress = create_test_distribution_progress();
        let initial_cursor = progress.pagination_cursor;
        
        let result = progress.update_cursor(initial_cursor + 5);
        assert!(result.is_ok());
        assert_eq!(progress.pagination_cursor, initial_cursor + 5);
    }

    #[test]
    fn test_update_cursor_invalid() {
        let mut progress = create_test_distribution_progress();
        let initial_cursor = progress.pagination_cursor;
        
        let result = progress.update_cursor(initial_cursor - 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_dust_operations() {
        let mut progress = create_test_distribution_progress();
        let initial_dust = progress.carry_over_dust;
        
        // Add dust
        let result = progress.add_dust(500);
        assert!(result.is_ok());
        assert_eq!(progress.carry_over_dust, initial_dust + 500);
        
        // Consume partial dust
        let consumed = progress.consume_dust(300).unwrap();
        assert_eq!(consumed, 300);
        assert_eq!(progress.carry_over_dust, initial_dust + 500 - 300);
        
        // Consume more than available
        let consumed = progress.consume_dust(10000).unwrap();
        assert_eq!(consumed, initial_dust + 500 - 300);
        assert_eq!(progress.carry_over_dust, 0);
    }

    #[test]
    fn test_complete_day() {
        let mut progress = create_test_distribution_progress();
        assert!(!progress.day_complete);
        
        progress.complete_day();
        assert!(progress.day_complete);
    }

    #[test]
    fn test_reset_for_continuation() {
        let mut progress = create_test_distribution_progress();
        let original_ts = progress.last_distribution_ts;
        let original_distributed = progress.current_day_distributed;
        
        progress.pagination_cursor = 50;
        progress.day_complete = true;
        
        progress.reset_for_continuation();
        
        // These should remain unchanged
        assert_eq!(progress.last_distribution_ts, original_ts);
        assert_eq!(progress.current_day_distributed, original_distributed);
        
        // These should be reset
        assert_eq!(progress.pagination_cursor, 0);
        assert!(!progress.day_complete);
    }

    #[test]
    fn test_distribution_progress_space_calculation() {
        // Verify the INIT_SPACE calculation is correct
        let expected_space = 32 + 8 + 8 + 8 + 4 + 1 + 1; // 62 bytes
        assert_eq!(DistributionProgress::INIT_SPACE, expected_space);
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use anchor_lang::{AnchorSerialize, AnchorDeserialize};

    #[test]
    fn test_policy_config_serialization() {
        let original = PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 7500,
            daily_cap_lamports: Some(1_000_000_000),
            min_payout_lamports: 5000,
            y0_total_allocation: 500_000_000_000,
            bump: 254,
        };

        // Serialize
        let mut serialized = Vec::new();
        original.serialize(&mut serialized).unwrap();

        // Deserialize
        let deserialized = PolicyConfig::deserialize(&mut serialized.as_slice()).unwrap();

        // Verify all fields match
        assert_eq!(original.vault, deserialized.vault);
        assert_eq!(original.quote_mint, deserialized.quote_mint);
        assert_eq!(original.creator_wallet, deserialized.creator_wallet);
        assert_eq!(original.investor_fee_share_bps, deserialized.investor_fee_share_bps);
        assert_eq!(original.daily_cap_lamports, deserialized.daily_cap_lamports);
        assert_eq!(original.min_payout_lamports, deserialized.min_payout_lamports);
        assert_eq!(original.y0_total_allocation, deserialized.y0_total_allocation);
        assert_eq!(original.bump, deserialized.bump);
    }

    #[test]
    fn test_distribution_progress_serialization() {
        let original = DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1234567890,
            current_day_distributed: 500_000_000,
            carry_over_dust: 1500,
            pagination_cursor: 25,
            day_complete: true,
            bump: 253,
        };

        // Serialize
        let mut serialized = Vec::new();
        original.serialize(&mut serialized).unwrap();

        // Deserialize
        let deserialized = DistributionProgress::deserialize(&mut serialized.as_slice()).unwrap();

        // Verify all fields match
        assert_eq!(original.vault, deserialized.vault);
        assert_eq!(original.last_distribution_ts, deserialized.last_distribution_ts);
        assert_eq!(original.current_day_distributed, deserialized.current_day_distributed);
        assert_eq!(original.carry_over_dust, deserialized.carry_over_dust);
        assert_eq!(original.pagination_cursor, deserialized.pagination_cursor);
        assert_eq!(original.day_complete, deserialized.day_complete);
        assert_eq!(original.bump, deserialized.bump);
    }

    #[test]
    fn test_policy_config_serialization_with_none_daily_cap() {
        let original = PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 2500,
            daily_cap_lamports: None, // Test None case
            min_payout_lamports: 1000,
            y0_total_allocation: 1_000_000_000,
            bump: 255,
        };

        // Serialize
        let mut serialized = Vec::new();
        original.serialize(&mut serialized).unwrap();

        // Deserialize
        let deserialized = PolicyConfig::deserialize(&mut serialized.as_slice()).unwrap();

        // Verify None is preserved
        assert_eq!(original.daily_cap_lamports, deserialized.daily_cap_lamports);
        assert!(deserialized.daily_cap_lamports.is_none());
    }
}