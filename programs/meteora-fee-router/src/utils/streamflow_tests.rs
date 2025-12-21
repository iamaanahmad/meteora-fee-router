#[cfg(test)]
mod tests {
    use anchor_lang::prelude::*;
    use crate::utils::streamflow::{StreamflowIntegration, StreamflowStream};

    // Mock Streamflow program ID for testing
    const MOCK_STREAMFLOW_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);

    fn create_test_stream(
        recipient: Pubkey,
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
    ) -> StreamflowStream {
        StreamflowStream {
            sender: Pubkey::new_unique(),
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
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            can_topup: false,
            stream_name: [0u8; 64],
            last_withdrawn_at: 0,
            closed,
        }
    }

    /// Helper to create a simple test stream with defaults for cliff/period
    fn create_simple_test_stream(
        recipient: Pubkey,
        mint: Pubkey,
        deposited_amount: u64,
        withdrawn_amount: u64,
        start_time: i64,
        end_time: i64,
        cliff_time: i64,
        closed: bool,
    ) -> StreamflowStream {
        // Calculate reasonable defaults:
        // - period of 100 seconds
        // - amount_per_period calculated to fully vest by end_time
        let duration = (end_time - cliff_time) as u64;
        let period = 100u64;
        let num_periods = if period > 0 { duration / period } else { 1 };
        let amount_per_period = if num_periods > 0 { deposited_amount / num_periods } else { deposited_amount };
        
        create_test_stream(
            recipient,
            mint,
            deposited_amount,
            withdrawn_amount,
            start_time,
            end_time,
            cliff_time,
            0,  // no cliff amount
            amount_per_period,
            period,
            closed,
        )
    }

    /// Serialize a stream to bytes matching the actual Streamflow layout
    fn serialize_stream_to_bytes(stream: &StreamflowStream) -> Vec<u8> {
        let mut data = vec![0u8; 672]; // Minimum account size
        
        // Magic number (8 bytes) - non-zero to indicate valid stream
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        
        // Version (1 byte) at offset 8
        data[8] = 1;
        
        // created_at (8 bytes) at offset 9 - skip
        
        // withdrawn_amount (8 bytes) at offset 17
        data[17..25].copy_from_slice(&stream.withdrawn_amount.to_le_bytes());
        
        // canceled_at (8 bytes) at offset 25 - skip
        
        // end_time (8 bytes) at offset 33
        data[33..41].copy_from_slice(&stream.end_time.to_le_bytes());
        
        // last_withdrawn_at (8 bytes) at offset 41
        data[41..49].copy_from_slice(&stream.last_withdrawn_at.to_le_bytes());
        
        // sender (32 bytes) at offset 49
        data[49..81].copy_from_slice(&stream.sender.to_bytes());
        
        // sender_tokens (32 bytes) at offset 81 - skip
        
        // recipient (32 bytes) at offset 113
        data[113..145].copy_from_slice(&stream.recipient.to_bytes());
        
        // recipient_tokens (32 bytes) at offset 145 - skip
        
        // mint (32 bytes) at offset 177
        data[177..209].copy_from_slice(&stream.mint.to_bytes());
        
        // Skip escrow_tokens, streamflow_treasury, etc.
        
        // start_time (8 bytes) at offset 409
        data[409..417].copy_from_slice(&stream.start_time.to_le_bytes());
        
        // net_amount_deposited (8 bytes) at offset 417
        data[417..425].copy_from_slice(&stream.deposited_amount.to_le_bytes());
        
        // period (8 bytes) at offset 425
        data[425..433].copy_from_slice(&stream.period.to_le_bytes());
        
        // amount_per_period (8 bytes) at offset 433
        data[433..441].copy_from_slice(&stream.amount_per_period.to_le_bytes());
        
        // cliff (8 bytes) at offset 441
        data[441..449].copy_from_slice(&stream.cliff_time.to_le_bytes());
        
        // cliff_amount (8 bytes) at offset 449
        data[449..457].copy_from_slice(&stream.cliff_amount.to_le_bytes());
        
        // Boolean flags (1 byte each) at offsets 457-462
        data[457] = if stream.cancelable_by_sender { 1 } else { 0 };
        data[458] = if stream.cancelable_by_recipient { 1 } else { 0 };
        data[459] = if stream.automatic_withdrawal { 1 } else { 0 };
        data[460] = if stream.transferable_by_sender { 1 } else { 0 };
        data[461] = if stream.transferable_by_recipient { 1 } else { 0 };
        data[462] = if stream.can_topup { 1 } else { 0 };
        
        // stream_name (64 bytes) at offset 463
        data[463..527].copy_from_slice(&stream.stream_name);
        
        // closed (1 byte) at offset 671
        data[671] = if stream.closed { 1 } else { 0 };
        
        data
    }

    #[test]
    fn test_stream_serialization_deserialization() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let original_stream = create_test_stream(
            recipient,
            mint,
            1000000,
            100000,
            1000,
            2000,
            1100,
            50000,     // cliff_amount
            10000,     // amount_per_period
            100,       // period
            false,
        );

        let serialized = serialize_stream_to_bytes(&original_stream);
        let deserialized = StreamflowStream::try_from_account_data(&serialized).unwrap();

        assert_eq!(deserialized.recipient, original_stream.recipient);
        assert_eq!(deserialized.mint, original_stream.mint);
        assert_eq!(deserialized.deposited_amount, original_stream.deposited_amount);
        assert_eq!(deserialized.withdrawn_amount, original_stream.withdrawn_amount);
        assert_eq!(deserialized.start_time, original_stream.start_time);
        assert_eq!(deserialized.end_time, original_stream.end_time);
        assert_eq!(deserialized.cliff_time, original_stream.cliff_time);
        assert_eq!(deserialized.closed, original_stream.closed);
    }

    #[test]
    fn test_locked_amount_calculation_scenarios() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test case 1: Before start time - all tokens locked
        let stream = create_test_stream(
            recipient, mint, 
            1000000,  // deposited
            0,        // withdrawn
            1000,     // start
            2000,     // end
            1100,     // cliff
            100000,   // cliff_amount
            100000,   // amount_per_period
            100,      // period
            false,
        );
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 500).unwrap();
        assert_eq!(locked, 1000000, "All tokens should be locked before start time");

        // Test case 2: Before cliff time - all tokens locked
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1050).unwrap();
        assert_eq!(locked, 1000000, "All tokens should be locked before cliff time");

        // Test case 3: At cliff time - cliff amount unlocked
        // With cliff_amount=100000, after cliff we should have 900000 locked
        let stream = create_test_stream(
            recipient, mint, 
            1000000,  // deposited
            0,        // withdrawn
            1000,     // start
            2000,     // end
            1000,     // cliff (at start)
            100000,   // cliff_amount
            100000,   // amount_per_period (releases 100k per 100s period)
            100,      // period
            false,
        );
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1100).unwrap();
        // At t=1100, cliff passed at t=1000, 1 period elapsed (100s / 100 = 1)
        // unlocked = cliff_amount(100k) + 1 * amount_per_period(100k) = 200k
        // locked = 1000k - 200k = 800k
        assert_eq!(locked, 800000, "Should unlock cliff + 1 period at t=1100");

        // Test case 4: After end time
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 2500).unwrap();
        assert_eq!(locked, 0, "No tokens should be locked after end time");
    }

    #[test]
    fn test_locked_amount_with_withdrawals() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Stream with 1M deposited, 200k withdrawn
        // Uses period-based vesting
        let stream = create_test_stream(
            recipient, mint,
            1000000,  // deposited
            200000,   // withdrawn
            1000,     // start
            2000,     // end
            1000,     // cliff
            0,        // cliff_amount
            100000,   // amount_per_period
            100,      // period (10 periods total = 1M)
            false,
        );
        
        // At t=1500: 5 periods elapsed, 500k unlocked
        // locked = 1M - 500k = 500k (withdrawn doesn't affect locked calculation)
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 500000, "Should have 500k locked at 50% vesting");
    }

    #[test]
    fn test_locked_amount_closed_stream() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let stream = create_test_stream(
            recipient, mint,
            1000000, 0, 1000, 2000, 1000,
            0, 100000, 100,
            true,  // closed
        );
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1600).unwrap();
        assert_eq!(locked, 0, "Closed streams should have no locked tokens");
    }

    #[test]
    fn test_edge_case_zero_period() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Stream with zero period - should return 0 locked (instant unlock)
        let stream = create_test_stream(
            recipient, mint,
            1000000, 0, 1000, 2000, 1000,
            0, 100000, 0,  // period = 0
            false,
        );
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 0, "Zero period stream should have no locked tokens");
    }

    #[test]
    fn test_edge_case_overflow_protection() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Large numbers that could cause overflow
        let stream = create_test_stream(
            recipient, 
            mint, 
            u64::MAX, 
            0, 
            0, 
            i64::MAX, 
            0, 
            0,      // cliff_amount
            1000,   // amount_per_period
            100,    // period
            false,
        );
        
        // Should not panic due to overflow protection
        let result = StreamflowIntegration::calculate_locked_amount(&stream, i64::MAX / 2);
        assert!(result.is_ok(), "Should handle large numbers without overflow");
    }

    #[test]
    fn test_multiple_streams_aggregation() {
        let investor1 = Pubkey::new_unique();
        let investor2 = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Create streams with period-based vesting
        let stream1 = create_test_stream(
            investor1, mint, 500000, 0, 1000, 2000, 1000,
            0, 50000, 100,  // 10 periods of 50k each
            false,
        );
        let stream2 = create_test_stream(
            investor1, mint, 300000, 0, 1000, 2000, 1000,
            0, 30000, 100,  // 10 periods of 30k each
            false,
        );
        let stream3 = create_test_stream(
            investor2, mint, 700000, 0, 1000, 2000, 1000,
            0, 70000, 100,  // 10 periods of 70k each
            false,
        );

        let current_time = 1500; // 5 periods through vesting (50%)

        let locked1 = StreamflowIntegration::calculate_locked_amount(&stream1, current_time).unwrap();
        let locked2 = StreamflowIntegration::calculate_locked_amount(&stream2, current_time).unwrap();
        let locked3 = StreamflowIntegration::calculate_locked_amount(&stream3, current_time).unwrap();

        // At 50%: each stream should have 50% locked
        assert_eq!(locked1, 250000, "Stream 1: 50% of 500k locked");
        assert_eq!(locked2, 150000, "Stream 2: 50% of 300k locked");
        assert_eq!(locked3, 350000, "Stream 3: 50% of 700k locked");

        // Total locked should be 750k
        assert_eq!(locked1 + locked2 + locked3, 750000);
    }

    #[test]
    fn test_dust_amounts() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Very small stream amount
        let stream = create_test_stream(
            recipient, mint, 100, 0, 1000, 2000, 1000,
            0, 10, 100,  // 10 periods of 10 tokens each
            false,
        );
        
        // At various points in vesting
        let locked_start = StreamflowIntegration::calculate_locked_amount(&stream, 1000).unwrap();
        let locked_50 = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        let locked_end = StreamflowIntegration::calculate_locked_amount(&stream, 2000).unwrap();

        // Should handle small amounts correctly
        assert_eq!(locked_start, 100, "All locked at start");
        assert_eq!(locked_50, 50, "50% locked at midpoint");
        assert_eq!(locked_end, 0, "None locked at end");
    }

    #[test]
    fn test_invalid_stream_parameters() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test invalid time parameters (end before start)
        let invalid_stream = create_test_stream(
            recipient, mint, 1000000, 0, 2000, 1000, 1500,
            0, 100000, 100,
            false,
        );
        
        // This should be caught during validation, but let's test the calculation doesn't panic
        let result = StreamflowIntegration::calculate_locked_amount(&invalid_stream, 1500);
        assert!(result.is_ok(), "Should handle invalid parameters gracefully");
    }

    #[test]
    fn test_precision_with_large_amounts() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with large token amounts (like USDC with 6 decimals)
        let large_amount = 1_000_000_000_000u64; // 1M USDC (6 decimals)
        let stream = create_test_stream(
            recipient, mint, large_amount, 0, 1000, 2000, 1000,
            0, 
            large_amount / 10,  // amount_per_period
            100,                 // period (10 periods total)
            false,
        );
        
        // At 50% through vesting (5 periods)
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        let expected_locked = large_amount / 2;
        
        assert_eq!(locked, expected_locked, "Should maintain precision with large amounts");
    }

    #[test]
    fn test_stream_data_parsing_edge_cases() {
        // Test with insufficient data
        let short_data = vec![0u8; 50];
        let result = StreamflowStream::try_from_account_data(&short_data);
        assert!(result.is_err(), "Should fail with insufficient data");

        // Test with data that's too short (less than MIN_ACCOUNT_SIZE)
        let almost_enough = vec![0u8; 500];
        let result = StreamflowStream::try_from_account_data(&almost_enough);
        assert!(result.is_err(), "Should fail with data below minimum size");
        
        // Test with zero magic number
        let mut zero_magic = vec![0u8; 672];
        // Magic is already 0, should fail validation
        let result = StreamflowStream::try_from_account_data(&zero_magic);
        assert!(result.is_err(), "Should fail with zero magic number");
    }
}