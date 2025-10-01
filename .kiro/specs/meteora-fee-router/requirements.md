# Requirements Document

## Introduction

The Meteora Fee Router is a standalone Anchor-compatible Solana program that creates and manages an "honorary" DAMM v2 LP position for quote-only fee accrual and provides a permissionless 24-hour distribution crank system. This system enables automated fee collection from Meteora DAMM V2 pools and distributes them proportionally to investors based on their still-locked token amounts from Streamflow, with remaining fees routed to the creator wallet.

The program serves as a critical infrastructure component for Star's fundraising platform, enabling transparent and automated fee sharing between investors and creators based on vesting schedules.

## Requirements

### Requirement 1: Honorary Fee Position Management

**User Story:** As a Star platform administrator, I want to create and manage an honorary DAMM v2 LP position that accrues fees exclusively in the quote mint, so that I can collect trading fees without requiring base token exposure.

#### Acceptance Criteria

1. WHEN initializing an honorary position THEN the system SHALL create a DAMM v2 position owned by a program-derived address (PDA)
2. WHEN validating pool configuration THEN the system SHALL confirm the quote mint identity and reject any configuration that could accrue base fees
3. WHEN the position is created THEN the system SHALL use deterministic PDA seeds [VAULT_SEED, vault, "investor_fee_pos_owner"]
4. IF base fee accrual is detected during validation THEN the system SHALL fail deterministically without creating the position
5. WHEN the position is successfully initialized THEN the system SHALL emit an HonoraryPositionInitialized event

### Requirement 2: Quote-Only Fee Enforcement

**User Story:** As a platform operator, I want strict enforcement of quote-only fee accrual, so that the system never accepts or processes base token fees.

#### Acceptance Criteria

1. WHEN validating pool token order THEN the system SHALL identify and confirm which mint is the quote mint
2. WHEN any base fees are detected during claim operations THEN the system SHALL fail deterministically with no distribution
3. WHEN performing preflight validation THEN the system SHALL provide a deterministic step that rejects configurations allowing base fee accrual
4. IF a claim operation returns non-zero base tokens THEN the crank SHALL terminate without processing any distributions

### Requirement 3: Permissionless 24-Hour Distribution Crank

**User Story:** As any network participant, I want to trigger the fee distribution crank once per 24-hour period, so that fees are automatically distributed to eligible investors and creators.

#### Acceptance Criteria

1. WHEN the first crank of a day is called THEN the system SHALL require now >= last_distribution_ts + 86400 seconds
2. WHEN subsequent crank calls are made within the same day THEN the system SHALL allow pagination across multiple calls
3. WHEN the crank executes THEN the system SHALL claim fees from the honorary position into a program-owned quote treasury ATA
4. WHEN claiming fees THEN the system SHALL emit a QuoteFeesClaimed event with the claimed amount
5. WHEN processing is complete for a day THEN the system SHALL emit a CreatorPayoutDayClosed event

### Requirement 4: Investor Fee Distribution Logic

**User Story:** As an investor with locked tokens, I want to receive my proportional share of quote fees based on my still-locked amount, so that I benefit from trading activity while my tokens remain vested.

#### Acceptance Criteria

1. WHEN calculating investor shares THEN the system SHALL read still-locked amounts from Streamflow at current timestamp
2. WHEN computing eligible investor share THEN the system SHALL use formula: eligible_investor_share_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))
3. WHEN calculating individual payouts THEN the system SHALL use weight_i(t) = locked_i(t) / locked_total(t)
4. WHEN distributing fees THEN the system SHALL apply floor() to all proportional calculations
5. WHEN processing payouts THEN the system SHALL enforce min_payout_lamports threshold and carry dust forward
6. WHEN an investor page is processed THEN the system SHALL emit an InvestorPayoutPage event

### Requirement 5: Creator Remainder Distribution

**User Story:** As a project creator, I want to receive the remaining quote fees after investor distributions, so that I benefit from trading activity on unlocked portions.

#### Acceptance Criteria

1. WHEN all investor pages are processed for a day THEN the system SHALL route remaining claimed quote fees to creator's quote ATA
2. WHEN all tokens are unlocked THEN the system SHALL route 100% of fees to the creator
3. WHEN calculating creator share THEN the system SHALL use the complement of eligible investor share
4. WHEN creator payout occurs THEN the system SHALL transfer to the provided creator quote ATA
5. IF creator ATA doesn't exist THEN the system SHALL create it according to policy configuration

### Requirement 6: Idempotent Pagination System

**User Story:** As a crank operator, I want the system to handle pagination safely with resumable operations, so that partial failures don't result in double payments or lost distributions.

#### Acceptance Criteria

1. WHEN tracking daily progress THEN the system SHALL maintain last_distribution_ts, cumulative distributed, carry-over, and pagination cursor
2. WHEN re-running pages THEN the system SHALL not double-pay any investor
3. WHEN resuming mid-day after partial success THEN the system SHALL continue from the correct pagination cursor
4. WHEN a page fails THEN the system SHALL allow safe retry without affecting completed pages
5. WHEN daily caps are applied THEN the system SHALL track cumulative distributions and enforce limits

### Requirement 7: Policy Configuration and Limits

**User Story:** As a platform administrator, I want configurable policies for fee distribution including caps and dust handling, so that the system operates within defined parameters.

#### Acceptance Criteria

1. WHEN initializing the system THEN the system SHALL accept investor_fee_share_bps configuration
2. WHEN processing distributions THEN the system SHALL apply optional daily caps if configured
3. WHEN handling small amounts THEN the system SHALL enforce min_payout_lamports and carry dust forward
4. WHEN Y0 (total investor allocation) is provided THEN the system SHALL use it for locked ratio calculations
5. WHEN policy parameters are invalid THEN the system SHALL reject initialization with clear error messages

### Requirement 8: Integration and Account Management

**User Story:** As an integrator, I want clear account requirements and deterministic PDA derivation, so that I can reliably interact with the program.

#### Acceptance Criteria

1. WHEN deriving PDAs THEN the system SHALL use deterministic seeds for all program-derived addresses
2. WHEN requiring accounts for initialization THEN the system SHALL document cp-amm program, pool accounts, token vaults, and system programs
3. WHEN requiring accounts for crank THEN the system SHALL document honorary position, treasury ATA, creator ATA, Streamflow accounts, and policy PDAs
4. WHEN account derivation fails THEN the system SHALL provide clear error codes and messages
5. WHEN integrating THEN the system SHALL be fully Anchor-compatible with no unsafe code

### Requirement 9: Comprehensive Testing and Validation

**User Story:** As a developer, I want comprehensive test coverage including edge cases, so that the system operates reliably in all scenarios.

#### Acceptance Criteria

1. WHEN testing initialization THEN tests SHALL cover pool setup and honorary position creation
2. WHEN testing distribution logic THEN tests SHALL cover partial locks, full unlocks, and dust scenarios
3. WHEN testing failure cases THEN tests SHALL verify base-fee presence causes deterministic failure
4. WHEN testing pagination THEN tests SHALL verify idempotent retries and resumable operations
5. WHEN testing caps and limits THEN tests SHALL verify proper enforcement and carry-forward behavior

### Requirement 10: Documentation and Deliverables

**User Story:** As a user of this module, I want comprehensive documentation and clear integration instructions, so that I can successfully implement and maintain the system.

#### Acceptance Criteria

1. WHEN delivering the module THEN the system SHALL include a clear README with integration steps
2. WHEN documenting accounts THEN the README SHALL include account tables and PDA derivation details
3. WHEN documenting errors THEN the README SHALL include error codes and failure mode explanations
4. WHEN documenting operations THEN the README SHALL explain day/pagination semantics clearly
5. WHEN providing examples THEN the documentation SHALL include end-to-end integration examples