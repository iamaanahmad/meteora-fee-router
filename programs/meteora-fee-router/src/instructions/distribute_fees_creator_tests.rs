use anchor_lang::prelude::*;
use crate::{
    instructions::distribute_fees::{DistributeFees, DistributeFeesParams},
    state::{PolicyConfig, DistributionProgress, DistributionTimingState},
    utils::{
        creator_distribution::CreatorDistribution,
        math::calculate_distribution,
    },
    error::ErrorCode,
    CreatorPayoutDayClosed,
};

/// Integration tests for creator remainder distribution in distribute_fees instruction
#[cfg(test)]
mod creator_payout_integration_tests {
    use super::*;

    fn create_test_policy_config() -> PolicyConfig {
        PolicyConfig {
            vault: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            creator_wallet: Pubkey::new_unique(),
            investor_fee_share_bps: 7500, // 75%
            daily_cap_lamports: Some(2_000_000),
            min_payout_lamports: 1000,
            y0_total_allocation: 10_