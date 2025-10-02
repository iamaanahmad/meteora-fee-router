# Fee Claiming Implementation

## Overview

This document details the implementation of DAMM V2 fee claiming functionality for the Meteora Fee Router, ensuring quote-only fee collection with comprehensive validation.

## Core Components

### 1. Fee Claiming Structure

```rust
#[derive(Debug, Clone)]
pub struct FeeClaimResult {
    pub quote_amount: u64,
    pub base_amount: u64,
    pub success: bool,
}

#[derive(Debug)]
pub struct PositionFeeData {
    pub quote_fees: u64,
    pub base_fees: u64,
    pub position_mint: Pubkey,
}
```

### 2. Quote-Only Validation

The core validation ensures only quote fees are processed:

```rust
pub fn validate_quote_only_fees(
    claimed_fees: &FeeClaimResult,
    quote_mint: &Pubkey,
    position_data: &PositionFeeData,
) -> Result<()> {
    // Strict validation: no base fees allowed
    if claimed_fees.base_amount > 0 {
        msg!("Base fees detected: {}, rejecting transaction", claimed_fees.base_amount);
        return Err(ErrorCode::BaseFeeDetected.into());
    }

    // Ensure we have quote fees to process
    if claimed_fees.quote_amount == 0 {
        msg!("No quote fees claimed");
        return Err(ErrorCode::NoQuoteFeesToClaim.into());
    }

    // Validate quote mint consistency
    if &position_data.position_mint != quote_mint {
        return Err(ErrorCode::InvalidQuoteMint.into());
    }

    Ok(())
}
```

### 3. DAMM V2 Integration

```rust
pub fn claim_fees_from_position<'info>(
    position_account: &AccountInfo<'info>,
    position_owner_pda: &AccountInfo<'info>,
    treasury_quote_ata: &AccountInfo<'info>,
    cp_amm_program: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<FeeClaimResult> {
    
    // Prepare CPI instruction for fee claiming
    let claim_instruction = create_claim_fee_instruction(
        position_account.key,
        position_owner_pda.key,
        treasury_quote_ata.key,
    )?;

    // Execute CPI call to DAMM V2
    let accounts = vec![
        position_account.clone(),
        position_owner_pda.clone(),
        treasury_quote_ata.clone(),
        token_program.clone(),
    ];

    invoke_signed(
        &claim_instruction,
        &accounts,
        signer_seeds,
    ).map_err(|e| {
        msg!("Fee claiming CPI failed: {:?}", e);
        ErrorCode::FeeClaimingFailed
    })?;

    // Extract and validate claimed amounts
    let fee_result = extract_claimed_amounts(treasury_quote_ata)?;
    
    Ok(fee_result)
}
```

### 4. Treasury Management

```rust
pub fn validate_treasury_ata<'info>(
    treasury_ata: &AccountInfo<'info>,
    quote_mint: &Pubkey,
    program_authority: &Pubkey,
) -> Result<()> {
    // Validate account ownership
    if treasury_ata.owner != &spl_token::ID {
        return Err(ErrorCode::InvalidTreasuryAccount.into());
    }

    // Parse token account data
    let token_account = spl_token::state::Account::unpack(&treasury_ata.data.borrow())?;
    
    // Validate mint consistency
    if &token_account.mint != quote_mint {
        return Err(ErrorCode::InvalidTreasuryMint.into());
    }

    // Validate authority
    if &token_account.owner != program_authority {
        return Err(ErrorCode::InvalidTreasuryAuthority.into());
    }

    Ok(())
}
```

## Key Features

### 1. Quote-Only Enforcement
- Strict validation prevents any base fee processing
- Deterministic failure if base fees are detected
- Comprehensive mint validation throughout

### 2. CPI Integration
- Proper cross-program invocation to DAMM V2
- Secure signer seed management
- Error handling and recovery

### 3. Treasury Security
- Program-owned treasury accounts
- Proper ATA validation and management
- Secure fee accumulation

### 4. Error Handling
- Comprehensive error types for all failure modes
- Detailed logging for debugging
- Graceful failure with proper cleanup

## Implementation Details

### 1. Fee Claiming Flow

```rust
pub fn execute_fee_claiming<'info>(
    ctx: &Context<'_, '_, '_, 'info, DistributeFees<'info>>,
    signer_seeds: &[&[&[u8]]],
) -> Result<u64> {
    // Validate preconditions
    validate_claim_preconditions(
        &ctx.accounts.honorary_position,
        &ctx.accounts.treasury_quote_ata,
        &ctx.accounts.policy_config.quote_mint,
    )?;

    // Claim fees via CPI
    let fee_result = claim_fees_from_position(
        &ctx.accounts.honorary_position,
        &ctx.accounts.position_owner_pda,
        &ctx.accounts.treasury_quote_ata,
        &ctx.accounts.cp_amm_program,
        &ctx.accounts.token_program,
        signer_seeds,
    )?;

    // Validate quote-only requirement
    validate_quote_only_fees(
        &fee_result,
        &ctx.accounts.policy_config.quote_mint,
        &extract_position_fee_data(&ctx.accounts.honorary_position)?,
    )?;

    // Emit event for monitoring
    emit!(QuoteFeesClaimed {
        vault: ctx.accounts.policy_config.vault,
        quote_amount: fee_result.quote_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(fee_result.quote_amount)
}
```

### 2. Error Recovery

```rust
pub fn handle_fee_claiming_error(error: &ProgramError) -> Result<()> {
    match error {
        ProgramError::Custom(code) if *code == ErrorCode::BaseFeeDetected as u32 => {
            msg!("Base fees detected - transaction rejected for safety");
            Err(ErrorCode::BaseFeeDetected.into())
        },
        ProgramError::Custom(code) if *code == ErrorCode::NoQuoteFeesToClaim as u32 => {
            msg!("No quote fees available - skipping distribution");
            Ok(()) // This is acceptable
        },
        _ => {
            msg!("Unexpected fee claiming error: {:?}", error);
            Err(ErrorCode::FeeClaimingFailed.into())
        }
    }
}
```

## Testing Coverage

### Unit Tests
- `test_validate_quote_only_fees_*`: Quote-only validation
- `test_treasury_ata_validation_*`: Treasury management
- `test_fee_claiming_error_scenarios_*`: Error handling

### Integration Tests
- End-to-end fee claiming with DAMM V2
- Quote-only enforcement validation
- Treasury security testing

## Security Considerations

### 1. Quote-Only Guarantee
- Multiple validation layers prevent base fee processing
- Fail-safe design with deterministic rejection
- Comprehensive logging for audit trails

### 2. CPI Security
- Proper signer seed management
- Account validation before CPI calls
- Error handling prevents state corruption

### 3. Treasury Protection
- Program-owned accounts only
- Proper mint validation
- Authority verification

This implementation ensures secure, reliable fee claiming with strict quote-only enforcement for the Meteora Fee Router.