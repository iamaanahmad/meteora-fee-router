use anchor_lang::prelude::*;

pub mod error;
pub mod constants;
pub mod utils;
pub mod state;
pub mod instructions;
pub mod security_audit;

pub use instructions::*;

#[cfg(test)]
mod events_tests;

#[cfg(test)]
mod events_simple_tests;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod meteora_fee_router {
    use super::*;

    /// Initialize an honorary DAMM V2 LP position for quote-only fee accrual
    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>,
        params: InitializeHonoraryPositionParams,
    ) -> Result<()> {
        instructions::initialize_honorary_position::handler(ctx, params)
    }

    /// Distribute fees to investors and creator via 24-hour crank system
    pub fn distribute_fees<'info>(
        ctx: Context<'_, '_, '_, 'info, DistributeFees<'info>>,
        params: DistributeFeesParams,
    ) -> Result<()> {
        instructions::distribute_fees::handler(ctx, params)
    }
}

/// Events emitted by the program
#[event]
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub quote_mint: Pubkey,
    pub creator_wallet: Pubkey,
    pub investor_fee_share_bps: u16,
    pub daily_cap_lamports: Option<u64>,
    pub min_payout_lamports: u64,
    pub y0_total_allocation: u64,
    pub position_owner_pda: Pubkey,
    pub policy_config: Pubkey,
    pub distribution_progress: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub claimed_amount: u64,
    pub base_amount: u64, // Should always be 0 for quote-only
    pub quote_mint: Pubkey,
    pub honorary_position: Pubkey,
    pub treasury_ata: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub page_start: u32,
    pub page_end: u32,
    pub total_distributed: u64,
    pub processed_count: u32,
    pub dust_carried_forward: u64,
    pub cumulative_day_distributed: u64,
    pub timestamp: i64,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub creator_payout: u64,
    pub creator_wallet: Pubkey,
    pub total_day_distributed: u64,
    pub total_investors_processed: u32,
    pub final_dust_amount: u64,
    pub timestamp: i64,
}