use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::ErrorCode;

#[account]
pub struct PolicyConfig {
    /// The vault account used as seed for PDAs
    pub vault: Pubkey,
    /// The quote mint for fee collection
    pub quote_mint: Pubkey,
    /// Creator wallet to receive remaining fees
    pub creator_wallet: Pubkey,
    /// Investor fee share in basis points (0-10000)
    pub investor_fee_share_bps: u16,
    /// Optional daily cap in lamports
    pub daily_cap_lamports: Option<u64>,
    /// Minimum payout threshold in lamports
    pub min_payout_lamports: u64,
    /// Total investor allocation at TGE (Y0)
    pub y0_total_allocation: u64,
    /// PDA bump
    pub bump: u8,
}

impl PolicyConfig {
    /// Calculate space needed for account
    /// 32 (vault) + 32 (quote_mint) + 32 (creator_wallet) + 2 (investor_fee_share_bps) 
    /// + 9 (daily_cap_lamports Option<u64>) + 8 (min_payout_lamports) + 8 (y0_total_allocation) + 1 (bump)
    pub const INIT_SPACE: usize = 32 + 32 + 32 + 2 + 9 + 8 + 8 + 1;

    /// Validate policy configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Validate investor fee share is within valid range
        require!(
            self.investor_fee_share_bps <= MAX_BASIS_POINTS,
            ErrorCode::InvalidFeeShareBasisPoints
        );

        // Validate minimum payout is reasonable (not zero)
        require!(
            self.min_payout_lamports > 0,
            ErrorCode::InvalidMinPayoutThreshold
        );

        // Validate Y0 total allocation is not zero
        require!(
            self.y0_total_allocation > 0,
            ErrorCode::InvalidTotalAllocation
        );

        // Validate daily cap if set
        if let Some(daily_cap) = self.daily_cap_lamports {
            require!(
                daily_cap > 0,
                ErrorCode::InvalidDailyCap
            );
        }

        Ok(())
    }

    /// Initialize a new policy configuration
    pub fn initialize(
        &mut self,
        vault: Pubkey,
        quote_mint: Pubkey,
        creator_wallet: Pubkey,
        investor_fee_share_bps: u16,
        daily_cap_lamports: Option<u64>,
        min_payout_lamports: u64,
        y0_total_allocation: u64,
        bump: u8,
    ) -> Result<()> {
        self.vault = vault;
        self.quote_mint = quote_mint;
        self.creator_wallet = creator_wallet;
        self.investor_fee_share_bps = investor_fee_share_bps;
        self.daily_cap_lamports = daily_cap_lamports;
        self.min_payout_lamports = min_payout_lamports;
        self.y0_total_allocation = y0_total_allocation;
        self.bump = bump;

        // Validate the configuration
        self.validate()?;

        Ok(())
    }

    /// Get the PDA seeds for this policy config (for signing)
    pub fn get_signer_seeds(&self) -> [&[u8]; 3] {
        [POLICY_SEED, self.vault.as_ref(), std::slice::from_ref(&self.bump)]
    }
}