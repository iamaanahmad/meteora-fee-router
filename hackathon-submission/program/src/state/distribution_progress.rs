use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::ErrorCode;

/// Represents the timing state for distribution operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistributionTimingState {
    /// Can start a new 24-hour distribution period
    NewDay,
    /// Can continue pagination within the same day
    ContinueSameDay,
}

/// Information about the current distribution period (for debugging/monitoring)
#[derive(Debug, Clone)]
pub struct DistributionPeriodInfo {
    pub last_distribution_ts: i64,
    pub current_timestamp: i64,
    pub time_until_next: i64,
    pub can_start_new_day: bool,
    pub is_same_day: bool,
    pub day_complete: bool,
    pub pagination_cursor: u32,
}

#[account]
pub struct DistributionProgress {
    /// The vault account used as seed for PDAs
    pub vault: Pubkey,
    /// Timestamp of last distribution
    pub last_distribution_ts: i64,
    /// Amount distributed in current day
    pub current_day_distributed: u64,
    /// Accumulated dust from previous distributions
    pub carry_over_dust: u64,
    /// Current pagination cursor
    pub pagination_cursor: u32,
    /// Whether the current day is complete
    pub day_complete: bool,
    /// PDA bump
    pub bump: u8,
}

impl DistributionProgress {
    /// Calculate space needed for account
    /// 32 (vault) + 8 (last_distribution_ts) + 8 (current_day_distributed) 
    /// + 8 (carry_over_dust) + 4 (pagination_cursor) + 1 (day_complete) + 1 (bump)
    pub const INIT_SPACE: usize = 32 + 8 + 8 + 8 + 4 + 1 + 1;

    /// Initialize a new distribution progress tracker
    pub fn initialize(
        &mut self,
        vault: Pubkey,
        bump: u8,
    ) -> Result<()> {
        self.vault = vault;
        self.last_distribution_ts = 0;
        self.current_day_distributed = 0;
        self.carry_over_dust = 0;
        self.pagination_cursor = 0;
        self.day_complete = false;
        self.bump = bump;

        Ok(())
    }

    /// Check if 24 hours have passed since last distribution
    pub fn can_start_new_day(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.last_distribution_ts + TWENTY_FOUR_HOURS
    }

    /// Check if we're in the same day as the last distribution
    pub fn is_same_day(&self, current_timestamp: i64) -> bool {
        if self.last_distribution_ts == 0 {
            return false;
        }
        
        let time_diff = current_timestamp - self.last_distribution_ts;
        time_diff >= 0 && time_diff < TWENTY_FOUR_HOURS
    }

    /// Get the day boundary timestamp for the current distribution period
    pub fn get_day_boundary(&self) -> i64 {
        self.last_distribution_ts + TWENTY_FOUR_HOURS
    }

    /// Check if we can continue distribution in the same day (for pagination)
    pub fn can_continue_same_day(&self, current_timestamp: i64) -> bool {
        self.is_same_day(current_timestamp) && !self.day_complete
    }

    /// Validate timing for distribution operation
    pub fn validate_distribution_timing(&self, current_timestamp: i64) -> Result<DistributionTimingState> {
        if self.last_distribution_ts == 0 {
            // First distribution ever
            return Ok(DistributionTimingState::NewDay);
        }

        if self.can_start_new_day(current_timestamp) {
            // 24+ hours have passed, can start new day
            Ok(DistributionTimingState::NewDay)
        } else if self.can_continue_same_day(current_timestamp) {
            // Within same day and not complete, can continue pagination
            Ok(DistributionTimingState::ContinueSameDay)
        } else if self.is_same_day(current_timestamp) && self.day_complete {
            // Same day but already complete
            Err(ErrorCode::DayAlreadyComplete.into())
        } else {
            // Within 24 hours but something is wrong
            Err(ErrorCode::CooldownNotElapsed.into())
        }
    }

    /// Start a new distribution day
    pub fn start_new_day(&mut self, current_timestamp: i64) -> Result<()> {
        require!(
            self.can_start_new_day(current_timestamp),
            ErrorCode::CooldownNotElapsed
        );

        self.last_distribution_ts = current_timestamp;
        self.current_day_distributed = 0;
        self.pagination_cursor = 0;
        self.day_complete = false;

        Ok(())
    }

    /// Check if daily cap would be exceeded
    pub fn check_daily_cap(&self, additional_amount: u64, daily_cap: Option<u64>) -> Result<()> {
        if let Some(cap) = daily_cap {
            let new_total = self.current_day_distributed
                .checked_add(additional_amount)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            
            require!(
                new_total <= cap,
                ErrorCode::DailyCapExceeded
            );
        }
        Ok(())
    }

    /// Add to current day distributed amount
    pub fn add_distributed(&mut self, amount: u64) -> Result<()> {
        self.current_day_distributed = self.current_day_distributed
            .checked_add(amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        Ok(())
    }

    /// Update pagination cursor with validation
    pub fn update_cursor(&mut self, new_cursor: u32) -> Result<()> {
        require!(
            new_cursor >= self.pagination_cursor,
            ErrorCode::InvalidPaginationCursor
        );
        self.pagination_cursor = new_cursor;
        Ok(())
    }

    /// Advance pagination cursor by a specific amount (for idempotent operations)
    pub fn advance_cursor(&mut self, page_size: u32) -> Result<u32> {
        let new_cursor = self.pagination_cursor
            .checked_add(page_size)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        self.pagination_cursor = new_cursor;
        Ok(new_cursor)
    }

    /// Check if a cursor position has already been processed (for idempotent operations)
    pub fn is_cursor_processed(&self, cursor_position: u32) -> bool {
        cursor_position < self.pagination_cursor
    }

    /// Validate cursor for idempotent retry
    pub fn validate_cursor_for_retry(&self, requested_cursor: u32) -> Result<bool> {
        if requested_cursor < self.pagination_cursor {
            // This cursor has already been processed - idempotent retry
            Ok(true)
        } else if requested_cursor == self.pagination_cursor {
            // This is the next expected cursor - normal operation
            Ok(false)
        } else {
            // Cursor is ahead of expected - invalid
            Err(ErrorCode::InvalidPaginationCursor.into())
        }
    }

    /// Reset cursor to a specific position (for error recovery)
    pub fn reset_cursor_to(&mut self, cursor_position: u32) -> Result<()> {
        require!(
            cursor_position <= self.pagination_cursor,
            ErrorCode::InvalidPaginationCursor
        );
        self.pagination_cursor = cursor_position;
        Ok(())
    }

    /// Mark day as complete
    pub fn complete_day(&mut self) {
        self.day_complete = true;
    }

    /// Add dust to carry over
    pub fn add_dust(&mut self, dust_amount: u64) -> Result<()> {
        self.carry_over_dust = self.carry_over_dust
            .checked_add(dust_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        Ok(())
    }

    /// Consume carry over dust
    pub fn consume_dust(&mut self, amount: u64) -> Result<u64> {
        let consumed = std::cmp::min(self.carry_over_dust, amount);
        self.carry_over_dust = self.carry_over_dust
            .checked_sub(consumed)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        Ok(consumed)
    }

    /// Get the PDA seeds for this distribution progress (for signing)
    pub fn get_signer_seeds(&self) -> [&[u8]; 3] {
        [PROGRESS_SEED, self.vault.as_ref(), std::slice::from_ref(&self.bump)]
    }

    /// Reset for same-day continuation (pagination)
    pub fn reset_for_continuation(&mut self) {
        // Don't reset last_distribution_ts or current_day_distributed
        // Only reset pagination state
        self.pagination_cursor = 0;
        self.day_complete = false;
    }

    /// Get time remaining until next distribution window
    pub fn time_until_next_distribution(&self, current_timestamp: i64) -> i64 {
        if self.last_distribution_ts == 0 {
            return 0; // Can distribute immediately
        }
        
        let next_allowed = self.last_distribution_ts + TWENTY_FOUR_HOURS;
        if current_timestamp >= next_allowed {
            0 // Can distribute now
        } else {
            next_allowed - current_timestamp
        }
    }

    /// Check if distribution can be started or continued
    pub fn can_distribute(&self, current_timestamp: i64) -> Result<DistributionTimingState> {
        self.validate_distribution_timing(current_timestamp)
    }

    /// Prepare for distribution operation with timing validation
    pub fn prepare_for_distribution(&mut self, current_timestamp: i64) -> Result<DistributionTimingState> {
        let timing_state = self.validate_distribution_timing(current_timestamp)?;
        
        match timing_state {
            DistributionTimingState::NewDay => {
                self.start_new_day(current_timestamp)?;
            },
            DistributionTimingState::ContinueSameDay => {
                // No state changes needed for continuation
            }
        }
        
        Ok(timing_state)
    }

    /// Mark a page as processed (idempotent operation support)
    pub fn mark_page_processed(&mut self, page_start: u32, page_size: u32) -> Result<()> {
        let expected_cursor = page_start + page_size;
        
        // Ensure we're processing pages in order
        require!(
            page_start == self.pagination_cursor,
            ErrorCode::InvalidPaginationCursor
        );
        
        self.pagination_cursor = expected_cursor;
        Ok(())
    }

    /// Check if we're at the start of a new distribution period
    pub fn is_new_distribution_period(&self, current_timestamp: i64) -> bool {
        matches!(self.validate_distribution_timing(current_timestamp), Ok(DistributionTimingState::NewDay))
    }

    /// Get distribution period info for debugging/monitoring
    pub fn get_distribution_period_info(&self, current_timestamp: i64) -> DistributionPeriodInfo {
        DistributionPeriodInfo {
            last_distribution_ts: self.last_distribution_ts,
            current_timestamp,
            time_until_next: self.time_until_next_distribution(current_timestamp),
            can_start_new_day: self.can_start_new_day(current_timestamp),
            is_same_day: self.is_same_day(current_timestamp),
            day_complete: self.day_complete,
            pagination_cursor: self.pagination_cursor,
        }
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn create_test_progress() -> DistributionProgress {
        DistributionProgress {
            vault: Pubkey::new_unique(),
            last_distribution_ts: 0,
            current_day_distributed: 0,
            carry_over_dust: 0,
            pagination_cursor: 0,
            day_complete: false,
            bump: 255,
        }
    }

    #[test]
    fn test_24_hour_cooldown_validation() {
        let mut progress = create_test_progress();
        
        // First distribution should be allowed
        assert!(progress.can_start_new_day(1000));
        
        // Set initial distribution time
        progress.start_new_day(1000).unwrap();
        assert_eq!(progress.last_distribution_ts, 1000);
        
        // Should not allow distribution before 24 hours
        assert!(!progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS - 1));
        
        // Should allow distribution after exactly 24 hours
        assert!(progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS));
        
        // Should allow distribution after more than 24 hours
        assert!(progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS + 3600));
    }

    #[test]
    fn test_day_boundary_detection() {
        let mut progress = create_test_progress();
        progress.start_new_day(1000).unwrap();
        
        // Same day detection
        assert!(progress.is_same_day(1000));
        assert!(progress.is_same_day(1000 + TWENTY_FOUR_HOURS - 1));
        assert!(!progress.is_same_day(1000 + TWENTY_FOUR_HOURS));
        
        // Day boundary calculation
        assert_eq!(progress.get_day_boundary(), 1000 + TWENTY_FOUR_HOURS);
        
        // Same day continuation
        assert!(progress.can_continue_same_day(1000 + 3600)); // 1 hour later
        
        // Mark day complete and test continuation
        progress.complete_day();
        assert!(!progress.can_continue_same_day(1000 + 3600));
    }

    #[test]
    fn test_distribution_timing_validation() {
        let mut progress = create_test_progress();
        
        // First distribution
        let result = progress.validate_distribution_timing(1000).unwrap();
        assert_eq!(result, DistributionTimingState::NewDay);
        
        // Start the day
        progress.start_new_day(1000).unwrap();
        
        // Same day continuation
        let result = progress.validate_distribution_timing(1000 + 3600).unwrap();
        assert_eq!(result, DistributionTimingState::ContinueSameDay);
        
        // Complete the day
        progress.complete_day();
        
        // Should fail if trying to continue completed day
        let result = progress.validate_distribution_timing(1000 + 3600);
        assert!(result.is_err());
        
        // Should allow new day after 24 hours
        let result = progress.validate_distribution_timing(1000 + TWENTY_FOUR_HOURS).unwrap();
        assert_eq!(result, DistributionTimingState::NewDay);
    }

    #[test]
    fn test_pagination_cursor_management() {
        let mut progress = create_test_progress();
        
        // Initial cursor should be 0
        assert_eq!(progress.pagination_cursor, 0);
        
        // Test cursor advancement
        let new_cursor = progress.advance_cursor(10).unwrap();
        assert_eq!(new_cursor, 10);
        assert_eq!(progress.pagination_cursor, 10);
        
        // Test cursor validation
        assert!(progress.update_cursor(15).is_ok());
        assert!(progress.update_cursor(10).is_err()); // Can't go backwards
        
        // Test processed cursor check
        assert!(progress.is_cursor_processed(5));
        assert!(progress.is_cursor_processed(14));
        assert!(!progress.is_cursor_processed(15));
        assert!(!progress.is_cursor_processed(20));
    }

    #[test]
    fn test_idempotent_operation_support() {
        let mut progress = create_test_progress();
        progress.pagination_cursor = 20;
        
        // Test cursor validation for retry
        let is_retry = progress.validate_cursor_for_retry(15).unwrap();
        assert!(is_retry); // Already processed
        
        let is_retry = progress.validate_cursor_for_retry(20).unwrap();
        assert!(!is_retry); // Next expected
        
        let result = progress.validate_cursor_for_retry(25);
        assert!(result.is_err()); // Invalid - ahead of expected
        
        // Test page processing
        assert!(progress.mark_page_processed(20, 5).is_ok());
        assert_eq!(progress.pagination_cursor, 25);
        
        // Test invalid page processing
        assert!(progress.mark_page_processed(30, 5).is_err()); // Wrong start position
    }

    #[test]
    fn test_cursor_reset_functionality() {
        let mut progress = create_test_progress();
        progress.pagination_cursor = 30;
        
        // Can reset to earlier position
        assert!(progress.reset_cursor_to(20).is_ok());
        assert_eq!(progress.pagination_cursor, 20);
        
        // Can reset to same position
        assert!(progress.reset_cursor_to(20).is_ok());
        assert_eq!(progress.pagination_cursor, 20);
        
        // Cannot reset to later position
        assert!(progress.reset_cursor_to(25).is_err());
    }

    #[test]
    fn test_time_until_next_distribution() {
        let mut progress = create_test_progress();
        
        // No previous distribution
        assert_eq!(progress.time_until_next_distribution(1000), 0);
        
        // Set distribution time
        progress.start_new_day(1000).unwrap();
        
        // Check time remaining
        assert_eq!(progress.time_until_next_distribution(1000 + 3600), TWENTY_FOUR_HOURS - 3600);
        assert_eq!(progress.time_until_next_distribution(1000 + TWENTY_FOUR_HOURS - 1), 1);
        assert_eq!(progress.time_until_next_distribution(1000 + TWENTY_FOUR_HOURS), 0);
        assert_eq!(progress.time_until_next_distribution(1000 + TWENTY_FOUR_HOURS + 100), 0);
    }

    #[test]
    fn test_prepare_for_distribution() {
        let mut progress = create_test_progress();
        
        // First distribution
        let state = progress.prepare_for_distribution(1000).unwrap();
        assert_eq!(state, DistributionTimingState::NewDay);
        assert_eq!(progress.last_distribution_ts, 1000);
        assert_eq!(progress.pagination_cursor, 0);
        assert!(!progress.day_complete);
        
        // Same day continuation
        let state = progress.prepare_for_distribution(1000 + 3600).unwrap();
        assert_eq!(state, DistributionTimingState::ContinueSameDay);
        assert_eq!(progress.last_distribution_ts, 1000); // Unchanged
    }

    #[test]
    fn test_distribution_period_info() {
        let mut progress = create_test_progress();
        progress.start_new_day(1000).unwrap();
        progress.pagination_cursor = 15;
        
        let info = progress.get_distribution_period_info(1000 + 3600);
        
        assert_eq!(info.last_distribution_ts, 1000);
        assert_eq!(info.current_timestamp, 1000 + 3600);
        assert_eq!(info.time_until_next, TWENTY_FOUR_HOURS - 3600);
        assert!(!info.can_start_new_day);
        assert!(info.is_same_day);
        assert!(!info.day_complete);
        assert_eq!(info.pagination_cursor, 15);
    }

    #[test]
    fn test_edge_case_day_transitions() {
        let mut progress = create_test_progress();
        
        // Test exact boundary conditions
        progress.start_new_day(1000).unwrap();
        
        // One second before 24 hours
        assert!(progress.is_same_day(1000 + TWENTY_FOUR_HOURS - 1));
        assert!(!progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS - 1));
        
        // Exactly 24 hours
        assert!(!progress.is_same_day(1000 + TWENTY_FOUR_HOURS));
        assert!(progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS));
        
        // One second after 24 hours
        assert!(!progress.is_same_day(1000 + TWENTY_FOUR_HOURS + 1));
        assert!(progress.can_start_new_day(1000 + TWENTY_FOUR_HOURS + 1));
    }

    #[test]
    fn test_arithmetic_overflow_protection() {
        let mut progress = create_test_progress();
        progress.pagination_cursor = u32::MAX - 5;
        
        // Should handle overflow gracefully
        let result = progress.advance_cursor(10);
        assert!(result.is_err());
        
        // Should still work within bounds
        assert!(progress.advance_cursor(3).is_ok());
        assert_eq!(progress.pagination_cursor, u32::MAX - 2);
    }

    #[test]
    fn test_new_distribution_period_detection() {
        let mut progress = create_test_progress();
        
        // Initially should be new period
        assert!(progress.is_new_distribution_period(1000));
        
        // After starting, same day should not be new period
        progress.start_new_day(1000).unwrap();
        assert!(!progress.is_new_distribution_period(1000 + 3600));
        
        // After 24 hours should be new period
        assert!(progress.is_new_distribution_period(1000 + TWENTY_FOUR_HOURS));
    }

    #[test]
    fn test_cooldown_error_conditions() {
        let mut progress = create_test_progress();
        progress.start_new_day(1000).unwrap();
        
        // Should fail to start new day before cooldown
        let result = progress.start_new_day(1000 + TWENTY_FOUR_HOURS - 1);
        assert!(result.is_err());
        
        // Should succeed after cooldown
        let result = progress.start_new_day(1000 + TWENTY_FOUR_HOURS);
        assert!(result.is_ok());
    }
}