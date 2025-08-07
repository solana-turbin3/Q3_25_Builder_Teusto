use anchor_lang::prelude::*;

/// Custom error types for our staking system
/// Each error provides a clear message about what went wrong
#[error_code]
pub enum StakingError {
    // Pool Management Errors
    #[msg("Staking pool is not currently active")]
    PoolNotActive,
    
    #[msg("Only the pool authority can perform this action")]
    UnauthorizedPoolAuthority,
    
    #[msg("Pool already exists with this configuration")]
    PoolAlreadyExists,
    
    #[msg("Invalid reward rate provided")]
    InvalidRewardRate,
    
    #[msg("Invalid lock duration provided")]
    InvalidLockDuration,
    
    // Staking Errors
    #[msg("Stake amount is below minimum required")]
    StakeAmountTooSmall,
    
    #[msg("Stake amount exceeds maximum allowed")]
    StakeAmountTooLarge,
    
    #[msg("User already has an active stake in this pool")]
    UserAlreadyStaked,
    
    #[msg("Insufficient token balance to stake")]
    InsufficientBalance,
    
    // Unstaking Errors
    #[msg("No active stake found for this user")]
    NoActiveStake,
    
    #[msg("Stake is still locked, cannot unstake yet")]
    StakeStillLocked,
    
    #[msg("Cannot unstake zero amount")]
    CannotUnstakeZero,
    
    // Reward Errors
    #[msg("No rewards available to claim")]
    NoRewardsAvailable,
    
    #[msg("Insufficient reward tokens in vault")]
    InsufficientRewardTokens,
    
    #[msg("Reward calculation overflow")]
    RewardCalculationOverflow,
    
    // Time and Math Errors
    #[msg("Invalid timestamp provided")]
    InvalidTimestamp,
    
    #[msg("Mathematical overflow in calculations")]
    MathOverflow,
    
    #[msg("Division by zero in reward calculations")]
    DivisionByZero,
    
    // Token and Account Errors
    #[msg("Invalid token mint provided")]
    InvalidTokenMint,
    
    #[msg("Token account has insufficient balance")]
    InsufficientTokenBalance,
    
    #[msg("Invalid token account provided")]
    InvalidTokenAccount,
    
    #[msg("Token account is not owned by the expected authority")]
    InvalidTokenAccountOwner,
    
    // Vault Errors
    #[msg("Stake vault is empty")]
    EmptyStakeVault,
    
    #[msg("Reward vault is empty")]
    EmptyRewardVault,
    
    #[msg("Vault balance mismatch")]
    VaultBalanceMismatch,
    
    // General Validation Errors
    #[msg("Invalid account provided")]
    InvalidAccount,
    
    #[msg("Account is not initialized")]
    AccountNotInitialized,
    
    #[msg("Account is already initialized")]
    AccountAlreadyInitialized,
    
    #[msg("Invalid program authority")]
    InvalidProgramAuthority,
    
    // Business Logic Errors
    #[msg("Operation not allowed in current state")]
    OperationNotAllowed,
    
    #[msg("Pool has no staked tokens")]
    NoStakedTokens,
    
    #[msg("Cannot perform operation on inactive stake")]
    InactiveStake,
    
    #[msg("Lock period has not started yet")]
    LockPeriodNotStarted,
    
    #[msg("Reward period has ended")]
    RewardPeriodEnded,
}

impl StakingError {
    /// Get error code as u32 for logging
    pub fn error_code(&self) -> u32 {
        match self {
            // Pool errors: 1000-1099
            StakingError::PoolNotActive => 1001,
            StakingError::UnauthorizedPoolAuthority => 1002,
            StakingError::PoolAlreadyExists => 1003,
            StakingError::InvalidRewardRate => 1004,
            StakingError::InvalidLockDuration => 1005,
            
            // Staking errors: 1100-1199
            StakingError::StakeAmountTooSmall => 1101,
            StakingError::StakeAmountTooLarge => 1102,
            StakingError::UserAlreadyStaked => 1103,
            StakingError::InsufficientBalance => 1104,
            
            // Unstaking errors: 1200-1299
            StakingError::NoActiveStake => 1201,
            StakingError::StakeStillLocked => 1202,
            StakingError::CannotUnstakeZero => 1203,
            
            // Reward errors: 1300-1399
            StakingError::NoRewardsAvailable => 1301,
            StakingError::InsufficientRewardTokens => 1302,
            StakingError::RewardCalculationOverflow => 1303,
            
            // Math errors: 1400-1499
            StakingError::InvalidTimestamp => 1401,
            StakingError::MathOverflow => 1402,
            StakingError::DivisionByZero => 1403,
            
            // Token errors: 1500-1599
            StakingError::InvalidTokenMint => 1501,
            StakingError::InsufficientTokenBalance => 1502,
            StakingError::InvalidTokenAccount => 1503,
            StakingError::InvalidTokenAccountOwner => 1504,
            
            // Vault errors: 1600-1699
            StakingError::EmptyStakeVault => 1601,
            StakingError::EmptyRewardVault => 1602,
            StakingError::VaultBalanceMismatch => 1603,
            
            // General errors: 1700-1799
            StakingError::InvalidAccount => 1701,
            StakingError::AccountNotInitialized => 1702,
            StakingError::AccountAlreadyInitialized => 1703,
            StakingError::InvalidProgramAuthority => 1704,
            
            // Business logic errors: 1800-1899
            StakingError::OperationNotAllowed => 1801,
            StakingError::NoStakedTokens => 1802,
            StakingError::InactiveStake => 1803,
            StakingError::LockPeriodNotStarted => 1804,
            StakingError::RewardPeriodEnded => 1805,
        }
    }
    
    /// Get human-readable error category
    pub fn category(&self) -> &'static str {
        match self.error_code() {
            1000..=1099 => "Pool Management",
            1100..=1199 => "Staking Operations",
            1200..=1299 => "Unstaking Operations", 
            1300..=1399 => "Reward Operations",
            1400..=1499 => "Mathematical Operations",
            1500..=1599 => "Token Operations",
            1600..=1699 => "Vault Operations",
            1700..=1799 => "Account Validation",
            1800..=1899 => "Business Logic",
            _ => "Unknown",
        }
    }
}

/// Helper macro for logging errors with context
#[macro_export]
macro_rules! log_error {
    ($error:expr, $context:expr) => {
        msg!(
            "Error {}: {} in context: {}",
            $error.error_code(),
            $error.category(),
            $context
        );
    };
}

/// Helper function to validate timestamp
pub fn validate_timestamp(timestamp: i64) -> Result<()> {
    // Check if timestamp is reasonable (not too far in past or future)
    let current_time = Clock::get()?.unix_timestamp;
    let one_year = 365 * 24 * 60 * 60; // seconds in a year
    
    if timestamp < current_time - one_year || timestamp > current_time + one_year {
        return Err(StakingError::InvalidTimestamp.into());
    }
    
    Ok(())
}

/// Helper function to safely add two u64 values
pub fn safe_add_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(StakingError::MathOverflow.into())
}

/// Helper function to safely subtract two u64 values
pub fn safe_sub_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(StakingError::MathOverflow.into())
}

/// Helper function to safely multiply two u64 values
pub fn safe_mul_u64(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(StakingError::MathOverflow.into())
}

/// Helper function to safely divide two u64 values
pub fn safe_div_u64(a: u64, b: u64) -> Result<u64> {
    if b == 0 {
        return Err(StakingError::DivisionByZero.into());
    }
    Ok(a / b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(StakingError::PoolNotActive.error_code(), 1001);
        assert_eq!(StakingError::StakeAmountTooSmall.error_code(), 1101);
        assert_eq!(StakingError::NoActiveStake.error_code(), 1201);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(StakingError::PoolNotActive.category(), "Pool Management");
        assert_eq!(StakingError::StakeAmountTooSmall.category(), "Staking Operations");
        assert_eq!(StakingError::NoActiveStake.category(), "Unstaking Operations");
    }

    #[test]
    fn test_safe_math_functions() {
        // Test safe addition
        assert!(safe_add_u64(100, 200).is_ok());
        assert_eq!(safe_add_u64(100, 200).unwrap(), 300);
        assert!(safe_add_u64(u64::MAX, 1).is_err());

        // Test safe subtraction
        assert!(safe_sub_u64(200, 100).is_ok());
        assert_eq!(safe_sub_u64(200, 100).unwrap(), 100);
        assert!(safe_sub_u64(100, 200).is_err());

        // Test safe division
        assert!(safe_div_u64(100, 10).is_ok());
        assert_eq!(safe_div_u64(100, 10).unwrap(), 10);
        assert!(safe_div_u64(100, 0).is_err());
    }
}
