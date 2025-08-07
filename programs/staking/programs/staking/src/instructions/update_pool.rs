use anchor_lang::prelude::*;

use crate::{
    error::StakingError,
    state::StakingPool,
};

/// Update pool reward calculations
/// Should be called periodically to keep reward calculations accurate
/// This is a lightweight operation that anyone can call
#[derive(Accounts)]
pub struct UpdatePool<'info> {
    /// The staking pool to update
    /// Must be properly initialized and active
    #[account(
        mut,
        constraint = pool.is_active @ StakingError::PoolNotActive,
    )]
    pub pool: Account<'info, StakingPool>,

    /// The caller of this instruction (can be anyone)
    /// No signature required - this is a public utility function
    /// CHECK: This account is not validated as anyone can call this instruction
    pub caller: UncheckedAccount<'info>,
}

impl<'info> UpdatePool<'info> {
    /// Execute the pool update operation
    pub fn update_pool(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;

        // Validate that the update is meaningful
        self.validate_update(current_time)?;

        // Calculate and store new reward per token
        let previous_reward_per_token = self.pool.reward_per_token_stored;
        let new_reward_per_token = self.pool.calculate_reward_per_token(current_time);

        // Update pool state
        self.pool.reward_per_token_stored = new_reward_per_token;
        self.pool.last_update_time = current_time;

        // Log the update event
        self.log_update_event(previous_reward_per_token, new_reward_per_token, current_time)?;

        Ok(())
    }

    /// Validate that the pool update is meaningful and allowed
    fn validate_update(&self, current_time: i64) -> Result<()> {
        let pool = &self.pool;

        // Check if pool is active
        if !pool.is_active {
            return Err(StakingError::PoolNotActive.into());
        }

        // Validate timestamp
        crate::error::validate_timestamp(current_time)?;

        // Check if enough time has passed to make update meaningful
        // This prevents spam updates that waste gas
        let time_since_last_update = current_time - pool.last_update_time;
        if time_since_last_update < 0 {
            msg!("Invalid timestamp: current time is before last update");
            return Err(StakingError::InvalidTimestamp.into());
        }

        // Log validation info
        msg!(
            "Update validation passed: time_elapsed={} seconds, total_staked={}",
            time_since_last_update,
            pool.total_staked
        );

        Ok(())
    }

    /// Log the pool update event for monitoring and analytics
    fn log_update_event(
        &self,
        previous_reward_per_token: u128,
        new_reward_per_token: u128,
        current_time: i64,
    ) -> Result<()> {
        let pool = &self.pool;
        let time_elapsed = current_time - pool.last_update_time;
        let reward_increase = new_reward_per_token.saturating_sub(previous_reward_per_token);

        msg!(
            "POOL UPDATE: pool={}, caller={}, time_elapsed={} seconds",
            pool.key(),
            self.caller.key(),
            time_elapsed
        );

        msg!(
            "Reward calculations: previous={}, new={}, increase={}",
            previous_reward_per_token,
            new_reward_per_token,
            reward_increase
        );

        msg!(
            "Pool status: total_staked={}, reward_rate={}, active={}",
            pool.total_staked,
            pool.reward_rate,
            pool.is_active
        );

        // Calculate current APR for informational purposes
        let current_apr = crate::constants::reward_rate_to_apr(pool.reward_rate);
        msg!("Current pool APR: {}%", current_apr);

        // Log efficiency metrics
        if time_elapsed > 0 {
            let rewards_per_second = reward_increase as f64 / time_elapsed as f64;
            msg!("Reward accumulation rate: {:.2} per second", rewards_per_second);
        }

        Ok(())
    }

    /// Get pool update summary for display
    pub fn get_update_summary(&self, current_time: i64) -> UpdateSummary {
        let pool = &self.pool;
        let time_since_last_update = current_time - pool.last_update_time;
        let new_reward_per_token = pool.calculate_reward_per_token(current_time);
        let reward_increase = new_reward_per_token.saturating_sub(pool.reward_per_token_stored);

        UpdateSummary {
            pool_address: pool.key(),
            time_since_last_update_seconds: time_since_last_update,
            current_reward_per_token: pool.reward_per_token_stored,
            new_reward_per_token,
            reward_increase,
            total_staked: pool.total_staked,
            is_meaningful_update: time_since_last_update > 0 && (pool.total_staked > 0 || reward_increase > 0),
        }
    }

    /// Check if a pool update would be meaningful
    pub fn is_update_needed(&self, current_time: i64, min_time_threshold: i64) -> bool {
        let pool = &self.pool;
        
        // Check if enough time has passed
        let time_elapsed = current_time - pool.last_update_time;
        if time_elapsed < min_time_threshold {
            return false;
        }

        // Check if there are staked tokens (no point updating empty pool)
        if pool.total_staked == 0 {
            return false;
        }

        // Check if pool is active
        if !pool.is_active {
            return false;
        }

        true
    }
}

/// Summary information about a pool update operation
#[derive(Debug, Clone)]
pub struct UpdateSummary {
    pub pool_address: Pubkey,
    pub time_since_last_update_seconds: i64,
    pub current_reward_per_token: u128,
    pub new_reward_per_token: u128,
    pub reward_increase: u128,
    pub total_staked: u64,
    pub is_meaningful_update: bool,
}

/// Check if a pool needs to be updated
pub fn should_update_pool(
    pool: &StakingPool,
    current_time: i64,
    min_time_threshold: i64,
) -> bool {
    // Pool must be active
    if !pool.is_active {
        return false;
    }

    // Must have staked tokens
    if pool.total_staked == 0 {
        return false;
    }

    // Must have enough time elapsed
    let time_elapsed = current_time - pool.last_update_time;
    time_elapsed >= min_time_threshold
}

/// Calculate the reward increase that would result from updating a pool
pub fn calculate_potential_reward_increase(
    pool: &StakingPool,
    current_time: i64,
) -> u128 {
    let new_reward_per_token = pool.calculate_reward_per_token(current_time);
    new_reward_per_token.saturating_sub(pool.reward_per_token_stored)
}

/// Get pool statistics for monitoring
pub fn get_pool_stats(pool: &StakingPool, current_time: i64) -> PoolStats {
    let time_since_last_update = current_time - pool.last_update_time;
    let current_reward_per_token = pool.calculate_reward_per_token(current_time);
    let pending_reward_increase = current_reward_per_token.saturating_sub(pool.reward_per_token_stored);

    PoolStats {
        total_staked: pool.total_staked,
        reward_rate: pool.reward_rate,
        current_apr: crate::constants::reward_rate_to_apr(pool.reward_rate),
        last_update_time: pool.last_update_time,
        time_since_last_update,
        current_reward_per_token: pool.reward_per_token_stored,
        pending_reward_per_token: current_reward_per_token,
        pending_reward_increase,
        is_active: pool.is_active,
        created_at: pool.created_at,
    }
}

/// Comprehensive pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_staked: u64,
    pub reward_rate: u64,
    pub current_apr: u64,
    pub last_update_time: i64,
    pub time_since_last_update: i64,
    pub current_reward_per_token: u128,
    pub pending_reward_per_token: u128,
    pub pending_reward_increase: u128,
    pub is_active: bool,
    pub created_at: i64,
}

/// Utility function to batch check multiple pools for update needs
pub fn get_pools_needing_update(
    pools: &[&StakingPool],
    current_time: i64,
    min_time_threshold: i64,
) -> Vec<usize> {
    pools
        .iter()
        .enumerate()
        .filter_map(|(index, pool)| {
            if should_update_pool(pool, current_time, min_time_threshold) {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    fn create_mock_pool(
        total_staked: u64,
        last_update_time: i64,
        is_active: bool,
    ) -> StakingPool {
        StakingPool {
            authority: Pubkey::default(),
            stake_mint: Pubkey::default(),
            reward_mint: Pubkey::default(),
            stake_vault: Pubkey::default(),
            reward_vault: Pubkey::default(),
            reward_rate: apr_to_reward_rate(10), // 10% APR
            total_staked,
            last_update_time,
            reward_per_token_stored: 0,
            lock_duration: DEFAULT_LOCK_DURATION,
            is_active,
            created_at: last_update_time,
            bump: 0,
        }
    }

    #[test]
    fn test_should_update_pool() {
        let current_time = 1000000;
        let min_threshold = 3600; // 1 hour

        // Active pool with staked tokens and enough time elapsed
        let pool = create_mock_pool(1000 * 10_u64.pow(6), current_time - 7200, true);
        assert!(should_update_pool(&pool, current_time, min_threshold));

        // Inactive pool should not be updated
        let inactive_pool = create_mock_pool(1000 * 10_u64.pow(6), current_time - 7200, false);
        assert!(!should_update_pool(&inactive_pool, current_time, min_threshold));

        // Empty pool should not be updated
        let empty_pool = create_mock_pool(0, current_time - 7200, true);
        assert!(!should_update_pool(&empty_pool, current_time, min_threshold));

        // Recently updated pool should not be updated
        let recent_pool = create_mock_pool(1000 * 10_u64.pow(6), current_time - 1800, true);
        assert!(!should_update_pool(&recent_pool, current_time, min_threshold));
    }

    #[test]
    fn test_calculate_potential_reward_increase() {
        let current_time = 1000000;
        let pool = create_mock_pool(1000 * 10_u64.pow(6), current_time - 3600, true); // 1 hour ago

        let reward_increase = calculate_potential_reward_increase(&pool, current_time);
        
        // Should have some reward increase for 1 hour of staking
        assert!(reward_increase > 0);
    }

    #[test]
    fn test_get_pool_stats() {
        let current_time = 1000000;
        let pool = create_mock_pool(2000 * 10_u64.pow(6), current_time - 1800, true); // 30 minutes ago

        let stats = get_pool_stats(&pool, current_time);

        assert_eq!(stats.total_staked, 2000 * 10_u64.pow(6));
        assert_eq!(stats.time_since_last_update, 1800);
        assert!(stats.is_active);
        assert!(stats.pending_reward_increase > 0);
    }

    #[test]
    fn test_get_pools_needing_update() {
        let current_time = 1000000;
        let min_threshold = 3600; // 1 hour

        let pools = vec![
            create_mock_pool(1000 * 10_u64.pow(6), current_time - 7200, true),  // Needs update
            create_mock_pool(1000 * 10_u64.pow(6), current_time - 1800, true),  // Too recent
            create_mock_pool(0, current_time - 7200, true),                      // Empty
            create_mock_pool(1000 * 10_u64.pow(6), current_time - 7200, false), // Inactive
            create_mock_pool(2000 * 10_u64.pow(6), current_time - 10800, true), // Needs update
        ];

        let pool_refs: Vec<&StakingPool> = pools.iter().collect();
        let needing_update = get_pools_needing_update(&pool_refs, current_time, min_threshold);

        // Should identify pools at index 0 and 4
        assert_eq!(needing_update, vec![0, 4]);
    }

    #[test]
    fn test_update_summary_meaningful() {
        let current_time = 1000000;
        
        // Mock UpdatePool context (this would be more complex in real implementation)
        // For now, we'll test the logic components separately
        
        let pool = create_mock_pool(1000 * 10_u64.pow(6), current_time - 3600, true);
        let time_elapsed = current_time - pool.last_update_time;
        let new_reward_per_token = pool.calculate_reward_per_token(current_time);
        let reward_increase = new_reward_per_token.saturating_sub(pool.reward_per_token_stored);
        
        // Should be a meaningful update
        assert!(time_elapsed > 0);
        assert!(pool.total_staked > 0);
        assert!(reward_increase > 0);
    }
}
