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
        closed_at: Option<i64>,
    ) -> StreamflowStream {
        StreamflowStream {
            recipient,
            sender: Pubkey::new_unique(),
            mint,
            deposited_amount,
            withdrawn_amount,
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
            closed_at,
        }
    }

    fn serialize_stream_to_bytes(stream: &StreamflowStream) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Add 8-byte discriminator
        data.extend_from_slice(&[0u8; 8]);
        
        // Serialize stream data
        data.extend_from_slice(&stream.recipient.to_bytes());
        data.extend_from_slice(&stream.sender.to_bytes());
        data.extend_from_slice(&stream.mint.to_bytes());
        data.extend_from_slice(&stream.deposited_amount.to_le_bytes());
        data.extend_from_slice(&stream.withdrawn_amount.to_le_bytes());
        data.extend_from_slice(&stream.start_time.to_le_bytes());
        data.extend_from_slice(&stream.end_time.to_le_bytes());
        data.extend_from_slice(&stream.cliff_time.to_le_bytes());
        
        // Boolean flags
        data.push(if stream.cancelable_by_sender { 1 } else { 0 });
        data.push(if stream.cancelable_by_recipient { 1 } else { 0 });
        data.push(if stream.automatic_withdrawal { 1 } else { 0 });
        data.push(if stream.transferable_by_sender { 1 } else { 0 });
        data.push(if stream.transferable_by_recipient { 1 } else { 0 });
        data.push(if stream.can_topup { 1 } else { 0 });
        
        // Stream name
        data.extend_from_slice(&stream.stream_name);
        
        // Additional fields
        data.extend_from_slice(&stream.withdrawn_tokens_recipient.to_le_bytes());
        data.extend_from_slice(&stream.withdrawn_tokens_sender.to_le_bytes());
        data.extend_from_slice(&stream.last_withdrawn_at.to_le_bytes());
        
        // Closed at (0 if None)
        let closed_at_value = stream.closed_at.unwrap_or(0);
        data.extend_from_slice(&closed_at_value.to_le_bytes());
        
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
            None,
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
        assert_eq!(deserialized.closed_at, original_stream.closed_at);
    }

    #[test]
    fn test_locked_amount_calculation_scenarios() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test case 1: Before start time
        let stream = create_test_stream(recipient, mint, 1000000, 0, 1000, 2000, 1100, None);
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 500).unwrap();
        assert_eq!(locked, 1000000, "All tokens should be locked before start time");

        // Test case 2: Before cliff time
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1050).unwrap();
        assert_eq!(locked, 1000000, "All tokens should be locked before cliff time");

        // Test case 3: Linear vesting - 25% through
        let stream = create_test_stream(recipient, mint, 1000000, 0, 1000, 2000, 1000, None);
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1250).unwrap();
        assert_eq!(locked, 750000, "75% should be locked at 25% vesting progress");

        // Test case 4: Linear vesting - 50% through
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 500000, "50% should be locked at 50% vesting progress");

        // Test case 5: Linear vesting - 90% through
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1900).unwrap();
        assert_eq!(locked, 100000, "10% should be locked at 90% vesting progress");

        // Test case 6: After end time
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 2500).unwrap();
        assert_eq!(locked, 0, "No tokens should be locked after end time");
    }

    #[test]
    fn test_locked_amount_with_withdrawals() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Stream with 1M deposited, 200k withdrawn
        let stream = create_test_stream(recipient, mint, 1000000, 200000, 1000, 2000, 1000, None);
        
        // At 50% vesting: 500k vested, but only 800k available (1M - 200k withdrawn)
        // So locked = 800k - 500k = 300k
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        assert_eq!(locked, 300000, "Should account for withdrawn tokens");

        // At 75% vesting: 750k vested, 800k available
        // So locked = 800k - 750k = 50k
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1750).unwrap();
        assert_eq!(locked, 50000, "Should account for withdrawn tokens at 75% vesting");
    }

    #[test]
    fn test_locked_amount_closed_stream() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let stream = create_test_stream(recipient, mint, 1000000, 0, 1000, 2000, 1000, Some(1500));
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1600).unwrap();
        assert_eq!(locked, 0, "Closed streams should have no locked tokens");
    }

    #[test]
    fn test_edge_case_zero_duration() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Stream with same start and end time
        let stream = create_test_stream(recipient, mint, 1000000, 0, 1000, 1000, 1000, None);
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1000).unwrap();
        assert_eq!(locked, 0, "Zero duration stream should have no locked tokens");
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
            None
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

        // Create mock account data for multiple streams
        let stream1 = create_test_stream(investor1, mint, 500000, 0, 1000, 2000, 1000, None);
        let stream2 = create_test_stream(investor1, mint, 300000, 0, 1000, 2000, 1000, None);
        let stream3 = create_test_stream(investor2, mint, 700000, 0, 1000, 2000, 1000, None);

        let data1 = serialize_stream_to_bytes(&stream1);
        let data2 = serialize_stream_to_bytes(&stream2);
        let data3 = serialize_stream_to_bytes(&stream3);

        // Test aggregation logic manually (since we can't easily create AccountInfo in unit tests)
        let current_time = 1500; // 50% through vesting

        let locked1 = StreamflowIntegration::calculate_locked_amount(&stream1, current_time).unwrap();
        let locked2 = StreamflowIntegration::calculate_locked_amount(&stream2, current_time).unwrap();
        let locked3 = StreamflowIntegration::calculate_locked_amount(&stream3, current_time).unwrap();

        // Investor 1 should have 250k + 150k = 400k locked
        assert_eq!(locked1 + locked2, 400000);
        
        // Investor 2 should have 350k locked
        assert_eq!(locked3, 350000);

        // Total locked should be 750k
        assert_eq!(locked1 + locked2 + locked3, 750000);
    }

    #[test]
    fn test_dust_amounts() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Very small stream amount
        let stream = create_test_stream(recipient, mint, 100, 0, 1000, 2000, 1000, None);
        
        // At various points in vesting
        let locked_25 = StreamflowIntegration::calculate_locked_amount(&stream, 1250).unwrap();
        let locked_50 = StreamflowIntegration::calculate_locked_amount(&stream, 1500).unwrap();
        let locked_75 = StreamflowIntegration::calculate_locked_amount(&stream, 1750).unwrap();

        // Should handle small amounts correctly
        assert!(locked_25 <= 100);
        assert!(locked_50 <= locked_25);
        assert!(locked_75 <= locked_50);
        assert_eq!(locked_25 + locked_50 + locked_75, 75 + 50 + 25); // Expected values for 100 token stream
    }

    #[test]
    fn test_invalid_stream_parameters() {
        let recipient = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test invalid time parameters (end before start)
        let invalid_stream = create_test_stream(recipient, mint, 1000000, 0, 2000, 1000, 1500, None);
        
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
        let stream = create_test_stream(recipient, mint, large_amount, 0, 1000, 2000, 1000, None);
        
        // At 33.33% through vesting
        let locked = StreamflowIntegration::calculate_locked_amount(&stream, 1333).unwrap();
        
        // Should maintain precision with large amounts
        let expected_vested = (large_amount as u128 * 333) / 1000;
        let expected_locked = large_amount - expected_vested as u64;
        
        // Allow for small rounding differences
        let diff = if locked > expected_locked { 
            locked - expected_locked 
        } else { 
            expected_locked - locked 
        };
        assert!(diff <= 1, "Should maintain precision with large amounts");
    }

    #[test]
    fn test_stream_data_parsing_edge_cases() {
        // Test with insufficient data
        let short_data = vec![0u8; 50];
        let result = StreamflowStream::try_from_account_data(&short_data);
        assert!(result.is_err(), "Should fail with insufficient data");

        // Test with no discriminator
        let no_discriminator = vec![0u8; 5];
        let result = StreamflowStream::try_from_account_data(&no_discriminator);
        assert!(result.is_err(), "Should fail with no discriminator");
    }
}