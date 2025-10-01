# Meteora Fee Router - Comprehensive Test Suite

This directory contains a comprehensive test suite for the Meteora Fee Router program, covering all requirements specified in the design document.

## Test Structure

### Core Test Files

1. **`initialize-honorary-position.test.ts`**
   - Tests honorary position initialization
   - Validates quote-only configuration
   - Tests PDA derivation and account creation
   - **Requirements Covered**: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 8.1, 8.5

2. **`fee-claiming.test.ts`**
   - Tests DAMM V2 fee claiming integration
   - Validates quote-only enforcement during claims
   - Tests treasury ATA management
   - **Requirements Covered**: 3.3, 3.4, 2.4, 9.1

3. **`comprehensive-integration.test.ts`**
   - End-to-end integration tests
   - Full distribution cycle testing
   - Event emission verification
   - **Requirements Covered**: 9.1, 9.2, 9.3, 9.4, 9.5

4. **`streamflow-integration.test.ts`**
   - Streamflow vesting schedule integration
   - Locked amount calculations
   - Multiple streams per investor
   - **Requirements Covered**: 4.1, 8.3, 9.2

5. **`performance-compute.test.ts`**
   - Compute unit estimation and optimization
   - Memory usage analysis
   - Scalability testing
   - **Requirements Covered**: 9.5, 8.5

6. **`failure-edge-cases.test.ts`**
   - Comprehensive failure case testing
   - Edge case validation
   - Error handling verification
   - **Requirements Covered**: 9.1, 9.3, 2.4

7. **`pagination-resumption.test.ts`**
   - Pagination logic testing
   - Resumable operations after failures
   - State management during distribution
   - **Requirements Covered**: 6.1, 6.2, 6.3, 6.4, 9.4

### Test Utilities

- **`run-all-tests.ts`** - Comprehensive test runner with reporting
- **`README.md`** - This documentation file

## Running Tests

### Individual Test Suites

```bash
# Run specific test suites
npm run test:init          # Initialize honorary position tests
npm run test:fees          # Fee claiming tests
npm run test:comprehensive # Comprehensive integration tests
npm run test:streamflow    # Streamflow integration tests
npm run test:performance   # Performance and compute tests
npm run test:failures      # Failure and edge case tests
npm run test:pagination    # Pagination and resumption tests
```

### All Tests

```bash
# Run all tests with standard Anchor test runner
npm run test

# Run comprehensive test suite with detailed reporting
npx ts-node tests/run-all-tests.ts
```

## Test Coverage

### Requirements Coverage Matrix

| Requirement | Description | Test Files |
|-------------|-------------|------------|
| 1.1-1.5 | Honorary Position Management | initialize-honorary-position.test.ts |
| 2.1-2.4 | Quote-Only Fee Enforcement | initialize-honorary-position.test.ts, fee-claiming.test.ts, failure-edge-cases.test.ts |
| 3.1-3.5 | 24-Hour Distribution Crank | comprehensive-integration.test.ts, fee-claiming.test.ts |
| 4.1-4.6 | Investor Fee Distribution | streamflow-integration.test.ts, comprehensive-integration.test.ts |
| 5.1-5.5 | Creator Remainder Distribution | comprehensive-integration.test.ts |
| 6.1-6.5 | Idempotent Pagination System | pagination-resumption.test.ts, comprehensive-integration.test.ts |
| 7.1-7.5 | Policy Configuration and Limits | initialize-honorary-position.test.ts, comprehensive-integration.test.ts |
| 8.1-8.5 | Integration and Account Management | initialize-honorary-position.test.ts, performance-compute.test.ts |
| 9.1-9.5 | Comprehensive Testing and Validation | All test files |
| 10.1-10.5 | Documentation and Deliverables | This README and test documentation |

### Test Scenarios Covered

#### Initialization Tests
- ✅ Valid parameter initialization
- ✅ PDA derivation validation
- ✅ Account creation and ownership
- ✅ Parameter range validation
- ✅ Invalid parameter rejection
- ✅ Event emission verification

#### Quote-Only Enforcement Tests
- ✅ Base fee detection and rejection
- ✅ Pool configuration validation
- ✅ Mint validation
- ✅ Runtime fee validation
- ✅ Preflight validation

#### Distribution Logic Tests
- ✅ Partial lock scenarios
- ✅ Full unlock scenarios
- ✅ Dust accumulation and carry-forward
- ✅ Daily cap enforcement
- ✅ Minimum payout thresholds
- ✅ Proportional distribution calculations

#### Streamflow Integration Tests
- ✅ Active stream locked amount calculations
- ✅ Fully vested stream handling
- ✅ Streams with withdrawals
- ✅ Cliff period handling
- ✅ Multiple streams per investor
- ✅ Edge cases and error handling

#### Pagination and Resumption Tests
- ✅ Single page distribution
- ✅ Multi-page distribution
- ✅ Edge case page sizes
- ✅ Resumption after partial failure
- ✅ Multiple failure and resumption cycles
- ✅ Idempotent operations during retries
- ✅ State management during pagination

#### Performance Tests
- ✅ Compute unit estimation
- ✅ Memory usage optimization
- ✅ Transaction size analysis
- ✅ Account limit validation
- ✅ Scalability analysis
- ✅ Network congestion impact

#### Failure Case Tests
- ✅ Timing and cooldown failures
- ✅ Daily cap and limit failures
- ✅ Pagination and state failures
- ✅ Arithmetic overflow scenarios
- ✅ Account and permission failures
- ✅ Invalid parameter handling

## Test Environment Setup

### Prerequisites

1. **Solana CLI** - Latest version
2. **Anchor Framework** - v0.29.0 or later
3. **Node.js** - v16 or later
4. **Local Validator** - For integration tests

### Setup Commands

```bash
# Install dependencies
npm install

# Build the program
npm run build

# Start local validator (in separate terminal)
solana-test-validator

# Run tests
npm run test
```

### Mock Data and Accounts

The test suite uses mock accounts and data structures to simulate:
- DAMM V2 pool and position accounts
- Streamflow vesting streams
- Investor wallets and ATAs
- Fee claiming scenarios

## Test Reporting

The comprehensive test runner (`run-all-tests.ts`) provides:

- **Execution Summary** - Pass/fail rates and timing
- **Requirements Coverage** - Which tests cover which requirements
- **Performance Analysis** - Duration analysis and optimization recommendations
- **Detailed Results** - Per-test-suite breakdown with error details

## Continuous Integration

These tests are designed to be run in CI/CD pipelines with:
- Deterministic test execution
- Clear pass/fail criteria
- Comprehensive error reporting
- Performance regression detection

## Contributing

When adding new tests:

1. Follow the existing test structure and naming conventions
2. Update the requirements coverage matrix
3. Add appropriate mock data and scenarios
4. Include both positive and negative test cases
5. Update this README with new test descriptions

## Troubleshooting

### Common Issues

1. **Account Not Initialized Errors**
   - Expected with mock DAMM V2 accounts
   - Tests validate structure, not actual program execution

2. **Timeout Errors**
   - Increase timeout in test configuration
   - Check local validator is running

3. **Compute Budget Errors**
   - Review compute unit estimates in performance tests
   - Optimize instruction complexity

### Debug Mode

Run tests with additional logging:

```bash
ANCHOR_LOG=true npm run test
```

## Performance Benchmarks

Expected test execution times:
- Individual test suites: 10-60 seconds
- Full test suite: 3-10 minutes
- Comprehensive test runner: 5-15 minutes

Performance targets:
- Compute units per instruction: < 200,000
- Account rent costs: < 0.01 SOL per account
- Transaction success rate: > 95% under normal conditions