use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    constants::*,
    error::StakingError,
    state::StakingPool,
};

/// Initialize a new staking pool with specified parameters
/// This creates the master pool account and associated token vaults
#[derive(Accounts)]
#[instruction(pool_id: u64)]
pub struct InitializePool<'info> {
    /// The authority who will control this pool (usually the pool creator)
    /// This account pays for the initialization and becomes the pool authority
    #[account(mut)]
    pub authority: Signer<'info>,

    /// The staking pool account - this is our master account
    /// PDA: ["pool", authority.key(), pool_id]
    /// This ensures each authority can create multiple pools with different IDs
    #[account(
        init,
        payer = authority,
        space = StakingPool::INIT_SPACE,
        seeds = [POOL_SEED, authority.key().as_ref(), pool_id.to_le_bytes().as_ref()],
        bump
    )]
    pub pool: Account<'info, StakingPool>,

    /// The token that users will stake (e.g., project token, governance token)
    pub stake_mint: Account<'info, Mint>,

    /// The token that will be paid out as rewards
    /// Can be the same as stake_mint for single-token staking
    pub reward_mint: Account<'info, Mint>,

    /// Token account that will hold all staked tokens
    /// PDA: ["stake_vault", pool.key()]
    /// Program authority ensures only the program can control these tokens
    #[account(
        init,
        payer = authority,
        seeds = [STAKE_VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = stake_mint,
        token::authority = pool,
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    /// Token account that will hold reward tokens for distribution
    /// PDA: ["reward_vault", pool.key()]
    /// Must be funded by the authority before rewards can be distributed
    #[account(
        init,
        payer = authority,
        seeds = [REWARD_VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = pool,
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    /// Required system programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitializePool<'info> {
    /// Initialize the staking pool with the provided parameters
    pub fn initialize_pool(
        &mut self,
        pool_id: u64,
        reward_rate: u64,
        lock_duration: i64,
        bumps: &InitializePoolBumps,
    ) -> Result<()> {
        // Get current timestamp for pool creation
        let current_time = Clock::get()?.unix_timestamp;

        // Validate input parameters before proceeding
        self.validate_parameters(reward_rate, lock_duration)?;

        // Initialize the pool account with all necessary data
        let pool = &mut self.pool;
        
        // Set pool authority and basic configuration
        pool.authority = self.authority.key();
        pool.stake_mint = self.stake_mint.key();
        pool.reward_mint = self.reward_mint.key();
        pool.stake_vault = self.stake_vault.key();
        pool.reward_vault = self.reward_vault.key();

        // Set reward parameters
        pool.reward_rate = reward_rate;
        pool.lock_duration = lock_duration;

        // Initialize state variables
        pool.total_staked = 0;
        pool.last_update_time = current_time;
        pool.reward_per_token_stored = 0;

        // Set pool status and metadata
        pool.is_active = true;
        pool.created_at = current_time;
        pool.bump = bumps.pool;

        // Log pool creation for monitoring and debugging
        msg!(
            "Staking pool initialized: ID={}, Authority={}, StakeMint={}, RewardMint={}",
            pool_id,
            pool.authority,
            pool.stake_mint,
            pool.reward_mint
        );

        msg!(
            "Pool parameters: RewardRate={}, LockDuration={} seconds, APR={}%",
            pool.reward_rate,
            pool.lock_duration,
            reward_rate_to_apr(pool.reward_rate)
        );

        Ok(())
    }

    /// Validate all input parameters to ensure they meet our requirements
    fn validate_parameters(&self, reward_rate: u64, lock_duration: i64) -> Result<()> {
        // Validate reward rate is within acceptable bounds
        if !is_valid_reward_rate(reward_rate) {
            msg!(
                "Invalid reward rate: {}. Must be between {} and {}",
                reward_rate,
                MIN_REWARD_RATE,
                MAX_REWARD_RATE
            );
            return Err(StakingError::InvalidRewardRate.into());
        }

        // Validate lock duration is within acceptable bounds
        if !is_valid_lock_duration(lock_duration) {
            msg!(
                "Invalid lock duration: {} seconds. Must be between {} and {} seconds",
                lock_duration,
                MIN_LOCK_DURATION,
                MAX_LOCK_DURATION
            );
            return Err(StakingError::InvalidLockDuration.into());
        }

        // Validate token mints are different if this is a dual-token pool
        // (This is actually allowed - same token can be used for stake and rewards)
        if self.stake_mint.key() == self.reward_mint.key() {
            msg!("Single-token staking pool detected (stake and reward tokens are the same)");
        } else {
            msg!("Dual-token staking pool detected (different stake and reward tokens)");
        }

        // Validate mint decimals are reasonable (not too high to cause overflow issues)
        if self.stake_mint.decimals > 9 || self.reward_mint.decimals > 9 {
            msg!("Warning: High decimal count detected. Ensure calculations don't overflow.");
        }

        Ok(())
    }

    /// Get pool initialization summary for logging
    pub fn get_initialization_summary(&self, pool_id: u64, reward_rate: u64, lock_duration: i64) -> String {
        format!(
            "Pool {} initialized with {}% APR, {}-day lock period",
            pool_id,
            reward_rate_to_apr(reward_rate),
            lock_duration / (24 * 60 * 60)
        )
    }
}

/// Helper function to validate pool initialization parameters
/// This can be called by frontend applications before submitting transactions
pub fn validate_pool_params(reward_rate: u64, lock_duration: i64) -> Result<()> {
    if !is_valid_reward_rate(reward_rate) {
        return Err(StakingError::InvalidRewardRate.into());
    }
    
    if !is_valid_lock_duration(lock_duration) {
        return Err(StakingError::InvalidLockDuration.into());
    }
    
    Ok(())
}

/// Calculate estimated rewards for a given stake amount and time period
/// Useful for frontend applications to show users expected returns
pub fn calculate_estimated_rewards(
    stake_amount: u64,
    reward_rate: u64,
    time_period_seconds: i64,
) -> u64 {
    // Formula: (stake_amount * reward_rate * time_period) / RATE_PRECISION
    let rewards = (stake_amount as u128)
        .checked_mul(reward_rate as u128)
        .and_then(|x| x.checked_mul(time_period_seconds as u128))
        .and_then(|x| x.checked_div(RATE_PRECISION as u128))
        .unwrap_or(0) as u64;
    
    rewards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pool_params() {
        // Valid parameters should pass
        assert!(validate_pool_params(apr_to_reward_rate(10), DEFAULT_LOCK_DURATION).is_ok());
        
        // Invalid reward rate should fail
        assert!(validate_pool_params(0, DEFAULT_LOCK_DURATION).is_err());
        assert!(validate_pool_params(MAX_REWARD_RATE + 1, DEFAULT_LOCK_DURATION).is_err());
        
        // Invalid lock duration should fail
        assert!(validate_pool_params(apr_to_reward_rate(10), 0).is_err());
        assert!(validate_pool_params(apr_to_reward_rate(10), MAX_LOCK_DURATION + 1).is_err());
    }

    #[test]
    fn test_calculate_estimated_rewards() {
        let stake_amount = 1000 * 10_u64.pow(6); // 1000 tokens with 6 decimals
        let reward_rate = apr_to_reward_rate(10); // 10% APR
        let one_year = 365 * 24 * 60 * 60; // seconds in a year
        
        let rewards = calculate_estimated_rewards(stake_amount, reward_rate, one_year);
        
        // Should be approximately 10% of stake amount (100 tokens)
        let expected = 100 * 10_u64.pow(6);
        let tolerance = expected / 100; // 1% tolerance
        
        assert!(rewards >= expected - tolerance && rewards <= expected + tolerance);
    }

    #[test]
    fn test_apr_reward_rate_conversion() {
        // Test various APR values
        let test_cases = vec![1, 5, 10, 25, 50, 100];
        
        for apr in test_cases {
            let rate = apr_to_reward_rate(apr);
            let back_to_apr = reward_rate_to_apr(rate);
            
            // Should be approximately equal (allowing for rounding)
            assert!(back_to_apr >= apr - 1 && back_to_apr <= apr + 1);
        }
    }
}
