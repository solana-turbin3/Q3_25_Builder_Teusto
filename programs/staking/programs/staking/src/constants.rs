// PDA Seeds for deterministic address generation

/// Seed for StakingPool PDAs: ["pool", authority.key(), pool_id]
/// This allows one authority to create multiple pools with different IDs
pub const POOL_SEED: &[u8] = b"pool";

/// Seed for UserStake PDAs: ["stake", pool.key(), user.key()]
/// This ensures one stake account per user per pool
pub const STAKE_SEED: &[u8] = b"stake";

/// Seed for Stake Vault PDAs: ["stake_vault", pool.key()]
/// Token account that holds all staked tokens for a pool
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";

/// Seed for Reward Vault PDAs: ["reward_vault", pool.key()]
/// Token account that holds reward tokens for distribution
pub const REWARD_VAULT_SEED: &[u8] = b"reward_vault";

// Precision and Mathematical Constants

/// Precision multiplier for reward calculations (1e18)
/// We use high precision to avoid rounding errors in reward calculations
pub const REWARD_PRECISION: u128 = 1_000_000_000_000_000_000;

/// Precision multiplier for reward rates (1e9)
/// Reward rates are stored as tokens per second * 1e9 for precision
pub const RATE_PRECISION: u64 = 1_000_000_000;

// Time Constants

/// Minimum lock duration (1 day in seconds)
pub const MIN_LOCK_DURATION: i64 = 24 * 60 * 60; // 86,400 seconds

/// Maximum lock duration (365 days in seconds)
pub const MAX_LOCK_DURATION: i64 = 365 * 24 * 60 * 60; // 31,536,000 seconds

/// Default lock duration (7 days in seconds)
pub const DEFAULT_LOCK_DURATION: i64 = 7 * 24 * 60 * 60; // 604,800 seconds

// Staking Limits

/// Minimum stake amount (to prevent dust attacks)
pub const MIN_STAKE_AMOUNT: u64 = 1_000_000; // 1 token with 6 decimals

/// Maximum stake amount per user (to prevent concentration)
pub const MAX_STAKE_AMOUNT: u64 = 1_000_000_000_000; // 1M tokens with 6 decimals

// Pool Configuration Limits

/// Minimum reward rate (very small but not zero)
pub const MIN_REWARD_RATE: u64 = 1; // 1 token per second per 1B staked tokens

/// Maximum reward rate (to prevent excessive inflation)
pub const MAX_REWARD_RATE: u64 = 1_000_000_000; // 1 token per second per staked token

// Account Space Constants

/// Anchor discriminator size (8 bytes)
pub const DISCRIMINATOR_SIZE: usize = 8;

// Error Messages (for better debugging)

/// Standard error message for insufficient rewards in vault
pub const INSUFFICIENT_REWARDS_MSG: &str = "Insufficient reward tokens in vault";

/// Standard error message for stake still locked
pub const STAKE_LOCKED_MSG: &str = "Stake is still locked, cannot unstake yet";

/// Standard error message for inactive pool
pub const POOL_INACTIVE_MSG: &str = "Staking pool is not currently active";

// Utility Functions for Constants

/// Convert annual percentage rate to reward rate per second
/// APR is expected as a percentage (e.g., 10 for 10% APR)
pub fn apr_to_reward_rate(apr_percent: u64) -> u64 {
    // Formula: (APR / 100) / (365 * 24 * 60 * 60) * RATE_PRECISION
    // This gives us tokens per second per staked token
    let seconds_per_year = 365u64 * 24 * 60 * 60; // 31,536,000
    
    apr_percent
        .checked_mul(RATE_PRECISION)
        .and_then(|x| x.checked_div(100))
        .and_then(|x| x.checked_div(seconds_per_year))
        .unwrap_or(0)
}

/// Convert reward rate per second to annual percentage rate
pub fn reward_rate_to_apr(reward_rate: u64) -> u64 {
    let seconds_per_year = 365u64 * 24 * 60 * 60;
    
    reward_rate
        .checked_mul(seconds_per_year)
        .and_then(|x| x.checked_mul(100))
        .and_then(|x| x.checked_div(RATE_PRECISION))
        .unwrap_or(0)
}

/// Check if a lock duration is valid
pub fn is_valid_lock_duration(duration: i64) -> bool {
    duration >= MIN_LOCK_DURATION && duration <= MAX_LOCK_DURATION
}

/// Check if a stake amount is valid
pub fn is_valid_stake_amount(amount: u64) -> bool {
    amount >= MIN_STAKE_AMOUNT && amount <= MAX_STAKE_AMOUNT
}

/// Check if a reward rate is valid
pub fn is_valid_reward_rate(rate: u64) -> bool {
    rate >= MIN_REWARD_RATE && rate <= MAX_REWARD_RATE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apr_conversion() {
        // Test 10% APR conversion
        let rate = apr_to_reward_rate(10);
        let back_to_apr = reward_rate_to_apr(rate);
        
        // Should be approximately 10% (allowing for rounding)
        assert!(back_to_apr >= 9 && back_to_apr <= 11);
    }

    #[test]
    fn test_validation_functions() {
        // Test lock duration validation
        assert!(is_valid_lock_duration(DEFAULT_LOCK_DURATION));
        assert!(!is_valid_lock_duration(0));
        assert!(!is_valid_lock_duration(MAX_LOCK_DURATION + 1));

        // Test stake amount validation
        assert!(is_valid_stake_amount(MIN_STAKE_AMOUNT));
        assert!(!is_valid_stake_amount(MIN_STAKE_AMOUNT - 1));
        assert!(!is_valid_stake_amount(MAX_STAKE_AMOUNT + 1));

        // Test reward rate validation
        assert!(is_valid_reward_rate(MIN_REWARD_RATE));
        assert!(!is_valid_reward_rate(0));
        assert!(!is_valid_reward_rate(MAX_REWARD_RATE + 1));
    }
}
