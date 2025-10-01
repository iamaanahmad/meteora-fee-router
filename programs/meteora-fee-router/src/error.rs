use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid quote mint configuration")]
    InvalidQuoteMint,
    
    #[msg("Base fees detected - quote-only enforcement failed")]
    BaseFeeDetected,
    
    #[msg("24-hour cooldown not elapsed")]
    CooldownNotElapsed,
    
    #[msg("Daily distribution cap exceeded")]
    DailyCapExceeded,
    
    #[msg("Payout below minimum threshold")]
    PayoutBelowMinimum,
    
    #[msg("Invalid pagination cursor")]
    InvalidPaginationCursor,
    
    #[msg("Arithmetic overflow in fee calculation")]
    ArithmeticOverflow,
    
    #[msg("Streamflow account validation failed")]
    StreamflowValidationFailed,
    
    #[msg("Invalid PDA derivation")]
    InvalidPda,
    
    #[msg("Invalid Streamflow stream mint")]
    InvalidStreamMint,
    
    #[msg("Streamflow stream is closed")]
    StreamClosed,
    
    #[msg("Invalid Streamflow stream time parameters")]
    InvalidStreamTimeParameters,
    
    #[msg("Streamflow account data parsing failed")]
    StreamflowDataParsingFailed,
    
    #[msg("Invalid pool configuration for quote-only fees")]
    InvalidPoolConfiguration,
    
    #[msg("Position owner PDA mismatch")]
    PositionOwnerMismatch,
    
    #[msg("Invalid tick range for quote-only accrual")]
    InvalidTickRange,
    
    #[msg("Treasury ATA not found or invalid")]
    InvalidTreasuryAta,
    
    #[msg("Creator ATA not found or invalid")]
    InvalidCreatorAta,
    
    #[msg("Insufficient funds for distribution")]
    InsufficientFunds,
    
    #[msg("Invalid investor fee share basis points")]
    InvalidInvestorFeeShare,
    
    #[msg("Day already complete, cannot continue distribution")]
    DayAlreadyComplete,
    
    #[msg("Day not complete, cannot process creator payout")]
    DayNotComplete,
    
    #[msg("Invalid Y0 total allocation")]
    InvalidY0TotalAllocation,
    
    #[msg("Pool token order validation failed")]
    PoolTokenOrderValidationFailed,
    
    #[msg("Cross-program invocation failed")]
    CpiCallFailed,
    
    #[msg("Account initialization failed")]
    AccountInitializationFailed,
    
    #[msg("Invalid fee share basis points (must be 0-10000)")]
    InvalidFeeShareBasisPoints,
    
    #[msg("Invalid minimum payout threshold")]
    InvalidMinPayoutThreshold,
    
    #[msg("Invalid total allocation (must be greater than 0)")]
    InvalidTotalAllocation,
    
    #[msg("Invalid daily cap (must be greater than 0)")]
    InvalidDailyCap,
    
    #[msg("Fee claiming failed")]
    FeeClaimingFailed,
    
    #[msg("Treasury transfer failed")]
    TreasuryTransferFailed,
    
    #[msg("Position fee data extraction failed")]
    PositionFeeDataExtractionFailed,
    
    #[msg("Invalid vault account")]
    InvalidVaultAccount,
}