#[cfg(test)]
use anchor_lang::prelude::*;
use crate::utils::streamflow::StreamflowStream;

#[cfg(test)]
pub struct MockStreamflowBuilder {
    recipient: Pubkey,
    sender: Pubkey,
    mint: Pubkey,
    deposited_amount: u64,
    withdrawn_amount: u64,
    start_time: i64,
    end_time: i64,
    cliff_time: i64,
    closed_at: Option<i64>,
}

#[cfg(test)]
impl MockStreamflowBuilder {
    pub fn new(recipient: Pubkey, mint: Pubkey) -> Self {
        Self {
            recipient,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount: 1_000_000,
            withdrawn_amount: 0,
            start_time: 1000,
            end_time: 2000,
            cliff_time: 1000,
            closed_at: None,
        }
    }

    pub fn deposited_amount(mut self, amount: u64) -> Self {
        self.deposited_amount = amount;
        self
    }

    pub fn withdrawn_amount(mut self, amount: u64) -> Self {
        self.withdrawn_amount = amount;
        self
    }

    pub fn vesting_period(mut self, start: i64, end: i64) -> Self {
        self.start_time = start;
        self.end_time = end;
        self
    }

    pub fn cliff_time(mut self, cliff: i64) -> Self {
        self.cliff_time = cliff;
        self
    }

    pub fn closed_at(mut self, closed: Option<i64>) -> Self {
        self.closed_at = closed;
        self
    }

    pub fn sender(mut self, sender: Pubkey) -> Self {
        self.sender = sender;
        self
    }

    pub fn build(self) -> StreamflowStream {
        StreamflowStream {
            recipient: self.recipient,
            sender: self.sender,
            mint: self.mint,
            deposited_amount: self.deposited_amount,
            withdrawn_amount: self.withdrawn_amount,
            start_time: self.start_time,
            end_time: self.end_time,
            cliff_time: self.cliff_time,
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
            closed_at: self.closed_at,
        }
    }

    /// Create a stream that's fully locked at the given timestamp
    pub fn fully_locked_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp + 1000, timestamp + 2000) // Starts in future
            .build()
    }

    /// Create a stream that's fully vested at the given timestamp
    pub fn fully_vested_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp - 2000, timestamp - 1000) // Ended in past
            .build()
    }

    /// Create a stream that's 50% vested at the given timestamp
    pub fn half_vested_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        let duration = 1000i64;
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp - duration / 2, timestamp + duration / 2)
            .cliff_time(timestamp - duration / 2)
            .build()
    }

    /// Create a stream with custom vesting percentage at the given timestamp
    pub fn vested_percentage_at(
        recipient: Pubkey, 
        mint: Pubkey, 
        amount: u64, 
        timestamp: i64, 
        vested_percentage: f64
    ) -> StreamflowStream {
        let duration = 1000i64;
        let start_time = timestamp - (duration as f64 * vested_percentage) as i64;
        let end_time = start_time + duration;
        
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(start_time, end_time)
            .cliff_time(start_time)
            .build()
    }
}

#[cfg(test)]
pub struct MockInvestorScenario {
    pub investors: Vec<Pubkey>,
    pub streams: Vec<StreamflowStream>,
    pub mint: Pubkey,
    pub timestamp: i64,
}

#[cfg(test)]
impl MockInvestorScenario {
    /// Create a scenario with multiple investors at different vesting stages
    pub fn diverse_vesting_scenario() -> Self {
        let mint = Pubkey::new_unique();
        let timestamp = 1500i64;
        
        let investor1 = Pubkey::new_unique();
        let investor2 = Pubkey::new_unique();
        let investor3 = Pubkey::new_unique();
        let investor4 = Pubkey::new_unique();

        let streams = vec![
            // Investor 1: Fully locked (1M tokens)
            MockStreamflowBuilder::fully_locked_at(investor1, mint, 1_000_000, timestamp),
            
            // Investor 2: 50% vested (2M tokens, 1M locked)
            MockStreamflowBuilder::half_vested_at(investor2, mint, 2_000_000, timestamp),
            
            // Investor 3: 75% vested (1M tokens, 250k locked)
            MockStreamflowBuilder::vested_percentage_at(investor3, mint, 1_000_000, timestamp, 0.75),
            
            // Investor 4: Fully vested (500k tokens, 0 locked)
            MockStreamflowBuilder::fully_vested_at(investor4, mint, 500_000, timestamp),
        ];

        Self {
            investors: vec![investor1, investor2, investor3, investor4],
            streams,
            mint,
            timestamp,
        }
    }

    /// Create a scenario with multiple streams per investor
    pub fn multiple_streams_per_investor() -> Self {
        let mint = Pubkey::new_unique();
        let timestamp = 1500i64;
        
        let investor1 = Pubkey::new_unique();
        let investor2 = Pubkey::new_unique();

        let streams = vec![
            // Investor 1: Two streams
            MockStreamflowBuilder::half_vested_at(investor1, mint, 600_000, timestamp),
            MockStreamflowBuilder::vested_percentage_at(investor1, mint, 400_000, timestamp, 0.25)
                .with_withdrawn_amount(100_000), // Some withdrawn
            
            // Investor 2: Three streams with different vesting
            MockStreamflowBuilder::fully_locked_at(investor2, mint, 300_000, timestamp),
            MockStreamflowBuilder::half_vested_at(investor2, mint, 500_000, timestamp),
            MockStreamflowBuilder::vested_percentage_at(investor2, mint, 200_000, timestamp, 0.9),
        ];

        Self {
            investors: vec![investor1, investor2],
            streams,
            mint,
            timestamp,
        }
    }

    /// Create a scenario with dust amounts for testing small payouts
    pub fn dust_scenario() -> Self {
        let mint = Pubkey::new_unique();
        let timestamp = 1500i64;
        
        let investors: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();

        let streams = investors.iter().enumerate().map(|(i, &investor)| {
            // Create small amounts that will generate dust
            let amount = 100 + (i as u64 * 50); // 100, 150, 200, 250, 300
            MockStreamflowBuilder::half_vested_at(investor, mint, amount, timestamp)
        }).collect();

        Self {
            investors,
            streams,
            mint,
            timestamp,
        }
    }

    /// Get total locked amount across all streams
    pub fn total_locked(&self) -> u64 {
        use crate::utils::streamflow::StreamflowIntegration;
        
        self.streams.iter()
            .map(|stream| StreamflowIntegration::calculate_locked_amount(stream, self.timestamp).unwrap_or(0))
            .sum()
    }

    /// Get locked amount for a specific investor
    pub fn investor_locked(&self, investor: &Pubkey) -> u64 {
        use crate::utils::streamflow::StreamflowIntegration;
        
        self.streams.iter()
            .filter(|stream| stream.recipient == *investor)
            .map(|stream| StreamflowIntegration::calculate_locked_amount(stream, self.timestamp).unwrap_or(0))
            .sum()
    }

    /// Get total allocation across all streams
    pub fn total_allocation(&self) -> u64 {
        self.streams.iter()
            .map(|stream| stream.deposited_amount)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::streamflow::StreamflowIntegration;

    #[test]
    fn test_mock_builder() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        
        let stream = MockStreamflowBuilder::new(recipient, mint)
            .deposited_amount(500_000)
            .withdrawn_amount(50_000)
            .vesting_period(1000, 2000)
            .cliff_time(1100)
            .build();

        assert_eq!(stream.recipient, recipient);
        assert_eq!(stream.mint, mint);
        assert_eq!(stream.deposited_amount, 500_000);
        assert_eq!(stream.withdrawn_amount, 50_000);
        assert_eq!(stream.start_time, 1000);
        assert_eq!(stream.end_time, 2000);
        assert_eq!(stream.cliff_time, 1100);
    }

    #[test]
    fn test_convenience_builders() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let timestamp = 1500i64;
        let amount = 1_000_000u64;

        // Test fully locked
        let locked_stream = MockStreamflowBuilder::fully_locked_at(recipient, mint, amount, timestamp);
        let locked_amount = StreamflowIntegration::calculate_locked_amount(&locked_stream, timestamp).unwrap();
        assert_eq!(locked_amount, amount);

        // Test fully vested
        let vested_stream = MockStreamflowBuilder::fully_vested_at(recipient, mint, amount, timestamp);
        let locked_amount = StreamflowIntegration::calculate_locked_amount(&vested_stream, timestamp).unwrap();
        assert_eq!(locked_amount, 0);

        // Test half vested
        let half_stream = MockStreamflowBuilder::half_vested_at(recipient, mint, amount, timestamp);
        let locked_amount = StreamflowIntegration::calculate_locked_amount(&half_stream, timestamp).unwrap();
        assert_eq!(locked_amount, amount / 2);
    }

    #[test]
    fn test_diverse_scenario() {
        let scenario = MockInvestorScenario::diverse_vesting_scenario();
        
        assert_eq!(scenario.investors.len(), 4);
        assert_eq!(scenario.streams.len(), 4);
        
        let total_locked = scenario.total_locked();
        // Investor 1: 1M locked + Investor 2: 1M locked + Investor 3: 250k locked + Investor 4: 0 locked
        assert_eq!(total_locked, 2_250_000);
        
        let total_allocation = scenario.total_allocation();
        // 1M + 2M + 1M + 500k = 4.5M
        assert_eq!(total_allocation, 4_500_000);
    }

    #[test]
    fn test_multiple_streams_scenario() {
        let scenario = MockInvestorScenario::multiple_streams_per_investor();
        
        assert_eq!(scenario.investors.len(), 2);
        assert_eq!(scenario.streams.len(), 5);
        
        // Test individual investor locked amounts
        let investor1_locked = scenario.investor_locked(&scenario.investors[0]);
        let investor2_locked = scenario.investor_locked(&scenario.investors[1]);
        
        // Verify totals
        assert_eq!(investor1_locked + investor2_locked, scenario.total_locked());
    }
}