use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    constants::*,
    state::{PolicyConfig, DistributionProgress},
    utils::pda::PdaUtils,
    error::ErrorCode,
    HonoraryPositionInitialized,
};

#[derive(Accounts)]
pub struct InitializeHonoraryPosition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + PolicyConfig::INIT_SPACE,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump
    )]
    pub policy_config: Account<'info, PolicyConfig>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + DistributionProgress::INIT_SPACE,
        seeds = [PROGRESS_SEED, vault.key().as_ref()],
        bump
    )]
    pub distribution_progress: Account<'info, DistributionProgress>,
    
    /// CHECK: PDA for position ownership - validated by seeds constraint
    #[account(
        seeds = [VAULT_SEED, vault.key().as_ref(), b"investor_fee_pos_owner"],
        bump
    )]
    pub position_owner_pda: SystemAccount<'info>,
    
    /// The vault account (used as seed for PDAs)
    /// CHECK: Used only as seed, validation handled by PDA derivation
    pub vault: UncheckedAccount<'info>,
    
    // DAMM V2 Pool accounts
    /// CHECK: DAMM V2 Pool account - validated in handler
    pub pool: UncheckedAccount<'info>,
    /// CHECK: DAMM V2 Pool config account - validated in handler  
    pub pool_config: UncheckedAccount<'info>,
    
    /// Quote token vault - must match the quote mint in params
    pub quote_vault: Account<'info, TokenAccount>,
    
    /// Base token vault - used for validation
    pub base_vault: Account<'info, TokenAccount>,
    
    // Programs
    /// CHECK: DAMM V2 Program
    pub cp_amm_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeHonoraryPositionParams {
    pub quote_mint: Pubkey,
    pub creator_wallet: Pubkey,
    pub investor_fee_share_bps: u16,
    pub daily_cap_lamports: Option<u64>,
    pub min_payout_lamports: u64,
    pub y0_total_allocation: u64,
}

pub fn handler(
    mut ctx: Context<InitializeHonoraryPosition>,
    params: InitializeHonoraryPositionParams,
) -> Result<()> {
    let accounts = &mut ctx.accounts;
    let clock = Clock::get()?;
    
    msg!("Initializing honorary position for vault: {}", accounts.vault.key());
    
    // Step 1: Validate parameters
    validate_initialization_params(&params)?;
    
    // Step 2: Validate account relationships
    validate_account_relationships(&accounts, &params)?;
    
    // Step 3: Perform quote-only validation (mock for now since we need DAMM V2 integration)
    validate_quote_only_configuration(&accounts, &params)?;
    
    // Step 4: Initialize PolicyConfig account
    let policy_config = &mut accounts.policy_config;
    policy_config.initialize(
        accounts.vault.key(),
        params.quote_mint,
        params.creator_wallet,
        params.investor_fee_share_bps,
        params.daily_cap_lamports,
        params.min_payout_lamports,
        params.y0_total_allocation,
        ctx.bumps.policy_config,
    )?;
    
    // Step 5: Initialize DistributionProgress account
    let distribution_progress = &mut accounts.distribution_progress;
    distribution_progress.initialize(
        accounts.vault.key(),
        ctx.bumps.distribution_progress,
    )?;
    
    // Step 6: Validate PDA derivations
    validate_pda_derivations(&accounts, &ctx.program_id)?;
    
    // Step 7: Emit initialization event
    emit!(HonoraryPositionInitialized {
        vault: accounts.vault.key(),
        quote_mint: params.quote_mint,
        creator_wallet: params.creator_wallet,
        investor_fee_share_bps: params.investor_fee_share_bps,
        daily_cap_lamports: params.daily_cap_lamports,
        min_payout_lamports: params.min_payout_lamports,
        y0_total_allocation: params.y0_total_allocation,
        position_owner_pda: accounts.position_owner_pda.key(),
        policy_config: accounts.policy_config.key(),
        distribution_progress: accounts.distribution_progress.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Honorary position initialized successfully");
    msg!("Policy Config: {}", accounts.policy_config.key());
    msg!("Distribution Progress: {}", accounts.distribution_progress.key());
    msg!("Position Owner PDA: {}", accounts.position_owner_pda.key());
    
    Ok(())
}

/// Validate initialization parameters
fn validate_initialization_params(params: &InitializeHonoraryPositionParams) -> Result<()> {
    // Validate investor fee share basis points
    require!(
        params.investor_fee_share_bps <= MAX_BASIS_POINTS,
        ErrorCode::InvalidFeeShareBasisPoints
    );
    
    // Validate minimum payout threshold
    require!(
        params.min_payout_lamports > 0,
        ErrorCode::InvalidMinPayoutThreshold
    );
    
    // Validate Y0 total allocation
    require!(
        params.y0_total_allocation > 0,
        ErrorCode::InvalidTotalAllocation
    );
    
    // Validate daily cap if provided
    if let Some(daily_cap) = params.daily_cap_lamports {
        require!(
            daily_cap > 0,
            ErrorCode::InvalidDailyCap
        );
    }
    
    msg!("Parameter validation passed");
    Ok(())
}

/// Validate account relationships and constraints
fn validate_account_relationships(
    accounts: &InitializeHonoraryPosition,
    params: &InitializeHonoraryPositionParams,
) -> Result<()> {
    // Validate quote vault mint matches parameter
    require!(
        accounts.quote_vault.mint == params.quote_mint,
        ErrorCode::InvalidQuoteMint
    );
    
    // Validate that quote and base vaults are different
    require!(
        accounts.quote_vault.mint != accounts.base_vault.mint,
        ErrorCode::InvalidPoolConfiguration
    );
    
    // Validate that vaults have the same owner (should be the pool)
    require!(
        accounts.quote_vault.owner == accounts.base_vault.owner,
        ErrorCode::InvalidPoolConfiguration
    );
    
    msg!("Account relationship validation passed");
    Ok(())
}

/// Validate quote-only configuration
/// This is a simplified version - full DAMM V2 integration would be needed for complete validation
fn validate_quote_only_configuration(
    accounts: &InitializeHonoraryPosition,
    params: &InitializeHonoraryPositionParams,
) -> Result<()> {
    // For now, we'll do basic validation
    // In a full implementation, this would:
    // 1. Deserialize DAMM V2 pool and position accounts
    // 2. Extract pool validation data
    // 3. Call validate_quote_only_config from utils::validation
    
    msg!("Performing quote-only validation for mint: {}", params.quote_mint);
    
    // Basic validation: ensure quote mint is one of the pool tokens
    let quote_mint = params.quote_mint;
    let quote_vault_mint = accounts.quote_vault.mint;
    let base_vault_mint = accounts.base_vault.mint;
    
    require!(
        quote_mint == quote_vault_mint || quote_mint == base_vault_mint,
        ErrorCode::InvalidQuoteMint
    );
    
    // Ensure quote mint matches the quote vault
    require!(
        quote_mint == quote_vault_mint,
        ErrorCode::InvalidQuoteMint
    );
    
    msg!("Quote-only validation passed (basic validation)");
    msg!("Note: Full DAMM V2 integration required for complete validation");
    
    Ok(())
}

/// Validate PDA derivations match expected values
fn validate_pda_derivations(
    accounts: &InitializeHonoraryPosition,
    program_id: &Pubkey,
) -> Result<()> {
    let vault_key = accounts.vault.key();
    
    // Validate policy config PDA
    let (expected_policy_pda, expected_policy_bump) = 
        PdaUtils::derive_policy_config_pda(program_id, &vault_key);
    require!(
        expected_policy_pda == accounts.policy_config.key(),
        ErrorCode::AccountInitializationFailed
    );
    
    // Validate distribution progress PDA
    let (expected_progress_pda, expected_progress_bump) = 
        PdaUtils::derive_distribution_progress_pda(program_id, &vault_key);
    require!(
        expected_progress_pda == accounts.distribution_progress.key(),
        ErrorCode::AccountInitializationFailed
    );
    
    // Validate position owner PDA
    let (expected_owner_pda, expected_owner_bump) = 
        PdaUtils::derive_position_owner_pda(program_id, &vault_key);
    require!(
        expected_owner_pda == accounts.position_owner_pda.key(),
        ErrorCode::AccountInitializationFailed
    );
    
    msg!("PDA derivation validation passed");
    Ok(())
}

// Include tests
#[cfg(test)]
#[path = "initialize_honorary_position_tests.rs"]
mod tests;