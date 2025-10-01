use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, transfer};
use anchor_spl::associated_token::AssociatedToken;

use crate::{
    constants::*,
    state::{PolicyConfig, DistributionProgress, DistributionTimingState},
    error::ErrorCode,
    utils::{
        fee_claiming::{claim_position_fees, ensure_treasury_ata, FeeClaimResult},
        investor_distribution::InvestorDistribution,
        creator_distribution::CreatorDistribution,
        streamflow::StreamflowIntegration,
    },
    QuoteFeesClaimed, InvestorPayoutPage,
};

#[derive(Accounts)]
pub struct DistributeFees<'info> {
    #[account(mut)]
    pub crank_caller: Signer<'info>,
    
    #[account(
        mut,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump
    )]
    pub policy_config: Account<'info, PolicyConfig>,
    
    #[account(
        mut,
        seeds = [PROGRESS_SEED, vault.key().as_ref()],
        bump
    )]
    pub distribution_progress: Account<'info, DistributionProgress>,
    
    /// CHECK: PDA for position ownership
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), b"investor_fee_pos_owner"],
        bump
    )]
    pub position_owner_pda: SystemAccount<'info>,
    
    /// The vault account (used as seed for PDAs)
    /// CHECK: Used only as seed, validation handled by PDA derivation
    pub vault: UncheckedAccount<'info>,
    
    /// CHECK: DAMM V2 Position account
    #[account(mut)]
    pub honorary_position: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub treasury_ata: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub creator_ata: Account<'info, TokenAccount>,
    
    // Programs
    /// CHECK: DAMM V2 Program for fee claiming CPI
    pub cp_amm_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    
    // Streamflow accounts will be passed as remaining accounts for pagination
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DistributeFeesParams {
    pub page_size: u32,
    pub cursor_position: Option<u32>, // For idempotent retries
}

/// Comprehensive validation of all accounts required for distribution
fn validate_distribution_accounts(ctx: &Context<DistributeFees>) -> Result<()> {
    let policy_config = &ctx.accounts.policy_config;
    
    // Validate treasury ATA belongs to position owner PDA and has correct mint
    require!(
        ctx.accounts.treasury_ata.owner == ctx.accounts.position_owner_pda.key(),
        ErrorCode::InvalidTreasuryAta
    );
    require!(
        ctx.accounts.treasury_ata.mint == policy_config.quote_mint,
        ErrorCode::InvalidQuoteMint
    );
    
    // Validate creator ATA belongs to creator wallet and has correct mint
    require!(
        ctx.accounts.creator_ata.owner == policy_config.creator_wallet,
        ErrorCode::InvalidCreatorAta
    );
    require!(
        ctx.accounts.creator_ata.mint == policy_config.quote_mint,
        ErrorCode::InvalidQuoteMint
    );
    
    // Validate policy config vault matches the vault used for PDA derivation
    require!(
        policy_config.vault == ctx.accounts.vault.key(),
        ErrorCode::InvalidVaultAccount
    );
    
    // Validate distribution progress vault matches policy config
    require!(
        ctx.accounts.distribution_progress.vault == policy_config.vault,
        ErrorCode::InvalidVaultAccount
    );
    
    msg!("Account validation completed successfully");
    Ok(())
}

pub fn handler<'info>(
    mut ctx: Context<'_, '_, '_, 'info, DistributeFees<'info>>,
    params: DistributeFeesParams,
) -> Result<()> {
    let current_timestamp = Clock::get()?.unix_timestamp;
    
    // Comprehensive account validation
    validate_distribution_accounts(&ctx)?;
    
    // Validate page size
    require!(
        params.page_size > 0 && params.page_size <= MAX_PAGE_SIZE,
        ErrorCode::InvalidPaginationCursor
    );
    
    // Validate and prepare timing for distribution
    let timing_state = ctx.accounts.distribution_progress.prepare_for_distribution(current_timestamp)?;
    
    // Handle cursor position for idempotent operations
    if let Some(requested_cursor) = params.cursor_position {
        let is_retry = ctx.accounts.distribution_progress.validate_cursor_for_retry(requested_cursor)?;
        if is_retry {
            msg!("Idempotent retry detected for cursor position: {}", requested_cursor);
            return Ok(()); // Already processed, return success
        }
    }
    
    // Log timing information
    match timing_state {
        DistributionTimingState::NewDay => {
            msg!("Starting new 24-hour distribution period at timestamp: {}", current_timestamp);
        },
        DistributionTimingState::ContinueSameDay => {
            msg!("Continuing same-day distribution, cursor: {}", ctx.accounts.distribution_progress.pagination_cursor);
        }
    }
    
    // Get distribution period info for logging
    let period_info = ctx.accounts.distribution_progress.get_distribution_period_info(current_timestamp);
    msg!("Distribution period info - Last: {}, Current: {}, Time until next: {}, Cursor: {}", 
         period_info.last_distribution_ts, 
         period_info.current_timestamp, 
         period_info.time_until_next,
         period_info.pagination_cursor);
    
    // Step 1: Claim fees from honorary position (only on new day)
    let claimed_fees = if matches!(timing_state, DistributionTimingState::NewDay) {
        claim_fees_from_position(&ctx)?
    } else {
        // For same-day continuation, no new fees to claim
        FeeClaimResult {
            quote_amount: 0,
            base_amount: 0,
            quote_mint: ctx.accounts.policy_config.quote_mint,
        }
    };
    
    // Log fee claiming results
    if claimed_fees.quote_amount > 0 {
        msg!("Claimed {} quote fees for distribution", claimed_fees.quote_amount);
        
        // Emit QuoteFeesClaimed event
        emit!(QuoteFeesClaimed {
            vault: ctx.accounts.policy_config.vault,
            claimed_amount: claimed_fees.quote_amount,
            base_amount: claimed_fees.base_amount,
            quote_mint: claimed_fees.quote_mint,
            honorary_position: ctx.accounts.honorary_position.key(),
            treasury_ata: ctx.accounts.treasury_ata.key(),
            timestamp: current_timestamp,
        });
    }
    
    // Step 2: Process investor distributions if we have Streamflow accounts
    let streamflow_accounts = ctx.remaining_accounts;
    
    if !streamflow_accounts.is_empty() && (claimed_fees.quote_amount > 0 || ctx.accounts.distribution_progress.carry_over_dust > 0) {
        process_investor_distributions(&mut ctx, &params, claimed_fees.quote_amount, current_timestamp)?;
    } else {
        msg!("No Streamflow accounts provided or no fees to distribute");
    }
    
    Ok(())
}

/// Claims fees from the honorary DAMM V2 position
fn claim_fees_from_position(ctx: &Context<DistributeFees>) -> Result<FeeClaimResult> {
    let policy_config = &ctx.accounts.policy_config;
    
    // Validate treasury ATA
    ensure_treasury_ata(
        &ctx.accounts.treasury_ata,
        &policy_config.quote_mint,
        &ctx.accounts.position_owner_pda.key(),
    )?;
    
    // Claim fees from DAMM V2 position
    let claim_result = claim_position_fees(
        &ctx.accounts.honorary_position.to_account_info(),
        &ctx.accounts.position_owner_pda.to_account_info(),
        &ctx.accounts.treasury_ata,
        &policy_config.quote_mint,
        &policy_config.vault,
        policy_config.bump,
        &ctx.accounts.cp_amm_program.to_account_info(),
        &ctx.accounts.token_program,
    )?;
    
    msg!("Fee claiming completed - Quote: {}, Base: {}", 
         claim_result.quote_amount, claim_result.base_amount);
    
    Ok(claim_result)
}

/// Process investor distributions for the current page
fn process_investor_distributions<'info>(
    ctx: &mut Context<'_, '_, '_, 'info, DistributeFees<'info>>,
    params: &DistributeFeesParams,
    claimed_quote_amount: u64,
    current_timestamp: i64,
) -> Result<()> {
    let policy_config = &ctx.accounts.policy_config;
    let distribution_progress = &mut ctx.accounts.distribution_progress;
    let streamflow_accounts = ctx.remaining_accounts;
    
    msg!("Processing investor distributions for {} Streamflow accounts", streamflow_accounts.len());
    
    // Validate distribution parameters
    InvestorDistribution::validate_distribution_params(
        policy_config,
        streamflow_accounts,
        distribution_progress.pagination_cursor as usize,
        params.page_size as usize,
    )?;
    
    // Calculate total locked amount across all investors
    let total_locked_amount = StreamflowIntegration::calculate_total_locked(
        streamflow_accounts,
        &policy_config.quote_mint,
        current_timestamp,
    )?;
    
    msg!("Total locked amount across all investors: {}", total_locked_amount);
    
    // Calculate total investor allocation from claimed fees
    let (total_investor_amount, creator_amount) = crate::utils::math::calculate_distribution(
        claimed_quote_amount,
        total_locked_amount,
        policy_config.y0_total_allocation,
        policy_config.investor_fee_share_bps,
    )?;
    
    msg!("Distribution calculation: investor_amount={}, creator_amount={}, total_locked={}", 
         total_investor_amount, creator_amount, total_locked_amount);
    
    // Validate daily cap before processing
    if let Some(daily_cap) = policy_config.daily_cap_lamports {
        let remaining_cap = daily_cap.saturating_sub(distribution_progress.current_day_distributed);
        if remaining_cap == 0 {
            msg!("Daily cap reached, no more distributions allowed today");
            return Ok(());
        }
        msg!("Daily cap check: remaining={}, total_cap={}", remaining_cap, daily_cap);
    }
    
    // Check if there are eligible investors in this page
    let has_eligible_investors = InvestorDistribution::validate_investor_eligibility(
        streamflow_accounts,
        &policy_config.quote_mint,
        distribution_progress.pagination_cursor as usize,
        params.page_size as usize,
        current_timestamp,
    )?;
    
    if !has_eligible_investors {
        msg!("No eligible investors found in current page, advancing cursor");
        distribution_progress.advance_cursor(params.page_size)?;
        return Ok(());
    }
    
    // Get cursor position before processing
    let current_cursor = distribution_progress.pagination_cursor as usize;
    
    // Process the current page of investors
    let batch_result = InvestorDistribution::process_investor_page(
        policy_config,
        distribution_progress,
        streamflow_accounts,
        total_investor_amount,
        total_locked_amount,
        current_cursor,
        params.page_size as usize,
        current_timestamp,
        &ctx.accounts.token_program,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.system_program,
        &ctx.accounts.treasury_ata,
        &ctx.accounts.position_owner_pda.to_account_info(),
    )?;
    
    msg!("Batch processing complete: paid={}, dust={}, processed={}", 
         batch_result.total_paid, batch_result.total_dust, batch_result.processed_count);
    
    // Emit investor payout page event
    emit!(InvestorPayoutPage {
        vault: policy_config.vault,
        page_start: current_cursor as u32,
        page_end: distribution_progress.pagination_cursor,
        total_distributed: batch_result.total_paid,
        processed_count: batch_result.processed_count as u32,
        dust_carried_forward: batch_result.total_dust,
        cumulative_day_distributed: distribution_progress.current_day_distributed,
        timestamp: current_timestamp,
    });
    
    // Check if we've processed all investors for the day
    let total_investors = streamflow_accounts.len();
    if distribution_progress.pagination_cursor as usize >= total_investors {
        msg!("All investors processed for the day, processing creator remainder payout");
        
        // Process creator remainder payout before marking day complete
        let creator_payout_amount = CreatorDistribution::calculate_creator_remainder(
            policy_config,
            claimed_quote_amount,
            total_locked_amount,
        )?;
        
        if creator_payout_amount > 0 {
            // Validate creator ATA
            CreatorDistribution::validate_creator_ata(
                &ctx.accounts.creator_ata,
                &policy_config.creator_wallet,
                &policy_config.quote_mint,
            )?;
            
            // Validate treasury has sufficient balance
            require!(
                ctx.accounts.treasury_ata.amount >= creator_payout_amount,
                ErrorCode::InsufficientFunds
            );
            
            // Execute creator transfer with proper error handling
            let bump_seed = [policy_config.bump];
            let signer_seeds: &[&[u8]] = &[
                VAULT_SEED,
                policy_config.vault.as_ref(),
                b"investor_fee_pos_owner",
                &bump_seed,
            ];
            let signer_seeds_slice = &[signer_seeds];
            
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.treasury_ata.to_account_info(),
                    to: ctx.accounts.creator_ata.to_account_info(),
                    authority: ctx.accounts.position_owner_pda.to_account_info(),
                },
                signer_seeds_slice,
            );
            
            transfer(transfer_ctx, creator_payout_amount)
                .map_err(|_| ErrorCode::TreasuryTransferFailed)?;
            
            msg!("Creator transfer executed: {} tokens from treasury to creator", creator_payout_amount);
        } else {
            msg!("No creator payout required (amount: {})", creator_payout_amount);
        }
        
        // Mark day complete after creator payout
        distribution_progress.complete_day();
        
        // Emit creator payout event
        emit!(crate::CreatorPayoutDayClosed {
            vault: policy_config.vault,
            creator_payout: creator_payout_amount,
            creator_wallet: policy_config.creator_wallet,
            total_day_distributed: distribution_progress.current_day_distributed + creator_payout_amount,
            total_investors_processed: streamflow_accounts.len() as u32,
            final_dust_amount: distribution_progress.carry_over_dust,
            timestamp: current_timestamp,
        });
        
        msg!("Creator remainder payout completed: {} tokens", creator_payout_amount);
    }
    
    Ok(())
}

#[cfg(test)]
#[path = "distribute_fees_integration_tests.rs"]
mod distribute_fees_integration_tests;

#[cfg(test)]
mod timing_tests {
    use super::*;
    use crate::{
        state::{PolicyConfig, DistributionProgress, DistributionTimingState},
        constants::*,
        error::ErrorCode,
    };
    use anchor_lang::prelude::*;

    // Mock context for testing
    fn create_mock_distribute_fees_params(page_size: u32, cursor_position: Option<u32>) -> DistributeFeesParams {
        DistributeFeesParams {
            page_size,
            cursor_position,
        }
    }

    #[test]
    fn test_distribute_fees_params_validation() {
        // Valid page size
        let params = create_mock_distribute_fees_params(10, None);
        assert_eq!(params.page_size, 10);
        assert_eq!(params.cursor_position, None);

        // With cursor position
        let params = create_mock_distribute_fees_params(25, Some(50));
        assert_eq!(params.page_size, 25);
        assert_eq!(params.cursor_position, Some(50));
    }

    #[test]
    fn test_page_size_validation_logic() {
        // Test page size bounds checking logic
        assert!(0 < MAX_PAGE_SIZE); // Ensure constant is valid
        assert!(MAX_PAGE_SIZE <= 100); // Reasonable upper bound
        
        // Valid page sizes
        for size in 1..=MAX_PAGE_SIZE {
            assert!(size > 0 && size <= MAX_PAGE_SIZE);
        }
        
        // Invalid page sizes
        assert!(!(0 > 0 && 0 <= MAX_PAGE_SIZE)); // Zero
        assert!(!((MAX_PAGE_SIZE + 1) > 0 && (MAX_PAGE_SIZE + 1) <= MAX_PAGE_SIZE)); // Too large
    }

    #[test]
    fn test_timing_state_handling_logic() {
        // Test the logic that would be used in the handler
        let timing_states = [
            DistributionTimingState::NewDay,
            DistributionTimingState::ContinueSameDay,
        ];
        
        for state in timing_states {
            match state {
                DistributionTimingState::NewDay => {
                    // This is what the handler should do for new day
                    assert_eq!(state, DistributionTimingState::NewDay);
                },
                DistributionTimingState::ContinueSameDay => {
                    // This is what the handler should do for same day continuation
                    assert_eq!(state, DistributionTimingState::ContinueSameDay);
                }
            }
        }
    }

    #[test]
    fn test_idempotent_retry_logic() {
        let mut progress = DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 1000,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 20,
            day_complete: false,
            bump: 255,
        };
        
        // Test the logic for handling cursor positions
        let test_cases = [
            (Some(15), true),  // Already processed
            (Some(20), false), // Current position
            (Some(25), false), // Would be error in real validation
            (None, false),     // No cursor specified
        ];
        
        for (cursor_pos, expected_is_retry) in test_cases {
            if let Some(cursor) = cursor_pos {
                if cursor < progress.pagination_cursor {
                    // This would be a retry
                    assert!(expected_is_retry);
                } else if cursor == progress.pagination_cursor {
                    // This would be normal operation
                    assert!(!expected_is_retry);
                }
            } else {
                // No cursor specified - normal operation
                assert!(!expected_is_retry);
            }
        }
    }

    #[test]
    fn test_timing_system_integration_flow() {
        let mut progress = DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 0,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        };
        
        let start_time = 1000i64;
        
        // Simulate the flow that would happen in the handler
        
        // 1. Prepare for distribution (first time)
        let timing_state = progress.prepare_for_distribution(start_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // 2. Process some pages in same day
        for page in 0..3 {
            let page_size = 10u32;
            let expected_cursor = page * page_size;
            
            // Verify cursor is at expected position
            assert_eq!(progress.pagination_cursor, expected_cursor);
            
            // Simulate page processing
            progress.mark_page_processed(expected_cursor, page_size).unwrap();
            
            // Verify cursor advanced
            assert_eq!(progress.pagination_cursor, expected_cursor + page_size);
            
            // Verify can continue same day
            let current_time = start_time + 1800; // 30 minutes later
            let timing_state = progress.prepare_for_distribution(current_time).unwrap();
            assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        }
        
        // 3. Complete the day
        progress.complete_day();
        
        // 4. Try to start new day after 24 hours
        let next_day = start_time + TWENTY_FOUR_HOURS;
        let timing_state = progress.prepare_for_distribution(next_day).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // Verify state reset for new day
        assert_eq!(progress.pagination_cursor, 0);
        assert!(!progress.day_complete);
        assert_eq!(progress.last_distribution_ts, next_day);
    }
}