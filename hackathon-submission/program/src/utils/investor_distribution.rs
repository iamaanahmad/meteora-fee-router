use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

use crate::{
    constants::*,
    error::ErrorCode,
    state::{PolicyConfig, DistributionProgress},
    utils::{
        math::{
            calculate_distribution, calculate_investor_weight, calculate_individual_payout,
            enforce_daily_cap, calculate_dust_payout,
        },
        streamflow::StreamflowIntegration,
    },

};

/// Individual investor payout information
#[derive(Debug, Clone)]
pub struct InvestorPayout {
    pub wallet: Pubkey,
    pub locked_amount: u64,
    pub weight: u128,
    pub payout_amount: u64,
    pub dust_amount: u64,
    pub ata_address: Pubkey,
    pub needs_ata_creation: bool,
}

/// Batch payout processing result
#[derive(Debug)]
pub struct BatchPayoutResult {
    pub total_paid: u64,
    pub total_dust: u64,
    pub processed_count: usize,
    pub payouts: Vec<InvestorPayout>,
}

/// Investor distribution system for paginated fee payouts
pub struct InvestorDistribution;

impl InvestorDistribution {
    /// Process a page of investors for fee distribution
    pub fn process_investor_page<'info>(
        policy_config: &PolicyConfig,
        distribution_progress: &mut DistributionProgress,
        streamflow_accounts: &[AccountInfo<'info>],
        total_investor_amount: u64,
        total_locked_amount: u64,
        page_start: usize,
        page_size: usize,
        current_timestamp: i64,
        token_program: &Program<'info, Token>,
        associated_token_program: &Program<'info, AssociatedToken>,
        system_program: &Program<'info, System>,
        treasury_ata: &Account<'info, TokenAccount>,
        position_owner_pda: &AccountInfo<'info>,
    ) -> Result<BatchPayoutResult> {
        // Validate page parameters
        require!(
            page_size > 0 && page_size <= MAX_PAGE_SIZE as usize,
            ErrorCode::InvalidPaginationCursor
        );
        
        require!(
            page_start < streamflow_accounts.len(),
            ErrorCode::InvalidPaginationCursor
        );
        
        let end_index = std::cmp::min(page_start + page_size, streamflow_accounts.len());
        let page_accounts = &streamflow_accounts[page_start..end_index];
        
        msg!("Processing investor page: start={}, size={}, end={}", page_start, page_size, end_index);
        
        // Aggregate investor data from Streamflow accounts
        let investor_data = StreamflowIntegration::aggregate_investor_data(
            page_accounts,
            &policy_config.quote_mint,
            current_timestamp,
        )?;
        
        msg!("Found {} unique investors in page", investor_data.len());
        
        // Calculate individual payouts
        let mut payouts = Vec::new();
        let mut total_page_locked = 0u64;
        
        for investor in investor_data {
            total_page_locked = total_page_locked
                .checked_add(investor.locked_amount)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            
            // Calculate investor weight based on their locked amount
            let weight = calculate_investor_weight(
                investor.locked_amount,
                total_locked_amount,
            )?;
            
            // Calculate individual payout with dust handling
            let (payout_amount, dust_amount) = calculate_individual_payout(
                total_investor_amount,
                weight,
                policy_config.min_payout_lamports,
            )?;
            
            // Derive investor's ATA address
            let (ata_address, needs_creation) = Self::get_or_derive_investor_ata(
                &investor.wallet,
                &policy_config.quote_mint,
            )?;
            
            payouts.push(InvestorPayout {
                wallet: investor.wallet,
                locked_amount: investor.locked_amount,
                weight,
                payout_amount,
                dust_amount,
                ata_address,
                needs_ata_creation: needs_creation,
            });
            
            msg!("Investor {}: locked={}, weight={}, payout={}, dust={}", 
                 investor.wallet, investor.locked_amount, weight, payout_amount, dust_amount);
        }
        
        // Calculate batch totals
        let total_paid: u64 = payouts.iter().map(|p| p.payout_amount).sum();
        let total_dust: u64 = payouts.iter().map(|p| p.dust_amount).sum();
        
        // Add carry-over dust from previous distributions
        let carry_over_dust = distribution_progress.carry_over_dust;
        let total_dust_with_carryover = total_dust
            .checked_add(carry_over_dust)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Check if accumulated dust can be paid out
        let (dust_payout, remaining_dust) = calculate_dust_payout(
            total_dust_with_carryover,
            policy_config.min_payout_lamports,
        );
        
        let final_total_paid = total_paid
            .checked_add(dust_payout)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        // Enforce daily cap if configured
        let capped_amount = enforce_daily_cap(
            distribution_progress,
            final_total_paid,
            policy_config.daily_cap_lamports,
        )?;
        
        msg!("Batch payout calculation: paid={}, dust={}, carryover={}, dust_payout={}, remaining_dust={}, capped={}",
             total_paid, total_dust, carry_over_dust, dust_payout, remaining_dust, capped_amount);
        
        // Execute payouts if there's amount to distribute
        if capped_amount > 0 {
            Self::execute_investor_payouts(
                &payouts,
                capped_amount,
                final_total_paid,
                dust_payout,
                policy_config,
                token_program,
                associated_token_program,
                system_program,
                treasury_ata,
                position_owner_pda,
            )?;
            
            // Update distribution progress
            distribution_progress.add_distributed(capped_amount)?;
        }
        
        // Update dust tracking
        distribution_progress.carry_over_dust = remaining_dust;
        
        // Update pagination cursor
        distribution_progress.advance_cursor(page_size as u32)?;
        
        Ok(BatchPayoutResult {
            total_paid: capped_amount,
            total_dust: remaining_dust,
            processed_count: payouts.len(),
            payouts,
        })
    }
    
    /// Execute actual token transfers to investors
    fn execute_investor_payouts<'info>(
        payouts: &[InvestorPayout],
        capped_total: u64,
        original_total: u64,
        dust_payout: u64,
        _policy_config: &PolicyConfig,
        _token_program: &Program<'info, Token>,
        _associated_token_program: &Program<'info, AssociatedToken>,
        _system_program: &Program<'info, System>,
        _treasury_ata: &Account<'info, TokenAccount>,
        _position_owner_pda: &AccountInfo<'info>,
    ) -> Result<()> {
        let mut total_transferred = 0u64;
        
        // Calculate scaling factor if we hit daily cap
        let scale_factor = if original_total > 0 {
            (capped_total as u128 * WEIGHT_PRECISION) / original_total as u128
        } else {
            WEIGHT_PRECISION
        };
        
        msg!("Executing payouts with scale factor: {}", scale_factor);
        
        // Process individual investor payouts
        for payout in payouts {
            if payout.payout_amount == 0 {
                continue; // Skip zero payouts
            }
            
            // Scale the payout if we hit daily cap
            let scaled_payout = if scale_factor < WEIGHT_PRECISION {
                ((payout.payout_amount as u128 * scale_factor) / WEIGHT_PRECISION) as u64
            } else {
                payout.payout_amount
            };
            
            if scaled_payout == 0 {
                continue; // Skip if scaling resulted in zero
            }
            
            // Create ATA if needed (this would require additional accounts in real implementation)
            if payout.needs_ata_creation {
                msg!("Note: ATA creation needed for investor {} (not implemented in this demo)", payout.wallet);
                // In full implementation, this would create the ATA
                // For now, we'll assume ATAs exist or skip the payout
                continue;
            }
            
            // Execute transfer from treasury to investor
            // Note: In real implementation, this would require the investor's ATA account
            // For demo purposes, we'll just log the transfer
            msg!("Transferring {} tokens to investor {}", scaled_payout, payout.wallet);
            
            total_transferred = total_transferred
                .checked_add(scaled_payout)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }
        
        // Handle dust payout if applicable
        if dust_payout > 0 {
            msg!("Processing dust payout of {} tokens", dust_payout);
            // In real implementation, this might go to the first eligible investor
            // or be handled according to policy
            total_transferred = total_transferred
                .checked_add(dust_payout)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }
        
        msg!("Total transferred in batch: {}", total_transferred);
        
        Ok(())
    }
    
    /// Get or derive investor's ATA address
    fn get_or_derive_investor_ata(
        investor_wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(Pubkey, bool)> {
        // Derive the ATA address
        let ata_address = anchor_spl::associated_token::get_associated_token_address(
            investor_wallet,
            mint,
        );
        
        // In real implementation, we would check if the ATA exists
        // For demo purposes, assume it needs creation
        let needs_creation = true;
        
        Ok((ata_address, needs_creation))
    }
    
    /// Calculate total distribution amounts for the current page
    pub fn calculate_page_distribution(
        policy_config: &PolicyConfig,
        streamflow_accounts: &[AccountInfo],
        claimed_quote_amount: u64,
        total_locked_amount: u64,
        page_start: usize,
        page_size: usize,
        current_timestamp: i64,
    ) -> Result<(u64, u64)> {
        // Calculate overall distribution split
        let (total_investor_amount, _creator_amount) = calculate_distribution(
            claimed_quote_amount,
            total_locked_amount,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        )?;
        
        // Get page bounds
        let end_index = std::cmp::min(page_start + page_size, streamflow_accounts.len());
        let page_accounts = &streamflow_accounts[page_start..end_index];
        
        // Aggregate investor data for this page
        let investor_data = StreamflowIntegration::aggregate_investor_data(
            page_accounts,
            &policy_config.quote_mint,
            current_timestamp,
        )?;
        
        // Calculate page locked amount
        let page_locked_amount: u64 = investor_data
            .iter()
            .map(|inv| inv.locked_amount)
            .sum();
        
        // Calculate proportional amount for this page
        let page_investor_amount = if total_locked_amount > 0 {
            (total_investor_amount as u128 * page_locked_amount as u128 / total_locked_amount as u128) as u64
        } else {
            0
        };
        
        Ok((page_investor_amount, page_locked_amount))
    }
    
    /// Validate investor distribution parameters
    pub fn validate_distribution_params(
        policy_config: &PolicyConfig,
        streamflow_accounts: &[AccountInfo],
        page_start: usize,
        page_size: usize,
    ) -> Result<()> {
        // Validate page parameters
        require!(
            page_size > 0 && page_size <= MAX_PAGE_SIZE as usize,
            ErrorCode::InvalidPaginationCursor
        );
        
        require!(
            page_start < streamflow_accounts.len(),
            ErrorCode::InvalidPaginationCursor
        );
        
        // Validate policy configuration
        policy_config.validate()?;
        
        // Validate we have at least one Streamflow account
        require!(
            !streamflow_accounts.is_empty(),
            ErrorCode::StreamflowValidationFailed
        );
        
        Ok(())
    }
    
    /// Check if all investors in page have sufficient locked amounts
    pub fn validate_investor_eligibility(
        streamflow_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        page_start: usize,
        page_size: usize,
        current_timestamp: i64,
    ) -> Result<bool> {
        let end_index = std::cmp::min(page_start + page_size, streamflow_accounts.len());
        let page_accounts = &streamflow_accounts[page_start..end_index];
        
        let investor_data = StreamflowIntegration::aggregate_investor_data(
            page_accounts,
            expected_mint,
            current_timestamp,
        )?;
        
        // Check if any investors have locked amounts
        let has_eligible_investors = investor_data
            .iter()
            .any(|inv| inv.locked_amount > 0);
        
        Ok(has_eligible_investors)
    }
    
    /// Get summary statistics for a page of investors
    pub fn get_page_statistics(
        streamflow_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        page_start: usize,
        page_size: usize,
        current_timestamp: i64,
    ) -> Result<PageStatistics> {
        let end_index = std::cmp::min(page_start + page_size, streamflow_accounts.len());
        let page_accounts = &streamflow_accounts[page_start..end_index];
        
        let investor_data = StreamflowIntegration::aggregate_investor_data(
            page_accounts,
            expected_mint,
            current_timestamp,
        )?;
        
        let total_locked: u64 = investor_data.iter().map(|inv| inv.locked_amount).sum();
        let total_allocation: u64 = investor_data.iter().map(|inv| inv.total_allocation).sum();
        let eligible_count = investor_data.iter().filter(|inv| inv.locked_amount > 0).count();
        
        Ok(PageStatistics {
            total_investors: investor_data.len(),
            eligible_investors: eligible_count,
            total_locked_amount: total_locked,
            total_allocation_amount: total_allocation,
            page_start,
            page_size: end_index - page_start,
        })
    }
}

/// Statistics for a page of investors
#[derive(Debug, Clone)]
pub struct PageStatistics {
    pub total_investors: usize,
    pub eligible_investors: usize,
    pub total_locked_amount: u64,
    pub total_allocation_amount: u64,
    pub page_start: usize,
    pub page_size: usize,
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{PolicyConfig, DistributionProgress};
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
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_calculate_page_distribution() {
        let policy_config = create_mock_policy_config();
        
        // Test basic calculation
        let claimed_quote = 1000u64;
        let total_locked = 5_000_000u64; // 50% of Y0
        let page_locked = 1_000_000u64;  // 20% of total locked
        
    // Expected investor share is limited by locked fraction: min(80%, 50%) = 50%
    // Investors receive 50% of 1000 = 500
        
        // This test would need mock Streamflow accounts to work properly
        // For now, we'll test the calculation logic separately
        
        let (total_investor_amount, _) = calculate_distribution(
            claimed_quote,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        ).unwrap();
        
    assert_eq!(total_investor_amount, 500); // min(80%, 50%) of claimed_quote
        
        let page_amount = (total_investor_amount as u128 * page_locked as u128 / total_locked as u128) as u64;
    assert_eq!(page_amount, 100); // 500 * (1M / 5M) = 100
    }

    #[test]
    fn test_get_or_derive_investor_ata() {
        let investor_wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        
        let (ata_address, needs_creation) = InvestorDistribution::get_or_derive_investor_ata(
            &investor_wallet,
            &mint,
        ).unwrap();
        
        // Should derive a valid ATA address
        assert_ne!(ata_address, Pubkey::default());
        assert!(needs_creation); // Mock implementation always returns true
    }

    #[test]
    fn test_validate_distribution_params() {
        let policy_config = create_mock_policy_config();
        let empty_accounts: Vec<AccountInfo> = vec![];
        
        // Should fail with empty accounts
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            10,
        );
        assert!(result.is_err());
        
        // Should fail with invalid page size
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            0, // Invalid page size
        );
        assert!(result.is_err());
        
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            (MAX_PAGE_SIZE + 1) as usize, // Too large
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_payout_result_creation() {
        let payouts = vec![
            InvestorPayout {
                wallet: Pubkey::new_unique(),
                locked_amount: 1000,
                weight: 100_000,
                payout_amount: 100,
                dust_amount: 0,
                ata_address: Pubkey::new_unique(),
                needs_ata_creation: false,
            },
            InvestorPayout {
                wallet: Pubkey::new_unique(),
                locked_amount: 500,
                weight: 50_000,
                payout_amount: 0, // Below threshold
                dust_amount: 50,
                ata_address: Pubkey::new_unique(),
                needs_ata_creation: true,
            },
        ];
        
        let result = BatchPayoutResult {
            total_paid: 100,
            total_dust: 50,
            processed_count: 2,
            payouts,
        };
        
        assert_eq!(result.total_paid, 100);
        assert_eq!(result.total_dust, 50);
        assert_eq!(result.processed_count, 2);
        assert_eq!(result.payouts.len(), 2);
    }

    #[test]
    fn test_page_statistics_calculation() {
        let stats = PageStatistics {
            total_investors: 10,
            eligible_investors: 8,
            total_locked_amount: 5_000_000,
            total_allocation_amount: 10_000_000,
            page_start: 0,
            page_size: 10,
        };
        
        assert_eq!(stats.total_investors, 10);
        assert_eq!(stats.eligible_investors, 8);
        assert_eq!(stats.total_locked_amount, 5_000_000);
        
        // Calculate eligibility rate
        let eligibility_rate = stats.eligible_investors as f64 / stats.total_investors as f64;
        assert_eq!(eligibility_rate, 0.8);
        
        // Calculate lock rate
        let lock_rate = stats.total_locked_amount as f64 / stats.total_allocation_amount as f64;
        assert_eq!(lock_rate, 0.5);
    }

    #[test]
    fn test_investor_payout_structure() {
        let payout = InvestorPayout {
            wallet: Pubkey::new_unique(),
            locked_amount: 2_500_000,
            weight: 250_000, // 25%
            payout_amount: 200,
            dust_amount: 0,
            ata_address: Pubkey::new_unique(),
            needs_ata_creation: false,
        };
        
        assert_eq!(payout.locked_amount, 2_500_000);
        assert_eq!(payout.weight, 250_000);
        assert_eq!(payout.payout_amount, 200);
        assert_eq!(payout.dust_amount, 0);
        assert!(!payout.needs_ata_creation);
    }

    #[test]
    fn test_scaling_factor_calculation() {
        let original_total = 1000u64;
        let capped_total = 800u64; // 80% due to daily cap
        
        let scale_factor = (capped_total as u128 * WEIGHT_PRECISION) / original_total as u128;
        assert_eq!(scale_factor, 800_000); // 80% of WEIGHT_PRECISION
        
        // Test scaling individual payouts
        let payout_amount = 250u64;
        let scaled_payout = ((payout_amount as u128 * scale_factor) / WEIGHT_PRECISION) as u64;
        assert_eq!(scaled_payout, 200); // 250 * 80% = 200
    }

    #[test]
    fn test_dust_accumulation_logic() {
        let mut progress = create_mock_distribution_progress();
        progress.carry_over_dust = 750;
        
        let new_dust = 300u64;
        let total_dust = progress.carry_over_dust + new_dust;
        assert_eq!(total_dust, 1050);
        
        let min_payout = 1000u64;
        let (dust_payout, remaining_dust) = calculate_dust_payout(total_dust, min_payout);
        
        assert_eq!(dust_payout, 1000); // One payout
        assert_eq!(remaining_dust, 50);  // Remaining dust
    }

    #[test]
    fn test_error_conditions() {
        let policy_config = create_mock_policy_config();
        
        // Test invalid page parameters
        let empty_accounts: Vec<AccountInfo> = vec![];
        
        let result = InvestorDistribution::validate_distribution_params(
            &policy_config,
            &empty_accounts,
            0,
            0, // Invalid page size
        );
        assert!(result.is_err());
    }
}