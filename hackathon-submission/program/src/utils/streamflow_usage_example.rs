/// Example usage of Streamflow integration in the distribution instruction
/// This demonstrates how the Streamflow module integrates with the main program logic

#[cfg(test)]
mod usage_examples {
    use super::*;
    use crate::utils::{StreamflowIntegration, InvestorData};
    use crate::state::{PolicyConfig, DistributionProgress};
    use anchor_lang::prelude::*;

    /// Example: Processing a page of investors during fee distribution
    /// This simulates how the DistributeFees instruction would use Streamflow integration
    pub fn example_process_investor_page(
        policy_config: &PolicyConfig,
        distribution_progress: &mut DistributionProgress,
        stream_accounts: &[AccountInfo],
        claimed_quote_fees: u64,
        page_size: usize,
    ) -> Result<(Vec<(Pubkey, u64)>, bool)> {
        // Get current timestamp
        let current_timestamp = StreamflowIntegration::get_current_timestamp()?;
        
        // Validate 24-hour cooldown
        if current_timestamp < distribution_progress.last_distribution_ts + 86400 {
            return Err(crate::error::ErrorCode::CooldownNotElapsed.into());
        }
        
        // Calculate pagination bounds
        let page_start = distribution_progress.pagination_cursor as usize;
        let page_end = std::cmp::min(page_start + page_size, stream_accounts.len());
        let page_accounts = &stream_accounts[page_start..page_end];
        
        // Process this page of Streamflow accounts
        let (processed_count, page_locked_total, investor_payouts) = 
            StreamflowIntegration::process_investor_page(
                page_accounts,
                &policy_config.quote_mint,
                current_timestamp,
                0, // Start from beginning of page
                page_size,
            )?;
        
        // Calculate total locked across ALL investors (for distribution calculation)
        let total_locked = StreamflowIntegration::calculate_total_locked(
            stream_accounts,
            &policy_config.quote_mint,
            current_timestamp,
        )?;
        
        // Calculate distribution amounts
        let (total_investor_amount, creator_amount) = crate::utils::math::calculate_distribution(
            claimed_quote_fees,
            total_locked,
            policy_config.y0_total_allocation,
            policy_config.investor_fee_share_bps,
        )?;
        
        // Calculate individual payouts based on weights
        let locked_amounts: Vec<u64> = investor_payouts.iter().map(|(_, locked)| *locked).collect();
        let weights = crate::utils::math::calculate_all_weights(&locked_amounts, page_locked_total)?;
        
        let mut final_payouts = Vec::new();
        let mut total_page_payout = 0u64;
        
        for (i, (investor_wallet, locked_amount)) in investor_payouts.iter().enumerate() {
            if *locked_amount == 0 {
                continue; // Skip investors with no locked tokens
            }
            
            // Calculate this investor's share of the total investor amount
            let investor_payout = (total_investor_amount as u128 * weights[i] as u128 / crate::constants::WEIGHT_PRECISION as u128) as u64;
            
            // Apply minimum payout threshold
            if investor_payout >= policy_config.min_payout_lamports {
                final_payouts.push((*investor_wallet, investor_payout));
                total_page_payout += investor_payout;
            } else {
                // Accumulate dust for later distribution
                distribution_progress.carry_over_dust += investor_payout;
            }
        }
        
        // Update pagination cursor
        distribution_progress.pagination_cursor = page_end as u32;
        let is_last_page = page_end >= stream_accounts.len();
        
        // Update distribution progress
        distribution_progress.current_day_distributed += total_page_payout;
        
        if is_last_page {
            distribution_progress.day_complete = true;
            distribution_progress.last_distribution_ts = current_timestamp;
        }
        
        Ok((final_payouts, is_last_page))
    }
    
    /// Example: Validating Streamflow accounts before starting distribution
    pub fn example_validate_streamflow_setup(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        min_investors: usize,
    ) -> Result<()> {
        // Validate all streams belong to the expected mint
        StreamflowIntegration::validate_all_streams_mint(stream_accounts, expected_mint)?;
        
        // Check minimum investor count
        let unique_investors = StreamflowIntegration::get_unique_investor_count(stream_accounts, expected_mint)?;
        if unique_investors < min_investors {
            return Err(crate::error::ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Validate that there are actually locked tokens
        let current_timestamp = StreamflowIntegration::get_current_timestamp()?;
        let total_locked = StreamflowIntegration::calculate_total_locked(
            stream_accounts,
            expected_mint,
            current_timestamp,
        )?;
        
        if total_locked == 0 {
            return Err(crate::error::ErrorCode::StreamflowValidationFailed.into());
        }
        
        Ok(())
    }
    
    /// Example: Handling dust accumulation and payout
    pub fn example_handle_dust_payout(
        distribution_progress: &mut DistributionProgress,
        policy_config: &PolicyConfig,
    ) -> Result<u64> {
        let (dust_payout, remaining_dust) = crate::utils::math::calculate_dust_payout(
            distribution_progress.carry_over_dust,
            policy_config.min_payout_lamports,
        );
        
        distribution_progress.carry_over_dust = remaining_dust;
        
        Ok(dust_payout)
    }
    
    /// Example: Complete distribution cycle simulation
    pub fn example_complete_distribution_cycle() -> Result<()> {
        // Mock setup
        let mint = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let vault = Pubkey::new_unique();
        
        let policy_config = PolicyConfig {
            vault,
            quote_mint: mint,
            creator_wallet: creator,
            investor_fee_share_bps: 7500, // 75%
            daily_cap_lamports: Some(1_000_000),
            min_payout_lamports: 1000,
            y0_total_allocation: 10_000_000,
            bump: 255,
        };
        
        let mut distribution_progress = DistributionProgress {
            vault,
            last_distribution_ts: 0, // Allow distribution
            current_day_distributed: 0,
            carry_over_dust: 500, // Some existing dust
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        };
        
        // Simulate claimed fees
        let claimed_quote_fees = 100_000u64;
        
        // This would be the actual Streamflow accounts in a real scenario
        // For this example, we'll simulate the process without actual AccountInfo
        
        // Step 1: Validate setup (would use real stream_accounts)
        // example_validate_streamflow_setup(&stream_accounts, &mint, 1)?;
        
        // Step 2: Process pages of investors
        let page_size = 10;
        let mut all_payouts = Vec::new();
        let mut total_distributed = 0u64;
        
        // Simulate processing multiple pages
        for page in 0..3 {
            // In real implementation, this would process actual stream accounts
            // For simulation, we'll create mock payouts
            let mock_payouts = vec![
                (Pubkey::new_unique(), 5000),
                (Pubkey::new_unique(), 3000),
                (Pubkey::new_unique(), 2000),
            ];
            
            for (investor, amount) in mock_payouts {
                all_payouts.push((investor, amount));
                total_distributed += amount;
            }
            
            distribution_progress.current_day_distributed += total_distributed;
            distribution_progress.pagination_cursor += page_size;
        }
        
        // Step 3: Handle dust payout
        let dust_payout = example_handle_dust_payout(&mut distribution_progress, &policy_config)?;
        if dust_payout > 0 {
            all_payouts.push((creator, dust_payout));
        }
        
        // Step 4: Calculate creator remainder
        let total_investor_distributed = total_distributed + dust_payout;
        let creator_remainder = claimed_quote_fees.saturating_sub(total_investor_distributed);
        if creator_remainder > 0 {
            all_payouts.push((creator, creator_remainder));
        }
        
        // Step 5: Mark day as complete
    distribution_progress.day_complete = true;
    distribution_progress.last_distribution_ts = StreamflowIntegration::get_current_timestamp().unwrap_or_default();
        distribution_progress.pagination_cursor = 0; // Reset for next day
        
        // Verify total distribution
        let total_paid: u64 = all_payouts.iter().map(|(_, amount)| amount).sum();
        assert!(total_paid <= claimed_quote_fees);
        
        Ok(())
    }
}

/// Integration patterns and best practices
#[cfg(test)]
mod integration_patterns {
    use super::*;
    
    /// Pattern 1: Batch processing with error recovery
    pub fn pattern_batch_processing_with_recovery() {
        // When processing large numbers of investors, implement checkpointing
        // so that partial failures can be recovered from
        
        // Pseudocode:
        // 1. Save pagination cursor before processing each page
        // 2. Process page atomically
        // 3. Update cursor only after successful page processing
        // 4. On failure, cursor remains at last successful position
        // 5. Retry can resume from cursor position
    }
    
    /// Pattern 2: Gas optimization for large investor sets
    pub fn pattern_gas_optimization() {
        // For large investor sets, consider:
        // 1. Optimal page sizes (balance between gas usage and number of transactions)
        // 2. Precompute total locked amounts when possible
        // 3. Use efficient data structures for weight calculations
        // 4. Minimize cross-program invocations per transaction
    }
    
    /// Pattern 3: Handling edge cases gracefully
    pub fn pattern_edge_case_handling() {
        // Common edge cases to handle:
        // 1. All investors fully vested (no locked tokens)
        // 2. Single investor with all locked tokens
        // 3. Very small amounts that create dust
        // 4. Streams that become fully vested during processing
        // 5. Invalid or corrupted Streamflow accounts
    }
    
    /// Pattern 4: Event emission for monitoring
    pub fn pattern_event_emission() {
        // Emit events at key points:
        // 1. Start of distribution cycle
        // 2. Completion of each investor page
        // 3. Dust accumulation and payout
        // 4. Creator remainder distribution
        // 5. End of distribution cycle
        // 6. Error conditions and recovery
    }
}

#[cfg(test)]
mod tests {
    use super::usage_examples::*;
    
    #[test]
    fn test_complete_distribution_simulation() {
        let result = example_complete_distribution_cycle();
        assert!(result.is_ok(), "Complete distribution cycle should succeed");
    }
}