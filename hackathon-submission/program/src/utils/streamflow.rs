use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use crate::error::ErrorCode;
use std::collections::{HashMap, HashSet};

/// Streamflow Stream account structure
/// This mirrors the essential fields from Streamflow's Stream account
#[derive(Debug, Clone)]
pub struct StreamflowStream {
    pub recipient: Pubkey,
    pub sender: Pubkey,
    pub mint: Pubkey,
    pub deposited_amount: u64,
    pub withdrawn_amount: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub cliff_time: i64,
    pub cancelable_by_sender: bool,
    pub cancelable_by_recipient: bool,
    pub automatic_withdrawal: bool,
    pub transferable_by_sender: bool,
    pub transferable_by_recipient: bool,
    pub can_topup: bool,
    pub stream_name: [u8; 64],
    pub withdrawn_tokens_recipient: u64,
    pub withdrawn_tokens_sender: u64,
    pub last_withdrawn_at: i64,
    pub closed_at: Option<i64>,
}

impl StreamflowStream {
    /// Deserialize a Streamflow Stream account from raw account data
    pub fn try_from_account_data(data: &[u8]) -> Result<Self> {
        // Skip the 8-byte discriminator
        if data.len() < 8 {
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        let data = &data[8..];
        
        // Basic validation of account data length
        if data.len() < 200 { // Minimum expected size for Stream account
            return Err(ErrorCode::StreamflowValidationFailed.into());
        }
        
        // Parse the account data (simplified parsing for hackathon)
        // In production, this would use proper borsh deserialization
        let mut offset = 0;
        
        let recipient = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        offset += 32;
        
        let sender = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        offset += 32;
        
        let mint = Pubkey::try_from(&data[offset..offset + 32])
            .map_err(|_| ErrorCode::StreamflowValidationFailed)?;
        offset += 32;
        
        let deposited_amount = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let withdrawn_amount = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let start_time = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let end_time = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let cliff_time = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        // Parse boolean flags (1 byte each)
        let cancelable_by_sender = data[offset] != 0;
        offset += 1;
        let cancelable_by_recipient = data[offset] != 0;
        offset += 1;
        let automatic_withdrawal = data[offset] != 0;
        offset += 1;
        let transferable_by_sender = data[offset] != 0;
        offset += 1;
        let transferable_by_recipient = data[offset] != 0;
        offset += 1;
        let can_topup = data[offset] != 0;
        offset += 1;
        
        // Parse stream name (64 bytes)
        let mut stream_name = [0u8; 64];
        stream_name.copy_from_slice(&data[offset..offset + 64]);
        offset += 64;
        
        let withdrawn_tokens_recipient = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let withdrawn_tokens_sender = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        let last_withdrawn_at = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        offset += 8;
        
        // Parse optional closed_at timestamp
        let closed_at_raw = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|_| ErrorCode::StreamflowValidationFailed)?
        );
        let closed_at = if closed_at_raw == 0 { None } else { Some(closed_at_raw) };
        
        Ok(StreamflowStream {
            recipient,
            sender,
            mint,
            deposited_amount,
            withdrawn_amount,
            start_time,
            end_time,
            cliff_time,
            cancelable_by_sender,
            cancelable_by_recipient,
            automatic_withdrawal,
            transferable_by_sender,
            transferable_by_recipient,
            can_topup,
            stream_name,
            withdrawn_tokens_recipient,
            withdrawn_tokens_sender,
            last_withdrawn_at,
            closed_at,
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
    pub fn calculate_locked_amount(
        stream: &StreamflowStream,
        current_timestamp: i64,
    ) -> Result<u64> {
        // If stream is closed, no tokens are locked
        if stream.closed_at.is_some() {
            return Ok(0);
        }
        
        // If we haven't reached the start time, all tokens are locked
        if current_timestamp < stream.start_time {
            return Ok(stream.deposited_amount.saturating_sub(stream.withdrawn_amount));
        }
        
        // If we haven't reached the cliff, all tokens are locked
        if current_timestamp < stream.cliff_time {
            return Ok(stream.deposited_amount.saturating_sub(stream.withdrawn_amount));
        }
        
        // If we've passed the end time, no tokens are locked
        if current_timestamp >= stream.end_time {
            return Ok(0);
        }
        
        // Calculate vested amount based on linear vesting
        let vesting_duration = stream.end_time.saturating_sub(stream.start_time);
        if vesting_duration == 0 {
            return Ok(0);
        }
        
        let elapsed_since_start = current_timestamp.saturating_sub(stream.start_time);
        
        // Use 128-bit arithmetic to prevent overflow
        let vested_amount = (stream.deposited_amount as u128)
            .saturating_mul(elapsed_since_start as u128)
            .saturating_div(vesting_duration as u128) as u64;
        
        let available_amount = stream.deposited_amount.saturating_sub(stream.withdrawn_amount);
        let locked_amount = available_amount.saturating_sub(vested_amount);
        
        Ok(locked_amount)
    }
    
    /// Validate a Streamflow account and extract stream data
    pub fn validate_and_parse_stream(
        stream_account: &AccountInfo,
        expected_mint: &Pubkey,
    ) -> Result<StreamflowStream> {
        // Validate account ownership (Streamflow program)
        // Note: In production, this would check against the actual Streamflow program ID
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
        if stream.closed_at.is_some() {
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
#[cfg(test)]
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
        StreamflowStream {
            recipient,
            sender: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            deposited_amount,
            withdrawn_amount: 0,
            start_time,
            end_time,
            cliff_time,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            withdrawn_tokens_recipient: 0,
            withdrawn_tokens_sender: 0,
            last_withdrawn_at: 0,
            closed_at: None,
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
    fn test_calculate_locked_amount_linear_vesting() {
        let stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens
            1000,    // start time
            2000,    // end time (1000 seconds duration)
            1000,    // cliff time (same as start)
        );
        
        // At 50% through vesting period
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 500000); // 50% still locked
        
        // At 75% through vesting period
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1750).unwrap();
        assert_eq!(locked, 250000); // 25% still locked
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
    fn test_calculate_locked_amount_with_withdrawals() {
        let mut stream = create_mock_stream(
            Pubkey::new_unique(),
            1000000, // 1M tokens deposited
            1000,    // start time
            2000,    // end time
            1000,    // cliff time
        );
        stream.withdrawn_amount = 200000; // 200k already withdrawn
        
        // At 50% through vesting period
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        // Available: 1M - 200k = 800k
        // Vested: 50% of 1M = 500k
        // Locked: 800k - 500k = 300k
        assert_eq!(locked, 300000);
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
        stream.closed_at = Some(1500);
        
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1600).unwrap();
        assert_eq!(locked, 0); // No tokens locked in closed stream
    }
}