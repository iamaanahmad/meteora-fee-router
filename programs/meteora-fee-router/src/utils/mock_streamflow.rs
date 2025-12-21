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
    cliff_amount: u64,
    amount_per_period: u64,
    period: u64,
    closed: bool,
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
            cliff_amount: 0,
            amount_per_period: 1000,  // 1000 per period
            period: 100,               // 100 second periods
            closed: false,
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

    pub fn cliff_amount(mut self, amount: u64) -> Self {
        self.cliff_amount = amount;
        self
    }

    pub fn amount_per_period(mut self, amount: u64) -> Self {
        self.amount_per_period = amount;
        self
    }

    pub fn period(mut self, period: u64) -> Self {
        self.period = period;
        self
    }

    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    pub fn sender(mut self, sender: Pubkey) -> Self {
        self.sender = sender;
        self
    }

    pub fn build(self) -> StreamflowStream {
        StreamflowStream {
            sender: self.sender,
            recipient: self.recipient,
            mint: self.mint,
            deposited_amount: self.deposited_amount,
            withdrawn_amount: self.withdrawn_amount,
            start_time: self.start_time,
            end_time: self.end_time,
            cliff_time: self.cliff_time,
            cliff_amount: self.cliff_amount,
            amount_per_period: self.amount_per_period,
            period: self.period,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            last_withdrawn_at: 0,
            closed: self.closed,
        }
    }

    /// Create a stream that's fully locked at the given timestamp
    pub fn fully_locked_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp + 1000, timestamp + 2000) // Starts in future
            .cliff_time(timestamp + 1000)
            .build()
    }

    /// Create a stream that's fully vested at the given timestamp
    pub fn fully_vested_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp - 2000, timestamp - 1000) // Ended in past
            .cliff_time(timestamp - 2000)
            .build()
    }

    /// Create a stream that's exactly 50% vested at the given timestamp
    /// Uses 10 periods of 100 seconds each to get clean 50% at midpoint
    pub fn half_vested_at(recipient: Pubkey, mint: Pubkey, amount: u64, timestamp: i64) -> StreamflowStream {
        let duration = 1000i64;
        let period = 100u64;  // 10 periods total
        let num_periods = (duration as u64) / period;
        let amount_per_period = amount / num_periods;
        
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(timestamp - duration / 2, timestamp + duration / 2)
            .cliff_time(timestamp - duration / 2)
            .period(period)
            .amount_per_period(amount_per_period)
            .cliff_amount(0)
            .build()
    }

    /// Create a stream with custom vesting percentage at the given timestamp
    pub fn vested_percentage_at_builder(
        recipient: Pubkey, 
        mint: Pubkey, 
        amount: u64, 
        timestamp: i64, 
        vested_percentage: f64
    ) -> Self {
        let duration = 1000i64;
        let start_time = timestamp - (duration as f64 * vested_percentage) as i64;
        let end_time = start_time + duration;
        let period = 100u64;
        let num_periods = (duration as u64) / period;
        let amount_per_period = amount / num_periods;
        
        Self::new(recipient, mint)
            .deposited_amount(amount)
            .vesting_period(start_time, end_time)
            .cliff_time(start_time)
            .period(period)
            .amount_per_period(amount_per_period)
            .cliff_amount(0)
    }

    pub fn vested_percentage_at(
        recipient: Pubkey,
        mint: Pubkey,
        amount: u64,
        timestamp: i64,
        vested_percentage: f64,
    ) -> StreamflowStream {
        Self::vested_percentage_at_builder(recipient, mint, amount, timestamp, vested_percentage)
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
            MockStreamflowBuilder::vested_percentage_at_builder(investor1, mint, 400_000, timestamp, 0.25)
                .withdrawn_amount(100_000)
                .build(), // Some withdrawn
            
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
        assert_eq!(locked_amount, amount, "Fully locked stream should have all tokens locked");

        // Test fully vested
        let vested_stream = MockStreamflowBuilder::fully_vested_at(recipient, mint, amount, timestamp);
        let locked_amount = StreamflowIntegration::calculate_locked_amount(&vested_stream, timestamp).unwrap();
        assert_eq!(locked_amount, 0, "Fully vested stream should have no tokens locked");

        // Test half vested - with period-based vesting, we get 50% locked
        let half_stream = MockStreamflowBuilder::half_vested_at(recipient, mint, amount, timestamp);
        let locked_amount = StreamflowIntegration::calculate_locked_amount(&half_stream, timestamp).unwrap();
        assert_eq!(locked_amount, amount / 2, "Half vested stream should have 50% locked");
    }

    #[test]
    fn test_diverse_scenario() {
        let scenario = MockInvestorScenario::diverse_vesting_scenario();
        
        assert_eq!(scenario.investors.len(), 4);
        assert_eq!(scenario.streams.len(), 4);
        
        // Calculate individual locked amounts for clarity
        let locked1 = StreamflowIntegration::calculate_locked_amount(&scenario.streams[0], scenario.timestamp).unwrap();
        let locked2 = StreamflowIntegration::calculate_locked_amount(&scenario.streams[1], scenario.timestamp).unwrap();
        let locked3 = StreamflowIntegration::calculate_locked_amount(&scenario.streams[2], scenario.timestamp).unwrap();
        let locked4 = StreamflowIntegration::calculate_locked_amount(&scenario.streams[3], scenario.timestamp).unwrap();
        
        // Investor 1: 1M fully locked
        assert_eq!(locked1, 1_000_000);
        // Investor 2: 2M half vested = 1M locked
        assert_eq!(locked2, 1_000_000);
        // Investor 3: 1M at 75% vested with period-based vesting
        // 75% = 750 seconds elapsed, period = 100, so 7 periods elapsed
        // amount_per_period = 1M / 10 = 100k, unlocked = 7 * 100k = 700k
        // locked = 1M - 700k = 300k
        assert_eq!(locked3, 300_000);
        // Investor 4: Fully vested = 0 locked
        assert_eq!(locked4, 0);
        
        let total_locked = scenario.total_locked();
        // 1M + 1M + 300k + 0 = 2.3M
        assert_eq!(total_locked, 2_300_000);
        
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