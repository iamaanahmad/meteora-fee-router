#[cfg(test)]
mod timing_integration_tests {
    use crate::{
        state::{DistributionProgress, DistributionTimingState},
        constants::TWENTY_FOUR_HOURS,
        error::ErrorCode,
    };
    use anchor_lang::prelude::*;

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
    fn test_complete_24_hour_cycle() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        // Test initial state
        assert_eq!(progress.last_distribution_ts, 0);
        assert_eq!(progress.pagination_cursor, 0);
        assert!(!progress.day_complete);
        
        // First distribution - should start new day
        let timing_state = progress.prepare_for_distribution(start_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        assert_eq!(progress.last_distribution_ts, start_time);
        
        // Same day operations
        for hour in 1..24 {
            let current_time = start_time + (hour * 3600);
            let timing_state = progress.prepare_for_distribution(current_time).unwrap();
            assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        }
        
        // Just before 24 hours - should still be same day
        let almost_24h = start_time + TWENTY_FOUR_HOURS - 1;
        let timing_state = progress.prepare_for_distribution(almost_24h).unwrap();
        assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        
        // Exactly 24 hours - should be new day
        let exactly_24h = start_time + TWENTY_FOUR_HOURS;
        let timing_state = progress.prepare_for_distribution(exactly_24h).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        assert_eq!(progress.last_distribution_ts, exactly_24h);
    }

    #[test]
    fn test_pagination_with_timing() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        // Start distribution
        progress.prepare_for_distribution(start_time).unwrap();
        
        // Simulate pagination within same day
        let page_size = 10u32;
        for page in 0..5 {
            let expected_cursor = page * page_size;
            assert_eq!(progress.pagination_cursor, expected_cursor);
            
            // Process page
            progress.mark_page_processed(expected_cursor, page_size).unwrap();
            
            // Verify cursor advanced
            assert_eq!(progress.pagination_cursor, expected_cursor + page_size);
            
            // Verify same day continuation still works
            let current_time = start_time + 3600; // 1 hour later
            let timing_state = progress.prepare_for_distribution(current_time).unwrap();
            assert_eq!(timing_state, DistributionTimingState::ContinueSameDay);
        }
    }

    #[test]
    fn test_idempotent_retry_scenarios() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        // Start distribution and advance cursor
        progress.prepare_for_distribution(start_time).unwrap();
        progress.advance_cursor(20).unwrap();
        
        // Test retry of already processed cursor
        let is_retry = progress.validate_cursor_for_retry(10).unwrap();
        assert!(is_retry);
        
        // Test retry of current cursor
        let is_retry = progress.validate_cursor_for_retry(20).unwrap();
        assert!(!is_retry);
        
        // Test invalid future cursor
        let result = progress.validate_cursor_for_retry(30);
        assert!(result.is_err());
    }

    #[test]
    fn test_day_completion_and_reset() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        // Start and complete a day
        progress.prepare_for_distribution(start_time).unwrap();
        progress.advance_cursor(50).unwrap();
        progress.complete_day();
        
        // Should not allow same day continuation
        let same_day_time = start_time + 3600;
        let result = progress.validate_distribution_timing(same_day_time);
        assert!(result.is_err());
        
        // Should allow new day after 24 hours
        let next_day_time = start_time + TWENTY_FOUR_HOURS;
        let timing_state = progress.prepare_for_distribution(next_day_time).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // Cursor should be reset for new day
        assert_eq!(progress.pagination_cursor, 0);
        assert!(!progress.day_complete);
    }

    #[test]
    fn test_error_recovery_scenarios() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        // Start distribution and advance cursor
        progress.prepare_for_distribution(start_time).unwrap();
        progress.advance_cursor(30).unwrap();
        
        // Simulate error recovery by resetting cursor
        progress.reset_cursor_to(20).unwrap();
        assert_eq!(progress.pagination_cursor, 20);
        
        // Should be able to continue from reset position
        progress.advance_cursor(5).unwrap();
        assert_eq!(progress.pagination_cursor, 25);
        
        // Cannot reset to future position
        let result = progress.reset_cursor_to(30);
        assert!(result.is_err());
    }

    #[test]
    fn test_timing_edge_cases() {
        let mut progress = create_test_progress();
        
        // Test with zero timestamp
        let timing_state = progress.prepare_for_distribution(0).unwrap();
        assert_eq!(timing_state, DistributionTimingState::NewDay);
        
        // Test with very large timestamp
        let large_time = i64::MAX / 2;
        progress.prepare_for_distribution(large_time).unwrap();
        
        // Should still respect 24-hour rule
        let result = progress.validate_distribution_timing(large_time + TWENTY_FOUR_HOURS - 1);
        assert_eq!(result.unwrap(), DistributionTimingState::ContinueSameDay);
        
        let result = progress.validate_distribution_timing(large_time + TWENTY_FOUR_HOURS);
        assert_eq!(result.unwrap(), DistributionTimingState::NewDay);
    }

    #[test]
    fn test_distribution_period_info_accuracy() {
        let mut progress = create_test_progress();
        let start_time = 1000i64;
        
        progress.prepare_for_distribution(start_time).unwrap();
        progress.advance_cursor(15).unwrap();
        
        let current_time = start_time + 7200; // 2 hours later
        let info = progress.get_distribution_period_info(current_time);
        
        assert_eq!(info.last_distribution_ts, start_time);
        assert_eq!(info.current_timestamp, current_time);
        assert_eq!(info.time_until_next, TWENTY_FOUR_HOURS - 7200);
        assert!(!info.can_start_new_day);
        assert!(info.is_same_day);
        assert!(!info.day_complete);
        assert_eq!(info.pagination_cursor, 15);
    }

    #[test]
    fn test_multiple_day_transitions() {
        let mut progress = create_test_progress();
        let mut current_time = 1000i64;
        
        // Simulate multiple days
        for day in 0..5 {
            let timing_state = progress.prepare_for_distribution(current_time).unwrap();
            assert_eq!(timing_state, DistributionTimingState::NewDay);
            
            // Simulate some pagination within the day
            progress.advance_cursor(20).unwrap();
            progress.complete_day();
            
            // Move to next day
            current_time += TWENTY_FOUR_HOURS;
        }
        
        // Verify final state
        assert_eq!(progress.last_distribution_ts, current_time - TWENTY_FOUR_HOURS);
        assert!(progress.day_complete);
    }
}