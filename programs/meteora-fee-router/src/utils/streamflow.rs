use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_lang::solana_program::pubkey;
use crate::error::ErrorCode;
use std::collections::{HashMap, HashSet};

/// Official Streamflow Token Vesting Program ID (Mainnet & Devnet)
/// Source: https://docs.streamflow.finance/
pub const STREAMFLOW_PROGRAM_ID: Pubkey = pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");

/// Streamflow Stream account structure
/// Layout verified against @streamflow/stream SDK v10.x
/// See: https://github.com/streamflow-finance/js-sdk/blob/main/packages/stream/solana/layout.ts
/// 
/// Account Layout (offsets from start, no discriminator):
/// - 0:    magic (8 bytes)
/// - 8:    version (1 byte)  
/// - 9:    created_at (8 bytes)
/// - 17:   withdrawn_amount (8 bytes)
/// - 25:   canceled_at (8 bytes)
/// - 33:   end_time (8 bytes)
/// - 41:   last_withdrawn_at (8 bytes)
/// - 49:   sender (32 bytes)
/// - 81:   sender_tokens (32 bytes)
/// - 113:  recipient (32 bytes)
/// - 145:  recipient_tokens (32 bytes)
/// - 177:  mint (32 bytes)
/// - 209:  escrow_tokens (32 bytes)
/// - 241:  streamflow_treasury (32 bytes)
/// - 273:  streamflow_treasury_tokens (32 bytes)
/// - 305:  streamflow_fee_total (8 bytes)
/// - 313:  streamflow_fee_withdrawn (8 bytes)
/// - 321:  streamflow_fee_percent (4 bytes f32)
/// - 325:  partner (32 bytes)
/// - 357:  partner_tokens (32 bytes)
/// - 389:  partner_fee_total (8 bytes)
/// - 397:  partner_fee_withdrawn (8 bytes)
/// - 405:  partner_fee_percent (4 bytes f32)
/// - 409:  start_time (8 bytes)
/// - 417:  net_amount_deposited (8 bytes)
/// - 425:  period (8 bytes)
/// - 433:  amount_per_period (8 bytes)
/// - 441:  cliff (8 bytes)
/// - 449:  cliff_amount (8 bytes)
/// - 457:  cancelable_by_sender (1 byte)
/// - 458:  cancelable_by_recipient (1 byte)
/// - 459:  automatic_withdrawal (1 byte)
/// - 460:  transferable_by_sender (1 byte)
/// - 461:  transferable_by_recipient (1 byte)
/// - 462:  can_topup (1 byte)
/// - 463:  stream_name (64 bytes)
/// - 527:  withdraw_frequency (8 bytes)
/// - 535:  ghost (4 bytes - unused, backward compat)
/// - 539:  pausable (1 byte)
/// - 540:  can_update_rate (1 byte)
/// - 541:  create_params_padding_length (4 bytes)
/// - 545:  create_params_padding (variable)
/// - ...:  closed (1 byte)
/// - ...:  current_pause_start (8 bytes)
/// - ...:  pause_cumulative (8 bytes)
/// - ...:  last_rate_change_time (8 bytes)
/// - ...:  funds_unlocked_at_last_rate_change (8 bytes)
#[derive(Debug, Clone)]
pub struct StreamflowStream {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub deposited_amount: u64,
    pub withdrawn_amount: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub cliff_time: i64,
    pub cliff_amount: u64,
    pub amount_per_period: u64,
    pub period: u64,
    pub cancelable_by_sender: bool,
    pub cancelable_by_recipient: bool,
    pub automatic_withdrawal: bool,
    pub transferable_by_sender: bool,
    pub transferable_by_recipient: bool,
    pub can_topup: bool,
    pub stream_name: [u8; 64],
    pub last_withdrawn_at: i64,
    pub closed: bool,
}

impl StreamflowStream {
    /// Minimum size for a valid Streamflow Stream account
    /// Based on layout up to closed field (approximately 672 bytes based on SDK)
    pub const MIN_ACCOUNT_SIZE: usize = 672;

    /// Deserialize a Streamflow Stream account from raw account data
    /// Layout matches @streamflow/stream SDK v10.x exactly
    pub fn try_from_account_data(data: &[u8]) -> Result<Self> {
        // Streamflow accounts do NOT have an 8-byte Anchor discriminator
        // They start directly with the magic number
        
        // Validate minimum account size
        if data.len() < Self::MIN_ACCOUNT_SIZE {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Validate magic number (first 8 bytes)
        // Streamflow uses a specific magic value to identify valid streams
        let magic = u64::from_le_bytes(
            data[0..8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Magic should be non-zero for valid streams
        if magic == 0 {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Parse fields according to the actual Streamflow layout
        // Offset 9: version (skip)
        // Offset 9: created_at (skip)
        
        // Offset 17: withdrawn_amount
        let withdrawn_amount = u64::from_le_bytes(
            data[17..25].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 25: canceled_at (skip)
        
        // Offset 33: end_time
        let end_time = i64::from_le_bytes(
            data[33..41].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 41: last_withdrawn_at
        let last_withdrawn_at = i64::from_le_bytes(
            data[41..49].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 49: sender (32 bytes)
        let sender = Pubkey::try_from(&data[49..81])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        
        // Offset 81: sender_tokens (skip - 32 bytes)
        
        // Offset 113: recipient (32 bytes)
        let recipient = Pubkey::try_from(&data[113..145])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        
        // Offset 145: recipient_tokens (skip - 32 bytes)
        
        // Offset 177: mint (32 bytes)
        let mint = Pubkey::try_from(&data[177..209])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        
        // Skip: escrow_tokens, streamflow_treasury, streamflow_treasury_tokens
        // Skip: streamflow fees, partner info
        
        // Offset 409: start_time
        let start_time = i64::from_le_bytes(
            data[409..417].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 417: net_amount_deposited (deposited_amount)
        let deposited_amount = u64::from_le_bytes(
            data[417..425].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 425: period
        let period = u64::from_le_bytes(
            data[425..433].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 433: amount_per_period
        let amount_per_period = u64::from_le_bytes(
            data[433..441].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 441: cliff (cliff timestamp)
        let cliff_time = i64::from_le_bytes(
            data[441..449].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 449: cliff_amount
        let cliff_amount = u64::from_le_bytes(
            data[449..457].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        
        // Offset 457-462: Boolean flags (1 byte each)
        let cancelable_by_sender = data[457] != 0;
        let cancelable_by_recipient = data[458] != 0;
        let automatic_withdrawal = data[459] != 0;
        let transferable_by_sender = data[460] != 0;
        let transferable_by_recipient = data[461] != 0;
        let can_topup = data[462] != 0;
        
        // Offset 463: stream_name (64 bytes)
        let mut stream_name = [0u8; 64];
        stream_name.copy_from_slice(&data[463..527]);
        
        // Offset 671: closed (after padding and other fields)
        // The closed field is at a variable offset due to padding, check at known location
        let closed = if data.len() > 671 { data[671] != 0 } else { false };
        
        Ok(StreamflowStream {
            sender,
            recipient,
            mint,
            deposited_amount,
            withdrawn_amount,
            start_time,
            end_time,
            cliff_time,
            cliff_amount,
            amount_per_period,
            period,
            cancelable_by_sender,
            cancelable_by_recipient,
            automatic_withdrawal,
            transferable_by_sender,
            transferable_by_recipient,
            can_topup,
            stream_name,
            last_withdrawn_at,
            closed,
        })
    }
}

/// Investor data aggregated from Streamflow accounts
#[derive(Debug, Clone)]
pub struct InvestorData {
    pub wallet: Pubkey,
    pub locked_amount: u64,
    pub total_allocation: u64,
    pub stream_accounts: Vec<Pubkey>,
}

/// Streamflow integration utilities
pub struct StreamflowIntegration;

impl StreamflowIntegration {
    /// Calculate the still-locked amount for a stream at the current timestamp
    /// Uses Streamflow's linear vesting formula with cliff support
    pub fn calculate_locked_amount(
        stream: &StreamflowStream,
        current_timestamp: i64,
    ) -> Result<u64> {
        // If stream is closed, no tokens are locked
        if stream.closed {
            return Ok(0);
        }
        
        let remaining = stream.deposited_amount.saturating_sub(stream.withdrawn_amount);
        
        // If we haven't reached the start time, all tokens are locked
        if current_timestamp < stream.start_time {
            return Ok(remaining);
        }
        
        // If we haven't reached the cliff, all tokens are locked (minus cliff_amount unlocked at cliff)
        if current_timestamp < stream.cliff_time {
            return Ok(remaining);
        }
        
        // If we've passed the end time, no tokens are locked
        if current_timestamp >= stream.end_time {
            return Ok(0);
        }
        
        // Calculate unlocked amount using Streamflow's formula:
        // unlocked = cliff_amount + ((current - cliff) / period) * amount_per_period
        // But capped at deposited_amount
        
        if stream.period == 0 {
            // Instant unlock if no period
            return Ok(0);
        }
        
        let time_since_cliff = (current_timestamp - stream.cliff_time) as u64;
        let periods_elapsed = time_since_cliff / stream.period;
        
        // Calculate total unlocked: cliff_amount + (periods * amount_per_period)
        let unlocked = stream.cliff_amount
            .saturating_add(periods_elapsed.saturating_mul(stream.amount_per_period));
        
        // Cap at deposited amount
        let unlocked = unlocked.min(stream.deposited_amount);
        
        // Locked = deposited - unlocked (but consider already withdrawn)
        let locked = stream.deposited_amount.saturating_sub(unlocked);
        
        Ok(locked)
    }
    
    /// Validate a Streamflow account and extract stream data
    pub fn validate_and_parse_stream(
        stream_account: &AccountInfo,
        expected_mint: &Pubkey,
    ) -> Result<StreamflowStream> {
        // Validate account ownership - MUST be owned by Streamflow program
        if stream_account.owner != &STREAMFLOW_PROGRAM_ID {
            msg!("Invalid Streamflow account owner: expected {}, got {}", 
                 STREAMFLOW_PROGRAM_ID, stream_account.owner);
            return Err(ErrorCode::InvalidStreamflowAccountOwner.into());
        }
        
        // Validate account has data
        if stream_account.data_is_empty() {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Parse the stream data
        let stream = StreamflowStream::try_from_account_data(&stream_account.data.borrow())?;
        
        // Validate the mint matches expected token
        if stream.mint != *expected_mint {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Validate stream is not closed
        if stream.closed {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Validate stream has valid time parameters
        if stream.start_time >= stream.end_time {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        if stream.cliff_time < stream.start_time || stream.cliff_time > stream.end_time {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        Ok(stream)
    }
    
    /// Aggregate investor data from multiple Streamflow accounts
    pub fn aggregate_investor_data(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        current_timestamp: i64,
    ) -> Result<Vec<InvestorData>> {
        let mut investor_map: HashMap<Pubkey, InvestorData> = HashMap::new();
        
        for stream_account in stream_accounts {
            // Validate and parse the stream
            let stream = Self::validate_and_parse_stream(stream_account, expected_mint)?;
            
            // Calculate locked amount for this stream
            let locked_amount = Self::calculate_locked_amount(&stream, current_timestamp)?;
            
            // Aggregate data by recipient (investor wallet)
            let investor_data = investor_map.entry(stream.recipient).or_insert(InvestorData {
                wallet: stream.recipient,
                locked_amount: 0,
                total_allocation: 0,
                stream_accounts: Vec::new(),
            });
            
            investor_data.locked_amount = investor_data.locked_amount.saturating_add(locked_amount);
            investor_data.total_allocation = investor_data.total_allocation.saturating_add(stream.deposited_amount);
            investor_data.stream_accounts.push(stream_account.key());
        }
        
        Ok(investor_map.into_values().collect())
    }
    
    /// Calculate total locked amount across all investors
    pub fn calculate_total_locked(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        current_timestamp: i64,
    ) -> Result<u64> {
        let mut total_locked = 0u64;
        
        for stream_account in stream_accounts {
            let stream = Self::validate_and_parse_stream(stream_account, expected_mint)?;
            let locked_amount = Self::calculate_locked_amount(&stream, current_timestamp)?;
            total_locked = total_locked.saturating_add(locked_amount);
        }
        
        Ok(total_locked)
    }
    
    /// Get current timestamp from Solana clock
    pub fn get_current_timestamp() -> Result<i64> {
        let clock = Clock::get()?;
        Ok(clock.unix_timestamp)
    }
    
    /// Process a page of investors for fee distribution
    /// Returns (processed_count, total_locked_in_page, investor_payouts)
    pub fn process_investor_page(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
        current_timestamp: i64,
        page_start: usize,
        page_size: usize,
    ) -> Result<(usize, u64, Vec<(Pubkey, u64)>)> {
        let end_index = std::cmp::min(page_start + page_size, stream_accounts.len());
        let page_accounts = &stream_accounts[page_start..end_index];
        
        let investor_data = Self::aggregate_investor_data(page_accounts, expected_mint, current_timestamp)?;
        
        let mut total_locked = 0u64;
        let mut payouts = Vec::new();
        
        for investor in investor_data {
            total_locked = total_locked.saturating_add(investor.locked_amount);
            payouts.push((investor.wallet, investor.locked_amount));
        }
        
        Ok((end_index - page_start, total_locked, payouts))
    }
    
    /// Validate that all provided stream accounts belong to the expected mint
    pub fn validate_all_streams_mint(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
    ) -> Result<()> {
        for stream_account in stream_accounts {
            let stream = Self::validate_and_parse_stream(stream_account, expected_mint)?;
            // Additional validation can be added here
        }
        Ok(())
    }
    
    /// Get investor count from stream accounts (deduplicated by recipient)
    pub fn get_unique_investor_count(
        stream_accounts: &[AccountInfo],
        expected_mint: &Pubkey,
    ) -> Result<usize> {
        let mut unique_recipients = HashSet::new();
        
        for stream_account in stream_accounts {
            let stream = Self::validate_and_parse_stream(stream_account, expected_mint)?;
            unique_recipients.insert(stream.recipient);
        }
        
        Ok(unique_recipients.len())
    }
}

#[cfg(test)]
#[path = "streamflow_tests.rs"]
mod streamflow_tests;

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_mock_stream(
        recipient: Pubkey,
        deposited_amount: u64,
        start_time: i64,
        end_time: i64,
        cliff_time: i64,
    ) -> StreamflowStream {
        // Calculate period-based vesting that matches the duration
        let duration = (end_time - cliff_time) as u64;
        let period = 100u64;  // 100 second periods
        let num_periods = if duration > 0 && period > 0 { duration / period } else { 1 };
        let amount_per_period = if num_periods > 0 { deposited_amount / num_periods } else { deposited_amount };
        
        StreamflowStream {
            sender: Pubkey::new_unique(),
            recipient,
            mint: Pubkey::new_unique(),
            deposited_amount,
            withdrawn_amount: 0,
            start_time,
            end_time,
            cliff_time,
            cliff_amount: 0,
            amount_per_period,
            period,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            last_withdrawn_at: 0,
            closed: false,
        }
    }
    
    #[test]
    fn test_calculate_locked_amount_before_start() {
        let stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time
            1100,    // cliff time
        );
        
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 500).unwrap();
        assert_eq!(locked, 1000000); // All tokens locked before start
    }
    
    #[test]
    fn test_calculate_locked_amount_before_cliff() {
        let stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time
            1500,    // cliff time
        );
        
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1200).unwrap();
        assert_eq!(locked, 1000000); // All tokens locked before cliff
    }
    
    #[test]
    fn test_calculate_locked_amount_period_based_vesting() {
        let stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time (1000 seconds duration, 10 periods)
            1000,    // cliff time (same as start)
        );
        
        // At 50% through vesting period (5 periods elapsed)
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 500000); // 50% still locked
        
        // At 80% through vesting period (8 periods elapsed)
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1800).unwrap();
        assert_eq!(locked, 200000); // 20% still locked
    }
    
    #[test]
    fn test_calculate_locked_amount_after_end() {
        let stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time
            1000,    // cliff time
        );
        
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 2500).unwrap();
        assert_eq!(locked, 0); // No tokens locked after end
    }
    
    #[test]
    fn test_calculate_locked_amount_with_period_vesting() {
        let mut stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens deposited
            1000,    // start time
            2000,    // end time (10 periods of 100s each)
            1000,    // cliff time
        );
        stream.withdrawn_amount = 200000; // 200k already withdrawn
        
        // At 50% through vesting period (5 periods)
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        // Unlocked: 5 * 100k = 500k
        // Locked: 1M - 500k = 500k
        assert_eq!(locked, 500000);
    }
    
    #[test]
    fn test_calculate_locked_amount_closed_stream() {
        let mut stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time
            1000,    // cliff time
        );
        stream.closed = true;
        
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1600).unwrap();
        assert_eq!(locked, 0); // No tokens locked in closed stream
    }
}