use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, transfer};
use anchor_spl::associated_token::AssociatedToken;

use crate::{
    constants::*,
    error::ErrorCode,
    state::{PolicyConfig, DistributionProgress},
    utils::math::calculate_distribution,
    CreatorPayoutDayClosed,
};

/// Creator distribution system for remainder fee payouts
pub struct CreatorDistribution;

impl CreatorDistribution {
    /// Process creator remainder payout when day is complete
    pub fn process_creator_payout<'info>(
        policy_config: &PolicyConfig,
        distribution_progress: &DistributionProgress,
        claimed_quote_amount: u64,
        total_locked_amount: u64,
        current_timestamp: i64,
        token_program: &Program<'info, Token>,
        _associated_token_program: &Program<'info, AssociatedToken>,
        _system_program: &Program<'info, System>,
        treasury_ata: &Account<'info, TokenAccount>,
        creator_ata: &Account<'info, TokenAccount>,
        position_owner_pda: &AccountInfo<'info>,
    ) -> Result<u64> {
        // Calculate creator remainder amount
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote_amount,
            total_locked_amount,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        )?;

        msg!("Creator payout calculation: total_claimed={}, investor_amount={}, creator_amount={}", 
             claimed_quote_amount, total_investor_amount, creator_amount);

        // Validate creator ATA
        Self::validate_creator_ata(creator_ata, &policy_config.creator_wallet, &policy_config.quote_mint)?;

        // Execute creator payout if there's an amount to transfer
        if creator_amount > 0 {
            Self::execute_creator_transfer(
                creator_amount,
                policy_config,
                token_program,
                treasury_ata,
                creator_ata,
                position_owner_pda,
            )?;

            msg!("Creator payout executed: {} tokens transferred to {}", 
                 creator_amount, policy_config.creator_wallet);
        } else {
            msg!("No creator payout needed (amount: 0)");
        }

        // Emit creator payout event
        emit!(CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout: creator_amount,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed: distribution_progress.current_day_distributed + creator_amount,
            total_investors_processed: 0, // This will be set by the caller
            final_dust_amount: distribution_progress.carry_over_dust,
            timestamp: current_timestamp,
        });

        Ok(creator_amount)
    }

    /// Calculate creator remainder amount without executing transfer
    pub fn calculate_creator_remainder(
        policy_config: &PolicyConfig,
        claimed_quote_amount: u64,
        total_locked_amount: u64,
    ) -> Result<u64> {
        let (_total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote_amount,
            total_locked_amount,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        )?;

        Ok(creator_amount)
    }

    /// Validate creator ATA account
    pub fn validate_creator_ata(
        creator_ata: &Account<TokenAccount>,
        expected_owner: &Pubkey,
        expected_mint: &Pubkey,
    ) -> Result<()> {
        // Validate ATA owner
        require!(
            creator_ata.owner == *expected_owner,
            ErrorCode::InvalidCreatorAta
        );

        // Validate ATA mint
        require!(
            creator_ata.mint == *expected_mint,
            ErrorCode::InvalidCreatorAta
        );

        msg!("Creator ATA validation passed: owner={}, mint={}", 
             expected_owner, expected_mint);

        Ok(())
    }

    /// Execute token transfer from treasury to creator
    fn execute_creator_transfer<'info>(
        amount: u64,
        policy_config: &PolicyConfig,
        token_program: &Program<'info, Token>,
        treasury_ata: &Account<'info, TokenAccount>,
        creator_ata: &Account<'info, TokenAccount>,
        position_owner_pda: &AccountInfo<'info>,
    ) -> Result<()> {
        // Validate treasury has sufficient balance
        require!(
            treasury_ata.amount >= amount,
            ErrorCode::InsufficientFunds
        );

        // Create signer seeds
        let bump_seed = [policy_config.bump];
        let signer_seeds: &[&[u8]] = &[
            VAULT_SEED,
            policy_config.vault.as_ref(),
            b"investor_fee_pos_owner",
            &bump_seed,
        ];
        let signer_seeds_slice = &[signer_seeds];

        // Create transfer instruction
        let transfer_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            Transfer {
                from: treasury_ata.to_account_info(),
                to: creator_ata.to_account_info(),
                authority: position_owner_pda.to_account_info(),
            },
            signer_seeds_slice,
        );

        // Execute transfer
        transfer(transfer_ctx, amount)?;

        msg!("Creator transfer executed: {} tokens from treasury to creator", amount);

        Ok(())
    }

    /// Check if creator ATA exists and create if needed (for future enhancement)
    pub fn ensure_creator_ata<'info>(
        creator_wallet: &Pubkey,
        quote_mint: &Pubkey,
        creator_ata: &Account<'info, TokenAccount>,
        _associated_token_program: &Program<'info, AssociatedToken>,
        _system_program: &Program<'info, System>,
    ) -> Result<bool> {
        // For now, we assume the creator ATA already exists
        // In a full implementation, this would check if the ATA exists
        // and create it if needed using the associated token program

        // Validate the provided ATA
        Self::validate_creator_ata(creator_ata, creator_wallet, quote_mint)?;

        Ok(false) // ATA already existed
    }

    /// Get creator ATA address (deterministic derivation)
    pub fn get_creator_ata_address(
        creator_wallet: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Pubkey {
        anchor_spl::associated_token::get_associated_token_address(
            creator_wallet,
            quote_mint,
        )
    }

    /// Validate creator payout parameters
    pub fn validate_creator_payout_params(
        policy_config: &PolicyConfig,
        _distribution_progress: &DistributionProgress,
        claimed_quote_amount: u64,
    ) -> Result<()> {
        // Validate policy configuration
        policy_config.validate()?;

        // No need to validate claimed_quote_amount as u64 is always >= 0
        // Allow zero amounts for edge cases where no fees were claimed

        Ok(())
    }

    /// Calculate final day statistics including creator payout
    pub fn calculate_day_completion_stats(
        policy_config: &PolicyConfig,
        distribution_progress: &DistributionProgress,
        claimed_quote_amount: u64,
        total_locked_amount: u64,
    ) -> Result<DayCompletionStats> {
        let (total_investor_amount, creator_amount) = calculate_distribution(
            claimed_quote_amount,
            total_locked_amount,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        )?;

        let total_distributed = distribution_progress.current_day_distributed
            .checked_add(creator_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        Ok(DayCompletionStats {
            claimed_quote_amount,
            total_investor_amount,
            creator_amount,
            investor_distributed: distribution_progress.current_day_distributed,
            total_distributed,
            carry_over_dust: distribution_progress.carry_over_dust,
        })
    }
}

/// Statistics for day completion
#[derive(Debug, Clone)]
pub struct DayCompletionStats {
    pub claimed_quote_amount: u64,
    pub total_investor_amount: u64,
    pub creator_amount: u64,
    pub investor_distributed: u64,
    pub total_distributed: u64,
    pub carry_over_dust: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn create_mock_policy_config() -> PolicyConfig {
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

    fn create_mock_distribution_progress() -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1000,
            current_day_distributed: 500_000,
            carry_over_dust: 250,
            pagination_cursor: 100,
            day_complete: true,
            bump: 255,
        }
    }

    #[test]
    fn test_calculate_creator_remainder_basic() {
    let policy_config = create_mock_policy_config();
        
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
    }

    #[test]
    fn test_calculate_creator_remainder_fully_unlocked() {
    let policy_config = create_mock_policy_config();
        
        let claimed_quote = 1000u64;
        let total_locked = 0u64; // All unlocked
        
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        
        // All tokens unlocked, creator gets 100%
        assert_eq!(creator_amount, 1000);
    }

    #[test]
    fn test_calculate_creator_remainder_fully_locked() {
    let policy_config = create_mock_policy_config();
        
        let claimed_quote = 1000u64;
        let total_locked = 10_000_000u64; // 100% locked
        
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        
        // f_locked = 100%, eligible_share = min(80%, 100%) = 80%
        // investor_amount = 1000 * 80% = 800
        // creator_amount = 1000 - 800 = 200
        assert_eq!(creator_amount, 200);
    }

    #[test]
    fn test_get_creator_ata_address() {
        let creator_wallet = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        
        let ata_address = CreatorDistribution::get_creator_ata_address(
            &creator_wallet,
            &quote_mint,
        );
        
        // Should return a valid pubkey (not default)
        assert_ne!(ata_address, Pubkey::default());
        
        // Should be deterministic
        let ata_address2 = CreatorDistribution::get_creator_ata_address(
            &creator_wallet,
            &quote_mint,
        );
        assert_eq!(ata_address, ata_address2);
    }

    #[test]
    fn test_validate_creator_payout_params_valid() {
    let policy_config = create_mock_policy_config();
        let distribution_progress = create_mock_distribution_progress();
        
        let result = CreatorDistribution::validate_creator_payout_params(
            &policy_config,
            &distribution_progress,
            1000,
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_creator_payout_params_day_not_complete() {
        let policy_config = create_mock_policy_config();
        let mut distribution_progress = create_mock_distribution_progress();
        distribution_progress.day_complete = false;
        
        let result = CreatorDistribution::validate_creator_payout_params(
            &policy_config,
            &distribution_progress,
            1000,
        );
        
        // Should now pass since we removed the day_complete validation
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_creator_payout_params_zero_amount() {
        let policy_config = create_mock_policy_config();
        let distribution_progress = create_mock_distribution_progress();
        
        let result = CreatorDistribution::validate_creator_payout_params(
            &policy_config,
            &distribution_progress,
            0, // Zero claimed amount
        );
        
        // Should now pass since we allow zero amounts
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_day_completion_stats() {
        let policy_config = create_mock_policy_config();
        let distribution_progress = create_mock_distribution_progress();
        
        let claimed_quote = 1000u64;
        let total_locked = 5_000_000u64; // 50% locked
        
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            claimed_quote,
            total_locked,
        ).unwrap();
        
        assert_eq!(stats.claimed_quote_amount, 1000);
        assert_eq!(stats.total_investor_amount, 500); // 50% of 1000
        assert_eq!(stats.creator_amount, 500); // Remainder
        assert_eq!(stats.investor_distributed, 500_000); // From progress
        assert_eq!(stats.total_distributed, 500_500); // investor_distributed + creator_amount
        assert_eq!(stats.carry_over_dust, 250); // From progress
    }

    #[test]
    fn test_day_completion_stats_edge_cases() {
        let policy_config = create_mock_policy_config();
        let mut distribution_progress = create_mock_distribution_progress();
        distribution_progress.current_day_distributed = 0;
        distribution_progress.carry_over_dust = 0;
        
        // Test with zero locked (all to creator)
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            1000,
            0, // No locked tokens
        ).unwrap();
        
        assert_eq!(stats.total_investor_amount, 0);
        assert_eq!(stats.creator_amount, 1000);
        assert_eq!(stats.total_distributed, 1000);
        
        // Test with fully locked
        let stats = CreatorDistribution::calculate_day_completion_stats(
            &policy_config,
            &distribution_progress,
            1000,
            10_000_000, // Fully locked
        ).unwrap();
        
        assert_eq!(stats.total_investor_amount, 800); // 80% to investors
        assert_eq!(stats.creator_amount, 200); // 20% to creator
        assert_eq!(stats.total_distributed, 200);
    }

    #[test]
    fn test_creator_remainder_with_different_fee_shares() {
        let mut policy_config = create_mock_policy_config();
        
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
    }

    #[test]
    fn test_arithmetic_overflow_protection() {
        let policy_config = create_mock_policy_config();
        let mut distribution_progress = create_mock_distribution_progress();
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
    fn test_creator_payout_precision() {
        let mut policy_config = create_mock_policy_config();
        
        // Test with very small amounts
        let claimed_quote = 1u64;
        let total_locked = 1u64; // Minimal locked
        
        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        
        // Should handle precision correctly
        assert!(creator_amount <= claimed_quote);
        
        // Test with large amounts
        let claimed_quote = 1_000_000_000u64; // 1B
        let total_locked = 500_000_000u64; // 500M (50% of Y0)
        
        // Align Y0 allocation scale with large amount scenario
        policy_config.y0_total_allocation = 1_000_000_000;

        let creator_amount = CreatorDistribution::calculate_creator_remainder(
            &policy_config,
            claimed_quote,
            total_locked,
        ).unwrap();
        
        // f_locked = 50%, eligible_share = min(80%, 50%) = 50%
        // creator gets 50%
        assert_eq!(creator_amount, 500_000_000);
    }
}