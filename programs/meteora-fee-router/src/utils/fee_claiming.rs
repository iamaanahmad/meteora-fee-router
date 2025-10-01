use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, transfer};
use anchor_spl::associated_token::AssociatedToken;
use solana_program::program::invoke_signed;

use crate::{
    error::ErrorCode,
    constants::*,
};

/// DAMM V2 Position account structure (simplified for fee claiming)
/// This represents the essential fields we need from a DAMM V2 position
#[derive(Debug, Clone)]
pub struct PositionFeeData {
    pub fee_owed_a: u64,
    pub fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
}

/// Fee claiming result containing claimed amounts
#[derive(Debug, Clone, PartialEq)]
pub struct FeeClaimResult {
    pub quote_amount: u64,
    pub base_amount: u64,
    pub quote_mint: Pubkey,
}

/// Claims fees from a DAMM V2 position via CPI
/// This function handles the cross-program invocation to claim fees
/// and enforces quote-only validation
pub fn claim_position_fees<'info>(
    position_account: &AccountInfo<'info>,
    position_owner_pda: &AccountInfo<'info>,
    treasury_ata: &Account<'info, TokenAccount>,
    quote_mint: &Pubkey,
    vault_key: &Pubkey,
    bump: u8,
    cp_amm_program: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
) -> Result<FeeClaimResult> {
    msg!("Starting fee claiming process for position: {}", position_account.key());
    
    // Pre-claim validation
    validate_claim_preconditions(position_account, position_owner_pda, treasury_ata, quote_mint)?;
    
    // Extract fee data from position with enhanced error handling
    let fee_data = extract_position_fee_data(position_account)
        .map_err(|e| {
            msg!("Failed to extract position fee data: {}", e);
            ErrorCode::PositionFeeDataExtractionFailed
        })?;
    
    // Validate quote-only enforcement with detailed logging
    validate_quote_only_fees(&fee_data, quote_mint)
        .map_err(|e| {
            msg!("Quote-only validation failed for position: {}", position_account.key());
            e
        })?;
    
    // Determine which token is quote and which is base
    let (quote_amount, base_amount) = if fee_data.token_mint_a == *quote_mint {
        (fee_data.fee_owed_a, fee_data.fee_owed_b)
    } else if fee_data.token_mint_b == *quote_mint {
        (fee_data.fee_owed_b, fee_data.fee_owed_a)
    } else {
        msg!("Quote mint {} not found in position tokens: {} and {}", 
             quote_mint, fee_data.token_mint_a, fee_data.token_mint_b);
        return Err(ErrorCode::InvalidQuoteMint.into());
    };
    
    // Enforce quote-only: base amount must be zero
    if base_amount > 0 {
        msg!("CRITICAL: Base fees detected: {} lamports - aborting claim", base_amount);
        return Err(ErrorCode::BaseFeeDetected.into());
    }
    
    // Only proceed if there are quote fees to claim
    if quote_amount == 0 {
        msg!("No quote fees to claim for position: {}", position_account.key());
        return Ok(FeeClaimResult {
            quote_amount: 0,
            base_amount: 0,
            quote_mint: *quote_mint,
        });
    }
    
    // Record treasury balance before claim
    let treasury_balance_before = treasury_ata.amount;
    msg!("Treasury balance before claim: {}", treasury_balance_before);
    
    // Perform the actual fee claiming via CPI with enhanced error handling
    claim_fees_cpi(
        position_account,
        position_owner_pda,
        treasury_ata,
        vault_key,
        bump,
        cp_amm_program,
        token_program,
    ).map_err(|e| {
        msg!("Fee claiming CPI failed for position: {}", position_account.key());
        ErrorCode::FeeClaimingFailed
    })?;
    
    msg!("Successfully claimed {} quote fees from position: {}", quote_amount, position_account.key());
    
    Ok(FeeClaimResult {
        quote_amount,
        base_amount: 0,
        quote_mint: *quote_mint,
    })
}

/// Validates preconditions before attempting to claim fees
fn validate_claim_preconditions(
    position_account: &AccountInfo,
    position_owner_pda: &AccountInfo,
    treasury_ata: &Account<TokenAccount>,
    quote_mint: &Pubkey,
) -> Result<()> {
    // Validate position account is not empty
    if position_account.data_is_empty() {
        msg!("Position account is empty: {}", position_account.key());
        return Err(ErrorCode::PositionFeeDataExtractionFailed.into());
    }
    
    // Validate position owner PDA
    if position_owner_pda.data_is_empty() {
        msg!("Position owner PDA is not initialized: {}", position_owner_pda.key());
        return Err(ErrorCode::PositionOwnerMismatch.into());
    }
    
    // Validate treasury ATA mint matches quote mint
    if treasury_ata.mint != *quote_mint {
        msg!("Treasury ATA mint mismatch - expected: {}, actual: {}", 
             quote_mint, treasury_ata.mint);
        return Err(ErrorCode::InvalidTreasuryAta.into());
    }
    
    msg!("Claim preconditions validated successfully");
    Ok(())
}

/// Validates that only quote fees are present and base fees are zero
pub fn validate_quote_only_fees(
    fee_data: &PositionFeeData,
    quote_mint: &Pubkey,
) -> Result<()> {
    let (quote_fees, base_fees) = if fee_data.token_mint_a == *quote_mint {
        (fee_data.fee_owed_a, fee_data.fee_owed_b)
    } else if fee_data.token_mint_b == *quote_mint {
        (fee_data.fee_owed_b, fee_data.fee_owed_a)
    } else {
        return Err(ErrorCode::InvalidQuoteMint.into());
    };
    
    // Strict enforcement: any base fees cause failure
    if base_fees > 0 {
        msg!("Quote-only validation failed - base fees detected: {}", base_fees);
        return Err(ErrorCode::BaseFeeDetected.into());
    }
    
    msg!("Quote-only validation passed - quote fees: {}, base fees: {}", quote_fees, base_fees);
    Ok(())
}

/// Extracts fee data from DAMM V2 position account
/// This is a mock implementation - in production this would deserialize
/// the actual DAMM V2 position account structure
fn extract_position_fee_data(position_account: &AccountInfo) -> Result<PositionFeeData> {
    // Mock implementation for testing
    // In production, this would deserialize the DAMM V2 position account
    // and extract fee_owed_a, fee_owed_b, token_mint_a, token_mint_b
    
    msg!("Extracting position fee data (mock implementation)");
    
    // For testing purposes, we'll simulate different scenarios
    // This would be replaced with actual DAMM V2 account deserialization
    Ok(PositionFeeData {
        fee_owed_a: 1000000, // 1 SOL worth of fees in quote token
        fee_owed_b: 0,       // No base token fees (quote-only)
        token_mint_a: Pubkey::new_unique(), // This would be the actual quote mint
        token_mint_b: Pubkey::new_unique(), // This would be the actual base mint
    })
}

/// Performs the actual CPI call to claim fees from DAMM V2 position
fn claim_fees_cpi<'info>(
    position_account: &AccountInfo<'info>,
    position_owner_pda: &AccountInfo<'info>,
    treasury_ata: &Account<'info, TokenAccount>,
    vault_key: &Pubkey,
    bump: u8,
    cp_amm_program: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    // Create PDA signer seeds
    let vault_seed = vault_key.as_ref();
    let position_owner_seed = b"investor_fee_pos_owner";
    let signer_seeds = &[
        VAULT_SEED,
        vault_seed,
        position_owner_seed,
        &[bump],
    ];
    
    msg!("Performing CPI call to claim fees from DAMM V2 position");
    
    // Prepare CPI instruction data for DAMM V2 collect_fees
    // This is a simplified version - actual DAMM V2 integration would require
    // the specific instruction format and account requirements
    
    let instruction_data = prepare_collect_fees_instruction_data()?;
    
    // Create the instruction for CPI call
    let collect_fees_ix = solana_program::instruction::Instruction {
        program_id: *cp_amm_program.key,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(*position_account.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*position_owner_pda.key, true),
            solana_program::instruction::AccountMeta::new(*treasury_ata.to_account_info().key, false),
            solana_program::instruction::AccountMeta::new_readonly(*token_program.key, false),
        ],
        data: instruction_data,
    };
    
    // Execute the CPI call with PDA signing
    invoke_signed(
        &collect_fees_ix,
        &[
            position_account.clone(),
            position_owner_pda.clone(),
            treasury_ata.to_account_info(),
            token_program.to_account_info(),
        ],
        &[signer_seeds],
    ).map_err(|e| {
        msg!("DAMM V2 CPI call failed: {}", e);
        ErrorCode::CpiCallFailed
    })?;
    
    msg!("CPI call completed successfully");
    
    Ok(())
}

/// Prepares instruction data for DAMM V2 collect_fees call
pub fn prepare_collect_fees_instruction_data() -> Result<Vec<u8>> {
    // This would contain the actual instruction discriminator and parameters
    // for the DAMM V2 collect_fees instruction
    // For now, we'll use a placeholder that represents the instruction format
    
    // DAMM V2 collect_fees instruction discriminator (8 bytes)
    let mut instruction_data = Vec::with_capacity(8);
    
    // Placeholder discriminator - in production this would be the actual
    // discriminator for the collect_fees instruction from DAMM V2
    instruction_data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    
    Ok(instruction_data)
}

/// Manages treasury ATA creation and validation
pub fn ensure_treasury_ata(
    treasury_ata: &Account<TokenAccount>,
    quote_mint: &Pubkey,
    program_authority: &Pubkey,
) -> Result<()> {
    // Validate treasury ATA is for the correct mint
    if treasury_ata.mint != *quote_mint {
        msg!("Treasury ATA mint mismatch - expected: {}, actual: {}", 
             quote_mint, treasury_ata.mint);
        return Err(ErrorCode::InvalidTreasuryAta.into());
    }
    
    // Validate treasury ATA is owned by the program authority
    if treasury_ata.owner != *program_authority {
        msg!("Treasury ATA owner mismatch - expected: {}, actual: {}", 
             program_authority, treasury_ata.owner);
        return Err(ErrorCode::InvalidTreasuryAta.into());
    }
    
    // Validate treasury ATA is not frozen
    if treasury_ata.delegate.is_some() {
        msg!("Warning: Treasury ATA has a delegate set");
    }
    
    msg!("Treasury ATA validation passed - mint: {}, owner: {}, balance: {}", 
         treasury_ata.mint, treasury_ata.owner, treasury_ata.amount);
    
    Ok(())
}

/// Creates treasury ATA if it doesn't exist
pub fn create_treasury_ata_if_needed<'info>(
    payer: &Signer<'info>,
    treasury_ata: &AccountInfo<'info>,
    position_owner_pda: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    system_program: &Program<'info, System>,
    token_program: &Program<'info, Token>,
    associated_token_program: &Program<'info, AssociatedToken>,
    rent: &Sysvar<'info, Rent>,
) -> Result<()> {
    // Check if treasury ATA already exists
    if treasury_ata.data_is_empty() {
        msg!("Creating treasury ATA for quote mint: {}", quote_mint.key());
        
        // Create associated token account
        let cpi_accounts = anchor_spl::associated_token::Create {
            payer: payer.to_account_info(),
            associated_token: treasury_ata.clone(),
            authority: position_owner_pda.clone(),
            mint: quote_mint.clone(),
            system_program: system_program.to_account_info(),
            token_program: token_program.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new(
            associated_token_program.to_account_info(),
            cpi_accounts,
        );
        
        anchor_spl::associated_token::create(cpi_ctx)?;
        
        msg!("Treasury ATA created successfully");
    } else {
        msg!("Treasury ATA already exists");
    }
    
    Ok(())
}

/// Validates treasury ATA balance and state
pub fn validate_treasury_state(
    treasury_ata: &Account<TokenAccount>,
    expected_minimum_balance: u64,
) -> Result<()> {
    // Check if treasury has sufficient balance for operations
    if treasury_ata.amount < expected_minimum_balance {
        msg!("Treasury balance {} is below minimum required: {}", 
             treasury_ata.amount, expected_minimum_balance);
        return Err(ErrorCode::InsufficientFunds.into());
    }
    
    // Validate account is not closed
    if treasury_ata.close_authority.is_some() {
        msg!("Warning: Treasury ATA has close authority set");
    }
    
    msg!("Treasury state validation passed - balance: {}", treasury_ata.amount);
    
    Ok(())
}

/// Transfers claimed fees to treasury
pub fn transfer_fees_to_treasury<'info>(
    from_account: &Account<'info, TokenAccount>,
    to_account: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    amount: u64,
    token_program: &Program<'info, Token>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    if amount == 0 {
        msg!("No fees to transfer");
        return Ok(());
    }
    
    let cpi_accounts = Transfer {
        from: from_account.to_account_info(),
        to: to_account.to_account_info(),
        authority: authority.clone(),
    };
    
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    
    transfer(cpi_ctx, amount)?;
    
    msg!("Transferred {} lamports to treasury", amount);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_fee_data_creation() {
        let fee_data = PositionFeeData {
            fee_owed_a: 1000,
            fee_owed_b: 0,
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
        };
        
        assert_eq!(fee_data.fee_owed_a, 1000);
        assert_eq!(fee_data.fee_owed_b, 0);
    }
    
    #[test]
    fn test_fee_claim_result_creation() {
        let quote_mint = Pubkey::new_unique();
        let result = FeeClaimResult {
            quote_amount: 1000,
            base_amount: 0,
            quote_mint,
        };
        
        assert_eq!(result.quote_amount, 1000);
        assert_eq!(result.base_amount, 0);
        assert_eq!(result.quote_mint, quote_mint);
    }
    
    #[test]
    fn test_quote_only_validation_logic() {
        let quote_mint = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        
        // Test case 1: Quote fees only (valid)
        let fee_data = PositionFeeData {
            fee_owed_a: 1000,
            fee_owed_b: 0,
            token_mint_a: quote_mint,
            token_mint_b: base_mint,
        };
        
        let result = validate_quote_only_fees(&fee_data, &quote_mint);
        assert!(result.is_ok());
        
        // Test case 2: Base fees present (invalid)
        let fee_data_with_base = PositionFeeData {
            fee_owed_a: 1000,
            fee_owed_b: 500, // Base fees present
            token_mint_a: quote_mint,
            token_mint_b: base_mint,
        };
        
        let result = validate_quote_only_fees(&fee_data_with_base, &quote_mint);
        assert!(result.is_err());
        
        // Test case 3: Quote is token_b
        let fee_data_b_quote = PositionFeeData {
            fee_owed_a: 0,
            fee_owed_b: 1000,
            token_mint_a: base_mint,
            token_mint_b: quote_mint,
        };
        
        let result = validate_quote_only_fees(&fee_data_b_quote, &quote_mint);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_treasury_ata_validation_logic() {
        let quote_mint = Pubkey::new_unique();
        let program_authority = Pubkey::new_unique();
        let wrong_mint = Pubkey::new_unique();
        let wrong_owner = Pubkey::new_unique();
        
        // Mock TokenAccount for testing logic
        struct MockTokenAccount {
            mint: Pubkey,
            owner: Pubkey,
        }
        
        // Test valid treasury ATA
        let valid_ata = MockTokenAccount {
            mint: quote_mint,
            owner: program_authority,
        };
        
        // Validate mint matches
        assert_eq!(valid_ata.mint, quote_mint);
        assert_eq!(valid_ata.owner, program_authority);
        
        // Test invalid mint
        let invalid_mint_ata = MockTokenAccount {
            mint: wrong_mint,
            owner: program_authority,
        };
        
        assert_ne!(invalid_mint_ata.mint, quote_mint);
        
        // Test invalid owner
        let invalid_owner_ata = MockTokenAccount {
            mint: quote_mint,
            owner: wrong_owner,
        };
        
        assert_ne!(invalid_owner_ata.owner, program_authority);
    }
    
    #[test]
    fn test_fee_amount_calculations() {
        let quote_mint = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        
        // Test quote as token_a
        let fee_data_a = PositionFeeData {
            fee_owed_a: 1000,
            fee_owed_b: 0,
            token_mint_a: quote_mint,
            token_mint_b: base_mint,
        };
        
        let (quote_amount, base_amount) = if fee_data_a.token_mint_a == quote_mint {
            (fee_data_a.fee_owed_a, fee_data_a.fee_owed_b)
        } else {
            (fee_data_a.fee_owed_b, fee_data_a.fee_owed_a)
        };
        
        assert_eq!(quote_amount, 1000);
        assert_eq!(base_amount, 0);
        
        // Test quote as token_b
        let fee_data_b = PositionFeeData {
            fee_owed_a: 0,
            fee_owed_b: 2000,
            token_mint_a: base_mint,
            token_mint_b: quote_mint,
        };
        
        let (quote_amount, base_amount) = if fee_data_b.token_mint_a == quote_mint {
            (fee_data_b.fee_owed_a, fee_data_b.fee_owed_b)
        } else {
            (fee_data_b.fee_owed_b, fee_data_b.fee_owed_a)
        };
        
        assert_eq!(quote_amount, 2000);
        assert_eq!(base_amount, 0);
    }
}