// Constants for the Meteora Fee Router program

/// Seed for policy configuration PDA
pub const POLICY_SEED: &[u8] = b"policy";

/// Seed for distribution progress PDA
pub const PROGRESS_SEED: &[u8] = b"progress";

/// Seed for vault-related PDAs
pub const VAULT_SEED: &[u8] = b"vault";

/// 24 hours in seconds
pub const TWENTY_FOUR_HOURS: i64 = 86400;

/// Maximum basis points (100%)
pub const MAX_BASIS_POINTS: u16 = 10000;

/// Precision multiplier for weight calculations
pub const WEIGHT_PRECISION: u128 = 1_000_000;

/// Default minimum payout threshold (1000 lamports)
pub const DEFAULT_MIN_PAYOUT: u64 = 1000;

/// Maximum page size for pagination
pub const MAX_PAGE_SIZE: u32 = 50;