use anchor_lang::prelude::*;

/// The main staking pool that manages all stakes and rewards
/// This is the "master" account that contains global state
#[account]
#[derive(InitSpace)]
pub struct StakingPool {
    /// Authority that can manage the pool (usually the program creator)
    pub authority: Pubkey,
    
    /// The token that users stake (e.g., a project token)
    pub stake_mint: Pubkey,
    
    /// The token paid out as rewards (could be same as stake_mint)
    pub reward_mint: Pubkey,
    
    /// Token account that holds all staked tokens
    pub stake_vault: Pubkey,
    
    /// Token account that holds reward tokens for distribution
    pub reward_vault: Pubkey,
    
    /// Reward rate: tokens per second per staked token (scaled by 1e9 for precision)
    /// Example: 1e9 = 1 reward token per second per staked token
    pub reward_rate: u64,
    
    /// Total amount of tokens currently staked in the pool
    pub total_staked: u64,
    
    /// Last time the reward calculations were updated
    pub last_update_time: i64,
    
    /// Accumulated reward per token (scaled by 1e18 for precision)
    /// This is the key to efficient reward calculation
    pub reward_per_token_stored: u128,
    
    /// Minimum lock duration in seconds (e.g., 7 days = 604800)
    pub lock_duration: i64,
    
    /// Whether the pool is currently active and accepting stakes
    pub is_active: bool,
    
    /// When this pool was created
    pub created_at: i64,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
}

/// Individual user stake account - one per user per pool
/// This is the "detail" account that tracks each user's stake
#[account]
#[derive(InitSpace)]
pub struct UserStake {
    /// The user who owns this stake
    pub user: Pubkey,
    
    /// Which staking pool this stake belongs to
    pub pool: Pubkey,
    
    /// Amount of tokens this user has staked
    pub amount: u64,
    
    /// The reward_per_token value when user last claimed/updated
    /// Used to calculate how much reward they've earned since then
    pub reward_per_token_paid: u128,
    
    /// Unclaimed rewards accumulated for this user
    pub rewards: u64,
    
    /// When the user first staked (for lock period calculation)
    pub stake_time: i64,
    
    /// When the user can unstake (stake_time + lock_duration)
    pub unlock_time: i64,
    
    /// Whether this stake is currently active
    pub is_active: bool,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl StakingPool {
    /// Calculate the current reward per token
    /// This is the core of our reward system
    pub fn calculate_reward_per_token(&self, current_time: i64) -> u128 {
        // If no tokens are staked, no rewards accumulate
        if self.total_staked == 0 {
            return self.reward_per_token_stored;
        }
        
        // Calculate time elapsed since last update
        let time_elapsed = (current_time - self.last_update_time) as u128;
        
        // Calculate additional reward per token since last update
        // Formula: (reward_rate * time_elapsed * PRECISION) / total_staked
        let additional_reward_per_token = (self.reward_rate as u128)
            .checked_mul(time_elapsed)
            .and_then(|x| x.checked_mul(1_000_000_000_000_000_000)) // 1e18 precision
            .and_then(|x| x.checked_div(self.total_staked as u128))
            .unwrap_or(0);
        
        // Add to stored value
        self.reward_per_token_stored
            .checked_add(additional_reward_per_token)
            .unwrap_or(self.reward_per_token_stored)
    }
    
    /// Check if the pool is currently accepting stakes
    pub fn can_stake(&self, current_time: i64) -> bool {
        self.is_active
    }
    
    /// Get pool statistics for display
    pub fn get_stats(&self) -> (u64, u64, u128) {
        (self.total_staked, self.reward_rate, self.reward_per_token_stored)
    }
}

impl UserStake {
    /// Calculate pending rewards for this user
    pub fn calculate_pending_rewards(&self, current_reward_per_token: u128) -> u64 {
        // Calculate rewards earned since last update
        let reward_per_token_diff = current_reward_per_token
            .checked_sub(self.reward_per_token_paid)
            .unwrap_or(0);
        
        // Calculate user's share: amount * reward_per_token_diff / precision
        let new_rewards = (self.amount as u128)
            .checked_mul(reward_per_token_diff)
            .and_then(|x| x.checked_div(1_000_000_000_000_000_000)) // 1e18 precision
            .unwrap_or(0) as u64;
        
        // Add to existing unclaimed rewards
        self.rewards.checked_add(new_rewards).unwrap_or(self.rewards)
    }
    
    /// Check if user can unstake (lock period has passed)
    pub fn can_unstake(&self, current_time: i64) -> bool {
        self.is_active && current_time >= self.unlock_time
    }
    
    /// Get time remaining until unlock
    pub fn time_until_unlock(&self, current_time: i64) -> i64 {
        if current_time >= self.unlock_time {
            0
        } else {
            self.unlock_time - current_time
        }
    }
    
    /// Get user stake summary
    pub fn get_summary(&self, current_time: i64) -> (u64, u64, i64, bool) {
        (
            self.amount,
            self.rewards,
            self.time_until_unlock(current_time),
            self.can_unstake(current_time),
        )
    }
}
